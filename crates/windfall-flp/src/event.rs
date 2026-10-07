//! The container of an FL Studio project file: a header, and a row of
//! events.
//!
//! A file is two chunks. The header chunk is `FLhd`, a length of 6, and
//! three 16-bit numbers: the kind of file, the number of channels and the
//! time base in ticks per quarter note. The data chunk is `FLdt`, its
//! length, and events until the end. An event is one byte that says what it
//! is, followed by its value. The number says how large the value is:
//!
//! | Id | Value |
//! |---|---|
//! | 0 to 63 | one byte |
//! | 64 to 127 | two bytes |
//! | 128 to 191 | four bytes |
//! | 192 to 255 | a length, then that many bytes |
//!
//! The length is a base-128 number, least significant seven bits first,
//! with the top bit of a byte saying that another byte follows. Everything
//! is little-endian.
//!
//! Sources: PyFLP `docs/architecture/flp-format.rst` and `pyflp/__init__.py`
//! (the chunk layout and the four value sizes), LMMS `FlpImport.cpp` (the
//! same, the search for `FLdt` past other chunks, and the length encoding).
//!
//! Nothing here knows what an event means. [`parse`](crate::parse) does.

use crate::error::{EventError, FlpError};

/// First id whose value is two bytes.
pub const WORD: u8 = 64;
/// First id whose value is four bytes.
pub const DWORD: u8 = 128;
/// First id whose value has a length of its own.
pub const DATA: u8 = 192;

/// Largest file that is read, in bytes. Every allocation the reader and the
/// parser make is a small multiple of the size of the input, so this caps
/// them all.
pub const MAX_FILE_BYTES: usize = 512 * 1024 * 1024;

const HEADER_MAGIC: &[u8; 4] = b"FLhd";
const DATA_MAGIC: &[u8; 4] = b"FLdt";
/// Bytes of the header chunk: its magic, its length and its three fields.
const HEADER_BYTES: usize = 14;

/// The header chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// What kind of file this is. See [`FileFormat`].
    pub format: FileFormat,
    /// The number of channels in the channel rack, as the header gives it.
    /// The channels that the events describe are what counts.
    pub channel_count: u16,
    /// Ticks per quarter note. Every position and length in the file is in
    /// these ticks. Never 0.
    pub ppq: u16,
}

/// What a file holds. FL Studio writes presets and scores in the same
/// container as projects.
///
/// Source: PyFLP `pyflp/project.py`, `FileFormat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    /// A project (`.flp`).
    Project,
    /// A score (`.fsc`): the notes of one pattern.
    Score,
    /// Automation data.
    Automation,
    /// The state of one channel (`.fst`).
    ChannelState,
    /// The state of one plugin (`.fst`).
    PluginState,
    /// The state of a hosted instrument.
    GeneratorState,
    /// The state of a hosted effect.
    EffectState,
    /// The state of one mixer insert (`.fst`).
    InsertState,
    /// A value the sources do not name.
    Other(i16),
}

impl FileFormat {
    /// The format for the number in the header.
    pub fn from_raw(raw: i16) -> Self {
        match raw {
            0 => FileFormat::Project,
            0x10 => FileFormat::Score,
            24 => FileFormat::Automation,
            0x20 => FileFormat::ChannelState,
            0x30 => FileFormat::PluginState,
            0x31 => FileFormat::GeneratorState,
            0x32 => FileFormat::EffectState,
            0x40 => FileFormat::InsertState,
            other => FileFormat::Other(other),
        }
    }

    /// The number the header holds for this format.
    pub fn raw(self) -> i16 {
        match self {
            FileFormat::Project => 0,
            FileFormat::Score => 0x10,
            FileFormat::Automation => 24,
            FileFormat::ChannelState => 0x20,
            FileFormat::PluginState => 0x30,
            FileFormat::GeneratorState => 0x31,
            FileFormat::EffectState => 0x32,
            FileFormat::InsertState => 0x40,
            FileFormat::Other(raw) => raw,
        }
    }

