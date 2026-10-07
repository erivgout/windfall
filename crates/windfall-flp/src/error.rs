//! Why a file could not be read at all.

use thiserror::Error;

/// A problem that stops a file from being read. Problems that only cost a
/// part of the file are [`Diagnostic`](crate::Diagnostic)s instead, and the
/// rest is still read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FlpError {
    #[error(
        "the file is {size} bytes long, and no more than {limit} bytes of an FL Studio project are read"
    )]
    TooLarge { size: u64, limit: u64 },
    #[error("the file is too short to be an FL Studio project: it has {size} bytes")]
    TooShort { size: usize },
    #[error(
        "this is a zip archive, not an FL Studio project. Unpack it and open the .flp file inside"
    )]
    Zipped,
    #[error("this is not an FL Studio project: it starts with {found} instead of \"FLhd\"")]
    NotFlp { found: String },
    #[error("the header of the file is damaged: it claims to be {length} bytes long, not 6")]
    HeaderLength { length: u32 },
    #[error("the header of the file is damaged: it gives a time base of 0 ticks per quarter note")]
    ZeroPpq,
    #[error(
        "the file is damaged: a part of it at byte {offset} claims to be {length} bytes long, which is more than there is"
    )]
    ChunkLength { offset: usize, length: u32 },
    #[error(
        "the file has a header but nothing after it: the part that holds the project is missing"
    )]
    NoData,
}

/// Where an event stream stopped being readable. The events before it are
/// good.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EventError {
    #[error("the file ends in the middle of the event that starts at byte {offset}")]
    Truncated { offset: usize },
    #[error("the event at byte {offset} has a length that cannot be read")]
    Length { offset: usize },
}
