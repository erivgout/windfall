//! Checks that every factory sound is a well-formed one-shot and sounds
//! like its category, as far as numbers can tell.

use std::collections::HashSet;
use std::sync::OnceLock;

use crate::analysis::{Spectrum, power_at};
use crate::dsp::{self, Audio};
use crate::{BASS_ROOT_HZ, Category, DEFAULT_KIT, Rendered, SAMPLE_RATE, Sound, manifest};

const RATE: f64 = SAMPLE_RATE as f64;

/// One sound rendered every way the tests look at it.
struct Render {
    sound: &'static Sound,
    /// Length of the render before the tail was trimmed, in frames.
    raw_frames: usize,
    /// The trimmed and normalized render, before rounding to 24 bits.
    finished: Audio,
    /// The spectrum of each channel of `finished`.
    spectra: Vec<Spectrum>,
    rendered: Rendered,
}

/// Every sound, rendered once and shared by all tests.
fn renders() -> &'static [Render] {
    static RENDERS: OnceLock<Vec<Render>> = OnceLock::new();
    RENDERS.get_or_init(|| {
        manifest()
            .iter()
            .map(|sound| {
                let raw = (sound.synth)();
                let raw_frames = raw.frames();
                let finished = dsp::finish(raw, sound.peak_db);
                let spectra = finished
                    .channels()
                    .iter()
                    .map(|channel| Spectrum::of(channel, RATE))
                    .collect();
                Render {
                    sound,
                    raw_frames,
                    finished,
                    spectra,
                    rendered: sound.render(),
                }
            })
            .collect()
    })
}

fn seconds(render: &Render) -> f64 {
    render.finished.frames() as f64 / RATE
}

/// Share of each channel's energy between `low` and `high` hertz; the
/// smallest of the channels.
fn share(render: &Render, low: f64, high: f64) -> f64 {
    render
        .spectra
        .iter()
        .map(|spectrum| spectrum.share(low, high))
        .fold(f64::INFINITY, f64::min)
}

#[test]
fn manifest_names_and_paths_are_unique_and_match() {
    let sounds = manifest();
    assert!(sounds.len() >= 30);
    let names: HashSet<_> = sounds.iter().map(|sound| sound.name).collect();
    let paths: HashSet<_> = sounds.iter().map(|sound| sound.path).collect();
    assert_eq!(names.len(), sounds.len(), "duplicate name");
    assert_eq!(paths.len(), sounds.len(), "duplicate path");
    for sound in sounds {
        let expected = format!("{}/{}.wav", sound.category.folder(), sound.name);
        assert_eq!(sound.path, expected);
    }
}

#[test]
fn names_are_plain_words_we_own() {
    // Model numbers and maker names of other companies' drum machines,
    // samplers and DAWs. A name token that matches one of these is a mistake.
    const TAKEN: [&str; 12] = [
        "tr", "cr", "sp", "mpc", "fpc", "fl", "linn", "dmx", "sds", "rx", "sr", "oberheim",
    ];
    for sound in manifest() {
        for token in sound.name.split(' ') {
            let is_word = token.chars().all(|c| c.is_ascii_alphabetic());
            // A single digit tells variants apart; anything longer would read
            // as a model number.
            let is_variant = token.len() == 1 && token.chars().all(|c| c.is_ascii_digit());
            assert!(is_word || is_variant, "{:?}", sound.name);
            assert!(
                !TAKEN.contains(&token.to_lowercase().as_str()),
                "{:?}",
                sound.name
            );
        }
    }
}

#[test]
fn every_category_has_sounds() {
    let count = |category| {
        manifest()
            .iter()
            .filter(|sound| sound.category == category)
            .count()
    };
    assert!(count(Category::Kick) >= 5);
    assert!(count(Category::Snare) >= 4);
    assert!(count(Category::Clap) >= 2);
    assert!(count(Category::Hat) >= 5);
    assert!(count(Category::Cymbal) >= 2);
    assert!(count(Category::Tom) >= 3);
    assert!(count(Category::Percussion) >= 6);
    assert!(count(Category::Bass) >= 2);
}

