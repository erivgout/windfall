//! Turns commands into primitive edits.
//!
//! Each command checks everything it needs before it makes its first edit,
//! so a command that fails has changed nothing. A batch runs its commands one
//! after another against the live project, so a later command sees what an
//! earlier one did; when one fails, the edits made so far are taken back.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use crate::automation::AutomationRange;
use crate::check::{reaches, time_signature_problem};
use crate::command::{
    AudioClipPatch, AudioClipUpdate, AutomationPatch, ChannelPatch, ClipInit, ClipPatch,
    ClipUpdate, Command, EffectSlotPatch, MixerTrackPatch, NoteInit, NotePatch, NoteUpdate,
    PatternPatch, PlaylistTrackPatch, SamplerPatch, SettingsPatch,
};
use crate::edit::{self, Change, Direction, Edit, ListEdit, Move, PatternInfo};
use crate::error::CommandError;
use windfall_dsp::ParamInfo;

use crate::model::{
    Automation, AutomationId, AutomationPoint, AutomationTarget, MAX_AUTOMATION_POINTS,
};
use crate::model::{
    Channel, ChannelId, ChannelSource, Clip, ClipContent, ClipId, DEFAULT_CHANNEL_VOLUME,
    DEFAULT_KEY, DEFAULT_PATTERN_STEPS, DEFAULT_VELOCITY, EffectId, EffectKind, EffectParams,
    EffectSlot, Envelope, InstrumentKind, InstrumentParams, Lane, MAX_EFFECT_SLOTS,
    MAX_ENVELOPE_MS, MAX_GAIN, MAX_KEY, MAX_MIXER_TRACKS, MAX_PATTERN_STEPS, MAX_PATTERN_TICKS,
    MAX_SONG_TICKS, MAX_TEMPO_BPM, MAX_TUNE_SEMITONES, MIN_TEMPO_BPM, MixerTrack, Note, NoteId,
    PPQ, Pattern, PatternId, PlaylistTrack, PlaylistTrackId, Project, SampleAsset, SampleId,
    SamplePath, SamplerSettings, Send, TICKS_PER_STEP, TrackId, palette_color,
};

type Label = &'static str;

/// What a command did to the project.
pub(crate) struct Outcome {
    /// The edits that were applied, in order. Empty when nothing changed.
    pub edits: Vec<Edit>,
    pub created: Vec<u32>,
    pub label: String,
}

/// Runs a command against the project. On failure the project is exactly as
/// it was, `next_id` included.
pub(crate) fn execute(project: &mut Project, command: Command) -> Result<Outcome, CommandError> {
    let first_id = project.next_id;
    let mut transaction = Transaction {
        project,
        edits: Vec::new(),
        created: Vec::new(),
    };
    match transaction.run(command) {
        Ok(label) => Ok(Outcome {
            edits: transaction.edits,
            created: transaction.created,
            label,
        }),
        Err(error) => {
            for edit in transaction.edits.iter().rev() {
                edit.apply(transaction.project, Direction::Backward);
            }
            transaction.project.next_id = first_id;
            Err(error)
        }
    }
}

struct Transaction<'a> {
    project: &'a mut Project,
    edits: Vec<Edit>,
    created: Vec<u32>,
}

