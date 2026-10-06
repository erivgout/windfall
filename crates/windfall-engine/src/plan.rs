//! The compiled form of a project that the audio thread plays.
//!
//! [`compile`] runs on the control side. It resolves every id to an index,
//! applies swing, sorts notes by time, orders the mixer tracks and works out
//! what mute and solo leave audible, so the audio thread only ever follows
//! indices. A [`Plan`] is immutable once built. The values that move while
//! it plays, the gain and pan ramps, live in a [`PlanState`] sized to match.

use std::collections::HashSet;

use windfall_core::{AudioBuffer, TICKS_PER_STEP};
use windfall_project::{
    Channel, ChannelId, ChannelSource, ClipContent, DEFAULT_PATTERN_STEPS, Envelope, MAX_GAIN,
    MAX_KEY, MAX_MIXER_TRACKS, MAX_TEMPO_BPM, MIN_TEMPO_BPM, Mixer, MixerTrack, Pattern, PatternId,
    Playlist, Project, SamplerSettings, TrackId,
};

use crate::pool::SamplePool;
use crate::ramp::Ramp;

/// Tempo used when a project carries one that is not a number.
const FALLBACK_TEMPO_BPM: f64 = 120.0;

/// Two sixteenth-note steps: the unit swing works on.
const PAIR_TICKS: u32 = 2 * TICKS_PER_STEP;

/// How far full swing delays the second step of a pair. It lands two thirds
/// of the way through the pair, which is a triplet feel.
const FULL_SWING_DELAY_TICKS: f64 = TICKS_PER_STEP as f64 / 3.0;

/// Length a pattern-mode loop falls back to when the plan has no pattern.
pub(crate) const FALLBACK_LOOP_TICKS: u32 = DEFAULT_PATTERN_STEPS * TICKS_PER_STEP;

/// Maps ids to positions in one of the plan's lists.
#[derive(Debug, Default)]
pub(crate) struct IdIndex {
    /// `(id, position)` sorted by id.
    entries: Vec<(u32, u32)>,
}

impl IdIndex {
    fn new(ids: impl Iterator<Item = u32>) -> Self {
        let mut entries: Vec<(u32, u32)> = ids
            .enumerate()
            .map(|(position, id)| (id, position as u32))
            .collect();
        entries.sort_unstable();
        // Ids are unique in a valid project. If one repeats, the first wins.
        entries.dedup_by_key(|entry| entry.0);
        Self { entries }
    }

    pub fn get(&self, id: u32) -> Option<usize> {
        self.entries
            .binary_search_by_key(&id, |entry| entry.0)
            .ok()
            .map(|found| self.entries[found].1 as usize)
    }
}

#[derive(Debug)]
pub(crate) struct Plan {
    pub tempo_bpm: f64,
    /// Channel rack order.
    pub channels: Vec<PlanChannel>,
    pub channel_ids: IdIndex,
    pub patterns: Vec<PlanPattern>,
    pub pattern_ids: IdIndex,
    /// Mixer order. `tracks[0]` is the master and always exists.
    pub tracks: Vec<PlanTrack>,
    pub track_ids: IdIndex,
    /// Track indices ordered so every track comes after the tracks that feed
    /// it.
    pub order: Vec<usize>,
    /// Number of routing edges, which is the number of edge ramps a
    /// [`PlanState`] holds.
    pub edge_count: usize,
    /// Pattern clips that sound, sorted by start.
    pub clips: Vec<PlanClip>,
    /// End of the last clip on the playlist in ticks, muted clips included.
    pub song_end: u32,
}

#[derive(Debug)]
pub(crate) struct PlanChannel {
    pub id: ChannelId,
    /// Index of the mixer track the channel plays into.
    pub track: usize,
    /// Channel volume, or zero when mute or solo silences the channel.
    pub gain: f32,
    pub pan: f32,
    pub sampler: PlanSampler,
}

