//! One way to write every format Windfall exports.

use std::path::Path;

use crate::error::CodecError;
use crate::flac::{self, FlacBitDepth, FlacWriter};
use crate::wav::{self, WavSampleFormat, WavWriter};

/// The audio file formats Windfall writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioFormat {
    Wav,
    Flac,
}

impl AudioFormat {
    /// The ending of a file of this format, lowercase and without the dot.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Flac => "flac",
        }
    }

    /// The name of the format, as a user knows it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Wav => "WAV",
            Self::Flac => "FLAC",
        }
    }
}

/// A format and how to write it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EncoderSettings {
    /// Uncompressed. See [`WavWriter`].
    Wav { format: WavSampleFormat },
    /// Lossless. `level` is the compression level, 0 to
    /// [`MAX_FLAC_LEVEL`](crate::MAX_FLAC_LEVEL). See [`FlacWriter`].
    Flac { depth: FlacBitDepth, level: u8 },
}

impl EncoderSettings {
    pub fn format(&self) -> AudioFormat {
        match self {
            Self::Wav { .. } => AudioFormat::Wav,
            Self::Flac { .. } => AudioFormat::Flac,
        }
    }

    /// Says whether audio at `sample_rate` on `channels` channels can be
    /// written with these settings, without creating a file. What fails
    /// here is what [`Encoder::open`] would refuse, with the same message.
    pub fn check(&self, sample_rate: u32, channels: u16) -> Result<(), CodecError> {
        match *self {
            Self::Wav { format } => wav::check_layout(sample_rate, channels, format),
            Self::Flac { level, .. } => flac::check_layout(sample_rate, channels, level),
        }
    }
}

/// Writes an audio file of any format Windfall exports, a block at a time.
///
/// Every format is written the same way: [`open`](Self::open),
/// [`write`](Self::write) interleaved samples as often as needed, then
/// [`finalize`](Self::finalize). The audio goes to a temporary file beside
/// the destination, which takes the destination's name only in `finalize`.
/// An encoder that is dropped first removes the temporary file and leaves
/// whatever was at the destination untouched.
///
/// The same samples and settings always give the same file, byte for byte:
/// nothing in a file comes from the clock or from the system's randomness.
#[derive(Debug)]
pub struct Encoder {
    inner: Inner,
}

#[derive(Debug)]
enum Inner {
    Wav(WavWriter),
    Flac(Box<FlacWriter>),
}

impl Encoder {
    /// Starts a file that will appear at `path` when finalized.
    pub fn open(
        path: impl AsRef<Path>,
        settings: &EncoderSettings,
        sample_rate: u32,
        channels: u16,
    ) -> Result<Self, CodecError> {
        let inner = match *settings {
            EncoderSettings::Wav { format } => {
                Inner::Wav(WavWriter::create(path, sample_rate, channels, format)?)
            }
            EncoderSettings::Flac { depth, level } => {
                let writer = FlacWriter::create(path, sample_rate, channels, depth, level)?;
                Inner::Flac(Box::new(writer))
            }
        };
        Ok(Self { inner })
    }

    /// Appends interleaved samples. The length must be a whole number of
    /// frames.
    pub fn write(&mut self, interleaved: &[f32]) -> Result<(), CodecError> {
        match &mut self.inner {
            Inner::Wav(writer) => writer.write(interleaved),
            Inner::Flac(writer) => writer.write(interleaved),
        }
    }

    /// Completes the file and moves it to its destination, replacing any
    /// file already there.
    pub fn finalize(self) -> Result<(), CodecError> {
        match self.inner {
            Inner::Wav(writer) => writer.finalize(),
            Inner::Flac(writer) => writer.finalize(),
        }
    }
}
