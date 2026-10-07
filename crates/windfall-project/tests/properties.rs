//! Property tests: random runs of commands, undo, redo and saves must keep
//! every promise the document makes.

use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::{Index, select};
use windfall_dsp::{LimiterParams, SynthParams};
use windfall_project::file::{from_json, load, save, to_json};
use windfall_project::*;

/// An id no project in these tests ever reaches.
const MISSING: u32 = 4_000_000;

/// Chooses one of the things that exist when the command is built or, now
/// and then, an id that does not exist.
#[derive(Debug, Clone)]
struct Pick {
    index: Index,
    missing: bool,
}

impl Pick {
    fn among<T: Copy + From<u32>>(&self, ids: &[T]) -> T {
        if self.missing || ids.is_empty() {
            T::from(MISSING)
        } else {
            *self.index.get(ids)
        }
    }
}

fn pick() -> impl Strategy<Value = Pick> {
    (any::<Index>(), prop::bool::weighted(0.04))
        .prop_map(|(index, missing)| Pick { index, missing })
}

/// A command whose ids are filled in from the project it is about to run on.
#[derive(Debug, Clone)]
enum Action {
    UpdateSettings(SettingsPatch),
    AddSample(String, SamplePath),
    RemoveSample(Pick),
    AddChannel {
        name: Option<String>,
        sample: Option<Pick>,
        instrument: Option<InstrumentKind>,
        index: Option<u32>,
        mixer_track: Option<Pick>,
    },
    RemoveChannel(Pick),
    DuplicateChannel(Pick),
    MoveChannel(Pick, u32),
    UpdateChannel(Pick, ChannelPatch, Option<Pick>),
    SetChannelSample(Pick, Option<Pick>),
    UpdateSampler(Pick, SamplerPatch),
    SetSamplerEnvelope(Pick, Option<Envelope>),
    SetInstrumentParam(Pick, u32, f32),
    SetInstrumentParams(Pick, Settings),
    AddPattern(Option<String>),
    RemovePattern(Pick),
    DuplicatePattern(Pick),
    MovePattern(Pick, u32),
    UpdatePattern(Pick, PatternPatch),
    ToggleStep(Pick, Pick, u32),
    AddNotes(Pick, Pick, Vec<NoteInit>),
    RemoveNotes(Pick, Pick, Vec<Pick>),
    UpdateNotes(Pick, Pick, Vec<(Pick, NotePatch)>),
    ClearLane(Pick, Pick),
    AddMixerTrack(Option<String>),
    RemoveMixerTrack(Pick),
    UpdateMixerTrack(Pick, MixerTrackPatch),
    SetTrackOutput(Pick, Option<Pick>),
    SetSend(Pick, Pick, Option<f32>),
    AddEffect(Pick, EffectKind, Option<u32>),
    RemoveEffect(Pick, Pick),
    MoveEffect(Pick, Pick, Option<Pick>, u32),
    UpdateEffect(Pick, Pick, EffectSlotPatch),
    SetEffectParam(Pick, Pick, u32, f32),
    /// The kind is the effect's own unless one is given here.
    SetEffectParams(Pick, Pick, Option<EffectKind>, Settings),
    DuplicateEffect(Pick, Pick),
    ReplaceEffect(Pick, Pick, EffectKind),
    AddPlaylistTrack(Option<String>, Option<u32>),
    RemovePlaylistTrack(Pick),
    UpdatePlaylistTrack(Pick, PlaylistTrackPatch),
    MovePlaylistTrack(Pick, u32),
    AddClips(Vec<ClipSpec>),
    RemoveClips(Vec<Pick>),
    UpdateClips(Vec<(Pick, ClipPatch, Option<Pick>)>),
    /// The clip is an audio clip when there is one, and now and then
    /// any clip.
    UpdateAudioClips(Vec<(Pick, AudioClipPatch, Option<Pick>)>),
    AddAutomation(Option<String>, TargetSpec, Option<Vec<AutomationPoint>>),
    RemoveAutomation(Pick),
    UpdateAutomation(Pick, AutomationPatch),
    SetAutomationPoints(Pick, Vec<AutomationPoint>),
    DuplicateAutomation(Pick),
    Batch(Option<String>, Vec<Action>),
}

/// What an automation is to move.
#[derive(Debug, Clone)]
enum TargetSpec {
    ChannelVolume(Pick),
    ChannelPan(Pick),
    TrackVolume(Pick),
    TrackPan(Pick),
    /// One of the sends there are or, with `any`, from and to any tracks.
    SendGain {
        send: Pick,
        to: Pick,
        any: bool,
    },
    EffectParam(Pick, Pick, u32),
    EffectMix(Pick, Pick),
    InstrumentParam(Pick, u32),
    Tempo,
}

/// A clip to add.
#[derive(Debug, Clone)]
struct ClipSpec {
    track: Pick,
    start: u32,
    length: Option<u32>,
    offset: Option<u32>,
    muted: Option<bool>,
    content: ContentSpec,
}

#[derive(Debug, Clone)]
enum ContentSpec {
    Pattern(Pick),
    Audio {
        sample: Pick,
        mixer_track: Pick,
        /// Gain, pan and pitch.
        levels: (f32, f32, f32),
        fades: (u32, u32),
        reverse: bool,
    },
    Automation(Pick),
}

/// Settings for an effect or instrument, built on its defaults.
#[derive(Debug, Clone)]
struct Settings {
    /// Settings changed one at a time, which brings each into its range.
    changes: Vec<(Index, f32)>,
    /// A value written straight into one field, range or no range.
    raw: Option<f32>,
}

impl Settings {
    fn effect(&self, kind: EffectKind) -> EffectParams {
        let mut params = kind.default_params();
        let count = kind.descriptors().len();
        for (index, value) in &self.changes {
            params.set(index.index(count), *value);
        }
        match (params, self.raw) {
            (EffectParams::Limiter(limiter), Some(raw)) => EffectParams::Limiter(LimiterParams {
                release_ms: raw * 400.0,
                ..limiter
            }),
            (params, _) => params,
        }
    }

    fn instrument(&self, kind: InstrumentKind) -> InstrumentParams {
        let mut params = kind.default_params();
        let count = kind.descriptors().len();
        for (index, value) in &self.changes {
            params.set(index.index(count), *value);
        }
        let InstrumentParams::SubtractiveSynth(synth) = params;
        InstrumentParams::SubtractiveSynth(SynthParams {
            gain: self.raw.unwrap_or(synth.gain),
            ..synth
        })
    }
}

