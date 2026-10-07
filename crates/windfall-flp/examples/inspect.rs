//! Read-only verification tool. Input files are never copied into the project.
use windfall_flp::{ConvertOptions, convert, parse};
fn main() {
    for path in std::env::args().skip(1) {
        let bytes = match std::fs::read(&path) { Ok(bytes) => bytes, Err(e) => { println!("{path}: {e}"); continue; } };
        match parse(&bytes) {
            Err(e) => println!("{path}: parse error: {e}"),
            Ok(flp) => {
                let conversion = convert(&flp, &ConvertOptions::default());
                let notes: usize = flp.patterns.iter().map(|p| p.notes.len()).sum();
                let items: usize = flp.arrangements.iter().map(|a| a.items.len()).sum();
                println!("{path} | {} | channels={} patterns={} notes={} inserts={} items={} unknown={:?} diagnostics={} valid={:?}", flp.version_text.as_deref().unwrap_or("unknown"), flp.channels.len(), flp.patterns.len(), notes, flp.mixer.inserts.len(), items, flp.unknown_ids(), flp.diagnostics.len(), conversion.project.check());
                for c in &conversion.report.categories { if c.total() != 0 { println!("  {}: exact={} approximated={} placeholders={} dropped={}", c.title, c.exact, c.approximated, c.placeholders, c.dropped); } }
                println!("  result: channels={} patterns={} notes={} clips={} automations={} placeholders={}", conversion.project.channels.len(), conversion.project.patterns.len(), conversion.project.patterns.iter().flat_map(|p| &p.lanes).map(|l| l.notes.len()).sum::<usize>(), conversion.project.playlist.clips.len(), conversion.project.automations.len(), conversion.plugins.len());
            }
        }
    }
}
