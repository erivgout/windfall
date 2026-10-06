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
            path,
            pool,
            loaded,
            loading,
            failed,
            generation,
        } = state;
        let project = document.project();
        let project_dir = path.as_deref().and_then(Path::parent);

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
            .spawn(move || session.load_samples(generation, pending));
        if let Err(error) = spawned {
            log::error!("could not start a thread to load samples: {error}");
            for id in ids {
                loading.remove(&id);
            }
        }
    }

    /// Decodes files for the document numbered `generation` and hands the
    /// audio to the engine, unless that document has been replaced.
    fn load_samples(&self, generation: u64, pending: Vec<(SampleId, PathBuf)>) {
        let results: Vec<(SampleId, Result<AudioBuffer, String>)> = pending
            .into_iter()
            .map(|(id, file)| (id, decode(&self.inner.cache, &file)))
            .collect();

        let mut state = self.state();
        if state.generation != generation {
            return;
        }
        let mut warnings = Vec::new();
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
        if changed {
            self.controller()
                .set_project(state.document.project(), &state.pool);
        }
        if !warnings.is_empty() {
            self.emit(Event::ProjectWarnings(warnings));
        }
    }
}
