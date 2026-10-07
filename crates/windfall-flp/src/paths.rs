//! Where a sample is, from what FL Studio wrote.
//!
//! FL Studio writes a sample's path in one of four ways:
//!
//! - a full path, `C:\Samples\Kick.wav`;
//! - a path that starts with a variable for one of its own folders,
//!   `%FLStudioFactoryData%\Data\Patches\Packs\Drums\Kick.wav`;
//! - in files from before those variables, a path that starts with a
//!   backslash and is meant from the `Data` folder of the installation;
//! - a bare relative path, meant from the folder of the project.
//!
//! This module works on text alone and never touches a disk, so it runs
//! the same on every platform and in WebAssembly. Whether the file is
//! there is for the shell to find out.
//!
//! Sources: PyFLP's documentation of `Sampler.sample_path` and DawVert
//! `plugins/input/mi_flp.py` (the variables), LMMS `FlpImport.cpp` (paths
//! from the `Data` folder).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// How the paths of the computer doing the import are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum PathStyle {
    /// Backslashes and drive letters.
    #[default]
    Windows,
    /// Forward slashes from a single root.
    Posix,
}

impl PathStyle {
    /// The style of the platform this was compiled for.
    pub fn host() -> Self {
        if cfg!(windows) {
            PathStyle::Windows
        } else {
            PathStyle::Posix
        }
    }

    fn separator(self) -> char {
        match self {
            PathStyle::Windows => '\\',
            PathStyle::Posix => '/',
        }
    }
}

/// The folders a path of an FL Studio project can be meant from.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Folders<'a> {
    pub(crate) project: Option<&'a str>,
    /// The folder FL Studio is installed in.
    pub(crate) factory: Option<&'a str>,
    pub(crate) user_data: Option<&'a str>,
    pub(crate) user_profile: Option<&'a str>,
    pub(crate) style: PathStyle,
}

/// A path worked out from what the project says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Resolved {
    /// The full path, or the path as FL Studio wrote it when it could not
    /// be made full.
    pub(crate) path: String,
    /// False when the path depends on a folder that was not given.
    pub(crate) resolved: bool,
}

/// Where a path that starts with one of FL Studio's variables is meant
/// from.
enum Base {
    Factory,
    /// The `Data` folder of the installation.
    FactoryData,
    UserData,
    UserProfile,
}

const VARIABLES: [(&str, Base); 4] = [
    ("%FLStudioFactoryData%", Base::Factory),
    ("%FLStudioUserData%", Base::UserData),
    ("%FLStudioData%", Base::FactoryData),
    ("%USERPROFILE%", Base::UserProfile),
];

/// Makes the path of a sample full and tidy.
pub(crate) fn resolve(written: &str, folders: Folders<'_>) -> Resolved {
    let written = written.trim();
    let unresolved = || Resolved {
        path: written.to_owned(),
        resolved: false,
    };
    let join = |base: Option<&str>, middle: &[&str], rest: &str| match base {
        Some(base) if !base.trim().is_empty() => {
            let mut parts = vec![base.trim()];
            parts.extend(middle);
            parts.push(rest);
            Resolved {
                path: normalize(&parts.join("/"), folders.style),
                resolved: true,
            }
        }
        _ => unresolved(),
    };

    for (variable, base) in VARIABLES {
        let Some(head) = written.get(..variable.len()) else {
            continue;
        };
        if !head.eq_ignore_ascii_case(variable) {
            continue;
        }
        let rest = &written[variable.len()..];
        return match base {
            Base::Factory => join(folders.factory, &[], rest),
            Base::FactoryData => join(folders.factory, &["Data"], rest),
            Base::UserData => join(folders.user_data, &[], rest),
            Base::UserProfile => join(folders.user_profile, &[], rest),
        };
    }
    if written.starts_with('%') {
        // A variable this crate does not know.
        return unresolved();
    }
    if is_absolute(written) {
        return Resolved {
            path: normalize(written, folders.style),
            resolved: true,
        };
    }
    if written.starts_with(['\\', '/']) {
        return join(folders.factory, &["Data"], written);
    }
    join(folders.project, &[], written)
}