impl Transaction<'_> {
    fn push(&mut self, edit: Edit) {
        if edit.is_identity() {
            return;
        }
        edit.apply(self.project, Direction::Forward);
        edit::push(&mut self.edits, edit);
    }

    fn allocate(&mut self) -> Result<u32, CommandError> {
        let id = self.project.next_id;
        self.project.next_id = id
            .checked_add(1)
            .ok_or_else(|| CommandError::invalid("the project has run out of ids"))?;
        Ok(id)
    }

    fn run(&mut self, command: Command) -> Result<String, CommandError> {
        let label = match command {
            Command::AddPluginInstrument { mut plugin } => {
                plugin.validate().map_err(CommandError::invalid)?;
                let first = self.created.len();
                self.add_channel(
                    Some(plugin.name.clone()),
                    None,
                    Some(InstrumentKind::SubtractiveSynth),
                    None,
                    None,
                )?;
                plugin.target = crate::PluginTarget::Instrument {
                    channel: ChannelId(self.created[first]),
                };
                self.bind_plugin(plugin);
                "Add plugin instrument"
            }
            Command::AddPluginEffect { track, mut plugin } => {
                plugin.validate().map_err(CommandError::invalid)?;
                self.add_effect(track, EffectKind::Eq, None)?;
                plugin.target = crate::PluginTarget::Effect {
                    effect: EffectId(
                        *self
                            .created
                            .last()
                            .ok_or_else(|| CommandError::invalid("the effect was not created"))?,
                    ),
                };
                self.bind_plugin(plugin);
                "Add plugin effect"
            }
            Command::SetPluginParam { target, id, value } => {
                self.plugin_param(target, id, value)?;
                "Change plugin parameter"
            }
            Command::SetPluginState { target, state } => {
                let mut plugins = self.project.plugins.clone();
                let plugin = plugins
                    .iter_mut()
                    .find(|plugin| plugin.target == target)
                    .ok_or_else(|| CommandError::invalid("the plugin is not in this project"))?;
                plugin.state = state;
                plugin.validate().map_err(CommandError::invalid)?;
                self.push(Edit::Plugins(Change {
                    old: self.project.plugins.clone(),
                    new: plugins,
                }));
                "Save plugin state"
            }
            Command::UpdateSettings { patch } => self.update_settings(patch)?,
            Command::AddSample { name, path } => self.add_sample(name, path)?,
            Command::RemoveSample { id } => self.remove_sample(id)?,
            Command::AddChannel {
                name,
                sample,
                instrument,
                index,
                mixer_track,
            } => self.add_channel(name, sample, instrument, index, mixer_track)?,
            Command::RemoveChannel { id } => self.remove_channel(id)?,
            Command::DuplicateChannel { id } => self.duplicate_channel(id)?,
            Command::MoveChannel { id, index } => self.move_channel(id, index)?,
            Command::UpdateChannel { id, patch } => self.update_channel(id, patch)?,
            Command::SetChannelSample { id, sample } => self.set_channel_sample(id, sample)?,
            Command::UpdateSampler { id, patch } => self.update_sampler(id, patch)?,
            Command::SetSamplerEnvelope { id, envelope } => {
                self.set_sampler_envelope(id, envelope)?
            }
            Command::SetInstrumentParam {
                channel,
                param,
                value,
            } => return self.set_instrument_param(channel, param, value),
            Command::SetInstrumentParams { channel, params } => {
                self.set_instrument_params(channel, params)?
            }
            Command::AddPattern { name } => self.add_pattern(name)?,
            Command::RemovePattern { id } => self.remove_pattern(id)?,
            Command::DuplicatePattern { id } => self.duplicate_pattern(id)?,
            Command::MovePattern { id, index } => self.move_pattern(id, index)?,
            Command::UpdatePattern { id, patch } => self.update_pattern(id, patch)?,
            Command::ToggleStep {
                pattern,
                channel,
                step,
            } => self.toggle_step(pattern, channel, step)?,
            Command::AddNotes {
                pattern,
                channel,
                notes,
            } => self.add_notes(pattern, channel, &notes)?,
            Command::RemoveNotes {
                pattern,
                channel,
                notes,
            } => self.remove_notes(pattern, channel, &notes)?,
            Command::UpdateNotes {
                pattern,
                channel,
                updates,
            } => self.update_notes(pattern, channel, &updates)?,
            Command::ClearLane { pattern, channel } => self.clear_lane(pattern, channel)?,
            Command::TransformNotes {
                pattern,
                channel,
                notes,
                transform,
            } => self.transform_notes(pattern, channel, &notes, transform)?,
            Command::AddMixerTrack { name } => self.add_mixer_track(name)?,
            Command::RemoveMixerTrack { id } => self.remove_mixer_track(id)?,
            Command::UpdateMixerTrack { id, patch } => self.update_mixer_track(id, patch)?,
            Command::SetTrackOutput { id, output } => self.set_track_output(id, output)?,
            Command::SetSend { from, to, gain } => self.set_send(from, to, gain)?,
            Command::AddEffect { track, kind, index } => self.add_effect(track, kind, index)?,
            Command::RemoveEffect { track, effect } => self.remove_effect(track, effect)?,
            Command::MoveEffect {
                track,
                effect,
                to_track,
                index,
            } => self.move_effect(track, effect, to_track, index)?,
            Command::UpdateEffect {
                track,
                effect,
                patch,
            } => self.update_effect(track, effect, patch)?,
            Command::SetEffectParam {
                track,
                effect,
                param,
                value,
            } => return self.set_effect_param(track, effect, param, value),
            Command::SetEffectParams {
                track,
                effect,
                params,
            } => self.set_effect_params(track, effect, params)?,
            Command::DuplicateEffect { track, effect } => self.duplicate_effect(track, effect)?,
            Command::ReplaceEffect {
                track,
                effect,
                kind,
            } => self.replace_effect(track, effect, kind)?,
            Command::AddPlaylistTrack { name, index } => self.add_playlist_track(name, index)?,
            Command::RemovePlaylistTrack { id } => self.remove_playlist_track(id)?,
            Command::UpdatePlaylistTrack { id, patch } => self.update_playlist_track(id, patch)?,
            Command::MovePlaylistTrack { id, index } => self.move_playlist_track(id, index)?,
            Command::AddClips { clips } => self.add_clips(&clips)?,
            Command::RemoveClips { clips } => self.remove_clips(&clips)?,
            Command::UpdateClips { updates } => self.update_clips(&updates)?,
            Command::UpdateAudioClips { updates } => self.update_audio_clips(&updates)?,
            Command::AddAutomation {
                name,
                target,
                points,
            } => self.add_automation(name, target, points)?,
            Command::RemoveAutomation { id } => self.remove_automation(id)?,
            Command::UpdateAutomation { id, patch } => self.update_automation(id, patch)?,
            Command::SetAutomationPoints { id, points } => {
                self.set_automation_points(id, points)?
            }
            Command::DuplicateAutomation { id } => self.duplicate_automation(id)?,
            Command::Batch { label, commands } => {
                let mut first = None;
                for command in commands {
                    let label = self.run(command)?;
                    first.get_or_insert(label);
                }
                // A blank label would be a blank line in the history menu.
                let label = label.filter(|label| !label.trim().is_empty());
                return Ok(label.or(first).unwrap_or_else(|| "Edit".to_owned()));
            }
        };
        self.prune_plugins();
        Ok(label.to_owned())
    }

    fn bind_plugin(&mut self, plugin: crate::PluginBinding) {
        let mut plugins = self.project.plugins.clone();
        plugins.retain(|other| other.target != plugin.target);
        plugins.push(plugin);
        self.push(Edit::Plugins(Change {
            old: self.project.plugins.clone(),
            new: plugins,
        }));
    }

    fn plugin_param(
        &mut self,
        target: crate::PluginTarget,
        id: u32,
        value: f32,
    ) -> Result<(), CommandError> {
        if !value.is_finite() {
            return Err(CommandError::invalid("the plugin parameter must be finite"));
        }
        let mut plugins = self.project.plugins.clone();
        let plugin = plugins
            .iter_mut()
            .find(|plugin| plugin.target == target)
            .ok_or_else(|| CommandError::invalid("the plugin is not in this project"))?;
        let param = plugin
            .parameters
            .iter_mut()
            .find(|param| param.id == id && !param.read_only)
            .ok_or_else(|| CommandError::invalid("the plugin parameter cannot be changed"))?;
        let value = value.clamp(param.min, param.max);
        param.value = if param.stepped {
            value.round().clamp(param.min, param.max)
        } else {
            value
        };
        self.push(Edit::Plugins(Change {
            old: self.project.plugins.clone(),
            new: plugins,
        }));
        Ok(())
    }

    fn prune_plugins(&mut self) {
        let plugins = self
            .project
            .plugins
            .iter()
            .filter(|plugin| match plugin.target {
                crate::PluginTarget::Instrument { channel } => {
                    self.project.channels.iter().any(|c| {
                        c.id == channel && matches!(c.source, ChannelSource::Instrument { .. })
                    })
                }
                crate::PluginTarget::Effect { effect } => self
                    .project
                    .mixer
                    .tracks
                    .iter()
                    .any(|track| track.effects.iter().any(|slot| slot.id == effect)),
            })
            .cloned()
            .collect();
        self.push(Edit::Plugins(Change {
            old: self.project.plugins.clone(),
            new: plugins,
        }));
    }

    fn update_settings(&mut self, patch: SettingsPatch) -> Result<Label, CommandError> {
        let label = single_label(
            &[
                (patch.name.is_some(), "Rename project"),
                (patch.tempo_bpm.is_some(), "Change tempo"),
                (patch.time_signature.is_some(), "Change time signature"),
                (patch.swing.is_some(), "Change swing"),
            ],
            "Change project settings",
        );
        let old = self.project.settings.clone();
        let mut new = old.clone();
        if let Some(name) = patch.name {
            new.name = name;
        }
        if let Some(tempo) = patch.tempo_bpm {
            if tempo.is_nan() {
                return Err(CommandError::invalid("the tempo is not a number"));
            }
            new.tempo_bpm = tempo.clamp(MIN_TEMPO_BPM, MAX_TEMPO_BPM);
        }
        if let Some(signature) = patch.time_signature {
            if let Some(problem) = time_signature_problem(signature) {
                return Err(CommandError::invalid(problem));
            }
            new.time_signature = signature;
        }
        if let Some(swing) = patch.swing {
            new.swing = clamped("the swing", swing, 0.0, 1.0)?;
        }
        self.push(Edit::Settings(Change { old, new }));
        Ok(label)
    }

    fn add_sample(&mut self, name: String, path: SamplePath) -> Result<Label, CommandError> {
        let label = "Add sample";
        if let Some(problem) = path.problem() {
            return Err(CommandError::invalid(format!(
                "the sample path is not valid: {problem}"
            )));
        }
        if let Some(existing) = self.project.samples.iter().find(|s| s.path == path) {
            self.created.push(existing.id.0);
            return Ok(label);
        }
        let id = SampleId(self.allocate()?);
        self.push(Edit::Sample(ListEdit::Insert {
            index: self.project.samples.len(),
            item: SampleAsset { id, name, path },
        }));
        self.created.push(id.0);
        Ok(label)
    }

    fn remove_sample(&mut self, id: SampleId) -> Result<Label, CommandError> {
        let index = self.sample_index(id)?;
        let channels = &self.project.channels;
        let user = channels.iter().find(|c| c.source.sample() == Some(id));
        if let Some(channel) = user {
            return Err(CommandError::invalid(format!(
                "the sample is still used by the channel \"{}\"",
                channel.name
            )));
        }
        let playlist = &self.project.playlist;
        let clip = playlist
            .clips
            .iter()
            .find(|c| c.content.sample() == Some(id));
        if let Some(clip) = clip {
            let track = playlist.tracks.iter().find(|t| t.id == clip.track);
            return Err(CommandError::invalid(format!(
                "the sample is still used by an audio clip on the playlist track \"{}\"",
                track.map_or("", |track| track.name.as_str())
            )));
        }
        let item = self.project.samples[index].clone();
        self.push(Edit::Sample(ListEdit::Remove { index, item }));
        Ok("Delete sample")
    }

    fn add_channel(
        &mut self,
        name: Option<String>,
        sample: Option<SampleId>,
        instrument: Option<InstrumentKind>,
        index: Option<u32>,
        mixer_track: Option<TrackId>,
    ) -> Result<Label, CommandError> {
        let sample_name = match sample {
            Some(id) => Some(self.project.samples[self.sample_index(id)?].name.clone()),
            None => None,
        };
        if sample.is_some() && instrument.is_some() {
            return Err(CommandError::invalid(
                "a channel plays a sample or an instrument, not both",
            ));
        }
        if let Some(track) = mixer_track {
            self.mixer_track_index(track)?;
        }
        let source = match instrument {
            Some(kind) => ChannelSource::Instrument {
                params: kind.default_params(),
            },
            None => ChannelSource::Sampler(SamplerSettings {
                sample,
                ..SamplerSettings::default()
            }),
        };
        let name = name.or(sample_name).unwrap_or_else(|| {
            let fallback = instrument.map_or("Sampler", InstrumentKind::name);
            fallback.to_owned()
        });
        let count = self.project.channels.len();
        let color = palette_color(count);
        let index = index.map_or(count, |index| (index as usize).min(count));

        let id = ChannelId(self.allocate()?);
        let track_count = self.project.mixer.tracks.len();
        let (track, made_track) = match mixer_track {
            Some(track) => (track, false),
            None if track_count < MAX_MIXER_TRACKS => (TrackId(self.allocate()?), true),
            // A full mixer must not stop the user adding channels.
            None => (TrackId::MASTER, false),
        };
        if made_track {
            self.push(Edit::MixerTrack(ListEdit::Insert {
                index: track_count,
                item: new_mixer_track(track, name.clone(), color),
            }));
        }
        self.push(Edit::Channel(Box::new(ListEdit::Insert {
            index,
            item: Channel {
                id,
                name,
                color,
                volume: DEFAULT_CHANNEL_VOLUME,
                pan: 0.0,
                muted: false,
                solo: false,
                mixer_track: track,
                source,
            },
        })));
        self.created.push(id.0);
        if made_track {
            self.created.push(track.0);
        }
        Ok("Add channel")
    }

    fn remove_channel(&mut self, id: ChannelId) -> Result<Label, CommandError> {
        let index = self.channel_index(id)?;
        self.remove_automations_of(|target| match *target {
            AutomationTarget::ChannelVolume { channel }
            | AutomationTarget::ChannelPan { channel }
            | AutomationTarget::InstrumentParam { channel, .. } => channel == id,
            _ => false,
        });
        for (pattern, notes) in self.lanes_of(id) {
            self.push(Edit::Notes {
                pattern,
                channel: id,
                remove: notes,
                insert: Vec::new(),
            });
        }
        let item = self.project.channels[index].clone();
        self.push(Edit::Channel(Box::new(ListEdit::Remove { index, item })));
        Ok("Delete channel")
    }

    fn duplicate_channel(&mut self, id: ChannelId) -> Result<Label, CommandError> {
        let index = self.channel_index(id)?;
        let original = &self.project.channels[index];
        let name = copy_name(
            &original.name,
            self.project.channels.iter().map(|c| c.name.as_str()),
        );
        let mut copy = Channel {
            name,
            ..original.clone()
        };
        let mut lanes = self.lanes_of(id);

        copy.id = ChannelId(self.allocate()?);
        for (_, notes) in &mut lanes {
            for note in notes {
                note.id = NoteId(self.allocate()?);
            }
        }
        let copy_id = copy.id;
        self.push(Edit::Channel(Box::new(ListEdit::Insert {
            index: index + 1,
            item: copy,
        })));
        for (pattern, notes) in lanes {
            self.push(Edit::Notes {
                pattern,
                channel: copy_id,
                remove: Vec::new(),
                insert: notes,
            });
        }
        self.created.push(copy_id.0);
        if let Some(mut plugin) = self
            .project
            .plugin(crate::PluginTarget::Instrument { channel: id })
            .cloned()
        {
            plugin.target = crate::PluginTarget::Instrument { channel: copy_id };
            self.bind_plugin(plugin);
        }
        Ok("Duplicate channel")
    }

    fn move_channel(&mut self, id: ChannelId, index: u32) -> Result<Label, CommandError> {
        let from = self.channel_index(id)?;
        let to = (index as usize).min(self.project.channels.len() - 1);
        self.push(Edit::MoveChannel(Move { from, to }));
        Ok("Move channel")
    }

    fn update_channel(
        &mut self,
        id: ChannelId,
        patch: ChannelPatch,
    ) -> Result<Label, CommandError> {
        let label = single_label(
            &[
                (patch.name.is_some(), "Rename channel"),
                (patch.color.is_some(), "Change channel color"),
                (patch.volume.is_some(), "Change channel volume"),
                (patch.pan.is_some(), "Change channel pan"),
                (patch.muted == Some(true), "Mute channel"),
                (patch.muted == Some(false), "Unmute channel"),
                (patch.solo == Some(true), "Solo channel"),
                (patch.solo == Some(false), "Unsolo channel"),
                (patch.mixer_track.is_some(), "Route channel"),
            ],
            "Change channel",
        );
        let old = self.project.channels[self.channel_index(id)?].clone();
        let mut new = old.clone();
        if let Some(name) = patch.name {
            new.name = name;
        }
        if let Some(color) = patch.color {
            new.color = checked_color(color)?;
        }
        if let Some(volume) = patch.volume {
            new.volume = clamped("the volume", volume, 0.0, MAX_GAIN)?;
        }
        if let Some(pan) = patch.pan {
            new.pan = clamped("the pan", pan, -1.0, 1.0)?;
        }
        if let Some(muted) = patch.muted {
            new.muted = muted;
        }
        if let Some(solo) = patch.solo {
            new.solo = solo;
        }
        if let Some(track) = patch.mixer_track {
            self.mixer_track_index(track)?;
            new.mixer_track = track;
        }
        // The track made for the channel goes by the channel's name, for
        // as long as the channel stays on it.
        let renamed = new.name != old.name && new.mixer_track == old.mixer_track;
        if let Some(track) = self.track_named_after(&old).filter(|_| renamed) {
            let name = new.name.clone();
            self.change_track(track, |track| track.name = name);
        }
        self.push(Edit::SetChannel(Box::new(Change { old, new })));
        Ok(label)
    }

    fn set_channel_sample(
        &mut self,
        id: ChannelId,
        sample: Option<SampleId>,
    ) -> Result<Label, CommandError> {
        let index = self.channel_index(id)?;
        let mut sampler = self.sampler(index)?;
        if let Some(sample) = sample {
            self.sample_index(sample)?;
        }
        sampler.sample = sample;
        self.set_sampler(index, sampler);
        Ok("Change channel sample")
    }

    fn update_sampler(
        &mut self,
        id: ChannelId,
        patch: SamplerPatch,
    ) -> Result<Label, CommandError> {
        let label = single_label(
            &[
                (patch.root_key.is_some(), "Change root key"),
                (patch.tune.is_some(), "Change tuning"),
                (patch.gain.is_some(), "Change sample gain"),
                (
                    patch.start.is_some() || patch.end.is_some(),
                    "Change sample range",
                ),
                (patch.reverse.is_some(), "Reverse sample"),
                (patch.loop_mode.is_some(), "Change sample loop mode"),
                (
                    patch.loop_start.is_some() || patch.loop_end.is_some(),
                    "Change sample loop range",
                ),
                (patch.cut_self.is_some(), "Toggle cut itself"),
                (patch.cut_group.is_some(), "Change cut group"),
            ],
            "Change sampler",
        );
        let index = self.channel_index(id)?;
        let mut sampler = self.sampler(index)?;
        if let Some(root_key) = patch.root_key {
            sampler.root_key = checked_key(root_key)?;
        }
        if let Some(tune) = patch.tune {
            let range = MAX_TUNE_SEMITONES;
            sampler.tune = clamped("the tuning", tune, -range, range)?;
        }
        if let Some(gain) = patch.gain {
            sampler.gain = clamped("the sample gain", gain, 0.0, MAX_GAIN)?;
        }
        if let Some(start) = patch.start {
            sampler.start = clamped("the sample start", start, 0.0, 1.0)?;
        }
        if let Some(end) = patch.end {
            sampler.end = clamped("the sample end", end, 0.0, 1.0)?;
        }
        if sampler.start >= sampler.end {
            return Err(CommandError::invalid(
                "the sample start must come before its end",
            ));
        }
        if let Some(reverse) = patch.reverse {
            sampler.reverse = reverse;
        }
        if let Some(mode) = patch.loop_mode {
            sampler.loop_mode = mode;
        }
        if let Some(start) = patch.loop_start {
            if !start.is_finite() {
                return Err(CommandError::invalid("the loop start is not a number"));
            }
            sampler.loop_start = clamped("the loop start", start, 0.0, 1.0)?;
        }
        if let Some(end) = patch.loop_end {
            if !end.is_finite() {
                return Err(CommandError::invalid("the loop end is not a number"));
            }
            sampler.loop_end = clamped("the loop end", end, 0.0, 1.0)?;
        }
        if sampler.loop_start >= sampler.loop_end {
            return Err(CommandError::invalid(
                "the loop start must come before its end",
            ));
        }
        if let Some(cut_self) = patch.cut_self {
            sampler.cut_self = cut_self;
        }
        if let Some(cut_group) = patch.cut_group {
            sampler.cut_group = cut_group;
        }
        self.set_sampler(index, sampler);
        Ok(label)
    }

    fn set_sampler_envelope(
        &mut self,
        id: ChannelId,
        envelope: Option<Envelope>,
    ) -> Result<Label, CommandError> {
        let index = self.channel_index(id)?;
        let mut sampler = self.sampler(index)?;
        sampler.envelope = match envelope {
            Some(envelope) => Some(Envelope {
                attack_ms: clamped("the attack", envelope.attack_ms, 0.0, MAX_ENVELOPE_MS)?,
                decay_ms: clamped("the decay", envelope.decay_ms, 0.0, MAX_ENVELOPE_MS)?,
                sustain: clamped("the sustain", envelope.sustain, 0.0, 1.0)?,
                release_ms: clamped("the release", envelope.release_ms, 0.0, MAX_ENVELOPE_MS)?,
            }),
            None => None,
        };
        self.set_sampler(index, sampler);
        Ok("Change envelope")
    }

    fn set_instrument_param(
        &mut self,
        channel: ChannelId,
        param: u32,
        value: f32,
    ) -> Result<String, CommandError> {
        let target = crate::PluginTarget::Instrument { channel };
        if let Some(plugin) = self.project.plugin(target) {
            let parameter = plugin
                .parameters
                .get(param as usize)
                .ok_or_else(|| CommandError::invalid("the plugin parameter does not exist"))?;
            let id = parameter.id;
            let label = format!("Change {}", parameter.name);
            self.plugin_param(target, id, value)?;
            return Ok(label);
        }
        let index = self.channel_index(channel)?;
        let mut params = self.instrument(index)?;
        let kind = params.kind();
        if !params.set(param as usize, checked_setting(value)?) {
            return Err(no_setting(kind.name(), param));
        }
        self.set_instrument(index, params);
        Ok(setting_label(kind.descriptors(), param))
    }

    fn set_instrument_params(
        &mut self,
        channel: ChannelId,
        params: InstrumentParams,
    ) -> Result<Label, CommandError> {
        if self
            .project
            .plugin(crate::PluginTarget::Instrument { channel })
            .is_some()
        {
            return Err(CommandError::invalid("use the hosted plugin's parameters"));
        }
        let index = self.channel_index(channel)?;
        let current = self.instrument(index)?;
        if params.kind() != current.kind() {
            return Err(other_kind(params.kind().name(), current.kind().name()));
        }
        self.set_instrument(index, checked_instrument(params)?);
        Ok("Change instrument settings")
    }

    fn add_pattern(&mut self, name: Option<String>) -> Result<Label, CommandError> {
        let count = self.project.patterns.len();
        let name = name.unwrap_or_else(|| {
            numbered_name(
                "Pattern",
                count + 1,
                self.project.patterns.iter().map(|p| p.name.as_str()),
            )
        });
        let id = PatternId(self.allocate()?);
        self.push(Edit::Pattern(ListEdit::Insert {
            index: count,
            item: Pattern {
                id,
                name,
                color: palette_color(count),
                length_steps: DEFAULT_PATTERN_STEPS,
                lanes: Vec::new(),
            },
        }));
        self.created.push(id.0);
        Ok("Add pattern")
    }

    fn remove_pattern(&mut self, id: PatternId) -> Result<Label, CommandError> {
        let index = self.pattern_index(id)?;
        if self.project.patterns.len() == 1 {
            return Err(CommandError::invalid(
                "a project needs at least one pattern",
            ));
        }
        let clips = self.clips_where(
            |clip| matches!(clip.content, ClipContent::Pattern { pattern } if pattern == id),
        );
        self.push(Edit::Clips {
            remove: clips,
            insert: Vec::new(),
        });
        let item = self.project.patterns[index].clone();
        self.push(Edit::Pattern(ListEdit::Remove { index, item }));
        Ok("Delete pattern")
    }

    fn duplicate_pattern(&mut self, id: PatternId) -> Result<Label, CommandError> {
        let index = self.pattern_index(id)?;
        let original = &self.project.patterns[index];
        let name = copy_name(
            &original.name,
            self.project.patterns.iter().map(|p| p.name.as_str()),
        );
        let mut copy = Pattern {
            name,
            ..original.clone()
        };
        copy.id = PatternId(self.allocate()?);
        for lane in &mut copy.lanes {
            // Ids go out in lane order, so the copies sort the same way.
            for note in &mut lane.notes {
                note.id = NoteId(self.allocate()?);
            }
        }
        self.created.push(copy.id.0);
        self.push(Edit::Pattern(ListEdit::Insert {
            index: index + 1,
            item: copy,
        }));
        Ok("Duplicate pattern")
    }

    fn move_pattern(&mut self, id: PatternId, index: u32) -> Result<Label, CommandError> {
        let from = self.pattern_index(id)?;
        let to = (index as usize).min(self.project.patterns.len() - 1);
        self.push(Edit::MovePattern(Move { from, to }));
        Ok("Move pattern")
    }

    fn update_pattern(
        &mut self,
        id: PatternId,
        patch: PatternPatch,
    ) -> Result<Label, CommandError> {
        let label = single_label(
            &[
                (patch.name.is_some(), "Rename pattern"),
                (patch.color.is_some(), "Change pattern color"),
                (patch.length_steps.is_some(), "Change pattern length"),
            ],
            "Change pattern",
        );
        let old = PatternInfo::of(&self.project.patterns[self.pattern_index(id)?]);
        let mut new = old.clone();
        if let Some(name) = patch.name {
            new.name = name;
        }
        if let Some(color) = patch.color {
            new.color = checked_color(color)?;
        }
        if let Some(length_steps) = patch.length_steps {
            new.length_steps = length_steps.clamp(1, MAX_PATTERN_STEPS);
        }
        self.push(Edit::PatternInfo {
            id,
            change: Change { old, new },
        });
        Ok(label)
    }

    fn toggle_step(
        &mut self,
        pattern: PatternId,
        channel: ChannelId,
        step: u32,
    ) -> Result<Label, CommandError> {
        let label = "Toggle step";
        let lane = self.lane(pattern, channel)?;
        if step >= MAX_PATTERN_STEPS {
            return Err(CommandError::invalid(format!(
                "step {} is past the end of the longest pattern, which has {MAX_PATTERN_STEPS} steps",
                u64::from(step) + 1
            )));
        }
        let start = step * TICKS_PER_STEP;
        let on_step = notes_where(lane, |note| note.start == start);

        if !on_step.is_empty() {
            self.push(Edit::Notes {
                pattern,
                channel,
                remove: on_step,
                insert: Vec::new(),
            });
            return Ok(label);
        }
        let id = NoteId(self.allocate()?);
        self.push(Edit::Notes {
            pattern,
            channel,
            remove: Vec::new(),
            insert: vec![Note {
                id,
                start,
                length: TICKS_PER_STEP,
                key: DEFAULT_KEY,
                velocity: DEFAULT_VELOCITY,
                pan: 0.0,
            }],
        });
        self.created.push(id.0);
        Ok(label)
    }

    fn add_notes(
        &mut self,
        pattern: PatternId,
        channel: ChannelId,
        notes: &[NoteInit],
    ) -> Result<Label, CommandError> {
        self.lane(pattern, channel)?;
        let mut insert = Vec::with_capacity(notes.len());
        for init in notes {
            // The id is filled in below, once every note is known to be valid.
            insert.push(placed_note(Note {
                id: NoteId(0),
                start: init.start,
                length: init.length,
                key: init.key,
                velocity: init.velocity.unwrap_or(DEFAULT_VELOCITY),
                pan: init.pan.unwrap_or(0.0),
            })?);
        }
        for note in &mut insert {
            note.id = NoteId(self.allocate()?);
            self.created.push(note.id.0);
        }
        self.push(Edit::Notes {
            pattern,
            channel,
            remove: Vec::new(),
            insert,
        });
        Ok(plural(notes.len(), "Add note", "Add notes"))
    }

    fn remove_notes(
        &mut self,
        pattern: PatternId,
        channel: ChannelId,
        notes: &[NoteId],
    ) -> Result<Label, CommandError> {
        let lane = self.lane(pattern, channel)?;
        let wanted: HashSet<NoteId> = notes.iter().copied().collect();
        let remove = notes_where(lane, |note| wanted.contains(&note.id));
        if remove.len() < wanted.len() {
            let found = remove.iter().map(|note| note.id);
            return Err(first_missing("note", notes.iter().copied(), found));
        }
        self.push(Edit::Notes {
            pattern,
            channel,
            remove,
            insert: Vec::new(),
        });
        Ok(plural(notes.len(), "Delete note", "Delete notes"))
    }

    fn update_notes(
        &mut self,
        pattern: PatternId,
        channel: ChannelId,
        updates: &[NoteUpdate],
    ) -> Result<Label, CommandError> {
        let lane = self.lane(pattern, channel)?;
        let wanted: HashSet<NoteId> = updates.iter().map(|update| update.id).collect();
        let remove = notes_where(lane, |note| wanted.contains(&note.id));
        if remove.len() < wanted.len() {
            let wanted = updates.iter().map(|update| update.id);
            let found = remove.iter().map(|note| note.id);
            return Err(first_missing("note", wanted, found));
        }

        // Two updates may name the same note; the later one builds on the
        // earlier one.
        let mut changed: HashMap<NoteId, Note> = remove.iter().map(|n| (n.id, *n)).collect();
        let mut moves = false;
        for update in updates {
            if let Some(note) = changed.get_mut(&update.id) {
                let patch = &update.patch;
                let new = Note {
                    id: note.id,
                    start: patch.start.unwrap_or(note.start),
                    length: patch.length.unwrap_or(note.length),
                    key: patch.key.unwrap_or(note.key),
                    velocity: patch.velocity.unwrap_or(note.velocity),
                    pan: patch.pan.unwrap_or(note.pan),
                };
                // A start that moves while the end stays where it was is
                // the note being resized from its front.
                let end = |note: &Note| u64::from(note.start) + u64::from(note.length);
                let resized = patch.length.is_some() && end(&new) == end(note);
                moves |= patch.key.is_some() || (patch.start.is_some() && !resized);
                // A note that is already past the end of the longest
                // pattern, as an older file may hold, can still be changed
                // where it is. It cannot be moved further out.
                *note = if new.start == note.start {
                    checked_note(new)?
                } else {
                    placed_note(new)?
                };
            }
        }
        let label = if moves {
            plural(updates.len(), "Move note", "Move notes")
        } else {
            let has = |field: fn(&NotePatch) -> bool| updates.iter().any(|u| field(&u.patch));
            single_label(
                &[
                    (
                        has(|p| p.length.is_some()),
                        plural(updates.len(), "Resize note", "Resize notes"),
                    ),
                    (has(|p| p.velocity.is_some()), "Change note velocity"),
                    (has(|p| p.pan.is_some()), "Change note pan"),
                ],
                plural(updates.len(), "Change note", "Change notes"),
            )
        };
        let (remove, insert): (Vec<Note>, Vec<Note>) = remove
            .into_iter()
            .filter_map(|old| {
                let new = changed.get(&old.id).copied()?;
                (new != old).then_some((old, new))
            })
            .unzip();
        self.push(Edit::Notes {
            pattern,
            channel,
            remove,
            insert,
        });
        Ok(label)
    }

    fn transform_notes(
        &mut self,
        pattern: PatternId,
        channel: ChannelId,
        expected: &[Note],
        transform: crate::NoteTransform,
    ) -> Result<Label, CommandError> {
        let selected = crate::piano_tools::selection(expected)?;
        let lane = self.lane(pattern, channel)?;
        let wanted: HashSet<_> = selected.iter().map(|n| n.id).collect();
        let current: HashMap<_, _> = lane
            .into_iter()
            .flat_map(|l| &l.notes)
            .filter(|n| wanted.contains(&n.id))
            .map(|n| (n.id, n))
            .collect();
        for note in &selected {
            if current.get(&note.id).copied() != Some(note) {
                return Err(CommandError::invalid(
                    "the selected notes changed; reopen the tool",
                ));
            }
        }
        let label = transform.label();
        let mut insert = crate::piano_tools::transform_selected_notes(&selected, transform)?;
        // Identity transformations preserve history, dirty state and redo.
        if selected == insert {
            return Ok(label);
        }
        for note in &mut insert {
            if note.id.0 == 0 {
                note.id = NoteId(self.allocate()?);
                self.created.push(note.id.0);
            }
        }
        let end = insert.iter().map(|n| n.start + n.length).max().unwrap_or(0);
        self.push(Edit::Notes {
            pattern,
            channel,
            remove: selected,
            insert,
        });
        let index = self.pattern_index(pattern)?;
        let old = PatternInfo::of(&self.project.patterns[index]);
        let steps = end.div_ceil(TICKS_PER_STEP);
        if steps > old.length_steps {
            self.push(Edit::PatternInfo {
                id: pattern,
                change: Change {
                    new: PatternInfo {
                        length_steps: steps,
                        ..old.clone()
                    },
                    old,
                },
            });
        }
        Ok(label)
    }

    fn clear_lane(
        &mut self,
        pattern: PatternId,
        channel: ChannelId,
    ) -> Result<Label, CommandError> {
        let remove = self
            .lane(pattern, channel)?
            .map(|lane| lane.notes.clone())
            .unwrap_or_default();
        self.push(Edit::Notes {
            pattern,
            channel,
            remove,
            insert: Vec::new(),
        });
        Ok("Clear notes")
    }

    fn add_mixer_track(&mut self, name: Option<String>) -> Result<Label, CommandError> {
        let tracks = &self.project.mixer.tracks;
        let count = tracks.len();
        if count >= MAX_MIXER_TRACKS {
            return Err(CommandError::invalid(format!(
                "the mixer is full: it holds {MAX_MIXER_TRACKS} tracks at most"
            )));
        }
        let name = name.unwrap_or_else(|| {
            numbered_name("Insert", count, tracks.iter().map(|t| t.name.as_str()))
        });
        // The master does not take a palette color, so inserts start the cycle.
        let color = palette_color(count.saturating_sub(1));
        let id = TrackId(self.allocate()?);
        self.push(Edit::MixerTrack(ListEdit::Insert {
            index: count,
            item: new_mixer_track(id, name, color),
        }));
        self.created.push(id.0);
        Ok("Add mixer track")
    }

    fn remove_mixer_track(&mut self, id: TrackId) -> Result<Label, CommandError> {
        let index = self.mixer_track_index(id)?;
        if id == TrackId::MASTER {
            return Err(CommandError::invalid("the master track cannot be deleted"));
        }
        self.remove_automations_of(|target| match *target {
            AutomationTarget::TrackVolume { track }
            | AutomationTarget::TrackPan { track }
            | AutomationTarget::EffectParam { track, .. }
            | AutomationTarget::EffectMix { track, .. } => track == id,
            AutomationTarget::SendGain { track, target } => track == id || target == id,
            _ => false,
        });
        let channels: Vec<Channel> = self
            .project
            .channels
            .iter()
            .filter(|channel| channel.mixer_track == id)
            .cloned()
            .collect();
        for old in channels {
            let new = Channel {
                mixer_track: TrackId::MASTER,
                ..old.clone()
            };
            self.push(Edit::SetChannel(Box::new(Change { old, new })));
        }
        let (remove, insert): (Vec<Clip>, Vec<Clip>) = self
            .project
            .playlist
            .clips
            .iter()
            .filter_map(|old| {
                let mut new = old.clone();
                match &mut new.content {
                    ClipContent::Audio { mixer_track, .. } if *mixer_track == id => {
                        *mixer_track = TrackId::MASTER;
                    }
                    _ => return None,
                }
                Some((old.clone(), new))
            })
            .unzip();
        self.push(Edit::Clips { remove, insert });
        // The master feeds no other track, so routing a track's output there
        // cannot close a loop.
        let tracks: Vec<MixerTrack> = self
            .project
            .mixer
            .tracks
            .iter()
            .filter(|track| track.id != id)
            .cloned()
            .collect();
        for old in tracks {
            let mut new = old.clone();
            if new.output == Some(id) {
                new.output = Some(TrackId::MASTER);
            }
            new.sends.retain(|send| send.target != id);
            self.push(Edit::SetMixerTrack(Box::new(Change { old, new })));
        }
        let item = self.project.mixer.tracks[index].clone();
        self.push(Edit::MixerTrack(ListEdit::Remove { index, item }));
        Ok("Delete mixer track")
    }

    fn update_mixer_track(
        &mut self,
        id: TrackId,
        patch: MixerTrackPatch,
    ) -> Result<Label, CommandError> {
        let label = single_label(
            &[
                (patch.name.is_some(), "Rename mixer track"),
                (patch.color.is_some(), "Change mixer track color"),
                (patch.volume.is_some(), "Change mixer track volume"),
                (patch.pan.is_some(), "Change mixer track pan"),
                (patch.muted == Some(true), "Mute mixer track"),
                (patch.muted == Some(false), "Unmute mixer track"),
                (patch.solo == Some(true), "Solo mixer track"),
                (patch.solo == Some(false), "Unsolo mixer track"),
            ],
            "Change mixer track",
        );
        let old = self.project.mixer.tracks[self.mixer_track_index(id)?].clone();
        let mut new = old.clone();
        if let Some(name) = patch.name {
            new.name = name;
        }
        if let Some(color) = patch.color {
            new.color = checked_color(color)?;
        }
        if let Some(volume) = patch.volume {
            new.volume = clamped("the volume", volume, 0.0, MAX_GAIN)?;
        }
        if let Some(pan) = patch.pan {
            new.pan = clamped("the pan", pan, -1.0, 1.0)?;
        }
        if let Some(muted) = patch.muted {
            new.muted = muted;
        }
        if let Some(solo) = patch.solo {
            new.solo = solo;
        }
        self.push(Edit::SetMixerTrack(Box::new(Change { old, new })));
        Ok(label)
    }

    fn set_track_output(
        &mut self,
        id: TrackId,
        output: Option<TrackId>,
    ) -> Result<Label, CommandError> {
        let old = self.project.mixer.tracks[self.mixer_track_index(id)?].clone();
        if id == TrackId::MASTER {
            return Err(CommandError::invalid(
                "the master track's output cannot be changed",
            ));
        }
        if let Some(output) = output {
            self.mixer_track_index(output)?;
            if reaches(&self.project.mixer, output, id) {
                return Err(CommandError::invalid(
                    "that output would make the mixer routing loop back on itself",
                ));
            }
        }
        let new = MixerTrack {
            output,
            ..old.clone()
        };
        self.push(Edit::SetMixerTrack(Box::new(Change { old, new })));
        Ok("Route mixer track")
    }

    fn set_send(
        &mut self,
        from: TrackId,
        to: TrackId,
        gain: Option<f32>,
    ) -> Result<Label, CommandError> {
        let old = self.project.mixer.tracks[self.mixer_track_index(from)?].clone();
        self.mixer_track_index(to)?;
        let mut new = old.clone();
        let existing = new.sends.iter().position(|send| send.target == to);
        let label = match (gain, existing) {
            (None, Some(index)) => {
                new.sends.remove(index);
                self.remove_automations_of(|target| {
                    *target
                        == AutomationTarget::SendGain {
                            track: from,
                            target: to,
                        }
                });
                "Remove send"
            }
            (None, None) => "Remove send",
            (Some(gain), existing) => {
                let gain = clamped("the send level", gain, 0.0, MAX_GAIN)?;
                if from == TrackId::MASTER {
                    return Err(CommandError::invalid(
                        "the master track cannot send to other tracks",
                    ));
                }
                match existing {
                    Some(index) => {
                        new.sends[index].gain = gain;
                        "Change send level"
                    }
                    None => {
                        if reaches(&self.project.mixer, to, from) {
                            return Err(CommandError::invalid(
                                "that send would make the mixer routing loop back on itself",
                            ));
                        }
                        new.sends.push(Send { target: to, gain });
                        "Add send"
                    }
                }
            }
        };
        self.push(Edit::SetMixerTrack(Box::new(Change { old, new })));
        Ok(label)
    }

    fn add_effect(
        &mut self,
        track: TrackId,
        kind: EffectKind,
        index: Option<u32>,
    ) -> Result<Label, CommandError> {
        let track = self.mixer_track_index(track)?;
        let count = self.room_for_effect(track)?;
        let index = index.map_or(count, |index| (index as usize).min(count));
        let id = EffectId(self.allocate()?);
        let slot = EffectSlot {
            id,
            enabled: true,
            mix: 1.0,
            params: kind.default_params(),
        };
        self.change_track(track, |track| track.effects.insert(index, slot));
        self.created.push(id.0);
        Ok("Add effect")
    }

    fn remove_effect(&mut self, track: TrackId, effect: EffectId) -> Result<Label, CommandError> {
        let (track, slot) = self.effect_index(track, effect)?;
        self.remove_automations_of(|target| targets_effect(target, effect));
        self.change_track(track, |track| {
            track.effects.remove(slot);
        });
        Ok("Delete effect")
    }

    fn move_effect(
        &mut self,
        track: TrackId,
        effect: EffectId,
        to_track: Option<TrackId>,
        index: u32,
    ) -> Result<Label, CommandError> {
        let (from_track, from) = self.effect_index(track, effect)?;
        let to_track = match to_track {
            Some(id) => self.mixer_track_index(id)?,
            None => from_track,
        };
        let index = index as usize;
        if to_track == from_track {
            self.change_track(from_track, |track| {
                let slot = track.effects.remove(from);
                let to = index.min(track.effects.len());
                track.effects.insert(to, slot);
            });
        } else {
            let count = self.room_for_effect(to_track)?;
            let slot = self.project.mixer.tracks[from_track].effects[from].clone();
            self.change_track(from_track, |track| {
                track.effects.remove(from);
            });
            self.change_track(to_track, |track| {
                track.effects.insert(index.min(count), slot);
            });
            // The automations of the effect name the track it is on.
            let home = self.project.mixer.tracks[to_track].id;
            let moved: Vec<Automation> = self
                .project
                .automations
                .iter()
                .filter(|automation| targets_effect(&automation.target, effect))
                .cloned()
                .collect();
            for old in moved {
                let mut new = old.clone();
                match &mut new.target {
                    AutomationTarget::EffectParam { track, .. }
                    | AutomationTarget::EffectMix { track, .. } => *track = home,
                    _ => {}
                }
                self.push(Edit::SetAutomation(Box::new(Change { old, new })));
            }
        }
        Ok("Move effect")
    }

    fn update_effect(
        &mut self,
        track: TrackId,
        effect: EffectId,
        patch: EffectSlotPatch,
    ) -> Result<Label, CommandError> {
        let label = single_label(
            &[
                (patch.enabled == Some(true), "Switch effect on"),
                (patch.enabled == Some(false), "Switch effect off"),
                (patch.mix.is_some(), "Change effect mix"),
            ],
            "Change effect",
        );
        let (track, slot) = self.effect_index(track, effect)?;
        let mix = match patch.mix {
            Some(mix) => Some(clamped("the mix", mix, 0.0, 1.0)?),
            None => None,
        };
        self.change_track(track, |track| {
            let slot = &mut track.effects[slot];
            slot.enabled = patch.enabled.unwrap_or(slot.enabled);
            slot.mix = mix.unwrap_or(slot.mix);
        });
        Ok(label)
    }

    fn set_effect_param(
        &mut self,
        track: TrackId,
        effect: EffectId,
        param: u32,
        value: f32,
    ) -> Result<String, CommandError> {
        let target = crate::PluginTarget::Effect { effect };
        if let Some(plugin) = self.project.plugin(target) {
            self.effect_index(track, effect)?;
            let parameter = plugin
                .parameters
                .get(param as usize)
                .ok_or_else(|| CommandError::invalid("the plugin parameter does not exist"))?;
            let id = parameter.id;
            let label = format!("Change {}", parameter.name);
            self.plugin_param(target, id, value)?;
            return Ok(label);
        }
        let (track, slot) = self.effect_index(track, effect)?;
        let mut params = self.project.mixer.tracks[track].effects[slot].params;
        let kind = params.kind();
        if !params.set(param as usize, checked_setting(value)?) {
            return Err(no_setting(kind.name(), param));
        }
        self.change_track(track, |track| track.effects[slot].params = params);
        Ok(setting_label(kind.descriptors(), param))
    }

    fn set_effect_params(
        &mut self,
        track: TrackId,
        effect: EffectId,
        params: EffectParams,
    ) -> Result<Label, CommandError> {
        if self
            .project
            .plugin(crate::PluginTarget::Effect { effect })
            .is_some()
        {
            return Err(CommandError::invalid("use the hosted plugin's parameters"));
        }
        let (track, slot) = self.effect_index(track, effect)?;
        let kind = self.project.mixer.tracks[track].effects[slot].kind();
        if params.kind() != kind {
            return Err(other_kind(params.kind().name(), kind.name()));
        }
        let params = checked_effect(params)?;
        self.change_track(track, |track| track.effects[slot].params = params);
        Ok("Change effect settings")
    }

    fn duplicate_effect(
        &mut self,
        track: TrackId,
        effect: EffectId,
    ) -> Result<Label, CommandError> {
        let (track, slot) = self.effect_index(track, effect)?;
        self.room_for_effect(track)?;
        let mut copy = self.project.mixer.tracks[track].effects[slot].clone();
        copy.id = EffectId(self.allocate()?);
        let id = copy.id;
        self.change_track(track, |track| track.effects.insert(slot + 1, copy));
        self.created.push(id.0);
        if let Some(mut plugin) = self
            .project
            .plugin(crate::PluginTarget::Effect { effect })
            .cloned()
        {
            plugin.target = crate::PluginTarget::Effect { effect: id };
            self.bind_plugin(plugin);
        }
        Ok("Duplicate effect")
    }

    fn replace_effect(
        &mut self,
        track: TrackId,
        effect: EffectId,
        kind: EffectKind,
    ) -> Result<Label, CommandError> {
        let (track, slot) = self.effect_index(track, effect)?;
        let id = EffectId(self.allocate()?);
        let new = EffectSlot {
            id,
            enabled: true,
            mix: 1.0,
            params: kind.default_params(),
        };
        self.remove_automations_of(|target| targets_effect(target, effect));
        self.change_track(track, |track| track.effects[slot] = new);
        self.created.push(id.0);
        Ok("Replace effect")
    }

    fn add_playlist_track(
        &mut self,
        name: Option<String>,
        index: Option<u32>,
    ) -> Result<Label, CommandError> {
        let tracks = &self.project.playlist.tracks;
        let count = tracks.len();
        let name = name.unwrap_or_else(|| {
            numbered_name("Track", count + 1, tracks.iter().map(|t| t.name.as_str()))
        });
        let index = index.map_or(count, |index| (index as usize).min(count));
        let id = PlaylistTrackId(self.allocate()?);
        self.push(Edit::PlaylistTrack(ListEdit::Insert {
            index,
            item: PlaylistTrack {
                id,
                name,
                muted: false,
            },
        }));
        self.created.push(id.0);
        Ok("Add playlist track")
    }

    fn remove_playlist_track(&mut self, id: PlaylistTrackId) -> Result<Label, CommandError> {
        let index = self.playlist_track_index(id)?;
        let clips = self.clips_where(|clip| clip.track == id);
        self.push(Edit::Clips {
            remove: clips,
            insert: Vec::new(),
        });
        let item = self.project.playlist.tracks[index].clone();
        self.push(Edit::PlaylistTrack(ListEdit::Remove { index, item }));
        Ok("Delete playlist track")
    }

    fn update_playlist_track(
        &mut self,
        id: PlaylistTrackId,
        patch: PlaylistTrackPatch,
    ) -> Result<Label, CommandError> {
        let label = single_label(
            &[
                (patch.name.is_some(), "Rename playlist track"),
                (patch.muted == Some(true), "Mute playlist track"),
                (patch.muted == Some(false), "Unmute playlist track"),
            ],
            "Change playlist track",
        );
        let old = self.project.playlist.tracks[self.playlist_track_index(id)?].clone();
        let mut new = old.clone();
        if let Some(name) = patch.name {
            new.name = name;
        }
        if let Some(muted) = patch.muted {
            new.muted = muted;
        }
        self.push(Edit::SetPlaylistTrack(Change { old, new }));
        Ok(label)
    }

    fn move_playlist_track(
        &mut self,
        id: PlaylistTrackId,
        index: u32,
    ) -> Result<Label, CommandError> {
        let from = self.playlist_track_index(id)?;
        let to = (index as usize).min(self.project.playlist.tracks.len() - 1);
        self.push(Edit::MovePlaylistTrack(Move { from, to }));
        Ok("Move playlist track")
    }

    fn add_clips(&mut self, clips: &[ClipInit]) -> Result<Label, CommandError> {
        let mut insert = Vec::with_capacity(clips.len());
        for init in clips {
            self.playlist_track_index(init.track)?;
            let (content, natural) = self.checked_content(&init.content)?;
            let length = init.length.or(natural).ok_or_else(|| {
                CommandError::invalid(
                    "an audio clip has to be given a length: the project does not know how long its audio lasts",
                )
            })?;
            // The id is filled in below, once every clip is known to be valid.
            insert.push(checked_clip(Clip {
                id: ClipId(0),
                track: init.track,
                start: init.start,
                length,
                offset: init.offset.unwrap_or(0),
                muted: init.muted.unwrap_or(false),
                content,
            })?);
        }
        for clip in &mut insert {
            clip.id = ClipId(self.allocate()?);
            self.created.push(clip.id.0);
        }
        self.push(Edit::Clips {
            remove: Vec::new(),
            insert,
        });
        Ok(plural(clips.len(), "Add clip", "Add clips"))
    }

    fn remove_clips(&mut self, clips: &[ClipId]) -> Result<Label, CommandError> {
        let wanted: HashSet<ClipId> = clips.iter().copied().collect();
        let remove = self.clips_where(|clip| wanted.contains(&clip.id));
        if remove.len() < wanted.len() {
            let found = remove.iter().map(|clip| clip.id);
            return Err(first_missing("clip", clips.iter().copied(), found));
        }
        self.push(Edit::Clips {
            remove,
            insert: Vec::new(),
        });
        Ok(plural(clips.len(), "Delete clip", "Delete clips"))
    }

    fn update_clips(&mut self, updates: &[ClipUpdate]) -> Result<Label, CommandError> {
        let has = |field: fn(&ClipPatch) -> bool| updates.iter().any(|u| field(&u.patch));
        let count = updates.len();
        let label = if has(|p| p.track.is_some() || p.start.is_some()) {
            plural(count, "Move clip", "Move clips")
        } else if has(|p| p.length.is_some() || p.offset.is_some()) {
            plural(count, "Resize clip", "Resize clips")
        } else if has(|p| p.muted == Some(true)) {
            plural(count, "Mute clip", "Mute clips")
        } else if has(|p| p.muted == Some(false)) {
            plural(count, "Unmute clip", "Unmute clips")
        } else {
            plural(count, "Change clip", "Change clips")
        };

        let wanted: HashSet<ClipId> = updates.iter().map(|update| update.id).collect();
        let remove = self.clips_where(|clip| wanted.contains(&clip.id));
        if remove.len() < wanted.len() {
            let wanted = updates.iter().map(|update| update.id);
            let found = remove.iter().map(|clip| clip.id);
            return Err(first_missing("clip", wanted, found));
        }

        // Two updates may name the same clip; the later one builds on the
        // earlier one.
        let mut changed: HashMap<ClipId, Clip> = remove.iter().map(|c| (c.id, c.clone())).collect();
        for update in updates {
            if let Some(clip) = changed.get_mut(&update.id) {
                let patch = &update.patch;
                if let Some(track) = patch.track {
                    self.playlist_track_index(track)?;
                    clip.track = track;
                }
                clip.start = patch.start.unwrap_or(clip.start);
                clip.length = patch.length.unwrap_or(clip.length);
                clip.offset = patch.offset.unwrap_or(clip.offset);
                clip.muted = patch.muted.unwrap_or(clip.muted);
                *clip = checked_clip(clip.clone())?;
            }
        }
        let (remove, insert): (Vec<Clip>, Vec<Clip>) = remove
            .into_iter()
            .filter_map(|old| {
                let new = changed.remove(&old.id)?;
                (new != old).then_some((old, new))
            })
            .unzip();
        self.push(Edit::Clips { remove, insert });
        Ok(label)
    }

    fn update_audio_clips(&mut self, updates: &[AudioClipUpdate]) -> Result<Label, CommandError> {
        let has = |field: fn(&AudioClipPatch) -> bool| updates.iter().any(|u| field(&u.patch));
        let label = single_label(
            &[
                (has(|p| p.mixer_track.is_some()), "Route clip"),
                (has(|p| p.gain.is_some()), "Change clip gain"),
                (has(|p| p.pan.is_some()), "Change clip pan"),
                (
                    has(|p| p.fade_in.is_some() || p.fade_out.is_some()),
                    "Change clip fade",
                ),
                (has(|p| p.reverse.is_some()), "Reverse clip"),
                (has(|p| p.pitch.is_some()), "Change clip pitch"),
                (has(|p| p.stretch.is_some()), "Change clip stretch"),
            ],
            plural(updates.len(), "Change audio clip", "Change audio clips"),
        );

        let wanted: HashSet<ClipId> = updates.iter().map(|update| update.id).collect();
        let remove = self.clips_where(|clip| wanted.contains(&clip.id));
        if remove.len() < wanted.len() {
            let wanted = updates.iter().map(|update| update.id);
            let found = remove.iter().map(|clip| clip.id);
            return Err(first_missing("clip", wanted, found));
        }

        // Two updates may name the same clip; the later one builds on the
        // earlier one.
        let mut changed: HashMap<ClipId, Clip> = remove.iter().map(|c| (c.id, c.clone())).collect();
        for update in updates {
            let Some(clip) = changed.get_mut(&update.id) else {
                continue;
            };
            let ClipContent::Audio {
                sample,
                mixer_track,
                gain,
                pan,
                fade_in,
                fade_out,
                reverse,
                pitch,
                stretch,
            } = clip.content
            else {
                return Err(CommandError::invalid(format!(
                    "clip {} is not an audio clip",
                    clip.id.0
                )));
            };
            let patch = &update.patch;
            let patched = ClipContent::Audio {
                sample,
                mixer_track: patch.mixer_track.unwrap_or(mixer_track),
                gain: patch.gain.unwrap_or(gain),
                pan: patch.pan.unwrap_or(pan),
                fade_in: patch.fade_in.unwrap_or(fade_in),
                fade_out: patch.fade_out.unwrap_or(fade_out),
                reverse: patch.reverse.unwrap_or(reverse),
                pitch: patch.pitch.unwrap_or(pitch),
                stretch: patch.stretch.unwrap_or(stretch),
            };
            clip.content = self.checked_content(&patched)?.0;
        }
        let (remove, insert): (Vec<Clip>, Vec<Clip>) = remove
            .into_iter()
            .filter_map(|old| {
                let new = changed.remove(&old.id)?;
                (new != old).then_some((old, new))
            })
            .unzip();
        self.push(Edit::Clips { remove, insert });
        Ok(label)
    }

    /// What a clip is to play, checked against the project and with every
    /// value inside its range, and the length such a clip has when none is
    /// given, if the project can tell.
    fn checked_content(
        &self,
        content: &ClipContent,
    ) -> Result<(ClipContent, Option<u32>), CommandError> {
        match *content {
            ClipContent::Pattern { pattern } => {
                let pattern = &self.project.patterns[self.pattern_index(pattern)?];
                Ok((content.clone(), Some(pattern.length_ticks())))
            }
            ClipContent::Audio {
                sample,
                mixer_track,
                gain,
                pan,
                fade_in,
                fade_out,
                reverse,
                pitch,
                stretch,
            } => {
                self.sample_index(sample)?;
                self.mixer_track_index(mixer_track)?;
                let range = if matches!(stretch, crate::ClipStretch::Spectral { .. }) {
                    24.0
                } else {
                    MAX_TUNE_SEMITONES
                };
                if let crate::ClipStretch::Spectral { ratio, .. } = stretch
                    && (!ratio.is_finite() || !(0.25..=4.0).contains(&ratio))
                {
                    return Err(CommandError::invalid(
                        "Stretch ratio must be between 0.25 and 4",
                    ));
                }
                let checked = ClipContent::Audio {
                    sample,
                    mixer_track,
                    gain: clamped("the clip gain", gain, 0.0, MAX_GAIN)?,
                    pan: clamped("the pan", pan, -1.0, 1.0)?,
                    fade_in: fade_in.min(MAX_SONG_TICKS),
                    fade_out: fade_out.min(MAX_SONG_TICKS),
                    reverse,
                    pitch: clamped("the pitch", pitch, -range, range)?,

                    stretch,
                };
                Ok((checked, None))
            }
            ClipContent::Automation { automation } => {
                let automation = &self.project.automations[self.automation_index(automation)?];
                // The whole curve, and at least a bar to take hold of.
                let last = automation.points.last().map_or(0, |point| point.tick);
                let bar = self.project.settings.time_signature.ticks_per_bar();
                Ok((content.clone(), Some(last.max(bar))))
            }
        }
    }

    fn add_automation(
        &mut self,
        name: Option<String>,
        target: AutomationTarget,
        points: Option<Vec<AutomationPoint>>,
    ) -> Result<Label, CommandError> {
        let (range, stored) = self.automation_target(&target)?;
        let points = match points {
            Some(points) => checked_points(points)?,
            None => vec![AutomationPoint {
                tick: 0,
                value: range.normalized(stored),
                curve: 0.0,
                hold: false,
            }],
        };
        let name = name.unwrap_or_else(|| self.automation_name(&target));
        let count = self.project.automations.len();
        let id = AutomationId(self.allocate()?);
        self.push(Edit::Automation(ListEdit::Insert {
            index: count,
            item: Automation {
                id,
                name,
                color: palette_color(count),
                target,
                points,
            },
        }));
        self.created.push(id.0);
        Ok("Add automation")
    }

    fn remove_automation(&mut self, id: AutomationId) -> Result<Label, CommandError> {
        self.automation_index(id)?;
        self.take_out_automations(&[id]);
        Ok("Delete automation")
    }

    fn update_automation(
        &mut self,
        id: AutomationId,
        patch: AutomationPatch,
    ) -> Result<Label, CommandError> {
        let label = single_label(
            &[
                (patch.name.is_some(), "Rename automation"),
                (patch.color.is_some(), "Change automation color"),
            ],
            "Change automation",
        );
        let old = self.project.automations[self.automation_index(id)?].clone();
        let mut new = old.clone();
        if let Some(name) = patch.name {
            new.name = name;
        }
        if let Some(color) = patch.color {
            new.color = checked_color(color)?;
        }
        self.push(Edit::SetAutomation(Box::new(Change { old, new })));
        Ok(label)
    }

    fn set_automation_points(
        &mut self,
        id: AutomationId,
        points: Vec<AutomationPoint>,
    ) -> Result<Label, CommandError> {
        let old = self.project.automations[self.automation_index(id)?].clone();
        let new = Automation {
            points: checked_points(points)?,
            ..old.clone()
        };
        self.push(Edit::SetAutomation(Box::new(Change { old, new })));
        Ok("Change automation curve")
    }

    fn duplicate_automation(&mut self, id: AutomationId) -> Result<Label, CommandError> {
        let index = self.automation_index(id)?;
        let original = &self.project.automations[index];
        let name = copy_name(
            &original.name,
            self.project.automations.iter().map(|a| a.name.as_str()),
        );
        let mut copy = Automation {
            name,
            ..original.clone()
        };
        copy.id = AutomationId(self.allocate()?);
        self.created.push(copy.id.0);
        self.push(Edit::Automation(ListEdit::Insert {
            index: index + 1,
            item: copy,
        }));
        Ok("Duplicate automation")
    }

    /// Removes every automation whose target `gone` picks out, with its
    /// clips: what those automations move is about to leave the project.
    fn remove_automations_of(&mut self, gone: impl Fn(&AutomationTarget) -> bool) {
        let automations = self.project.automations.iter();
        let ids: Vec<AutomationId> = automations
            .filter(|automation| gone(&automation.target))
            .map(|automation| automation.id)
            .collect();
        self.take_out_automations(&ids);
    }

    /// Removes automations and the clips that show them.
    fn take_out_automations(&mut self, ids: &[AutomationId]) {
        if ids.is_empty() {
            return;
        }
        let clips = self.clips_where(|clip| {
            let shown = clip.content.automation();
            shown.is_some_and(|automation| ids.contains(&automation))
        });
        self.push(Edit::Clips {
            remove: clips,
            insert: Vec::new(),
        });
        for id in ids {
            let automations = &self.project.automations;
            if let Some(index) = automations.iter().position(|a| a.id == *id) {
                let item = automations[index].clone();
                self.push(Edit::Automation(ListEdit::Remove { index, item }));
            }
        }
    }

    /// The range of what `target` moves and the value it has now. Fails,
    /// saying what is missing, when the project has no such thing.
    fn automation_target(
        &self,
        target: &AutomationTarget,
    ) -> Result<(AutomationRange, f32), CommandError> {
        match *target {
            AutomationTarget::ChannelVolume { channel }
            | AutomationTarget::ChannelPan { channel } => {
                self.channel_index(channel)?;
            }
            AutomationTarget::TrackVolume { track } | AutomationTarget::TrackPan { track } => {
                self.mixer_track_index(track)?;
            }
            AutomationTarget::SendGain { track, target } => {
                let from = &self.project.mixer.tracks[self.mixer_track_index(track)?];
                let to = &self.project.mixer.tracks[self.mixer_track_index(target)?];
                if !from.sends.iter().any(|send| send.target == target) {
                    return Err(CommandError::invalid(format!(
                        "the mixer track \"{}\" has no send to \"{}\"",
                        from.name, to.name
                    )));
                }
            }
            AutomationTarget::EffectParam {
                track,
                effect,
                param,
            } => {
                let (track, slot) = self.effect_index(track, effect)?;
                let kind = self.project.mixer.tracks[track].effects[slot].kind();
                if kind.descriptors().get(param as usize).is_none() {
                    return Err(no_setting(kind.name(), param));
                }
            }
            AutomationTarget::EffectMix { track, effect } => {
                self.effect_index(track, effect)?;
            }
            AutomationTarget::InstrumentParam { channel, param } => {
                let kind = self.instrument(self.channel_index(channel)?)?.kind();
                if kind.descriptors().get(param as usize).is_none() {
                    return Err(no_setting(kind.name(), param));
                }
            }
            AutomationTarget::Tempo => {}
        }
        let range = self.project.automation_range(target);
        let stored = self.project.automation_stored_value(target);
        range.zip(stored).ok_or_else(|| {
            CommandError::invalid("the project has nothing for the automation to move")
        })
    }

    /// The name an automation of `target` gets when none is given. The
    /// target is known to exist.
    ///
    /// The name says what kind of thing is moved, so that a channel and
    /// the mixer track named after it do not give two automations one
    /// name: "Kick volume" is the channel's and "Kick track volume" the
    /// track's. An effect is named with its track, "Kick Reverb mix", and
    /// counted when the track has several of its kind, "Kick Reverb 2
    /// Decay". Whatever comes out is numbered if an automation has that
    /// name already: "Kick pan", then "Kick pan 2".
    fn automation_name(&self, target: &AutomationTarget) -> String {
        let project = &*self.project;
        let channel = |id| {
            project
                .channel(id)
                .map_or("", |channel| channel.name.as_str())
        };
        let track = |id| {
            project
                .mixer
                .track(id)
                .map_or("", |track| track.name.as_str())
        };
        // "Kick Reverb", or "Kick Reverb 2" for the second reverb in the
        // track's chain.
        let effect = |track_id: TrackId, effect: EffectId| {
            if let Some(plugin) = project.plugin(crate::PluginTarget::Effect { effect }) {
                return format!("{} {}", track(track_id), plugin.name);
            }
            let chain = project.mixer.track(track_id).map(|track| &track.effects);
            let chain = chain.map_or(&[][..], Vec::as_slice);
            let Some(place) = chain.iter().position(|slot| slot.id == effect) else {
                return format!("{} Effect", track(track_id));
            };
            let kind = chain[place].kind();
            let earlier = chain[..place].iter().filter(|slot| slot.kind() == kind);
            match earlier.count() {
                0 => format!("{} {}", track(track_id), kind.name()),
                earlier => format!("{} {} {}", track(track_id), kind.name(), earlier + 1),
            }
        };
        let effect_kind = |track: TrackId, effect| {
            let slot = project
                .mixer
                .track(track)
                .and_then(|track| track.effect(effect));
            slot.map(EffectSlot::kind)
        };
        let setting = |descriptors: &[ParamInfo], param: u32| {
            descriptors
                .get(param as usize)
                .map_or("setting", |info| info.name)
        };
        let name = match *target {
            AutomationTarget::ChannelVolume { channel: id } => format!("{} volume", channel(id)),
            AutomationTarget::ChannelPan { channel: id } => format!("{} pan", channel(id)),
            AutomationTarget::TrackVolume { track: id } => format!("{} track volume", track(id)),
            AutomationTarget::TrackPan { track: id } => format!("{} track pan", track(id)),
            AutomationTarget::SendGain {
                track: from,
                target,
            } => {
                format!("{} to {} send", track(from), track(target))
            }
            AutomationTarget::EffectParam {
                track,
                effect: id,
                param,
            } => {
                if let Some(plugin) = project.plugin(crate::PluginTarget::Effect { effect: id }) {
                    let name = plugin
                        .parameters
                        .get(param as usize)
                        .map_or("setting", |parameter| parameter.name.as_str());
                    return unused_name(
                        format!("{} {name}", effect(track, id)),
                        project
                            .automations
                            .iter()
                            .map(|automation| automation.name.as_str()),
                    );
                }
                let descriptors = effect_kind(track, id).map_or(&[][..], EffectKind::descriptors);
                format!("{} {}", effect(track, id), setting(descriptors, param))
            }
            AutomationTarget::EffectMix { track, effect: id } => {
                format!("{} mix", effect(track, id))
            }
            AutomationTarget::InstrumentParam { channel: id, param } => {
                if let Some(plugin) =
                    project.plugin(crate::PluginTarget::Instrument { channel: id })
                {
                    let name = plugin
                        .parameters
                        .get(param as usize)
                        .map_or("setting", |parameter| parameter.name.as_str());
                    return unused_name(
                        format!("{} {name}", channel(id)),
                        project
                            .automations
                            .iter()
                            .map(|automation| automation.name.as_str()),
                    );
                }
                let source = project.channel(id).map(|channel| &channel.source);
                let name = match source {
                    Some(ChannelSource::Instrument { params }) => {
                        setting(params.kind().descriptors(), param)
                    }
                    _ => "setting",
                };
                format!("{} {name}", channel(id))
            }
            AutomationTarget::Tempo => "Tempo".to_owned(),
        };
        let taken = project.automations.iter().map(|a| a.name.as_str());
        unused_name(name, taken)
    }

    fn automation_index(&self, id: AutomationId) -> Result<usize, CommandError> {
        let found = self.project.automations.iter().position(|a| a.id == id);
        found.ok_or_else(|| CommandError::not_found("automation", id))
    }

    fn sample_index(&self, id: SampleId) -> Result<usize, CommandError> {
        let found = self.project.samples.iter().position(|s| s.id == id);
        found.ok_or_else(|| CommandError::not_found("sample", id))
    }

    fn channel_index(&self, id: ChannelId) -> Result<usize, CommandError> {
        let found = self.project.channels.iter().position(|c| c.id == id);
        found.ok_or_else(|| CommandError::not_found("channel", id))
    }

    fn pattern_index(&self, id: PatternId) -> Result<usize, CommandError> {
        let found = self.project.patterns.iter().position(|p| p.id == id);
        found.ok_or_else(|| CommandError::not_found("pattern", id))
    }

    fn mixer_track_index(&self, id: TrackId) -> Result<usize, CommandError> {
        let found = self.project.mixer.tracks.iter().position(|t| t.id == id);
        found.ok_or_else(|| CommandError::not_found("mixer track", id))
    }

    fn playlist_track_index(&self, id: PlaylistTrackId) -> Result<usize, CommandError> {
        let found = self.project.playlist.tracks.iter().position(|t| t.id == id);
        found.ok_or_else(|| CommandError::not_found("playlist track", id))
    }

    /// The channel's lane in the pattern, or `None` when it has no notes
    /// there. Fails when the pattern or the channel does not exist.
    fn lane(&self, pattern: PatternId, channel: ChannelId) -> Result<Option<&Lane>, CommandError> {
        let pattern = &self.project.patterns[self.pattern_index(pattern)?];
        self.channel_index(channel)?;
        Ok(pattern.lane(channel))
    }

    /// The notes a channel has in each pattern where it has any.
    fn lanes_of(&self, channel: ChannelId) -> Vec<(PatternId, Vec<Note>)> {
        let patterns = self.project.patterns.iter();
        patterns
            .filter_map(|pattern| Some((pattern.id, pattern.lane(channel)?.notes.clone())))
            .collect()
    }

    fn clips_where(&self, keep: impl Fn(&Clip) -> bool) -> Vec<Clip> {
        let clips = self.project.playlist.clips.iter();
        clips.filter(|clip| keep(clip)).cloned().collect()
    }

    /// The index of a mixer track and, in its chain, of one of its effects.
    /// An effect that sits on another track is not found.
    fn effect_index(
        &self,
        track: TrackId,
        effect: EffectId,
    ) -> Result<(usize, usize), CommandError> {
        let track = self.mixer_track_index(track)?;
        let effects = &self.project.mixer.tracks[track].effects;
        let found = effects.iter().position(|slot| slot.id == effect);
        found
            .map(|slot| (track, slot))
            .ok_or_else(|| CommandError::not_found("effect", effect))
    }

    /// How many effects the mixer track at `index` holds, when it can take
    /// one more.
    fn room_for_effect(&self, index: usize) -> Result<usize, CommandError> {
        let track = &self.project.mixer.tracks[index];
        if track.effects.len() >= MAX_EFFECT_SLOTS {
            return Err(CommandError::invalid(format!(
                "the mixer track \"{}\" is full: it holds {MAX_EFFECT_SLOTS} effects at most",
                track.name
            )));
        }
        Ok(track.effects.len())
    }

    /// The index of the mixer track that is `channel`'s own and still has
    /// the channel's name, as the track made with a channel does: a rename
    /// of the channel is then a rename of both. A track is the channel's
    /// own when nothing else plays into it: no other channel, no audio
    /// clip, and no other track through its output or a send. The master
    /// is nobody's own, and a track the user gave another name keeps it.
    fn track_named_after(&self, channel: &Channel) -> Option<usize> {
        let project = &*self.project;
        let id = channel.mixer_track;
        let tracks = &project.mixer.tracks;
        let index = tracks.iter().position(|track| track.id == id)?;
        let shared = id == TrackId::MASTER
            || tracks[index].name != channel.name
            || project
                .channels
                .iter()
                .any(|other| other.id != channel.id && other.mixer_track == id)
            || project.playlist.clips.iter().any(|clip| {
                matches!(clip.content, ClipContent::Audio { mixer_track, .. } if mixer_track == id)
            })
            || tracks.iter().any(|track| {
                track.output == Some(id) || track.sends.iter().any(|send| send.target == id)
            });
        (!shared).then_some(index)
    }

    /// Edits the mixer track at `index`.
    fn change_track(&mut self, index: usize, change: impl FnOnce(&mut MixerTrack)) {
        let old = self.project.mixer.tracks[index].clone();
        let mut new = old.clone();
        change(&mut new);
        self.push(Edit::SetMixerTrack(Box::new(Change { old, new })));
    }

    /// The sampler settings of the channel at `index`. Fails on a channel
    /// that plays an instrument.
    fn sampler(&self, index: usize) -> Result<SamplerSettings, CommandError> {
        let channel = &self.project.channels[index];
        match &channel.source {
            ChannelSource::Sampler(sampler) => Ok(sampler.clone()),
            ChannelSource::Instrument { .. } => Err(CommandError::invalid(format!(
                "the channel \"{}\" plays an instrument, so it has no sampler settings",
                channel.name
            ))),
        }
    }

    fn set_sampler(&mut self, index: usize, sampler: SamplerSettings) {
        self.set_source(index, ChannelSource::Sampler(sampler));
    }

    /// The instrument settings of the channel at `index`. Fails on a
    /// sampler channel.
    fn instrument(&self, index: usize) -> Result<InstrumentParams, CommandError> {
        let channel = &self.project.channels[index];
        match &channel.source {
            ChannelSource::Instrument { params } => Ok(*params),
            ChannelSource::Sampler(_) => Err(CommandError::invalid(format!(
                "the channel \"{}\" is a sampler, so it has no instrument settings",
                channel.name
            ))),
        }
    }

    fn set_instrument(&mut self, index: usize, params: InstrumentParams) {
        self.set_source(index, ChannelSource::Instrument { params });
    }

    fn set_source(&mut self, index: usize, source: ChannelSource) {
        let old = self.project.channels[index].clone();
        let new = Channel {
            source,
            ..old.clone()
        };
        self.push(Edit::SetChannel(Box::new(Change { old, new })));
    }
}

