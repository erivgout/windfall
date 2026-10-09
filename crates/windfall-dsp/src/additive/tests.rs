//! Measurements of the six additive instruments.
//!
//! An additive instrument is tested the way it is built: a long steady note
//! is rendered and the level of each partial is measured, and what comes
//! out is compared with what the parameters asked for. The measurement is a
//! Goertzel filter rather than a transform, because only a handful of
//! frequencies are ever of interest and the crate has no transform of its
//! own.
//!
//! The harmonic measurements run at 44 kHz, where A4 is exactly 100 samples
//! a cycle, so every harmonic of it lands exactly on a bin of a window that
//! is a whole number of hundreds of samples long. With no leakage between
//! them the measured level of a partial is its level, and the tests can
//! compare against the parameters directly instead of against a tolerance
//! for the window.

use std::f64::consts::TAU;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::blocks::math::key_to_hz;
use crate::param::{ParamInfo, ParamKind, ParamSet};
use crate::{Instrument, ParamScale};

use super::engine::sine_turns;
use super::{
    HarmonicStack, HarmonicStackParams, Inharmonic, InharmonicParams, MAX_PARTIALS, MORPH_PARTIALS,
    PartialGains, PartialMorph, PartialMorphParams, Resynth, ResynthParams, SCAN_STEPS, ScanSynth,
    ScanSynthParams, SeedPatch, SeedPatchParams, TABLE_PARTIALS,
};

/// A rate at which A4 is exactly 100 samples a cycle.
const HARMONIC_RATE: f32 = 44_000.0;

/// Samples measured: a whole number of hundreds, so every harmonic of A4 is
/// a whole number of cycles.
const HARMONIC_FRAMES: usize = 22_000;

/// Samples skipped before measuring, to let the attack finish.
const SETTLE: usize = 4_400;

/// The A at 440 Hz.
const A4: u8 = 69;

/// An envelope that rises quickly and then holds exactly still, so that a
/// measured level is the partial's level and nothing else.
fn steady_voicing(gain: f32) -> super::VoicingParams {
    super::VoicingParams {
        envelope: crate::EnvelopeParams {
            attack_ms: 1.0,
            decay_ms: 1.0,
            sustain: 1.0,
            release_ms: 100.0,
        },
        polyphony: 8,
        velocity: 0.0,
        gain,
        pan: 0.0,
    }
}

/// Renders `frames` samples in blocks of at most `block` and returns the
/// left channel.
///
/// Both buffers start full of values that are not numbers, so a sample the
/// instrument failed to write shows up as one.
fn render<I: Instrument>(instrument: &mut I, frames: usize, block: usize) -> Vec<f32> {
    let mut left = vec![f32::NAN; frames];
    let mut right = vec![f32::NAN; frames];
    let mut at = 0;
    while at < frames {
        let end = (at + block).min(frames);
        instrument.process(&mut left[at..end], &mut right[at..end]);
        at = end;
    }
    left
}

/// The largest step between one sample and the next.
fn roughest_step(signal: &[f32]) -> f32 {
    signal
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).abs())
        .fold(0.0, f32::max)
}

/// Prepares `instrument`, applies `params`, holds one note at `key` and
/// returns the samples after the attack has finished.
fn steady_note<I: Instrument>(
    instrument: &mut I,
    params: &I::Params,
    key: u8,
    rate: f32,
) -> Vec<f32> {
    instrument.prepare(rate, 512);
    instrument.set_params(params);
    instrument.note_on(key, 1.0);
    let signal = render(instrument, SETTLE + HARMONIC_FRAMES, 256);
    assert!(signal.iter().all(|sample| sample.is_finite()));
    signal[SETTLE..].to_vec()
}

/// The amplitude of `frequency` in `signal`, by Goertzel's recurrence.
///
/// Exact when `frequency` is a whole number of cycles over `signal`, which
/// is what [`HARMONIC_RATE`] and [`HARMONIC_FRAMES`] arrange for every
/// harmonic of A4.
fn amplitude(signal: &[f32], frequency: f32, rate: f32) -> f64 {
    let turn = TAU * f64::from(frequency) / f64::from(rate);
    let coefficient = 2.0 * turn.cos();
    let (mut previous, mut older) = (0.0_f64, 0.0_f64);
    for sample in signal {
        let current = f64::from(*sample) + coefficient * previous - older;
        older = previous;
        previous = current;
    }
    let real = previous - older * turn.cos();
    let imaginary = older * turn.sin();
    2.0 * real.hypot(imaginary) / signal.len() as f64
}

/// [`amplitude`] through a Hann window, for a frequency that is not a whole
/// number of cycles. The window costs about a decibel of accuracy and buys
/// freedom from the leakage a rectangular one would let in.
fn windowed_amplitude(signal: &[f32], frequency: f32, rate: f32) -> f64 {
    let frames = signal.len();
    let windowed: Vec<f32> = signal
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            let turn = TAU * index as f64 / frames as f64;
            (f64::from(*sample) * 0.5 * (1.0 - turn.cos())) as f32
        })
        .collect();
    // A Hann window passes half the signal through, and spreads a partial
    // over three bins in the proportions 1, 2, 1.
    2.0 * amplitude(&windowed, frequency, rate)
}

fn rms(signal: &[f32]) -> f64 {
    let power: f64 = signal.iter().map(|s| f64::from(*s) * f64::from(*s)).sum();
    (power / signal.len().max(1) as f64).sqrt()
}

// ---------------------------------------------------------------------
// The oscillator
// ---------------------------------------------------------------------

#[test]
fn the_polynomial_sine_matches_the_library_one() {
    let mut worst = 0.0_f32;
    for step in 0..100_000 {
        let turns = step as f32 / 100_000.0;
        let wanted = (std::f32::consts::TAU * turns).sin();
        worst = worst.max((sine_turns(turns) - wanted).abs());
    }
    assert!(worst < 5.0e-6, "worst error {worst}");
}

