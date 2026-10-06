//! The document: the one copy of a project, and its undo history.

use crate::command::Command;
use crate::edit::{self, Direction, Edit};
use crate::error::CommandError;
use crate::lower;
use crate::model::Project;
use crate::patch::{DocumentSnapshot, HistoryEntry, HistoryView, ProjectPatch, Touched};

/// What a successful [`Document::dispatch`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct Applied {
    /// Ids the command created, in the order its documentation gives. A
    /// batch lists the ids of its commands one after another.
    pub created: Vec<u32>,
    /// The sections of the project that changed. Empty when the command
    /// changed nothing.
    pub touched: Touched,
    /// The history label of the command, such as "Add channel".
    pub label: String,
}

/// Owns a [`Project`] and the history of edits made to it.
///
/// # Undo and ids
///
/// Undo restores every part of the project exactly, with one exception:
/// `next_id` never goes back. Ids an undone edit handed out stay retired, so
/// an edit made after an undo can never receive an id that an earlier,
/// undone edit used. Anything that keys on an id for the life of the
/// document, such as decoded audio cached by sample id or a selection in the
/// UI, therefore never sees that id come to mean something else.
///
/// Redo does not allocate. It replays the stored edits, which carry the ids
/// they first created, so undo followed by redo brings back the same ids.
///
/// A command that fails does not use up ids: `next_id` is put back along
/// with everything else. A command that succeeds but nets out to no change
/// keeps any ids it allocated retired, because it has already reported them.
///
/// Because of this, `next_id` is not part of what [`is_dirty`](Self::is_dirty)
/// tracks: undoing back to the saved state makes the document clean even
/// though `next_id` is higher than in the file.
#[derive(Debug, Clone)]
pub struct Document {
    project: Project,
    revision: u64,
    entries: Vec<Entry>,
    /// How many entries are applied. Entries from here on have been undone.
    cursor: usize,
    /// The cursor at the last save, or `None` once that point of the history
    /// has been discarded and cannot be reached again.
    saved: Option<usize>,
    /// The gesture that made the last applied entry, while later dispatches
    /// of that gesture may still merge into it.
    open_gesture: Option<u64>,
}

#[derive(Debug, Clone)]
struct Entry {
    label: String,
    edits: Vec<Edit>,
}