/// The notes of a lane that `keep` accepts.
fn notes_where(lane: Option<&Lane>, keep: impl Fn(&Note) -> bool) -> Vec<Note> {
    let notes = lane.map_or(&[][..], |lane| lane.notes.as_slice());
    notes.iter().filter(|note| keep(note)).copied().collect()
}

fn new_mixer_track(id: TrackId, name: String, color: u32) -> MixerTrack {
    MixerTrack {
        id,
        name,
        color,
        volume: 1.0,
        pan: 0.0,
        muted: false,
        solo: false,
        output: Some(TrackId::MASTER),
        sends: Vec::new(),
        effects: Vec::new(),
    }
}

/// True for a target that is a setting, or the mix, of this effect.
fn targets_effect(target: &AutomationTarget, id: EffectId) -> bool {
    match *target {
        AutomationTarget::EffectParam { effect, .. }
        | AutomationTarget::EffectMix { effect, .. } => effect == id,
        _ => false,
    }
}

/// The points of a curve, with every value and curve inside its range.
fn checked_points(points: Vec<AutomationPoint>) -> Result<Vec<AutomationPoint>, CommandError> {
    if points.is_empty() {
        return Err(CommandError::invalid(
            "an automation needs at least one point",
        ));
    }
    if points.len() > MAX_AUTOMATION_POINTS {
        return Err(CommandError::invalid(format!(
            "an automation can have {MAX_AUTOMATION_POINTS} points at most, not {}",
            points.len()
        )));
    }
    if !points.is_sorted_by_key(|point| point.tick) {
        return Err(CommandError::invalid(
            "the points of an automation must be in order of time",
        ));
    }
    let longest = MAX_SONG_TICKS / PPQ;
    if points
        .last()
        .is_some_and(|point| point.tick > MAX_SONG_TICKS)
    {
        return Err(CommandError::invalid(format!(
            "a point of the automation is past the end of the longest song, which has {longest} beats"
        )));
    }
    points
        .into_iter()
        .map(|point| {
            Ok(AutomationPoint {
                value: clamped("the value of a point", point.value, 0.0, 1.0)?,
                curve: clamped("the curve of a point", point.curve, -1.0, 1.0)?,
                ..point
            })
        })
        .collect()
}

