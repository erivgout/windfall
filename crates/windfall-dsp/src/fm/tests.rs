use std::f64::consts::TAU;

use serde::{Serialize, de::DeserializeOwned};

use super::common::{Bank, Controls, Voice};
use super::*;
use crate::instrument::Instrument;
use crate::param::{ParamKind, ParamSet};

const RATE: f32 = 48_000.0;

fn render<I: Instrument>(instrument: &mut I, frames: usize) -> Vec<f32> {
    // Also checks replacement of corrupt pre-existing output contents.
    let mut left = vec![f32::NAN; frames];
    let mut right = vec![f32::INFINITY; frames];
    instrument.process(&mut left, &mut right);
    assert!(left.iter().all(|v| v.is_finite()));
    assert_eq!(left, right);
    left
}

fn rms(samples: &[f32]) -> f64 {
    (samples.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / samples.len() as f64).sqrt()
}

fn magnitude(samples: &[f32], frequency: f64) -> f64 {
    let mut real = 0.0;
    let mut imaginary = 0.0;
    for (i, value) in samples.iter().enumerate() {
        let phase = TAU * frequency * i as f64 / f64::from(RATE);
        real += f64::from(*value) * phase.cos();
        imaginary += f64::from(*value) * phase.sin();
    }
    (real * real + imaginary * imaginary).sqrt() / samples.len() as f64
}

fn spectrum<I: Instrument + Default>(params: I::Params) -> [f64; 16] {
    let mut instrument = I::default();
    instrument.prepare(RATE, 128);
    instrument.set_params(&params);
    instrument.note_on(69, 1.0);
    let audio = render(&mut instrument, 14_400);
    let mut bins = std::array::from_fn(|i| magnitude(&audio[9_600..], 440.0 * (i + 1) as f64));
    let sum = bins.iter().sum::<f64>();
    assert!(sum > 0.001);
    for bin in &mut bins {
        *bin /= sum;
    }
    bins
}

fn spectrum_distance(a: [f64; 16], b: [f64; 16]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum()
}

fn sound_contract<I: Instrument + Default>() {
    let mut instrument = I::default();
    instrument.prepare(RATE, 128);
    instrument.process(&mut [], &mut []);
    assert_eq!(instrument.active_voices(), 0);
    instrument.note_on(69, 1.0);
    let audio = render(&mut instrument, 12_000);
    let steady = &audio[7_200..];
    let level = rms(steady);
    assert!((0.015..0.5).contains(&level), "RMS {level}");
    let crossings = steady
        .windows(2)
        .filter(|pair| pair[0] <= 0.0 && pair[1] > 0.0)
        .count();
    let frequency = crossings as f32 * RATE / steady.len() as f32;
    assert!(
        (frequency - 440.0).abs() < 12.0,
        "A4 measured {frequency} Hz"
    );
    assert!(magnitude(steady, 440.0) > 10.0 * magnitude(steady, 880.0));
    instrument.note_off(69);
    let tail = render(&mut instrument, 16_000);
    assert!(tail[..1_000].iter().any(|sample| sample.abs() > 0.001));
    assert!(tail[8_000..].iter().all(|sample| *sample == 0.0));
    assert_eq!(instrument.active_voices(), 0);
    instrument.note_on(69, 0.8);
    assert!(
        render(&mut instrument, 512)
            .iter()
            .any(|sample| sample.abs() > 0.001)
    );
    instrument.reset();
    assert_eq!(instrument.active_voices(), 0);
    assert!(
        render(&mut instrument, 512)
            .iter()
            .all(|sample| *sample == 0.0)
    );
    instrument.note_on(69, 1.0);
    render(&mut instrument, 256);
    instrument.all_notes_off();
    assert!(
        render(&mut instrument, 800)[241..]
            .iter()
            .all(|sample| *sample == 0.0)
    );
    assert_eq!(instrument.active_voices(), 0);
    for key in 0..48 {
        instrument.note_on(key, 1.0);
    }
    assert_eq!(instrument.active_voices(), MAX_POLYPHONY);
    assert!(
        render(&mut instrument, 64)
            .iter()
            .all(|sample| sample.is_finite())
    );
    instrument.process(&mut [], &mut []);
}