#[derive(Debug)]
pub(crate) struct PlanSampler {
    pub sample: Option<AudioBuffer>,
    /// First frame of the region that plays.
    pub start: usize,
    /// One past the last frame of the region. Greater than `start` whenever
    /// there is a sample.
    pub end: usize,
    pub reverse: bool,
    /// Semitones added to a note's key to get its pitch relative to the
    /// recorded pitch: the tuning minus the root key.
    pub key_offset: f32,
    pub gain: f32,
    pub envelope: Option<Envelope>,
    pub cut_self: bool,
    pub cut_group: u8,
}

#[derive(Debug)]
pub(crate) struct PlanPattern {
    pub id: PatternId,
    /// Loop length in ticks, at least one step.
    pub length: u32,
    /// Sorted by `tick`. Every tick is below `length`.
    pub events: Vec<NoteEvent>,
}

/// A note with swing already applied.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NoteEvent {
    /// Start in ticks from the beginning of the pattern.
    pub tick: f64,
    /// Length in ticks, greater than zero.
    pub length: f64,
    /// Index into [`Plan::channels`].
    pub channel: usize,
    pub key: u8,
    pub velocity: f32,
    pub pan: f32,
}

#[derive(Debug)]
pub(crate) struct PlanTrack {
    pub id: TrackId,
    /// Fader gain, or zero when mute or solo silences the track.
    pub gain: f32,
    pub pan: f32,
    /// Where the post-fader signal goes: the output first, then the sends.
    pub edges: Vec<PlanEdge>,
}

#[derive(Debug)]
pub(crate) struct PlanEdge {
    /// Index of the receiving track.
    pub target: usize,
    pub target_id: TrackId,
    /// A send, as opposed to the track's output.
    pub send: bool,
    pub gain: f32,
    /// Index of this edge's ramp in [`PlanState::edges`].
    pub slot: usize,
}

#[derive(Debug)]
pub(crate) struct PlanClip {
    pub start: u32,
    pub end: u32,
    /// How far into the pattern the clip starts, below the pattern length.
    pub offset: u32,
    /// Index into [`Plan::patterns`].
    pub pattern: usize,
    /// Largest `end` among this clip and all clips before it. It lets the
    /// sequencer skip, with one binary search, every clip that ended before
    /// the stretch of time it is looking at.
    pub reach: u32,
}

impl Plan {
    /// What plays before a project is set: nothing, through a master track
    /// at unity so sample previews are still heard.
    pub fn empty() -> Self {
        compile(&Project::new(""), &SamplePool::new())
    }

    /// True when the plan keeps `buffer` alive, which makes dropping another
    /// handle to it free of any deallocation.
    pub fn holds(&self, buffer: &AudioBuffer) -> bool {
        self.channels.iter().any(|channel| {
            channel
                .sampler
                .sample
                .as_ref()
                .is_some_and(|sample| same_audio(sample, buffer))
        })
    }
}

/// True when both handles share one block of sample data.
pub(crate) fn same_audio(a: &AudioBuffer, b: &AudioBuffer) -> bool {
    std::ptr::eq(a.samples(), b.samples())
}

/// The gain and pan ramps of one plan. The control side allocates it and the
/// audio thread carries values over from the plan it replaces.
#[derive(Debug)]
pub(crate) struct PlanState {
    pub channels: Vec<Strip>,
    pub tracks: Vec<Strip>,
    pub edges: Vec<Ramp>,
}

/// The smoothed gain and pan of a channel or mixer track.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Strip {
    pub gain: Ramp,
    pub pan: Ramp,
}

impl Strip {
    pub fn at_rest(gain: f32, pan: f32) -> Self {
        Self {
            gain: Ramp::at_rest(gain),
            pan: Ramp::at_rest(pan),
        }
    }

    fn retarget(&mut self, gain: f32, pan: f32, now: u64, frames: u64) {
        self.gain.retarget(gain, now, frames);
        self.pan.retarget(pan, now, frames);
    }
}