/// A value for one setting of an effect or instrument, ready for the
/// setting to bring into its own range. Infinity means the end of the range.
/// NaN has no nearest valid value, so it is an error.
fn checked_setting(value: f32) -> Result<f32, CommandError> {
    if value.is_nan() {
        return Err(CommandError::invalid("the value is not a number"));
    }
    // Adding zero turns -0.0 into 0.0, which keeps "-0.0" out of project files.
    Ok(value.clamp(f32::MIN, f32::MAX) + 0.0)
}

/// Brings every setting of an effect into its range, the way setting them
/// one by one would.
fn checked_effect(mut params: EffectParams) -> Result<EffectParams, CommandError> {
    for index in 0..params.kind().descriptors().len() {
        if let Some(value) = params.get(index) {
            params.set(index, checked_setting(value)?);
        }
    }
    let params = params.sanitized();
    // What no setting reaches is not cleaned by the loop above.
    if params.sanitized() != params {
        return Err(CommandError::invalid(
            "the settings hold a value that is not a number",
        ));
    }
    Ok(params)
}

/// Brings every setting of an instrument into its range, the way setting
/// them one by one would.
fn checked_instrument(mut params: InstrumentParams) -> Result<InstrumentParams, CommandError> {
    for index in 0..params.kind().descriptors().len() {
        if let Some(value) = params.get(index) {
            params.set(index, checked_setting(value)?);
        }
    }
    let params = params.sanitized();
    if params.sanitized() != params {
        return Err(CommandError::invalid(
            "the settings hold a value that is not a number",
        ));
    }
    Ok(params)
}

