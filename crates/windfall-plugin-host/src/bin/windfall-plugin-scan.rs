//! Loads one plugin file and prints what is in it.
//!
//! ```text
//! windfall-plugin-scan <plugin file> [--skip <plugin id>]...
//! ```
//!
//! The app runs this once per plugin file instead of loading the file
//! itself, so that a plugin that crashes or hangs while loading ends this
//! process and not the app. The output is one JSON object per line,
//! described in `windfall_plugin_host::scan::ScanLine`.

use std::path::PathBuf;
use std::process::ExitCode;

use windfall_plugin_host::scan::probe;

fn main() -> ExitCode {
    let mut path = None;
    let mut skip = Vec::new();
    let mut arguments = std::env::args_os().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--skip" {
            if let Some(id) = arguments.next() {
                skip.push(id.to_string_lossy().into_owned());
            }
        } else if path.is_none() {
            path = Some(PathBuf::from(argument));
        }
    }
    let Some(path) = path else {
        eprintln!("usage: windfall-plugin-scan <plugin file> [--skip <plugin id>]...");
        return ExitCode::from(2);
    };

    probe::silence_error_dialogs();
    probe::run(&path, &skip, &mut std::io::stdout().lock());
    ExitCode::SUCCESS
}
