//! What the stretcher does to sound, measured: pitch, level, the place
//! and sharpness of hits, the stereo image, and what is left of noise.
//!
//! The bounds are what the crate documentation states. `bench.rs` prints
//! the figures themselves.

use std::f64::consts::TAU;

use windfall_stretch::Quality;

use crate::support::{
    RATE, average_spectrum, band_ripple_db, cents, chord, clicks, db, energy, frequency_near,
    noise, peak, rms, run, sine, spectral_flatness, stream_mono, stretcher, tone, tone_level,
};

/// A chord whose notes are far enough apart for every preset to tell
/// them apart: 110 Hz at the closest.
const CHORD: [f64; 5] = [440.0, 554.37, 659.26, 880.0, 1_108.73];

const SINES: [f64; 4] = [110.0, 440.0, 1_000.0, 3_520.0];

/// The part of a stretched signal that is clear of its start and end.
pub fn middle(signal: &[f32]) -> &[f32] {
    let quarter = signal.len() / 4;
    &signal[quarter..signal.len() - quarter]
}

#[test]
fn stretching_keeps_a_sine_in_tune_and_at_its_level() {
    let frames = 3 * RATE as usize;
    for quality in Quality::ALL {
        for frequency in SINES {
            let input = sine(frequency, 0.5, frames, RATE);
            for ratio in [0.5, 0.75, 1.0, 1.25, 1.5, 2.0] {
                let output = stream_mono(&input, ratio, 0.0, quality);
                let part = middle(&output);
                let found = frequency_near(part, frequency, 0.03, RATE);
                let what = format!("{quality:?}, {frequency} Hz at {ratio}");
                let off = cents(found, frequency);
                assert!(off.abs() < 0.1, "{what}: {off:+.4} cents");
                let level = db(tone_level(part, found, RATE) / 0.5);
                assert!(level.abs() < 0.5, "{what}: {level:+.3} dB");
            }
        }
    }
}

#[test]
fn stretching_keeps_a_chord_in_tune_and_at_its_level() {
    let frames = 3 * RATE as usize;
    let input = chord(&CHORD, 0.8, frames, RATE);
    let each = 0.8 / CHORD.len() as f64;
    for quality in Quality::ALL {
        for ratio in [0.5, 0.75, 1.25, 1.5, 2.0] {
            let output = stream_mono(&input, ratio, 0.0, quality);
            let part = middle(&output);
            for note in CHORD {
                let found = frequency_near(part, note, 0.02, RATE);
                let what = format!("{quality:?}, {note} Hz of the chord at {ratio}");
                let off = cents(found, note);
                assert!(off.abs() < 0.25, "{what}: {off:+.4} cents");
                let level = db(tone_level(part, found, RATE) / each);
                assert!(level.abs() < 0.5, "{what}: {level:+.3} dB");
            }
            let whole = db(rms(part) / rms(middle(&input)));
            assert!(whole.abs() < 0.5, "{quality:?} at {ratio}: {whole:+.3} dB");
        }
    }
}

#[test]
fn shifting_moves_a_sine_by_exactly_the_semitones_asked_for() {
    let frames = 3 * RATE as usize;
    for quality in Quality::ALL {
        for frequency in SINES {
            let input = sine(frequency, 0.5, frames, RATE);
            for pitch in [-12.0, -7.0, -1.0, 0.31, 3.0, 7.0, 12.0] {
                // The length is the input's: the ratio is 1.
                let output = stream_mono(&input, 1.0, pitch, quality);
                assert_eq!(output.len(), input.len());
                let part = middle(&output);
                let wanted = frequency * 2.0_f64.powf(pitch / 12.0);
                let found = frequency_near(part, wanted, 0.03, RATE);
                let what = format!("{quality:?}, {frequency} Hz by {pitch}");
                let off = cents(found, wanted);
                assert!(off.abs() < 0.1, "{what}: {off:+.4} cents");
                let level = db(tone_level(part, found, RATE) / 0.5);
                assert!(level.abs() < 0.5, "{what}: {level:+.3} dB");
            }
        }
    }
}