impl Action {
    fn resolve(&self, project: &Project) -> Command {
        let samples: Vec<SampleId> = project.samples.iter().map(|s| s.id).collect();
        let channels: Vec<ChannelId> = project.channels.iter().map(|c| c.id).collect();
        let patterns: Vec<PatternId> = project.patterns.iter().map(|p| p.id).collect();
        let tracks: Vec<TrackId> = project.mixer.tracks.iter().map(|t| t.id).collect();
        let lanes: Vec<PlaylistTrackId> = project.playlist.tracks.iter().map(|t| t.id).collect();
        let clips: Vec<ClipId> = project.playlist.clips.iter().map(|c| c.id).collect();
        let audio_clips: Vec<ClipId> = project
            .playlist
            .clips
            .iter()
            .filter(|clip| matches!(clip.content, ClipContent::Audio { .. }))
            .map(|clip| clip.id)
            .collect();
        let automations: Vec<AutomationId> = project.automations.iter().map(|a| a.id).collect();
        let sends: Vec<(TrackId, TrackId)> = project
            .mixer
            .tracks
            .iter()
            .flat_map(|track| track.sends.iter().map(|send| (track.id, send.target)))
            .collect();

        // For commands that act on existing notes: a lane that has some, and
        // the ids of its notes.
        let notes_in = |pattern: &Pick, channel: &Pick| {
            let pattern = pattern.among(&patterns);
            let lanes = project
                .pattern(pattern)
                .map_or(&[][..], |p| p.lanes.as_slice());
            let with_notes: Vec<ChannelId> = lanes.iter().map(|lane| lane.channel).collect();
            let channel = if with_notes.is_empty() {
                channel.among(&channels)
            } else {
                channel.among(&with_notes)
            };
            let lane = lanes.iter().find(|lane| lane.channel == channel);
            let notes: Vec<NoteId> = lane
                .map(|lane| lane.notes.iter().map(|note| note.id).collect())
                .unwrap_or_default();
            (pattern, channel, notes)
        };

        // For commands that act on an effect: a track that has some, and
        // one of them.
        let effect_on = |track: &Pick, effect: &Pick| {
            let all = &project.mixer.tracks;
            let with_effects: Vec<TrackId> = all
                .iter()
                .filter(|track| !track.effects.is_empty())
                .map(|track| track.id)
                .collect();
            let track = if with_effects.is_empty() {
                track.among(&tracks)
            } else {
                track.among(&with_effects)
            };
            let effects = project.mixer.track(track).map_or(&[][..], |t| &t.effects);
            let ids: Vec<EffectId> = effects.iter().map(|slot| slot.id).collect();
            let effect = effect.among(&ids);
            let kind = effects.iter().find(|slot| slot.id == effect);
            (track, effect, kind.map(EffectSlot::kind))
        };

        // For commands that act on an instrument: a channel that plays one.
        // Now and then a sampler, which is one of the ways they fail.
        let instrument = |channel: &Pick| {
            let instruments: Vec<ChannelId> = project
                .channels
                .iter()
                .filter(|c| matches!(c.source, ChannelSource::Instrument { .. }))
                .map(|c| c.id)
                .collect();
            if instruments.is_empty() || channel.index.index(8) == 0 {
                channel.among(&channels)
            } else {
                channel.among(&instruments)
            }
        };

        match self {
            Action::UpdateSettings(patch) => Command::UpdateSettings {
                patch: patch.clone(),
            },
            Action::AddSample(name, path) => Command::AddSample {
                name: name.clone(),
                path: path.clone(),
            },
            Action::RemoveSample(id) => Command::RemoveSample {
                id: id.among(&samples),
            },
            Action::AddChannel {
                name,
                sample,
                instrument,
                index,
                mixer_track,
            } => Command::AddChannel {
                name: name.clone(),
                sample: sample.as_ref().map(|sample| sample.among(&samples)),
                instrument: *instrument,
                index: *index,
                mixer_track: mixer_track.as_ref().map(|track| track.among(&tracks)),
            },
            Action::RemoveChannel(id) => Command::RemoveChannel {
                id: id.among(&channels),
            },
            Action::DuplicateChannel(id) => Command::DuplicateChannel {
                id: id.among(&channels),
            },
            Action::MoveChannel(id, index) => Command::MoveChannel {
                id: id.among(&channels),
                index: *index,
            },
            Action::UpdateChannel(id, patch, track) => Command::UpdateChannel {
                id: id.among(&channels),
                patch: ChannelPatch {
                    mixer_track: track.as_ref().map(|track| track.among(&tracks)),
                    ..patch.clone()
                },
            },
            Action::SetChannelSample(id, sample) => Command::SetChannelSample {
                id: id.among(&channels),
                sample: sample.as_ref().map(|sample| sample.among(&samples)),
            },
            Action::UpdateSampler(id, patch) => Command::UpdateSampler {
                id: id.among(&channels),
                patch: patch.clone(),
            },
            Action::SetSamplerEnvelope(id, envelope) => Command::SetSamplerEnvelope {
                id: id.among(&channels),
                envelope: *envelope,
            },
            Action::SetInstrumentParam(channel, param, value) => Command::SetInstrumentParam {
                channel: instrument(channel),
                param: *param,
                value: *value,
            },
            Action::SetInstrumentParams(channel, settings) => Command::SetInstrumentParams {
                channel: instrument(channel),
                params: settings.instrument(InstrumentKind::SubtractiveSynth),
            },
            Action::AddPattern(name) => Command::AddPattern { name: name.clone() },
            Action::RemovePattern(id) => Command::RemovePattern {
                id: id.among(&patterns),
            },
            Action::DuplicatePattern(id) => Command::DuplicatePattern {
                id: id.among(&patterns),
            },
            Action::MovePattern(id, index) => Command::MovePattern {
                id: id.among(&patterns),
                index: *index,
            },
            Action::UpdatePattern(id, patch) => Command::UpdatePattern {
                id: id.among(&patterns),
                patch: patch.clone(),
            },
            Action::ToggleStep(pattern, channel, step) => Command::ToggleStep {
                pattern: pattern.among(&patterns),
                channel: channel.among(&channels),
                step: *step,
            },
            Action::AddNotes(pattern, channel, notes) => Command::AddNotes {
                pattern: pattern.among(&patterns),
                channel: channel.among(&channels),
                notes: notes.clone(),
            },
            Action::RemoveNotes(pattern, channel, picks) => {
                let (pattern, channel, notes) = notes_in(pattern, channel);
                Command::RemoveNotes {
                    pattern,
                    channel,
                    notes: picks.iter().map(|pick| pick.among(&notes)).collect(),
                }
            }
            Action::UpdateNotes(pattern, channel, updates) => {
                let (pattern, channel, notes) = notes_in(pattern, channel);
                Command::UpdateNotes {
                    pattern,
                    channel,
                    updates: updates
                        .iter()
                        .map(|(pick, patch)| NoteUpdate {
                            id: pick.among(&notes),
                            patch: *patch,
                        })
                        .collect(),
                }
            }
            Action::ClearLane(pattern, channel) => Command::ClearLane {
                pattern: pattern.among(&patterns),
                channel: channel.among(&channels),
            },
            Action::AddMixerTrack(name) => Command::AddMixerTrack { name: name.clone() },
            Action::RemoveMixerTrack(id) => Command::RemoveMixerTrack {
                id: id.among(&tracks),
            },
            Action::UpdateMixerTrack(id, patch) => Command::UpdateMixerTrack {
                id: id.among(&tracks),
                patch: patch.clone(),
            },
            Action::SetTrackOutput(id, output) => Command::SetTrackOutput {
                id: id.among(&tracks),
                output: output.as_ref().map(|output| output.among(&tracks)),
            },
            Action::SetSend(from, to, gain) => Command::SetSend {
                from: from.among(&tracks),
                to: to.among(&tracks),
                gain: *gain,
            },
            Action::AddEffect(track, kind, index) => Command::AddEffect {
                track: track.among(&tracks),
                kind: *kind,
                index: *index,
            },
            Action::RemoveEffect(track, effect) => {
                let (track, effect, _) = effect_on(track, effect);
                Command::RemoveEffect { track, effect }
            }
            Action::MoveEffect(track, effect, to_track, index) => {
                let (track, effect, _) = effect_on(track, effect);
                Command::MoveEffect {
                    track,
                    effect,
                    to_track: to_track.as_ref().map(|to| to.among(&tracks)),
                    index: *index,
                }
            }
            Action::UpdateEffect(track, effect, patch) => {
                let (track, effect, _) = effect_on(track, effect);
                Command::UpdateEffect {
                    track,
                    effect,
                    patch: *patch,
                }
            }
            Action::SetEffectParam(track, effect, param, value) => {
                let (track, effect, _) = effect_on(track, effect);
                Command::SetEffectParam {
                    track,
                    effect,
                    param: *param,
                    value: *value,
                }
            }
            Action::SetEffectParams(track, effect, other_kind, settings) => {
                let (track, effect, kind) = effect_on(track, effect);
                let kind = other_kind.or(kind).unwrap_or(EffectKind::Eq);
                Command::SetEffectParams {
                    track,
                    effect,
                    params: settings.effect(kind),
                }
            }
            Action::DuplicateEffect(track, effect) => {
                let (track, effect, _) = effect_on(track, effect);
                Command::DuplicateEffect { track, effect }
            }
            Action::ReplaceEffect(track, effect, kind) => {
                let (track, effect, _) = effect_on(track, effect);
                Command::ReplaceEffect {
                    track,
                    effect,
                    kind: *kind,
                }
            }
            Action::AddPlaylistTrack(name, index) => Command::AddPlaylistTrack {
                name: name.clone(),
                index: *index,
            },
            Action::MovePlaylistTrack(id, index) => Command::MovePlaylistTrack {
                id: id.among(&lanes),
                index: *index,
            },
            Action::RemovePlaylistTrack(id) => Command::RemovePlaylistTrack {
                id: id.among(&lanes),
            },
            Action::UpdatePlaylistTrack(id, patch) => Command::UpdatePlaylistTrack {
                id: id.among(&lanes),
                patch: patch.clone(),
            },
            Action::AddClips(specs) => Command::AddClips {
                clips: specs
                    .iter()
                    .map(|spec| ClipInit {
                        track: spec.track.among(&lanes),
                        start: spec.start,
                        length: spec.length,
                        offset: spec.offset,
                        muted: spec.muted,
                        content: match &spec.content {
                            ContentSpec::Pattern(pattern) => ClipContent::Pattern {
                                pattern: pattern.among(&patterns),
                            },
                            ContentSpec::Audio {
                                sample,
                                mixer_track,
                                levels: (gain, pan, pitch),
                                fades: (fade_in, fade_out),
                                reverse,
                            } => ClipContent::Audio {
                                sample: sample.among(&samples),
                                mixer_track: mixer_track.among(&tracks),
                                gain: *gain,
                                pan: *pan,
                                fade_in: *fade_in,
                                fade_out: *fade_out,
                                reverse: *reverse,
                                pitch: *pitch * 30.0,

                                stretch: Default::default(),
                            },
                            ContentSpec::Automation(automation) => ClipContent::Automation {
                                automation: automation.among(&automations),
                            },
                        },
                    })
                    .collect(),
            },
            Action::UpdateAudioClips(updates) => Command::UpdateAudioClips {
                updates: updates
                    .iter()
                    .map(|(pick, patch, track)| AudioClipUpdate {
                        id: if audio_clips.is_empty() || pick.index.index(10) == 0 {
                            pick.among(&clips)
                        } else {
                            pick.among(&audio_clips)
                        },
                        patch: AudioClipPatch {
                            mixer_track: track.as_ref().map(|track| track.among(&tracks)),
                            ..*patch
                        },
                    })
                    .collect(),
            },
            Action::RemoveClips(picks) => Command::RemoveClips {
                clips: picks.iter().map(|pick| pick.among(&clips)).collect(),
            },
            Action::UpdateClips(updates) => Command::UpdateClips {
                updates: updates
                    .iter()
                    .map(|(pick, patch, track)| ClipUpdate {
                        id: pick.among(&clips),
                        patch: ClipPatch {
                            track: track.as_ref().map(|track| track.among(&lanes)),
                            ..patch.clone()
                        },
                    })
                    .collect(),
            },
            Action::AddAutomation(name, target, points) => Command::AddAutomation {
                name: name.clone(),
                target: match target {
                    TargetSpec::ChannelVolume(channel) => AutomationTarget::ChannelVolume {
                        channel: channel.among(&channels),
                    },
                    TargetSpec::ChannelPan(channel) => AutomationTarget::ChannelPan {
                        channel: channel.among(&channels),
                    },
                    TargetSpec::TrackVolume(track) => AutomationTarget::TrackVolume {
                        track: track.among(&tracks),
                    },
                    TargetSpec::TrackPan(track) => AutomationTarget::TrackPan {
                        track: track.among(&tracks),
                    },
                    TargetSpec::SendGain { send, to, any } => {
                        if *any || send.missing || sends.is_empty() {
                            AutomationTarget::SendGain {
                                track: send.among(&tracks),
                                target: to.among(&tracks),
                            }
                        } else {
                            let (track, target) = *send.index.get(&sends);
                            AutomationTarget::SendGain { track, target }
                        }
                    }
                    TargetSpec::EffectParam(track, effect, param) => {
                        let (track, effect, _) = effect_on(track, effect);
                        AutomationTarget::EffectParam {
                            track,
                            effect,
                            param: *param,
                        }
                    }
                    TargetSpec::EffectMix(track, effect) => {
                        let (track, effect, _) = effect_on(track, effect);
                        AutomationTarget::EffectMix { track, effect }
                    }
                    TargetSpec::InstrumentParam(channel, param) => {
                        AutomationTarget::InstrumentParam {
                            channel: instrument(channel),
                            param: *param,
                        }
                    }
                    TargetSpec::Tempo => AutomationTarget::Tempo,
                },
                points: points.clone(),
            },
            Action::RemoveAutomation(id) => Command::RemoveAutomation {
                id: id.among(&automations),
            },
            Action::UpdateAutomation(id, patch) => Command::UpdateAutomation {
                id: id.among(&automations),
                patch: patch.clone(),
            },
            Action::SetAutomationPoints(id, points) => Command::SetAutomationPoints {
                id: id.among(&automations),
                points: points.clone(),
            },
            Action::DuplicateAutomation(id) => Command::DuplicateAutomation {
                id: id.among(&automations),
            },
            Action::Batch(label, actions) => Command::Batch {
                label: label.clone(),
                // Every command is built from the project as it is before the
                // batch, so a later one can name something an earlier one
                // removed. That is one of the ways a batch fails.
                commands: actions.iter().map(|a| a.resolve(project)).collect(),
            },
        }
    }
}

