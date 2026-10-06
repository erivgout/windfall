//! Sampler voices: one playing copy of a sample each, in a pool allocated
//! once.

use std::ops::Range;

use rtrb::Producer;
use windfall_core::{AudioBuffer, pan_gains};
use windfall_project::{ChannelId, Envelope};

use crate::message::{Garbage, retire};
use crate::mixer::{Frame, Mixer};
use crate::plan::{Plan, PlanState, Strip};

/// Most voices that sound at full level at once. One more steals a voice.
pub(crate) const MAX_VOICES: usize = 256;

/// Extra slots for voices that are fading out after being stolen or cut, so
/// that taking a voice never has to end one abruptly.
const FADING_SLOTS: usize = 64;

/// Length of the fade that ends a voice early.
const FADE_SECONDS: f64 = 0.004;

/// Shortest envelope release. A release of zero would be a click.
const MIN_RELEASE_SECONDS: f32 = 0.001;

/// Who started a voice. It decides what stops it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Origin {
    /// A note from a pattern.
    Sequenced,
    /// A note played by hand from the UI.
    Live,
    /// The browser's sample preview.
    Preview,
}

/// Where a voice's signal goes and whose gain and pan shape it.
#[derive(Debug, Clone, Copy)]
enum Route {
    /// Through the channel at this index of the current plan.
    Channel(usize),
    /// The channel was removed while the voice sounded. It fades out into
    /// the master with the gain and pan the channel last had.
    Detached { gain: f32, pan: f32 },
    /// Straight into the master track.
    Master,
}

/// A note to start.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Note {
    /// Index into [`Plan::channels`].
    pub channel: usize,
    pub key: u8,
    pub velocity: f32,
    pub pan: f32,
    /// Frame on which the note ends, for samplers with an envelope.
    pub release_at: u64,
    pub origin: Origin,
}

/// The part of a sample a voice plays, and in which direction.
#[derive(Debug, Clone, Copy)]
struct Region {
    first: usize,
    frames: usize,
    reverse: bool,
}

impl Region {
    /// The frame `index` steps into the region in playing order. Outside
    /// the region is silence.
    #[inline]
    fn frame(&self, sample: &AudioBuffer, index: isize) -> (f32, f32) {
        if index < 0 || index as usize >= self.frames {
            return (0.0, 0.0);
        }
        let index = index as usize;
        sample.stereo_frame(if self.reverse {
            self.first + self.frames - 1 - index
        } else {
            self.first + index
        })
    }

    /// The sample at a fractional position, by four-point Hermite
    /// interpolation. A whole-numbered position returns the stored frame
    /// untouched.
    #[inline]
    fn read(&self, sample: &AudioBuffer, position: f64) -> (f32, f32) {
        let index = position as isize;
        let fraction = (position - index as f64) as f32;
        let current = self.frame(sample, index);
        if fraction == 0.0 {
            return current;
        }
        let before = self.frame(sample, index - 1);
        let next = self.frame(sample, index + 1);
        let after = self.frame(sample, index + 2);
        (
            hermite(before.0, current.0, next.0, after.0, fraction),
            hermite(before.1, current.1, next.1, after.1, fraction),
        )
    }
}

/// Catmull-Rom spline through four evenly spaced points, evaluated
/// `fraction` of the way from `y1` to `y2`.
#[inline]
fn hermite(y0: f32, y1: f32, y2: f32, y3: f32, fraction: f32) -> f32 {
    let c1 = 0.5 * (y2 - y0);
    let c2 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let c3 = 0.5 * (y3 - y0) + 1.5 * (y1 - y2);
    ((c3 * fraction + c2) * fraction + c1) * fraction + y1
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Attack,
    Decay,
    Sustain,
    Release,
    Done,
}

/// A linear attack, decay, sustain, release envelope, advanced one frame at
/// a time.
#[derive(Debug, Clone, Copy)]
struct EnvelopeState {
    stage: Stage,
    level: f32,
    attack_step: f32,
    decay_step: f32,
    sustain: f32,
    release_frames: f32,
    release_step: f32,
}

