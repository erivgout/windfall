//! Runs one plugin through its whole life and prints how each stage went.
//!
//! ```text
//! windfall-plugin-check <plugin file> <plugin id>
//! ```
//!
//! It loads the file, creates the plugin, activates it, processes two
//! thirds of a second of silence (with one note for an instrument), reads
//! every parameter, saves and restores the state, and deactivates. No sound
//! is played: the audio only ever goes into buffers.
//!
//! The output is one JSON object per line. `windfall_plugin_host::check_plugin`
//! runs this program and turns the lines into a report.

use std::path::PathBuf;
use std::process::ExitCode;

use windfall_plugin_host::check_in_process;
use windfall_plugin_host::scan::probe;

fn main() -> ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    let (Some(path), Some(id)) = (arguments.next(), arguments.next()) else {
        eprintln!("usage: windfall-plugin-check <plugin file> <plugin id>");
        return ExitCode::from(2);
    };
    probe::silence_error_dialogs();
    check_in_process(
        &PathBuf::from(path),
        &id.to_string_lossy(),
        &mut std::io::stdout().lock(),
    );
    ExitCode::SUCCESS
}
