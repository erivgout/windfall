//! MIDI files on disk. Everything else in this crate works on bytes in
//! memory; these two functions are the only ones that touch a file.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use windfall_project::file::write_atomic;

use crate::error::MidiError;
use crate::read::{MAX_FILE_BYTES, read};
use crate::song::MidiSong;
use crate::write::{WriteOptions, write};

/// Reads the MIDI file at `path`. A file larger than [`MAX_FILE_BYTES`] is
/// refused before it is read.
pub fn read_file(path: impl AsRef<Path>) -> Result<MidiSong, MidiError> {
    let path = path.as_ref();
    let failed = |error: std::io::Error| MidiError::Io {
        path: path.display().to_string(),
        message: error.to_string(),
    };
    let file = File::open(path).map_err(failed)?;
    // One byte more than is allowed is enough to know the file is too
    // large, whatever its size turns out to be.
    let limit = MAX_FILE_BYTES as u64;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(failed)?;
    if bytes.len() as u64 > limit {
        let size = std::fs::metadata(path).map_or(limit + 1, |metadata| metadata.len());
        return Err(MidiError::TooLarge {
            bytes: size.max(limit + 1),
            limit,
        });
    }
    read(&bytes)
}

/// Writes a song to the MIDI file at `path`, replacing any file there. The
/// file is written whole or not at all, the way a project is saved.
pub fn write_file(
    path: impl AsRef<Path>,
    song: &MidiSong,
    options: &WriteOptions,
) -> Result<(), MidiError> {
    let path = path.as_ref();
    write_atomic(path, &write(song, options)).map_err(|error| MidiError::Io {
        path: path.display().to_string(),
        message: error.to_string(),
    })
}