impl EnvelopeState {
    fn new(envelope: &Envelope, sample_rate: f64) -> Self {
        let frames = |ms: f32| ms * sample_rate as f32 / 1000.0;
        let attack_frames = frames(envelope.attack_ms);
        let decay_frames = frames(envelope.decay_ms).max(1.0);
        let instant_attack = attack_frames < 1.0;
        Self {
            stage: if instant_attack {
                Stage::Decay
            } else {
                Stage::Attack
            },
            level: if instant_attack { 1.0 } else { 0.0 },
            attack_step: if instant_attack {
                0.0
            } else {
                1.0 / attack_frames
            },
            decay_step: (1.0 - envelope.sustain) / decay_frames,
            sustain: envelope.sustain,
            release_frames: frames(envelope.release_ms.max(MIN_RELEASE_SECONDS * 1000.0)).max(1.0),
            release_step: 0.0,
        }
    }

    /// The level for this frame, or `None` once the envelope has run out.
    #[inline]
    fn next(&mut self) -> Option<f32> {
        let level = self.level;
        match self.stage {
            Stage::Attack => {
                self.level += self.attack_step;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                self.level -= self.decay_step;
                if self.level <= self.sustain {
                    self.level = self.sustain;
                    self.stage = Stage::Sustain;
                }
            }
            // Holding a level of zero would only burn a voice.
            Stage::Sustain if self.sustain <= 0.0 => return None,
            Stage::Sustain => {}
            Stage::Release => {
                self.level -= self.release_step;
                if self.level <= 0.0 {
                    self.stage = Stage::Done;
                }
            }
            Stage::Done => return None,
        }
        Some(level)
    }

    fn release(&mut self) {
        if matches!(self.stage, Stage::Release | Stage::Done) {
            return;
        }
        self.release_step = self.level / self.release_frames;
        self.stage = if self.level > 0.0 {
            Stage::Release
        } else {
            Stage::Done
        };
    }

    fn releasing(&self) -> bool {
        self.stage == Stage::Release
    }
}

struct Voice {
    active: bool,
    /// Held for as long as the voice may read it. A stopped voice can still
    /// hold its sample for a moment when there was no room to hand it back.
    sample: Option<AudioBuffer>,
    origin: Origin,
    /// The channel that started the voice. Meaningless for a preview.
    channel: ChannelId,
    route: Route,
    key: u8,
    cut_group: u8,
    /// Frame the voice started on.
    started: u64,
    /// Frame the note ends on. Voices without an envelope ignore it.
    release_at: u64,
    region: Region,
    /// Frames into the region, in playing order.
    position: f64,
    /// Sample frames to advance per output frame: pitch times the ratio of
    /// the sample's rate to the output rate.
    step: f64,
    /// Velocity times the sampler's own gain.
    gain: f32,
    /// The note's pan, added to the channel's.
    pan: f32,
    /// `None` plays the sample to its end at full level.
    envelope: Option<EnvelopeState>,
    /// Frames left of an early fade-out. Zero means the voice is not fading.
    fade: u32,
}

impl Voice {
    fn idle() -> Self {
        Self {
            active: false,
            sample: None,
            origin: Origin::Sequenced,
            channel: ChannelId(0),
            route: Route::Master,
            key: 0,
            cut_group: 0,
            started: 0,
            release_at: u64::MAX,
            region: Region {
                first: 0,
                frames: 0,
                reverse: false,
            },
            position: 0.0,
            step: 1.0,
            gain: 0.0,
            pan: 0.0,
            envelope: None,
            fade: 0,
        }
    }