    /// What to call the file in a message.
    pub fn describe(self) -> &'static str {
        match self {
            FileFormat::Project => "a project",
            FileFormat::Score => "a score",
            FileFormat::Automation => "automation data",
            FileFormat::ChannelState => "a channel preset",
            FileFormat::PluginState | FileFormat::GeneratorState | FileFormat::EffectState => {
                "a plugin preset"
            }
            FileFormat::InsertState => "a mixer track preset",
            FileFormat::Other(_) => "a file of an unknown kind",
        }
    }
}

/// One event of the data chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event<'a> {
    pub id: u8,
    /// Where in the file the event starts.
    pub offset: usize,
    pub value: EventValue<'a>,
}

/// The value of an event. Which variant it is follows from the id alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventValue<'a> {
    Byte(u8),
    Word(u16),
    DWord(u32),
    Data(&'a [u8]),
}

impl EventValue<'_> {
    /// The value as a number, for the three sizes that are one. `None` for
    /// data.
    pub fn number(&self) -> Option<u32> {
        match *self {
            EventValue::Byte(value) => Some(u32::from(value)),
            EventValue::Word(value) => Some(u32::from(value)),
            EventValue::DWord(value) => Some(value),
            EventValue::Data(_) => None,
        }
    }

    /// The bytes of a data event. `None` for a number.
    pub fn data(&self) -> Option<&[u8]> {
        match self {
            EventValue::Data(data) => Some(data),
            _ => None,
        }
    }
}

/// An opened file: its header, and its events one at a time.
#[derive(Debug, Clone)]
pub struct Container<'a> {
    pub header: Header,
    /// The events of the data chunk.
    pub events: EventReader<'a>,
    /// The data chunk says it is longer than what is left of the file. The
    /// events that are there are still read.
    pub cut_short: bool,
    /// Bytes after the end of the data chunk, which nothing reads.
    pub trailing_bytes: usize,
}

/// Reads the two chunk headers of a file and returns what is needed to read
/// its events. It does not look at any event.
pub fn open(bytes: &[u8]) -> Result<Container<'_>, FlpError> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(FlpError::TooLarge {
            size: bytes.len() as u64,
            limit: MAX_FILE_BYTES as u64,
        });
    }
    if bytes.starts_with(b"PK\x03\x04") {
        return Err(FlpError::Zipped);
    }
    let Some(magic) = bytes.get(..4) else {
        return Err(FlpError::TooShort { size: bytes.len() });
    };
    if magic != HEADER_MAGIC {
        return Err(FlpError::NotFlp {
            found: printable(magic),
        });
    }
    if bytes.len() < HEADER_BYTES {
        return Err(FlpError::TooShort { size: bytes.len() });
    }
    let length = u32_at(bytes, 4);
    if length != 6 {
        return Err(FlpError::HeaderLength { length });
    }
    let header = Header {
        format: FileFormat::from_raw(u16_at(bytes, 8) as i16),
        channel_count: u16_at(bytes, 10),
        ppq: u16_at(bytes, 12),
    };
    if header.ppq == 0 {
        return Err(FlpError::ZeroPpq);
    }

    // Other chunks may sit between the header and the data. They are
    // skipped, as LMMS's importer does.
    let mut at = HEADER_BYTES;
    loop {
        let Some(chunk) = bytes.get(at..at + 8) else {
            return Err(FlpError::NoData);
        };
        let length = u32_at(chunk, 4);
        let start = at + 8;
        let left = bytes.len() - start;
        if &chunk[..4] == DATA_MAGIC {
            let declared = length as usize;
            let end = start + declared.min(left);
            return Ok(Container {
                header,
                events: EventReader {
                    bytes,
                    at: start,
                    end,
                    done: false,
                },
                cut_short: declared > left,
                trailing_bytes: bytes.len() - end,
            });
        }
        if length as usize > left {
            return Err(FlpError::ChunkLength { offset: at, length });
        }
        at = start + length as usize;
    }
}