fn maybe<T: std::fmt::Debug + Clone>(
    value: impl Strategy<Value = T>,
) -> impl Strategy<Value = Option<T>> {
    prop::option::weighted(0.4, value)
}

/// A value a knob might send: mostly near its range, sometimes far outside,
/// sometimes not a number.
fn level() -> impl Strategy<Value = f32> {
    prop_oneof![
        6 => -2.5f32..2.5,
        3 => select(vec![0.0f32, 0.25, 0.5, 0.8, 1.0, -1.0, 2.0, 60.0, -60.0]),
        1 => select(vec![f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.0]),
    ]
}

/// A value for a setting of an effect or instrument. Their ranges differ
/// widely, from a mix of 0 to 1 to a cutoff of 20 to 20000 Hz.
fn setting() -> impl Strategy<Value = f32> {
    prop_oneof![
        5 => -2.5f32..2.5,
        3 => -80.0f32..120.0,
        2 => 0.0f32..25_000.0,
        2 => select(vec![0.0f32, 0.3, 1.0, 2.0, 4.0, 5.0, 100.0]),
        1 => select(vec![f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.0]),
    ]
}

/// The index of a setting: usually one the processor has.
fn setting_index() -> impl Strategy<Value = u32> {
    prop_oneof![12 => 0u32..12, 3 => 0u32..80, 1 => any::<u32>()]
}

