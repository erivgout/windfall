//! Standard MIDI Files for Windfall.

pub mod error;
pub mod file;
pub mod read;
pub mod song;
pub mod write;

pub use error::MidiError;
pub use file::{read_file, write_file};
pub use read::{MAX_FILE_BYTES, read};
pub use song::*;
pub use write::{Resolution, SmfFormat, WriteOptions, write};