impl PlanState {
    /// Every ramp resting on its target.
    pub fn new(plan: &Plan) -> Self {
        let mut edges = vec![Ramp::at_rest(0.0); plan.edge_count];
        for edge in plan.tracks.iter().flat_map(|track| &track.edges) {
            edges[edge.slot] = Ramp::at_rest(edge.gain);
        }
        Self {
            channels: plan
                .channels
                .iter()
                .map(|channel| Strip::at_rest(channel.gain, channel.pan))
                .collect(),
            tracks: plan
                .tracks
                .iter()
                .map(|track| Strip::at_rest(track.gain, track.pan))
                .collect(),
            edges,
        }
    }

    /// Carries the current values over from the state of the plan being
    /// replaced, matching channels and tracks by id, and starts every value
    /// that changed gliding to its new target. Runs on the audio thread, so
    /// it must not allocate.
    pub fn inherit(
        &mut self,
        plan: &Plan,
        old_plan: &Plan,
        old: &PlanState,
        now: u64,
        frames: u64,
    ) {
        for (channel, strip) in plan.channels.iter().zip(&mut self.channels) {
            if let Some(found) = old_plan.channel_ids.get(channel.id.0) {
                *strip = old.channels[found];
                strip.retarget(channel.gain, channel.pan, now, frames);
            }
        }
        for (track, strip) in plan.tracks.iter().zip(&mut self.tracks) {
            let Some(found) = old_plan.track_ids.get(track.id.0) else {
                continue;
            };
            *strip = old.tracks[found];
            strip.retarget(track.gain, track.pan, now, frames);
            for edge in &track.edges {
                let before = old_plan.tracks[found]
                    .edges
                    .iter()
                    .find(|old_edge| {
                        old_edge.target_id == edge.target_id && old_edge.send == edge.send
                    })
                    .map(|old_edge| old.edges[old_edge.slot]);
                // A route that did not exist a moment ago fades in.
                let mut ramp = before.unwrap_or(Ramp::at_rest(0.0));
                ramp.retarget(edge.gain, now, frames);
                self.edges[edge.slot] = ramp;
            }
        }
    }
}

/// Builds the plan for a project. Samples missing from `pool` leave their
/// channels silent.
///
/// The model's rules are not assumed to hold: values out of range are
/// clamped, and references to things that do not exist are dropped.
pub(crate) fn compile(project: &Project, pool: &SamplePool) -> Plan {
    let tempo_bpm = if project.settings.tempo_bpm.is_finite() {
        project
            .settings
            .tempo_bpm
            .clamp(MIN_TEMPO_BPM, MAX_TEMPO_BPM)
    } else {
        FALLBACK_TEMPO_BPM
    };
    let swing = f64::from(unit(project.settings.swing));

    let (tracks, order) = compile_mixer(&project.mixer);
    let track_ids = IdIndex::new(tracks.iter().map(|track| track.id.0));
    let edge_count = tracks.iter().map(|track| track.edges.len()).sum();

    let any_solo = project.channels.iter().any(|channel| channel.solo);
    let channels: Vec<PlanChannel> = project
        .channels
        .iter()
        .map(|channel| compile_channel(channel, pool, &track_ids, any_solo))
        .collect();
    let channel_ids = IdIndex::new(channels.iter().map(|channel| channel.id.0));

    let patterns: Vec<PlanPattern> = project
        .patterns
        .iter()
        .map(|pattern| compile_pattern(pattern, &channel_ids, swing))
        .collect();
    let pattern_ids = IdIndex::new(patterns.iter().map(|pattern| pattern.id.0));

    let (clips, song_end) = compile_playlist(&project.playlist, &pattern_ids, &patterns);

    Plan {
        tempo_bpm,
        channels,
        channel_ids,
        patterns,
        pattern_ids,
        tracks,
        track_ids,
        order,
        edge_count,
        clips,
        song_end,
    }
}