// ---------------------------------------------------------------------
// HarmonicStack
// ---------------------------------------------------------------------

#[test]
fn a_harmonic_stack_sounds_the_levels_it_is_given() {
    let mut params = HarmonicStackParams {
        partials: 12,
        odd_even_tilt: 0.0,
        rolloff: 0.0,
        gains: PartialGains::from_fn(|index| {
            if index < 12 {
                0.9 / (index + 1) as f32
            } else {
                0.0
            }
        }),
        voicing: steady_voicing(1.0),
    };
    params = params.sanitized();
    let mut stack = HarmonicStack::default();
    let signal = steady_note(&mut stack, &params, A4, HARMONIC_RATE);

    // The partials are the harmonic series, exactly.
    let spectrum = stack.spectrum();
    assert_eq!(spectrum.ratios().len(), 12);
    for (index, ratio) in spectrum.ratios().iter().enumerate() {
        assert_eq!(*ratio, (index + 1) as f32);
    }

    for partial in 0..12 {
        let wanted = f64::from(params.gains[partial]);
        let found = amplitude(
            &signal,
            key_to_hz(f32::from(A4)) * (partial + 1) as f32,
            HARMONIC_RATE,
        );
        assert!(
            (found - wanted).abs() <= 0.02 * wanted + 1.0e-4,
            "partial {}: asked for {wanted}, measured {found}",
            partial + 1
        );
    }
    // Nothing is sounded above the partials that were asked for.
    let above = amplitude(&signal, key_to_hz(f32::from(A4)) * 13.0, HARMONIC_RATE);
    assert!(above < 1.0e-3, "partial 13 sounded at {above}");
}

#[test]
fn the_tilt_silences_one_half_of_the_harmonics() {
    let gains = PartialGains::from_fn(|index| if index < 8 { 0.5 } else { 0.0 });
    let base = HarmonicStackParams {
        partials: 8,
        odd_even_tilt: 0.0,
        rolloff: 0.0,
        gains,
        voicing: steady_voicing(1.0),
    };
    for (tilt, silent_is_even) in [(-1.0, true), (1.0, false)] {
        let params = HarmonicStackParams {
            odd_even_tilt: tilt,
            ..base
        }
        .sanitized();
        let mut stack = HarmonicStack::default();
        let signal = steady_note(&mut stack, &params, A4, HARMONIC_RATE);
        for partial in 1..=8_u32 {
            let found = amplitude(
                &signal,
                key_to_hz(f32::from(A4)) * partial as f32,
                HARMONIC_RATE,
            );
            if partial.is_multiple_of(2) == silent_is_even {
                assert!(found < 1.0e-3, "tilt {tilt}: partial {partial} at {found}");
            } else {
                assert!(found > 0.4, "tilt {tilt}: partial {partial} only {found}");
            }
        }
    }
}

#[test]
fn the_rolloff_takes_the_top_off() {
    let gains = PartialGains::from_fn(|index| if index < 8 { 0.5 } else { 0.0 });
    let params = HarmonicStackParams {
        partials: 8,
        odd_even_tilt: 0.0,
        rolloff: 0.25,
        gains,
        voicing: steady_voicing(1.0),
    }
    .sanitized();
    let mut stack = HarmonicStack::default();
    let signal = steady_note(&mut stack, &params, A4, HARMONIC_RATE);
    // A rolloff of 0.25 is one octave of level per octave of frequency, so
    // the second harmonic is half the first and the fourth a quarter.
    let first = amplitude(&signal, 440.0, HARMONIC_RATE);
    let second = amplitude(&signal, 880.0, HARMONIC_RATE);
    let fourth = amplitude(&signal, 1760.0, HARMONIC_RATE);
    assert!((second / first - 0.5).abs() < 0.02, "{second} / {first}");
    assert!((fourth / first - 0.25).abs() < 0.02, "{fourth} / {first}");
}

// ---------------------------------------------------------------------
// PartialMorph
// ---------------------------------------------------------------------

fn morph_params(morph: f32) -> PartialMorphParams {
    PartialMorphParams {
        partials: 8,
        morph,
        // A sine against a spectrum with nothing in common with it: the
        // four partials of one are silent in the other.
        snapshot_a: std::array::from_fn(|index| if index < 4 { 0.6 } else { 0.0 }),
        snapshot_b: std::array::from_fn(|index| if (4..8).contains(&index) { 0.6 } else { 0.0 }),
        voicing: steady_voicing(1.0),
    }
    .sanitized()
}

#[test]
fn the_morph_ends_match_the_two_snapshots() {
    for (morph, loud) in [(0.0_f32, 0..4), (1.0, 4..8)] {
        let params = morph_params(morph);
        let mut morpher = PartialMorph::default();
        let signal = steady_note(&mut morpher, &params, A4, HARMONIC_RATE);
        // The levels handed to the engine are the snapshot itself, with
        // nothing of the other one left in them.
        let snapshot = if morph == 0.0 {
            params.snapshot_a
        } else {
            params.snapshot_b
        };
        let spectrum = morpher.spectrum();
        assert_eq!(spectrum.gains(), &snapshot[..8]);
        for partial in 0..8 {
            let found = amplitude(
                &signal,
                key_to_hz(f32::from(A4)) * (partial + 1) as f32,
                HARMONIC_RATE,
            );
            if loud.contains(&partial) {
                assert!(
                    (found - 0.6).abs() < 0.02,
                    "morph {morph}: partial {} measured {found}",
                    partial + 1
                );
            } else {
                assert!(
                    found < 1.0e-3,
                    "morph {morph}: partial {} should be silent, measured {found}",
                    partial + 1
                );
            }
        }
    }
}

