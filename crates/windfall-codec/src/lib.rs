//! Audio files in and out of Windfall.
//!
//! Reading: WAV, AIFF, FLAC, MP3 and Ogg Vorbis are decoded whole into an
//! [`AudioBuffer`](windfall_core::AudioBuffer) at the file's own sample rate
//! and channel count. Nothing is resampled here; the engine does that at
//! playback.
//!
//! Writing: WAV, in one call or a block at a time.
//!
//! Drawing: [`peaks`] reduces a buffer to a waveform overview.

mod decode;
mod error;
mod peaks;
mod wav;

pub use decode::{
    AudioInfo, DEFAULT_MAX_DECODED_BYTES, DecodeOptions, decode_bytes, decode_bytes_with,
    decode_file, decode_file_with, probe_bytes, probe_file,
};
pub use error::CodecError;
pub use peaks::peaks;
pub use wav::{DEFAULT_DITHER_SEED, WavSampleFormat, WavWriter, write_wav};

/// File extensions Windfall can decode, lowercase and without the dot.
pub const AUDIO_EXTENSIONS: &[&str] = &[
    "wav", "wave", "aif", "aiff", "aifc", "flac", "mp3", "ogg", "oga",
];

/// Whether a file extension (without the dot, in any letter case) names a
/// format Windfall can decode.
pub fn is_audio_extension(ext: &str) -> bool {
    AUDIO_EXTENSIONS
        .iter()
        .any(|known| known.eq_ignore_ascii_case(ext))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_extensions_match_in_any_case() {
        for ext in ["wav", "WAV", "Flac", "mp3", "ogg", "aiff", "AIF"] {
            assert!(is_audio_extension(ext), "{ext}");
        }
    }

    #[test]
    fn other_extensions_do_not_match() {
        for ext in ["", ".wav", "txt", "mid", "windfall", "wav ", "mp4"] {
            assert!(!is_audio_extension(ext), "{ext:?}");
        }
    }

    #[test]
    fn the_extension_list_is_lowercase_without_dots() {
        for ext in AUDIO_EXTENSIONS {
            assert!(!ext.is_empty());
            assert!(
                ext.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            );
        }
    }
}
