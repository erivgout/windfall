//! Feeds the decoder empty, foreign, cut-off and damaged input. Every call
//! must come back with a buffer or an error; a panic fails the test.

use std::fs;

use windfall_codec::{
    CodecError, DecodeOptions, decode_bytes, decode_bytes_with, decode_file, probe_bytes,
    probe_file,
};
use windfall_core::AudioBuffer;

use crate::common::*;

/// Every fixture's name, bytes and full decode.
fn fixtures() -> Vec<(String, Vec<u8>, AudioBuffer)> {
    fixture_files()
        .into_iter()
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let bytes = fs::read(&path).unwrap();
            let decoded = decode_file(&path).unwrap();
            (name, bytes, decoded)
        })
        .collect()
}

/// Decodes and probes `bytes`. The results may be errors, but a buffer that
/// does come back has to be one the engine can use.
fn decode_any(bytes: &[u8], hint: Option<&str>) -> Result<AudioBuffer, CodecError> {
    let _ = probe_bytes(bytes, hint);
    let decoded = decode_bytes(bytes, hint);
    if let Ok(buffer) = &decoded {
        assert!(buffer.frames() > 0);
        assert!(buffer.sample_rate() > 0);
        assert!(buffer.channels() > 0);
        assert!(buffer.samples().iter().all(|sample| sample.is_finite()));
    }
    decoded
}

#[test]
fn empty_input_holds_no_audio() {
    let dir = TempDir::new("empty");
    let path = dir.path("empty.wav");
    fs::write(&path, b"").unwrap();

    assert!(matches!(decode_file(&path), Err(CodecError::NoAudio)));
    assert!(matches!(probe_file(&path), Err(CodecError::NoAudio)));
    assert!(matches!(
        decode_bytes(&[], Some("wav")),
        Err(CodecError::NoAudio)
    ));
    assert!(matches!(probe_bytes(&[], None), Err(CodecError::NoAudio)));
}

#[test]
fn a_missing_file_or_a_folder_is_an_io_error() {
    let dir = TempDir::new("missing");
    let missing = dir.path("no-such-file.wav");
    assert!(matches!(decode_file(&missing), Err(CodecError::Io(_))));
    assert!(matches!(probe_file(&missing), Err(CodecError::Io(_))));

    let folder = dir.path("folder.wav");
    fs::create_dir(&folder).unwrap();
    assert!(decode_file(&folder).is_err());
    assert!(probe_file(&folder).is_err());
}

#[test]
fn files_that_are_not_audio_are_unsupported() {
    let script = fs::read(fixture("generate.sh")).unwrap();
    let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
    let cases: [(&str, &[u8]); 6] = [
        ("a shell script", &script),
        (
            "json",
            br#"{"name": "My Song", "tempo": 140.0, "channels": []}"#,
        ),
        ("a png header", &png),
        ("zeros", &[0; 4096]),
        ("one byte", b"R"),
        (
            "an avi",
            b"RIFF\x24\x00\x00\x00AVI LIST\x10\x00\x00\x00hdrlavih",
        ),
    ];
    for (what, bytes) in cases {
        for hint in [None, Some("wav"), Some("mp3")] {
            let error = decode_any(bytes, hint).unwrap_err();
            assert!(
                matches!(error, CodecError::UnsupportedFormat(_)),
                "{what}: {error:?}"
            );
            let error = probe_bytes(bytes, hint).unwrap_err();
            assert!(
                matches!(error, CodecError::UnsupportedFormat(_)),
                "{what}: {error:?}"
            );
        }
    }
}

#[test]
fn error_messages_read_as_sentences() {
    let not_audio = decode_bytes(b"plain text, not audio", None).unwrap_err();
    assert_eq!(
        not_audio.to_string(),
        "this is not a supported audio file (the format was not recognized)"
    );

    // A WAV holding IMA ADPCM, which is not among the codecs built in.
    let mut adpcm = 0x0011_u16.to_le_bytes().to_vec();
    adpcm.extend(1_u16.to_le_bytes());
    adpcm.extend(22_050_u32.to_le_bytes());
    adpcm.extend(11_100_u32.to_le_bytes());
    // 256-byte blocks of 4-bit samples, 505 samples to a block.
    adpcm.extend(256_u16.to_le_bytes());
    adpcm.extend(4_u16.to_le_bytes());
    adpcm.extend(2_u16.to_le_bytes());
    adpcm.extend(505_u16.to_le_bytes());
    let bytes = riff_wave(&[chunk(b"fmt ", &adpcm), chunk(b"data", &[0; 512])]);
    let other_codec = "this is not a supported audio file (it uses a codec Windfall cannot read)";
    assert_eq!(
        decode_bytes(&bytes, None).unwrap_err().to_string(),
        other_codec
    );
    assert_eq!(
        probe_bytes(&bytes, None).unwrap_err().to_string(),
        other_codec
    );

    let cut_short = decode_bytes(b"RIFF\x24\x00\x00\x00WAVEfmt ", None).unwrap_err();
    assert_eq!(
        cut_short.to_string(),
        "the audio file is damaged (the file ends unexpectedly)"
    );
    assert_eq!(
        CodecError::NoAudio.to_string(),
        "the file contains no audio"
    );
}