fn process_parts<I: Instrument>(
    instrument: &mut I,
    left: &mut [f32],
    right: &mut [f32],
    sizes: &[usize],
) {
    let mut offset = 0;
    for &size in sizes.iter().cycle() {
        if offset == left.len() {
            break;
        }
        let end = (offset + size).min(left.len());
        instrument.process(&mut left[offset..end], &mut right[offset..end]);
        offset = end;
    }
}

fn partition_contract<I: Instrument + Default>(initial: I::Params, changed: I::Params) {
    let mut whole = I::default();
    let mut split = I::default();
    // Pre-prepare settings must survive prepare and apply immediately.
    for instrument in [&mut whole, &mut split] {
        instrument.set_params(&initial);
        instrument.prepare(RATE, 1);
        instrument.note_on(69, 0.8);
    }
    let mut a = [0.0; 2_048];
    let mut ar = a;
    let mut b = a;
    let mut br = a;
    let events = [0, 197, 701, 913, 2_048];
    for pair in events.windows(2) {
        let start = pair[0];
        let end = pair[1];
        if start == 197 {
            whole.set_params(&changed);
            split.set_params(&changed);
        }
        if start == 701 {
            whole.note_on(76, 0.4);
            split.note_on(76, 0.4);
        }
        if start == 913 {
            whole.note_off(69);
            split.note_off(69);
        }
        whole.process(&mut a[start..end], &mut ar[start..end]);
        process_parts(
            &mut split,
            &mut b[start..end],
            &mut br[start..end],
            &[1, 17, 3, 64, 7],
        );
    }
    assert_eq!(a, b);
    assert_eq!(ar, br);
    assert_eq!(whole.active_voices(), split.active_voices());
}

fn params_contract<P: ParamSet + Serialize + DeserializeOwned>() {
    let default = P::default();
    assert_eq!(default.sanitized(), default);
    assert_eq!(serde_json::from_str::<P>("{}").unwrap(), default);
    let json = serde_json::to_value(default).unwrap();
    assert_eq!(serde_json::from_value::<P>(json.clone()).unwrap(), default);
    for (i, info) in P::descriptors().iter().enumerate() {
        assert_eq!(P::index_of(info.id), Some(i));
        assert_eq!(default.get(i), Some(info.default), "{}", info.id);
        let mut cursor = &json;
        for segment in info.id.split('.') {
            cursor = if let Ok(index) = segment.parse::<usize>() {
                &cursor[index]
            } else {
                &cursor[segment]
            };
        }
        if info.kind == ParamKind::Choice {
            assert_eq!(
                cursor.as_str(),
                Some(info.choices[info.default as usize].value)
            );
        } else {
            assert_eq!(cursor.as_f64().unwrap() as f32, info.default, "{}", info.id);
        }
        for (value, expected) in [(-f32::MAX, info.min), (f32::MAX, info.max)] {
            let mut params = default;
            assert!(params.set(i, value));
            assert_eq!(params.get(i), Some(expected));
            assert_eq!(params, params.sanitized());
        }
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut params = default;
            assert!(params.set(i, value));
            assert_eq!(params, default, "invalid {}", info.id);
        }
    }
    let mut params = default;
    assert_eq!(params.get(P::descriptors().len()), None);
    assert!(!params.set(P::descriptors().len(), 1.0));
}

fn extreme_contract<I: Instrument + Default>() {
    for rate in [f32::NAN, f32::INFINITY, -f32::MAX, 1.0, RATE, 384_000.0] {
        let mut instrument = I::default();
        instrument.prepare(rate, 0);
        for extreme in [
            f32::MAX,
            -f32::MAX,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ] {
            let mut params = I::Params::default();
            for i in 0..I::Params::descriptors().len() {
                params.set(i, extreme);
            }
            instrument.set_params(&params);
            instrument.set_tempo(extreme);
            instrument.note_on(255, extreme);
            instrument.note_on(0, 1.0);
            instrument.note_on(127, 1.0);
            render(&mut instrument, 257);
            instrument.all_notes_off();
            instrument.reset();
        }
    }
}

