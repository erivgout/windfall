//! Streaming MP3 encoding with the vendored LAME encoder.

use std::io::{Seek, SeekFrom, Write};
use std::num::NonZeroU32;
use std::path::Path;

use mp3lame_encoder::{
    Bitrate, Builder, FlushGap, InterleavedPcm, Mode, MonoPcm, Quality, VbrMode,
};

use crate::{CodecError, atomic::AtomicFile};

/// MPEG sample rates accepted without resampling.
pub const MP3_SAMPLE_RATES: &[u32] = &[
    8_000, 11_025, 12_000, 16_000, 22_050, 24_000, 32_000, 44_100, 48_000,
];

/// The rate policy of an MP3 file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mp3Rate {
    /// Constant kilobits per second: 128, 192, 256 or 320.
    Cbr(u16),
    /// LAME V0 (best) to V9 (smallest).
    Vbr(u8),
}

/// How stereo channels are encoded. Mono input requires Mono.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mp3Channels {
    Mono,
    Stereo,
    JointStereo,
}

/// Settings for an MP3 stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mp3Settings {
    pub rate: Mp3Rate,
    pub channels: Mp3Channels,
}

impl Default for Mp3Settings {
    fn default() -> Self {
        Self {
            rate: Mp3Rate::Cbr(192),
            channels: Mp3Channels::JointStereo,
        }
    }
}

pub(crate) fn check_layout(
    rate: u32,
    channels: u16,
    settings: Mp3Settings,
) -> Result<(), CodecError> {
    let invalid = |message: &str| Err(CodecError::InvalidInput(message.to_owned()));
    if !MP3_SAMPLE_RATES.contains(&rate) {
        return invalid(
            "MP3 requires 8000, 11025, 12000, 16000, 22050, 24000, 32000, 44100 or 48000 Hz; render at a supported rate",
        );
    }
    if channels
        != if settings.channels == Mp3Channels::Mono {
            1
        } else {
            2
        }
    {
        return invalid("MP3 mono mode requires one channel; stereo and joint stereo require two");
    }
    match settings.rate {
        Mp3Rate::Cbr(bitrate) => {
            if ![128, 192, 256, 320].contains(&bitrate) {
                return invalid("MP3 constant bitrate must be 128, 192, 256 or 320 kbit/s");
            }
            if rate < 32_000 && bitrate > 128 {
                return invalid("MP3 bitrates above 128 kbit/s require 32000, 44100 or 48000 Hz");
            }
            if rate < 16_000 {
                return invalid(
                    "MP3 constant 128 kbit/s or higher requires a sample rate of at least 16000 Hz; use VBR at lower rates",
                );
            }
        }
        Mp3Rate::Vbr(quality) if quality > 9 => return invalid("MP3 VBR quality must be 0 to 9"),
        Mp3Rate::Vbr(_) => {}
    }
    Ok(())
}

/// Writes bounded blocks into an atomic file and patches its gapless LAME header at completion.
pub struct Mp3Writer {
    encoder: mp3lame_encoder::Encoder,
    file: AtomicFile,
    channels: usize,
    samples: Vec<f32>,
    bytes: Vec<u8>,
    broken: bool,
}

impl std::fmt::Debug for Mp3Writer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mp3Writer")
            .field("channels", &self.channels)
            .field("broken", &self.broken)
            .finish_non_exhaustive()
    }
}

fn encoding(error: impl std::fmt::Display) -> CodecError {
    CodecError::Encoding(error.to_string())
}

