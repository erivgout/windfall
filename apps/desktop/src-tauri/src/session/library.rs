//! The browser panel, and bringing audio files into the project.

use std::path::{Path, PathBuf};

use windfall_core::{AudioBuffer, PPQ};
use windfall_ipc::{BrowserEntry, BrowserRoot, SampleInfo};
use windfall_project::file::sample_path_for;
use windfall_project::{
    ChannelId, ClipContent, ClipInit, Command, DispatchResult, MAX_MIXER_TRACKS, MAX_SONG_TICKS,
    PlaylistTrackId, Project, SampleId, TrackId,
};

use super::samples::locate;
use super::{Session, State};
use crate::browser;
use crate::paths;

/// What a call about a sample says when the project it was made for has
/// been replaced by another.
pub const PROJECT_REPLACED: &str = "Another project was opened in the meantime.";

/// Where on the playlist an audio clip is to go.
#[derive(Debug, Clone, Copy)]
pub struct ClipPlace {
    /// The playlist track. `None` adds a track for the clip, at the end.
    pub track: Option<PlaylistTrackId>,
    /// The tick the clip starts on.
    pub start: u32,
    /// The mixer track the clip plays into. `None` leaves it to the
    /// project: the clip joins the audio clip of the same sample that was
    /// made last, on whatever mixer track that one plays into now, so that
    /// a file dropped three times is on one track and not on three. Only
    /// when no clip plays the sample is a mixer track added, named after
    /// the audio and routed to the master; and when the mixer is full the
    /// clip plays into the master instead, as a new channel does.
    pub mixer_track: Option<TrackId>,
}

/// An audio file that is decoded and about to be put to use in the project.
struct Import {
    file: PathBuf,
    buffer: AudioBuffer,
    /// The document that was open when decoding started.
    generation: u64,
}

impl Session {
    /// The browser's top-level folders: the factory content first, then the
    /// folders the user added, in the order they were added.
    pub fn browser_roots(&self) -> Vec<BrowserRoot> {
        let settings = self.store();
        self.roots(&settings.settings().browser_roots)
    }

    /// Adds a folder to the browser and remembers it.
    pub fn browser_add_root(&self, path: &str) -> Result<Vec<BrowserRoot>, String> {
        if path.trim().is_empty() {
            return Err("Choose a folder to add.".to_owned());
        }
        let folder = paths::absolute(path)?;
        let shown = paths::display(&folder);
        if !folder.is_dir() {
            return Err(format!("\"{shown}\" is not a folder."));
        }
        let mut settings = self.store();
        let known = paths::same(&folder, &self.inner.factory_dir)
            || settings
                .settings()
                .browser_roots
                .iter()
                .any(|root| paths::same(Path::new(root), &folder));
        if known {
            return Err(format!("\"{shown}\" is already in the browser."));
        }
        settings.update(|stored| stored.browser_roots.push(shown));
        Ok(self.roots(&settings.settings().browser_roots))
    }

    /// Takes a folder the user added out of the browser. Nothing on disk is
    /// touched.
    pub fn browser_remove_root(&self, path: &str) -> Result<Vec<BrowserRoot>, String> {
        let folder = paths::clean(Path::new(path));
        if paths::same(&folder, &self.inner.factory_dir) {
            return Err("The factory library cannot be removed.".to_owned());
        }
        let mut settings = self.store();
        let is_it = |root: &String| paths::same(Path::new(root), &folder);
        if !settings.settings().browser_roots.iter().any(is_it) {
            return Err(format!("\"{path}\" is not in the browser."));
        }
        settings.update(|stored| stored.browser_roots.retain(|root| !is_it(root)));
        Ok(self.roots(&settings.settings().browser_roots))
    }

    /// Lists a folder for the browser. Reads the disk.
    pub fn browser_list(&self, path: &str) -> Result<Vec<BrowserEntry>, String> {
        browser::list_folder(&paths::absolute(path)?)
    }