/// True for a path with a drive letter, a network share, or, when it uses
/// no backslash, a root.
fn is_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    let drive = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/');
    let share = path.starts_with("\\\\") || path.starts_with("//");
    // A leading slash is the root on Linux and macOS. With a backslash
    // anywhere the path is a Windows one, where a single leading
    // separator is a path from FL Studio's Data folder.
    let rooted = path.starts_with('/') && !path.contains('\\');
    drive || share || rooted
}

/// Rewrites a path with one kind of separator and without `.`, `..` or
/// empty parts. A `..` that would climb above the root is dropped.
fn normalize(path: &str, style: PathStyle) -> String {
    let separator = style.separator();
    let share = path.starts_with("\\\\") || path.starts_with("//");
    let rooted = !share && path.starts_with(['\\', '/']);
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split(['\\', '/']) {
        match part {
            "" | "." => {}
            ".." => {
                // The first part of a share or a drive is not a folder to
                // climb out of.
                let floor = usize::from(share || parts.first().is_some_and(|p| p.ends_with(':')));
                if parts.len() > floor {
                    parts.pop();
                }
            }
            part => parts.push(part),
        }
    }
    let mut tidy = String::new();
    if share {
        tidy.push(separator);
        tidy.push(separator);
    } else if rooted {
        tidy.push(separator);
    }
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            tidy.push(separator);
        }
        tidy.push_str(part);
    }
    // "C:" alone is not a folder; "C:\" is.
    if parts.len() == 1 && parts[0].ends_with(':') {
        tidy.push(separator);
    }
    tidy
}

