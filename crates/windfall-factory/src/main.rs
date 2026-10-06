//! Command line for the factory sounds.
//!
//! `windfall-factory generate <out-dir>` writes every sound.
//! `windfall-factory verify <dir>` checks a folder against the generator.

use std::path::Path;
use std::process::ExitCode;

const USAGE: &str =
    "usage: windfall-factory generate <out-dir>\n       windfall-factory verify <dir>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (command, dir) = match args.as_slice() {
        [command, dir] => (command.as_str(), Path::new(dir)),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match command {
        "generate" => generate(dir),
        "verify" => verify(dir),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn generate(dir: &Path) -> ExitCode {
    if let Err(error) = windfall_factory::generate(dir) {
        eprintln!("could not write {}: {error}", dir.display());
        return ExitCode::FAILURE;
    }
    let sounds = windfall_factory::manifest().len();
    println!("wrote {sounds} sounds to {}", dir.display());
    ExitCode::SUCCESS
}

fn verify(dir: &Path) -> ExitCode {
    let mismatches = match windfall_factory::verify(dir) {
        Ok(mismatches) => mismatches,
        Err(error) => {
            eprintln!("could not read {}: {error}", dir.display());
            return ExitCode::FAILURE;
        }
    };
    if mismatches.is_empty() {
        println!("{} matches the generator", dir.display());
        return ExitCode::SUCCESS;
    }
    for mismatch in &mismatches {
        eprintln!("{mismatch}");
    }
    eprintln!(
        "{} differs from the generator in {} files",
        dir.display(),
        mismatches.len()
    );
    ExitCode::FAILURE
}