fn compile_channel(
    channel: &Channel,
    pool: &SamplePool,
    track_ids: &IdIndex,
    any_solo: bool,
) -> PlanChannel {
    let ChannelSource::Sampler(settings) = &channel.source;
    let audible = !channel.muted && (!any_solo || channel.solo);
    PlanChannel {
        id: channel.id,
        // A channel whose track is gone plays into the master.
        track: track_ids.get(channel.mixer_track.0).unwrap_or(0),
        gain: if audible { gain(channel.volume) } else { 0.0 },
        pan: pan(channel.pan),
        sampler: compile_sampler(settings, pool),
    }
}

fn compile_sampler(settings: &SamplerSettings, pool: &SamplePool) -> PlanSampler {
    let sample = settings
        .sample
        .and_then(|id| pool.get(id))
        .filter(|buffer| buffer.frames() > 0)
        .cloned();
    let frames = sample.as_ref().map_or(0, AudioBuffer::frames);
    let start_fraction = f64::from(unit(settings.start));
    let end_fraction = if settings.end.is_finite() {
        f64::from(settings.end.clamp(0.0, 1.0))
    } else {
        1.0
    };
    let start = ((start_fraction * frames as f64).round() as usize).min(frames.saturating_sub(1));
    let end =
        ((end_fraction * frames as f64).round() as usize).clamp((start + 1).min(frames), frames);
    let tune = if settings.tune.is_finite() {
        settings.tune
    } else {
        0.0
    };
    PlanSampler {
        sample,
        start,
        end,
        reverse: settings.reverse,
        key_offset: tune - f32::from(settings.root_key),
        gain: gain(settings.gain),
        envelope: settings.envelope.map(|envelope| Envelope {
            attack_ms: duration(envelope.attack_ms),
            decay_ms: duration(envelope.decay_ms),
            sustain: unit(envelope.sustain),
            release_ms: duration(envelope.release_ms),
        }),
        cut_self: settings.cut_self,
        cut_group: settings.cut_group,
    }
}

fn compile_pattern(pattern: &Pattern, channel_ids: &IdIndex, swing: f64) -> PlanPattern {
    let length = pattern.length_steps.max(1).saturating_mul(TICKS_PER_STEP);
    // A trailing step with no partner is left alone, so swing can never push
    // a note past the end of the pattern.
    let swung_length = f64::from(length / PAIR_TICKS * PAIR_TICKS);
    let warp = |tick: f64| swing_warp(tick, swing, swung_length);

    let mut events = Vec::new();
    for lane in &pattern.lanes {
        let Some(channel) = channel_ids.get(lane.channel.0) else {
            continue;
        };
        for note in lane.notes.iter().filter(|note| note.start < length) {
            let start = f64::from(note.start);
            let tick = warp(start);
            events.push(NoteEvent {
                tick,
                length: warp(start + f64::from(note.length.max(1))) - tick,
                channel,
                key: note.key.min(MAX_KEY),
                velocity: unit(note.velocity),
                pan: pan(note.pan),
            });
        }
    }
    // The sort is stable, so notes on the same tick keep lane order.
    events.sort_by(|a, b| a.tick.total_cmp(&b.tick));
    PlanPattern {
        id: pattern.id,
        length,
        events,
    }
}

/// Moves a tick to where swing puts it.
///
/// Time is stretched inside each pair of steps: the first step grows by the
/// swing delay and the second shrinks by it, so the pair keeps its length, a
/// note on the second step lands late by exactly the delay, and notes between
/// steps keep their order. Ticks at or past `swung_length` are left alone.
fn swing_warp(tick: f64, swing: f64, swung_length: f64) -> f64 {
    if swing <= 0.0 || tick >= swung_length {
        return tick;
    }
    let pair = f64::from(PAIR_TICKS);
    let step = f64::from(TICKS_PER_STEP);
    let delay = swing * FULL_SWING_DELAY_TICKS;
    let pair_start = (tick / pair).floor() * pair;
    let local = tick - pair_start;
    let warped = if local < step {
        local * (step + delay) / step
    } else {
        step + delay + (local - step) * (step - delay) / step
    };
    pair_start + warped
}

