//! The browser panel, and bringing audio files into the project.

use std::path::{Path, PathBuf};

use windfall_core::AudioBuffer;
use windfall_ipc::{BrowserEntry, BrowserRoot, SampleInfo};
use windfall_project::file::sample_path_for;
use windfall_project::{ChannelId, Command, DispatchResult, SampleId};

use super::samples::locate;
use super::{Session, State};
use crate::browser;
use crate::paths;

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
    pub fn sample_info_by_id(&self, sample: SampleId) -> Result<SampleInfo, String> {
        let file = {
            let state = self.state();
            let asset = state
                .document
                .project()
                .sample(sample)
                .ok_or_else(|| format!("sample {} does not exist", sample.0))?;
            locate(asset, state.project_dir(), &self.inner.factory_dir)?
        };
        self.inner.cache.info(&file)
    }

    /// Adds a channel that plays the audio file at `path`, as one undo
    /// step. The channel is named after the file and goes at `index` in the
    /// rack, or at the end. Creates the sample, the channel and the
    /// channel's mixer track, in that order; a file the project already
    /// uses is not added again, and its sample is the one reported.
    ///
    /// Slow the first time a file is asked for. A file that cannot be
    /// decoded is refused, and the project is not touched.
    pub fn add_channel_from_file(
        &self,
        path: &str,
        index: Option<u32>,
    ) -> Result<DispatchResult, String> {
        let (file, buffer) = self.decode_for_project(path)?;
        let name = paths::stem(&file);
        self.dispatch_with_sample(&file, buffer, "Add channel", |sample| Command::AddChannel {
            name: Some(name.clone()),
            sample: Some(sample),
            index,
            mixer_track: None,
        })
    }

    /// Makes a channel play the audio file at `path`, as one undo step.
    /// Creates the sample, or reports the one the project already has for
    /// that file.
    pub fn set_channel_sample_from_file(
        &self,
        channel: ChannelId,
        path: &str,
    ) -> Result<DispatchResult, String> {
        let (file, buffer) = self.decode_for_project(path)?;
        self.dispatch_with_sample(&file, buffer, "Change channel sample", |sample| {
            Command::SetChannelSample {
                id: channel,
                sample: Some(sample),
            }
        })
    }

    fn roots(&self, user: &[String]) -> Vec<BrowserRoot> {
        std::iter::once(browser::factory_root(&self.inner.factory_dir))
            .chain(user.iter().map(|root| browser::user_root(Path::new(root))))
            .collect()
    }

    /// Decodes a file before the project is changed to use it.
    fn decode_for_project(&self, path: &str) -> Result<(PathBuf, AudioBuffer), String> {
        let file = paths::absolute(path)?;
        let buffer = self.inner.cache.decode(&file)?;
        Ok((file, buffer))
    }

    /// Dispatches one undo step that adds the sample for `file` and then
    /// runs the command `then` builds from the sample's id.
    fn dispatch_with_sample(
        &self,
        file: &Path,
        buffer: AudioBuffer,
        label: &str,
        then: impl FnOnce(SampleId) -> Command,
    ) -> Result<DispatchResult, String> {
        let mut state = self.state();
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

/// Puts audio that is already decoded into the pool, so the edit that needs
/// it plays at once instead of waiting for the file to be read again.
fn hold_sample(state: &mut State, sample: SampleId, buffer: AudioBuffer) {
    if state.document.project().sample(sample).is_some() && state.loaded.insert(sample) {
        state.pool.insert(sample, buffer);
        state.failed.remove(&sample);
    }
}
