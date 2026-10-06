//! Writes WAV files and reads them back.

use std::fs;

use windfall_codec::{CodecError, WavSampleFormat, WavWriter, decode_file, probe_file, write_wav};
use windfall_core::AudioBuffer;

use crate::common::*;

const FORMATS: [WavSampleFormat; 3] = [
    WavSampleFormat::Int16,
    WavSampleFormat::Int24,
    WavSampleFormat::Float32,
];

/// A different tone on every channel plus a little noise, so that swapped
/// channels and dropped low bits both show up. It peaks below full scale so
/// that nothing clips.
fn test_signal(sample_rate: u32, channels: u16, frames: usize) -> AudioBuffer {
    let frequencies: Vec<f64> = (0..channels)
        .map(|channel| 220.0 * f64::from(channel + 1))
        .collect();
    let mut rng = Rng::new(u64::from(channels) * 1_000 + frames as u64);
    let samples = tones(sample_rate, &frequencies, frames, 0.8)
        .samples()
        .iter()
        .map(|tone| tone + rng.sample() * 0.1)
        .collect();
    AudioBuffer::from_interleaved(sample_rate, channels, samples)
}

/// Rounding moves a sample by up to half a step and dither by up to one more.
fn tolerance(format: WavSampleFormat) -> f32 {
    match format {
        WavSampleFormat::Int16 => 1.5 / 32_768.0,
        WavSampleFormat::Int24 => 1.5 / 8_388_608.0,
        WavSampleFormat::Float32 => 0.0,
    }
}

fn codec_name(format: WavSampleFormat) -> &'static str {
    match format {
        WavSampleFormat::Int16 => "pcm_s16le",
        WavSampleFormat::Int24 => "pcm_s24le",
        WavSampleFormat::Float32 => "pcm_f32le",
    }
}

#[test]
fn every_format_and_channel_count_survives_a_round_trip() {
    let dir = TempDir::new("round-trip");
    // The odd frame count with three 24-bit channels makes an odd-length
    // data chunk, and 20 000 frames span several conversion chunks.
    for (sample_rate, channels, frames) in [
        (44_100, 1, 20_000),
        (48_000, 2, 20_000),
        (96_000, 2, 501),
        (48_000, 3, 333),
        (48_000, 6, 4_000),
        (22_050, 9, 250),
    ] {
        let original = test_signal(sample_rate, channels, frames);
        for format in FORMATS {
            let label = format!("{format:?} {channels} ch {sample_rate} Hz");
            let path = dir.path("round-trip.wav");
            write_wav(&path, &original, format).unwrap();

            let info = probe_file(&path).unwrap();
            assert_eq!(info.sample_rate, sample_rate, "{label}");
            assert_eq!(info.channels, channels, "{label}");
            assert_eq!(info.frames, Some(frames as u64), "{label}");
            assert_eq!(info.codec, codec_name(format), "{label}");

            let decoded = decode_file(&path).unwrap();
            assert_eq!(decoded.sample_rate(), sample_rate, "{label}");
            assert_eq!(decoded.channels(), channels, "{label}");
            assert_eq!(decoded.frames(), frames, "{label}");

            if format == WavSampleFormat::Float32 {
                let bits = |buffer: &AudioBuffer| -> Vec<u32> {
                    buffer.samples().iter().map(|s| s.to_bits()).collect()
                };
                assert_eq!(bits(&decoded), bits(&original), "{label}");
            } else {
                let difference = max_difference(decoded.samples(), original.samples());
                assert!(
                    difference <= tolerance(format),
                    "{label}: off by {difference}"
                );
            }
        }
    }
}

#[test]
fn float_files_keep_values_beyond_full_scale() {
    let dir = TempDir::new("float-range");
    let path = dir.path("hot.wav");
    let samples = vec![
        1.5,
        -3.0,
        1.0,
        -1.0,
        1.0e-30,
        -0.0,
        f32::MAX,
        f32::MIN_POSITIVE,
    ];
    let original = AudioBuffer::from_interleaved(48_000, 2, samples);
    write_wav(&path, &original, WavSampleFormat::Float32).unwrap();
    assert_eq!(decode_file(&path).unwrap().samples(), original.samples());
}

#[test]
fn float_files_store_samples_untouched_and_decode_cleans_them() {
    let dir = TempDir::new("float-nan");
    let path = dir.path("nan.wav");
    let samples = [0.25, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.5];
    let original = AudioBuffer::from_interleaved(48_000, 1, samples.to_vec());
    write_wav(&path, &original, WavSampleFormat::Float32).unwrap();

    // The file holds the exact bits it was given.
    let bytes = fs::read(&path).unwrap();
    let stored = &bytes[bytes.len() - samples.len() * 4..];
    let expected: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    assert_eq!(stored, expected);

    // The decoder hands the engine nothing it cannot mix.
    assert_eq!(
        decode_file(&path).unwrap().samples(),
        [0.25, 0.0, 0.0, 0.0, -0.5]
    );
}

