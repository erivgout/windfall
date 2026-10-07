//! Keeping the engine's sample pool in line with the project.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use windfall_core::AudioBuffer;
use windfall_engine::SamplePool;
use windfall_project::file::resolve_sample_path;
use windfall_project::{Project, SampleAsset, SampleId, SamplePath};

use super::{Session, State};
use crate::events::Event;
use crate::paths;
use crate::samples::SampleCache;

/// The decoded samples of a project, and what could not be decoded.
pub(super) struct Decoded {
    pub pool: SamplePool,
    pub loaded: HashSet<SampleId>,
    pub failed: HashSet<SampleId>,
    /// One sentence per sample that is missing or unreadable.
    pub warnings: Vec<String>,
}

/// Decodes every sample of a project. Slow: call it with no lock held.
pub(super) fn decode_all(
    cache: &SampleCache,
    project: &Project,
    project_dir: Option<&Path>,
    factory_dir: &Path,
) -> Decoded {
    let mut decoded = Decoded {
        pool: SamplePool::new(),
        loaded: HashSet::new(),
        failed: HashSet::new(),
        warnings: Vec::new(),
    };
    if !project.retained_plugins.is_empty() {
        decoded.warnings.push(format!("{} imported plugin states are retained; unsupported instruments stay silent and effects bypassed. Use the source FL project with a compatible host to recover those sounds.", project.retained_plugins.len()));
    }
    for sample in &project.samples {
        let buffer = locate(sample, project_dir, factory_dir).and_then(|file| decode(cache, &file));
        match buffer {
            Ok(buffer) => {
                decoded.pool.insert(sample.id, buffer);
                decoded.loaded.insert(sample.id);
            }
            Err(warning) => {
                decoded.failed.insert(sample.id);
                decoded.warnings.push(warning);
            }
        }
    }
    decoded
}

/// The file a sample's audio is in, or the warning to show when its stored
/// path leads nowhere.
pub(super) fn locate(
    sample: &SampleAsset,
    project_dir: Option<&Path>,
    factory_dir: &Path,
) -> Result<PathBuf, String> {
    resolve_sample_path(&sample.path, project_dir, factory_dir)
        .map(|file| paths::clean(&file))
        .ok_or_else(|| {
            let (SamplePath::Factory(stored)
            | SamplePath::Project(stored)
            | SamplePath::External(stored)) = &sample.path;
            format!("Missing sample: {stored}")
        })
}

/// Decodes a sample's file, with the warning to show when that fails.
fn decode(cache: &SampleCache, file: &Path) -> Result<AudioBuffer, String> {
    cache.decode(file).map_err(|error| {
        if file.exists() {
            error
        } else {
            format!("Missing sample: {}", paths::display(file))
        }
    })
}

impl Session {
    /// Brings the pool in line with the project after an edit that touched
    /// its samples: drops what is gone and loads what is new.
    ///
    /// Runs under the session lock, so it only takes audio that is already
    /// decoded. Files not seen before are decoded on another thread, and
    /// their channels are silent until that is done.
    pub(super) fn sync_samples(&self, state: &mut State) {
        let State {
            document,
            sample_dir,
            pool,
            loaded,
            loading,
            failed,
            generation,
            ..
        } = state;
        let project = document.project();
        let project_dir = sample_dir.as_deref();

        let wanted: HashSet<SampleId> = project.samples.iter().map(|sample| sample.id).collect();
        loaded.retain(|id| {
            let keep = wanted.contains(id);
            if !keep {
                pool.remove(*id);
            }
            keep
        });
        failed.retain(|id| wanted.contains(id));

        let mut warnings = Vec::new();
        let mut pending = Vec::new();
        for sample in &project.samples {
            let id = sample.id;
            if loaded.contains(&id) || loading.contains(&id) || failed.contains(&id) {
                continue;
            }
            match locate(sample, project_dir, &self.inner.factory_dir) {
                Err(warning) => {
                    failed.insert(id);
                    warnings.push(warning);
                }
                Ok(file) => match self.inner.cache.peek(&file) {
                    Some(buffer) => {
                        pool.insert(id, buffer);
                        loaded.insert(id);
                    }
                    None => {
                        loading.insert(id);
                        pending.push((id, file));
                    }
                },
            }
        }

        if !warnings.is_empty() {
            self.emit(Event::ProjectWarnings(warnings));
        }
        if pending.is_empty() {
            return;
        }
        let session = self.clone();
        let generation = *generation;
        let ids: Vec<SampleId> = pending.iter().map(|(id, _)| *id).collect();
        let spawned = std::thread::Builder::new()
            .name("windfall-sample-loader".to_owned())
            .spawn(move || {
                session.load_samples(generation, pending, Vec::new(), false);
            });
        if let Err(error) = spawned {
            log::error!("could not start a thread to load samples: {error}");
            for id in ids {
                loading.remove(&id);
            }
        }
    }