fn settings() -> impl Strategy<Value = Settings> {
    let raw = prop::option::weighted(0.15, level());
    (vec((any::<Index>(), setting()), 0..5), raw)
        .prop_map(|(changes, raw)| Settings { changes, raw })
}

fn effect_kind() -> impl Strategy<Value = EffectKind> {
    select(EffectKind::ALL.to_vec())
}

fn effect_patch() -> impl Strategy<Value = EffectSlotPatch> {
    (maybe(any::<bool>()), maybe(level()))
        .prop_map(|(enabled, mix)| EffectSlotPatch { enabled, mix })
}

fn name() -> impl Strategy<Value = String> {
    select(vec![
        "Kick",
        "Snare",
        "Hat",
        "Bass",
        "Lead #2",
        "Pattern 3",
        "",
    ])
    .prop_map(str::to_owned)
}

fn color() -> impl Strategy<Value = u32> {
    prop_oneof![9 => 0u32..=0xFF_FFFF, 1 => Just(0x1_000_000)]
}

fn tick() -> impl Strategy<Value = u32> {
    prop_oneof![
        24 => (0u32..64).prop_map(|step| step * 120),
        8 => 0u32..20_000,
        // Around the end of the longest pattern and of the longest song.
        1 => MAX_PATTERN_TICKS - 2..MAX_PATTERN_TICKS + 2,
        1 => MAX_SONG_TICKS - 2_000..MAX_SONG_TICKS + 2,
        2 => Just(u32::MAX),
    ]
}

fn length() -> impl Strategy<Value = u32> {
    prop_oneof![10 => 1u32..2000, 1 => Just(0), 1 => Just(u32::MAX)]
}

fn key() -> impl Strategy<Value = u8> {
    prop_oneof![12 => 0u8..=127, 1 => 128u8..=255]
}

fn position() -> impl Strategy<Value = u32> {
    prop_oneof![9 => 0u32..8, 1 => any::<u32>()]
}

fn sample_path() -> impl Strategy<Value = SamplePath> {
    let file = select(vec![
        "kick.wav",
        "drums/snare.wav",
        "drums/808/hat.wav",
        "",
        "../x.wav",
    ]);
    (0..3, file).prop_map(|(kind, file)| match kind {
        0 => SamplePath::Factory(file.to_owned()),
        1 => SamplePath::Project(file.to_owned()),
        _ => SamplePath::External(format!("/music/{file}")),
    })
}

fn settings_patch() -> impl Strategy<Value = SettingsPatch> {
    let tempo = prop_oneof![8 => 5.0f64..600.0, 1 => Just(120.0), 1 => Just(f64::NAN)];
    let numerator = prop_oneof![9 => 1u8..=16, 1 => select(vec![0u8, 17])];
    let denominator = prop_oneof![9 => select(vec![2u8, 4, 8, 16]), 1 => select(vec![0u8, 3])];
    let signature = (numerator, denominator).prop_map(|(numerator, denominator)| TimeSignature {
        numerator,
        denominator,
    });
    (
        maybe(name()),
        maybe(tempo),
        maybe(signature),
        maybe(level()),
    )
        .prop_map(|(name, tempo_bpm, time_signature, swing)| SettingsPatch {
            name,
            tempo_bpm,
            time_signature,
            swing,
        })
}

fn channel_patch() -> impl Strategy<Value = ChannelPatch> {
    (
        maybe(name()),
        maybe(color()),
        maybe(level()),
        maybe(level()),
        maybe(any::<bool>()),
        maybe(any::<bool>()),
    )
        .prop_map(|(name, color, volume, pan, muted, solo)| ChannelPatch {
            name,
            color,
            volume,
            pan,
            muted,
            solo,
            mixer_track: None,
        })
}

fn sampler_patch() -> impl Strategy<Value = SamplerPatch> {
    (
        maybe(key()),
        maybe(level()),
        maybe(level()),
        maybe(level()),
        maybe(level()),
        maybe(any::<bool>()),
        maybe(any::<bool>()),
        maybe(any::<u8>()),
        maybe(prop_oneof![
            Just(SamplerLoopMode::Off),
            Just(SamplerLoopMode::Forward),
            Just(SamplerLoopMode::PingPong)
        ]),
        maybe(level()),
        maybe(level()),
    )
        .prop_map(
            |(
                root_key,
                tune,
                gain,
                start,
                end,
                reverse,
                cut_self,
                cut_group,
                loop_mode,
                loop_start,
                loop_end,
            )| SamplerPatch {
                root_key,
                tune,
                gain,
                start,
                end,
                reverse,
                cut_self,
                cut_group,
                loop_mode,
                loop_start,
                loop_end,
            },
        )
}

fn envelope() -> impl Strategy<Value = Envelope> {
    (level(), level(), level(), level()).prop_map(|(attack, decay, sustain, release)| Envelope {
        attack_ms: attack * 100.0,
        decay_ms: decay * 100.0,
        sustain,
        release_ms: release * 100_000.0,
    })
}

fn pattern_patch() -> impl Strategy<Value = PatternPatch> {
    let steps = prop_oneof![8 => 1u32..64, 1 => Just(0), 1 => Just(5000)];
    (maybe(name()), maybe(color()), maybe(steps)).prop_map(|(name, color, length_steps)| {
        PatternPatch {
            name,
            color,
            length_steps,
        }
    })
}

fn note_init() -> impl Strategy<Value = NoteInit> {
    (tick(), length(), key(), maybe(level()), maybe(level())).prop_map(
        |(start, length, key, velocity, pan)| NoteInit {
            start,
            length,
            key,
            velocity,
            pan,
        },
    )
}

fn note_patch() -> impl Strategy<Value = NotePatch> {
    (
        maybe(tick()),
        maybe(length()),
        maybe(key()),
        maybe(level()),
        maybe(level()),
    )
        .prop_map(|(start, length, key, velocity, pan)| NotePatch {
            start,
            length,
            key,
            velocity,
            pan,
        })
}

fn mixer_track_patch() -> impl Strategy<Value = MixerTrackPatch> {
    (
        maybe(name()),
        maybe(color()),
        maybe(level()),
        maybe(level()),
        maybe(any::<bool>()),
        maybe(any::<bool>()),
    )
        .prop_map(|(name, color, volume, pan, muted, solo)| MixerTrackPatch {
            name,
            color,
            volume,
            pan,
            muted,
            solo,
        })
}