/// Walks the events of a data chunk. It borrows the file and allocates
/// nothing.
///
/// After the first error it ends: an event stream cannot be picked up again
/// once its framing is lost.
#[derive(Debug, Clone)]
pub struct EventReader<'a> {
    bytes: &'a [u8],
    at: usize,
    end: usize,
    done: bool,
}

impl<'a> EventReader<'a> {
    /// Where in the file the next event starts.
    pub fn offset(&self) -> usize {
        self.at
    }

    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let end = self.at.checked_add(count)?;
        if end > self.end {
            return None;
        }
        let taken = &self.bytes[self.at..end];
        self.at = end;
        Some(taken)
    }

    fn read(&mut self) -> Result<Event<'a>, EventError> {
        let offset = self.at;
        let cut = EventError::Truncated { offset };
        let id = self.take(1).ok_or(cut.clone())?[0];
        let value = if id < WORD {
            EventValue::Byte(self.take(1).ok_or(cut)?[0])
        } else if id < DWORD {
            EventValue::Word(u16_at(self.take(2).ok_or(cut)?, 0))
        } else if id < DATA {
            EventValue::DWord(u32_at(self.take(4).ok_or(cut)?, 0))
        } else {
            let length = self.length(offset)?;
            EventValue::Data(self.take(length).ok_or(cut)?)
        };
        Ok(Event { id, offset, value })
    }

    /// Reads the length of a data event. Five bytes hold any 32-bit length,
    /// so a sixth byte means the file is damaged.
    fn length(&mut self, offset: usize) -> Result<usize, EventError> {
        let mut length = 0_u64;
        for index in 0..5 {
            let byte = self.take(1).ok_or(EventError::Truncated { offset })?[0];
            length |= u64::from(byte & 0x7F) << (7 * index);
            if byte & 0x80 == 0 {
                return usize::try_from(length).map_err(|_| EventError::Length { offset });
            }
        }
        Err(EventError::Length { offset })
    }
}

impl<'a> Iterator for EventReader<'a> {
    type Item = Result<Event<'a>, EventError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.at >= self.end {
            return None;
        }
        let event = self.read();
        if event.is_err() {
            self.done = true;
        }
        Some(event)
    }
}

/// Reads a 16-bit number. The caller has checked that the bytes are there.
pub(crate) fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

