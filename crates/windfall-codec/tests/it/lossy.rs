use crate::common::*;
use std::{fs, process::Command};
use windfall_codec::{Encoder, EncoderSettings, Mp3Channels, Mp3Rate, Mp3Settings, decode_file};

fn settings() -> Vec<EncoderSettings> {
    let mut formats = vec![
        EncoderSettings::Vorbis { quality: -1.0 },
        EncoderSettings::Vorbis { quality: 6.0 },
        EncoderSettings::Vorbis { quality: 10.0 },
    ];
    for rate in [
        Mp3Rate::Cbr(128),
        Mp3Rate::Cbr(192),
        Mp3Rate::Cbr(256),
        Mp3Rate::Cbr(320),
        Mp3Rate::Vbr(0),
        Mp3Rate::Vbr(5),
        Mp3Rate::Vbr(9),
    ] {
        formats.push(EncoderSettings::Mp3 {
            settings: Mp3Settings {
                rate,
                channels: Mp3Channels::JointStereo,
            },
        });
    }
    formats
}

#[test]
fn lossy_streams_are_gapless_deterministic_and_keep_the_tones() {
    let dir = TempDir::new("lossy-gapless");
    for settings in settings() {
        for rate in [44_100, 48_000] {
            let original = tones(rate, &[440.0, 1000.0], 24_013, 0.5);
            let path = dir.path(&format!("out.{}", settings.format().extension()));
            let mut encoder = Encoder::open(&path, &settings, rate, 2).unwrap();
            for block in original.samples().chunks(2 * 137) {
                encoder.write(block).unwrap();
            }
            encoder.finalize().unwrap();
            let first = fs::read(&path).unwrap();
            let decoded = decode_file(&path).unwrap();
            assert_eq!(
                decoded.frames(),
                original.frames(),
                "{settings:?} at {rate}"
            );
            assert_eq!((decoded.sample_rate(), decoded.channels()), (rate, 2));
            for side in 0..2 {
                assert_eq!(
                    dominant_frequency(&channel(&decoded, side), rate),
                    [440.0, 1000.0][side]
                );
            }
            let mut encoder = Encoder::open(&path, &settings, rate, 2).unwrap();
            encoder.write(original.samples()).unwrap();
            encoder.finalize().unwrap();
            assert_eq!(fs::read(&path).unwrap(), first, "{settings:?}");
        }
    }
}

#[test]
fn mono_short_files_and_abandoned_streams_are_safe() {
    let dir = TempDir::new("lossy-short");
    for settings in [
        EncoderSettings::Vorbis { quality: 6.0 },
        EncoderSettings::Mp3 {
            settings: Mp3Settings {
                rate: Mp3Rate::Vbr(2),
                channels: Mp3Channels::Mono,
            },
        },
    ] {
        for frames in [1, 17, 1151, 4097] {
            let path = dir.path("short");
            let mut encoder = Encoder::open(&path, &settings, 48_000, 1).unwrap();
            encoder.write(&vec![0.0; frames]).unwrap();
            encoder.finalize().unwrap();
            assert_eq!(decode_file(&path).unwrap().frames(), frames, "{settings:?}");
            fs::remove_file(&path).unwrap();
            let mut encoder = Encoder::open(&path, &settings, 48_000, 1).unwrap();
            encoder.write(&[0.25; 4097]).unwrap();
            drop(encoder);
            assert!(dir.entries().is_empty());
        }
    }
}

#[test]
fn supported_mp3_rates_keep_their_rate_and_length() {
    let dir = TempDir::new("mp3-rates");
    for &rate in windfall_codec::MP3_SAMPLE_RATES {
        for channels in [
            Mp3Channels::Mono,
            Mp3Channels::Stereo,
            Mp3Channels::JointStereo,
        ] {
            let count = if channels == Mp3Channels::Mono { 1 } else { 2 };
            let audio = tones(rate, &vec![440.0; count], 12_345, 0.4);
            let settings = EncoderSettings::Mp3 {
                settings: Mp3Settings {
                    rate: Mp3Rate::Vbr(2),
                    channels,
                },
            };
            let path = dir.path("rate.mp3");
            let mut encoder = Encoder::open(&path, &settings, rate, count as u16).unwrap();
            encoder.write(audio.samples()).unwrap();
            encoder.finalize().unwrap();
            let decoded = decode_file(&path).unwrap();
            assert_eq!(
                (decoded.sample_rate(), decoded.channels(), decoded.frames()),
                (rate, count as u16, audio.frames())
            );
        }
    }
}

#[test]
fn vorbis_rate_boundaries_and_quality_extremes_decode_gaplessly() {
    let dir = TempDir::new("vorbis-rates");
    for rate in [8_000, 22_050, 44_100, 48_000, 96_000, 192_000] {
        for channels in [1_u16, 2] {
            for quality in [-1.0, 6.0, 10.0] {
                let audio = tones(rate, &vec![440.0; usize::from(channels)], 4097, 0.4);
                let settings = EncoderSettings::Vorbis { quality };
                let path = dir.path("rate.ogg");
                let mut encoder = Encoder::open(&path, &settings, rate, channels).unwrap();
                encoder.write(audio.samples()).unwrap();
                encoder.finalize().unwrap();
                let decoded = decode_file(&path).unwrap();
                assert_eq!(
                    (decoded.sample_rate(), decoded.channels(), decoded.frames()),
                    (rate, channels, audio.frames())
                );
            }
        }
    }
}