fn clip_patch() -> impl Strategy<Value = ClipPatch> {
    (
        maybe(tick()),
        maybe(length()),
        maybe(tick()),
        maybe(any::<bool>()),
    )
        .prop_map(|(start, length, offset, muted)| ClipPatch {
            track: None,
            start,
            length,
            offset,
            muted,
        })
}

fn clip_spec() -> impl Strategy<Value = ClipSpec> {
    let audio = (
        pick(),
        pick(),
        (level(), level(), level()),
        (tick(), tick()),
        any::<bool>(),
    )
        .prop_map(
            |(sample, mixer_track, levels, fades, reverse)| ContentSpec::Audio {
                sample,
                mixer_track,
                levels,
                fades,
                reverse,
            },
        );
    let content = prop_oneof![
        3 => pick().prop_map(ContentSpec::Pattern),
        2 => audio,
        3 => pick().prop_map(ContentSpec::Automation),
    ];
    (
        pick(),
        tick(),
        // An audio clip has no length of its own, so most are given one.
        prop::option::weighted(0.7, length()),
        maybe(tick()),
        maybe(any::<bool>()),
        content,
    )
        .prop_map(|(track, start, length, offset, muted, content)| ClipSpec {
            track,
            start,
            length,
            offset,
            muted,
            content,
        })
}

fn audio_clip_patch() -> impl Strategy<Value = AudioClipPatch> {
    (
        maybe(level()),
        maybe(level()),
        maybe(tick()),
        maybe(tick()),
        maybe(any::<bool>()),
        maybe(setting()),
    )
        .prop_map(
            |(gain, pan, fade_in, fade_out, reverse, pitch)| AudioClipPatch {
                mixer_track: None,
                stretch: None,
                gain,
                pan,
                fade_in,
                fade_out,
                reverse,
                pitch,
            },
        )
}

/// A curve: mostly a few points in order, now and then something the
/// document has to refuse.
fn points() -> impl Strategy<Value = Vec<AutomationPoint>> {
    let one =
        (0u32..8_000, level(), level(), any::<bool>()).prop_map(|(tick, value, curve, hold)| {
            AutomationPoint {
                tick,
                value,
                curve,
                hold,
            }
        });
    (
        vec(one, 0..6),
        prop::bool::weighted(0.9),
        prop::bool::weighted(0.05),
    )
        .prop_map(|(mut points, in_order, far)| {
            if in_order {
                points.sort_by_key(|point| point.tick);
            }
            if let (true, Some(last)) = (far, points.last_mut()) {
                last.tick = MAX_SONG_TICKS + 1;
            }
            points
        })
}

fn target_spec() -> impl Strategy<Value = TargetSpec> {
    prop_oneof![
        2 => pick().prop_map(TargetSpec::ChannelVolume),
        1 => pick().prop_map(TargetSpec::ChannelPan),
        2 => pick().prop_map(TargetSpec::TrackVolume),
        1 => pick().prop_map(TargetSpec::TrackPan),
        2 => (pick(), pick(), prop::bool::weighted(0.2))
            .prop_map(|(send, to, any)| TargetSpec::SendGain { send, to, any }),
        3 => (pick(), pick(), setting_index())
            .prop_map(|(track, effect, param)| TargetSpec::EffectParam(track, effect, param)),
        2 => (pick(), pick()).prop_map(|(track, effect)| TargetSpec::EffectMix(track, effect)),
        2 => (pick(), setting_index())
            .prop_map(|(channel, param)| TargetSpec::InstrumentParam(channel, param)),
        1 => Just(TargetSpec::Tempo),
    ]
}

/// Any command except a batch.
fn single_action() -> impl Strategy<Value = Action> {
    let step = prop_oneof![12 => 0u32..32, 1 => Just(u32::MAX)];
    prop_oneof![
        3 => settings_patch().prop_map(Action::UpdateSettings),
        2 => (name(), sample_path()).prop_map(|(name, path)| Action::AddSample(name, path)),
        1 => pick().prop_map(Action::RemoveSample),
        3 => (
            maybe(name()),
            prop::option::weighted(0.3, pick()),
            prop::option::weighted(0.4, Just(InstrumentKind::SubtractiveSynth)),
            maybe(position()),
            maybe(pick()),
        )
            .prop_map(|(name, sample, instrument, index, mixer_track)| Action::AddChannel {
                name,
                sample,
                instrument,
                index,
                mixer_track,
            }),
        1 => pick().prop_map(Action::RemoveChannel),
        1 => pick().prop_map(Action::DuplicateChannel),
        2 => (pick(), position()).prop_map(|(id, index)| Action::MoveChannel(id, index)),
        4 => (pick(), channel_patch(), maybe(pick()))
            .prop_map(|(id, patch, track)| Action::UpdateChannel(id, patch, track)),
        2 => (pick(), maybe(pick())).prop_map(|(id, sample)| Action::SetChannelSample(id, sample)),
        3 => (pick(), sampler_patch()).prop_map(|(id, patch)| Action::UpdateSampler(id, patch)),
        2 => (pick(), maybe(envelope()))
            .prop_map(|(id, envelope)| Action::SetSamplerEnvelope(id, envelope)),
        4 => (pick(), setting_index(), setting())
            .prop_map(|(id, param, value)| Action::SetInstrumentParam(id, param, value)),
        2 => (pick(), settings())
            .prop_map(|(id, settings)| Action::SetInstrumentParams(id, settings)),
        2 => maybe(name()).prop_map(Action::AddPattern),
        1 => pick().prop_map(Action::RemovePattern),
        1 => pick().prop_map(Action::DuplicatePattern),
        2 => (pick(), position()).prop_map(|(id, index)| Action::MovePattern(id, index)),
        2 => (pick(), pattern_patch()).prop_map(|(id, patch)| Action::UpdatePattern(id, patch)),
        6 => (pick(), pick(), step)
            .prop_map(|(pattern, channel, step)| Action::ToggleStep(pattern, channel, step)),
        4 => (pick(), pick(), vec(note_init(), 0..6))
            .prop_map(|(pattern, channel, notes)| Action::AddNotes(pattern, channel, notes)),
        3 => (pick(), pick(), vec(pick(), 0..4))
            .prop_map(|(pattern, channel, notes)| Action::RemoveNotes(pattern, channel, notes)),
        6 => (pick(), pick(), vec((pick(), note_patch()), 0..5)).prop_map(
            |(pattern, channel, updates)| Action::UpdateNotes(pattern, channel, updates)
        ),
        1 => (pick(), pick()).prop_map(|(pattern, channel)| Action::ClearLane(pattern, channel)),
        2 => maybe(name()).prop_map(Action::AddMixerTrack),
        1 => pick().prop_map(Action::RemoveMixerTrack),
        3 => (pick(), mixer_track_patch())
            .prop_map(|(id, patch)| Action::UpdateMixerTrack(id, patch)),
        3 => (pick(), maybe(pick())).prop_map(|(id, output)| Action::SetTrackOutput(id, output)),
        4 => (pick(), pick(), maybe(level()))
            .prop_map(|(from, to, gain)| Action::SetSend(from, to, gain)),
        4 => (pick(), effect_kind(), maybe(position()))
            .prop_map(|(track, kind, index)| Action::AddEffect(track, kind, index)),
        1 => (pick(), pick()).prop_map(|(track, effect)| Action::RemoveEffect(track, effect)),
        3 => (pick(), pick(), maybe(pick()), position()).prop_map(
            |(track, effect, to_track, index)| Action::MoveEffect(track, effect, to_track, index)
        ),
        3 => (pick(), pick(), effect_patch())
            .prop_map(|(track, effect, patch)| Action::UpdateEffect(track, effect, patch)),
        5 => (pick(), pick(), setting_index(), setting()).prop_map(
            |(track, effect, param, value)| Action::SetEffectParam(track, effect, param, value)
        ),
        2 => (
            pick(),
            pick(),
            prop::option::weighted(0.1, effect_kind()),
            settings(),
        )
            .prop_map(|(track, effect, kind, settings)| {
                Action::SetEffectParams(track, effect, kind, settings)
            }),
        2 => (pick(), pick())
            .prop_map(|(track, effect)| Action::DuplicateEffect(track, effect)),
        2 => (pick(), pick(), effect_kind())
            .prop_map(|(track, effect, kind)| Action::ReplaceEffect(track, effect, kind)),
        2 => (maybe(name()), maybe(position()))
            .prop_map(|(name, index)| Action::AddPlaylistTrack(name, index)),
        2 => (pick(), position()).prop_map(|(id, index)| Action::MovePlaylistTrack(id, index)),
        1 => pick().prop_map(Action::RemovePlaylistTrack),
        1 => (pick(), (maybe(name()), maybe(any::<bool>()))).prop_map(|(id, (name, muted))| {
            Action::UpdatePlaylistTrack(id, PlaylistTrackPatch { name, muted })
        }),
        4 => vec(clip_spec(), 0..4).prop_map(Action::AddClips),
        4 => vec((pick(), audio_clip_patch(), maybe(pick())), 0..4)
            .prop_map(Action::UpdateAudioClips),
        5 => (maybe(name()), target_spec(), maybe(points()))
            .prop_map(|(name, target, points)| Action::AddAutomation(name, target, points)),
        1 => pick().prop_map(Action::RemoveAutomation),
        2 => (pick(), (maybe(name()), maybe(color()))).prop_map(|(id, (name, color))| {
            Action::UpdateAutomation(id, AutomationPatch { name, color })
        }),
        5 => (pick(), points()).prop_map(|(id, points)| Action::SetAutomationPoints(id, points)),
        1 => pick().prop_map(Action::DuplicateAutomation),
        2 => vec(pick(), 0..3).prop_map(Action::RemoveClips),
        4 => vec((pick(), clip_patch(), maybe(pick())), 0..4).prop_map(Action::UpdateClips),
    ]
}