/// The history label for a change to the setting at `param` of a processor
/// with these descriptors: "Change" and the setting's name, as in "Change
/// Cutoff" or "Change Osc 1 level".
fn setting_label(descriptors: &[ParamInfo], param: u32) -> String {
    match descriptors.get(param as usize) {
        Some(info) => format!("Change {}", info.name),
        None => "Change setting".to_owned(),
    }
}

fn no_setting(processor: &str, param: u32) -> CommandError {
    CommandError::invalid(format!("the {processor} has no setting number {param}"))
}

/// The error for settings of one kind of processor given to another kind.
fn other_kind(given: &str, held: &str) -> CommandError {
    CommandError::invalid(format!(
        "these are the settings of a {given}, and this is a {held}"
    ))
}

/// Brings a value into its range. NaN has no nearest valid value, so it is
/// an error.
fn clamped(what: &str, value: f32, min: f32, max: f32) -> Result<f32, CommandError> {
    if value.is_nan() {
        return Err(CommandError::invalid(format!("{what} is not a number")));
    }
    // Adding zero turns -0.0 into 0.0, which keeps "-0.0" out of project files.
    Ok(value.clamp(min, max) + 0.0)
}

fn checked_color(color: u32) -> Result<u32, CommandError> {
    if color > 0xFF_FFFF {
        return Err(CommandError::invalid(format!(
            "{color:#x} is not a 0xRRGGBB color"
        )));
    }
    Ok(color)
}