#[test]
fn the_morph_control_is_audible_in_between() {
    let mut levels = Vec::new();
    for morph in [0.0_f32, 0.25, 0.5, 0.75, 1.0] {
        let params = morph_params(morph);
        let mut morpher = PartialMorph::default();
        let signal = steady_note(&mut morpher, &params, A4, HARMONIC_RATE);
        let low = amplitude(&signal, 440.0, HARMONIC_RATE);
        let high = amplitude(&signal, 440.0 * 5.0, HARMONIC_RATE);
        levels.push((low, high));
    }
    // Halfway means halfway, partial by partial.
    let (low, high) = levels[2];
    assert!((low - 0.3).abs() < 0.02, "halfway low partial {low}");
    assert!((high - 0.3).abs() < 0.02, "halfway high partial {high}");
    // And the sweep is monotonic in both directions at once.
    for window in levels.windows(2) {
        assert!(window[1].0 < window[0].0 - 0.1, "{levels:?}");
        assert!(window[1].1 > window[0].1 + 0.1, "{levels:?}");
    }
}

#[test]
fn a_morph_is_not_a_harmonic_stack() {
    // The same levels cannot be had from one snapshot: the control is what
    // makes the sound, and moving it changes the sound.
    let ends: Vec<Vec<f32>> = [0.0, 1.0]
        .into_iter()
        .map(|morph| {
            let params = morph_params(morph);
            let mut morpher = PartialMorph::default();
            steady_note(&mut morpher, &params, A4, HARMONIC_RATE)
        })
        .collect();
    let difference: Vec<f32> = ends[0]
        .iter()
        .zip(&ends[1])
        .map(|(first, second)| first - second)
        .collect();
    assert!(
        rms(&difference) > 0.5 * rms(&ends[0]),
        "the two ends of the morph are nearly the same sound"
    );
}

// ---------------------------------------------------------------------
// Inharmonic
// ---------------------------------------------------------------------

#[test]
fn no_inharmonic_partial_sits_on_its_harmonic() {
    // Every corner of the control range, not just the default.
    for stiffness in [0.0_f32, 0.25, 0.5, 0.75, 1.0] {
        for stretch in [0.02_f32, 0.1, 0.2, 0.3] {
            let params = InharmonicParams {
                partials: MAX_PARTIALS as u8,
                stiffness,
                stretch,
                rolloff: 0.3,
                voicing: steady_voicing(0.2),
            }
            .sanitized();
            let mut bell = Inharmonic::default();
            bell.set_params(&params);
            let spectrum = bell.spectrum();
            let ratios = spectrum.ratios();
            assert_eq!(ratios.len(), MAX_PARTIALS);
            assert!(
                (ratios[0] - 1.0).abs() < 1.0e-6,
                "the lowest partial is the pitch played"
            );
            for (index, ratio) in ratios.iter().enumerate().skip(1) {
                let harmonic = (index + 1) as f32;
                let departure = (ratio / harmonic - 1.0).abs();
                assert!(
                    departure > 0.02,
                    "stiffness {stiffness}, stretch {stretch}: partial {} at {ratio} \
                     is within {departure} of harmonic {harmonic}",
                    index + 1
                );
                assert!(*ratio > ratios[index - 1], "the series must climb");
            }
        }
    }
}

#[test]
fn an_inharmonic_note_has_no_energy_at_its_octave() {
    let params = InharmonicParams {
        partials: 6,
        rolloff: 0.0,
        voicing: steady_voicing(1.0),
        ..InharmonicParams::default()
    }
    .sanitized();
    let mut bell = Inharmonic::default();
    bell.set_params(&params);
    let ratios = bell.spectrum().ratios().to_vec();
    let signal = steady_note(&mut bell, &params, A4, HARMONIC_RATE);
    let fundamental = key_to_hz(f32::from(A4));

    let at_second = windowed_amplitude(&signal, fundamental * ratios[1], HARMONIC_RATE);
    let at_octave = windowed_amplitude(&signal, fundamental * 2.0, HARMONIC_RATE);
    assert!(
        at_second > 0.5,
        "the second partial is missing: {at_second}"
    );
    assert!(
        at_octave < 0.05 * at_second,
        "there is energy at the octave ({at_octave}) where the partial is at \
         {} times the pitch ({at_second})",
        ratios[1]
    );
}

// ---------------------------------------------------------------------
// Resynth
// ---------------------------------------------------------------------

#[test]
fn resynth_plays_the_table_it_is_given() {
    let mut params = ResynthParams {
        partials: 4,
        stretch: 1.0,
        voicing: steady_voicing(1.0),
        ..ResynthParams::default()
    };
    // Ratios that are nothing like a harmonic series, so there is no way
    // the measurement could be reading a harmonic instead.
    let wanted = [(1.0_f32, 0.5_f32), (2.5, 0.4), (4.75, 0.3), (9.125, 0.2)];
    for (row, (ratio, gain)) in params.table.iter_mut().zip(wanted) {
        row.ratio = ratio;
        row.gain = gain;
        row.phase = 0.0;
    }
    for row in &mut params.table[4..] {
        row.gain = 0.0;
    }
    let params = params.sanitized();
    let mut resynth = Resynth::default();
    let signal = steady_note(&mut resynth, &params, A4, HARMONIC_RATE);
    let fundamental = key_to_hz(f32::from(A4));
    for (ratio, gain) in wanted {
        let found = windowed_amplitude(&signal, fundamental * ratio, HARMONIC_RATE);
        assert!(
            (found - f64::from(gain)).abs() < 0.05 * f64::from(gain) + 0.01,
            "partial at {ratio}: asked for {gain}, measured {found}"
        );
    }
    // The harmonics the table does not hold are not there.
    for harmonic in [2.0_f32, 3.0, 4.0] {
        let found = windowed_amplitude(&signal, fundamental * harmonic, HARMONIC_RATE);
        assert!(found < 0.02, "harmonic {harmonic} sounded at {found}");
    }
}