    /// Facts about an audio file and its waveform overview. Slow the first
    /// time a file is asked for.
    pub fn sample_info(&self, path: &str) -> Result<SampleInfo, String> {
        self.inner.cache.info(&paths::absolute(path)?)
    }

    /// The same as [`sample_info`](Self::sample_info) for a sample of the
    /// project, whose stored path the UI cannot resolve by itself.
    ///
    /// Refused if another project was opened while the file was read: the
    /// id then means another sample, or none, and the answer would be shown
    /// as that sample's.
    pub fn sample_info_by_id(&self, sample: SampleId) -> Result<SampleInfo, String> {
        let (file, generation) = {
            let state = self.state();
            let asset = state
                .document
                .project()
                .sample(sample)
                .ok_or_else(|| format!("sample {} does not exist", sample.0))?;
            let file = locate(asset, state.project_dir(), &self.inner.factory_dir)?;
            (file, state.generation)
        };
        let info = self.inner.cache.info(&file)?;
        #[cfg(test)]
        self.pause("sample-info:read");
        if self.state().generation != generation {
            return Err(PROJECT_REPLACED.to_owned());
        }
        Ok(info)
    }

    /// Adds a channel that plays the audio file at `path`, as one undo
    /// step. The channel is named after the file and goes at `index` in the
    /// rack, or at the end. Creates the sample, the channel and the
    /// channel's mixer track, in that order; a file the project already
    /// uses is not added again, and its sample is the one reported.
    ///
    /// Slow the first time a file is asked for. A file that cannot be
    /// decoded is refused, and the project is not touched. Neither is a
    /// project that was opened while the file was being decoded: the
    /// channel was asked for in the one before it.
    pub fn add_channel_from_file(
        &self,
        path: &str,
        index: Option<u32>,
    ) -> Result<DispatchResult, String> {
        let import = self.decode_for_project(path)?;
        let name = paths::stem(&import.file);
        self.dispatch_with_sample(import, "Add channel", |sample| Command::AddChannel {
            name: Some(name.clone()),
            sample: Some(sample),
            instrument: None,
            index,
            mixer_track: None,
        })
    }

    /// Makes a channel play the audio file at `path`, as one undo step.
    /// Creates the sample, or reports the one the project already has for
    /// that file.
    ///
    /// Refused if another project was opened while the file was being
    /// decoded, in which the channel's id may name some other channel.
    pub fn set_channel_sample_from_file(
        &self,
        channel: ChannelId,
        path: &str,
    ) -> Result<DispatchResult, String> {
        let import = self.decode_for_project(path)?;
        self.dispatch_with_sample(import, "Change channel sample", |sample| {
            Command::SetChannelSample {
                id: channel,
                sample: Some(sample),
            }
        })
    }

    /// Puts the audio file at `path` on the playlist as an audio clip, as
    /// one undo step. The clip is as long as the file lasts at the tempo
    /// the project has now, at unity gain, with no fades.
    ///
    /// Creates, in this order: the sample, or the one the project already
    /// has for that file, which is reported in its place; the playlist
    /// track, if `place` named none; the mixer track, if one was made; and
    /// the clip, which is always last. A mixer track is made only when
    /// `place` named none, no audio clip plays the sample yet and the
    /// mixer has room, so its id is often left out: see
    /// [`ClipPlace::mixer_track`].
    ///
    /// Slow the first time a file is asked for. A file that cannot be
    /// decoded is refused, and the project is not touched. Neither is a
    /// project that was opened while the file was being decoded.
    pub fn add_audio_clip_from_file(
        &self,
        path: &str,
        place: ClipPlace,
    ) -> Result<DispatchResult, String> {
        let import = self.decode_for_project(path)?;
        let Import {
            file,
            buffer,
            generation,
        } = import;
        let mut state = self.state();
        if state.generation != generation {
            return Err(format!(
                "\"{}\" was not added, because another project was opened while it was loading.",
                paths::name(&file)
            ));
        }
        let sample_path = sample_path_for(&file, state.project_dir(), &self.inner.factory_dir);
        let project = state.document.project();
        let held = project.samples.iter().find(|held| held.path == sample_path);
        let name = paths::stem(&file);
        let (sample, next_id) = match held {
            Some(held) => (held.id, project.next_id),
            None => (SampleId(project.next_id), project.next_id.saturating_add(1)),
        };
        let mut commands = vec![Command::AddSample {
            name: name.clone(),
            path: sample_path,
        }];
        commands.extend(audio_clip_commands(
            project, sample, &buffer, &name, place, next_id,
        ));
        let batch = Command::Batch {
            label: Some("Add audio clip".to_owned()),
            commands,
        };
        let applied = state
            .document
            .dispatch(batch, None)
            .map_err(|error| error.to_string())?;

        hold_sample(&mut state, sample, buffer);
        Ok(DispatchResult {
            created: applied.created,
            patch: self.publish(&mut state, &applied.touched),
        })
    }