fn checked_key(key: u8) -> Result<u8, CommandError> {
    if key > MAX_KEY {
        return Err(CommandError::invalid(format!(
            "key {key} is above the highest key, {MAX_KEY}"
        )));
    }
    Ok(key)
}

fn checked_note(note: Note) -> Result<Note, CommandError> {
    if note.length == 0 {
        return Err(CommandError::invalid(
            "a note must be at least one tick long",
        ));
    }
    if note.start.checked_add(note.length).is_none() {
        return Err(CommandError::invalid("the note ends past the last tick"));
    }
    Ok(Note {
        key: checked_key(note.key)?,
        velocity: clamped("the velocity", note.velocity, 0.0, 1.0)?,
        pan: clamped("the pan", note.pan, -1.0, 1.0)?,
        ..note
    })
}

/// A note that an edit puts at a new start: [`checked_note`], and the start
/// has to lie inside the longest pattern.
fn placed_note(note: Note) -> Result<Note, CommandError> {
    if note.start >= MAX_PATTERN_TICKS {
        return Err(CommandError::invalid(format!(
            "the note would start at step {}, past the end of the longest pattern, which has {MAX_PATTERN_STEPS} steps",
            note.start / TICKS_PER_STEP + 1
        )));
    }
    checked_note(note)
}