#[test]
fn four_op_pitch_release_reset_empty_and_polyphony() {
    sound_contract::<FourOp>();
}
#[test]
fn matrix_fm_pitch_release_reset_empty_and_polyphony() {
    sound_contract::<MatrixFm>();
}
#[test]
fn ring_hybrid_pitch_release_reset_empty_and_polyphony() {
    sound_contract::<RingHybrid>();
}

#[test]
fn four_op_partition_invariance_with_automation_and_notes() {
    let mut initial = FourOpParams::default();
    initial.operators[1].level = 1.5;
    initial.operators[1].ratio = 2.0;
    let mut changed = initial;
    changed.algorithm = FourOpAlgorithm::Parallel;
    changed.feedback = 3.0;
    changed.gain = 0.4;
    changed.operators[3].level = 1.0;
    changed.operators[0].envelope.sustain = 0.2;
    partition_contract::<FourOp>(initial, changed);
}

#[test]
fn matrix_fm_partition_invariance_with_automation_and_notes() {
    let mut initial = MatrixFmParams::default();
    initial.operators[5].level = 1.0;
    initial.operators[5].ratio = 2.0;
    initial.matrix[0][5] = 2.0;
    let mut changed = initial;
    changed.matrix[5][0] = -2.5;
    changed.operators[0].feedback = 1.0;
    changed.operators[5].key_scaling = -0.5;
    changed.gain = 0.4;
    partition_contract::<MatrixFm>(initial, changed);
}

#[test]
fn ring_hybrid_partition_invariance_with_automation_and_notes() {
    let mut initial = RingHybridParams::default();
    initial.oscillators[1].level = 0.6;
    initial.oscillators[1].ratio = 2.0;
    let mut changed = initial;
    changed.oscillators[0].waveform = HybridWaveform::Saw;
    changed.fm_21 = 2.0;
    changed.ring_12 = 0.7;
    changed.resonance = 0.8;
    changed.cutoff_hz = 700.0;
    changed.gain = 0.4;
    partition_contract::<RingHybrid>(initial, changed);
}

#[test]
fn four_op_sanitizer_descriptors_and_serde() {
    params_contract::<FourOpParams>();
    let mut params = FourOpParams::default();
    params.operators[3].level = f32::NAN;
    params.operators[2].ratio = f32::MAX;
    params.feedback = -10.0;
    params.operators[0].envelope.release_ms = f32::INFINITY;
    let clean = params.sanitized();
    assert_eq!(clean.operators[3].level, 0.0);
    assert_eq!(clean.operators[2].ratio, 32.0);
    assert_eq!(clean.feedback, 0.0);
    assert_eq!(clean.operators[0].envelope.release_ms, 120.0);
    extreme_contract::<FourOp>();
}

#[test]
fn matrix_fm_sanitizer_descriptors_and_serde() {
    params_contract::<MatrixFmParams>();
    let mut params = MatrixFmParams::default();
    params.matrix[5][4] = f32::NAN;
    params.matrix[0][5] = -f32::MAX;
    params.operators[5].key_scaling = f32::MAX;
    params.operators[5].feedback = f32::INFINITY;
    let clean = params.sanitized();
    assert_eq!(clean.matrix[5][4], 0.0);
    assert_eq!(clean.matrix[0][5], -8.0);
    assert_eq!(clean.operators[5].key_scaling, 1.0);
    assert_eq!(clean.operators[5].feedback, 0.0);
    extreme_contract::<MatrixFm>();
}

