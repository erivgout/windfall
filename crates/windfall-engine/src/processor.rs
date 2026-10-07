//! The whole audio path with no device attached.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use rtrb::{Consumer, Producer};
use windfall_core::AudioBuffer;
use windfall_project::TrackId;

use crate::automation::{self, GRID, Look};
use crate::clips::ClipPlayer;
use crate::controller::Controller;
use crate::message::{GARBAGE_HEADROOM, Garbage, Message, retire};
use crate::mixer::{Frame, MAX_BLOCK, Mixer};
use crate::plan::Plan;
use crate::ramp::Ramp;
use crate::sequencer::{Cursor, Fire, Sequencer, Triggers};
use crate::shared::Shared;
use crate::state::{Fades, PlanState};
use crate::stems::Taps;
use crate::voice::{Note, Origin, Stretch, VoicePool};

/// Turns the current plan into audio: sequencer, sampler voices,
/// instruments, effects and mixer.
///
/// The device callback, the offline renderer and the tests all drive the
/// same `process`, so an export is the same audio as playback.
///
/// `process` is safe to call from a realtime thread. It never allocates,
/// frees, locks or blocks: every buffer is allocated in [`Processor::new`],
/// and whatever arrives from the [`Controller`] owning heap memory is sent
/// back to it to be dropped.
pub struct Processor {
    sample_rate: u32,
    /// Frames processed so far. It is the engine's clock.
    frame: u64,
    plan: Arc<Plan>,
    /// The moving values that belong to `plan`. Always built for it.
    state: Box<PlanState>,
    sequencer: Sequencer,
    voices: VoicePool,
    /// The audio clips of the playlist that are sounding.
    clips: ClipPlayer,
    mixer: Mixer,
    triggers: Triggers,
    /// Gain applied after the master track and its meter.
    output_gain: Ramp,
    fades: Fades,
    /// Sequence number of the last play or stop request handled.
    transport_sequence: u32,
    hardware_epoch: u64,
    /// Automated values were published the last time there were any to
    /// publish, so their going away has to be published once as well.
    reported: bool,
    /// Playback has begun or been moved since the automation was last
    /// looked at.
    landed: bool,
    /// The place in the song at which a song that has stopped keeps its
    /// automation, for as long as it is ringing out. See
    /// [`Processor::automate`].
    hold: Option<f64>,
    /// The offline renderer is what plays: nobody will use the transport
    /// again, so a hold lasts for good.
    hold_for_good: bool,
    /// The tracks a stem render listens to, each on its own. `None`
    /// whenever anything else runs the processor.
    taps: Option<Taps>,
    messages: Consumer<Message>,
    garbage: Producer<Garbage>,
    shared: Arc<Shared>,
}

impl Processor {
    /// Creates a processor running at `sample_rate` and the controller that
    /// drives it. The device layer does the same when it opens a stream;
    /// call this directly to run the engine without a device.
    pub fn new(sample_rate: u32) -> (Processor, Controller) {
        let controller = Controller::new();
        let processor = controller.attach(sample_rate);
        (processor, controller)
    }