    /// Tries again to load every sample that has no audio because its file
    /// was missing or could not be read: looks each file up afresh, decodes
    /// what is there now and hands the project to the engine again. What
    /// is still missing is reported as warnings, in one event. Returns how
    /// many samples of the open project are still without audio.
    ///
    /// Slow when files have turned up: they are decoded on the caller's
    /// thread, with no lock held.
    pub fn samples_reload(&self) -> u32 {
        let (generation, pending, warnings) = {
            let mut state = self.state();
            let State {
                document,
                sample_dir,
                loading,
                failed,
                generation,
                ..
            } = &mut *state;
            let mut pending = Vec::new();
            let mut warnings = Vec::new();
            for sample in &document.project().samples {
                if !failed.contains(&sample.id) {
                    continue;
                }
                match locate(sample, sample_dir.as_deref(), &self.inner.factory_dir) {
                    Ok(file) => {
                        // Marked as loading, so an edit made meanwhile
                        // does not start a second attempt.
                        failed.remove(&sample.id);
                        loading.insert(sample.id);
                        pending.push((sample.id, file));
                    }
                    Err(warning) => warnings.push(warning),
                }
            }
            (*generation, pending, warnings)
        };
        self.load_samples(generation, pending, warnings, true)
    }

    /// Decodes files for the document numbered `generation` and hands the
    /// audio to the engine, unless that document has been replaced.
    ///
    /// `warnings` are sent along with those of the files that fail here.
    /// The engine is handed the project when a sample got its audio, or
    /// whatever happened if `push` is set. Returns how many samples of the
    /// open project are without audio afterwards.
    fn load_samples(
        &self,
        generation: u64,
        pending: Vec<(SampleId, PathBuf)>,
        mut warnings: Vec<String>,
        push: bool,
    ) -> u32 {
        let results: Vec<(SampleId, Result<AudioBuffer, String>)> = pending
            .into_iter()
            .map(|(id, file)| (id, decode(&self.inner.cache, &file)))
            .collect();
        #[cfg(test)]
        self.pause("samples:decoded");

        let mut state = self.state();
        let missing = |state: &State| u32::try_from(state.failed.len()).unwrap_or(u32::MAX);
        if state.generation != generation {
            return missing(&state);
        }
        let mut changed = false;
        for (id, result) in results {
            state.loading.remove(&id);
            // The edit that added the sample may have been undone meanwhile.
            if state.document.project().sample(id).is_none() {
                continue;
            }
            match result {
                Ok(buffer) => {
                    state.pool.insert(id, buffer);
                    state.loaded.insert(id);
                    changed = true;
                }
                Err(warning) => {
                    state.failed.insert(id);
                    warnings.push(warning);
                }
            }
        }
        if changed || push {
            self.push_project(&state);
        }
        if !warnings.is_empty() {
            self.emit(Event::ProjectWarnings(warnings));
        }
        missing(&state)
    }
}