fn compile_playlist(
    playlist: &Playlist,
    pattern_ids: &IdIndex,
    patterns: &[PlanPattern],
) -> (Vec<PlanClip>, u32) {
    let muted_tracks: HashSet<_> = playlist
        .tracks
        .iter()
        .filter(|track| track.muted)
        .map(|track| track.id)
        .collect();

    let mut song_end = 0;
    let mut clips = Vec::new();
    for clip in &playlist.clips {
        let end = clip.start.saturating_add(clip.length);
        // A muted clip still takes up room on the timeline, so muting the
        // last clip does not shorten the song.
        song_end = song_end.max(end);
        if clip.muted || muted_tracks.contains(&clip.track) || end == clip.start {
            continue;
        }
        let ClipContent::Pattern { pattern } = &clip.content;
        let Some(pattern) = pattern_ids.get(pattern.0) else {
            continue;
        };
        clips.push(PlanClip {
            start: clip.start,
            end,
            offset: clip.offset % patterns[pattern].length,
            pattern,
            reach: end,
        });
    }
    clips.sort_by_key(|clip| clip.start);
    let mut reach = 0;
    for clip in &mut clips {
        reach = reach.max(clip.end);
        clip.reach = reach;
    }
    (clips, song_end)
}

/// Builds the mixer tracks and the order to process them in.
///
/// Mute and solo are folded into each track's gain. A muted track is always
/// silent. While any track is soloed, a track is heard only if it is soloed
/// itself, feeds a soloed track through outputs and sends, or carries a
/// soloed track's signal on toward the master.
fn compile_mixer(mixer: &Mixer) -> (Vec<PlanTrack>, Vec<usize>) {
    let fallback_master = [MixerTrack {
        id: TrackId::MASTER,
        name: String::new(),
        color: 0,
        volume: 1.0,
        pan: 0.0,
        muted: false,
        solo: false,
        output: None,
        sends: Vec::new(),
    }];
    let source: &[MixerTrack] = if mixer.tracks.is_empty() {
        &fallback_master
    } else {
        &mixer.tracks[..mixer.tracks.len().min(MAX_MIXER_TRACKS)]
    };
    let ids = IdIndex::new(source.iter().map(|track| track.id.0));

    let mut tracks: Vec<PlanTrack> = source
        .iter()
        .enumerate()
        .map(|(index, track)| {
            let edge = |target_id: TrackId, send: bool, gain: f32| {
                let target = ids.get(target_id.0)?;
                // The master is the end of the chain, and nothing feeds itself.
                (index != 0 && target != index).then_some(PlanEdge {
                    target,
                    target_id,
                    send,
                    gain,
                    slot: 0,
                })
            };
            let output = track.output.and_then(|target| edge(target, false, 1.0));
            let sends = track
                .sends
                .iter()
                .filter_map(|send| edge(send.target, true, gain(send.gain)));
            PlanTrack {
                id: track.id,
                gain: gain(track.volume),
                pan: pan(track.pan),
                edges: output.into_iter().chain(sends).collect(),
            }
        })
        .collect();

    let order = route(&mut tracks);

    let soloed: Vec<bool> = source.iter().map(|track| track.solo).collect();
    let heard = solo_set(&tracks, &soloed);
    let mut slot = 0;
    for (index, track) in tracks.iter_mut().enumerate() {
        if source[index].muted || !heard[index] {
            track.gain = 0.0;
        }
        for edge in &mut track.edges {
            edge.slot = slot;
            slot += 1;
        }
    }
    (tracks, order)
}

/// Orders the tracks so each comes after everything that feeds it. The model
/// forbids routing loops; if one turns up anyway it is broken by dropping
/// the edges that close it, so the mixer can never feed back.
fn route(tracks: &mut [PlanTrack]) -> Vec<usize> {
    let count = tracks.len();
    let mut waiting_on = vec![0usize; count];
    for edge in tracks.iter().flat_map(|track| &track.edges) {
        waiting_on[edge.target] += 1;
    }
    let mut placed = vec![false; count];
    let mut order = Vec::with_capacity(count);
    while order.len() < count {
        let ready = (0..count).find(|&index| !placed[index] && waiting_on[index] == 0);
        let next = ready.unwrap_or_else(|| {
            // Every track left is part of a loop or fed by one. Cut the
            // inputs of the last one, which is never the master unless
            // nothing else is left.
            let victim = (0..count).rfind(|&index| !placed[index]).unwrap_or(0);
            for (index, track) in tracks.iter_mut().enumerate() {
                if !placed[index] {
                    track.edges.retain(|edge| edge.target != victim);
                }
            }
            waiting_on[victim] = 0;
            victim
        });
        placed[next] = true;
        order.push(next);
        for edge in &tracks[next].edges {
            waiting_on[edge.target] -= 1;
        }
    }
    order
}

