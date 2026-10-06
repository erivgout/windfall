//! Turns commands into primitive edits.
//!
//! Each command checks everything it needs before it makes its first edit,
//! so a command that fails has changed nothing. A batch runs its commands one
//! after another against the live project, so a later command sees what an
//! earlier one did; when one fails, the edits made so far are taken back.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use crate::check::{reaches, time_signature_problem};
use crate::command::{
    ChannelPatch, ClipInit, ClipPatch, ClipUpdate, Command, MixerTrackPatch, NoteInit, NotePatch,
    NoteUpdate, PatternPatch, PlaylistTrackPatch, SamplerPatch, SettingsPatch,
};
use crate::edit::{self, Change, Direction, Edit, ListEdit, Move, PatternInfo};
use crate::error::CommandError;
use crate::model::{
    Channel, ChannelId, ChannelSource, Clip, ClipContent, ClipId, DEFAULT_CHANNEL_VOLUME,
    DEFAULT_KEY, DEFAULT_PATTERN_STEPS, DEFAULT_VELOCITY, Envelope, Lane, MAX_ENVELOPE_MS,
    MAX_GAIN, MAX_KEY, MAX_MIXER_TRACKS, MAX_PATTERN_STEPS, MAX_TEMPO_BPM, MAX_TUNE_SEMITONES,
    MIN_TEMPO_BPM, MixerTrack, Note, NoteId, Pattern, PatternId, PlaylistTrack, PlaylistTrackId,
    Project, SampleAsset, SampleId, SamplePath, SamplerSettings, Send, TICKS_PER_STEP, TrackId,
    palette_color,
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
            Command::UpdateSettings { patch } => self.update_settings(patch)?,
            Command::AddSample { name, path } => self.add_sample(name, path)?,
            Command::RemoveSample { id } => self.remove_sample(id)?,
            Command::AddChannel {
                name,
                sample,
                index,
                mixer_track,
            } => self.add_channel(name, sample, index, mixer_track)?,
            Command::RemoveChannel { id } => self.remove_channel(id)?,
            Command::DuplicateChannel { id } => self.duplicate_channel(id)?,
            Command::MoveChannel { id, index } => self.move_channel(id, index)?,
            Command::UpdateChannel { id, patch } => self.update_channel(id, patch)?,
            Command::SetChannelSample { id, sample } => self.set_channel_sample(id, sample)?,
            Command::UpdateSampler { id, patch } => self.update_sampler(id, patch)?,
            Command::SetSamplerEnvelope { id, envelope } => {
                self.set_sampler_envelope(id, envelope)?
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
            Command::AddMixerTrack { name } => self.add_mixer_track(name)?,
            Command::RemoveMixerTrack { id } => self.remove_mixer_track(id)?,
            Command::UpdateMixerTrack { id, patch } => self.update_mixer_track(id, patch)?,
            Command::SetTrackOutput { id, output } => self.set_track_output(id, output)?,
            Command::SetSend { from, to, gain } => self.set_send(from, to, gain)?,
            Command::AddPlaylistTrack { name } => self.add_playlist_track(name)?,
            Command::RemovePlaylistTrack { id } => self.remove_playlist_track(id)?,
            Command::UpdatePlaylistTrack { id, patch } => self.update_playlist_track(id, patch)?,
            Command::AddClips { clips } => self.add_clips(&clips)?,
            Command::RemoveClips { clips } => self.remove_clips(&clips)?,
            Command::UpdateClips { updates } => self.update_clips(&updates)?,
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
        Ok(label.to_owned())
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
        let user = self.project.channels.iter().find(|channel| {
            let ChannelSource::Sampler(sampler) = &channel.source;
            sampler.sample == Some(id)
        });
        if let Some(channel) = user {
            return Err(CommandError::invalid(format!(
                "the sample is still used by the channel \"{}\"",
                channel.name
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
        index: Option<u32>,
        mixer_track: Option<TrackId>,
    ) -> Result<Label, CommandError> {
        let sample_name = match sample {
            Some(id) => Some(self.project.samples[self.sample_index(id)?].name.clone()),
            None => None,
        };
        if let Some(track) = mixer_track {
            self.mixer_track_index(track)?;
        }
        let name = name.or(sample_name).unwrap_or_else(|| "Sampler".to_owned());
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
        self.push(Edit::Channel(ListEdit::Insert {
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
                source: ChannelSource::Sampler(SamplerSettings {
                    sample,
                    ..SamplerSettings::default()
                }),
            },
        }));
        self.created.push(id.0);
        if made_track {
            self.created.push(track.0);
        }
        Ok("Add channel")
    }

    fn remove_channel(&mut self, id: ChannelId) -> Result<Label, CommandError> {
        let index = self.channel_index(id)?;
        for (pattern, notes) in self.lanes_of(id) {
            self.push(Edit::Notes {
                pattern,
                channel: id,
                remove: notes,
                insert: Vec::new(),
            });
        }
        let item = self.project.channels[index].clone();
        self.push(Edit::Channel(ListEdit::Remove { index, item }));
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
        self.push(Edit::Channel(ListEdit::Insert {
            index: index + 1,
            item: copy,
        }));
        for (pattern, notes) in lanes {
            self.push(Edit::Notes {
                pattern,
                channel: copy_id,
                remove: Vec::new(),
                insert: notes,
            });
        }
        self.created.push(copy_id.0);
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
        self.push(Edit::SetChannel(Box::new(Change { old, new })));
        Ok(label)
    }

    fn set_channel_sample(
        &mut self,
        id: ChannelId,
        sample: Option<SampleId>,
    ) -> Result<Label, CommandError> {
        let index = self.channel_index(id)?;
        if let Some(sample) = sample {
            self.sample_index(sample)?;
        }
        self.change_sampler(index, |sampler| sampler.sample = sample);
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
                (patch.cut_self.is_some(), "Toggle cut itself"),
                (patch.cut_group.is_some(), "Change cut group"),
            ],
            "Change sampler",
        );
        let index = self.channel_index(id)?;
        let ChannelSource::Sampler(mut sampler) = self.project.channels[index].source.clone();
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
        if let Some(cut_self) = patch.cut_self {
            sampler.cut_self = cut_self;
        }
        if let Some(cut_group) = patch.cut_group {
            sampler.cut_group = cut_group;
        }
        self.change_sampler(index, |current| *current = sampler);
        Ok(label)
    }

    fn set_sampler_envelope(
        &mut self,
        id: ChannelId,
        envelope: Option<Envelope>,
    ) -> Result<Label, CommandError> {
        let index = self.channel_index(id)?;
        let envelope = match envelope {
            Some(envelope) => Some(Envelope {
                attack_ms: clamped("the attack", envelope.attack_ms, 0.0, MAX_ENVELOPE_MS)?,
                decay_ms: clamped("the decay", envelope.decay_ms, 0.0, MAX_ENVELOPE_MS)?,
                sustain: clamped("the sustain", envelope.sustain, 0.0, 1.0)?,
                release_ms: clamped("the release", envelope.release_ms, 0.0, MAX_ENVELOPE_MS)?,
            }),
            None => None,
        };
        self.change_sampler(index, |sampler| sampler.envelope = envelope);
        Ok("Change envelope")
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
        let clips = self.clips_where(|clip| {
            let ClipContent::Pattern { pattern } = &clip.content;
            *pattern == id
        });
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
        let start = step
            .checked_mul(TICKS_PER_STEP)
            .filter(|start| start.checked_add(TICKS_PER_STEP).is_some())
            .ok_or_else(|| CommandError::invalid(format!("step {step} is past the last tick")))?;
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
            insert.push(checked_note(Note {
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
        let moves = updates
            .iter()
            .any(|u| u.patch.start.is_some() || u.patch.key.is_some());
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
        for update in updates {
            if let Some(note) = changed.get_mut(&update.id) {
                let patch = &update.patch;
                *note = checked_note(Note {
                    id: note.id,
                    start: patch.start.unwrap_or(note.start),
                    length: patch.length.unwrap_or(note.length),
                    key: patch.key.unwrap_or(note.key),
                    velocity: patch.velocity.unwrap_or(note.velocity),
                    pan: patch.pan.unwrap_or(note.pan),
                })?;
            }
        }
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

    fn add_playlist_track(&mut self, name: Option<String>) -> Result<Label, CommandError> {
        let tracks = &self.project.playlist.tracks;
        let count = tracks.len();
        let name = name.unwrap_or_else(|| {
            numbered_name("Track", count + 1, tracks.iter().map(|t| t.name.as_str()))
        });
        let id = PlaylistTrackId(self.allocate()?);
        self.push(Edit::PlaylistTrack(ListEdit::Insert {
            index: count,
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

    fn add_clips(&mut self, clips: &[ClipInit]) -> Result<Label, CommandError> {
        let mut insert = Vec::with_capacity(clips.len());
        for init in clips {
            self.playlist_track_index(init.track)?;
            let ClipContent::Pattern { pattern } = &init.content;
            let pattern = &self.project.patterns[self.pattern_index(*pattern)?];
            // The id is filled in below, once every clip is known to be valid.
            insert.push(checked_clip(Clip {
                id: ClipId(0),
                track: init.track,
                start: init.start,
                length: init.length.unwrap_or_else(|| pattern.length_ticks()),
                offset: 0,
                muted: false,
                content: init.content.clone(),
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

    /// Edits the sampler of the channel at `index`.
    fn change_sampler(&mut self, index: usize, change: impl FnOnce(&mut SamplerSettings)) {
        let old = self.project.channels[index].clone();
        let mut new = old.clone();
        let ChannelSource::Sampler(sampler) = &mut new.source;
        change(sampler);
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
    }
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

fn checked_clip(clip: Clip) -> Result<Clip, CommandError> {
    if clip.length == 0 {
        return Err(CommandError::invalid(
            "a clip must be at least one tick long",
        ));
    }
    if clip.start.checked_add(clip.length).is_none() {
        return Err(CommandError::invalid("the clip ends past the last tick"));
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