    /// A processor that starts out on `plan`, with `state` built for it at
    /// this sample rate, and with the output at `output_gain`. All of it is
    /// simply there from the first frame: nothing has sounded yet, so there
    /// is nothing to glide or fade in from.
    pub(crate) fn with_queues(
        sample_rate: u32,
        plan: Arc<Plan>,
        state: Box<PlanState>,
        output_gain: f32,
        messages: Consumer<Message>,
        garbage: Producer<Garbage>,
        shared: Arc<Shared>,
    ) -> Self {
        let sample_rate = sample_rate.max(1);
        Self {
            sample_rate,
            frame: 0,
            state,
            sequencer: Sequencer::new(sample_rate, &plan),
            plan,
            voices: VoicePool::new(sample_rate),
            clips: ClipPlayer::new(sample_rate),
            mixer: Mixer::new(),
            triggers: Triggers::new(),
            output_gain: Ramp::at_rest(output_gain),
            fades: Fades::at(sample_rate),
            transport_sequence: 0,
            hardware_epoch: shared.hardware_epoch.load(Ordering::Acquire),
            reported: false,
            landed: false,
            hold: None,
            hold_for_good: false,
            taps: None,
            messages,
            garbage,
            shared,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Fills `out`, interleaved stereo, with the next `out.len() / 2` frames.
    ///
    /// Requests from the controller take effect at the start of the call.
    /// The audio does not depend on how the output is divided into calls,
    /// only on which frame each request arrives at.
    pub fn process(&mut self, out: &mut [f32]) {
        self.plugin_control_boundary();
        self.voices.sweep(&self.plan, &mut self.garbage);
        self.clips.sweep(&self.plan, &mut self.garbage);
        self.handle_messages();
        if let Some(taps) = &mut self.taps {
            taps.rewind();
        }
        let (frames, stray) = out.as_chunks_mut::<2>();
        stray.fill(0.0);
        let mut rest = frames;
        while !rest.is_empty() {
            let (block, later) = rest.split_at_mut(self.next_block(rest.len()));
            self.process_block(block);
            rest = later;
        }
        self.publish_automation();
        self.shared
            .publish_transport(self.transport_sequence, self.sequencer.playing());
        // The playhead is where the sound now leaving the engine was in the
        // song, which is behind what is being worked out by the latency of
        // the instruments and effects.
        self.shared.publish_position(
            self.sequencer
                .tick_heard(&self.plan, self.frame, self.state.latency),
            self.sequencer.start(),
            self.voices.active() + self.state.instrument_voices(),
        );
        let missing = self.clips.missing(self.sequencer.clock(), self.frame);
        self.shared
            .publish_clips(self.clips.playing() as u32, missing as u32);
    }

    /// Has every `process` from now on copy what leaves each of these
    /// tracks, past its fader, to where [`Processor::taps`] shows it. A
    /// call may then ask for no more frames than the taps have room for.
    pub(crate) fn listen(&mut self, taps: Taps) {
        self.taps = Some(taps);
    }

    /// What the last `process` copied out of the tracks listened to.
    pub(crate) fn taps(&self) -> Option<&Taps> {
        self.taps.as_ref()
    }

    /// Where a track is in the plan, and by how many frames what leaves it
    /// lags the notes that make it.
    pub(crate) fn track_place(&self, id: TrackId) -> Option<(usize, usize)> {
        let index = self.plan.track_ids.get(id.0)?;
        Some((index, self.state.behind[index]))
    }

    /// Audio clips that were to start and did not, because as many as can
    /// play at once were playing already. The offline renderer reports it.
    pub(crate) fn clips_left_out(&self) -> u32 {
        self.clips.refused()
    }

    /// True when nothing is sounding and nothing can sound again until a
    /// note starts: every voice and audio clip has ended, every instrument
    /// has finished, and every effect and delay has had silence coming in
    /// for as long as it can hold on to sound. The offline renderer asks
    /// this to know when a tail is over.
    pub(crate) fn settled(&self) -> bool {
        self.voices.active() == 0
            && self.clips.active() == 0
            && self.state.settled(&self.plan, self.frame)
    }

    fn handle_messages(&mut self) {
        self.sync_hardware_epoch();
        // Handling a message can retire values, and those must have
        // somewhere to go.
        let mut handled = false;
        while self.garbage.slots() >= GARBAGE_HEADROOM {
            let Ok(message) = self.messages.pop() else {
                break;
            };
            self.handle(message);
            handled = true;
        }
        // The transport may have started, stopped or moved, and a new plan
        // may have other curves: the automation is looked at right away,
        // not at the next step of its grid.
        if handled {
            self.automate();
        }
    }

    fn sync_hardware_epoch(&mut self) {
        let epoch = self.shared.hardware_epoch.load(Ordering::Acquire);
        if epoch != self.hardware_epoch {
            self.voices.fade_origin(Origin::Hardware);
            for unit in self.state.instrument_units() {
                unit.silence_hardware();
            }
            self.hardware_epoch = epoch;
        }
    }

    /// How many frames to process next, out of `left` to do. While
    /// automation is at work this is up to the next step of its grid, and
    /// the automation is looked at when a step has come. The grid is
    /// counted in frames of the engine's clock, so where it falls does not
    /// depend on how the output is divided into calls.
    fn next_block(&mut self, left: usize) -> usize {
        let most = left.min(MAX_BLOCK);
        let at_work = self.state.engaged > 0 || self.state.returning > 0;
        let automating = !self.plan.lanes.is_empty() && (self.sequencer.in_song() || at_work);
        if !automating {
            return most;
        }
        let into = self.frame % GRID;
        if into == 0 {
            self.automate();
        }
        most.min((GRID - into) as usize)
    }

    /// Looks at the automation: sets every automated target on its way to
    /// the value it is to have at the next step of the grid, and tells the
    /// effects and instruments the tempo the song has there.
    ///
    /// While the song plays, that is the value of the place in the song.
    /// A song that has stopped, by itself at its end or because it was
    /// told to, leaves its targets where they were for as long as anything
    /// is still sounding: a tail must not be heard through a fader that has
    /// gone back to its stored value. The hold ends when everything has
    /// rung out, or sooner when the transport is used or something is
    /// played by hand, and the targets then return to their stored values.
    fn automate(&mut self) {
        if self.plan.lanes.is_empty() {
            self.hold = None;
            return;
        }
        let now = self.frame;
        let playing = self.sequencer.playing();
        if self.hold.is_some() && (playing || (!self.hold_for_good && self.settled())) {
            self.hold = None;
        }
        let landed = std::mem::take(&mut self.landed);
        let until = (now / GRID + 1) * GRID;
        let ahead = self.sequencer.song_tick_at(&self.plan, until);
        let look = Look {
            now,
            until,
            tick: ahead.or(self.hold),
            landed: self
                .sequencer
                .song_tick_at(&self.plan, now)
                .filter(|_| landed),
            // With the song playing on, what starts next must find the
            // target where it belongs. With the transport stopped there is
            // only what still rings to hear the change.
            back: if playing {
                self.fades.ramp
            } else {
                self.fades.restore
            },
        };
        let tick = look.tick;
        automation::step(&self.plan, &mut self.state, look);
        if self.state.engaged == 0 {
            self.hold = None;
        }
        let tempo = match (tick, &self.plan.tempo_map) {
            (Some(tick), Some(map)) => map.tempo_at(tick),
            _ => self.plan.tempo_bpm,
        };
        self.state.tell_tempo(tempo);
    }

    /// Publishes the value of every automation that has its target in
    /// hand, for the UI to show the control moving.
    fn publish_automation(&mut self) {
        if self.state.engaged == 0 && !self.reported {
            return;
        }
        let lanes = self.state.lanes.iter().filter(|lane| lane.engaged);
        self.shared
            .publish_automated(lanes.map(|lane| (lane.automation, lane.value)));
        self.reported = self.state.engaged > 0;
    }

    fn handle(&mut self, message: Message) {
        // A stale hardware note must not even release a stopped song's
        // automation hold. Discard it before any transport/tail side effects.
        if let Message::HardwareNote { epoch, .. } = &message {
            self.sync_hardware_epoch();
            if *epoch != self.hardware_epoch {
                return;
            }
        }
        let now = self.frame;
        // Where the song is, in case this is what stops it.
        let song_at = self.sequencer.song_tick_at(&self.plan, now);
        // Using the transport, or playing something by hand, ends the wait
        // for a stopped song to ring out: what comes next is to be heard
        // at the stored values.
        let lets_go = match &message {
            Message::Play { .. }
            | Message::Stop { .. }
            | Message::Seek(_)
            | Message::NoteOn { .. }
            | Message::HardwareNote {
                velocity: 1..=127, ..
            }
            | Message::Preview(_) => true,
            Message::SetTransport { mode, .. } => *mode != self.sequencer.mode(),
            Message::SetPlan { .. }
            | Message::NoteOff { .. }
            | Message::HardwareNote { .. }
            | Message::StopPreview
            | Message::SetOutputGain(_) => false,
        };
        if lets_go {
            self.hold = None;
        }
        match message {
            Message::SetPlan { plan, state } => self.adopt(plan, state),
            Message::Play {
                sequence,
                passes,
                from,
            } => {
                self.transport_sequence = sequence;
                self.hold_for_good = passes.is_some();
                self.sequencer.play(&self.plan, now, passes, from);
            }
            Message::Stop { sequence } => {
                self.transport_sequence = sequence;
                self.sequencer.stop();
                self.clips.release_all();
                self.voices.fade_origin(Origin::Sequenced);
                self.voices.fade_origin(Origin::Live);
                self.voices.fade_origin(Origin::Hardware);
                for unit in self.state.instrument_units() {
                    unit.silence();
                }
            }
            Message::Seek(tick) => {
                if self.sequencer.seek(tick, &self.plan, now) {
                    self.end_sequenced();
                }
            }
            Message::SetTransport {
                mode,
                pattern,
                loop_song,
            } => {
                if self
                    .sequencer
                    .set_transport(mode, pattern, loop_song, &self.plan, now)
                {
                    self.end_sequenced();
                }
            }
            Message::NoteOn {
                channel,
                key,
                velocity,
            } => {
                if let Some(channel) = self.plan.channel(channel) {
                    let note = Note {
                        channel,
                        key,
                        velocity,
                        pan: 0.0,
                        end: f64::INFINITY,
                        origin: Origin::Live,
                    };
                    self.start(note, now);
                }
            }
            Message::NoteOff { channel, key } => {
                self.voices.release_live(channel, key);
                let index = self.plan.channel(channel);
                if let Some(unit) = index.and_then(|index| self.state.instrument(index)) {
                    unit.release_live(key);
                }
            }
            Message::HardwareNote {
                epoch,
                channel,
                key,
                velocity,
            } => {
                self.sync_hardware_epoch();
                if epoch == self.hardware_epoch {
                    if velocity == 0 {
                        self.voices.release_origin(channel, key, Origin::Hardware);
                        if let Some(unit) = self
                            .plan
                            .channel(channel)
                            .and_then(|i| self.state.instrument(i))
                        {
                            unit.release_hardware(key);
                        }
                    } else if let Some(channel) = self.plan.channel(channel) {
                        self.start(
                            Note {
                                channel,
                                key,
                                velocity: f32::from(velocity) / 127.0,
                                pan: 0.0,
                                end: f64::INFINITY,
                                origin: Origin::Hardware,
                            },
                            now,
                        );
                    }
                }
            }
            Message::Preview(sample) => self.preview(sample),
            Message::StopPreview => self.voices.fade_origin(Origin::Preview),
            Message::SetOutputGain(gain) => self.set_output_gain(gain),
        }
        // Whatever moved the playhead moved the clock under the notes that
        // are still sounding.
        let shift = self.sequencer.take_clock_shift();
        if shift != 0.0 {
            self.voices.shift_ends(shift);
            for unit in self.state.instrument_units() {
                unit.shift_ends(shift);
            }
        }
        if self.sequencer.take_jump() {
            self.landed = true;
            self.clips.release_all();
            self.chase_clips();
        }
        if let Some(tick) = song_at {
            self.hold_if_stopped(tick);
        }
    }

    /// Keeps the automation at `tick` of the song, where the song was a
    /// moment ago, if the transport has stopped since and a target is in
    /// automation's hands. [`Processor::automate`] ends the hold.
    fn hold_if_stopped(&mut self, tick: f64) {
        if !self.sequencer.playing() && self.state.engaged > 0 {
            self.hold = Some(tick);
        }
    }

    /// Starts the audio clips the playhead is inside of and that are not
    /// sounding, part way through: playback has begun or been moved there,
    /// or a new plan has put a clip there.
    fn chase_clips(&mut self) {
        if !self.sequencer.in_song() {
            return;
        }
        let now = self.frame;
        self.clips.chase(
            &self.plan,
            &mut self.garbage,
            self.sequencer.clock(),
            self.sequencer.pass_start(),
            self.sequencer.tick(&self.plan, now),
            now,
        );
    }

    /// Starts a note on frame `now`: on its channel's instrument if it has
    /// one, and as a sampler voice otherwise. An instrument places its own
    /// voices, so the note's pan is not used there.
    fn start(&mut self, note: Note, now: u64) {
        match self.state.instrument(note.channel) {
            Some(unit) => {
                let live = note.origin != Origin::Sequenced;
                unit.note_on(note.key, note.velocity, note.end, live);
                if note.origin == Origin::Hardware {
                    unit.mark_hardware(note.key);
                }
            }
            None => self.voices.start(&self.plan, &mut self.garbage, note, now),
        }
    }

    /// Ends what the sequencer started, because playback jumped away from
    /// under it. Notes played by hand carry on.
    fn end_sequenced(&mut self) {
        self.voices.fade_origin(Origin::Sequenced);
        for unit in self.state.instrument_units() {
            unit.end_sequenced();
        }
    }

    /// Switches to a new plan between two blocks. Sounding voices carry on,
    /// effects and instruments move across with all they remember, and
    /// whatever sound is passing through changes gently. Voices already
    /// fading out step aside first where they can, so they hold no gain
    /// back from changing at once.
    fn adopt(&mut self, plan: Arc<Plan>, mut state: Box<PlanState>) {
        let now = self.frame;
        self.voices.rebind(&plan, &self.plan, &self.state, now);
        self.clips.rebind(&plan, now, self.fades.ramp);
        let sounding = self.voices.destinations();
        state.take_over(
            &plan,
            &self.plan,
            &mut self.state,
            sounding.chain(self.clips.destinations()),
            now,
            self.fades,
        );
        if self.sequencer.set_plan(&plan, &self.plan, now) {
            // The tempo map changed under the song: every clock tick now
            // means another place in it, the ends of sounding notes too.
            let origin = self.sequencer.pass_start();
            let moved = |end: f64| Sequencer::moved(&plan, &self.plan, origin, end);
            self.voices.move_ends(moved);
            for unit in state.instrument_units() {
                unit.move_ends(moved);
            }
        }
        let old_plan = std::mem::replace(&mut self.plan, plan);
        let old_state = std::mem::replace(&mut self.state, state);
        retire(&mut self.garbage, Garbage::Plan(old_plan));
        retire(&mut self.garbage, Garbage::State(old_state));
        // A song that got shorter can have moved the playhead back into
        // it. Either way a clip may now lie under the playhead that was
        // not there, or not like this, a moment ago.
        if self.sequencer.take_jump() {
            self.landed = true;
            self.clips.release_all();
        }
        self.chase_clips();
    }

    fn preview(&mut self, sample: AudioBuffer) {
        let unplayed = self
            .voices
            .preview(&self.plan, &mut self.garbage, sample, self.frame);
        if let Some(sample) = unplayed {
            retire(&mut self.garbage, Garbage::Sample(sample));
        }
    }

    /// Glides the output to a new gain, or sets it outright while nothing
    /// is sounding and a glide would only reach into whatever starts next.
    fn set_output_gain(&mut self, gain: f32) {
        if self.settled() {
            self.output_gain = Ramp::at_rest(gain);
        } else {
            self.output_gain.retarget(gain, self.frame, self.fades.ramp);
        }
    }

    fn plugin_control_boundary(&mut self) {
        for chain in &mut self.state.chains {
            for unit in chain.iter_mut().flatten() {
                unit.plugin_control_boundary();
            }
        }
        for seat in self.state.instruments.iter_mut().flatten() {
            if let Some(unit) = &mut seat.unit {
                unit.plugin_control_boundary();
            }
        }
    }

    /// Processes up to [`MAX_BLOCK`] frames.
    fn process_block(&mut self, out: &mut [Frame]) {
        self.plugin_control_boundary();
        let tick = self.sequencer.tick(&self.plan, self.frame);
        let warped = self
            .plan
            .tempo_map
            .as_ref()
            .map_or(tick, |map| map.warp(tick));
        let transport = crate::plugins::PluginTransport {
            playing: self.sequencer.playing(),
            tempo_bpm: self.state.tempo(),
            position_beats: tick / windfall_core::PPQ as f64,
            position_seconds: warped * 60.0 / (self.plan.tempo_bpm * windfall_core::PPQ as f64),
            numerator: self.plan.signature.numerator as u16,
            denominator: self.plan.signature.denominator as u16,
        };
        for (index, chain) in self.state.chains.iter_mut().enumerate() {
            for (place, unit) in chain.iter_mut().enumerate() {
                if !self.plan.tracks[index].effects[place].leaving
                    && let Some(unit) = unit
                {
                    unit.plugin_transport(transport);
                }
            }
        }
        for (index, seat) in self.state.instruments.iter_mut().enumerate() {
            if !self.plan.channels[index].leaving
                && let Some(seat) = seat
                && let Some(unit) = &mut seat.unit
            {
                unit.plugin_transport(transport);
            }
        }
        let frames = out.len();
        let base = self.frame;
        let end = base + frames as u64;
        self.mixer.clear(self.plan.tracks.len(), frames);

        // Voices and instruments are rendered up to each note's frame
        // before the note starts, so everything a note-on does, cutting
        // other voices included, happens on its exact frame. The notes
        // arrive in rounds
        // of as many as the trigger buffer holds, in playing order, so a
        // block with more notes than that plays every one of them just the
        // same.
        let mut rendered = 0;
        let mut cursor = Cursor::at(base);
        let in_song = self.sequencer.in_song();
        loop {
            let next = self
                .sequencer
                .collect(&self.plan, cursor, end, &mut self.triggers);
            for index in 0..self.triggers.len() {
                let trigger = self.triggers.get(index);
                let offset = (trigger.frame - base) as usize;
                self.render_voices(base, rendered, offset);
                rendered = offset;
                match trigger.what {
                    Fire::Note {
                        channel,
                        key,
                        velocity,
                        pan,
                        end,
                    } => {
                        let note = Note {
                            channel,
                            key,
                            velocity,
                            pan,
                            end,
                            origin: Origin::Sequenced,
                        };
                        self.start(note, trigger.frame);
                    }
                    Fire::Clip { clip, origin } => {
                        let garbage = &mut self.garbage;
                        self.clips
                            .start(&self.plan, garbage, clip, origin, 0.0, trigger.frame);
                    }
                }
            }
            let Some(next) = next else {
                break;
            };
            self.sequencer.advance(&self.plan, cursor.frame, next.frame);
            cursor = next;
        }
        self.sequencer.advance(&self.plan, cursor.frame, end);
        if in_song {
            // A song that ran out in this block left off at its end.
            self.hold_if_stopped(f64::from(self.plan.song_end));
        }
        self.render_voices(base, rendered, frames);

        self.mixer
            .mix(&self.plan, &mut self.state, &self.shared, base, out);
        if let Some(taps) = &mut self.taps {
            taps.collect(&mut self.mixer, frames);
        }
        self.finish_output(base, out);
        self.frame = end;
    }

    fn render_voices(&mut self, base: u64, from: usize, to: usize) {
        if from < to {
            let clock = self.sequencer.clock();
            self.voices.render(
                &self.plan,
                &self.state,
                clock,
                &mut self.garbage,
                &mut self.mixer,
                Stretch {
                    base,
                    frames: from..to,
                },
            );
            self.state.render_instruments(clock, base, from..to);
            self.clips.render(
                &self.plan,
                clock,
                &mut self.garbage,
                &mut self.mixer,
                Stretch {
                    base,
                    frames: from..to,
                },
            );
        }
    }

    /// Applies the output gain and keeps anything that is not a number away
    /// from the device.
    fn finish_output(&self, base: u64, out: &mut [Frame]) {
        let steady = self.output_gain.settled(base);
        let mut gain = self.output_gain.at(base);
        for (offset, frame) in out.iter_mut().enumerate() {
            if !steady {
                gain = self.output_gain.at(base + offset as u64);
            }
            for sample in frame {
                *sample = if sample.is_finite() {
                    *sample * gain
                } else {
                    0.0
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windfall_project::ChannelId;

    #[test]
    fn stale_hardware_notes_preserve_stopped_automation_hold() {
        let (mut processor, controller) = Processor::new(48_000);
        processor.hold = Some(960.0);
        let epoch = controller.hardware_epoch();
        controller.panic_hardware();

        processor.handle(Message::HardwareNote {
            epoch,
            channel: ChannelId(123),
            key: 60,
            velocity: 127,
        });

        assert_eq!(processor.hold, Some(960.0));
    }
}