/// Which tracks solo leaves audible. All of them when nothing is soloed.
fn solo_set(tracks: &[PlanTrack], soloed: &[bool]) -> Vec<bool> {
    if !soloed.contains(&true) {
        return vec![true; tracks.len()];
    }
    let mut feeders = vec![Vec::new(); tracks.len()];
    for (index, track) in tracks.iter().enumerate() {
        for edge in &track.edges {
            feeders[edge.target].push(index);
        }
    }
    let spread = |neighbours: &dyn Fn(usize) -> Vec<usize>| {
        let mut reached = soloed.to_vec();
        let mut pending: Vec<usize> = (0..tracks.len()).filter(|&index| soloed[index]).collect();
        while let Some(index) = pending.pop() {
            for next in neighbours(index) {
                if !reached[next] {
                    reached[next] = true;
                    pending.push(next);
                }
            }
        }
        reached
    };
    let downstream = spread(&|index| tracks[index].edges.iter().map(|edge| edge.target).collect());
    let upstream = spread(&|index| feeders[index].clone());
    downstream
        .iter()
        .zip(&upstream)
        .map(|(down, up)| *down || *up)
        .collect()
}

fn gain(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, MAX_GAIN)
    } else {
        0.0
    }
}

fn pan(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

/// Clamps to 0..1, with anything that is not a number becoming 0.
fn unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn duration(ms: f32) -> f32 {
    if ms.is_finite() { ms.max(0.0) } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use windfall_project::{Lane, Note, NoteId, Send};

    use super::*;

    fn track(id: u32, output: Option<u32>, sends: &[u32]) -> MixerTrack {
        MixerTrack {
            id: TrackId(id),
            name: String::new(),
            color: 0,
            volume: 1.0,
            pan: 0.0,
            muted: false,
            solo: false,
            output: output.map(TrackId),
            sends: sends
                .iter()
                .map(|&target| Send {
                    target: TrackId(target),
                    gain: 0.5,
                })
                .collect(),
        }
    }

    fn mixer_gains(tracks: Vec<MixerTrack>) -> Vec<f32> {
        let (tracks, _) = compile_mixer(&Mixer { tracks });
        tracks.iter().map(|track| track.gain).collect()
    }

    #[test]
    fn swing_delays_the_second_step_of_each_pair() {
        let length = 16.0 * 240.0;
        assert_eq!(swing_warp(0.0, 1.0, length), 0.0);
        assert_eq!(swing_warp(240.0, 1.0, length), 320.0);
        assert_eq!(swing_warp(480.0, 1.0, length), 480.0);
        assert_eq!(swing_warp(720.0, 0.5, length), 760.0);
        assert_eq!(swing_warp(240.0, 0.0, length), 240.0);
    }

    #[test]
    fn swing_keeps_notes_in_order_and_inside_the_pair() {
        let length = 480.0;
        let mut last = -1.0;
        for tick in 0..480 {
            let warped = swing_warp(f64::from(tick), 1.0, length);
            assert!(warped > last && warped < 480.0, "tick {tick} -> {warped}");
            last = warped;
        }
    }

    #[test]
    fn an_unpaired_last_step_is_not_swung() {
        let mut project = Project::new("odd");
        project.channels.push(Channel {
            id: ChannelId(10),
            name: String::new(),
            color: 0,
            volume: 1.0,
            pan: 0.0,
            muted: false,
            solo: false,
            mixer_track: TrackId::MASTER,
            source: ChannelSource::Sampler(SamplerSettings::default()),
        });
        project.settings.swing = 1.0;
        project.patterns[0].length_steps = 3;
        project.patterns[0].lanes.push(Lane {
            channel: ChannelId(10),
            notes: [240, 600, 720]
                .iter()
                .enumerate()
                .map(|(index, &start)| Note {
                    id: NoteId(100 + index as u32),
                    start,
                    length: 240,
                    key: 60,
                    velocity: 1.0,
                    pan: 0.0,
                })
                .collect(),
        });
        let plan = compile(&project, &SamplePool::new());
        let ticks: Vec<f64> = plan.patterns[0].events.iter().map(|e| e.tick).collect();
        // Step 1 swings, the note inside the unpaired third step does not,
        // and the note at the pattern length is dropped.
        assert_eq!(ticks, vec![320.0, 600.0]);
    }

    #[test]
    fn tracks_are_ordered_after_what_feeds_them() {
        let (tracks, order) = compile_mixer(&Mixer {
            tracks: vec![
                track(0, None, &[]),
                track(1, Some(3), &[2]),
                track(2, Some(0), &[]),
                track(3, Some(0), &[]),
            ],
        });
        let place = |index: usize| order.iter().position(|&entry| entry == index).unwrap();
        for (index, track) in tracks.iter().enumerate() {
            for edge in &track.edges {
                assert!(place(index) < place(edge.target));
            }
        }
        assert_eq!(order.last(), Some(&0));
    }

    #[test]
    fn a_routing_loop_is_broken_instead_of_followed() {
        let (tracks, order) = compile_mixer(&Mixer {
            tracks: vec![
                track(0, None, &[]),
                track(1, Some(2), &[]),
                track(2, Some(1), &[0]),
            ],
        });
        assert_eq!(order.len(), 3);
        let place = |index: usize| order.iter().position(|&entry| entry == index).unwrap();
        for (index, track) in tracks.iter().enumerate() {
            for edge in &track.edges {
                assert!(place(index) < place(edge.target));
            }
        }
    }

    #[test]
    fn solo_keeps_feeders_and_the_path_to_the_master() {
        // 1 -> 2 -> 0, 3 -> 0, and 4 only sends into 2.
        let mut tracks = vec![
            track(0, None, &[]),
            track(1, Some(2), &[]),
            track(2, Some(0), &[]),
            track(3, Some(0), &[]),
            track(4, None, &[2]),
        ];
        tracks[2].solo = true;
        assert_eq!(mixer_gains(tracks.clone()), vec![1.0, 1.0, 1.0, 0.0, 1.0]);

        // Mute wins over solo.
        tracks[1].muted = true;
        assert_eq!(mixer_gains(tracks), vec![1.0, 0.0, 1.0, 0.0, 1.0]);
    }

    #[test]
    fn the_master_never_routes_anywhere() {
        let (tracks, _) = compile_mixer(&Mixer {
            tracks: vec![track(0, Some(1), &[1]), track(1, Some(0), &[])],
        });
        assert!(tracks[0].edges.is_empty());
        assert_eq!(tracks[1].edges.len(), 1);
    }

    #[test]
    fn muted_clips_do_not_play_but_still_set_the_song_length() {
        let mut project = Project::new("song");
        let pattern = project.patterns[0].id;
        let clip = |id: u32, start: u32, muted: bool| windfall_project::Clip {
            id: windfall_project::ClipId(id),
            track: windfall_project::PlaylistTrackId(50),
            start,
            length: 3840,
            offset: 3840 + 240,
            muted,
            content: ClipContent::Pattern { pattern },
        };
        project.playlist.clips = vec![clip(60, 0, false), clip(61, 3840, true)];
        let plan = compile(&project, &SamplePool::new());
        assert_eq!(plan.clips.len(), 1);
        assert_eq!(plan.clips[0].offset, 240);
        assert_eq!(plan.song_end, 7680);
    }
}
