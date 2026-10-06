//! The Windfall project model, edit commands, undo history and file format.
//!
//! One [`Document`] holds the single copy of a project that the UI and the
//! audio engine both follow. Every edit is a [`Command`] dispatched to it.

mod check;
pub mod command;
pub mod document;
mod edit;
pub mod error;
pub mod file;
mod lower;
pub mod model;
pub mod patch;

pub use command::*;
pub use document::{Applied, Document};
pub use error::CommandError;
pub use file::{LoadError, SaveError};
pub use model::*;
pub use patch::*;
