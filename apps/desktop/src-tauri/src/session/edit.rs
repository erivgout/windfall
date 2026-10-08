//! Edits to the document, and telling everyone about them.

use windfall_project::{
    AutomationId, AutomationTarget, ClipContent, ClipInit, Command, DispatchResult,
    DocumentSnapshot, MAX_SONG_TICKS, PlaylistTrackId, ProjectPatch, Touched,
};

use super::{Session, State};
use crate::events::Event;

impl Session {
    /// Native notifications schedule here; neither State nor owner idle waits
    /// for a processor. Recording exclusion covers the joined owner lifetime.
    pub(crate) fn capture_plugin_update(
        &self,
        runtime: &crate::plugins::Runtime,
        request: crate::plugins::PendingUpdate,
    ) -> Result<bool, String> {
        let crate::plugins::Update::Capture { target, .. } = request.update else {
            return Err("Not a native state request".into());
        };
        let _recording = self.recording_idle()?;
        let current = |state: &State| {
            runtime.is_current(request.revision, request.token)
                && state
                    .document
                    .project()
                    .plugin(target)
                    .is_some_and(|binding| {
                        crate::plugins::binding_identity(binding) == request.binding
                    })
        };
        let desired = {
            let state = self.state();
            if !current(&state) {
                return Ok(false);
            }
            state
                .document
                .project()
                .plugin(target)
                .expect("checked binding")
                .parameters
                .clone()
        };
        let Some(captured) = runtime.capture_pending(request.clone(), desired.clone())? else {
            return Ok(false);
        };
        let mut state = self.state();
        if !current(&state) {
            return Ok(false);
        }
        let binding = state
            .document
            .project()
            .plugin(target)
            .expect("checked binding");
        if binding.parameters != desired {
            return Err("Plugin parameters changed during state capture; retrying".into());
        }
        let mut commands: Vec<_> = captured
            .parameters
            .iter()
            .filter(|(id, _)| {
                binding
                    .parameters
                    .iter()
                    .any(|param| param.id == *id && !param.read_only)
            })
            .map(|&(id, value)| Command::SetPluginParam { target, id, value })
            .collect();
        commands.push(Command::SetPluginState {
            target,
            state: captured.bytes.clone(),
        });
        let applied = state
            .document
            .dispatch(
                Command::Batch {
                    label: Some("Native plugin state".into()),
                    commands,
                },
                None,
            )
            .map_err(|error| error.to_string())?;
        self.publish(&mut state, &applied.touched);
        drop(state);
        runtime.acknowledge_capture(&request, &captured)?;
        Ok(captured.restart)
    }
    pub fn document_snapshot(&self) -> DocumentSnapshot {
        let state = self.state();
        state.document.snapshot(state.path_text())
    }

    /// Applies a command. The patch in the result is the one every window
    /// is sent.
    pub fn dispatch(
        &self,
        command: Command,
        gesture: Option<u64>,
    ) -> Result<DispatchResult, String> {
        let _recording = self.recording_idle()?;
        let mut state = self.state();
        let mut candidate = state.document.clone();
        candidate
            .dispatch(command.clone(), gesture)
            .map_err(|error| error.to_string())?;
        if state.pool.needs_sampler_preparation(candidate.project()) {
            drop(state);
            drop(_recording);
            return self.prepare_sampler_edit(command, gesture, None);
        }
        let applied = state
            .document
            .dispatch(command, gesture)
            .map_err(|error| error.to_string())?;
        Ok(DispatchResult {
            created: applied.created,
            patch: self.publish(&mut state, &applied.touched),
        })
    }

