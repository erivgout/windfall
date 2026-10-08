//! The list of scanned plugins that the app keeps between runs.
//!
//! Scanning a file costs a process start and the plugin's own start-up, so
//! each result is kept with the file's size and date and reused until one
//! of them changes. That goes for failures too. A plugin that crashed the
//! scanner is not loaded again at the next start, which is what a blocklist
//! is. It gets another try when its file changes, or when the user asks.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::{FileScan, ScanFailure, ScanRunner, ScannedPlugin, ScannerUnavailable, scan_file};
use crate::descriptor::PluginFormat;
use crate::paths::{PluginFile, PluginFileIdentity, plugin_file_identity};

/// Raised when the stored shape changes. A file with another number is
/// thrown away and everything is scanned again.
const VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    path: String,
    identity: PluginFileIdentity,
    scan: FileScan,
}

#[derive(Serialize, Deserialize)]
struct Stored {
    version: u32,
    files: Vec<Entry>,
}

/// A plugin, or a whole file, that crashed or hung the scanner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockedPlugin {
    pub path: PathBuf,
    /// The plugin's id and name, or `None` when the file never got as far
    /// as listing its plugins.
    pub plugin: Option<(String, String)>,
    pub failure: ScanFailure,
}

/// What [`PluginCatalog::refresh`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RefreshSummary {
    /// Files that were scanned because they were new or had changed.
    pub scanned: usize,
    /// Files whose stored result was still good.
    pub reused: usize,
    /// Files that were in the catalog and are gone from disk.
    pub removed: usize,
}

/// Every plugin file the app has scanned, with what the scan found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PluginCatalog {
    files: BTreeMap<String, Entry>,
}