#[test]
fn ring_hybrid_sanitizer_descriptors_and_serde() {
    params_contract::<RingHybridParams>();
    let mut params = RingHybridParams {
        cutoff_hz: -f32::MAX,
        resonance: f32::NAN,
        fm_32: f32::INFINITY,
        ring_13: f32::MAX,
        ..RingHybridParams::default()
    };
    params.oscillators[2].ratio = f32::MAX;
    let clean = params.sanitized();
    assert_eq!(clean.cutoff_hz, 20.0);
    assert_eq!(clean.resonance, 0.0);
    assert_eq!(clean.oscillators[2].ratio, 32.0);
    assert_eq!(clean.fm_32, 0.0);
    assert_eq!(clean.ring_13, 1.0);
    extreme_contract::<RingHybrid>();
}

#[test]
fn all_eight_four_op_algorithms_have_distinct_normalized_spectra() {
    use FourOpAlgorithm::*;
    let mut params = FourOpParams::default();
    for (i, op) in params.operators.iter_mut().enumerate() {
        op.ratio = (i + 1) as f32;
        op.level = [1.0, 1.8, 1.4, 1.2][i];
        op.envelope.sustain = 1.0;
    }
    let spectra = [Stack, Pairs, FanIn, FanOut, Branch, Merge, Triple, Parallel].map(|algorithm| {
        params.algorithm = algorithm;
        spectrum::<FourOp>(params)
    });
    for i in 0..8 {
        for j in 0..i {
            let distance = spectrum_distance(spectra[i], spectra[j]);
            assert!(distance > 0.05, "algorithms {i} and {j}: {distance}");
        }
    }
    params.algorithm = Parallel;
    let plain = spectrum::<FourOp>(params);
    params.feedback = 2.0;
    assert!(spectrum_distance(plain, spectrum::<FourOp>(params)) > 0.1);
}

#[test]
fn matrix_routes_operator_six_cycles_and_feedback_change_spectrum() {
    let mut params = MatrixFmParams::default();
    params.operators[5].level = 1.0;
    params.operators[5].ratio = 2.0;
    let plain = spectrum::<MatrixFm>(params);
    params.matrix[0][5] = 2.0;
    let routed = spectrum::<MatrixFm>(params);
    assert!(spectrum_distance(plain, routed) > 0.3);
    params.matrix[5][0] = -1.5;
    let cyclic = spectrum::<MatrixFm>(params);
    assert!(spectrum_distance(routed, cyclic) > 0.1);
    params.operators[5].feedback = 2.0;
    assert!(spectrum_distance(cyclic, spectrum::<MatrixFm>(params)) > 0.1);
}

#[test]
fn matrix_key_scaling_changes_level_by_one_octave_per_octave() {
    let mut params = MatrixFmParams::default();
    params.operators[0].key_scaling = 1.0;
    let mut instrument = MatrixFm::default();
    instrument.prepare(RATE, 128);
    instrument.set_params(&params);
    instrument.note_on(69, 1.0);
    let base = rms(&render(&mut instrument, 12_000)[7_200..]);
    instrument.reset();
    instrument.note_on(81, 1.0);
    let upper = rms(&render(&mut instrument, 12_000)[7_200..]);
    assert!((upper / base - 2.0).abs() < 0.01);
}

#[test]
fn ring_hybrid_fm_ring_and_filter_change_normalized_spectrum() {
    let mut params = RingHybridParams::default();
    params.oscillators[1].ratio = 2.0;
    params.oscillators[1].level = 0.75;
    params.oscillators[2].ratio = 3.0;
    params.oscillators[2].level = 0.4;
    let plain = spectrum::<RingHybrid>(params);
    params.fm_21 = 2.0;
    params.fm_31 = -0.4;
    params.fm_32 = 0.7;
    let fm = spectrum::<RingHybrid>(params);
    assert!(spectrum_distance(plain, fm) > 0.2);
    params.ring_12 = 0.9;
    params.ring_13 = 0.7;
    let ring = spectrum::<RingHybrid>(params);
    assert!(spectrum_distance(fm, ring) > 0.2);
    params.cutoff_hz = 650.0;
    params.resonance = 0.4;
    assert!(spectrum_distance(ring, spectrum::<RingHybrid>(params)) > 0.2);
}