fn action() -> impl Strategy<Value = Action> {
    prop_oneof![
        9 => single_action(),
        1 => (maybe(name()), vec(single_action(), 0..5))
            .prop_map(|(label, actions)| Action::Batch(label, actions)),
    ]
}

#[derive(Debug, Clone)]
enum Step {
    Dispatch(Action, Option<u64>),
    Undo,
    Redo,
    Jump(Index),
    MarkSaved,
}

fn step() -> impl Strategy<Value = Step> {
    let gesture = prop_oneof![3 => Just(None), 3 => Just(Some(1u64)), 1 => Just(Some(2u64))];
    prop_oneof![
        14 => (action(), gesture).prop_map(|(action, gesture)| Step::Dispatch(action, gesture)),
        3 => Just(Step::Undo),
        2 => Just(Step::Redo),
        1 => any::<Index>().prop_map(Step::Jump),
        1 => Just(Step::MarkSaved),
    ]
}

/// A project with a little of everything, so the first random commands have
/// things to act on.
fn seed_project() -> Project {
    let mut doc = Document::new(Project::new("Seed"));
    let mut run = |command| doc.dispatch(command, None).expect("the seed is valid");
    let mut channels = Vec::new();
    let mut tracks = Vec::new();
    let mut project_sample = SampleId(0);
    for name in ["kick", "snare"] {
        let sample = run(Command::AddSample {
            name: name.to_owned(),
            path: SamplePath::Factory(format!("drums/{name}.wav")),
        });
        project_sample = SampleId(sample.created[0]);
        let channel = run(Command::AddChannel {
            name: None,
            sample: Some(SampleId(sample.created[0])),
            instrument: None,
            index: None,
            mixer_track: None,
        });
        channels.push(ChannelId(channel.created[0]));
        tracks.push(TrackId(channel.created[1]));
    }
    let lead = run(Command::AddChannel {
        name: Some("Lead".to_owned()),
        sample: None,
        instrument: Some(InstrumentKind::SubtractiveSynth),
        index: None,
        mixer_track: None,
    });
    let mut reverb = EffectId(0);
    for (track, kind) in [
        (TrackId(lead.created[1]), EffectKind::Reverb),
        (tracks[0], EffectKind::Compressor),
        (TrackId::MASTER, EffectKind::Limiter),
    ] {
        let added = run(Command::AddEffect {
            track,
            kind,
            index: None,
        });
        if kind == EffectKind::Reverb {
            reverb = EffectId(added.created[0]);
        }
    }
    let second = PatternId(run(Command::AddPattern { name: None }).created[0]);
    for (pattern, channel, step) in [
        (PatternId(1), channels[0], 0),
        (PatternId(1), channels[0], 4),
        (PatternId(1), channels[1], 4),
        (second, channels[1], 2),
    ] {
        run(Command::ToggleStep {
            pattern,
            channel,
            step,
        });
    }
    let bus = TrackId(run(Command::AddMixerTrack { name: None }).created[0]);
    run(Command::SetSend {
        from: tracks[0],
        to: bus,
        gain: Some(0.5),
    });
    let lane = PlaylistTrackId(
        run(Command::AddPlaylistTrack {
            name: None,
            index: None,
        })
        .created[0],
    );
    run(Command::AddClips {
        clips: vec![
            ClipInit {
                track: lane,
                start: 0,
                length: None,
                offset: None,
                muted: None,
                content: ClipContent::Pattern {
                    pattern: PatternId(1),
                },
            },
            ClipInit {
                track: lane,
                start: 3840,
                length: Some(1920),
                offset: None,
                muted: None,
                content: ClipContent::Pattern { pattern: second },
            },
            ClipInit {
                track: lane,
                start: 960,
                length: Some(2_400),
                offset: Some(120),
                muted: None,
                content: ClipContent::Audio {
                    sample: project_sample,
                    mixer_track: tracks[1],
                    gain: 0.8,
                    pan: -0.25,
                    fade_in: 240,
                    fade_out: 480,
                    reverse: false,
                    pitch: -2.0,

                    stretch: Default::default(),
                },
            },
        ],
    });
    // One automation of each kind of thing that can go away under it, each
    // with a clip, and one of them with two.
    let targets = [
        AutomationTarget::ChannelVolume {
            channel: channels[0],
        },
        AutomationTarget::TrackPan { track: tracks[1] },
        AutomationTarget::SendGain {
            track: tracks[0],
            target: bus,
        },
        AutomationTarget::EffectParam {
            track: TrackId(lead.created[1]),
            effect: reverb,
            param: 1,
        },
        AutomationTarget::EffectMix {
            track: TrackId(lead.created[1]),
            effect: reverb,
        },
        AutomationTarget::InstrumentParam {
            channel: ChannelId(lead.created[0]),
            param: 3,
        },
        AutomationTarget::Tempo,
    ];
    for (index, target) in targets.into_iter().enumerate() {
        let added = run(Command::AddAutomation {
            name: None,
            target,
            points: Some(vec![
                AutomationPoint {
                    tick: 0,
                    value: 0.25,
                    curve: 0.5,
                    hold: false,
                },
                AutomationPoint {
                    tick: 960 * (index as u32 + 1),
                    value: 0.75,
                    curve: 0.0,
                    hold: index % 2 == 0,
                },
            ]),
        });
        let automation = AutomationId(added.created[0]);
        let clip = |start: u32| ClipInit {
            track: lane,
            start,
            length: None,
            offset: None,
            muted: None,
            content: ClipContent::Automation { automation },
        };
        let clips = if index == 0 {
            vec![clip(0), clip(7_680)]
        } else {
            vec![clip(480 * index as u32)]
        };
        run(Command::AddClips { clips });
    }
    let project = doc.project().clone();
    project.check().expect("the seed is valid");
    project
}

