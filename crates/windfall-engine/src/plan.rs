//! The compiled form of a project that the audio thread plays.
//!
//! [`compile`] runs on the control side. It resolves every id to an index,
//! applies swing, sorts notes by time, orders the mixer tracks and works out
//! what mute and solo leave audible, so the audio thread only ever follows
//! indices. A [`Plan`] is immutable once built. Everything that moves while
//! it plays, from the gain ramps to the effects themselves, lives in a
//! [`PlanState`](crate::state::PlanState) sized to match.

use std::collections::HashSet;
use std::sync::{
    Arc,
    atomic::{AtomicU8, AtomicU64, Ordering},
};

use windfall_core::{AudioBuffer, PPQ, TICKS_PER_STEP};
use windfall_project::{
    Channel, ChannelId, ChannelSource, Clip, ClipContent, ClipId, DEFAULT_PATTERN_STEPS, EffectId,
    EffectParams, Envelope, InstrumentParams, MAX_EFFECT_SLOTS, MAX_ENVELOPE_MS, MAX_GAIN, MAX_KEY,
    MAX_MIXER_TRACKS, MAX_TEMPO_BPM, MAX_TUNE_SEMITONES, MIN_TEMPO_BPM, Mixer, MixerTrack, Pattern,
    PatternId, Playlist, Project, SamplerSettings, TrackId,
};

use crate::automation::{self, Lane};
use crate::pool::SamplePool;
use crate::tempo::TempoMap;
use crate::voice::LoopRegion;

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
    pub fn new(ids: impl Iterator<Item = u32>) -> Self {
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
    pub navigation: Vec<crate::timeline::NavigationPoint>,
    pub signature: windfall_project::TimeSignature,
    pub meters: Result<
        Vec<windfall_project::timeline::MeterSegment>,
        windfall_project::timeline::MeterMapError,
    >,
    pub plugins: Vec<windfall_project::PluginBinding>,
    pub plugin_factory: Option<std::sync::Arc<dyn crate::plugins::PluginFactory>>,
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
    /// [`PlanState`](crate::state::PlanState) holds.
    pub edge_count: usize,
    /// Finds an effect that is part of the project: an index into
    /// `effect_places`.
    pub effect_ids: IdIndex,
    /// The track of each such effect and its place in that track's chain.
    pub effect_places: Vec<(usize, usize)>,
    /// Pattern clips that sound, sorted by start.
    pub clips: Vec<PlanClip>,
    /// Audio clips that sound, sorted by start and then by id.
    pub audio_clips: Vec<PlanAudioClip>,
    /// Finds an audio clip that sounds: an index into `audio_clips`.
    pub audio_clip_ids: IdIndex,
    /// End of the last clip on the playlist in ticks, muted clips included.
    pub song_end: u32,
    /// What automation does to each of its targets along the song.
    pub lanes: Vec<Lane>,
    /// How long the song takes to get to each of its ticks, when
    /// automation moves the tempo. `None` when the tempo is steady.
    pub tempo_map: Option<TempoMap>,
}

#[derive(Debug)]
pub(crate) struct PlanChannel {
    pub id: ChannelId,
    /// Index of the mixer track the channel plays into.
    pub track: usize,
    /// Channel volume, or zero when mute or solo silences the channel.
    pub gain: f32,
    /// Neither mute nor solo silences the channel.
    pub audible: bool,
    pub pan: f32,
    /// What a sampler channel plays. An instrument channel has a sampler
    /// with no sample.
    pub sampler: PlanSampler,
    /// The settings of the instrument an instrument channel plays.
    pub instrument: Option<InstrumentParams>,
    /// The channel is no longer in the project. It is listed after the
    /// project's channels, where no note finds it, for as long as its
    /// instrument takes to fade out. See [`Plan::keep_leaving`].
    pub leaving: bool,
}

#[derive(Debug)]
pub(crate) struct PlanSampler {
    pub sample: Option<AudioBuffer>,
    pub bank: Option<std::sync::Arc<crate::sampler_processing::SamplerBank>>,
    pub spectral: bool,
    /// First frame of the region that plays.
    pub start: usize,
    /// One past the last frame of the region. Greater than `start` whenever
    /// there is a sample.
    pub end: usize,
    pub reverse: bool,
    pub loop_region: Option<LoopRegion>,
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
    /// Neither mute nor solo silences the track.
    pub audible: bool,
    pub pan: f32,
    /// Where the post-fader signal goes: the output first, then the sends.
    pub edges: Vec<PlanEdge>,
    /// The effects the track's input runs through before the fader, in
    /// order.
    pub effects: Vec<PlanEffect>,
    /// Indices of the channels whose instruments play into this track.
    pub instruments: Vec<usize>,
}