#[test]
fn random_bytes_never_panic() {
    let mut rng = Rng::new(0x5EED);
    // Bare noise, and noise behind each format's opening bytes so that the
    // readers get past their first check.
    let openings: [&[u8]; 8] = [
        b"",
        b"RIFF\xF0\xFF\x00\x00WAVEfmt ",
        b"RIFF\xF0\xFF\x00\x00WAVEdata",
        b"FORM\x00\x00\xFF\xF0AIFFCOMM",
        b"fLaC\x00\x00\x00\x22",
        b"OggS\x00\x02",
        b"ID3\x04\x00\x00\x00\x00\x00\x10",
        b"\xFF\xFB\x90\x00",
    ];
    for opening in openings {
        for _ in 0..150 {
            let mut bytes = opening.to_vec();
            let noise = rng.below(6_000);
            bytes.extend((0..noise).map(|_| rng.next() as u8));
            let _ = decode_any(&bytes, None);
        }
    }
}

#[test]
fn cut_off_files_give_their_first_part_or_an_error() {
    for (name, bytes, full) in fixtures() {
        let width = usize::from(full.channels());
        let mut cuts: Vec<usize> = (1..96).collect();
        cuts.extend((1..64).map(|step| bytes.len() * step / 64));
        cuts.extend([bytes.len() - 3, bytes.len() - 2, bytes.len() - 1]);

        for cut in cuts {
            let Ok(part) = decode_any(&bytes[..cut], None) else {
                continue;
            };
            assert_eq!(part.sample_rate(), full.sample_rate(), "{name} at {cut}");
            assert_eq!(part.channels(), full.channels(), "{name} at {cut}");
            assert!(part.frames() <= full.frames(), "{name} at {cut}");

            // What comes back is exactly the start of the real audio.
            let same_span = &full.samples()[..part.frames() * width];
            assert!(part.samples() == same_span, "{name} at {cut}");
        }

        // Losing the last byte costs at most the last block of a compressed
        // file, never the whole file.
        let nearly_all = decode_any(&bytes[..bytes.len() - 1], None).unwrap();
        assert!(
            nearly_all.frames() * 10 >= full.frames() * 7,
            "{name}: {} of {} frames",
            nearly_all.frames(),
            full.frames()
        );
    }
}

#[test]
fn damaged_files_never_panic() {
    let mut rng = Rng::new(0xDA_4A6E);
    for (_, bytes, _) in fixtures() {
        for round in 0..80 {
            let mut damaged = bytes.clone();
            // Half of the rounds hit the headers, where the parsers decide
            // how much to trust the rest.
            let reach = if round % 2 == 0 { 96 } else { bytes.len() };
            let start = rng.below(reach);
            let length = (1 + rng.below(48)).min(damaged.len() - start);
            for byte in &mut damaged[start..start + length] {
                *byte = match round % 3 {
                    0 => rng.next() as u8,
                    1 => 0x00,
                    _ => 0xFF,
                };
            }
            let _ = decode_any(&damaged, None);
        }
    }
}

#[test]
fn a_second_stream_glued_on_is_left_out() {
    // One buffer has one sample rate and channel count, so a file that
    // switches part-way gives only its first stream.
    for (first, second) in [
        ("mp3_44k_stereo.mp3", "mp3_22k_mono.mp3"),
        ("mp3_22k_mono.mp3", "mp3_44k_stereo.mp3"),
        ("vorbis_44k_stereo.ogg", "vorbis_48k_mono.ogg"),
        ("vorbis_48k_mono.ogg", "vorbis_44k_stereo.ogg"),
    ] {
        let alone = decode_file(fixture(first)).unwrap();
        let mut bytes = fs::read(fixture(first)).unwrap();
        bytes.extend(fs::read(fixture(second)).unwrap());

        let glued = decode_any(&bytes, None).unwrap();
        assert_eq!(glued.sample_rate(), alone.sample_rate(), "{first}");
        assert_eq!(glued.channels(), alone.channels(), "{first}");
        assert!(glued.samples() == alone.samples(), "{first}");
    }
}

/// One RIFF chunk, padded to an even length as the format requires.
fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut bytes = id.to_vec();
    bytes.extend((body.len() as u32).to_le_bytes());
    bytes.extend(body);
    if body.len() % 2 == 1 {
        bytes.push(0);
    }
    bytes
}