/// Equal apart from `next_id`, which undo leaves alone.
fn same_content(a: &Project, b: &Project) -> bool {
    let mut b = b.clone();
    b.next_id = a.next_id;
    *a == b
}

fn pattern_ids(project: &Project) -> Vec<PatternId> {
    project.patterns.iter().map(|pattern| pattern.id).collect()
}

/// Sections that are not marked as touched must not have changed.
fn check_untouched(
    before: &Project,
    after: &Project,
    touched: &Touched,
) -> Result<(), TestCaseError> {
    if !touched.settings {
        prop_assert_eq!(&before.settings, &after.settings);
    }
    if !touched.samples {
        prop_assert_eq!(&before.samples, &after.samples);
    }
    if !touched.channels {
        prop_assert_eq!(&before.channels, &after.channels);
    }
    if !touched.mixer {
        prop_assert_eq!(&before.mixer, &after.mixer);
    }
    if !touched.playlist {
        prop_assert_eq!(&before.playlist, &after.playlist);
    }
    if !touched.automations {
        prop_assert_eq!(&before.automations, &after.automations);
    }
    if !touched.pattern_list {
        prop_assert_eq!(pattern_ids(before), pattern_ids(after));
    }
    for pattern in &after.patterns {
        if !touched.patterns.contains(&pattern.id) {
            prop_assert_eq!(before.pattern(pattern.id), Some(pattern));
        }
    }
    for (index, id) in touched.patterns.iter().enumerate() {
        prop_assert!(after.pattern(*id).is_some(), "a removed pattern is listed");
        prop_assert!(!touched.patterns[..index].contains(id), "listed twice");
    }
    Ok(())
}

/// A single command marks a section only when it really changed it.
fn check_all_touched_changed(
    before: &Project,
    after: &Project,
    touched: &Touched,
) -> Result<(), TestCaseError> {
    if touched.settings {
        prop_assert_ne!(&before.settings, &after.settings);
    }
    if touched.samples {
        prop_assert_ne!(&before.samples, &after.samples);
    }
    if touched.channels {
        prop_assert_ne!(&before.channels, &after.channels);
    }
    if touched.mixer {
        prop_assert_ne!(&before.mixer, &after.mixer);
    }
    if touched.playlist {
        prop_assert_ne!(&before.playlist, &after.playlist);
    }
    if touched.automations {
        prop_assert_ne!(&before.automations, &after.automations);
    }
    if touched.pattern_list {
        prop_assert_ne!(pattern_ids(before), pattern_ids(after));
    }
    for id in &touched.patterns {
        prop_assert_ne!(before.pattern(*id), after.pattern(*id));
    }
    Ok(())
}

/// What a UI does with a patch: replace the sections it carries.
fn apply_patch(mirror: &mut Project, patch: &ProjectPatch) {
    if let Some(settings) = &patch.settings {
        mirror.settings = settings.clone();
    }
    if let Some(samples) = &patch.samples {
        mirror.samples = samples.clone();
    }
    if let Some(channels) = &patch.channels {
        mirror.channels = channels.clone();
    }
    if let Some(mixer) = &patch.mixer {
        mirror.mixer = mixer.clone();
    }
    if let Some(playlist) = &patch.playlist {
        mirror.playlist = playlist.clone();
    }
    if let Some(automations) = &patch.automations {
        mirror.automations = automations.clone();
    }
    for pattern in &patch.patterns {
        match mirror.patterns.iter_mut().find(|p| p.id == pattern.id) {
            Some(known) => *known = pattern.clone(),
            None => mirror.patterns.push(pattern.clone()),
        }
    }
    if let Some(order) = &patch.pattern_order {
        let mut known = std::mem::take(&mut mirror.patterns);
        for id in order {
            if let Some(index) = known.iter().position(|pattern| pattern.id == *id) {
                mirror.patterns.push(known.swap_remove(index));
            }
        }
    }
}

