//! The Windfall project model, edit commands, undo history and file format.
//!
//! One [`Document`] holds the single copy of a project that the UI and the
//! audio engine both follow. Every edit is a [`Command`] dispatched to it.

pub mod automation;
mod check;
pub mod command;
pub mod document;
mod edit;
pub mod error;
pub mod file;
mod lower;
pub mod model;
pub mod patch;
pub mod piano_tools;
pub mod plugin;
pub mod slicer;
pub use plugin::*;

pub use automation::{AutomationRange, AutomationTaper, curve_shape, curve_value};
pub use command::*;
pub use document::{Applied, Document};
pub use error::CommandError;
pub use file::{LoadError, ProjectSession, SaveError};
pub use model::*;
pub use patch::*;
pub use piano_tools::{NoteEdge, NoteGroove, NoteTransform};
