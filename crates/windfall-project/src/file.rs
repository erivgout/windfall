//! The `.windfall` file format: one pretty-printed JSON file holding a
//! [`Project`], next to the samples it owns and a `Backup` folder.

use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

use crate::model::{FORMAT_VERSION, Project, SamplePath, relative_path_problem};

/// File extension of a project, without the dot.
pub const FILE_EXTENSION: &str = "windfall";

/// Name of the folder, beside the project file, that holds its backups.
pub const BACKUP_FOLDER: &str = "Backup";

/// How many backups of a project [`write_backup`] usually keeps.
pub const DEFAULT_BACKUP_COUNT: usize = 20;

/// Why a project could not be loaded.
#[derive(Debug, Error)]
pub enum LoadError {
    #[error("could not read {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("this is not a Windfall project: {0}")]
    Parse(#[from] serde_json::Error),
    #[error(
        "this project was saved by a newer version of Windfall (file format {found}; this version reads up to format {supported})"
    )]
    TooNew { found: u32, supported: u32 },
    #[error("the project file is damaged: {0}")]
    Invalid(String),
}

/// Why a project could not be saved.
#[derive(Debug, Error)]
pub enum SaveError {
    #[error("could not write {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("could not encode the project: {0}")]
    Encode(#[from] serde_json::Error),
    #[error("{0}")]
    BadName(String),
}

/// Encodes a project as the text of a `.windfall` file.
pub fn to_json(project: &Project) -> Result<String, serde_json::Error> {
    let mut json = serde_json::to_string_pretty(project)?;
    json.push('\n');
    Ok(json)
}

/// Decodes the text of a `.windfall` file. Files in an older format are
/// upgraded, files in a newer format are refused, and the result is checked
/// with [`Project::check`].
pub fn from_json(json: &str) -> Result<Project, LoadError> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Header {
        format_version: u32,
    }

    let found = serde_json::from_str::<Header>(json)?.format_version;
    if found > FORMAT_VERSION {
        return Err(LoadError::TooNew {
            found,
            supported: FORMAT_VERSION,
        });
    }
    let project: Project = if found == FORMAT_VERSION {
        serde_json::from_str(json)?
    } else {
        let mut value: Value = serde_json::from_str(json)?;
        migrate(&mut value, found, MIGRATIONS)?;
        serde_json::from_value(value)?
    };
    project.check().map_err(LoadError::Invalid)?;
    Ok(project)
}

/// Writes a project to `path`, replacing any file there. The text goes to a
/// temporary file in the same folder first and is then renamed into place,
/// so a crash cannot leave half a file. Missing folders are created.
pub fn save(project: &Project, path: impl AsRef<Path>) -> Result<(), SaveError> {
    let path = path.as_ref();
    let json = to_json(project)?;
    write_atomic(path, json.as_bytes()).map_err(|source| SaveError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Reads the project at `path`. See [`from_json`] for what is accepted.
pub fn load(path: impl AsRef<Path>) -> Result<Project, LoadError> {
    let path = path.as_ref();
    let json = fs::read_to_string(path).map_err(|source| LoadError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    from_json(&json)
}

/// Writes a backup of the project to `Backup/<name> <timestamp>.windfall`
/// beside the project file at `project_path`, and returns the backup's path.
///
/// Afterwards only the newest `keep` backups of this project remain;
/// [`DEFAULT_BACKUP_COUNT`] is the usual number, and the backup just written
/// is always kept. "Newest" goes by file name, so every timestamp passed for
/// one project must have the same length and sort in time order, as
/// `2026-10-06 18-13-05` does. The timestamp becomes part of a file name and
/// so cannot contain characters such as `:` or `/`.
pub fn write_backup(
    project: &Project,
    project_path: impl AsRef<Path>,
    timestamp: &str,
    keep: usize,
) -> Result<PathBuf, SaveError> {
    let project_path = project_path.as_ref();
    let name = project_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| {
            SaveError::BadName(format!(
                "{} does not name a project file",
                project_path.display()
            ))
        })?;
    let forbidden = |c: char| c.is_control() || r#"/\:*?"<>|"#.contains(c);
    if timestamp.is_empty() || timestamp.contains(forbidden) {
        return Err(SaveError::BadName(format!(
            "\"{timestamp}\" cannot be used as the timestamp in a backup's file name"
        )));
    }

    let folder = project_path.with_file_name(BACKUP_FOLDER);
    let prefix = format!("{name} ");
    let suffix = format!(".{FILE_EXTENSION}");
    let backup = folder.join(format!("{prefix}{timestamp}{suffix}"));
    save(project, &backup)?;
    prune_backups(&folder, &prefix, &suffix, timestamp.len(), keep.max(1));
    Ok(backup)
}

/// Deletes all but the `keep` newest backups whose names are `prefix`, a
/// timestamp of `timestamp_len` bytes, then `suffix`.
fn prune_backups(folder: &Path, prefix: &str, suffix: &str, timestamp_len: usize, keep: usize) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    // The length test tells the backups of "Song" from those of "Song 2",
    // which share the folder and the prefix.
    let mut names: Vec<String> = entries
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| {
            name.len() == prefix.len() + timestamp_len + suffix.len()
                && name.starts_with(prefix)
                && name.ends_with(suffix)
        })
        .collect();
    names.sort_unstable_by(|a, b| b.cmp(a));
    for name in names.iter().skip(keep) {
        // A backup that cannot be deleted costs disk space only. It must not
        // turn a backup that was written into a failure.
        let _ = fs::remove_file(folder.join(name));
    }
}

