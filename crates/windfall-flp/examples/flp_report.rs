//! Reads FL Studio files and prints what was found in each, one block per
//! file. `docs/flp/coverage.md` is made from its output.
//!
//! `cargo run -p windfall-flp --example flp_report -- <file>...`

use std::process::ExitCode;

use windfall_flp::{FlpProject, PlaylistSource, parse};

fn main() -> ExitCode {
    let files: Vec<String> = std::env::args().skip(1).collect();
    if files.is_empty() {
        eprintln!("usage: flp_report <file>...");
        return ExitCode::FAILURE;
    }
    for path in files {
        let name = std::path::Path::new(&path)
            .file_name()
            .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
        println!("== {name}");
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                println!("cannot be read: {error}");
                continue;
            }
        };
        match parse(&bytes) {
            Ok(project) => summarize(&project),
            Err(error) => println!("parse: FAILED: {error}"),
        }
    }
    ExitCode::SUCCESS
}

fn summarize(project: &FlpProject) {
    let header = project.header;
    println!(
        "parse: ok, {} at {} ticks per quarter note, FL Studio {}",
        header.map_or("a file", |header| header.format.describe()),
        project.ppq(),
        project.version_text.as_deref().unwrap_or("(no version)")
    );
    let notes: usize = project.patterns.iter().map(|p| p.notes.len()).sum();
    let slots: usize = project.mixer.inserts.iter().map(|i| i.slots.len()).sum();
    let arrangement = project.main_arrangement();
    let items = arrangement.map_or(0, |a| a.items.len());
    let pattern_items = arrangement.map_or(0, |a| {
        a.items
            .iter()
            .filter(|item| matches!(item.source, PlaylistSource::Pattern { .. }))
            .count()
    });
    println!(
        "counts: {} channels, {} patterns, {notes} notes, {} inserts, {slots} effects, {} arrangements, {items} playlist items ({pattern_items} pattern clips), {} controller links",
        project.channels.len(),
        project.patterns.len(),
        project.mixer.inserts.len(),
        project.arrangements.len(),
        project.remote_controllers.len(),
    );
    let unknown = project.unknown_ids();
    if unknown.is_empty() {
        println!("unknown event ids: none");
    } else {
        let list: Vec<String> = unknown
            .iter()
            .map(|id| format!("{id} (x{})", project.uninterpreted_counts[id]))
            .collect();
        println!("unknown event ids: {}", list.join(", "));
    }
    for diagnostic in &project.diagnostics {
        println!("diagnostic: {}", diagnostic.message);
    }
}
