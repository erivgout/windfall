//! Why a MIDI file could not be read or written.

use std::fmt;

/// Why a MIDI file could not be read or written. Every message is written
/// for the person who chose the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MidiError {
    /// The data does not begin with a MIDI file header.
    NotMidi,
    /// The data is larger than a MIDI file is allowed to be.
    TooLarge { bytes: u64, limit: u64 },
    /// The file stops before the thing at `offset` is complete.
    Truncated { what: &'static str, offset: usize },
    /// The header names a format this reader does not know. Formats 0, 1
    /// and 2 are the only ones there are.
    UnsupportedFormat(u16),
    /// The header is malformed, or its time division cannot be used.
    BadHeader(&'static str),
    /// The file has a header and no track.
    NoTracks,
    /// A track holds something that is not a MIDI event. `track` counts
    /// from 1 and `offset` is the byte of the file where reading stopped.
    Corrupt {
        track: usize,
        offset: usize,
        problem: &'static str,
    },
    /// The file runs for more ticks than Windfall can count.
    TooLong,
    /// The file could not be opened, read or written.
    Io { path: String, message: String },
}

impl fmt::Display for MidiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MidiError::NotMidi => {
                write!(
                    f,
                    "this is not a MIDI file: it does not start with a MIDI header"
                )
            }
            MidiError::TooLarge { bytes, limit } => write!(
                f,
                "the file is {bytes} bytes, more than the {limit} bytes a MIDI file may have here"
            ),
            MidiError::Truncated { what, offset } => write!(
                f,
                "the MIDI file is cut short: {what} at byte {offset} runs past the end of the file"
            ),
            MidiError::UnsupportedFormat(format) => write!(
                f,
                "the MIDI file says it is format {format}, and only formats 0, 1 and 2 exist"
            ),
            MidiError::BadHeader(problem) => {
                write!(f, "the MIDI file's header cannot be used: {problem}")
            }
            MidiError::NoTracks => write!(f, "the MIDI file has no tracks"),
            MidiError::Corrupt {
                track,
                offset,
                problem,
            } => write!(
                f,
                "track {track} of the MIDI file is damaged at byte {offset}: {problem}"
            ),
            MidiError::TooLong => write!(
                f,
                "the MIDI file is longer than Windfall can count: its last event is past tick {}",
                u32::MAX
            ),
            MidiError::Io { path, message } => write!(f, "\"{path}\": {message}"),
        }
    }
}

impl std::error::Error for MidiError {}
