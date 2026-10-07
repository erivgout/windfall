//! Edits to the document, and telling everyone about them.

use windfall_project::{
    AutomationId, AutomationTarget, ClipContent, ClipInit, Command, DispatchResult,
    DocumentSnapshot, MAX_SONG_TICKS, PlaylistTrackId, ProjectPatch, Touched,
};

use super::{Session, State};
use crate::events::Event;

impl Session {
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
        let mut state = self.state();
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
        let mut state = self.state();
        let touched = state.document.undo()?;
        Some(self.publish(&mut state, &touched))
    }

    /// Applies the last undone edit again. `None` when there is none.
    pub fn redo(&self) -> Option<ProjectPatch> {
        let mut state = self.state();
        let touched = state.document.redo()?;
        Some(self.publish(&mut state, &touched))
    }

    /// Undoes or redoes until `cursor` history entries are applied.
    pub fn history_jump(&self, cursor: u32) -> ProjectPatch {
        let mut state = self.state();
        let touched = state.document.jump(cursor);
        self.publish(&mut state, &touched)
    }

    /// Builds the patch for a change that was just made, hands the changed
    /// project to the engine and sends the patch to every window.
    pub(super) fn publish(&self, state: &mut State, touched: &Touched) -> ProjectPatch {
        let patch = state.document.patch(touched);
        if touched.samples {
            self.sync_samples(state);
        }
        if !touched.is_empty() {
            state.edits += 1;
            self.push_project(state);
        }
        self.emit(Event::ProjectPatch(patch.clone()));
        patch
    }

    /// Makes the engine play the project as it is now. The engine moves the
    /// transport off a pattern that no longer exists, and the UI is told of
    /// that before it hears of the edit that removed the pattern.
    pub(super) fn push_project(&self, state: &State) {
        self.controller()
            .set_project(state.document.project(), &state.pool);
        self.sync_transport();
    }
}