    /// Puts a sample the project already has on the playlist as an audio
    /// clip, as one undo step, the way
    /// [`add_audio_clip_from_file`](Self::add_audio_clip_from_file) does
    /// for a file.
    ///
    /// Creates, in this order: the playlist track, if `place` named none;
    /// the mixer track, if one was made, which is under the same
    /// conditions; and the clip, which is always last.
    ///
    /// Fails for a sample whose file is missing or still being read: how
    /// long the clip is to be is not known without its audio.
    pub fn add_audio_clip_from_sample(
        &self,
        sample: SampleId,
        place: ClipPlace,
    ) -> Result<DispatchResult, String> {
        let mut state = self.state();
        let project = state.document.project();
        let asset = project
            .sample(sample)
            .ok_or_else(|| format!("sample {} does not exist", sample.0))?;
        let buffer = state.pool.get(sample).ok_or_else(|| {
            format!(
                "The audio of \"{}\" is not loaded, so a clip of it cannot be made. Check that its file is there, then reload the samples.",
                asset.name
            )
        })?;
        let commands =
            audio_clip_commands(project, sample, buffer, &asset.name, place, project.next_id);
        let batch = Command::Batch {
            label: Some("Add audio clip".to_owned()),
            commands,
        };
        let applied = state
            .document
            .dispatch(batch, None)
            .map_err(|error| error.to_string())?;
        Ok(DispatchResult {
            created: applied.created,
            patch: self.publish(&mut state, &applied.touched),
        })
    }

    fn roots(&self, user: &[String]) -> Vec<BrowserRoot> {
        std::iter::once(browser::factory_root(&self.inner.factory_dir))
            .chain(user.iter().map(|root| browser::user_root(Path::new(root))))
            .collect()
    }

    /// Decodes a file before the project is changed to use it, and notes
    /// which project that is.
    fn decode_for_project(&self, path: &str) -> Result<Import, String> {
        let file = paths::absolute(path)?;
        let generation = self.state().generation;
        let buffer = self.inner.cache.decode(&file)?;
        #[cfg(test)]
        self.pause("import:decoded");
        Ok(Import {
            file,
            buffer,
            generation,
        })
    }