impl Mp3Writer {
    /// Creates a stream without modifying its destination until finalization.
    pub fn create(
        path: impl AsRef<Path>,
        sample_rate: u32,
        channels: u16,
        settings: Mp3Settings,
    ) -> Result<Self, CodecError> {
        check_layout(sample_rate, channels, settings)?;
        let mut builder =
            Builder::new().ok_or_else(|| encoding("could not create the LAME encoder"))?;
        builder
            .set_num_channels(u8::try_from(channels).map_err(encoding)?)
            .map_err(encoding)?;
        builder.set_sample_rate(sample_rate).map_err(encoding)?;
        builder
            .set_output_sample_rate(NonZeroU32::new(sample_rate))
            .map_err(encoding)?;
        builder
            .set_mode(match settings.channels {
                Mp3Channels::Mono => Mode::Mono,
                Mp3Channels::Stereo => Mode::Stereo,
                Mp3Channels::JointStereo => Mode::JointStereo,
            })
            .map_err(encoding)?;
        builder.set_quality(Quality::NearBest).map_err(encoding)?;
        builder.set_to_write_vbr_tag(true).map_err(encoding)?;
        match settings.rate {
            Mp3Rate::Cbr(rate) => {
                builder.set_vbr_mode(VbrMode::Off).map_err(encoding)?;
                builder
                    .set_brate(match rate {
                        128 => Bitrate::Kbps128,
                        192 => Bitrate::Kbps192,
                        256 => Bitrate::Kbps256,
                        _ => Bitrate::Kbps320,
                    })
                    .map_err(encoding)?;
            }
            Mp3Rate::Vbr(quality) => {
                let quality = [
                    Quality::Best,
                    Quality::SecondBest,
                    Quality::NearBest,
                    Quality::VeryNice,
                    Quality::Nice,
                    Quality::Good,
                    Quality::Decent,
                    Quality::Ok,
                    Quality::SecondWorst,
                    Quality::Worst,
                ][usize::from(quality)];
                builder.set_vbr_mode(VbrMode::Mtrh).map_err(encoding)?;
                builder.set_vbr_quality(quality).map_err(encoding)?;
            }
        }
        Ok(Self {
            encoder: builder.build().map_err(encoding)?,
            file: AtomicFile::create(path.as_ref())?,
            channels: usize::from(channels),
            samples: Vec::with_capacity(8192),
            bytes: Vec::with_capacity(16384),
            broken: false,
        })
    }

    /// Appends finite interleaved samples, replacing nonfinite input with silence.
    pub fn write(&mut self, samples: &[f32]) -> Result<(), CodecError> {
        if self.broken {
            return Err(encoding("an earlier MP3 write failed"));
        }
        if !samples.len().is_multiple_of(self.channels) {
            return Err(CodecError::InvalidInput(
                "MP3 samples must make whole frames".to_owned(),
            ));
        }
        self.broken = true;
        for chunk in samples.chunks(4096 * self.channels) {
            self.samples.clear();
            self.samples.extend(
                chunk
                    .iter()
                    .map(|&sample| if sample.is_finite() { sample } else { 0.0 }),
            );
            self.bytes.clear();
            if self.channels == 1 {
                self.encoder
                    .encode_to_vec(MonoPcm(&self.samples), &mut self.bytes)
                    .map_err(encoding)?;
            } else {
                self.encoder
                    .encode_to_vec(InterleavedPcm(&self.samples), &mut self.bytes)
                    .map_err(encoding)?;
            }
            self.file.write_all(&self.bytes)?;
        }
        self.broken = false;
        Ok(())
    }

    /// Completes the stream, patches delay and padding, and places the file.
    pub fn finalize(mut self) -> Result<(), CodecError> {
        if self.broken {
            return Err(encoding("an earlier MP3 write failed"));
        }
        self.bytes.clear();
        self.encoder
            .flush_to_vec::<FlushGap>(&mut self.bytes)
            .map_err(encoding)?;
        self.file.write_all(&self.bytes)?;
        let mut tag = Vec::with_capacity(self.encoder.lame_tag_size());
        self.encoder
            .lame_tag_encode_to_vec(&mut tag)
            .ok_or_else(|| encoding("LAME did not supply its gapless header"))?;
        self.file.place(|file| {
            file.seek(SeekFrom::Start(0))?;
            file.write_all(&tag)
        })?;
        Ok(())
    }
}