#[test]
fn the_table_phase_decides_the_waveform() {
    // Two partials with the same levels but opposite phases make a
    // different wave, which is what a resynthesis table has to preserve.
    let shape = |phase: f32| {
        let mut params = ResynthParams {
            partials: 2,
            voicing: steady_voicing(1.0),
            ..ResynthParams::default()
        };
        params.table[0] = super::TablePartial {
            ratio: 1.0,
            gain: 0.5,
            phase: 0.0,
        };
        params.table[1] = super::TablePartial {
            ratio: 2.0,
            gain: 0.5,
            phase,
        };
        let params = params.sanitized();
        let mut resynth = Resynth::default();
        steady_note(&mut resynth, &params, A4, HARMONIC_RATE)
    };
    let first = shape(0.0);
    let second = shape(0.5);
    let difference: Vec<f32> = first
        .iter()
        .zip(&second)
        .map(|(one, other)| one - other)
        .collect();
    assert!(
        rms(&difference) > 0.2 * rms(&first),
        "the phase did nothing"
    );
    // Both are the same spectrum, so the levels must match all the same.
    for harmonic in [440.0_f32, 880.0] {
        let one = amplitude(&first, harmonic, HARMONIC_RATE);
        let other = amplitude(&second, harmonic, HARMONIC_RATE);
        assert!((one - other).abs() < 0.02, "{one} against {other}");
    }
}

#[test]
fn the_stretch_moves_the_whole_table() {
    let params = ResynthParams {
        partials: 4,
        stretch: 1.5,
        voicing: steady_voicing(1.0),
        ..ResynthParams::default()
    }
    .sanitized();
    let mut resynth = Resynth::default();
    let ratios = {
        let mut probe = Resynth::default();
        probe.set_params(&params);
        probe.spectrum().ratios().to_vec()
    };
    assert_eq!(ratios.len(), 4);
    for (index, ratio) in ratios.iter().enumerate() {
        assert!(
            (ratio - 1.5 * (index + 1) as f32).abs() < 1.0e-5,
            "{ratios:?}"
        );
    }
    let signal = steady_note(&mut resynth, &params, A4, HARMONIC_RATE);
    let found = windowed_amplitude(&signal, 440.0 * 1.5, HARMONIC_RATE);
    assert!(found > 0.4, "the stretched fundamental is at {found}");
}

// ---------------------------------------------------------------------
// SeedPatch
// ---------------------------------------------------------------------

#[test]
fn a_seed_decides_the_whole_timbre() {
    let patch = |seed: u32| {
        let params = SeedPatchParams {
            seed,
            partials: 16,
            voicing: steady_voicing(0.5),
            ..SeedPatchParams::default()
        }
        .sanitized();
        let mut synth = SeedPatch::default();
        synth.prepare(HARMONIC_RATE, 512);
        synth.set_params(&params);
        synth.note_on(A4, 1.0);
        (render(&mut synth, 4_096, 256), {
            let mut probe = SeedPatch::default();
            probe.set_params(&params);
            (
                probe.spectrum().ratios().to_vec(),
                probe.spectrum().gains().to_vec(),
            )
        })
    };

    let (first, first_spectrum) = patch(7);
    let (again, again_spectrum) = patch(7);
    let (other, other_spectrum) = patch(8);

    assert_eq!(first, again, "the same seed gave a different sound");
    assert_eq!(first_spectrum, again_spectrum);
    assert_ne!(first, other, "two seeds gave the same sound");
    assert_ne!(
        first_spectrum.0, other_spectrum.0,
        "the ratios did not change"
    );
    assert_ne!(
        first_spectrum.1, other_spectrum.1,
        "the levels did not change"
    );
    assert!(rms(&first) > 1.0e-3, "the seeded patch is silent");
}

#[test]
fn raising_the_partial_count_leaves_the_partials_below_alone() {
    let spectrum = |partials: u8| {
        let params = SeedPatchParams {
            seed: 99,
            partials,
            ..SeedPatchParams::default()
        }
        .sanitized();
        let mut synth = SeedPatch::default();
        synth.set_params(&params);
        (
            synth.spectrum().ratios().to_vec(),
            synth.spectrum().gains().to_vec(),
        )
    };
    let (few_ratios, few_gains) = spectrum(8);
    let (many_ratios, many_gains) = spectrum(32);
    assert_eq!(few_ratios.len(), 8);
    assert_eq!(many_ratios.len(), 32);
    assert_eq!(few_ratios, many_ratios[..8]);
    assert_eq!(few_gains, many_gains[..8]);
}

#[test]
fn a_spread_of_zero_leaves_a_seeded_patch_harmonic() {
    let params = SeedPatchParams {
        seed: 4,
        partials: 12,
        spread: 0.0,
        ..SeedPatchParams::default()
    }
    .sanitized();
    let mut synth = SeedPatch::default();
    synth.set_params(&params);
    for (index, ratio) in synth.spectrum().ratios().iter().enumerate() {
        assert!((ratio - (index + 1) as f32).abs() < 1.0e-5);
    }
}

// ---------------------------------------------------------------------
// ScanSynth
// ---------------------------------------------------------------------

/// A rate at which A4 is exactly 128 samples a cycle, which is four samples
/// for each of the 32 steps.
const SCAN_RATE: f32 = 440.0 * 128.0;
const SCAN_PERIOD: usize = 128;
const SAMPLES_PER_STEP: usize = SCAN_PERIOD / SCAN_STEPS;