    /// Dispatches one undo step that adds the sample of an import and then
    /// runs the command `then` builds from the sample's id. Refuses if the
    /// project is no longer the one the import was decoded for.
    fn dispatch_with_sample(
        &self,
        import: Import,
        label: &str,
        then: impl FnOnce(SampleId) -> Command,
    ) -> Result<DispatchResult, String> {
        let Import {
            file,
            buffer,
            generation,
        } = import;
        let file = file.as_path();
        let mut state = self.state();
        if state.generation != generation {
            return Err(format!(
                "\"{}\" was not added, because another project was opened while it was loading.",
                paths::name(file)
            ));
        }
        let sample_path = sample_path_for(file, state.project_dir(), &self.inner.factory_dir);
        let project = state.document.project();
        // A batch cannot pass an id from one command to the next, so the id
        // the sample will get is worked out first: the one it already has,
        // or the next the project hands out.
        let sample = project
            .samples
            .iter()
            .find(|held| held.path == sample_path)
            .map_or(SampleId(project.next_id), |held| held.id);
        let batch = Command::Batch {
            label: Some(label.to_owned()),
            commands: vec![
                Command::AddSample {
                    name: paths::stem(file),
                    path: sample_path,
                },
                then(sample),
            ],
        };
        let applied = state
            .document
            .dispatch(batch, None)
            .map_err(|error| error.to_string())?;

        hold_sample(&mut state, sample, buffer);
        Ok(DispatchResult {
            created: applied.created,
            patch: self.publish(&mut state, &applied.touched),
        })
    }
}

/// The commands that put `sample`, whose audio is `buffer`, on the playlist
/// as an audio clip: a playlist track where `place` names none, a mixer
/// track where it names none and no clip of the sample has one to share,
/// then the clip. `next_id` is the id the project hands out to the first of
/// them.
fn audio_clip_commands(
    project: &Project,
    sample: SampleId,
    buffer: &AudioBuffer,
    name: &str,
    place: ClipPlace,
    mut next_id: u32,
) -> Vec<Command> {
    // A batch cannot pass an id from one command to the next, so the ids
    // the new tracks will get are worked out here, in the order the
    // commands allocate them.
    let mut allocate = || {
        let id = next_id;
        next_id = next_id.saturating_add(1);
        id
    };
    let mut commands = Vec::new();
    let track = place.track.unwrap_or_else(|| {
        commands.push(Command::AddPlaylistTrack {
            name: None,
            index: None,
        });
        PlaylistTrackId(allocate())
    });
    // The clip of this sample that was made last has the highest id.
    let clips = project.playlist.clips.iter();
    let shared = clips
        .filter_map(|clip| match clip.content {
            ClipContent::Audio {
                sample: played,
                mixer_track,
                ..
            } if played == sample => Some((clip.id, mixer_track)),
            _ => None,
        })
        .max_by_key(|(id, _)| *id)
        .map(|(_, mixer_track)| mixer_track);
    let mixer_track = match place.mixer_track.or(shared) {
        Some(mixer_track) => mixer_track,
        None if project.mixer.tracks.len() < MAX_MIXER_TRACKS => {
            commands.push(Command::AddMixerTrack {
                name: Some(name.to_owned()),
            });
            TrackId(allocate())
        }
        // A full mixer must not stop the user adding audio.
        None => TrackId::MASTER,
    };

    // As long as the audio lasts at the tempo the project has now, cut
    // where the longest song ends.
    let ticks = buffer.duration_secs() * project.settings.tempo_bpm * f64::from(PPQ) / 60.0;
    let room = MAX_SONG_TICKS.saturating_sub(place.start).max(1);
    let length = (ticks.ceil().max(1.0) as u32).min(room);
    commands.push(Command::AddClips {
        clips: vec![ClipInit {
            track,
            start: place.start,
            length: Some(length),
            offset: None,
            muted: None,
            content: ClipContent::Audio {
                sample,
                mixer_track,
                gain: 1.0,
                pan: 0.0,
                fade_in: 0,
                fade_out: 0,
                reverse: false,
                pitch: 0.0,
            },
        }],
    });
    commands
}

/// Puts audio that is already decoded into the pool, so the edit that needs
/// it plays at once instead of waiting for the file to be read again.
fn hold_sample(state: &mut State, sample: SampleId, buffer: AudioBuffer) {
    if state.document.project().sample(sample).is_some() && state.loaded.insert(sample) {
        state.pool.insert(sample, buffer);
        state.failed.remove(&sample);
    }
}