#[test]
fn default_kit_is_kick_clap_hat_snare_from_the_manifest() {
    let categories: Vec<Category> = DEFAULT_KIT
        .iter()
        .map(|&(name, path)| {
            let sound = manifest()
                .iter()
                .find(|sound| sound.path == path)
                .unwrap_or_else(|| panic!("{path} is not in the manifest"));
            assert_eq!(sound.name, name);
            sound.category
        })
        .collect();
    assert_eq!(
        categories,
        [
            Category::Kick,
            Category::Clap,
            Category::Hat,
            Category::Snare
        ]
    );
}

#[test]
fn rendering_twice_gives_identical_samples() {
    for render in renders() {
        assert_eq!(
            render.sound.render(),
            render.rendered,
            "{}",
            render.sound.name
        );
    }
}

#[test]
fn no_two_sounds_are_the_same() {
    let distinct: HashSet<&[i32]> = renders()
        .iter()
        .map(|render| render.rendered.samples())
        .collect();
    assert_eq!(distinct.len(), renders().len());
}

#[test]
fn samples_are_finite_and_fit_24_bits() {
    for render in renders() {
        let name = render.sound.name;
        for channel in render.finished.channels() {
            assert!(channel.iter().all(|sample| sample.is_finite()), "{name}");
        }
        assert!(
            render
                .rendered
                .samples()
                .iter()
                .all(|sample| (-8_388_608..=8_388_607).contains(sample)),
            "{name}"
        );
        assert_eq!(render.rendered.frames(), render.finished.frames(), "{name}");
    }
}

#[test]
fn peaks_sit_at_their_targets() {
    for render in renders() {
        let name = render.sound.name;
        let target = render.sound.peak_db;
        // About -1 dBFS, with the bright sounds a few decibels lower.
        assert!((-8.0..=-1.0).contains(&target), "{name}: {target}");
        let peak = render
            .rendered
            .samples()
            .iter()
            .map(|sample| sample.unsigned_abs())
            .max()
            .unwrap();
        let peak_db = 20.0 * (peak as f64 / 8_388_607.0).log10();
        assert!((peak_db - target).abs() < 0.001, "{name}: {peak_db} dBFS");
    }
}

#[test]
fn hats_and_shakers_are_quieter_than_the_drums() {
    let peak = |name: &str| {
        manifest()
            .iter()
            .find(|sound| sound.name == name)
            .unwrap()
            .peak_db
    };
    for sound in manifest() {
        if sound.category == Category::Hat {
            assert!(sound.peak_db <= peak("Kick Punch") - 3.0, "{}", sound.name);
        }
    }
    assert!(peak("Shaker") <= peak("Snare Tight") - 3.0);
}

#[test]
fn there_is_no_dc_offset() {
    for render in renders() {
        for channel in render.finished.channels() {
            let mean = channel.iter().sum::<f64>() / channel.len() as f64;
            assert!(mean.abs() < 5e-4, "{}: mean {mean}", render.sound.name);
        }
    }
}

#[test]
fn sounds_start_on_their_transient() {
    for render in renders() {
        let name = render.sound.name;
        let peak = render.finished.peak();
        let onset = render
            .finished
            .channels()
            .iter()
            .filter_map(|channel| channel.iter().position(|s| s.abs() >= 0.1 * peak))
            .min()
            .unwrap();
        // 48 frames is one millisecond.
        assert!(onset < 48, "{name}: onset at frame {onset}");
        for channel in render.finished.channels() {
            assert_eq!(channel[0], 0.0, "{name}");
        }
    }
}

#[test]
fn sounds_end_in_silence_without_a_cut() {
    for render in renders() {
        let name = render.sound.name;
        let peak = render.finished.peak();
        for channel in render.finished.channels() {
            assert_eq!(channel[channel.len() - 1], 0.0, "{name}");
            let last_millisecond = &channel[channel.len() - 48..];
            assert!(
                last_millisecond.iter().all(|s| s.abs() < 1e-4 * peak),
                "{name}"
            );
        }
        // The tail faded out by itself before the render buffer ran out, so
        // nothing was chopped off.
        assert!(render.finished.frames() + 480 < render.raw_frames, "{name}");
    }
}