#[test]
fn oldest_voice_is_stolen_and_releasing_voices_remain_bounded() {
    let mut bank = Bank::<6>::default();
    for key in 60..76 {
        bank.note_on(key, 1.0);
    }
    bank.note_off(61);
    bank.note_on(76, 1.0);
    assert_eq!(bank.active(), MAX_POLYPHONY);
    assert!(
        !bank
            .voices
            .iter()
            .any(|voice| voice.active && voice.key == 60)
    );
    assert!(
        bank.voices
            .iter()
            .any(|voice| voice.active && voice.key == 61)
    );
    assert!(
        bank.voices
            .iter()
            .any(|voice| voice.active && voice.key == 76)
    );
}

#[test]
fn controls_snap_initial_settings_and_finish_a_glide_in_five_ms() {
    let mut controls = Controls::<FourOpParams>::default();
    controls.prepare(RATE);
    let mut params = FourOpParams {
        gain: 0.4,
        ..FourOpParams::default()
    };
    controls.set(&params);
    assert_eq!(controls.tick().gain, 0.4);
    params.gain = 0.8;
    controls.set(&params);
    for _ in 0..120 {
        controls.tick();
    }
    assert!((controls.current.gain - 0.6).abs() < 0.0001);
    for _ in 0..120 {
        controls.tick();
    }
    assert_eq!(controls.current.gain, 0.8);
    controls.reset();
    params.gain = 0.1;
    controls.set(&params);
    assert_eq!(controls.tick().gain, 0.1);
}
const HARMONICS: usize = 21;

/// The steady part of one operator pair, `modulator -> carrier`, at a
/// modulation depth of `index` radians on key 69.
fn pair_audio(carrier: i32, modulator: i32, index: f32) -> Vec<f32> {
    let mut params = FourOpParams {
        // Stack is 4 -> 3 -> 2 -> 1. Silencing 3 and 4 leaves the pair
        // 2 -> 1.
        algorithm: FourOpAlgorithm::Stack,
        gain: 1.0,
        ..FourOpParams::default()
    };
    for operator in &mut params.operators {
        operator.level = 0.0;
        // A flat envelope, so what is measured is the spectrum itself and
        // not the sidebands a moving modulator level sweeps through.
        operator.envelope = FmEnvelopeParams {
            attack_ms: 0.5,
            decay_ms: 1.0,
            sustain: 1.0,
            release_ms: 0.5,
        };
    }
    params.operators[0].ratio = carrier as f32;
    params.operators[0].level = 1.0;
    params.operators[1].ratio = modulator as f32;
    // An operator's level is its modulation depth in radians.
    params.operators[1].level = index;

    let mut instrument = FourOp::default();
    instrument.prepare(RATE, 128);
    instrument.set_params(&params);
    instrument.note_on(69, 1.0);
    let audio = render(&mut instrument, 14_400);
    // 4800 samples hold exactly 44 cycles of 440 Hz, so every harmonic of
    // 440 falls on a bin centre and leaks nothing into its neighbours.
    audio[9_600..].to_vec()
}

/// The ratios measured below. No two sidebands of a pair may land on the
/// same bin, or they could cancel there and the checks would be reading the
/// wrong thing: `|c + k m| = |c + k' m|` for `k != k'` needs `2c` to be a
/// multiple of `m`.
const PAIRS: [(i32, i32, f32); 3] = [(8, 3, 1.5), (9, 4, 1.5), (6, 5, 1.2)];