fn key(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// The size and date that tell whether a plugin file has changed.
fn stamp(path: &Path) -> Option<PluginFileIdentity> {
    plugin_file_identity(path).ok()
}

impl PluginCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads a catalog. A file that is missing, damaged or from another
    /// version gives an empty catalog, which only costs a full scan.
    pub fn load(file: &Path) -> Self {
        let stored = std::fs::read(file)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Stored>(&bytes).ok())
            .filter(|stored| stored.version == VERSION);
        let files = stored
            .map(|stored| stored.files)
            .unwrap_or_default()
            .into_iter()
            .map(|entry| (entry.path.clone(), entry))
            .collect();
        Self { files }
    }

    /// Writes the catalog so that a crash cannot leave half a file: to a
    /// temporary file beside it, which is then renamed into place.
    pub fn save(&self, file: &Path) -> io::Result<()> {
        let stored = Stored {
            version: VERSION,
            files: self.files.values().cloned().collect(),
        };
        let json = serde_json::to_vec_pretty(&stored).map_err(io::Error::other)?;
        write_atomic(file, &json)
    }

    /// Brings the catalog in line with `files`, which is what
    /// [`find_plugins`](crate::paths::find_plugins) returned. New and
    /// changed files are scanned with `runner`, files that are gone are
    /// dropped, and everything else is left as it was. `progress` hears of
    /// each file before it is scanned.
    ///
    /// Stops at the first file for which the scanner cannot be started at
    /// all, leaving what was done so far in place.
    pub fn refresh(
        &mut self,
        files: &[PluginFile],
        runner: &dyn ScanRunner,
        progress: &mut dyn FnMut(&Path),
    ) -> Result<RefreshSummary, ScannerUnavailable> {
        let mut summary = RefreshSummary::default();
        let before = self.files.len();
        let present: Vec<String> = files.iter().map(|file| key(&file.path)).collect();
        self.files.retain(|path, _| present.contains(path));
        summary.removed = before - self.files.len();

        for file in files {
            let Some(identity) = stamp(&file.path) else {
                if self.files.remove(&key(&file.path)).is_some() {
                    summary.removed += 1;
                }
                continue;
            };
            let path = key(&file.path);
            let fresh = self
                .files
                .get(&path)
                .is_some_and(|entry| entry.identity == identity);
            if fresh {
                summary.reused += 1;
                continue;
            }
            progress(&file.path);
            let scan = scan_file(runner, &file.path)?;
            self.files.insert(
                path.clone(),
                Entry {
                    path,
                    identity,
                    scan,
                },
            );
            summary.scanned += 1;
        }
        Ok(summary)
    }

    /// Every scanned file with its result, sorted by path.
    pub fn files(&self) -> impl Iterator<Item = (&Path, &FileScan)> {
        self.files
            .values()
            .map(|entry| (Path::new(&entry.path), &entry.scan))
    }
    /// The binary identity verified by this scan, rather than new metadata
    /// read after scanning. Loading must still compare it with the current file.
    pub fn identity(&self, path: &Path) -> Option<&PluginFileIdentity> {
        self.files.get(&key(path)).map(|entry| &entry.identity)
    }

    /// The plugins that can be used, with the file each is in.
    pub fn plugins(&self) -> impl Iterator<Item = (&Path, &ScannedPlugin)> {
        self.files().flat_map(|(path, scan)| {
            scan.plugins
                .iter()
                .filter(|plugin| plugin.is_usable())
                .map(move |plugin| (path, plugin))
        })
    }

    /// The usable plugin with this format and id, and its file. If two
    /// files hold the same plugin, the one whose path sorts first wins.
    pub fn find(&self, format: PluginFormat, id: &str) -> Option<(&Path, &ScannedPlugin)> {
        self.plugins()
            .find(|(_, plugin)| plugin.descriptor.format == format && plugin.descriptor.id == id)
    }

    /// The blocklist: every plugin and file that crashed or hung the
    /// scanner. The app loads none of these.
    pub fn blocked(&self) -> Vec<BlockedPlugin> {
        let mut blocked = Vec::new();
        for (path, scan) in self.files() {
            if let Some(failure) = scan
                .failure
                .as_ref()
                .filter(|failure| failure.is_dangerous())
            {
                blocked.push(BlockedPlugin {
                    path: path.to_path_buf(),
                    plugin: None,
                    failure: failure.clone(),
                });
            }
            for plugin in &scan.plugins {
                let Some(failure) = plugin.failure.as_ref() else {
                    continue;
                };
                if failure.is_dangerous() {
                    let descriptor = &plugin.descriptor;
                    blocked.push(BlockedPlugin {
                        path: path.to_path_buf(),
                        plugin: Some((descriptor.id.clone(), descriptor.name.clone())),
                        failure: failure.clone(),
                    });
                }
            }
        }
        blocked
    }

    /// Forgets what is known about one file, so the next
    /// [`refresh`](Self::refresh) scans it again. This is how a blocked
    /// plugin gets another try. Returns false if the file was not known.
    pub fn retry(&mut self, path: &Path) -> bool {
        self.files.remove(&key(path)).is_some()
    }

    /// Forgets every file with something on the blocklist. Returns how many.
    pub fn retry_blocked(&mut self) -> usize {
        let paths: Vec<PathBuf> = self.blocked().into_iter().map(|entry| entry.path).collect();
        paths.iter().filter(|path| self.retry(path)).count()
    }
}

/// Numbers this process's temporary files, so two writers never share one.
static SERIAL: AtomicU32 = AtomicU32::new(0);

fn write_atomic(file: &Path, bytes: &[u8]) -> io::Result<()> {
    let folder = file
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    if let Some(folder) = folder {
        std::fs::create_dir_all(folder)?;
    }
    let name = file
        .file_name()
        .ok_or_else(|| io::Error::other("the catalog path has no file name"))?
        .to_string_lossy();
    let serial = SERIAL.fetch_add(1, Ordering::Relaxed);
    let temporary = file.with_file_name(format!(".{name}.{}-{serial}.tmp", std::process::id()));
    std::fs::write(&temporary, bytes)?;

    // Windows refuses to replace a file another program holds open, as a
    // virus scanner does for a moment.
    let mut attempt = 0;
    loop {
        match std::fs::rename(&temporary, file) {
            Ok(()) => return Ok(()),
            Err(error) if attempt >= 5 => {
                let _ = std::fs::remove_file(&temporary);
                return Err(error);
            }
            Err(_) => {
                attempt += 1;
                std::thread::sleep(Duration::from_millis(20 * attempt));
            }
        }
    }
}