#[test]
fn shifting_moves_a_chord_by_exactly_the_semitones_asked_for() {
    let frames = 3 * RATE as usize;
    let input = chord(&CHORD, 0.8, frames, RATE);
    let each = 0.8 / CHORD.len() as f64;
    for quality in Quality::ALL {
        for pitch in [-12.0, -5.0, 4.0, 12.0] {
            let output = stream_mono(&input, 1.0, pitch, quality);
            let part = middle(&output);
            let factor = 2.0_f64.powf(pitch / 12.0);
            for note in CHORD {
                let found = frequency_near(part, note * factor, 0.02, RATE);
                let what = format!("{quality:?}, {note} Hz of the chord by {pitch}");
                let off = cents(found, note * factor);
                assert!(off.abs() < 0.25, "{what}: {off:+.4} cents");
                let level = db(tone_level(part, found, RATE) / each);
                assert!(level.abs() < 0.5, "{what}: {level:+.3} dB");
            }
            let whole = db(rms(part) / rms(middle(&input)));
            assert!(whole.abs() < 0.5, "{quality:?} by {pitch}: {whole:+.3} dB");
        }
    }
}

#[test]
fn stretching_and_shifting_at_once_do_both() {
    let frames = 3 * RATE as usize;
    for quality in Quality::ALL {
        for (ratio, pitch) in [(0.8, 3.0), (1.3, -3.0), (1.5, 7.0), (0.6, -12.0)] {
            for frequency in [220.0, 1_000.0] {
                let input = sine(frequency, 0.5, frames, RATE);
                let output = stream_mono(&input, ratio, pitch, quality);
                assert_eq!(output.len(), (frames as f64 * ratio).round() as usize);
                let part = middle(&output);
                let wanted = frequency * 2.0_f64.powf(pitch / 12.0);
                let found = frequency_near(part, wanted, 0.03, RATE);
                let what = format!("{quality:?}, {frequency} Hz at {ratio} by {pitch}");
                let off = cents(found, wanted);
                assert!(off.abs() < 0.1, "{what}: {off:+.4} cents");
                let level = db(tone_level(part, found, RATE) / 0.5);
                assert!(level.abs() < 0.5, "{what}: {level:+.3} dB");
            }
        }
    }
}

/// What became of the clicks of a click track `period` frames apart,
/// stretched by `ratio`: the share of each click's energy that comes more
/// than 2 ms early, in dB; the share that lies from 2 ms before to 10 ms
/// after its place; the energy against `before`, in dB; and the furthest
/// a click's loudest sample is from its place, in frames.
pub fn click_figures(
    output: &[f32],
    period: usize,
    ratio: f64,
    input: &[f32],
) -> (f64, f64, f64, i64) {
    let spacing = (period as f64 * ratio) as usize;
    let half = spacing / 2;
    let ms = RATE as usize / 1_000;
    let (mut early, mut near, mut total) = (0.0, 0.0, 0.0);
    let mut furthest = 0_i64;
    for index in 1..6 {
        let place = ((period / 2 + index * period) as f64 * ratio).round() as usize;
        let window = &output[place - half..place + half];
        total += energy(window);
        early += energy(&window[..half - 2 * ms]);
        near += energy(&window[half - 2 * ms..half + 10 * ms]);
        let loudest = (0..window.len())
            .max_by(|a, b| window[*a].abs().total_cmp(&window[*b].abs()))
            .unwrap_or(half);
        let off = loudest as i64 - half as i64;
        if off.abs() > furthest.abs() {
            furthest = off;
        }
    }
    let before = energy(&input[period..6 * period]);
    (
        10.0 * (early / total).log10(),
        near / total,
        10.0 * (total / before).log10(),
        furthest,
    )
}

