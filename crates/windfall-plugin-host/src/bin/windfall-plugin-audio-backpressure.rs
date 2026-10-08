//! Debug test bootstrap: actual helper owner with a controlled paused Load read.
//! Release packaging/discovery must never select this executable.
fn main() {
    #[cfg(all(windows, debug_assertions))]
    if windfall_plugin_host::bridge::helper::entry() {
        return;
    }
    std::process::exit(64);
}