/// The name of a file without its folders and its extension, for naming a
/// sample.
pub(crate) fn file_stem(path: &str) -> &str {
    let name = path.rsplit(['\\', '/']).next().unwrap_or(path);
    match name.rfind('.') {
        Some(dot) if dot > 0 => &name[..dot],
        _ => name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOWS: Folders<'static> = Folders {
        project: Some("D:\\Songs\\Night Drive"),
        factory: Some("C:\\Program Files\\FL"),
        user_data: Some("C:\\Users\\ada\\Documents\\FL"),
        user_profile: Some("C:\\Users\\ada"),
        style: PathStyle::Windows,
    };

    fn full(written: &str, folders: Folders<'_>) -> String {
        let resolved = resolve(written, folders);
        assert!(resolved.resolved, "{written} should resolve");
        resolved.path
    }

    #[test]
    fn a_full_path_is_tidied_and_kept() {
        assert_eq!(
            full("C:\\Samples\\Kick.wav", WINDOWS),
            "C:\\Samples\\Kick.wav"
        );
        assert_eq!(
            full("C:/Samples//Drums/./Loud/../Kick.wav", WINDOWS),
            "C:\\Samples\\Drums\\Kick.wav"
        );
        assert_eq!(
            full("\\\\nas\\audio\\Kick.wav", WINDOWS),
            "\\\\nas\\audio\\Kick.wav"
        );
        assert_eq!(full("C:\\", WINDOWS), "C:\\");
    }

    #[test]
    fn a_full_path_needs_no_folders() {
        assert_eq!(
            resolve("C:\\Samples\\Kick.wav", Folders::default()),
            Resolved {
                path: "C:\\Samples\\Kick.wav".to_owned(),
                resolved: true
            }
        );
    }

    #[test]
    fn variables_stand_for_the_folders_that_were_given() {
        assert_eq!(
            full("%FLStudioFactoryData%\\Data\\Patches\\Kick.wav", WINDOWS),
            "C:\\Program Files\\FL\\Data\\Patches\\Kick.wav"
        );
        assert_eq!(
            full("%flstudiofactorydata%/Data/Kick.wav", WINDOWS),
            "C:\\Program Files\\FL\\Data\\Kick.wav"
        );
        assert_eq!(
            full("%FLStudioData%\\Patches\\Kick.wav", WINDOWS),
            "C:\\Program Files\\FL\\Data\\Patches\\Kick.wav"
        );
        assert_eq!(
            full("%FLStudioUserData%\\Audio\\Take 1.wav", WINDOWS),
            "C:\\Users\\ada\\Documents\\FL\\Audio\\Take 1.wav"
        );
        assert_eq!(
            full("%USERPROFILE%\\Music\\Loop.wav", WINDOWS),
            "C:\\Users\\ada\\Music\\Loop.wav"
        );
    }

    #[test]
    fn a_variable_without_its_folder_stays_as_written() {
        let written = "%FLStudioFactoryData%\\Data\\Patches\\Kick.wav";
        assert_eq!(
            resolve(written, Folders::default()),
            Resolved {
                path: written.to_owned(),
                resolved: false
            }
        );
        let blank = Folders {
            factory: Some("  "),
            ..Folders::default()
        };
        assert!(!resolve(written, blank).resolved);
        assert!(!resolve("%SomethingNew%\\Kick.wav", WINDOWS).resolved);
    }

    #[test]
    fn a_path_from_the_data_folder_of_an_old_file_resolves_there() {
        assert_eq!(
            full("\\Projects\\Templates\\Basic\\Kick.wav", WINDOWS),
            "C:\\Program Files\\FL\\Data\\Projects\\Templates\\Basic\\Kick.wav"
        );
        assert!(!resolve("\\Projects\\Kick.wav", Folders::default()).resolved);
    }

    #[test]
    fn a_relative_path_is_meant_from_the_project() {
        assert_eq!(
            full("Samples\\Kick.wav", WINDOWS),
            "D:\\Songs\\Night Drive\\Samples\\Kick.wav"
        );
        assert_eq!(
            full("..\\Shared\\Kick.wav", WINDOWS),
            "D:\\Songs\\Shared\\Kick.wav"
        );
        let alone = resolve("Samples\\Kick.wav", Folders::default());
        assert_eq!(
            alone,
            Resolved {
                path: "Samples\\Kick.wav".to_owned(),
                resolved: false
            }
        );
    }

    #[test]
    fn climbing_stops_at_the_root() {
        assert_eq!(full("C:\\..\\..\\Kick.wav", WINDOWS), "C:\\Kick.wav");
        assert_eq!(
            full("\\\\nas\\..\\..\\Kick.wav", WINDOWS),
            "\\\\nas\\Kick.wav"
        );
    }

    #[test]
    fn posix_paths_come_out_with_forward_slashes() {
        let posix = Folders {
            project: Some("/home/ada/songs"),
            factory: Some("/opt/fl"),
            user_data: None,
            user_profile: Some("/home/ada"),
            style: PathStyle::Posix,
        };
        assert_eq!(full("/samples/kick.wav", posix), "/samples/kick.wav");
        assert_eq!(
            full("samples\\kick.wav", posix),
            "/home/ada/songs/samples/kick.wav"
        );
        assert_eq!(
            full("%FLStudioFactoryData%\\Data\\Kick.wav", posix),
            "/opt/fl/Data/Kick.wav"
        );
        assert_eq!(full("%USERPROFILE%\\Loop.wav", posix), "/home/ada/Loop.wav");
        assert_eq!(full("/../kick.wav", posix), "/kick.wav");
    }

    #[test]
    fn the_name_of_a_sample_is_its_file_without_the_extension() {
        assert_eq!(file_stem("C:\\Samples\\Kick 01.wav"), "Kick 01");
        assert_eq!(file_stem("/samples/snare.tight.flac"), "snare.tight");
        assert_eq!(file_stem("%FLStudioFactoryData%\\Data\\Hat"), "Hat");
        assert_eq!(file_stem(".hidden"), ".hidden");
        assert_eq!(file_stem(""), "");
    }
}
