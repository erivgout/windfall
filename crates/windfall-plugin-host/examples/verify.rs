//! Scan and check a supplied folder in supervised helper processes.
use std::path::{Path, PathBuf};
use std::time::Duration;
use windfall_plugin_host::{ProcessRunner, check_plugin, paths, scan_file};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let folders: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    if folders.is_empty() {
        return Err("usage: verify <plugin folder>...".into());
    }
    let executable = std::env::current_exe()?;
    let binaries = executable
        .parent()
        .and_then(Path::parent)
        .ok_or("missing binary directory")?;
    let mut runner = ProcessRunner::new(binaries.join(format!(
        "windfall-plugin-scan{}",
        std::env::consts::EXE_SUFFIX
    )));
    runner.timeout = Duration::from_secs(20);
    let check = binaries.join(format!(
        "windfall-plugin-check{}",
        std::env::consts::EXE_SUFFIX
    ));
    let mut reports = Vec::new();
    for file in paths::find_plugins(&folders) {
        let scan = scan_file(&runner, &file.path)?;
        let mut checks = Vec::new();
        for plugin in scan.plugins.iter().filter(|plugin| plugin.is_usable()) {
            let report = check_plugin(
                &check,
                &file.path,
                &plugin.descriptor.id,
                Duration::from_secs(20),
            )?;
            checks.push(
                serde_json::json!({"id": plugin.descriptor.id, "allPassed": report.all_passed(),
                "steps": report.steps.iter().map(|(step, outcome)| serde_json::json!({
                    "step": step, "outcome": format!("{outcome:?}")
                })).collect::<Vec<_>>() }),
            );
        }
        reports.push(serde_json::json!({ "path": file.path, "scan": scan, "checks": checks }));
    }
    println!("{}", serde_json::to_string_pretty(&reports)?);
    Ok(())
}