/// Only progress is shared: this never grants access to a plugin owner.
/// Unheard prepared slots need no departing definition; heard ones stay
/// until their actual rack splice completes. References retire with plans.
#[derive(Debug)]
pub(crate) struct EffectLife {
    progress: AtomicU8,
    pub generation: u64,
}

impl Default for EffectLife {
    fn default() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self {
            progress: AtomicU8::new(0),
            generation: NEXT.fetch_add(1, Ordering::Relaxed),
        }
    }
}

impl EffectLife {
    pub fn heard(&self) -> bool {
        self.progress.load(Ordering::Acquire) == 1
    }
    pub fn hear(&self) {
        self.progress.store(1, Ordering::Release);
    }
    pub fn finish(&self) {
        self.progress.store(2, Ordering::Release);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PlanEffect {
    pub id: EffectId,
    /// Every value inside its range.
    pub params: EffectParams,
    pub enabled: bool,
    pub mix: f32,
    /// The effect is no longer in the project. It keeps its place in the
    /// chain for as long as it takes to fade out. See
    /// [`Plan::keep_leaving`].
    pub leaving: bool,
    pub life: Arc<EffectLife>,
    /// Requested binding/provider/revision at compilation or installation.
    /// Running identity comes from preparation's ledger: this snapshot can
    /// predate attachment, and a live factory Arc can change revision.
    native_owner: Option<u64>,
    /// A restored id waits for this audible departure before its fresh
    /// owner joins. Progress only; never a native-owner handle or lineage.
    pub after: Option<Arc<EffectLife>>,
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

/// An audio clip: a sample that plays straight onto the timeline.
#[derive(Debug)]
pub(crate) struct PlanAudioClip {
    pub id: ClipId,
    pub start: u32,
    pub end: u32,
    /// How far into the audio the clip starts, in ticks at the project's
    /// tempo.
    pub offset: u32,
    pub sample: AudioBuffer,
    /// Index of the mixer track the clip plays into, and that track's id.
    pub track: usize,
    pub track_id: TrackId,
    pub gain: f32,
    pub pan: f32,
    /// Length of the fade in, in ticks from the start.
    pub fade_in: u32,
    /// Length of the fade out, in ticks before the end.
    pub fade_out: u32,
    pub reverse: bool,
    pub pitch: f32,
    /// Seconds of the sample that go by in one second of the song.
    pub speed: f64,
    /// Frames of the sample, in playing order, that `offset` skips.
    pub skip: f64,
    /// Largest `end` among this clip and all clips before it.
    pub reach: u32,
}

impl Plan {
    pub(crate) fn snapshot_native_owners(&mut self) {
        let provider = self.plugin_factory.as_ref().map_or(0, |factory| {
            factory.revision().rotate_left(17) ^ factory.provider_identity()
        });
        for effect in self.tracks.iter_mut().flat_map(|track| &mut track.effects) {
            if !effect.leaving {
                let key = windfall_project::PluginTarget::Effect { effect: effect.id };
                effect.native_owner = self
                    .plugins
                    .iter()
                    .find(|binding| binding.target == key)
                    .map(|binding| crate::plugins::identity(binding) ^ provider);
            }
        }
    }

    /// What plays before a project is set: nothing, through a master track
    /// at unity so sample previews are still heard.
    pub fn empty() -> Self {
        compile(&Project::new(""), &SamplePool::new())
    }

    /// Keeps what `previous` played until its actual rack splice finishes,
    /// so intervening plans cannot stop a departing effect dead.
    ///
    /// An effect that left the project stays in its track's chain, marked
    /// as leaving, right after the effect it followed. An instrument
    /// channel that left the project is listed after the project's own
    /// channels, on the track it played into or on the master if that
    /// track is gone, at the gain and pan it had. Effect progress is shared
    /// with the rack: unheard speculative slots and completed departures
    /// are excluded here, and their storage retires on the control side.
    /// Instrument channels retain their existing one-plan departure policy.
    /// With a stream, `prepared` supplies the identities actually prepared
    /// or reused, rather than the previous plan's compilation-time snapshot.
    ///
    /// A track that is gone takes its effects with it at once: there is
    /// nowhere left for them to be heard.
    pub fn keep_leaving(&mut self, previous: &Plan, prepared: Option<&crate::state::Ledger>) {
        // Refresh on installation too: precompilation may precede a retry.
        // Never query the old plan's mutable factory for its past revision.
        self.snapshot_native_owners();
        let same_factory = match (&self.plugin_factory, &previous.plugin_factory) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        // Progress follows a retained identity across track moves as well.
        // A restored departing id gets a fresh marker and owner preparation.
        for effect in self.tracks.iter_mut().flat_map(|track| &mut track.effects) {
            let Some(before) = previous.effect_ids.get(effect.id.0) else {
                continue;
            };
            let (track, place) = previous.effect_places[before];
            let before = &previous.tracks[track].effects[place];
            let native = effect.native_owner;
            // A detached or precompiled plan can predate stream preparation.
            // Only the preparation ledger freezes the running native identity;
            // querying the previous factory now would observe its new revision.
            let before_native =
                prepared.map_or(before.native_owner, |held| held.native_identity(effect.id));
            if effect.params.kind() == before.params.kind()
                && native == before_native
                && (native.is_none() || same_factory)
            {
                effect.life = before.life.clone();
                effect.after = before.after.as_ref().filter(|life| life.heard()).cloned();
            }
        }
        for track in &mut self.tracks {
            let Some(before) = previous.track_ids.get(track.id.0) else {
                continue;
            };
            // Where the next leaver goes: after the last effect that was
            // ahead of it and is still in this chain.
            let mut at = 0;
            for effect in &previous.tracks[before].effects {
                if let Some(kept) = track
                    .effects
                    .iter()
                    .position(|e| !e.leaving && e.id == effect.id)
                {
                    if effect.leaving && effect.life.heard() {
                        // The restored id owns a fresh unit. Keep the heard
                        // outgoing transfer ahead of it until its existing
                        // splice finishes; repeated restores never restart it.
                        track.effects[kept].after = Some(effect.life.clone());
                        track.effects.insert(kept, effect.clone());
                        at = at.max(kept + 2);
                    } else {
                        at = at.max(kept + 1);
                    }
                } else if self.effect_ids.get(effect.id.0).is_none() && effect.life.heard() {
                    track.effects.insert(
                        at,
                        PlanEffect {
                            leaving: true,
                            after: None,
                            ..effect.clone()
                        },
                    );
                    at += 1;
                }
            }
        }
        for channel in &previous.channels {
            let instrument = channel.instrument.filter(|_| !channel.leaving);
            if instrument.is_none() || self.channel_ids.get(channel.id.0).is_some() {
                continue;
            }
            let track = previous.tracks[channel.track].id;
            self.channels.push(PlanChannel {
                id: channel.id,
                track: self.track_ids.get(track.0).unwrap_or(0),
                gain: channel.gain,
                audible: channel.audible,
                pan: channel.pan,
                sampler: PlanSampler::silent(),
                instrument,
                leaving: true,
            });
        }
        self.link();
    }

    /// Works out the lists that follow from the channels and the chains.
    fn link(&mut self) {
        for track in &mut self.tracks {
            track.instruments.clear();
        }
        for (index, channel) in self.channels.iter().enumerate() {
            if channel.instrument.is_some() {
                self.tracks[channel.track].instruments.push(index);
            }
        }
        let places = self.tracks.iter().enumerate().flat_map(|(track, entry)| {
            let effects = entry.effects.iter().enumerate();
            effects
                .filter(|(_, effect)| !effect.leaving)
                .map(move |(place, effect)| (effect.id, (track, place)))
        });
        let (ids, places): (Vec<u32>, Vec<(usize, usize)>) =
            places.map(|(id, place)| (id.0, place)).unzip();
        self.effect_ids = IdIndex::new(ids.into_iter());
        self.effect_places = places;
    }

    /// The channel with this id, if the project has it.
    pub fn channel(&self, id: ChannelId) -> Option<usize> {
        self.channel_ids.get(id.0)
    }

    /// The place in the song's own time of a tick of the song: the tick
    /// itself while the tempo is steady, and the tick at which the same
    /// moment would come at the stored tempo when automation moves it.
    pub fn warp(&self, tick: f64) -> f64 {
        match &self.tempo_map {
            Some(map) => map.warp(tick),
            None => tick,
        }
    }

    /// The inverse of [`warp`](Self::warp).
    pub fn unwarp(&self, warped: f64) -> f64 {
        match &self.tempo_map {
            Some(map) => map.unwarp(warped),
            None => warped,
        }
    }

    /// True when the plan keeps `buffer` alive, which makes dropping another
    /// handle to it free of any deallocation.
    pub fn holds(&self, buffer: &AudioBuffer) -> bool {
        let channels = self.channels.iter();
        let mut samples = channels
            .filter_map(|channel| channel.sampler.sample.as_ref())
            .chain(self.audio_clips.iter().map(|clip| &clip.sample));
        samples.any(|sample| same_audio(sample, buffer))
            || self.channels.iter().any(|channel| {
                channel
                    .sampler
                    .bank
                    .as_ref()
                    .is_some_and(|bank| bank.holds(buffer))
            })
    }

    pub fn holds_sampler_bank(
        &self,
        bank: &std::sync::Arc<crate::sampler_processing::SamplerBank>,
    ) -> bool {
        self.channels.iter().any(|channel| {
            channel
                .sampler
                .bank
                .as_ref()
                .is_some_and(|held| std::sync::Arc::ptr_eq(held, bank))
        })
    }
}

/// True when both handles share one block of sample data.
pub(crate) fn same_audio(a: &AudioBuffer, b: &AudioBuffer) -> bool {
    std::ptr::eq(a.samples(), b.samples())
}

impl PlanSampler {
    /// A sampler with nothing to play.
    fn silent() -> Self {
        compile_sampler(&SamplerSettings::default(), &SamplePool::new())
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
    let audio_clips = compile_audio_clips(&project.playlist, pool, &track_ids, tempo_bpm);
    let audio_clip_ids = IdIndex::new(audio_clips.iter().map(|clip| clip.id.0));

    let mut plan = Plan {
        navigation: crate::timeline::compile(&project.playlist.timeline),
        signature: project.settings.time_signature,
        meters: windfall_project::timeline::MeterMap::checked(
            project.settings.time_signature,
            &project.playlist.timeline.meters,
        )
        .map(|map| map.segments().to_vec()),
        plugins: project.plugins.clone(),
        plugin_factory: pool.plugin_factory.clone(),
        tempo_bpm,
        channels,
        channel_ids,
        patterns,
        pattern_ids,
        tracks,
        track_ids,
        order,
        edge_count,
        effect_ids: IdIndex::default(),
        effect_places: Vec::new(),
        clips,
        audio_clips,
        audio_clip_ids,
        song_end,
        lanes: Vec::new(),
        tempo_map: None,
    };
    if plan.meters.is_ok() {
        plan.snapshot_native_owners();
    }
    plan.link();
    plan.lanes = automation::compile(project, &plan);
    let tempo_lane = plan.lanes.iter().find(|lane| lane.is_tempo());
    plan.tempo_map = tempo_lane.map(|lane| TempoMap::new(lane, tempo_bpm, f64::from(song_end)));
    plan
}

pub(crate) fn compile_render(project: &Project, pool: &SamplePool) -> Plan {
    let mut pool = pool.clone();
    if let Some(factory) = &pool.plugin_factory
        && let Some(render) = factory.render_factory()
    {
        pool.plugin_factory = Some(render);
    }
    compile(project, &pool)
}

fn compile_channel(
    channel: &Channel,
    pool: &SamplePool,
    track_ids: &IdIndex,
    any_solo: bool,
) -> PlanChannel {
    let (sampler, instrument) = match &channel.source {
        ChannelSource::Sampler(settings) => (compile_sampler(settings, pool), None),
        ChannelSource::Instrument { params } => (PlanSampler::silent(), Some(params.sanitized())),
    };
    let audible = !channel.muted && (!any_solo || channel.solo);
    PlanChannel {
        id: channel.id,
        // A channel whose track is gone plays into the master.
        track: track_ids.get(channel.mixer_track.0).unwrap_or(0),
        gain: if audible { gain(channel.volume) } else { 0.0 },
        audible,
        pan: pan(channel.pan),
        sampler,
        instrument,
        leaving: false,
    }
}

fn compile_sampler(settings: &SamplerSettings, pool: &SamplePool) -> PlanSampler {
    let spectral = matches!(
        settings.stretch,
        windfall_project::SamplerStretch::Spectral { .. }
    );
    let bank = spectral.then(|| pool.sampler_bank(settings)).flatten();
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
    let region_frames = bank.as_ref().map_or(end - start, |bank| bank.frames);
    let loop_region = (settings.loop_mode != windfall_project::SamplerLoopMode::Off
        && region_frames > 0)
        .then(|| {
            let first = ((f64::from(unit(settings.loop_start)) * region_frames as f64).round()
                as usize)
                .min(region_frames - 1);
            let end_fraction = if settings.loop_end.is_finite() {
                unit(settings.loop_end)
            } else {
                1.0
            };
            let end = ((f64::from(end_fraction) * region_frames as f64).round() as usize)
                .clamp(first + 1, region_frames);
            LoopRegion {
                first: if settings.reverse {
                    region_frames - end
                } else {
                    first
                },
                frames: end - first,
                mode: settings.loop_mode,
            }
        });
    PlanSampler {
        bank,
        spectral,
        sample,
        start,
        end,
        reverse: settings.reverse,
        loop_region,
        key_offset: tune - f32::from(settings.root_key),
        gain: gain(settings.gain),
        envelope: settings
            .envelope
            .or_else(|| {
                loop_region.map(|_| Envelope {
                    attack_ms: 0.0,
                    decay_ms: 0.0,
                    sustain: 1.0,
                    release_ms: 4.0,
                })
            })
            .map(|envelope| Envelope {
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
        let ClipContent::Pattern { pattern } = &clip.content else {
            continue;
        };
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

/// The audio clips that sound: not muted, not on a muted playlist track, and
/// with audio in the pool. A clip whose mixer track is gone plays into the
/// master.
fn compile_audio_clips(
    playlist: &Playlist,
    pool: &SamplePool,
    track_ids: &IdIndex,
    tempo_bpm: f64,
) -> Vec<PlanAudioClip> {
    let muted_track = |clip: &Clip| {
        let tracks = playlist.tracks.iter();
        tracks
            .into_iter()
            .any(|track| track.id == clip.track && track.muted)
    };
    let mut clips = Vec::new();
    for clip in &playlist.clips {
        let ClipContent::Audio {
            sample,
            mixer_track,
            gain: clip_gain,
            pan: clip_pan,
            fade_in,
            fade_out,
            reverse,
            pitch,
            stretch,
        } = clip.content
        else {
            continue;
        };
        let end = clip.start.saturating_add(clip.length);
        let buffer = pool
            .clip_audio(sample, stretch, pitch)
            .filter(|buffer| buffer.frames() > 0);
        let Some(buffer) = buffer.filter(|_| !clip.muted && end > clip.start) else {
            continue;
        };
        if muted_track(clip) {
            continue;
        }
        let pitch = if pitch.is_finite() {
            pitch.clamp(-MAX_TUNE_SEMITONES, MAX_TUNE_SEMITONES)
        } else {
            0.0
        };
        let speed = if matches!(stretch, windfall_project::ClipStretch::Tape) {
            2.0_f64.powf(f64::from(pitch) / 12.0)
        } else {
            1.0
        };
        let offset_seconds = f64::from(clip.offset) * 60.0 / (tempo_bpm * f64::from(PPQ));
        let track = track_ids.get(mixer_track.0);
        clips.push(PlanAudioClip {
            id: clip.id,
            start: clip.start,
            end,
            offset: clip.offset,
            sample: buffer.clone(),
            track: track.unwrap_or(0),
            track_id: track.map_or(TrackId::MASTER, |_| mixer_track),
            gain: gain(clip_gain),
            pan: pan(clip_pan),
            fade_in,
            fade_out,
            reverse,
            pitch,
            speed,
            skip: offset_seconds * speed * f64::from(buffer.sample_rate()),
            reach: end,
        });
    }
    // Clips that start together take their slots in the order of their
    // ids, which settles who is left out when there are too many.
    clips.sort_by_key(|clip| (clip.start, clip.id));
    let mut reach = 0;
    for clip in &mut clips {
        reach = reach.max(clip.end);
        clip.reach = reach;
    }
    clips
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
        effects: Vec::new(),
    }];
    let source: &[MixerTrack] = if mixer.tracks.is_empty() {
        &fallback_master
    } else {
        &mixer.tracks[..mixer.tracks.len().min(MAX_MIXER_TRACKS)]
    };
    let ids = IdIndex::new(source.iter().map(|track| track.id.0));
    // An effect id is one effect. If a damaged project repeats one, the
    // first wins.
    let mut effects_seen = HashSet::new();

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
            let effects = track
                .effects
                .iter()
                .filter(|effect| effects_seen.insert(effect.id))
                .take(MAX_EFFECT_SLOTS)
                .map(|effect| PlanEffect {
                    id: effect.id,
                    params: effect.params.sanitized(),
                    enabled: effect.enabled,
                    mix: if effect.mix.is_finite() {
                        effect.mix.clamp(0.0, 1.0)
                    } else {
                        1.0
                    },
                    leaving: false,
                    life: Arc::new(EffectLife::default()),
                    native_owner: None,
                    after: None,
                })
                .collect();
            PlanTrack {
                id: track.id,
                gain: gain(track.volume),
                audible: true,
                pan: pan(track.pan),
                edges: output.into_iter().chain(sends).collect(),
                effects,
                instruments: Vec::new(),
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
            track.audible = false;
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
    if ms.is_finite() {
        ms.clamp(0.0, MAX_ENVELOPE_MS)
    } else {
        0.0
    }
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
            effects: Vec::new(),
        }
    }

    fn mixer_gains(tracks: Vec<MixerTrack>) -> Vec<f32> {
        let (tracks, _) = compile_mixer(&Mixer { tracks });
        tracks.iter().map(|track| track.gain).collect()
    }

    #[test]
    fn repeated_restores_keep_one_outgoing_generation_and_one_active_slot_per_id() {
        let mut project = Project::new("bounded restores");
        let slots: Vec<_> = (0..MAX_EFFECT_SLOTS)
            .map(|index| windfall_project::EffectSlot {
                id: EffectId(index as u32 + 1),
                params: windfall_project::EffectKind::StereoMatrix.default_params(),
                enabled: true,
                mix: 1.0,
            })
            .collect();
        project.mixer.tracks[0].effects = slots.clone();
        let pool = SamplePool::new();
        let mut previous = compile(&project, &pool);
        let generations: Vec<_> = previous.tracks[0]
            .effects
            .iter()
            .map(|effect| {
                effect.life.hear();
                effect.life.generation
            })
            .collect();
        for _ in 0..1000 {
            project.mixer.tracks[0].effects.clear();
            let mut removed = compile(&project, &pool);
            removed.keep_leaving(&previous, None);
            assert_eq!(removed.tracks[0].effects.len(), MAX_EFFECT_SLOTS);
            assert!(
                removed.tracks[0]
                    .effects
                    .iter()
                    .all(|effect| effect.leaving && effect.after.is_none())
            );
            project.mixer.tracks[0].effects = slots.clone();
            let mut restored = compile(&project, &pool);
            restored.keep_leaving(&removed, None);
            assert_eq!(restored.tracks[0].effects.len(), 2 * MAX_EFFECT_SLOTS);
            for (index, pair) in restored.tracks[0]
                .effects
                .as_chunks::<2>()
                .0
                .iter()
                .enumerate()
            {
                assert!(pair[0].leaving && !pair[1].leaving);
                assert_eq!(pair[0].life.generation, generations[index]);
                assert!(Arc::ptr_eq(pair[1].after.as_ref().unwrap(), &pair[0].life));
                let active = restored.effect_ids.get(pair[1].id.0).unwrap();
                assert_eq!(restored.effect_places[active], (0, index * 2 + 1));
            }
            previous = restored;
        }
        for effect in previous.tracks[0]
            .effects
            .iter()
            .filter(|effect| effect.leaving)
        {
            effect.life.finish();
        }
        let mut settled = compile(&project, &pool);
        settled.keep_leaving(&previous, None);
        assert_eq!(settled.tracks[0].effects.len(), MAX_EFFECT_SLOTS);
        assert!(
            settled.tracks[0]
                .effects
                .iter()
                .all(|effect| !effect.leaving && effect.after.is_none())
        );
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
