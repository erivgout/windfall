// Keeps a console window from opening next to the app on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if windfall_desktop_lib::plugins::scanner_entry() {
        return;
    }
    windfall_desktop_lib::run();
}