#[test]
fn integer_files_clip_at_full_scale() {
    let dir = TempDir::new("clip");
    let original = AudioBuffer::from_interleaved(48_000, 1, vec![1.0, 2.5, -1.0, -2.5]);
    for (format, steps) in [
        (WavSampleFormat::Int16, 32_768.0_f32),
        (WavSampleFormat::Int24, 8_388_608.0),
    ] {
        let path = dir.path("clipped.wav");
        write_wav(&path, &original, format).unwrap();
        let top = (steps - 1.0) / steps;
        assert_eq!(
            decode_file(&path).unwrap().samples(),
            [top, top, -1.0, -1.0],
            "{format:?}"
        );
    }
}

#[test]
fn integer_silence_stays_within_the_dither() {
    let dir = TempDir::new("silence");
    let path = dir.path("silence.wav");
    let original = AudioBuffer::from_interleaved(48_000, 2, vec![0.0; 2_000]);
    write_wav(&path, &original, WavSampleFormat::Int16).unwrap();
    let decoded = decode_file(&path).unwrap();
    assert!(
        decoded
            .samples()
            .iter()
            .all(|sample| sample.abs() <= 1.0 / 32_768.0)
    );
}

#[test]
fn the_same_audio_always_gives_the_same_bytes() {
    let dir = TempDir::new("deterministic");
    let original = test_signal(48_000, 2, 5_000);
    for format in FORMATS {
        let (first, second) = (dir.path("first.wav"), dir.path("second.wav"));
        write_wav(&first, &original, format).unwrap();
        write_wav(&second, &original, format).unwrap();
        assert_eq!(
            fs::read(&first).unwrap(),
            fs::read(&second).unwrap(),
            "{format:?}"
        );
    }
}

/// Writes `buffer` in one block with the dither started from `seed`.
fn write_seeded(
    path: &std::path::Path,
    buffer: &AudioBuffer,
    format: WavSampleFormat,
    seed: u64,
) -> Vec<u8> {
    let mut writer = WavWriter::create(path, buffer.sample_rate(), buffer.channels(), format)
        .unwrap()
        .with_dither_seed(seed);
    writer.write(buffer.samples()).unwrap();
    writer.finalize().unwrap();
    fs::read(path).unwrap()
}

#[test]
fn the_dither_seed_changes_integer_output_only() {
    let dir = TempDir::new("seed");
    let path = dir.path("seeded.wav");
    let original = test_signal(48_000, 2, 5_000);
    for format in FORMATS {
        let one = write_seeded(&path, &original, format, 1);
        let one_again = write_seeded(&path, &original, format, 1);
        let two = write_seeded(&path, &original, format, 2);
        assert_eq!(one, one_again, "{format:?}");
        assert_eq!(one != two, format != WavSampleFormat::Float32, "{format:?}");
    }
}

#[test]
fn streaming_in_odd_blocks_equals_writing_at_once() {
    let dir = TempDir::new("streaming");
    for channels in [1, 2, 6] {
        let original = test_signal(48_000, channels, 30_011);
        let width = usize::from(channels);
        for format in FORMATS {
            let whole = dir.path("whole.wav");
            write_wav(&whole, &original, format).unwrap();

            let streamed = dir.path("streamed.wav");
            let mut writer = WavWriter::create(&streamed, 48_000, channels, format).unwrap();
            let mut rng = Rng::new(99);
            let mut rest = original.samples();
            // Block lengths in frames: fixed awkward sizes first, then random
            // ones, some of them empty and some longer than a conversion chunk.
            let mut sizes = vec![1, 3, 7, 127, 1_000, 0, 9_973];
            while !rest.is_empty() {
                let frames = sizes.pop().unwrap_or_else(|| rng.below(5_000));
                let (block, tail) = rest.split_at((frames * width).min(rest.len()));
                writer.write(block).unwrap();
                rest = tail;
            }
            writer.finalize().unwrap();

            assert_eq!(
                fs::read(&streamed).unwrap(),
                fs::read(&whole).unwrap(),
                "{format:?} {channels} ch"
            );
        }
    }
}

#[test]
fn an_empty_wav_is_valid_but_holds_no_audio() {
    let dir = TempDir::new("empty-wav");
    let path = dir.path("empty.wav");
    let empty = AudioBuffer::from_interleaved(44_100, 2, Vec::new());
    write_wav(&path, &empty, WavSampleFormat::Int16).unwrap();

    assert_eq!(fs::read(&path).unwrap().len(), 44);
    let info = probe_file(&path).unwrap();
    assert_eq!((info.sample_rate, info.channels), (44_100, 2));
    assert_eq!(info.frames, Some(0));
    assert!(matches!(decode_file(&path), Err(CodecError::NoAudio)));
}
