//! File/DSP work holds neither document nor audio/controller locks. The editor
//! lock only serializes these workers; it is never read by the audio callback.
use super::{Session, State};
use crate::sync::lock;
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use windfall_core::{AudioBuffer, samples_per_tick};
use windfall_engine::{Controller, audio_edit};
use windfall_ipc::{AudioEditOperation, AudioEditPreview, AudioEditRequest};
use windfall_project::{
    AutomationTarget, Clip, ClipContent, ClipId, ClipInit, Command, DispatchResult, MAX_SONG_TICKS,
    SampleId, SamplePath,
};

const STALE: &str = "The project or source changed. Reopen the audio editor and try again.";
static SERIAL: AtomicU64 = AtomicU64::new(0);

#[derive(Default)]
pub(super) struct Editor {
    serial: u32,
    prepared: Option<Prepared>,
}
struct Prepared {
    token: u32,
    generation: u64,
    edits: u64,
    clip: Clip,
    source: AudioBuffer,
    view: AudioBuffer,
    tempo: f64,
}
impl Prepared {
    fn valid(&self, state: &State) -> bool {
        let sample = self.clip.content.sample().expect("editor only holds audio");
        state.generation == self.generation
            && state.edits == self.edits
            && state
                .document
                .project()
                .playlist
                .clips
                .iter()
                .any(|clip| clip == &self.clip)
            && state
                .pool
                .get(sample)
                .is_some_and(|now| same_audio(now, &self.source))
    }
}
fn same_audio(a: &AudioBuffer, b: &AudioBuffer) -> bool {
    a.samples().as_ptr() == b.samples().as_ptr()
        && a.samples().len() == b.samples().len()
        && a.sample_rate() == b.sample_rate()
        && a.channels() == b.channels()
}

/// Deletes refused/unattached output only, after all session guards drop.
struct Output(PathBuf);
impl Drop for Output {
    fn drop(&mut self) {
        if !self.0.as_os_str().is_empty() {
            let _ = fs::remove_file(&self.0);
        }
    }
}

impl Session {
    pub fn audio_editor_open(&self, id: ClipId) -> Result<AudioEditPreview, String> {
        let mut editor = lock(&self.inner.audio_editor);
        // Drop the previous view before allocating another. This is worker-only.
        editor.prepared = None;
        let (clip, pool, generation, edits, name, tempo) = {
            let _recording = self.recording_idle()?;
            let state = self.state();
            let project = state.document.project();
            if project
                .automations
                .iter()
                .any(|a| matches!(a.target, AutomationTarget::Tempo))
            {
                return Err("Audio editing with tempo automation is not supported yet.".into());
            }
            let clip = project
                .playlist
                .clips
                .iter()
                .find(|c| c.id == id)
                .cloned()
                .ok_or("The clip no longer exists.")?;
            let sample = clip.content.sample().ok_or("Select an audio clip.")?;
            let name = project
                .sample(sample)
                .ok_or("The sample is missing.")?
                .name
                .clone();
            (
                clip,
                state.pool.clone(),
                state.generation,
                state.edits,
                name,
                project.settings.tempo_bpm,
            )
        };
        let source = pool
            .get(clip.content.sample().unwrap())
            .cloned()
            .ok_or("The clip's audio is not loaded. Reload samples first.")?;
        let view = audio_edit::render_view(&pool, &clip, tempo)?;
        editor.serial = editor
            .serial
            .checked_add(1)
            .ok_or("Audio editor tokens exhausted. Restart Windfall.")?;
        let prepared = Prepared {
            token: editor.serial,
            generation,
            edits,
            clip,
            source,
            view,
            tempo,
        };
        let preview = AudioEditPreview {
            token: prepared.token,
            clip: id,
            name,
            frames: prepared.view.frames() as u32,
            sample_rate: prepared.view.sample_rate(),
            peaks: audio_edit::overview(&prepared.view, 512),
        };
        #[cfg(test)]
        self.pause("audio-editor:rendered");
        let _recording = self.recording_idle()?;
        if !prepared.valid(&self.state()) {
            return Err(STALE.into());
        }
        editor.prepared = Some(prepared);
        Ok(preview)
    }

    pub fn audio_editor_discard(&self, token: u32) {
        let mut editor = lock(&self.inner.audio_editor);
        if editor.prepared.as_ref().is_some_and(|p| p.token == token) {
            editor.prepared = None;
        }
    }

