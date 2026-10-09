//! Owned file/namespace authority, acquired and retired only off State.
use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::path::Path;
#[cfg(windows)]
mod drive;

pub(super) struct SourceFile {
    file: File,
    #[cfg(windows)]
    _parents: Vec<File>,
    #[cfg(windows)]
    _drive: std::os::windows::io::OwnedHandle,
}
impl Read for SourceFile {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.file.read(bytes)
    }
}

pub(super) fn open(path: &Path) -> Result<SourceFile, String> {
    #[cfg(windows)]
    let drive = drive::pin(path)?;
    #[cfg(windows)]
    let parents = pin_parents(path)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(1).custom_flags(0x0020_0000); // OPEN_REPARSE_POINT
    }
    let file = options.open(path).map_err(|e| super::error("io", e))?;
    #[cfg(windows)]
    reject_reparse(&file)?;
    if !file
        .metadata()
        .map_err(|e| super::error("io", e))?
        .is_file()
    {
        return Err(super::error(
            "sourceNamespace",
            "Expected an ordinary audio file.",
        ));
    }
    Ok(SourceFile {
        file,
        #[cfg(windows)]
        _parents: parents,
        #[cfg(windows)]
        _drive: drive,
    })
}

#[cfg(windows)]
fn reject_reparse(file: &File) -> Result<(), String> {
    use std::os::windows::fs::MetadataExt;
    if file
        .metadata()
        .map_err(|e| super::error("io", e))?
        .file_attributes()
        & 0x400
        != 0
    {
        return Err(super::error(
            "sourceNamespace",
            "Analysis refuses reparse points, junctions and symbolic links. Use an ordinary local source path.",
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn pin_parents(path: &Path) -> Result<Vec<File>, String> {
    use std::os::windows::fs::OpenOptionsExt;
    use std::path::{Component, PathBuf, Prefix};
    // A finite root-to-leaf walk. No canonicalize/check-then-open race: each
    // opened component is itself queried without following its reparse target,
    // and its no-write/no-delete handle protects subsequent path traversal.
    if !path.is_absolute() || path.as_os_str().len() > 1024 {
        return Err(super::error(
            "sourceNamespace",
            "Expected a bounded absolute local path.",
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| super::error("sourceNamespace", "Missing source parent."))?;
    if parent.components().count() > 65 {
        // drive prefix plus at most 64 pinned directories
        return Err(super::error(
            "sourceNamespace",
            "Analysis namespace limit is 64 parent directory handles.",
        ));
    }
    let mut handles = Vec::new();
    let mut current = PathBuf::new();
    for component in parent.components() {
        match component {
            Component::Prefix(prefix) => {
                if !matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)) {
                    return Err(super::error(
                        "sourceNamespace",
                        "Analysis requires a local drive namespace.",
                    ));
                }
                current.push(component.as_os_str());
                continue;
            }
            Component::RootDir | Component::Normal(_) => current.push(component.as_os_str()),
            _ => {
                return Err(super::error(
                    "sourceNamespace",
                    "Noncanonical source path components.",
                ));
            }
        }
        let handle = OpenOptions::new()
            .read(true)
            // GENERIC_READ includes directory data/list access. Attributes-only
            // handles do not establish read/write share authority on Windows.
            .share_mode(1) // FILE_SHARE_READ only: deny write/delete/rename/retarget
            .custom_flags(0x0200_0000 | 0x0020_0000) // BACKUP_SEMANTICS | OPEN_REPARSE_POINT
            .open(&current)
            .map_err(|e| super::error("sourceNamespace", e))?;
        reject_reparse(&handle)?;
        if !handle
            .metadata()
            .map_err(|e| super::error("io", e))?
            .is_dir()
        {
            return Err(super::error(
                "sourceNamespace",
                "Source parent is not an ordinary directory.",
            ));
        }
        handles.push(handle);
    }
    Ok(handles)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn analysis_namespace_limits_refuse_unsupported_and_deep_paths_before_io() {
        for path in [
            r"relative.wav",
            r"\\server\share\source.wav",
            r"\\.\C:\source.wav",
        ] {
            assert!(
                open(Path::new(path))
                    .err()
                    .unwrap()
                    .contains("analysis:sourceNamespace")
            );
        }
        let mut deep = std::path::PathBuf::from(r"C:\");
        for _ in 0..64 {
            deep.push("directory");
        }
        deep.push("source.wav");
        assert!(
            open(&deep)
                .err()
                .unwrap()
                .contains("64 parent directory handles")
        );
    }
}