    /// Adds the voice's next frames to `out`, a stretch of its mixer track's
    /// buffer that begins on frame `start`. Returns true when the voice has
    /// ended.
    ///
    /// Every piece of state moves one frame at a time, so cutting the output
    /// at different places gives the same samples.
    fn render(&mut self, out: &mut [Frame], mix: Strip, start: u64, fade_frames: u32) -> bool {
        let Some(sample) = &self.sample else {
            return true;
        };
        let steady = mix.gain.settled(start) && mix.pan.settled(start);
        let (mut left, mut right) = stereo_gains(mix.gain.at(start), mix.pan.at(start) + self.pan);

        for (offset, out) in out.iter_mut().enumerate() {
            let frame = start + offset as u64;
            if !steady {
                (left, right) = stereo_gains(mix.gain.at(frame), mix.pan.at(frame) + self.pan);
            }
            let mut level = self.gain;
            if let Some(envelope) = &mut self.envelope {
                if frame >= self.release_at {
                    envelope.release();
                }
                match envelope.next() {
                    Some(envelope_level) => level *= envelope_level,
                    None => return true,
                }
            }
            if self.fade > 0 {
                level *= self.fade as f32 / fade_frames as f32;
            }
            let (sample_left, sample_right) = self.region.read(sample, self.position);
            out[0] += sample_left * level * left;
            out[1] += sample_right * level * right;

            self.position += self.step;
            if self.fade > 0 {
                self.fade -= 1;
                if self.fade == 0 {
                    return true;
                }
            }
            if self.position >= self.region.frames as f64 {
                return true;
            }
        }
        false
    }
}

#[inline]
fn stereo_gains(gain: f32, pan: f32) -> (f32, f32) {
    let (left, right) = pan_gains(pan);
    (gain * left, gain * right)
}

pub(crate) struct VoicePool {
    voices: Box<[Voice]>,
    sample_rate: f64,
    fade_frames: u32,
}

impl VoicePool {
    pub fn new(sample_rate: u32) -> Self {
        let sample_rate = f64::from(sample_rate);
        Self {
            voices: (0..MAX_VOICES + FADING_SLOTS)
                .map(|_| Voice::idle())
                .collect(),
            sample_rate,
            fade_frames: ((FADE_SECONDS * sample_rate).round() as u32).max(1),
        }
    }

    /// Voices sounding right now, fading ones included.
    pub fn active(&self) -> u32 {
        self.voices.iter().filter(|voice| voice.active).count() as u32
    }

    /// Hands back samples that stopped voices could not return earlier.
    pub fn sweep(&mut self, plan: &Plan, garbage: &mut Producer<Garbage>) {
        for index in 0..self.voices.len() {
            if !self.voices[index].active && self.voices[index].sample.is_some() {
                self.end(index, plan, garbage);
            }
        }
    }

    /// Starts a note on frame `now`, applying the channel's cut rules and
    /// stealing a voice if the pool is full.
    pub fn start(&mut self, plan: &Plan, garbage: &mut Producer<Garbage>, note: Note, now: u64) {
        let channel = &plan.channels[note.channel];
        let sampler = &channel.sampler;
        let Some(sample) = &sampler.sample else {
            return;
        };

        // Notes that start on the same frame are a chord, not a retrigger,
        // so only voices from before this frame are cut.
        let fade_frames = self.fade_frames;
        for voice in self
            .voices
            .iter_mut()
            .filter(|voice| voice.active && voice.origin != Origin::Preview && voice.started < now)
        {
            let same_channel = voice.channel == channel.id;
            let cut_by_self = sampler.cut_self && same_channel;
            let cut_by_group =
                sampler.cut_group != 0 && voice.cut_group == sampler.cut_group && !same_channel;
            if (cut_by_self || cut_by_group) && voice.fade == 0 {
                voice.fade = fade_frames;
            }
        }

        self.make_room();
        let Some(slot) = self.free_slot(plan, garbage) else {
            return;
        };
        let pitch = 2.0_f64.powf(f64::from(f32::from(note.key) + sampler.key_offset) / 12.0);
        self.voices[slot] = Voice {
            active: true,
            sample: Some(sample.clone()),
            origin: note.origin,
            channel: channel.id,
            route: Route::Channel(note.channel),
            key: note.key,
            cut_group: sampler.cut_group,
            started: now,
            release_at: note.release_at,
            region: Region {
                first: sampler.start,
                frames: sampler.end - sampler.start,
                reverse: sampler.reverse,
            },
            position: 0.0,
            step: pitch * f64::from(sample.sample_rate()) / self.sample_rate,
            gain: note.velocity * sampler.gain,
            pan: note.pan,
            envelope: sampler
                .envelope
                .as_ref()
                .map(|envelope| EnvelopeState::new(envelope, self.sample_rate)),
            fade: 0,
        };
    }