#[test]
fn the_scanned_waveform_is_the_row() {
    let row: [f32; SCAN_STEPS] =
        std::array::from_fn(|index| ((index * 7) % SCAN_STEPS) as f32 / 16.0 - 1.0);
    let params = ScanSynthParams {
        row,
        interpolate: false,
        smoothing: 0.0,
        voicing: steady_voicing(1.0),
    }
    .sanitized();
    let mut synth = ScanSynth::default();
    synth.prepare(SCAN_RATE, 512);
    synth.set_params(&params);
    synth.note_on(A4, 1.0);
    let signal = render(&mut synth, 8 * SCAN_PERIOD, 256);

    // The first cycle is still under the attack. From the third on the
    // envelope is holding exactly still, so a sample is a row value.
    for (index, sample) in signal.iter().enumerate().skip(2 * SCAN_PERIOD) {
        let step = (index % SCAN_PERIOD) / SAMPLES_PER_STEP;
        assert!(
            (sample - row[step]).abs() < 1.0e-6,
            "sample {index} is {sample} where step {step} of the row is {}",
            row[step]
        );
    }
    // Which is to say the waveform repeats with the row's period.
    for index in 2 * SCAN_PERIOD..signal.len() - SCAN_PERIOD {
        assert_eq!(signal[index], signal[index + SCAN_PERIOD]);
    }
}

#[test]
fn interpolating_joins_the_steps_without_moving_them() {
    let row: [f32; SCAN_STEPS] = std::array::from_fn(|index| if index < 16 { 1.0 } else { -1.0 });
    let params = |interpolate: bool| {
        ScanSynthParams {
            row,
            interpolate,
            smoothing: 0.0,
            voicing: steady_voicing(1.0),
        }
        .sanitized()
    };
    let play = |interpolate: bool| {
        let mut synth = ScanSynth::default();
        synth.prepare(SCAN_RATE, 512);
        synth.set_params(&params(interpolate));
        synth.note_on(A4, 1.0);
        render(&mut synth, 8 * SCAN_PERIOD, 64)
    };
    let held = play(false);
    let joined = play(true);
    // A square read as a staircase is a square. Joining the steps rounds
    // its corners, so it is no longer at full level everywhere.
    let flat = &held[2 * SCAN_PERIOD..];
    assert!(flat.iter().all(|sample| sample.abs() > 0.99));
    let sloped = &joined[2 * SCAN_PERIOD..];
    assert!(sloped.iter().any(|sample| sample.abs() < 0.5));
    // The period is untouched.
    for index in 0..sloped.len() - SCAN_PERIOD {
        assert!((sloped[index] - sloped[index + SCAN_PERIOD]).abs() < 1.0e-6);
    }
}

#[test]
fn smoothing_rounds_the_corners_off() {
    let row: [f32; SCAN_STEPS] = std::array::from_fn(|index| if index < 16 { 0.8 } else { -0.8 });
    let play = |smoothing: f32| {
        let params = ScanSynthParams {
            row,
            interpolate: false,
            smoothing,
            voicing: steady_voicing(1.0),
        }
        .sanitized();
        let mut synth = ScanSynth::default();
        steady_note(&mut synth, &params, A4, SCAN_RATE)
    };
    let sharp = play(0.0);
    let soft = play(1.0);
    // Unsmoothed, the row is reproduced exactly, so its one jump is the
    // whole 1.6 of it.
    assert!(
        (roughest_step(&sharp) - 1.6).abs() < 1.0e-3,
        "the staircase measured {}",
        roughest_step(&sharp)
    );
    assert!(
        roughest_step(&soft) < 0.25 * roughest_step(&sharp),
        "smoothing left a step of {}",
        roughest_step(&soft)
    );
    // The note is still there; only its corners went.
    assert!(
        rms(&soft) > 0.1,
        "smoothing silenced the note: {}",
        rms(&soft)
    );
}

// ---------------------------------------------------------------------
// What holds for all six
// ---------------------------------------------------------------------

/// Plays the same two notes through a fresh instrument at every block size
/// and insists the output is the same.
fn check_block_invariance<I: Instrument + Default>(params: &I::Params) {
    let play = |block: usize| {
        let mut instrument = I::default();
        instrument.prepare(48_000.0, 512);
        instrument.set_params(params);
        instrument.note_on(60, 0.8);
        let mut signal = render(&mut instrument, 1_000, block);
        instrument.note_on(67, 1.0);
        signal.extend(render(&mut instrument, 1_000, block));
        instrument.note_off(60);
        signal.extend(render(&mut instrument, 1_000, block));
        signal
    };
    let reference = play(512);
    for block in [1, 2, 3, 7, 16, 17, 64, 333] {
        let signal = play(block);
        assert_eq!(signal.len(), reference.len());
        for (index, (found, wanted)) in signal.iter().zip(&reference).enumerate() {
            assert!(
                (found - wanted).abs() < 1.0e-6,
                "{} differs at sample {index} with a block of {block}: {found} against {wanted}",
                std::any::type_name::<I>(),
            );
        }
    }
}

/// An instrument with no notes must put out exact silence, and must stay
/// finite however its controls are abused.
fn check_quiet_and_finite<I: Instrument + Default>(params: &I::Params) {
    let mut instrument = I::default();
    instrument.prepare(48_000.0, 512);
    instrument.set_params(params);
    let silence = render(&mut instrument, 1_000, 64);
    assert!(
        silence.iter().all(|sample| *sample == 0.0),
        "{} is not silent without a note",
        std::any::type_name::<I>()
    );
    assert_eq!(instrument.active_voices(), 0);

    // Every control at each end of its range, and a note held throughout.
    instrument.note_on(96, 1.0);
    for extreme in [f32::NAN, f32::NEG_INFINITY, -1.0e9, 1.0e9] {
        let mut damaged = *params;
        for index in 0..I::Params::descriptors().len() {
            damaged.set(index, extreme);
        }
        instrument.set_params(&damaged);
        let signal = render(&mut instrument, 400, 37);
        assert!(
            signal.iter().all(|sample| sample.is_finite()),
            "{} put out something that is not a number",
            std::any::type_name::<I>()
        );
    }
    instrument.all_notes_off();
    render(&mut instrument, 2_000, 64);
    assert_eq!(
        instrument.active_voices(),
        0,
        "{} kept a voice after all notes off",
        std::any::type_name::<I>()
    );
    let silence = render(&mut instrument, 1_000, 64);
    assert!(silence.iter().all(|sample| *sample == 0.0));
}

