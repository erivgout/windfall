//! Edits to the document, and telling everyone about them.

use windfall_project::{Command, DispatchResult, DocumentSnapshot, ProjectPatch, Touched};

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