#[test]
fn a_hit_stays_in_its_place_and_stays_sharp() {
    let period = RATE as usize / 2;
    let input = clicks(period / 2, period, 4 * RATE as usize, RATE);
    for quality in Quality::ALL {
        for ratio in [0.75, 0.9, 1.0, 1.1, 1.25, 1.5] {
            let output = stream_mono(&input, ratio, 0.0, quality);
            let (early, near, level, furthest) = click_figures(&output, period, ratio, &input);
            let what = format!("{quality:?} at {ratio}");
            // Less than a hundredth of a hit's energy comes early.
            assert!(early < -22.0, "{what}: {early:.1} dB of pre-echo");
            assert!(near > 0.99, "{what}: {:.2}% in place", near * 100.0);
            // The loudest sample of a click is in its first millisecond.
            assert!(
                (-24..72).contains(&furthest),
                "{what}: {furthest} frames off"
            );
            // A hit gains level with the length: see the crate docs.
            assert!(level.abs() < 2.0, "{what}: {level:+.2} dB");
            assert!(
                peak(&output) < 1.5 * peak(&input),
                "{what}: peak {}",
                peak(&output)
            );
        }
    }
}

#[test]
fn a_panned_source_stays_where_it_is() {
    let frames = 3 * RATE as usize;
    // Two notes: one mostly left with the right channel a quarter turn
    // behind, one mostly right and in phase.
    let (first, second) = (330.0, 1_500.0);
    let wave = |frequency: f64, level: f64, turn: f64| -> Vec<f32> {
        (0..frames)
            .map(|n| (level * (TAU * frequency * n as f64 / f64::from(RATE) + turn).sin()) as f32)
            .collect()
    };
    let add =
        |a: Vec<f32>, b: Vec<f32>| -> Vec<f32> { a.iter().zip(&b).map(|(a, b)| a + b).collect() };
    let input = vec![
        add(wave(first, 0.5, 0.0), wave(second, 0.1, 0.0)),
        add(wave(first, 0.2, -TAU / 4.0), wave(second, 0.4, 0.0)),
    ];
    for quality in Quality::ALL {
        for (ratio, pitch) in [(0.7, 0.0), (1.5, 0.0), (1.0, 5.0), (1.3, -4.0)] {
            let mut stretcher = stretcher(2, ratio, pitch, quality);
            let wanted = (frames as f64 * ratio) as usize;
            let output = run(&mut stretcher, &input, &mut 0, wanted, &[512]);
            let factor = 2.0_f64.powf(pitch / 12.0);
            let part = |channel: usize| &output[channel][wanted / 3..wanted - 4_800];
            for (note, balance, turn) in [(first, 0.5 / 0.2, -TAU / 4.0), (second, 0.1 / 0.4, 0.0)]
            {
                let found = frequency_near(part(0), note * factor, 0.02, RATE);
                let (left, left_phase) = tone(part(0), found, RATE);
                let (right, right_phase) = tone(part(1), found, RATE);
                let what = format!("{quality:?}, {note} Hz at {ratio} by {pitch}");
                let level = db(left / right / balance);
                assert!(level.abs() < 0.25, "{what}: balance off by {level:+.3} dB");
                let between =
                    (right_phase - left_phase - turn + TAU * 1.5).rem_euclid(TAU) - TAU / 2.0;
                assert!(
                    between.abs() < 0.02,
                    "{what}: phase off by {between:+.4} rad"
                );
            }
        }
    }
}

#[test]
fn a_mono_signal_on_two_channels_stays_mono() {
    let mono = chord(&CHORD, 0.7, 96_000, RATE);
    let input = vec![mono.clone(), mono];
    let mut stretcher = stretcher(2, 1.4, 3.0, Quality::Standard);
    let output = run(&mut stretcher, &input, &mut 0, 120_000, &[512]);
    let apart = output[0]
        .iter()
        .zip(&output[1])
        .fold(0.0_f32, |worst, (left, right)| {
            worst.max((left - right).abs())
        });
    assert!(apart < 1e-4, "the channels are {apart:e} apart");
    assert!(rms(&output[0][60_000..]) > 0.2);
}

