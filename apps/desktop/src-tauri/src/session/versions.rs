//! Numbered saves share the ordinary save/capture/carry workflow. The final
//! atomic no-clobber publish is the reservation, so even another process cannot
//! win a filename between an existence check and publication.
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use windfall_project::{Project, ProjectSession, file};

const ATTEMPTS: u32 = 10_000;
pub(super) fn write(
    project: &Project,
    session: &ProjectSession,
    base: &Path,
) -> Result<PathBuf, String> {
    project.check()?;
    let json = file::to_json_with(project, Some(session)).map_err(|e| e.to_string())?;
    let parent = base.parent().ok_or("The project has no folder.")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut staged = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    staged
        .write_all(json.as_bytes())
        .map_err(|e| e.to_string())?;
    staged.as_file().sync_all().map_err(|e| e.to_string())?;
    let stem = base
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("The project filename is not Unicode.")?;
    let (stem, first) = split(stem);
    for offset in 0..ATTEMPTS {
        let number = first
            .checked_add(offset)
            .filter(|n| *n <= 999_999)
            .ok_or("The project version number limit was reached.")?;
        let target = parent.join(format!("{stem} ({number:03}).windfall"));
        match staged.persist_noclobber(&target) {
            Ok(_) => return Ok(target),
            Err(error)
                if error.error.kind() == io::ErrorKind::AlreadyExists
                    || fs::symlink_metadata(&target).is_ok() =>
            {
                staged = error.file
            }
            Err(error) => return Err(format!("Could not save the new version: {}", error.error)),
        }
    }
    Err("No unused project version was found within 10,000 filenames.".into())
}
fn split(stem: &str) -> (&str, u32) {
    if let Some((base, suffix)) = stem.rsplit_once(" (")
        && let Some(digits) = suffix.strip_suffix(')')
        && (3..=6).contains(&digits.len())
        && digits.bytes().all(|c| c.is_ascii_digit())
        && let Ok(number) = digits.parse::<u32>()
    {
        return (base, number.saturating_add(1));
    }
    (stem, 1)
}
impl super::Session {
    /// Unsaved projects may supply a base filename from the Save picker.
    pub fn project_save_new_version(&self, path: Option<&str>) -> Result<String, String> {
        self.project_save_mode(path, true)
    }
}