    /// Plays a whole buffer straight into the master, replacing any preview
    /// already playing. Gives the buffer back when it cannot be played.
    pub fn preview(
        &mut self,
        plan: &Plan,
        garbage: &mut Producer<Garbage>,
        sample: AudioBuffer,
        now: u64,
    ) -> Option<AudioBuffer> {
        self.fade_origin(Origin::Preview);
        if sample.frames() == 0 {
            return Some(sample);
        }
        let Some(slot) = self.free_slot(plan, garbage) else {
            return Some(sample);
        };
        self.voices[slot] = Voice {
            active: true,
            origin: Origin::Preview,
            route: Route::Master,
            started: now,
            region: Region {
                first: 0,
                frames: sample.frames(),
                reverse: false,
            },
            step: f64::from(sample.sample_rate()) / self.sample_rate,
            gain: 1.0,
            sample: Some(sample),
            ..Voice::idle()
        };
        None
    }

    /// Ends a note played by hand. Samplers without an envelope play on to
    /// the end of the sample.
    pub fn release_live(&mut self, channel: ChannelId, key: u8) {
        for voice in self.voices.iter_mut().filter(|voice| {
            voice.active && voice.origin == Origin::Live && voice.channel == channel
        }) {
            if voice.key == key {
                voice.release_at = 0;
            }
        }
    }

    /// Fades out every voice with this origin.
    pub fn fade_origin(&mut self, origin: Origin) {
        let fade_frames = self.fade_frames;
        for voice in &mut self.voices {
            if voice.active && voice.origin == origin && voice.fade == 0 {
                voice.fade = fade_frames;
            }
        }
    }

    /// Points every voice at its channel's place in a new plan. Voices of
    /// channels the new plan no longer has fade out with the gain and pan
    /// their channel had in `old_state` on frame `now`.
    pub fn rebind(&mut self, plan: &Plan, old_state: &PlanState, now: u64) {
        let fade_frames = self.fade_frames;
        for voice in self.voices.iter_mut().filter(|voice| voice.active) {
            let Route::Channel(old_index) = voice.route else {
                continue;
            };
            match plan.channel_ids.get(voice.channel.0) {
                Some(index) => voice.route = Route::Channel(index),
                None => {
                    let strip = &old_state.channels[old_index];
                    voice.route = Route::Detached {
                        gain: strip.gain.at(now),
                        pan: strip.pan.at(now),
                    };
                    if voice.fade == 0 {
                        voice.fade = fade_frames;
                    }
                }
            }
        }
    }

    /// Adds every voice's output to its mixer track, for the frame offsets
    /// `frames` of the block that starts on frame `base`.
    pub fn render(
        &mut self,
        plan: &Plan,
        state: &PlanState,
        garbage: &mut Producer<Garbage>,
        mixer: &mut Mixer,
        base: u64,
        frames: Range<usize>,
    ) {
        let start = base + frames.start as u64;
        for index in 0..self.voices.len() {
            let voice = &mut self.voices[index];
            if !voice.active {
                continue;
            }
            let (mix, track) = match voice.route {
                Route::Channel(channel) => (state.channels[channel], plan.channels[channel].track),
                Route::Detached { gain, pan } => (Strip::at_rest(gain, pan), 0),
                Route::Master => (Strip::at_rest(1.0, 0.0), 0),
            };
            let out = &mut mixer.track_mut(track)[frames.clone()];
            if voice.render(out, mix, start, self.fade_frames) {
                self.end(index, plan, garbage);
            }
        }
    }