fn checked_clip(clip: Clip) -> Result<Clip, CommandError> {
    if clip.length == 0 {
        return Err(CommandError::invalid(
            "a clip must be at least one tick long",
        ));
    }
    let longest = MAX_SONG_TICKS / PPQ;
    if u64::from(clip.start) + u64::from(clip.length) > u64::from(MAX_SONG_TICKS) {
        return Err(CommandError::invalid(format!(
            "the clip would end past the end of the longest song, which has {longest} beats"
        )));
    }
    if clip.offset > MAX_SONG_TICKS {
        return Err(CommandError::invalid(format!(
            "the clip would start further into what it plays than the longest song lasts, which is {longest} beats"
        )));
    }
    Ok(clip)
}

/// The error for the first id of `wanted` that is not among `found`.
fn first_missing<I: Copy + Eq + Hash + Into<u32>>(
    kind: &'static str,
    wanted: impl IntoIterator<Item = I>,
    found: impl IntoIterator<Item = I>,
) -> CommandError {
    let found: HashSet<I> = found.into_iter().collect();
    let missing = wanted.into_iter().find(|id| !found.contains(id));
    CommandError::not_found(kind, missing.map_or(0, Into::into))
}

/// The label of the one field a patch sets, or `many` when it sets several
/// or none.
fn single_label(fields: &[(bool, Label)], many: Label) -> Label {
    let mut set = fields.iter().filter(|(is_set, _)| *is_set);
    match (set.next(), set.next()) {
        (Some((_, label)), None) => label,
        _ => many,
    }
}

