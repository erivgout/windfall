//! Prints the events of FL Studio files, one per line, for studying files.
//!
//! `cargo run -p windfall-flp --example flp_dump -- [--from N] [--to N] [--ids a,b] <file>`

use std::process::ExitCode;

use windfall_flp::{EventValue, open};

fn main() -> ExitCode {
    let mut from = 0_usize;
    let mut to = usize::MAX;
    let mut ids: Option<Vec<u8>> = None;
    let mut files = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--from" => from = args.next().and_then(|v| v.parse().ok()).unwrap_or(0),
            "--to" => {
                to = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(usize::MAX)
            }
            "--ids" => {
                ids = args
                    .next()
                    .map(|list| list.split(',').filter_map(|id| id.parse().ok()).collect());
            }
            _ => files.push(arg),
        }
    }
    for path in files {
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                eprintln!("{path}: {error}");
                return ExitCode::FAILURE;
            }
        };
        let container = match open(&bytes) {
            Ok(container) => container,
            Err(error) => {
                println!("{path}: {error}");
                continue;
            }
        };
        println!("{path}: {:?}", container.header);
        for (index, event) in container.events.enumerate() {
            if index < from || index > to {
                continue;
            }
            match event {
                Ok(event) => {
                    if ids.as_ref().is_some_and(|ids| !ids.contains(&event.id)) {
                        continue;
                    }
                    match event.value {
                        EventValue::Data(data) => {
                            let shown: Vec<String> = data
                                .iter()
                                .take(64)
                                .map(|byte| format!("{byte:02x}"))
                                .collect();
                            let text: String = data
                                .iter()
                                .take(64)
                                .filter(|&&byte| byte != 0)
                                .map(|&byte| {
                                    if byte.is_ascii_graphic() || byte == b' ' {
                                        byte as char
                                    } else {
                                        '.'
                                    }
                                })
                                .collect();
                            println!(
                                "{index:6} {:3} len {:6} {} |{text}|",
                                event.id,
                                data.len(),
                                shown.join("")
                            );
                        }
                        other => {
                            let number = other.number().unwrap_or(0);
                            println!("{index:6} {:3} {number} ({number:#x})", event.id);
                        }
                    }
                }
                Err(error) => println!("{index:6} error: {error}"),
            }
        }
    }
    ExitCode::SUCCESS
}