#[test]
fn tails_are_trimmed() {
    for render in renders() {
        let peak = render.finished.peak();
        let last_audible = render
            .finished
            .channels()
            .iter()
            .filter_map(|channel| channel.iter().rposition(|s| s.abs() > 1e-3 * peak))
            .max()
            .unwrap();
        let silent_ms = (render.finished.frames() - last_audible) as f64 / 48.0;
        // The closing fade pulls the last 10 ms under -60 dB, and in a noisy
        // tail the last peak before it can sit a few percent further back.
        let allowed_ms = 15.0 + 50.0 * seconds(render);
        assert!(
            silent_ms < allowed_ms,
            "{}: {silent_ms} ms",
            render.sound.name
        );
    }
}

#[test]
fn durations_suit_the_category() {
    for render in renders() {
        let range = match render.sound.category {
            Category::Kick => 0.15..1.6,
            Category::Snare => 0.15..0.8,
            Category::Clap => 0.1..0.7,
            Category::Hat => 0.03..1.8,
            Category::Cymbal => 1.0..3.0,
            Category::Tom => 0.4..1.3,
            Category::Percussion => 0.04..0.7,
            Category::Bass => 0.8..2.5,
        };
        let seconds = seconds(render);
        assert!(
            range.contains(&seconds),
            "{}: {seconds} s",
            render.sound.name
        );
    }
}

#[test]
fn energy_sits_in_the_band_of_the_category() {
    for render in renders() {
        let name = render.sound.name;
        // (low, high, least share of the energy that must lie between them)
        let bands: &[(f64, f64, f64)] = match render.sound.category {
            Category::Kick => &[(0.0, 200.0, 0.7), (30.0, 120.0, 0.3)],
            // A snare needs both its drum and its wires.
            Category::Snare => &[(100.0, 500.0, 0.15), (2_000.0, 12_000.0, 0.15)],
            Category::Clap => &[(500.0, 8_000.0, 0.8)],
            Category::Hat => &[(5_000.0, 24_000.0, 0.85)],
            Category::Cymbal => &[(2_000.0, 24_000.0, 0.85)],
            Category::Tom => &[(60.0, 500.0, 0.9)],
            Category::Percussion => &[(200.0, 24_000.0, 0.95)],
            Category::Bass => &[(40.0, 500.0, 0.85)],
        };
        for &(low, high, least) in bands {
            let share = share(render, low, high);
            assert!(share >= least, "{name}: {share:.3} in {low}..{high} Hz");
        }
    }
}

#[test]
fn kicks_get_lower_and_longer_from_tight_to_deep() {
    let find = |name: &str| {
        renders()
            .iter()
            .find(|render| render.sound.name == name)
            .unwrap()
    };
    let (tight, punch, deep) = (find("Kick Tight"), find("Kick Punch"), find("Kick Deep"));
    assert!(seconds(tight) < seconds(punch) && seconds(punch) < seconds(deep));
    assert!(share(tight, 0.0, 60.0) < share(punch, 0.0, 60.0));
    assert!(share(punch, 0.0, 60.0) < share(deep, 0.0, 60.0));
    // Only the hard kick is distorted enough to reach the midrange.
    let mids = |render| share(render, 500.0, 5_000.0);
    assert!(mids(find("Kick Hard")) > 4.0 * mids(punch));
}

#[test]
fn open_hats_ring_longer_than_closed_ones() {
    let length = |name: &str| {
        seconds(
            renders()
                .iter()
                .find(|render| render.sound.name == name)
                .unwrap(),
        )
    };
    for closed in ["Hat Closed 1", "Hat Closed 2", "Hat Closed 3", "Hat Pedal"] {
        assert!(length(closed) < 0.2, "{closed}");
        assert!(length("Hat Open 1") > 3.0 * length(closed), "{closed}");
    }
    assert!(length("Hat Open 2") > length("Hat Open 1"));
}

/// How far the strongest 50 Hz slice of the treble stands above the average
/// slice. Noise scores low. A spectrum made of separate lines scores high.
fn peakiness(spectrum: &Spectrum) -> f64 {
    let slices: Vec<f64> = (0..180)
        .map(|slice| {
            let low = 6_000.0 + 50.0 * slice as f64;
            spectrum.share(low, low + 50.0)
        })
        .collect();
    let strongest = slices.iter().copied().fold(0.0, f64::max);
    strongest / (slices.iter().sum::<f64>() / slices.len() as f64)
}