/// A control moved while a note sounds must not step.
///
/// What counts as a step depends on the waveform: a staircase moves further
/// between samples than a sine does, and that is the sound rather than a
/// fault. So the measure is the note once the change has settled, and the
/// glide is not allowed to be rougher than that.
fn check_glide<I: Instrument + Default>(quiet: &I::Params, loud: &I::Params) {
    let mut instrument = I::default();
    instrument.prepare(48_000.0, 512);
    instrument.set_params(quiet);
    instrument.note_on(A4, 1.0);
    render(&mut instrument, 4_000, 64);
    instrument.set_params(loud);
    let after = render(&mut instrument, 4_000, 64);
    let settled = roughest_step(&after[2_000..]);
    let gliding = roughest_step(&after[..2_000]);
    assert!(
        gliding <= 1.5 * settled + 0.02,
        "{} stepped by {gliding} while a control moved, against {settled} once settled",
        std::any::type_name::<I>()
    );
    assert!(settled > 0.0, "nothing was sounding to measure");
}

/// Polyphony is bounded: more notes than the limit never make more voices.
fn check_polyphony<I: Instrument + Default>(params: &I::Params) {
    let mut instrument = I::default();
    instrument.prepare(48_000.0, 512);
    instrument.set_params(params);
    for key in 40..100 {
        instrument.note_on(key, 1.0);
    }
    render(&mut instrument, 256, 64);
    let sounding = instrument.active_voices();
    assert!(
        sounding > 0 && sounding <= super::MAX_POLYPHONY + 4,
        "{} opened {sounding} voices for 60 notes",
        std::any::type_name::<I>()
    );
}

#[test]
fn every_instrument_ignores_the_block_size() {
    check_block_invariance::<HarmonicStack>(&HarmonicStackParams::default());
    check_block_invariance::<PartialMorph>(&PartialMorphParams {
        morph: 0.4,
        ..PartialMorphParams::default()
    });
    check_block_invariance::<Inharmonic>(&InharmonicParams::default());
    check_block_invariance::<Resynth>(&ResynthParams::default());
    check_block_invariance::<SeedPatch>(&SeedPatchParams::default());
    check_block_invariance::<ScanSynth>(&ScanSynthParams::default());
}

#[test]
fn every_instrument_is_silent_and_finite() {
    check_quiet_and_finite::<HarmonicStack>(&HarmonicStackParams::default());
    check_quiet_and_finite::<PartialMorph>(&PartialMorphParams::default());
    check_quiet_and_finite::<Inharmonic>(&InharmonicParams::default());
    check_quiet_and_finite::<Resynth>(&ResynthParams::default());
    check_quiet_and_finite::<SeedPatch>(&SeedPatchParams::default());
    check_quiet_and_finite::<ScanSynth>(&ScanSynthParams::default());
}

#[test]
fn every_instrument_glides_a_changed_control() {
    let quiet = super::VoicingParams {
        gain: 0.05,
        ..super::VoicingParams::default()
    };
    let loud = super::VoicingParams {
        gain: 1.0,
        ..super::VoicingParams::default()
    };
    check_glide::<HarmonicStack>(
        &HarmonicStackParams {
            voicing: quiet,
            ..HarmonicStackParams::default()
        },
        &HarmonicStackParams {
            voicing: loud,
            ..HarmonicStackParams::default()
        },
    );
    check_glide::<PartialMorph>(
        &PartialMorphParams {
            voicing: quiet,
            ..PartialMorphParams::default()
        },
        &PartialMorphParams {
            voicing: loud,
            ..PartialMorphParams::default()
        },
    );
    check_glide::<Inharmonic>(
        &InharmonicParams {
            voicing: quiet,
            ..InharmonicParams::default()
        },
        &InharmonicParams {
            voicing: loud,
            ..InharmonicParams::default()
        },
    );
    check_glide::<Resynth>(
        &ResynthParams {
            voicing: quiet,
            ..ResynthParams::default()
        },
        &ResynthParams {
            voicing: loud,
            ..ResynthParams::default()
        },
    );
    check_glide::<SeedPatch>(
        &SeedPatchParams {
            voicing: quiet,
            ..SeedPatchParams::default()
        },
        &SeedPatchParams {
            voicing: loud,
            ..SeedPatchParams::default()
        },
    );
    check_glide::<ScanSynth>(
        &ScanSynthParams {
            voicing: quiet,
            ..ScanSynthParams::default()
        },
        &ScanSynthParams {
            voicing: loud,
            ..ScanSynthParams::default()
        },
    );
}

#[test]
fn a_partial_brought_in_while_a_note_sounds_fades_in() {
    // The richest spectrum is the one the note ends on, so whatever the
    // glide does on the way there has to be gentler than that.
    let quiet = HarmonicStackParams {
        partials: 8,
        gains: PartialGains::from_fn(|index| f32::from(index == 0) * 0.5),
        voicing: steady_voicing(1.0),
        ..HarmonicStackParams::default()
    };
    let loud = HarmonicStackParams {
        gains: PartialGains::from_fn(|index| if index < 8 { 0.5 } else { 0.0 }),
        ..quiet
    };
    check_glide::<HarmonicStack>(&quiet.sanitized(), &loud.sanitized());
}

#[test]
fn the_morph_control_glides() {
    // Snapshot B is the richer of the two, so it is the settled measure.
    let quiet = PartialMorphParams {
        morph: 0.0,
        voicing: steady_voicing(0.5),
        ..PartialMorphParams::default()
    };
    let loud = PartialMorphParams {
        morph: 1.0,
        ..quiet
    };
    check_glide::<PartialMorph>(&quiet.sanitized(), &loud.sanitized());
}

