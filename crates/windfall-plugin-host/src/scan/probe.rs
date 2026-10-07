//! What the scanner program does with the one file it is given.
//!
//! This loads the plugin into the current process, so nothing but the
//! `windfall-plugin-scan` program should call it.

use std::io::Write;
use std::path::Path;

use super::ScanLine;
use super::format_of;
use crate::host::PluginHost;

/// Prepares this process for loading untrusted plugins: the system shows no
/// dialog when one of them crashes or lacks a library it needs. A dialog
/// would leave the scanner waiting for a click until its time ran out.
pub fn silence_error_dialogs() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Diagnostics::Debug::{
            SEM_FAILCRITICALERRORS, SEM_NOGPFAULTERRORBOX, SEM_NOOPENFILEERRORBOX, SetErrorMode,
        };
        // SAFETY: the call changes a setting of this process and nothing
        // else.
        unsafe {
            SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX | SEM_NOOPENFILEERRORBOX);
        }
    }
}

/// Scans `path` in this process and prints one [`ScanLine`] per step to
/// `out`. Plugins whose id is in `skip` are listed but not created.
pub fn run(path: &Path, skip: &[String], out: &mut dyn Write) {
    let mut emit = |line: ScanLine| {
        // The line starts on a fresh line of its own, whatever a plugin
        // printed before it, and is out of the process before the next
        // step can crash.
        let _ = writeln!(out, "\n{}", line.to_json());
        let _ = out.flush();
    };

    let format = match format_of(path) {
        Ok(format) => format,
        Err(message) => {
            emit(ScanLine::Failed { message });
            emit(ScanLine::Done);
            return;
        }
    };
    let host = PluginHost::windfall();
    let module = match host.load(path) {
        Ok(module) => module,
        Err(error) => {
            emit(ScanLine::Failed {
                message: error.to_string(),
            });
            emit(ScanLine::Done);
            return;
        }
    };

    let plugins = module.descriptors();
    emit(ScanLine::File {
        format,
        plugins: plugins.clone(),
    });
    for plugin in plugins {
        if skip.contains(&plugin.id) {
            continue;
        }
        emit(ScanLine::Probing {
            id: plugin.id.clone(),
        });
        match module.probe(&plugin.id) {
            Ok(layout) => emit(ScanLine::Probed {
                id: plugin.id,
                layout,
            }),
            Err(error) => emit(ScanLine::ProbeFailed {
                id: plugin.id,
                message: error.to_string(),
            }),
        }
    }
    emit(ScanLine::Done);
}
