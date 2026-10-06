use std::fs::File;
use std::io::Cursor;
use std::panic::{self, AssertUnwindSafe};
use std::path::Path;

use symphonia::core::codecs::audio::well_known::{
    CODEC_ID_PCM_F32BE, CODEC_ID_PCM_F32LE, CODEC_ID_PCM_F64BE, CODEC_ID_PCM_F64LE,
};
use symphonia::core::codecs::audio::{AudioCodecId, AudioCodecParameters, AudioDecoderOptions};
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, Track, TrackType};
use symphonia::core::io::{MediaSource, MediaSourceStream, ReadOnlySource};
use symphonia::core::meta::MetadataOptions;
use windfall_core::AudioBuffer;

use crate::error::{CodecError, UNREADABLE_CODEC, from_symphonia};

/// Default for [`DecodeOptions::max_decoded_bytes`]: 1.5 GB of samples, which
/// is a little over an hour of stereo at 48 kHz.
pub const DEFAULT_MAX_DECODED_BYTES: u64 = 1536 * 1024 * 1024;

/// Most bytes of decoded audio one byte of file is believed to hold. A header
/// can claim any length, and a claim beyond this ratio is ignored: the audio
/// is then measured as it is decoded instead. PCM is at most 4 to 1 and a
/// 128 kbit/s MP3 about 22 to 1. A file compressed harder than this only
/// loses the shortcut of being sized up front.
const MAX_BELIEVABLE_EXPANSION: u64 = 64;

const BYTES_PER_SAMPLE: u64 = size_of::<f32>() as u64;

/// Limits applied while decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeOptions {
    /// Largest decoded size to accept, in bytes of 32-bit float samples
    /// (frames × channels × 4). Longer audio is rejected with
    /// [`CodecError::TooLarge`] instead of being cut short.
    pub max_decoded_bytes: u64,
}

impl Default for DecodeOptions {
    fn default() -> Self {
        Self {
            max_decoded_bytes: DEFAULT_MAX_DECODED_BYTES,
        }
    }
}

/// What a file says about itself, read without decoding the audio.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioInfo {
    pub sample_rate: u32,
    pub channels: u16,
    /// Length in frames, when the container states it. Encoder delay and
    /// padding are already left out.
    pub frames: Option<u64>,
    /// Length in seconds, when the container states it.
    pub duration_secs: Option<f64>,
    /// Short name of the container, such as `wave`, `flac`, `mp3` or `ogg`.
    pub format: String,
    /// Short name of the codec, such as `pcm_s16le`, `flac`, `mp3` or `vorbis`.
    pub codec: String,
    /// Bit depth of the stored samples. Lossy codecs do not have one.
    pub bits_per_sample: Option<u32>,
}

/// Decodes a whole audio file with the default limits. See
/// [`decode_file_with`].
pub fn decode_file(path: impl AsRef<Path>) -> Result<AudioBuffer, CodecError> {
    decode_file_with(path, &DecodeOptions::default())
}

/// Decodes a whole audio file into interleaved 32-bit float at the file's own
/// sample rate and channel count.
///
/// Encoder delay and padding are trimmed when the file records them. If the
/// file is damaged or cut short part-way through, the audio decoded before the
/// damage is returned; an error is returned only when nothing could be decoded.
pub fn decode_file_with(
    path: impl AsRef<Path>,
    options: &DecodeOptions,
) -> Result<AudioBuffer, CodecError> {
    let path = path.as_ref();
    try_both_ways(|seekable| {
        let (file, len) = open_file(path)?;
        let source = as_source(file, seekable);
        decode(source, len, extension_of(path), options)
    })
}

/// Decodes audio held in memory with the default limits. See
/// [`decode_bytes_with`].
pub fn decode_bytes(bytes: &[u8], hint_ext: Option<&str>) -> Result<AudioBuffer, CodecError> {
    decode_bytes_with(bytes, hint_ext, &DecodeOptions::default())
}