    pub fn audio_editor_apply(&self, request: AudioEditRequest) -> Result<DispatchResult, String> {
        let mut editor = lock(&self.inner.audio_editor);
        let prepared = editor
            .prepared
            .as_ref()
            .filter(|p| p.token == request.token)
            .ok_or("This audio editor has expired. Reopen it.")?;
        let (mut document, mut pool, generation, edits) = {
            let _recording = self.recording_idle()?;
            let state = self.state();
            if !prepared.valid(&state) {
                return Err(STALE.into());
            }
            (
                state.document.clone(),
                state.pool.clone(),
                state.generation,
                state.edits,
            )
        };
        let output = audio_edit::edit(
            &prepared.view,
            request.operation,
            request.start_frame,
            request.end_frame,
        )?;
        let per_tick = samples_per_tick(prepared.tempo, f64::from(output.sample_rate()));
        let length = if matches!(
            request.operation,
            AudioEditOperation::Trim | AudioEditOperation::Extract | AudioEditOperation::Cut
        ) {
            ((output.frames() as f64 / per_tick).ceil().max(1.0) as u32).min(prepared.clip.length)
        } else {
            prepared.clip.length
        };
        let shift = if request.operation == AudioEditOperation::Extract {
            prepared.clip.length
        } else if request.operation == AudioEditOperation::Trim {
            (f64::from(request.start_frame) / per_tick).round() as u32
        } else {
            0
        };
        let clip = &prepared.clip;
        let start = clip
            .start
            .checked_add(shift)
            .ok_or("Edit falls outside the song.")?;
        if start.saturating_add(length) > MAX_SONG_TICKS {
            return Err("Edit falls outside the song.".into());
        }
        // Persistent app-owned storage works before the project has a save path.
        // Completed files remain for undo/redo and saved documents.
        let folder = self.store().recordings_dir().join("Audio edits");
        fs::create_dir_all(&folder)
            .map_err(|e| format!("Could not create audio edit folder: {e}"))?;
        let mut owned = None;
        for _ in 0..1000 {
            let path = folder.join(format!(
                "Edit-{}-{}.wav",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::Relaxed)
            ));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(_) => {
                    owned = Some(Output(path));
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(format!("Could not create audio edit file: {e}")),
            }
        }
        let mut owned = owned.ok_or("Could not reserve an audio edit file.")?;
        windfall_codec::write_wav(&owned.0, &output, windfall_codec::WavSampleFormat::Float32)
            .map_err(|e| e.to_string())?;
        // Populate ordinary sample cache off-lock too, so history can restore it
        // without decoding under state. WAV Float32 preserves the edit exactly.
        let decoded = self.inner.cache.decode(&owned.0)?;
        let sample = SampleId(document.project().next_id);
        let label = format!("Audio edit: {:?}", request.operation);
        let mut commands = vec![Command::AddSample {
            name: format!(
                "{} ({:?})",
                document
                    .project()
                    .sample(clip.content.sample().unwrap())
                    .unwrap()
                    .name,
                request.operation
            ),
            path: SamplePath::External(crate::paths::display(&owned.0)),
        }];
        if request.operation != AudioEditOperation::Extract {
            commands.push(Command::RemoveClips {
                clips: vec![clip.id],
            });
        }
        let ClipContent::Audio { mixer_track, output, .. } = clip.content else {
            unreachable!()
        };
        commands.push(Command::AddClips {
            clips: vec![ClipInit {
                track: clip.track,
                start,
                length: Some(length),
                offset: Some(0),
                muted: Some(clip.muted),
                content: ClipContent::Audio {
                    sample,
                    mixer_track,
                    output,
                    gain: 1.0,
                    pan: 0.0,
                    fade_in: 0,
                    fade_out: 0,
                    reverse: false,
                    pitch: 0.0,
                    stretch: Default::default(),
                },
            }],
        });
        let command = Command::Batch {
            label: Some(label),
            commands,
        };
        document
            .dispatch(command.clone(), None)
            .map_err(|e| e.to_string())?;
        pool.insert(sample, decoded.clone());
        let plan = Controller::prepare_project(document.project(), &pool)
            .map_err(|error| error.to_string())?;
        #[cfg(test)]
        self.pause("audio-editor:prepared");
        let _recording = self.recording_idle()?;
        let mut state = self.state();
        if state.generation != generation
            || state.edits != edits
            || !prepared.valid(&state)
            || state
                .pool
                .iter()
                .any(|(id, now)| pool.get(id).is_none_or(|old| !same_audio(now, old)))
            || pool.iter().any(|(id, old)| {
                id != sample && state.pool.get(id).is_none_or(|now| !same_audio(now, old))
            })
        {
            return Err(STALE.into());
        }
        let applied = state
            .document
            .dispatch(command, None)
            .map_err(|e| e.to_string())?;
        state.pool.insert(sample, decoded);
        state.loaded.insert(sample);
        state.failed.remove(&sample);
        let result = DispatchResult {
            created: applied.created,
            patch: self.publish_prepared(&mut state, &applied.touched, plan),
        };
        owned.0 = PathBuf::new(); // successful installation owns the persistent file
        drop(state);
        drop(_recording);
        editor.prepared = None;
        Ok(result)
    }
}