#[test]
fn noise_keeps_its_colour_and_most_of_its_level() {
    let frames = 4 * RATE as usize;
    let input = noise(3, 0.5, frames);
    let flat = spectral_flatness(&input, 4_096, RATE);
    for quality in Quality::ALL {
        for ratio in [0.75, 1.25, 1.5] {
            let output = stream_mono(&input, ratio, 0.0, quality);
            let part = middle(&output);
            let what = format!("{quality:?} at {ratio}");
            // Flat in thirds of an octave: no colour is added.
            let ripple = band_ripple_db(part, RATE);
            assert!(ripple < 0.75, "{what}: {ripple:.2} dB of ripple");
            // Noise loses some level, because blocks of it that are moved
            // apart no longer add up as they did.
            let level = db(rms(part) / rms(&input));
            assert!((-1.75..0.25).contains(&level), "{what}: {level:+.2} dB");
            // Within a block a stretched noise is more regular than
            // noise: this is the phasiness of a phase vocoder, and the
            // bound is how much of it there may be.
            let found = spectral_flatness(part, 4_096, RATE);
            assert!(
                found > 0.7 * flat,
                "{what}: flatness {found:.3} against {flat:.3}"
            );
        }
    }
}

/// A buzz at `fundamental` whose harmonics follow a spectral envelope
/// with a single formant at `formant`.
fn vowel(fundamental: f64, formant: f64, frames: usize) -> Vec<f32> {
    let harmonics = (10_000.0 / fundamental) as usize;
    (0..frames)
        .map(|n| {
            let time = n as f64 / f64::from(RATE);
            let sum: f64 = (1..=harmonics)
                .map(|harmonic| {
                    let frequency = fundamental * harmonic as f64;
                    let distance = (frequency / formant).log2();
                    let level = 0.03 + (-distance * distance * 6.0).exp();
                    level * (TAU * frequency * time + harmonic as f64).sin()
                })
                .sum();
            (sum * 0.08) as f32
        })
        .collect()
}

/// The frequency around which the energy of `signal` between 300 Hz and
/// 3 kHz is centred.
fn centre_of_energy(signal: &[f32]) -> f64 {
    let size = 8_192;
    let spectrum = average_spectrum(signal, size);
    let hz = f64::from(RATE) / size as f64;
    let band = (300.0 / hz) as usize..(3_000.0 / hz) as usize;
    let weighted: f64 = band
        .clone()
        .map(|bin| bin as f64 * hz * spectrum[bin])
        .sum();
    weighted / spectrum[band].iter().sum::<f64>()
}

#[test]
fn formant_preservation_keeps_the_envelope_while_the_pitch_moves() {
    let frames = 3 * RATE as usize;
    let (fundamental, formant) = (150.0, 900.0);
    let input = vowel(fundamental, formant, frames);
    let before = centre_of_energy(middle(&input));
    for pitch in [-5.0, 5.0] {
        let factor = 2.0_f64.powf(pitch / 12.0);
        let mut centres = [0.0; 2];
        for (slot, preserve) in centres.iter_mut().zip([false, true]) {
            let mut stretcher = stretcher(1, 1.0, pitch, Quality::Standard);
            stretcher.set_formant_preservation(preserve);
            let output = run(
                &mut stretcher,
                std::slice::from_ref(&input),
                &mut 0,
                frames,
                &[512],
            )
            .remove(0);
            let part = middle(&output);
            *slot = centre_of_energy(part);
            // The pitch moves either way.
            let wanted = fundamental * 6.0 * factor;
            let found = frequency_near(part, wanted, 0.02, RATE);
            assert!(
                cents(found, wanted).abs() < 2.0,
                "{pitch} {preserve}: {found} Hz"
            );
        }
        let [plain, kept] = centres;
        // Without it the formant moves with the pitch; with it the
        // formant stays within a semitone of where it was.
        assert!(
            cents(plain, before * factor).abs() < 100.0,
            "{pitch}: {plain} from {before}"
        );
        assert!(
            cents(kept, before).abs() < 100.0,
            "{pitch}: {kept} from {before}"
        );
    }
}