impl Document {
    /// Wraps a project with an empty history. The project is taken to be
    /// valid and saved; run [`Project::check`] first on one that comes from
    /// outside.
    pub fn new(project: Project) -> Self {
        Self {
            project,
            revision: 0,
            entries: Vec::new(),
            cursor: 0,
            saved: Some(0),
            open_gesture: None,
        }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    /// The revision of the last patch built by [`patch`](Self::patch). 0
    /// until the first one.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Applies a command and records it for undo.
    ///
    /// Consecutive dispatches that carry the same non-`None` gesture id
    /// collapse into one undo step, as for a fader drag. A different gesture
    /// id, `None`, an undo, a redo, a jump or [`mark_saved`](Self::mark_saved)
    /// in between starts a new step. A gesture that ends up back where it
    /// started leaves no step at all.
    ///
    /// A command that changes nothing succeeds with an empty `touched` and
    /// leaves the history, including anything that can be redone, alone.
    /// Any other successful command discards the edits that had been undone.
    ///
    /// On failure the project, the history and the dirty state are exactly
    /// as they were.
    pub fn dispatch(
        &mut self,
        command: Command,
        gesture: Option<u64>,
    ) -> Result<Applied, CommandError> {
        let outcome = lower::execute(&mut self.project, command)?;
        let mut touched = Touched::default();
        for edit in &outcome.edits {
            edit.mark(Direction::Forward, &mut touched);
        }
        self.drop_removed_patterns(&mut touched);

        if outcome.edits.is_empty() {
            if self.open_gesture != gesture {
                self.open_gesture = None;
            }
        } else if let Some(entry) = self.gesture_entry(gesture) {
            for edit in outcome.edits {
                edit::push(&mut entry.edits, edit);
            }
            if entry.edits.is_empty() {
                self.entries.pop();
                self.cursor -= 1;
                self.open_gesture = None;
            }
        } else {
            // The saved state lies in the part of the history being discarded.
            if self.saved.is_some_and(|saved| saved > self.cursor) {
                self.saved = None;
            }
            self.entries.truncate(self.cursor);
            self.entries.push(Entry {
                label: outcome.label.clone(),
                edits: outcome.edits,
            });
            self.cursor += 1;
            self.open_gesture = gesture;
        }

        Ok(Applied {
            created: outcome.created,
            touched,
            label: outcome.label,
        })
    }

    /// Undoes the last applied edit. Returns `None` when there is none.
    pub fn undo(&mut self) -> Option<Touched> {
        self.open_gesture = None;
        let mut touched = Touched::default();
        if !self.step_back(&mut touched) {
            return None;
        }
        self.drop_removed_patterns(&mut touched);
        Some(touched)
    }

    /// Applies the last undone edit again. Returns `None` when there is none.
    pub fn redo(&mut self) -> Option<Touched> {
        self.open_gesture = None;
        let mut touched = Touched::default();
        if !self.step_forward(&mut touched) {
            return None;
        }
        self.drop_removed_patterns(&mut touched);
        Some(touched)
    }

    /// Undoes or redoes until `cursor` entries are applied. A cursor past the
    /// end of the history means all of it.
    pub fn jump(&mut self, cursor: u32) -> Touched {
        self.open_gesture = None;
        let target = (cursor as usize).min(self.entries.len());
        let mut touched = Touched::default();
        while self.cursor > target && self.step_back(&mut touched) {}
        while self.cursor < target && self.step_forward(&mut touched) {}
        self.drop_removed_patterns(&mut touched);
        touched
    }

    pub fn history(&self) -> HistoryView {
        HistoryView {
            entries: self
                .entries
                .iter()
                .map(|entry| HistoryEntry {
                    label: entry.label.clone(),
                })
                .collect(),
            cursor: self.cursor as u32,
        }
    }

    /// True when the project differs from what was last saved. Undoing back
    /// to the saved state makes the document clean again.
    pub fn is_dirty(&self) -> bool {
        self.saved != Some(self.cursor)
    }

    /// Records that the project as it is now has been saved.
    pub fn mark_saved(&mut self) {
        self.saved = Some(self.cursor);
        // A gesture that carried on into the saved entry would change what
        // "saved" means without the document turning dirty.
        self.open_gesture = None;
    }

    /// Builds the patch for the UI and bumps the revision. Call it once for
    /// every `Touched` that `dispatch`, `undo`, `redo` or `jump` returns.
    pub fn patch(&mut self, touched: &Touched) -> ProjectPatch {
        self.revision += 1;
        let project = &self.project;
        ProjectPatch {
            revision: self.revision,
            settings: touched.settings.then(|| project.settings.clone()),
            samples: touched.samples.then(|| project.samples.clone()),
            channels: touched.channels.then(|| project.channels.clone()),
            mixer: touched.mixer.then(|| project.mixer.clone()),
            playlist: touched.playlist.then(|| project.playlist.clone()),
            pattern_order: touched
                .pattern_list
                .then(|| project.patterns.iter().map(|pattern| pattern.id).collect()),
            patterns: project
                .patterns
                .iter()
                .filter(|pattern| touched.patterns.contains(&pattern.id))
                .cloned()
                .collect(),
            history: self.history(),
            dirty: self.is_dirty(),
        }
    }

    /// The whole document, for a UI that is starting or has missed a patch.
    pub fn snapshot(&self, path: Option<String>) -> DocumentSnapshot {
        DocumentSnapshot {
            revision: self.revision,
            project: self.project.clone(),
            history: self.history(),
            dirty: self.is_dirty(),
            path,
        }
    }

    /// The entry a dispatch with this gesture id merges into, if any.
    fn gesture_entry(&mut self, gesture: Option<u64>) -> Option<&mut Entry> {
        if gesture.is_none() || self.open_gesture != gesture {
            return None;
        }
        // An open gesture always belongs to the last entry: anything that
        // moves the cursor closes it.
        self.entries.last_mut()
    }

    fn step_back(&mut self, touched: &mut Touched) -> bool {
        let Some(entry) = self.cursor.checked_sub(1).and_then(|i| self.entries.get(i)) else {
            return false;
        };
        for edit in entry.edits.iter().rev() {
            edit.apply(&mut self.project, Direction::Backward);
            edit.mark(Direction::Backward, touched);
        }
        self.cursor -= 1;
        true
    }

    fn step_forward(&mut self, touched: &mut Touched) -> bool {
        let Some(entry) = self.entries.get(self.cursor) else {
            return false;
        };
        for edit in &entry.edits {
            edit.apply(&mut self.project, Direction::Forward);
            edit.mark(Direction::Forward, touched);
        }
        self.cursor += 1;
        true
    }

    /// `Touched::patterns` lists patterns whose data changed; one that no
    /// longer exists has no data to send.
    fn drop_removed_patterns(&self, touched: &mut Touched) {
        touched
            .patterns
            .retain(|id| self.project.pattern(*id).is_some());
    }
}