/// Decodes audio held in memory, exactly as [`decode_file_with`] decodes a
/// file. `hint_ext` is the file extension the bytes came with, if any. It
/// only speeds up format detection; a wrong hint does no harm.
pub fn decode_bytes_with(
    bytes: &[u8],
    hint_ext: Option<&str>,
    options: &DecodeOptions,
) -> Result<AudioBuffer, CodecError> {
    if bytes.is_empty() {
        return Err(CodecError::NoAudio);
    }
    try_both_ways(|seekable| {
        let source = as_source(Cursor::new(bytes), seekable);
        decode(source, bytes.len() as u64, hint_ext, options)
    })
}

/// Reads a file's sample rate, channel count, length and format from its
/// headers, without decoding the audio.
pub fn probe_file(path: impl AsRef<Path>) -> Result<AudioInfo, CodecError> {
    let path = path.as_ref();
    try_both_ways(|seekable| {
        let (file, _) = open_file(path)?;
        probe(as_source(file, seekable), extension_of(path))
    })
}

/// Reads the same information as [`probe_file`] from audio held in memory.
pub fn probe_bytes(bytes: &[u8], hint_ext: Option<&str>) -> Result<AudioInfo, CodecError> {
    if bytes.is_empty() {
        return Err(CodecError::NoAudio);
    }
    try_both_ways(|seekable| probe(as_source(Cursor::new(bytes), seekable), hint_ext))
}

/// Opens a file and returns it with its length in bytes.
fn open_file(path: &Path) -> Result<(File, u64), CodecError> {
    let file = File::open(path)?;
    let len = file.metadata()?.len();
    if len == 0 {
        return Err(CodecError::NoAudio);
    }
    Ok((file, len))
}

fn extension_of(path: &Path) -> Option<&str> {
    path.extension().and_then(|extension| extension.to_str())
}

/// Boxes a source for Symphonia, either as it is or as a stream that can only
/// be read forwards.
fn as_source<'s, R>(source: R, seekable: bool) -> Box<dyn MediaSource + 's>
where
    R: MediaSource + 's,
{
    if seekable {
        Box::new(source)
    } else {
        Box::new(ReadOnlySource::new(source))
    }
}

/// Runs `attempt` on a seekable source and, if the file turns out to be
/// damaged, once more on a forward-only one.
///
/// Given a seekable source, the AIFF and Ogg readers look at the end of the
/// file before they hand out any audio, and give up if a cut-off file ends
/// early. Reading the same file as a stream gets the audio that is there.
fn try_both_ways<T>(
    mut attempt: impl FnMut(bool) -> Result<T, CodecError>,
) -> Result<T, CodecError> {
    match guarded(|| attempt(true)) {
        Err(CodecError::Corrupt(reason)) => {
            guarded(|| attempt(false)).map_err(|_| CodecError::Corrupt(reason))
        }
        result => result,
    }
}

/// Runs decoder code and turns a panic inside it into an error. The decoders
/// are fed whatever the user drops on the app, and one of them tripping over a
/// malformed file must not take the app down.
fn guarded<T>(work: impl FnOnce() -> Result<T, CodecError>) -> Result<T, CodecError> {
    panic::catch_unwind(AssertUnwindSafe(work)).unwrap_or_else(|_| {
        Err(CodecError::Corrupt(
            "the decoder could not make sense of it".to_owned(),
        ))
    })
}

fn open_format<'s>(
    source: Box<dyn MediaSource + 's>,
    hint_ext: Option<&str>,
) -> Result<Box<dyn FormatReader + 's>, CodecError> {
    let stream = MediaSourceStream::new(source, Default::default());
    let mut hint = Hint::new();
    if let Some(extension) = hint_ext {
        hint.with_extension(extension);
    }
    symphonia::default::get_probe()
        .probe(
            &hint,
            stream,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(from_symphonia)
}

fn audio_track(format: &dyn FormatReader) -> Result<(&Track, &AudioCodecParameters), CodecError> {
    let no_track = || CodecError::UnsupportedFormat("it has no audio track".to_owned());
    let track = format
        .default_track(TrackType::Audio)
        .ok_or_else(no_track)?;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|params| params.audio())
        .ok_or_else(no_track)?;
    Ok((track, params))
}