    /// Validate native ownership and apply its edit under the same document lock.
    /// An obsolete notification succeeds without changing the document; a
    /// temporary refusal remains an error so the manager can retain the edit.
    pub(crate) fn dispatch_plugin_update(
        &self,
        runtime: &crate::plugins::Runtime,
        (revision, token, binding): (u64, u64, u64),
        command: Command,
        gesture: Option<u64>,
    ) -> Result<bool, String> {
        #[cfg(test)]
        self.pause("plugin-update:apply");
        let _recording = self.recording_idle()?;
        let mut state = self.state();
        let target = match &command {
            Command::SetPluginParam { target, .. } | Command::SetPluginState { target, .. } => {
                *target
            }
            _ => return Err("This is not a native plugin update".into()),
        };
        if !runtime.is_current(revision, token)
            || !state
                .document
                .project()
                .plugin(target)
                .is_some_and(|current| crate::plugins::binding_identity(current) == binding)
        {
            return Ok(false);
        }
        let applied = state
            .document
            .dispatch(command, gesture)
            .map_err(|error| error.to_string())?;
        self.publish(&mut state, &applied.touched);
        Ok(true)
    }

    /// Makes an automation of `target` and puts it on the playlist, as one
    /// undo step: what "Create automation clip" on a knob or fader does.
    ///
    /// The automation starts as one point at the value the target has now,
    /// so nothing changes until the curve is drawn. Its clip goes on a new
    /// playlist track, at the end, from the start of the song for as long
    /// as the song is, and at least four bars.
    ///
    /// Creates, in this order: the automation, the playlist track and the
    /// clip. Fails when the project has nothing of that kind to automate.
    pub fn automate(&self, target: AutomationTarget) -> Result<DispatchResult, String> {
        let _recording = self.recording_idle()?;
        let mut state = self.state();
        let project = state.document.project();
        let clips = project.playlist.clips.iter();
        let song = clips
            .map(|clip| clip.start.saturating_add(clip.length))
            .max();
        let bars = project.settings.time_signature.ticks_per_bar() * 4;
        let length = song.unwrap_or(0).max(bars).min(MAX_SONG_TICKS);
        // A batch cannot pass an id from one command to the next, so the
        // ids the automation and the track will get are worked out first.
        let automation = AutomationId(project.next_id);
        let track = PlaylistTrackId(project.next_id.saturating_add(1));
        let batch = Command::Batch {
            label: Some("Create automation clip".to_owned()),
            commands: vec![
                Command::AddAutomation {
                    name: None,
                    target,
                    points: None,
                },
                Command::AddPlaylistTrack {
                    name: None,
                    index: None,
                },
                Command::AddClips {
                    clips: vec![ClipInit {
                        track,
                        start: 0,
                        length: Some(length),
                        offset: None,
                        muted: None,
                        content: ClipContent::Automation { automation },
                    }],
                },
            ],
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

    /// Undoes the last edit. `None` when there is nothing to undo.
    pub fn undo(&self) -> Option<ProjectPatch> {
        self.prepare_history(HistoryMove::Undo)
    }

    /// Applies the last undone edit again. `None` when there is none.
    pub fn redo(&self) -> Option<ProjectPatch> {
        self.prepare_history(HistoryMove::Redo)
    }

    /// Undoes or redoes until `cursor` history entries are applied.
    pub fn history_jump(&self, cursor: u32) -> ProjectPatch {
        self.prepare_history(HistoryMove::Jump(cursor))
            .unwrap_or_else(|| self.state().document.unchanged_patch())
    }

    /// Builds the patch for a change that was just made, hands the changed
    /// project to the engine and sends the patch to every window.
    pub(super) fn publish(&self, state: &mut State, touched: &Touched) -> ProjectPatch {
        self.publish_with_prepared(state, touched, None)
    }

    pub(super) fn publish_prepared(
        &self,
        state: &mut State,
        touched: &Touched,
        prepared: windfall_engine::PreparedProject,
    ) -> ProjectPatch {
        self.publish_with_prepared(state, touched, Some(prepared))
    }

    fn publish_with_prepared(
        &self,
        state: &mut State,
        touched: &Touched,
        prepared: Option<windfall_engine::PreparedProject>,
    ) -> ProjectPatch {
        let patch = state.document.patch(touched);
        if touched.samples {
            self.sync_samples(state);
        }
        if !touched.is_empty() {
            if let Some(manager) = &*crate::sync::lock(&self.inner.plugins) {
                manager.runtime.commit_parameters(state.document.project());
            }
            state.edits += 1;
            if touched.channels {
                self.controller().panic_hardware();
                if state
                    .midi_target
                    .is_some_and(|id| state.document.project().channel(id).is_none())
                {
                    state.midi_target = None;
                }
            }
            if let Some(prepared) = prepared {
                state
                    .pool
                    .install_sampler_preparation(prepared.sampler_pool());
                self.controller()
                    .set_prepared_project(state.document.project(), prepared);
                self.sync_transport();
            } else {
                self.push_project(state);
            }
        }
        self.emit(Event::ProjectPatch(patch.clone()));
        patch
    }

    /// Makes the engine play the project as it is now. The engine moves the
    /// transport off a pattern that no longer exists, and the UI is told of
    /// that before it hears of the edit that removed the pattern.
    pub(super) fn push_project(&self, state: &State) {
        if state
            .pool
            .needs_sampler_preparation(state.document.project())
        {
            self.queue_sampler_preparation();
            return;
        }
        state
            .pool
            .prune_sampler_preparation(state.document.project());
        if let Some(pool) = state.pool.cached_clip_pool(state.document.project()) {
            self.controller()
                .set_project(state.document.project(), &pool);
            self.sync_transport();
        } else {
            self.queue_clip_preparation();
        }
    }
}

#[derive(Clone, Copy)]
enum HistoryMove {
    Undo,
    Redo,
    Jump(u32),
}
impl HistoryMove {
    fn apply(self, document: &mut windfall_project::Document) -> Option<Touched> {
        match self {
            Self::Undo => document.undo(),
            Self::Redo => document.redo(),
            Self::Jump(cursor) => Some(document.jump(cursor)),
        }
    }
}
impl Session {
    // History can refer to a variant evicted from the bounded cache. Compile
    // its target snapshot before reacquiring the session lock as well.
    fn prepare_history(&self, action: HistoryMove) -> Option<ProjectPatch> {
        drop(self.recording_idle().ok()?);
        for _ in 0..8 {
            let (mut document, mut pool, directory, generation, edits, replacements) = {
                let state = self.state();
                (
                    state.document.clone(),
                    state.pool.clone(),
                    state.sample_dir.clone(),
                    state.generation,
                    state.edits,
                    state.replacements,
                )
            };
            action.apply(&mut document)?;
            for asset in &document.project().samples {
                if !pool.contains(asset.id)
                    && let Ok(path) =
                        super::samples::locate(asset, directory.as_deref(), &self.inner.factory_dir)
                    && let Some(audio) = self.inner.cache.peek(&path)
                {
                    pool.insert(asset.id, audio);
                }
            }
            let prepared =
                match windfall_engine::Controller::prepare_project(document.project(), &pool) {
                    Ok(prepared) => prepared,
                    Err(error) => {
                        self.emit(Event::ProjectWarnings(vec![error.to_string()]));
                        return None;
                    }
                };
            #[cfg(test)]
            self.pause("sampler:history-prepared");
            let _recording = self.recording_idle().ok()?;
            let mut state = self.state();
            if state.generation != generation || state.replacements != replacements {
                return None;
            }
            if state.edits != edits {
                continue;
            }
            // Existing sources must also still be the ones compiled.
            if state.pool.iter().any(|(id, audio)| {
                pool.get(id)
                    .is_none_or(|old| old.samples().as_ptr() != audio.samples().as_ptr())
            }) {
                continue;
            }
            let touched = action.apply(&mut state.document)?;
            return Some(self.publish_prepared(&mut state, &touched, prepared));
        }
        self.emit(Event::ProjectWarnings(vec![
            "Sampler history preparation was superseded by repeated edits. Try again.".into(),
        ]));
        None
    }
}