#[test]
fn hats_are_metal_and_not_plain_noise() {
    // A cluster of square waves leaves strong lines in the spectrum, which
    // is what makes a hat sound like metal. White noise of the same length
    // is the reference for a spectrum without lines.
    for render in renders() {
        let name = render.sound.name;
        if !matches!(name, "Hat Open 1" | "Hat Open 2") {
            continue;
        }
        let mut rng = dsp::Rng::new(1);
        let noise: Vec<f64> = (0..render.finished.frames()).map(|_| rng.white()).collect();
        let reference = peakiness(&Spectrum::of(&noise, RATE));
        let hat = peakiness(&render.spectra[0]);
        assert!(hat > 3.0 * reference, "{name}: {hat} against {reference}");
    }
}

#[test]
fn toms_are_tuned_low_to_high() {
    let pitch = |name: &str, freq: f64| {
        let render = renders()
            .iter()
            .find(|render| render.sound.name == name)
            .unwrap();
        // The head settles onto its note after the first tenth of a second.
        let settled = &render.finished.channels()[0][4_800..];
        let on = power_at(settled, RATE, freq);
        for off in [freq * 0.84, freq * 1.19] {
            assert!(
                on > 20.0 * power_at(settled, RATE, off),
                "{name} at {off} Hz"
            );
        }
    };
    pitch("Tom Low", 82.4);
    pitch("Tom Mid", 123.5);
    pitch("Tom High", 174.6);
}

#[test]
fn bass_sounds_are_tuned_to_c() {
    // C2 in equal temperament: 33 semitones below A4.
    let c2 = 440.0 * 2f64.powf(-33.0 / 12.0);
    assert!((BASS_ROOT_HZ - c2).abs() < 1e-9);

    for render in renders() {
        if render.sound.category != Category::Bass {
            continue;
        }
        let name = render.sound.name;
        let channel = render.finished.channels()[0];
        // Sweep a semitone either side of C in steps of two cents. The
        // strongest frequency has to be C itself.
        let power_off_by =
            |cents: f64| power_at(channel, RATE, BASS_ROOT_HZ * 2f64.powf(cents / 1_200.0));
        let strongest = (-50..=50)
            .map(|step| 2.0 * step as f64)
            .max_by(|a, b| power_off_by(*a).total_cmp(&power_off_by(*b)))
            .unwrap();
        assert!(strongest.abs() <= 6.0, "{name} is {strongest} cents off");
        let on = power_off_by(0.0);
        // The fundamental carries the note; no overtone is louder.
        for harmonic in 2..=8 {
            let overtone = power_at(channel, RATE, BASS_ROOT_HZ * harmonic as f64);
            assert!(on > overtone, "{name}: harmonic {harmonic}");
        }
    }
}

#[test]
fn only_the_wide_sounds_are_stereo_and_they_survive_mono() {
    for render in renders() {
        let name = render.sound.name;
        let wide = matches!(name, "Clap Wide" | "Crash" | "Ride");
        assert_eq!(
            render.rendered.channels(),
            if wide { 2 } else { 1 },
            "{name}"
        );
        if let Audio::Stereo(left, right) = &render.finished {
            let energy = |sign: f64| -> f64 {
                left.iter()
                    .zip(right)
                    .map(|(l, r)| (l + sign * r) * (l + sign * r))
                    .sum()
            };
            let (mid, side) = (energy(1.0), energy(-1.0));
            // The channels differ, and what they share outweighs the rest.
            assert!(side > 0.05 * mid, "{name} is nearly mono");
            assert!(mid > 2.0 * side, "{name} would thin out in mono");
        }
    }
}

#[test]
fn audio_buffer_matches_the_rendered_samples() {
    let render = &renders()[0];
    let buffer = render.rendered.to_audio_buffer();
    assert_eq!(buffer.sample_rate(), SAMPLE_RATE);
    assert_eq!(buffer.channels(), render.rendered.channels());
    assert_eq!(buffer.frames(), render.rendered.frames());
    let peak = buffer.samples().iter().fold(0.0f32, |p, s| s.abs().max(p));
    assert!((20.0 * peak.log10() - render.sound.peak_db as f32).abs() < 0.01);
}