fn probe<'s>(
    source: Box<dyn MediaSource + 's>,
    hint_ext: Option<&str>,
) -> Result<AudioInfo, CodecError> {
    let format = open_format(source, hint_ext)?;
    let (track, params) = audio_track(format.as_ref())?;

    let sample_rate = params
        .sample_rate
        .filter(|&rate| rate > 0)
        .ok_or_else(|| CodecError::Corrupt("the sample rate is missing".to_owned()))?;
    let channels = params
        .channels
        .as_ref()
        .map(|channels| channels.count())
        .filter(|&count| count > 0)
        .ok_or_else(|| CodecError::Corrupt("the channel count is missing".to_owned()))?;
    let codec = symphonia::default::get_codecs()
        .get_audio_decoder(params.codec)
        .ok_or_else(|| CodecError::UnsupportedFormat(UNREADABLE_CODEC.to_owned()))?
        .codec
        .info
        .short_name;

    let duration_secs = match track.num_frames {
        Some(frames) => Some(frames as f64 / f64::from(sample_rate)),
        None => track
            .time_base
            .zip(track.duration)
            .and_then(|(time_base, duration)| time_base.calc_duration(duration))
            .map(|time| time.as_secs_f64()),
    };

    Ok(AudioInfo {
        sample_rate,
        channels: channel_count(channels)?,
        frames: track.num_frames,
        duration_secs,
        format: format.format_info().short_name.to_owned(),
        codec: codec.to_owned(),
        bits_per_sample: params.bits_per_sample.or(float_bits(params.codec)),
    })
}

/// Bit depth of the float PCM codecs, which the container readers leave out.
fn float_bits(codec: AudioCodecId) -> Option<u32> {
    match codec {
        CODEC_ID_PCM_F32LE | CODEC_ID_PCM_F32BE => Some(32),
        CODEC_ID_PCM_F64LE | CODEC_ID_PCM_F64BE => Some(64),
        _ => None,
    }
}

fn channel_count(channels: usize) -> Result<u16, CodecError> {
    u16::try_from(channels)
        .map_err(|_| CodecError::UnsupportedFormat(format!("it has {channels} channels")))
}

/// The decoded samples so far, grown without ever aborting on a failed
/// allocation and without exceeding the caller's limit.
struct Sink {
    samples: Vec<f32>,
    limit_samples: usize,
    limit_bytes: u64,
}

impl Sink {
    fn new(options: &DecodeOptions) -> Self {
        Self {
            samples: Vec::new(),
            limit_samples: usize::try_from(options.max_decoded_bytes / BYTES_PER_SAMPLE)
                .unwrap_or(usize::MAX),
            limit_bytes: options.max_decoded_bytes,
        }
    }

    fn too_large(&self) -> CodecError {
        CodecError::TooLarge {
            limit_bytes: self.limit_bytes,
        }
    }

    /// Takes the length the header promises. A length over the limit is
    /// rejected right away, before any time is spent decoding; otherwise the
    /// memory for it is set aside in one piece.
    fn expect(&mut self, samples: u64) -> Result<(), CodecError> {
        let samples = usize::try_from(samples)
            .ok()
            .filter(|&samples| samples <= self.limit_samples)
            .ok_or_else(|| self.too_large())?;
        self.samples
            .try_reserve_exact(samples)
            .map_err(|_| CodecError::OutOfMemory)
    }

    /// Makes room for `additional` more samples and returns the slice to fill.
    fn grow(&mut self, additional: usize) -> Result<&mut [f32], CodecError> {
        let start = self.samples.len();
        let needed = start
            .checked_add(additional)
            .filter(|&needed| needed <= self.limit_samples)
            .ok_or_else(|| self.too_large())?;

        if needed > self.samples.capacity() {
            let target = needed
                .max(self.samples.capacity().saturating_mul(2))
                .min(self.limit_samples);
            self.samples
                .try_reserve_exact(target - start)
                .map_err(|_| CodecError::OutOfMemory)?;
        }

        self.samples.resize(needed, 0.0);
        Ok(&mut self.samples[start..])
    }
}