fn write_atomic(path: &Path, contents: &[u8]) -> io::Result<()> {
    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "the path has no file name"))?;
    if let Some(folder) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(folder)?;
    }
    let mut temp_name = OsString::from(".");
    temp_name.push(file_name);
    temp_name.push(".tmp");
    let temp = path.with_file_name(temp_name);

    let written = (|| {
        let mut file = File::create(&temp)?;
        file.write_all(contents)?;
        // Without this the rename can reach the disk before the contents do.
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)
    })();
    if written.is_err() {
        let _ = fs::remove_file(&temp);
    }
    written
}

/// Upgrades the JSON of a project by one format version.
type Migration = fn(&mut Value) -> Result<(), String>;

/// `MIGRATIONS[n]` upgrades a file of format version `n + 1` to version
/// `n + 2`. A migration changes the JSON in place and leaves `formatVersion`
/// alone; [`migrate`] sets it.
const MIGRATIONS: &[Migration] = &[];

// Every bump of FORMAT_VERSION must come with the migration that reaches it.
const _: () = assert!(MIGRATIONS.len() + 1 == FORMAT_VERSION as usize);

/// Runs the migrations that bring a file from format version `from` up to
/// the version after the last migration.
fn migrate(value: &mut Value, from: u32, migrations: &[Migration]) -> Result<(), LoadError> {
    if from == 0 {
        return Err(LoadError::Invalid(
            "format version 0 does not exist".to_owned(),
        ));
    }
    for (index, migration) in migrations.iter().enumerate().skip(from as usize - 1) {
        let version = index + 1;
        migration(value).map_err(|problem| {
            LoadError::Invalid(format!(
                "could not upgrade from format {version} to {}: {problem}",
                version + 1
            ))
        })?;
        if let Some(object) = value.as_object_mut() {
            object.insert("formatVersion".to_owned(), Value::from(version + 1));
        }
    }
    Ok(())
}

/// Where on disk a sample's audio file is. `project_dir` is the folder that
/// holds the project file, or `None` for a project that has never been
/// saved. Returns `None` for a project sample of an unsaved project, and for
/// a stored relative path that is malformed or would leave its folder.
pub fn resolve_sample_path(
    sample: &SamplePath,
    project_dir: Option<&Path>,
    factory_dir: &Path,
) -> Option<PathBuf> {
    match sample {
        SamplePath::Factory(relative) => join_relative(factory_dir, relative),
        SamplePath::Project(relative) => join_relative(project_dir?, relative),
        SamplePath::External(absolute) => Some(PathBuf::from(absolute)),
    }
}

