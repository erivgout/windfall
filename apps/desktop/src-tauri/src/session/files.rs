//! New, open, save and backups.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use windfall_ipc::{PlayMode, TransportPatch};
use windfall_project::file::{self, DEFAULT_BACKUP_COUNT, FILE_EXTENSION};
use windfall_project::{Document, DocumentSnapshot, Project, SamplePath, Touched};

use super::samples::{Decoded, decode_all};
use super::{Session, State};
use crate::events::Event;
use crate::paths;
use crate::template::default_project;

/// What saving says when the project has never been given a file.
pub const NO_FILE_YET: &str = "This project has no file yet. Use Save as.";

impl Session {
    /// Replaces the open project with the default one.
    pub fn project_new(&self) -> DocumentSnapshot {
        let project = default_project();
        let decoded = decode_all(&self.inner.cache, &project, None, &self.inner.factory_dir);
        self.install(project, None, decoded)
    }

    /// Opens the project file at `path`. Samples that are missing or cannot
    /// be read leave their channels silent and are reported as warnings.
    ///
    /// Slow: it reads the file and decodes every sample before the project
    /// is swapped in, so playback of the old project carries on until then.
    pub fn project_open(&self, path: &str) -> Result<DocumentSnapshot, String> {
        let file = paths::absolute(path)?;
        let project = file::load(&file).map_err(sentence)?;
        let decoded = decode_all(
            &self.inner.cache,
            &project,
            file.parent(),
            &self.inner.factory_dir,
        );
        let snapshot = self.install(project, Some(file.clone()), decoded);
        self.store().remember_project(&file);
        Ok(snapshot)
    }

    /// Saves the project to `path`, or to the file it already has. Returns
    /// the path saved to, which gains the `.windfall` extension if it lacks
    /// it.
    ///
    /// The project is written as it was when this was called. If it was
    /// edited while the file was being written, it stays marked as having
    /// unsaved changes.
    pub fn project_save(&self, path: Option<&str>) -> Result<String, String> {
        let (project, target, previous_dir, revision, generation) = {
            let state = self.state();
            let target = match path {
                Some(path) => with_project_extension(paths::absolute(path)?),
                None => state.path.clone().ok_or(NO_FILE_YET)?,
            };
            (
                state.document.project().clone(),
                target,
                state.project_dir().map(Path::to_path_buf),
                state.document.revision(),
                state.generation,
            )
        };

        let warnings = match (&previous_dir, target.parent()) {
            (Some(from), Some(to)) if !paths::same(from, to) => {
                copy_project_samples(&project, from, to)
            }
            _ => Vec::new(),
        };
        file::save(&project, &target).map_err(sentence)?;

        {
            let mut state = self.state();
            // A project opened while this one was being written is not the
            // one that was saved.
            if state.generation == generation {
                state.path = Some(target.clone());
                if state.document.revision() == revision {
                    state.document.mark_saved();
                }
                // Changes nothing in the project. It carries the new dirty
                // flag to every window.
                self.publish(&mut state, &Touched::default());
            }
        }
        if !warnings.is_empty() {
            self.emit(Event::ProjectWarnings(warnings));
        }
        self.store().remember_project(&target);
        Ok(paths::display(&target))
    }

    /// The recent projects that still exist, most recent first.
    pub fn recent_projects(&self) -> Vec<String> {
        self.store().recent_projects()
    }

    /// Writes a backup beside the project file if the project has one and
    /// has unsaved changes. Returns the backup's path, or `None` when there
    /// was nothing to back up. `timestamp` goes into the file name; see
    /// [`file::write_backup`] for what it may hold.
    pub fn write_backup(&self, timestamp: &str) -> Result<Option<PathBuf>, String> {
        let (project, path) = {
            let state = self.state();
            match &state.path {
                Some(path) if state.document.is_dirty() => {
                    (state.document.project().clone(), path.clone())
                }
                _ => return Ok(None),
            }
        };
        file::write_backup(&project, &path, timestamp, DEFAULT_BACKUP_COUNT)
            .map(Some)
            .map_err(sentence)
    }

    /// Swaps in a project that is ready to play: stops playback, replaces
    /// the document and the pool, resets the transport and tells the UI.
    fn install(
        &self,
        project: Project,
        path: Option<PathBuf>,
        decoded: Decoded,
    ) -> DocumentSnapshot {
        let mut state = self.state();
        let controller = self.controller();
        controller.stop();

        let first_pattern = project.patterns.first().map(|pattern| pattern.id);
        *state = State {
            document: Document::new(project),
            path,
            pool: decoded.pool,
            loaded: decoded.loaded,
            loading: HashSet::new(),
            failed: decoded.failed,
            generation: state.generation + 1,
        };
        controller.set_project(state.document.project(), &state.pool);
        controller.set_transport(TransportPatch {
            mode: Some(PlayMode::Pattern),
            pattern: first_pattern,
            loop_song: None,
        });
        controller.seek(0.0);

        let snapshot = state.document.snapshot(state.path_text());
        self.emit(Event::ProjectLoaded(snapshot.clone()));
        self.announce_transport();
        if !decoded.warnings.is_empty() {
            self.emit(Event::ProjectWarnings(decoded.warnings));
        }
        snapshot
    }
}

/// Starts a message from one of the project crate's errors, which begin in
/// lower case, with a capital.
fn sentence(error: impl std::fmt::Display) -> String {
    let text = error.to_string();
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => text,
    }
}

/// Adds `.windfall` to a file name that does not end in it.
fn with_project_extension(path: PathBuf) -> PathBuf {
    let has_it = path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(FILE_EXTENSION));
    if has_it {
        return path;
    }
    let mut name = path.into_os_string();
    name.push(".");
    name.push(FILE_EXTENSION);
    PathBuf::from(name)
}

/// Copies the samples a project keeps in its own folder to the folder it is
/// being saved into, so "Save as" to another folder does not leave the new
/// file pointing at nothing. Files already there are left alone. Returns a
/// warning for each sample that could not be copied.
fn copy_project_samples(project: &Project, from: &Path, to: &Path) -> Vec<String> {
    let mut warnings = Vec::new();
    for sample in &project.samples {
        if !matches!(sample.path, SamplePath::Project(_)) {
            continue;
        }
        let (Some(source), Some(target)) = (
            file::resolve_sample_path(&sample.path, Some(from), from),
            file::resolve_sample_path(&sample.path, Some(to), to),
        ) else {
            continue;
        };
        if target.exists() || !source.is_file() {
            continue;
        }
        let copied = target
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| fs::copy(&source, &target));
        if let Err(error) = copied {
            warnings.push(format!(
                "Could not copy the sample \"{}\" to {}: {error}",
                sample.name,
                paths::display(&target)
            ));
        }
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_project_extension_is_added_only_when_missing() {
        let named = |name: &str| with_project_extension(PathBuf::from(name));
        assert_eq!(named("beat"), PathBuf::from("beat.windfall"));
        assert_eq!(named("beat.windfall"), PathBuf::from("beat.windfall"));
        assert_eq!(named("Beat.WINDFALL"), PathBuf::from("Beat.WINDFALL"));
        // A dot in the name is part of the name, not an extension to replace.
        assert_eq!(named("take.2"), PathBuf::from("take.2.windfall"));
    }

    #[test]
    fn messages_start_with_a_capital() {
        assert_eq!(sentence("could not read x"), "Could not read x");
        assert_eq!(sentence(""), "");
    }
}
