//! Text in an FL Studio file.
//!
//! Text events end with a zero. From FL Studio 11.5 on they are UTF-16,
//! little-endian. Before that they are one byte a character in the code
//! page of the computer that saved the file, which the file does not name.
//! The version itself (event 199) is one byte a character in every version.
//!
//! Source: PyFLP `pyflp/__init__.py`, which picks the encoding by the
//! version this way. A project saved by 11.1 that this crate was run over
//! is one byte a character, as that rule says.

use crate::model::FlVersion;

/// How the text events of a file are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextEncoding {
    /// One byte a character.
    Bytes,
    Utf16,
}

impl TextEncoding {
    pub(crate) fn of(version: FlVersion) -> Self {
        if (version.major, version.minor) >= (11, 5) {
            TextEncoding::Utf16
        } else {
            TextEncoding::Bytes
        }
    }

    /// The encoding a text most likely has, for a file that does not give
    /// its version first. UTF-16 text in a Latin script has a zero in
    /// every second byte.
    pub(crate) fn guess(data: &[u8]) -> Self {
        let units = data.len() / 2;
        if !data.len().is_multiple_of(2) || units == 0 {
            return TextEncoding::Bytes;
        }
        let zero_high = data
            .as_chunks::<2>()
            .0
            .iter()
            .filter(|unit| unit[1] == 0)
            .count();
        if zero_high * 2 >= units {
            TextEncoding::Utf16
        } else {
            TextEncoding::Bytes
        }
    }
}

/// Reads a text event, up to its first zero.
pub(crate) fn decode(data: &[u8], encoding: TextEncoding) -> String {
    match encoding {
        TextEncoding::Utf16 => {
            let units: Vec<u16> = data
                .as_chunks::<2>()
                .0
                .iter()
                .map(|unit| u16::from_le_bytes([unit[0], unit[1]]))
                .take_while(|&unit| unit != 0)
                .collect();
            String::from_utf16_lossy(&units)
        }
        TextEncoding::Bytes => decode_bytes(data),
    }
}

/// Reads one-byte text. Text that is valid UTF-8 is taken as that, which
/// covers ASCII. Anything else is read as Latin-1, where every byte is the
/// character of the same number: the right letters for Western European
/// text, and at least stable ones for the rest.
pub(crate) fn decode_bytes(data: &[u8]) -> String {
    let end = data
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(data.len());
    let text = &data[..end];
    match std::str::from_utf8(text) {
        Ok(text) => text.to_owned(),
        Err(_) => text.iter().map(|&byte| byte as char).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_ends_at_the_first_zero() {
        let data = [b'K', 0, b'i', 0, 0xE9, 0, 0, 0, b'x', 0];
        assert_eq!(decode(&data, TextEncoding::Utf16), "Kié");
        assert_eq!(decode(&[0, 0], TextEncoding::Utf16), "");
        assert_eq!(decode(&[], TextEncoding::Utf16), "");
    }

    #[test]
    fn utf16_with_an_odd_byte_or_a_broken_pair_still_reads() {
        assert_eq!(decode(&[b'A', 0, b'B'], TextEncoding::Utf16), "A");
        assert_eq!(
            decode(&[0x00, 0xD8, b'A', 0], TextEncoding::Utf16),
            "\u{FFFD}A"
        );
    }

    #[test]
    fn one_byte_text_reads_as_utf8_or_latin1() {
        assert_eq!(decode(b"Kick\0", TextEncoding::Bytes), "Kick");
        assert_eq!(decode(b"Kick\0junk", TextEncoding::Bytes), "Kick");
        assert_eq!(decode("Größe\0".as_bytes(), TextEncoding::Bytes), "Größe");
        assert_eq!(
            decode(&[b'G', b'r', 0xF6, 0xDF, b'e', 0], TextEncoding::Bytes),
            "Größe"
        );
    }

    #[test]
    fn the_encoding_follows_the_version() {
        assert_eq!(
            TextEncoding::of(FlVersion::new(11, 1, 0)),
            TextEncoding::Bytes
        );
        assert_eq!(
            TextEncoding::of(FlVersion::new(11, 5, 0)),
            TextEncoding::Utf16
        );
        assert_eq!(
            TextEncoding::of(FlVersion::new(20, 8, 4)),
            TextEncoding::Utf16
        );
        assert_eq!(
            TextEncoding::of(FlVersion::new(9, 0, 3)),
            TextEncoding::Bytes
        );
    }

    #[test]
    fn a_guess_tells_the_two_apart_for_latin_text() {
        assert_eq!(
            TextEncoding::guess(&[b'K', 0, b'i', 0, 0, 0]),
            TextEncoding::Utf16
        );
        assert_eq!(TextEncoding::guess(b"Kick\0"), TextEncoding::Bytes);
        assert_eq!(TextEncoding::guess(b"Kicks\0"), TextEncoding::Bytes);
        assert_eq!(TextEncoding::guess(&[]), TextEncoding::Bytes);
    }
}