fn join_relative(folder: &Path, relative: &str) -> Option<PathBuf> {
    if relative_path_problem(relative).is_some() {
        return None;
    }
    let mut path = folder.to_path_buf();
    for part in relative.split('/') {
        // On Windows a part such as "C:" would replace the path built so far.
        let mut components = Path::new(part).components();
        match (components.next(), components.next()) {
            (Some(Component::Normal(_)), None) => path.push(part),
            _ => return None,
        }
    }
    Some(path)
}

/// The way to store the audio file at `file` in a project: relative to the
/// project folder when it is inside it, else relative to the factory content
/// folder when it is inside that, else as it is.
///
/// Paths are compared part by part without touching the disk, so `file` and
/// the folders must be written the same way: all absolute, and all with or
/// all without Windows' `\\?\` prefix. Letter case is ignored on Windows.
pub fn sample_path_for(file: &Path, project_dir: Option<&Path>, factory_dir: &Path) -> SamplePath {
    if let Some(relative) = project_dir.and_then(|folder| relative_to(file, folder)) {
        return SamplePath::Project(relative);
    }
    if let Some(relative) = relative_to(file, factory_dir) {
        return SamplePath::Factory(relative);
    }
    SamplePath::External(file.to_string_lossy().into_owned())
}

/// The path of `file` inside `folder`, with forward slashes, or `None` when
/// it is not inside or cannot be stored as a relative path.
fn relative_to(file: &Path, folder: &Path) -> Option<String> {
    let mut parts = file.components();
    for folder_part in folder.components() {
        if !same_part(parts.next()?, folder_part) {
            return None;
        }
    }
    let rest: Vec<&str> = parts
        .map(|part| match part {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect::<Option<_>>()?;
    let relative = rest.join("/");
    relative_path_problem(&relative)
        .is_none()
        .then_some(relative)
}

fn same_part(a: Component, b: Component) -> bool {
    if cfg!(windows) {
        a.as_os_str().eq_ignore_ascii_case(b.as_os_str())
    } else {
        a == b
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn rename_title(value: &mut Value) -> Result<(), String> {
        let object = value.as_object_mut().ok_or("not an object")?;
        let title = object.remove("title").ok_or("no title")?;
        object.insert("name".to_owned(), title);
        Ok(())
    }

    fn add_tempo(value: &mut Value) -> Result<(), String> {
        let object = value.as_object_mut().ok_or("not an object")?;
        object.insert("tempo".to_owned(), json!(120));
        Ok(())
    }

    #[test]
    fn migrations_run_in_order_from_the_file_version() {
        let mut value = json!({ "formatVersion": 1, "title": "Song" });
        migrate(&mut value, 1, &[rename_title, add_tempo]).unwrap();
        assert_eq!(
            value,
            json!({ "formatVersion": 3, "name": "Song", "tempo": 120 })
        );
    }

    #[test]
    fn migrations_before_the_file_version_are_skipped() {
        let mut value = json!({ "formatVersion": 2, "name": "Song" });
        migrate(&mut value, 2, &[rename_title, add_tempo]).unwrap();
        assert_eq!(
            value,
            json!({ "formatVersion": 3, "name": "Song", "tempo": 120 })
        );
    }

    #[test]
    fn a_file_at_the_newest_version_is_left_alone() {
        let mut value = json!({ "formatVersion": 3, "name": "Song" });
        migrate(&mut value, 3, &[rename_title, add_tempo]).unwrap();
        assert_eq!(value, json!({ "formatVersion": 3, "name": "Song" }));
    }

    #[test]
    fn a_failed_migration_names_the_versions() {
        let mut value = json!({ "formatVersion": 1 });
        let error = migrate(&mut value, 1, &[rename_title]).unwrap_err();
        assert_eq!(
            error.to_string(),
            "the project file is damaged: could not upgrade from format 1 to 2: no title"
        );
    }

    #[test]
    fn version_zero_is_refused() {
        let mut value = json!({ "formatVersion": 0 });
        assert!(matches!(
            migrate(&mut value, 0, &[]),
            Err(LoadError::Invalid(_))
        ));
    }
}