fn plural(count: usize, one: Label, many: Label) -> Label {
    if count == 1 { one } else { many }
}

/// "`prefix` N" for the first N from `start` up that no existing name uses.
fn numbered_name<'a>(
    prefix: &str,
    start: usize,
    existing: impl Iterator<Item = &'a str>,
) -> String {
    let taken: HashSet<&str> = existing.collect();
    (start..)
        .map(|number| format!("{prefix} {number}"))
        .find(|name| !taken.contains(name.as_str()))
        .unwrap_or_else(|| prefix.to_owned())
}

/// `name` if no existing name is the same, and otherwise "`name` N" for
/// the first N from 2 up that is free. Capitals make no difference to what
/// counts as the same.
fn unused_name<'a>(name: String, existing: impl Iterator<Item = &'a str>) -> String {
    let taken: Vec<&str> = existing.collect();
    let free = |name: &str| !taken.iter().any(|held| held.eq_ignore_ascii_case(name));
    if free(&name) {
        return name;
    }
    let numbered = (2..).map(|number| format!("{name} {number}"));
    numbered
        .into_iter()
        .find(|numbered| free(numbered))
        .unwrap_or(name)
}

/// The name for a copy: "Kick" becomes "Kick #2", and "Kick #2" becomes
/// "Kick #3", skipping names already in use.
fn copy_name<'a>(original: &str, existing: impl Iterator<Item = &'a str>) -> String {
    let base = match original.rsplit_once(" #") {
        Some((base, number)) if number.parse::<u32>().is_ok() => base,
        _ => original,
    };
    let taken: HashSet<&str> = existing.collect();
    (2..)
        .map(|number| format!("{base} #{number}"))
        .find(|name| !taken.contains(name.as_str()))
        .unwrap_or_else(|| original.to_owned())
}