/// Phase modulation of one sine by another puts energy at the carrier and
/// at `carrier +- k * modulator` for whole `k`, and nowhere else. Key 69 is
/// 440 Hz, so with whole ratios every component is a harmonic of 440.
#[test]
fn four_op_sidebands_sit_on_the_carrier_modulator_grid() {
    for (carrier, modulator, index) in PAIRS {
        assert_ne!(2 * carrier % modulator, 0, "ratios collide");
        let steady = pair_audio(carrier, modulator, index);
        let bins: [f64; HARMONICS] =
            std::array::from_fn(|i| magnitude(&steady, 440.0 * (i + 1) as f64));
        let loudest = bins.iter().copied().fold(0.0_f64, f64::max);
        let on_grid = |harmonic: i32| {
            (harmonic - carrier) % modulator == 0 || (-harmonic - carrier) % modulator == 0
        };

        // The carrier and the first two sideband pairs carry real energy.
        for order in -2..=2_i32 {
            let harmonic = (carrier + order * modulator).abs();
            assert!(
                (1..=HARMONICS as i32).contains(&harmonic),
                "{carrier}:{modulator} order {order} left the measured range"
            );
            let relative = bins[harmonic as usize - 1] / loudest;
            assert!(
                relative > 0.01,
                "{carrier}:{modulator} order {order} at {harmonic} x 440 Hz is {relative:.2e} \
                 of the loudest component"
            );
        }

        // Everything off the grid is at the arithmetic floor: the only
        // energy there is f32 phase-accumulator jitter.
        for harmonic in 1..=HARMONICS as i32 {
            if on_grid(harmonic) {
                continue;
            }
            let relative = bins[harmonic as usize - 1] / loudest;
            assert!(
                relative < 1.0e-5,
                "{carrier}:{modulator} has {relative:.2e} of its energy off the grid at \
                 {harmonic} x 440 Hz"
            );
        }
    }
}

/// The sidebands are spaced by the modulator's frequency, not the
/// carrier's: there is a component one whole spacing out from the carrier
/// and nothing a quarter of a spacing out.
#[test]
fn four_op_sideband_spacing_follows_the_modulator() {
    for (carrier, modulator, index) in PAIRS {
        let steady = pair_audio(carrier, modulator, index);
        let centre = f64::from(carrier) * 440.0;
        let spacing = f64::from(modulator) * 440.0;
        let at_centre = magnitude(&steady, centre);
        for side in [-1.0, 1.0] {
            // A quarter spacing off is not a bin centre, so this floor is
            // set by the skirts of the real components, not by precision.
            let quarter = magnitude(&steady, centre + side * spacing * 0.25);
            let sideband = magnitude(&steady, centre + side * spacing);
            assert!(
                at_centre > 10.0 * quarter && sideband > 10.0 * quarter,
                "{carrier}:{modulator} on side {side}: carrier {at_centre:.2e}, sideband \
                 {sideband:.2e}, quarter-spacing floor {quarter:.2e}"
            );
        }
    }
}

/// Every voice lives in a fixed array sized at compile time, so eight notes
/// at once need nothing from the allocator and more notes than there are
/// slots cannot grow it.
#[test]
fn eight_voices_run_from_the_preallocated_bank() {
    assert_eq!(
        size_of::<Bank<6>>(),
        MAX_POLYPHONY * size_of::<Voice<6>>(),
        "the bank is no longer a flat array of voices"
    );
    preallocated_contract::<FourOp>();
    preallocated_contract::<MatrixFm>();
    preallocated_contract::<RingHybrid>();
}

fn preallocated_contract<I: Instrument + Default>() {
    let mut instrument = I::default();
    instrument.prepare(RATE, 256);
    for key in 60..68 {
        instrument.note_on(key, 1.0);
    }
    assert_eq!(instrument.active_voices(), 8);
    let audio = render(&mut instrument, 4_096);
    assert!(audio.iter().all(|sample| sample.is_finite()));
    assert_eq!(instrument.active_voices(), 8);

    // Past the last slot, a new note takes one over instead of adding one.
    for key in 68..120 {
        instrument.note_on(key, 1.0);
    }
    assert_eq!(instrument.active_voices(), MAX_POLYPHONY);
    render(&mut instrument, 1_024);
    assert_eq!(instrument.active_voices(), MAX_POLYPHONY);
}
