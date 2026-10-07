//! What the scanner process prints: one JSON object per line.
//!
//! The lines come in the order the work is done, and each is flushed before
//! the next step starts. When the scanner dies, the lines it did print say
//! how far it got and which plugin it was looking at.
//!
//! A plugin can print to the same output. A line that is not one of these
//! objects is ignored.

use serde::{Deserialize, Serialize};

use crate::descriptor::{PluginDescriptor, PluginFormat, PluginLayout};

/// One line of the scanner's output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ScanLine {
    /// The file loaded, and these are the plugins it lists.
    File {
        format: PluginFormat,
        plugins: Vec<PluginDescriptor>,
    },
    /// The file could not be loaded. Nothing follows but [`ScanLine::Done`].
    Failed { message: String },
    /// The plugin with this id is about to be created. If nothing follows,
    /// it is the one that crashed or hung.
    Probing { id: String },
    /// The plugin was created and described itself.
    Probed { id: String, layout: PluginLayout },
    /// The plugin refused to be created or described itself badly.
    ProbeFailed { id: String, message: String },
    /// The scanner finished by itself.
    Done,
}

impl ScanLine {
    /// The line as the scanner prints it, without the line break.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("scan lines hold only strings and numbers")
    }

    /// Reads one line of output. `None` for anything that is not a scan
    /// line, such as something a plugin printed.
    pub fn parse(line: &str) -> Option<Self> {
        let line = line.trim();
        if !line.starts_with('{') {
            return None;
        }
        serde_json::from_str(line).ok()
    }
}