#[test]
fn quality_presets_order_file_sizes_and_a_sweep_keeps_its_spectrum() {
    let dir = TempDir::new("lossy-spectrum");
    let mut phase = 0.0_f64;

    let samples: Vec<f32> = (0..96_000)
        .flat_map(|frame| {
            let frequency = 100.0 + 3900.0 * frame as f64 / 96_000.0;
            phase += std::f64::consts::TAU * frequency / 48_000.0;
            let sample = (phase.sin() * 0.4) as f32;
            [sample, sample]
        })
        .collect();
    let choices = [
        EncoderSettings::Vorbis { quality: -1.0 },
        EncoderSettings::Vorbis { quality: 6.0 },
        EncoderSettings::Vorbis { quality: 10.0 },
        EncoderSettings::Mp3 {
            settings: Mp3Settings {
                rate: Mp3Rate::Vbr(9),
                channels: Mp3Channels::JointStereo,
            },
        },
        EncoderSettings::Mp3 {
            settings: Mp3Settings {
                rate: Mp3Rate::Vbr(5),
                channels: Mp3Channels::JointStereo,
            },
        },
        EncoderSettings::Mp3 {
            settings: Mp3Settings {
                rate: Mp3Rate::Vbr(0),
                channels: Mp3Channels::JointStereo,
            },
        },
    ];
    let mut sizes = Vec::new();
    for settings in choices {
        let path = dir.path("sweep");
        let mut encoder = Encoder::open(&path, &settings, 48_000, 2).unwrap();
        encoder.write(&samples).unwrap();
        encoder.finalize().unwrap();
        sizes.push(fs::metadata(&path).unwrap().len());
        let decoded = decode_file(&path).unwrap();
        assert_eq!(decoded.samples().len(), samples.len());
        // Compare energy in eight time windows. The sweep's frequency
        // climbs through each window; lost bands would lose their energy.
        for (input, output) in samples.chunks(24_000).zip(decoded.samples().chunks(24_000)) {
            let energy =
                |samples: &[f32]| samples.iter().map(|&s| f64::from(s).powi(2)).sum::<f64>();
            let ratio = energy(output) / energy(input);
            assert!(
                (0.75..1.25).contains(&ratio),
                "{settings:?}: spectrum energy ratio {ratio}"
            );
            let crossings = |samples: &[f32]| {
                samples
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|pair| pair[0])
                    .collect::<Vec<_>>()
                    .windows(2)
                    .filter(|pair| pair[0] < 0.0 && pair[1] >= 0.0)
                    .count()
            };
            let frequency_ratio = crossings(output) as f64 / crossings(input) as f64;
            assert!(
                (0.9..1.1).contains(&frequency_ratio),
                "{settings:?}: sweep frequency ratio {frequency_ratio}"
            );
        }
    }
    assert!(
        sizes[0] < sizes[1] && sizes[1] < sizes[2],
        "Vorbis sizes {sizes:?}"
    );
    assert!(
        sizes[3] < sizes[4] && sizes[4] < sizes[5],
        "MP3 sizes {sizes:?}"
    );
}

#[test]
fn invalid_lossy_settings_are_rejected_without_creating_files() {
    let dir = TempDir::new("lossy-invalid");
    for (settings, rate, channels) in [
        (EncoderSettings::Vorbis { quality: f32::NAN }, 48_000, 2),
        (EncoderSettings::Vorbis { quality: 11.0 }, 48_000, 2),
        (EncoderSettings::Vorbis { quality: 6.0 }, 48_000, 6),
        (
            EncoderSettings::Mp3 {
                settings: Mp3Settings::default(),
            },
            96_000,
            2,
        ),
        (
            EncoderSettings::Mp3 {
                settings: Mp3Settings::default(),
            },
            24_000,
            2,
        ),
        (
            EncoderSettings::Mp3 {
                settings: Mp3Settings {
                    rate: Mp3Rate::Vbr(10),
                    ..Mp3Settings::default()
                },
            },
            48_000,
            2,
        ),
    ] {
        assert!(settings.check(rate, channels).is_err());
        assert!(Encoder::open(dir.path("invalid"), &settings, rate, channels).is_err());
        assert!(dir.entries().is_empty());
    }
}

#[test]
#[ignore = "needs ffmpeg and ffprobe on PATH"]
fn ffmpeg_confirms_gapless_length_and_requested_cbr() {
    let dir = TempDir::new("lossy-ffmpeg");
    for settings in settings() {
        let audio = tones(48_000, &[440.0, 1000.0], 96_013, 0.5);
        let path = dir.path(&format!("out.{}", settings.format().extension()));
        let mut encoder = Encoder::open(&path, &settings, 48_000, 2).unwrap();
        encoder.write(audio.samples()).unwrap();
        encoder.finalize().unwrap();
        let output = Command::new("ffmpeg")
            .args(["-v", "error", "-err_detect", "explode", "-i"])
            .arg(&path)
            .args(["-f", "f32le", "-acodec", "pcm_f32le", "-"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout.len(), audio.frames() * 2 * 4, "{settings:?}");
        if let EncoderSettings::Mp3 {
            settings:
                Mp3Settings {
                    rate: Mp3Rate::Cbr(bitrate),
                    ..
                },
        } = settings
        {
            let probe = Command::new("ffprobe")
                .args([
                    "-v",
                    "error",
                    "-select_streams",
                    "a:0",
                    "-show_entries",
                    "stream=bit_rate",
                    "-of",
                    "default=nw=1:nk=1",
                ])
                .arg(&path)
                .output()
                .unwrap();
            assert!(probe.status.success());
            assert_eq!(
                String::from_utf8(probe.stdout)
                    .unwrap()
                    .trim()
                    .parse::<u32>()
                    .unwrap(),
                u32::from(bitrate) * 1000
            );
        }
    }
}