    /// Keeps the number of voices at full level within [`MAX_VOICES`] by
    /// fading one out when a new voice is about to join: the quietest voice
    /// already in its release if there is one, otherwise the oldest.
    fn make_room(&mut self) {
        let sounding =
            |voice: &&mut Voice| voice.active && voice.fade == 0 && voice.origin != Origin::Preview;
        if self.voices.iter_mut().filter(sounding).count() < MAX_VOICES {
            return;
        }
        let release_level = |voice: &Voice| {
            voice
                .envelope
                .filter(EnvelopeState::releasing)
                .map(|envelope| envelope.level)
        };
        let mut victim: Option<&mut Voice> = None;
        for voice in self.voices.iter_mut().filter(sounding) {
            let better = match &victim {
                None => true,
                Some(best) => match (release_level(voice), release_level(best)) {
                    (Some(level), Some(best_level)) => level < best_level,
                    (Some(_), None) => true,
                    (None, Some(_)) => false,
                    (None, None) => voice.started < best.started,
                },
            };
            if better {
                victim = Some(voice);
            }
        }
        if let Some(voice) = victim {
            voice.fade = self.fade_frames;
        }
    }

    /// A slot a new voice can take. When every slot is busy the pool is full
    /// of fading voices, and the one nearest silence is ended to make room.
    fn free_slot(&mut self, plan: &Plan, garbage: &mut Producer<Garbage>) -> Option<usize> {
        let free = |voice: &Voice| !voice.active && voice.sample.is_none();
        if let Some(index) = self.voices.iter().position(free) {
            return Some(index);
        }
        let index = self
            .voices
            .iter()
            .enumerate()
            .filter(|(_, voice)| voice.active && voice.fade > 0)
            .min_by_key(|(_, voice)| voice.fade)
            .map(|(index, _)| index)?;
        self.end(index, plan, garbage);
        free(&self.voices[index]).then_some(index)
    }

    /// Stops a voice and lets go of its sample without freeing memory here.
    fn end(&mut self, index: usize, plan: &Plan, garbage: &mut Producer<Garbage>) {
        let voice = &mut self.voices[index];
        voice.active = false;
        let Some(sample) = voice.sample.take() else {
            return;
        };
        if plan.holds(&sample) {
            // The plan keeps the audio alive, so this drop frees nothing.
            drop(sample);
        } else if garbage.is_full() {
            voice.sample = Some(sample);
        } else {
            retire(garbage, Garbage::Sample(sample));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolation_passes_through_the_stored_points() {
        assert_eq!(hermite(0.3, 0.7, -0.2, 0.9, 0.0), 0.7);
        assert!((hermite(0.3, 0.7, -0.2, 0.9, 1.0) + 0.2).abs() < 1e-6);
        // A straight line stays a straight line.
        assert!((hermite(0.0, 1.0, 2.0, 3.0, 0.25) - 1.25).abs() < 1e-6);
    }

    #[test]
    fn a_reversed_region_reads_backwards() {
        let sample = AudioBuffer::from_interleaved(48_000, 1, vec![1.0, 2.0, 3.0, 4.0]);
        let region = Region {
            first: 1,
            frames: 2,
            reverse: true,
        };
        assert_eq!(region.read(&sample, 0.0), (3.0, 3.0));
        assert_eq!(region.read(&sample, 1.0), (2.0, 2.0));
        assert_eq!(region.frame(&sample, 2), (0.0, 0.0));
    }

    #[test]
    fn the_envelope_walks_through_its_stages() {
        let envelope = Envelope {
            attack_ms: 1.0,
            decay_ms: 1.0,
            sustain: 0.5,
            release_ms: 2.0,
        };
        // 1000 frames a second makes one frame one millisecond.
        let mut state = EnvelopeState::new(&envelope, 1000.0);
        assert_eq!(state.next(), Some(0.0));
        assert_eq!(state.next(), Some(1.0));
        assert_eq!(state.next(), Some(0.5));
        assert_eq!(state.next(), Some(0.5));
        state.release();
        assert_eq!(state.next(), Some(0.5));
        assert_eq!(state.next(), Some(0.25));
        assert_eq!(state.next(), None);
    }
}