#[test]
fn every_instrument_bounds_its_polyphony() {
    check_polyphony::<HarmonicStack>(&HarmonicStackParams::default());
    check_polyphony::<PartialMorph>(&PartialMorphParams::default());
    check_polyphony::<Inharmonic>(&InharmonicParams::default());
    check_polyphony::<Resynth>(&ResynthParams::default());
    check_polyphony::<SeedPatch>(&SeedPatchParams::default());
    check_polyphony::<ScanSynth>(&ScanSynthParams::default());
}

#[test]
fn a_note_on_without_velocity_lets_the_note_go() {
    let mut stack = HarmonicStack::default();
    stack.prepare(48_000.0, 512);
    stack.set_params(&HarmonicStackParams::default());
    stack.note_on(A4, 1.0);
    render(&mut stack, 256, 64);
    assert_eq!(stack.active_voices(), 1);
    stack.note_on(A4, 0.0);
    render(&mut stack, 48_000, 256);
    assert_eq!(stack.active_voices(), 0);
}

#[test]
fn a_reset_silences_everything_at_once() {
    let mut stack = HarmonicStack::default();
    stack.prepare(48_000.0, 512);
    stack.set_params(&HarmonicStackParams::default());
    stack.note_on(A4, 1.0);
    render(&mut stack, 512, 64);
    stack.reset();
    assert_eq!(stack.active_voices(), 0);
    let after = render(&mut stack, 512, 64);
    assert!(after.iter().all(|sample| *sample == 0.0));
}

// ---------------------------------------------------------------------
// The parameter tables
// ---------------------------------------------------------------------

/// The value at a dotted path such as `"voicing.envelope.attackMs"`.
fn at_path<'a>(root: &'a Value, path: &str) -> &'a Value {
    path.split('.').fold(root, |value, part| {
        let next = match part.parse::<usize>() {
            Ok(index) => value.get(index),
            Err(_) => value.get(part),
        };
        next.unwrap_or_else(|| panic!("nothing at `{part}` of `{path}`"))
    })
}

/// What a control's number looks like in JSON.
fn expected_json(info: &ParamInfo, value: f32) -> Value {
    match info.kind {
        ParamKind::Float => json!(value),
        ParamKind::Integer => json!(value as i64),
        ParamKind::Toggle => json!(value >= 0.5),
        ParamKind::Choice => json!(info.choices[value as usize].value),
    }
}

/// Every check that holds for an additive instrument's parameters.
fn check_params<P: ParamSet + Serialize + DeserializeOwned>() {
    let rows = P::descriptors();
    let name = P::NAME;
    assert!(!rows.is_empty(), "{name} has no controls");

    for (index, row) in rows.iter().enumerate() {
        let id = row.id;
        assert!(
            !id.is_empty() && !row.name.is_empty(),
            "{name} row {index} is blank"
        );
        assert!(row.min < row.max, "{name} `{id}`: empty range");
        assert!(
            (row.min..=row.max).contains(&row.default),
            "{name} `{id}`: default {} outside {}..{}",
            row.default,
            row.min,
            row.max
        );
        assert_eq!(P::index_of(id), Some(index), "{name} `{id}`: duplicate id");
        if row.scale == ParamScale::Logarithmic {
            assert!(
                row.min > 0.0,
                "{name} `{id}`: a log scale needs a positive floor"
            );
        }
        if row.kind == ParamKind::Toggle {
            assert_eq!((row.min, row.max), (0.0, 1.0));
        }
        assert!(
            row.choices.is_empty(),
            "{name} `{id}`: no choices are expected"
        );
    }

    // The defaults in the table are the struct's defaults, and every path
    // leads somewhere in the JSON.
    let defaults = P::default();
    let json = serde_json::to_value(defaults).expect("the parameters serialize");
    for (index, row) in rows.iter().enumerate() {
        let value = defaults.get(index).expect("every row has a value");
        assert!(
            (value - row.default).abs() <= 1.0e-6 * row.default.abs().max(1.0),
            "{name} `{}`: the table says {} and the struct says {value}",
            row.id,
            row.default
        );
        let found = at_path(&json, row.id);
        let wanted = expected_json(row, value);
        let agrees = match (found.as_f64(), wanted.as_f64()) {
            (Some(found), Some(wanted)) => (found - wanted).abs() <= 1.0e-6 * wanted.abs().max(1.0),
            _ => *found == wanted,
        };
        assert!(
            agrees,
            "{name} `{}` is {found} in JSON, expected {wanted}",
            row.id
        );
    }

    // Writing a control writes that control and no other, and the value
    // comes back out.
    for (index, row) in rows.iter().enumerate() {
        let mut params = P::default();
        let wanted = row.min + 0.375 * (row.max - row.min);
        assert!(params.set(index, wanted));
        let found = params.get(index).expect("the value was written");
        let wanted = match row.kind {
            ParamKind::Float => wanted,
            ParamKind::Toggle => f32::from(wanted >= 0.5),
            _ => wanted.round(),
        };
        assert!(
            (found - wanted).abs() <= 1.0e-4 * wanted.abs().max(1.0),
            "{name} `{}`: wrote {wanted}, read back {found}",
            row.id
        );
        for (other, _) in rows.iter().enumerate().filter(|(other, _)| *other != index) {
            assert_eq!(
                params.get(other),
                P::default().get(other),
                "{name} `{}` also changed row {other}",
                row.id
            );
        }
    }

    assert!(
        !P::default().set(rows.len(), 0.5),
        "{name} accepted a row past the end"
    );
    assert_eq!(P::default().get(rows.len()), None);

    // Damage is forced into range.
    let mut damaged = P::default();
    for index in 0..rows.len() {
        damaged.set(index, f32::NAN);
    }
    let clean = damaged.sanitized();
    for (index, row) in rows.iter().enumerate() {
        let value = clean.get(index).expect("every row has a value");
        assert!(
            value.is_finite(),
            "{name} `{}` is {value} after cleaning",
            row.id
        );
        assert!(
            (row.min..=row.max).contains(&value),
            "{name} `{}` is {value}, outside {}..{}",
            row.id,
            row.min,
            row.max
        );
    }
    assert_eq!(
        clean.sanitized(),
        clean,
        "{name}: cleaning twice changed something"
    );

    // A glide arrives, and arrives exactly.
    let mut target = P::default();
    for (index, row) in rows.iter().enumerate() {
        target.set(index, row.max);
    }
    let mut moving = P::default();
    let mut steps = 0;
    while moving.approach(&target, 0.25) {
        steps += 1;
        assert!(steps < 10_000, "{name}: a glide never arrived");
    }
    assert_eq!(moving, target, "{name}: a glide stopped short");

    // The JSON round-trips.
    let text = serde_json::to_string(&clean).expect("the parameters serialize");
    let back: P = serde_json::from_str(&text).expect("the parameters deserialize");
    assert_eq!(back, clean, "{name} did not survive a round trip");
    let empty: P = serde_json::from_str("{}").expect("an empty object is all defaults");
    assert_eq!(
        empty,
        P::default(),
        "{name}: a missing field is not its default"
    );
}