fn decode<'s>(
    source: Box<dyn MediaSource + 's>,
    source_len: u64,
    hint_ext: Option<&str>,
    options: &DecodeOptions,
) -> Result<AudioBuffer, CodecError> {
    let mut format = open_format(source, hint_ext)?;
    let (track, params) = audio_track(format.as_ref())?;
    let track_id = track.id;

    let mut sink = Sink::new(options);
    let promised = track
        .num_frames
        .zip(params.channels.as_ref())
        .map(|(frames, channels)| frames.saturating_mul(channels.count() as u64));
    if let Some(samples) = promised
        && samples.saturating_mul(BYTES_PER_SAMPLE)
            <= source_len.saturating_mul(MAX_BELIEVABLE_EXPANSION)
    {
        sink.expect(samples)?;
    }

    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .map_err(from_symphonia)?;

    // Sample rate and channel count of the audio decoded so far.
    let mut layout: Option<(u32, usize)> = None;

    // The loop ends with the error that stopped it, or `None` at a clean end.
    let stopped_by = loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break None,
            Err(error) => break Some(error),
        };
        if packet.track_id != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(error) => break Some(error),
        };
        if decoded.frames() == 0 {
            continue;
        }

        let found = (decoded.spec().rate(), decoded.spec().channels().count());
        match layout {
            None => layout = Some(found),
            // One buffer holds one layout. The readers end the stream where a
            // second one with another layout is glued on, but that is theirs
            // to change, so it is checked here and only the first part kept.
            Some(first) if first != found => break None,
            Some(_) => {}
        }

        let block = sink.grow(decoded.samples_interleaved())?;
        decoded.copy_to_slice_interleaved(&mut *block);
        // Float files can carry NaN and infinity, which would spread through
        // every filter and meter downstream.
        for sample in block {
            if !sample.is_finite() {
                *sample = 0.0;
            }
        }
    };

    let Some((sample_rate, channels)) = layout.filter(|_| !sink.samples.is_empty()) else {
        return Err(stopped_by.map_or(CodecError::NoAudio, from_symphonia));
    };
    if sample_rate == 0 || channels == 0 {
        return Err(CodecError::Corrupt(
            "the sample rate or channel count is zero".to_owned(),
        ));
    }

    Ok(AudioBuffer::from_interleaved(
        sample_rate,
        channel_count(channels)?,
        sink.samples,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sink_limited_to(max_decoded_bytes: u64) -> Sink {
        Sink::new(&DecodeOptions { max_decoded_bytes })
    }

    #[test]
    fn a_panicking_decoder_becomes_an_error() {
        let result: Result<(), CodecError> = guarded(|| panic!("decoder bug"));
        assert!(matches!(result, Err(CodecError::Corrupt(_))));
    }

    #[test]
    fn the_sink_refuses_to_pass_its_limit() {
        let mut sink = sink_limited_to(40);
        assert_eq!(sink.grow(6).unwrap().len(), 6);
        assert_eq!(sink.grow(4).unwrap().len(), 4);
        assert!(matches!(
            sink.grow(1),
            Err(CodecError::TooLarge { limit_bytes: 40 })
        ));
        assert_eq!(sink.samples.len(), 10);
    }

    #[test]
    fn a_promised_length_over_the_limit_is_refused_up_front() {
        let mut sink = sink_limited_to(40);
        assert!(matches!(sink.expect(11), Err(CodecError::TooLarge { .. })));
        assert!(matches!(
            sink.expect(u64::MAX),
            Err(CodecError::TooLarge { .. })
        ));
        sink.expect(10).unwrap();
        assert!(sink.samples.capacity() >= 10);
    }

    #[test]
    fn the_default_limit_is_over_an_hour_of_stereo() {
        let seconds = DEFAULT_MAX_DECODED_BYTES / BYTES_PER_SAMPLE / 2 / 48_000;
        assert!((3_600..7_200).contains(&seconds), "{seconds} s");
    }
}