/// Reads a 32-bit number. The caller has checked that the bytes are there.
pub(crate) fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// Bytes as text for a message, with anything unprintable as a dot.
fn printable(bytes: &[u8]) -> String {
    let text: String = bytes
        .iter()
        .map(|&byte| {
            if byte.is_ascii_graphic() || byte == b' ' {
                byte as char
            } else {
                '.'
            }
        })
        .collect();
    format!("\"{text}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(events: &[u8]) -> Vec<u8> {
        let mut bytes = b"FLhd\x06\0\0\0\0\0\x01\0\x60\0FLdt".to_vec();
        bytes.extend_from_slice(&(events.len() as u32).to_le_bytes());
        bytes.extend_from_slice(events);
        bytes
    }

    fn events(bytes: &[u8]) -> Vec<Result<Event<'_>, EventError>> {
        open(bytes)
            .map(|file| file.events.collect())
            .unwrap_or_default()
    }

    #[test]
    fn the_header_gives_format_channels_and_time_base() {
        let bytes = file(&[]);
        let container = open(&bytes).expect("a valid file");
        assert_eq!(
            container.header,
            Header {
                format: FileFormat::Project,
                channel_count: 1,
                ppq: 96,
            }
        );
        assert!(!container.cut_short);
        assert_eq!(container.trailing_bytes, 0);
    }

    #[test]
    fn each_range_of_ids_has_its_own_value_size() {
        let bytes = file(&[
            9, 1, // byte
            66, 0x8C, 0x00, // word
            156, 0x40, 0x0D, 0x03, 0x00, // dword
            199, 3, b'1', b'2', 0, // data
        ]);
        let read: Vec<_> = events(&bytes).into_iter().map(Result::unwrap).collect();
        assert_eq!(read.len(), 4);
        assert_eq!(read[0].value, EventValue::Byte(1));
        assert_eq!(read[1].value, EventValue::Word(140));
        assert_eq!(read[2].value, EventValue::DWord(200_000));
        assert_eq!(read[3].value, EventValue::Data(b"12\0"));
        assert_eq!(read[1].offset, 24);
    }

    #[test]
    fn a_length_can_take_several_bytes() {
        let mut stream = vec![210, 0x80, 0x01];
        stream.extend(std::iter::repeat_n(7_u8, 128));
        let bytes = file(&stream);
        let read = events(&bytes);
        assert_eq!(read.len(), 1);
        assert_eq!(
            read[0]
                .as_ref()
                .map(|event| event.value.data().map(<[u8]>::len)),
            Ok(Some(128))
        );
    }

    #[test]
    fn an_event_that_is_cut_off_ends_the_stream_with_an_error() {
        let bytes = file(&[9, 1, 156, 1, 2]);
        let read = events(&bytes);
        assert_eq!(read.len(), 2);
        assert!(read[0].is_ok());
        assert_eq!(read[1], Err(EventError::Truncated { offset: 24 }));
    }

    #[test]
    fn a_length_longer_than_the_file_is_an_error_and_allocates_nothing() {
        let bytes = file(&[210, 0xFF, 0xFF, 0xFF, 0xFF, 0x0F, 1, 2, 3]);
        assert_eq!(
            events(&bytes),
            vec![Err(EventError::Truncated { offset: 22 })]
        );
    }

    #[test]
    fn a_length_of_more_than_five_bytes_is_an_error() {
        let bytes = file(&[210, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01]);
        assert_eq!(events(&bytes), vec![Err(EventError::Length { offset: 22 })]);
    }

    #[test]
    fn a_data_chunk_that_claims_too_much_is_read_as_far_as_it_goes() {
        let mut bytes = file(&[9, 1]);
        bytes[18..22].copy_from_slice(&1000_u32.to_le_bytes());
        let container = open(&bytes).expect("still readable");
        assert!(container.cut_short);
        assert_eq!(container.events.count(), 1);
    }

    #[test]
    fn chunks_before_the_data_are_skipped() {
        let mut bytes = b"FLhd\x06\0\0\0\0\0\x01\0\x60\0".to_vec();
        bytes.extend_from_slice(b"JUNK\x03\0\0\0abc");
        bytes.extend_from_slice(b"FLdt\x02\0\0\0\x09\x01");
        let container = open(&bytes).expect("data after another chunk");
        assert_eq!(container.events.count(), 1);
    }

    #[test]
    fn files_that_are_something_else_say_so() {
        let error = |bytes: &[u8]| open(bytes).err();
        assert_eq!(error(b""), Some(FlpError::TooShort { size: 0 }));
        assert_eq!(error(b"FLhd\x06\0"), Some(FlpError::TooShort { size: 6 }));
        assert_eq!(error(b"PK\x03\x04rest of a zip"), Some(FlpError::Zipped));
        assert_eq!(
            error(b"RIFF\x24\0\0\0WAVEfmt "),
            Some(FlpError::NotFlp {
                found: "\"RIFF\"".to_owned()
            })
        );
        assert_eq!(
            error(b"FLhd\x07\0\0\0\0\0\x01\0\x60\0"),
            Some(FlpError::HeaderLength { length: 7 })
        );
        assert_eq!(
            error(b"FLhd\x06\0\0\0\0\0\x01\0\0\0FLdt\0\0\0\0"),
            Some(FlpError::ZeroPpq)
        );
        assert_eq!(
            error(b"FLhd\x06\0\0\0\0\0\x01\0\x60\0"),
            Some(FlpError::NoData)
        );
        assert_eq!(
            error(b"FLhd\x06\0\0\0\0\0\x01\0\x60\0JUNK\xFF\0\0\0abc"),
            Some(FlpError::ChunkLength {
                offset: 14,
                length: 255
            })
        );
    }
}