#[test]
fn every_parameter_table_agrees_with_its_struct() {
    check_params::<HarmonicStackParams>();
    check_params::<PartialMorphParams>();
    check_params::<InharmonicParams>();
    check_params::<ResynthParams>();
    check_params::<SeedPatchParams>();
    check_params::<ScanSynthParams>();
}

#[test]
fn the_tables_are_the_size_the_banks_are() {
    let shared = super::VoicingParams::ROWS;
    assert_eq!(
        HarmonicStackParams::descriptors().len(),
        3 + MAX_PARTIALS + shared
    );
    assert_eq!(
        PartialMorphParams::descriptors().len(),
        2 + 2 * MORPH_PARTIALS + shared
    );
    assert_eq!(
        ResynthParams::descriptors().len(),
        2 + 3 * TABLE_PARTIALS + shared
    );
    assert_eq!(
        ScanSynthParams::descriptors().len(),
        SCAN_STEPS + 2 + shared
    );
    assert_eq!(InharmonicParams::descriptors().len(), 4 + shared);
    assert_eq!(SeedPatchParams::descriptors().len(), 4 + shared);
}

#[test]
fn a_short_bank_of_gains_loads_with_the_rest_silent() {
    let gains: PartialGains =
        serde_json::from_str("[1.0, 0.5, 0.25]").expect("a short array loads");
    assert_eq!(gains[0], 1.0);
    assert_eq!(gains[1], 0.5);
    assert_eq!(gains[2], 0.25);
    assert!(gains.as_slice()[3..].iter().all(|gain| *gain == 0.0));
    let text = serde_json::to_string(&gains).expect("the gains serialize");
    let back: PartialGains = serde_json::from_str(&text).expect("the gains load again");
    assert_eq!(back, gains);
    assert_eq!(
        serde_json::from_str::<Vec<f32>>(&text)
            .map(|all| all.len())
            .unwrap(),
        MAX_PARTIALS
    );
}

#[test]
fn the_six_instruments_are_six_sounds() {
    // Six ways of filling a spectrum, so six different notes. None of them
    // is another one under a different name.
    let play = |name: &'static str, signal: Vec<f32>| {
        assert!(rms(&signal) > 1.0e-3, "{name} is silent at its defaults");
        (name, signal)
    };
    let sounds = [
        play("harmonic stack", {
            let mut it = HarmonicStack::default();
            steady_note(&mut it, &HarmonicStackParams::default(), A4, HARMONIC_RATE)
        }),
        play("partial morph", {
            let mut it = PartialMorph::default();
            steady_note(&mut it, &PartialMorphParams::default(), A4, HARMONIC_RATE)
        }),
        play("inharmonic", {
            let mut it = Inharmonic::default();
            steady_note(&mut it, &InharmonicParams::default(), A4, HARMONIC_RATE)
        }),
        play("resynth", {
            let mut it = Resynth::default();
            steady_note(&mut it, &ResynthParams::default(), A4, HARMONIC_RATE)
        }),
        play("seed patch", {
            let mut it = SeedPatch::default();
            steady_note(&mut it, &SeedPatchParams::default(), A4, HARMONIC_RATE)
        }),
        play("scan synth", {
            let mut it = ScanSynth::default();
            steady_note(&mut it, &ScanSynthParams::default(), A4, HARMONIC_RATE)
        }),
    ];
    for (first, one) in &sounds {
        for (second, other) in &sounds {
            if first == second {
                continue;
            }
            let difference: Vec<f32> = one.iter().zip(other).map(|(a, b)| a - b).collect();
            let apart = rms(&difference) / rms(one).max(rms(other));
            assert!(
                apart > 0.2,
                "{first} and {second} are within {apart} of each other"
            );
        }
    }
}

#[test]
fn an_instrument_stays_a_handful_of_kilobytes() {
    use std::mem::size_of;
    // Everything is a fixed array, so the size is known. The parent will
    // be holding one of these behind a `Box`, and it should not grow by
    // accident.
    for (name, bytes) in [
        ("harmonic stack", size_of::<HarmonicStack>()),
        ("partial morph", size_of::<PartialMorph>()),
        ("inharmonic", size_of::<Inharmonic>()),
        ("resynth", size_of::<Resynth>()),
        ("seed patch", size_of::<SeedPatch>()),
        ("scan synth", size_of::<ScanSynth>()),
    ] {
        println!("{name}: {bytes} bytes");
        assert!(bytes < 16 * 1024, "{name} is {bytes} bytes");
    }
}