fn run_steps(steps: Vec<Step>) -> Result<(), TestCaseError> {
    let mut doc = Document::new(seed_project());
    // A UI's copy of the project, kept up to date from patches alone.
    let mut mirror = doc.project().clone();
    // `states[n]` is the project when `n` history entries are applied.
    let mut states = vec![doc.project().clone()];
    // What the document's own bookkeeping should say.
    let mut saved = Some(0);
    let mut open_gesture: Option<u64> = None;

    for step in steps {
        let before = doc.project().clone();
        let history = doc.history();
        let cursor = history.cursor as usize;

        let touched = match step {
            Step::Dispatch(action, gesture) => {
                let command = action.resolve(&before);
                let is_batch = matches!(command, Command::Batch { .. });
                match doc.dispatch(command, gesture) {
                    Err(error) => {
                        prop_assert_eq!(doc.project(), &before, "a failed command left a mark");
                        prop_assert_eq!(doc.history(), history);
                        prop_assert!(!error.to_string().is_empty());
                        Touched::default()
                    }
                    Ok(applied) => {
                        let now = doc.history().cursor as usize;
                        let after = doc.project();
                        if applied.touched.is_empty() {
                            prop_assert!(same_content(&before, after));
                            prop_assert_eq!(doc.history(), history);
                            if open_gesture != gesture {
                                open_gesture = None;
                            }
                        } else if gesture.is_some() && open_gesture == gesture {
                            // Merged into the gesture's entry, which vanishes
                            // when the gesture has come back to its start.
                            prop_assert!(now == cursor || now + 1 == cursor);
                            if now < cursor {
                                prop_assert!(same_content(&states[now], after));
                                open_gesture = None;
                            }
                            prop_assert_eq!(doc.history().entries.len(), now);
                            states.truncate(now);
                            states.push(after.clone());
                        } else {
                            prop_assert_eq!(now, cursor + 1);
                            let entries = doc.history().entries;
                            prop_assert_eq!(entries.len(), now, "the redo steps remain");
                            prop_assert_eq!(&entries[cursor].label, &applied.label);
                            if saved.is_some_and(|saved| saved > cursor) {
                                saved = None;
                            }
                            states.truncate(now);
                            states.push(after.clone());
                            open_gesture = gesture;
                        }
                        prop_assert!(!applied.label.is_empty());
                        for id in &applied.created {
                            prop_assert!(*id < after.next_id);
                        }
                        if !is_batch {
                            check_all_touched_changed(&before, after, &applied.touched)?;
                        }
                        applied.touched
                    }
                }
            }
            Step::Undo => {
                open_gesture = None;
                let touched = doc.undo();
                prop_assert_eq!(touched.is_some(), cursor > 0);
                touched.unwrap_or_default()
            }
            Step::Redo => {
                open_gesture = None;
                let touched = doc.redo();
                prop_assert_eq!(touched.is_some(), cursor < history.entries.len());
                touched.unwrap_or_default()
            }
            Step::Jump(target) => {
                open_gesture = None;
                let target = target.index(history.entries.len() + 1);
                let touched = doc.jump(target as u32);
                prop_assert_eq!(doc.history().cursor as usize, target);
                touched
            }
            Step::MarkSaved => {
                open_gesture = None;
                doc.mark_saved();
                saved = Some(cursor);
                Touched::default()
            }
        };

        let after = doc.project().clone();
        let cursor = doc.history().cursor as usize;
        if let Err(problem) = after.check() {
            return Err(TestCaseError::fail(problem));
        }
        prop_assert!(after.next_id >= before.next_id, "next_id went back");
        prop_assert!(
            same_content(&states[cursor], &after),
            "the project is not what it was at this point of the history"
        );
        check_untouched(&before, &after, &touched)?;
        prop_assert_eq!(doc.is_dirty(), saved != Some(cursor));

        let revision = doc.revision();
        let patch = doc.patch(&touched);
        prop_assert_eq!(patch.revision, revision + 1);
        prop_assert_eq!(&patch.history, &doc.history());
        prop_assert_eq!(patch.dirty, doc.is_dirty());
        apply_patch(&mut mirror, &patch);
        prop_assert!(same_content(&mirror, &after), "the patches lost an edit");
    }

    // Undo everything, then redo everything.
    let end = doc.project().clone();
    let end_cursor = doc.history().cursor;
    while doc.undo().is_some() {}
    prop_assert!(same_content(&states[0], doc.project()));
    prop_assert_eq!(doc.history().cursor, 0);
    while doc.redo().is_some() {}
    prop_assert!(same_content(&states[states.len() - 1], doc.project()));
    doc.jump(end_cursor);
    prop_assert_eq!(doc.project(), &end);
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn any_run_of_edits_keeps_every_promise(steps in vec(step(), 1..60)) {
        run_steps(steps)?;
    }

    /// Dispatches of one gesture make at most one undo step, and end up in
    /// the same place as the same commands dispatched one by one.
    #[test]
    fn one_gesture_is_one_undo_step(actions in vec(action(), 1..40)) {
        let mut merged = Document::new(seed_project());
        let mut separate = merged.clone();
        let original = merged.project().clone();

        for action in actions {
            let command = action.resolve(merged.project());
            let one = merged.dispatch(command.clone(), Some(9));
            let other = separate.dispatch(command, None);
            prop_assert_eq!(one, other);
            prop_assert_eq!(merged.project(), separate.project());
            prop_assert!(merged.history().entries.len() <= 1);
            prop_assert_eq!(merged.is_dirty(), !merged.history().entries.is_empty());
        }

        let end = merged.project().clone();
        if merged.history().entries.is_empty() {
            prop_assert!(same_content(&original, &end));
        } else {
            prop_assert!(merged.undo().is_some());
            prop_assert!(same_content(&original, merged.project()));
            prop_assert!(merged.undo().is_none());
            prop_assert!(!merged.is_dirty());
            prop_assert!(merged.redo().is_some());
            prop_assert_eq!(merged.project(), &end);
        }

        // The steps taken one by one unwind to the same start.
        while separate.undo().is_some() {}
        prop_assert!(same_content(&original, separate.project()));
    }

    #[test]
    fn a_project_survives_its_json(actions in vec(action(), 0..40)) {
        let mut doc = Document::new(seed_project());
        for action in actions {
            let command = action.resolve(doc.project());
            let _ = doc.dispatch(command, None);
        }
        let json = to_json(doc.project()).unwrap();
        let loaded = match from_json(&json) {
            Ok(loaded) => loaded,
            Err(error) => return Err(TestCaseError::fail(error.to_string())),
        };
        prop_assert_eq!(&loaded, doc.project());
        // Encoding is stable: the loaded project writes the same text.
        prop_assert_eq!(to_json(&loaded).unwrap(), json);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    /// Relinking a sample is not an edit, and it reaches the sample
    /// wherever the history holds it: every point of the history is a
    /// valid project, and the sample has its new path at each of them.
    #[test]
    fn a_relinked_sample_has_its_new_path_at_every_point_of_the_history(
        actions in vec(action(), 0..40),
        undos in 0usize..12,
        chosen in any::<Index>(),
    ) {
        let mut doc = Document::new(seed_project());
        for action in actions {
            let command = action.resolve(doc.project());
            let _ = doc.dispatch(command, None);
        }
        for _ in 0..undos {
            doc.undo();
        }
        let samples: Vec<SampleId> = doc.project().samples.iter().map(|s| s.id).collect();
        if samples.is_empty() {
            return Ok(());
        }
        let id = *chosen.get(&samples);
        // No generated sample has this path.
        let path = SamplePath::Project("moved/relinked.wav".to_owned());

        let before = doc.project().clone();
        let history = doc.history();
        let dirty = doc.is_dirty();
        let touched = match doc.relink_sample(id, path.clone()) {
            Ok(touched) => touched,
            Err(error) => return Err(TestCaseError::fail(error.to_string())),
        };
        prop_assert!(touched.samples);
        prop_assert_eq!(doc.history(), history.clone());
        prop_assert_eq!(doc.is_dirty(), dirty);
        // Only that sample's path is different.
        let mut expected = before;
        for sample in &mut expected.samples {
            if sample.id == id {
                sample.path = path.clone();
            }
        }
        prop_assert_eq!(doc.project(), &expected);

        for cursor in (0..=history.entries.len() as u32).rev().chain(0..=history.entries.len() as u32) {
            doc.jump(cursor);
            if let Err(problem) = doc.project().check() {
                return Err(TestCaseError::fail(problem));
            }
            if let Some(sample) = doc.project().sample(id) {
                prop_assert_eq!(&sample.path, &path);
            }
        }
        doc.jump(history.cursor);
        prop_assert_eq!(doc.project(), &expected);
        prop_assert_eq!(doc.is_dirty(), dirty);
    }
}

proptest! {
    // Each case writes a file, so this one runs fewer of them.
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn a_project_survives_save_and_load(actions in vec(action(), 0..40)) {
        let mut doc = Document::new(seed_project());
        for action in actions {
            let command = action.resolve(doc.project());
            let _ = doc.dispatch(command, None);
        }
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("Song.windfall");
        save(doc.project(), &path).unwrap();
        let loaded = match load(&path) {
            Ok(loaded) => loaded,
            Err(error) => return Err(TestCaseError::fail(error.to_string())),
        };
        prop_assert_eq!(&loaded, doc.project());
    }
}