fn riff_wave(chunks: &[Vec<u8>]) -> Vec<u8> {
    let body: Vec<u8> = chunks.concat();
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((body.len() as u32 + 4).to_le_bytes());
    bytes.extend(b"WAVE");
    bytes.extend(body);
    bytes
}

/// Overwrites the length field of a chunk or of the whole RIFF file.
fn claim_length(header: &mut [u8], length: u32) {
    header[4..8].copy_from_slice(&length.to_le_bytes());
}

/// A 16-bit PCM `fmt ` chunk.
fn fmt_chunk(channels: u16, sample_rate: u32) -> Vec<u8> {
    let mut body = 1_u16.to_le_bytes().to_vec();
    body.extend(channels.to_le_bytes());
    body.extend(sample_rate.to_le_bytes());
    body.extend((sample_rate * u32::from(channels) * 2).to_le_bytes());
    body.extend((channels * 2).to_le_bytes());
    body.extend(16_u16.to_le_bytes());
    chunk(b"fmt ", &body)
}

/// A rising ramp, one step per sample, as 16-bit PCM bytes.
fn ramp(samples: i16) -> Vec<u8> {
    (0..samples)
        .flat_map(|value| (value * 100).to_le_bytes())
        .collect()
}

fn ramp_decoded(samples: i16) -> Vec<f32> {
    (0..samples)
        .map(|value| f32::from(value * 100) / 32_768.0)
        .collect()
}

#[test]
fn chunks_around_the_audio_are_skipped() {
    let bytes = riff_wave(&[
        chunk(b"JUNK", &[0; 28]),
        chunk(b"bext", b"odd length body"),
        fmt_chunk(2, 32_000),
        chunk(b"cue ", &[0; 4]),
        chunk(b"data", &ramp(200)),
        chunk(b"LIST", b"INFOISFT\x06\x00\x00\x00tests\x00"),
        chunk(b"id3 ", b"ID3 not really"),
    ]);
    let buffer = decode_any(&bytes, None).unwrap();
    assert_eq!(buffer.sample_rate(), 32_000);
    assert_eq!(buffer.channels(), 2);
    assert_eq!(buffer.samples(), ramp_decoded(200));
    assert_eq!(probe_bytes(&bytes, None).unwrap().frames, Some(100));
}

#[test]
fn a_header_that_overstates_the_length_gives_the_audio_that_is_there() {
    // A recorder that was cut off: the header was written for almost 2 GB
    // of samples, and 600 bytes follow.
    let mut data = chunk(b"data", &ramp(300));
    claim_length(&mut data, 0x7FFF_FFF0);
    let mut bytes = riff_wave(&[fmt_chunk(1, 8_000), data]);
    claim_length(&mut bytes, 0x7FFF_FFF0 + 36);

    assert_eq!(probe_bytes(&bytes, None).unwrap().frames, Some(0x3FFF_FFF8));
    let buffer = decode_any(&bytes, None).unwrap();
    assert_eq!(buffer.samples(), ramp_decoded(300));

    // The claim is not believed, so only the real audio counts to the limit.
    let fits = DecodeOptions {
        max_decoded_bytes: 300 * 4,
    };
    assert_eq!(
        decode_bytes_with(&bytes, None, &fits).unwrap().frames(),
        300
    );
    let too_small = DecodeOptions {
        max_decoded_bytes: 300 * 4 - 1,
    };
    assert!(matches!(
        decode_bytes_with(&bytes, None, &too_small),
        Err(CodecError::TooLarge { .. })
    ));
}

#[test]
fn headers_with_impossible_values_are_errors() {
    let audio = chunk(b"data", &ramp(64));
    let mut no_bits = fmt_chunk(2, 44_100);
    no_bits[22..24].copy_from_slice(&0_u16.to_le_bytes());

    let cases = [
        (
            "no channels",
            riff_wave(&[fmt_chunk(0, 44_100), audio.clone()]),
        ),
        (
            "no sample rate",
            riff_wave(&[fmt_chunk(2, 0), audio.clone()]),
        ),
        ("no bits", riff_wave(&[no_bits, audio.clone()])),
        ("no format", riff_wave(std::slice::from_ref(&audio))),
        ("no data", riff_wave(&[fmt_chunk(2, 44_100)])),
        (
            "empty data",
            riff_wave(&[fmt_chunk(2, 44_100), chunk(b"data", &[])]),
        ),
        (
            "half a frame",
            riff_wave(&[fmt_chunk(2, 44_100), chunk(b"data", &[1, 2])]),
        ),
    ];
    for (what, bytes) in cases {
        assert!(decode_any(&bytes, None).is_err(), "{what} decoded");
    }
}
