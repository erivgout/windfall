//! The compressor: its static curve, its timing and its metering.

use windfall_dsp::{Compressor, CompressorParams, DetectorMode};

use crate::support::{RATE, RATES, db, noise, prepared, rms, run, run_mono, sine, square};

fn peak_params() -> CompressorParams {
    CompressorParams {
        sidechain: false,
        threshold_db: -20.0,
        ratio: 4.0,
        attack_ms: 5.0,
        release_ms: 50.0,
        knee_db: 0.0,
        makeup_db: 0.0,
        auto_makeup: false,
        detector: DetectorMode::Peak,
        mix: 1.0,
    }
}

/// A signal whose absolute value is a constant `level_db`, which makes the
/// compressor's steady state exact.
fn steady(level_db: f32, frames: usize) -> Vec<f32> {
    square(96, 10.0_f32.powf(level_db / 20.0), frames)
}

/// The gain in dB the compressor settles on for a steady signal.
fn settled_gain_db(params: &CompressorParams, level_db: f32) -> f64 {
    let mut compressor: Compressor = prepared(params, RATE);
    let input = steady(level_db, 96_000);
    let output = run_mono(&mut compressor, &input);
    db(rms(&output[72_000..]) / rms(&input[72_000..]))
}

#[test]
fn static_curve_follows_threshold_ratio_and_knee() {
    let mut cases = Vec::new();
    for ratio in [1.0, 2.0, 4.0, 10.0, 100.0] {
        for knee_db in [0.0, 6.0, 24.0] {
            cases.push(CompressorParams {
                ratio,
                knee_db,
                ..peak_params()
            });
        }
    }
    for params in &cases {
        for level_db in [-50.0, -30.0, -23.0, -20.0, -17.0, -10.0, 0.0] {
            let measured = settled_gain_db(params, level_db);
            let expected = f64::from(params.static_gain_db(level_db));
            assert!(
                (measured - expected).abs() < 0.03,
                "ratio {} knee {} at {level_db} dB: {measured} against {expected}",
                params.ratio,
                params.knee_db
            );
        }
    }
}

#[test]
fn static_curve_is_the_textbook_formula() {
    let hard = peak_params();
    assert_eq!(hard.static_gain_db(-40.0), 0.0);
    assert_eq!(hard.static_gain_db(-20.0), 0.0);
    // 12 dB over the threshold at 4:1 comes out 3 dB over: 9 dB less.
    assert!((hard.static_gain_db(-8.0) + 9.0).abs() < 1.0e-5);

    // At the top ratio nothing rises above the threshold.
    let limiting = CompressorParams {
        ratio: 100.0,
        ..peak_params()
    };
    assert!((limiting.static_gain_db(-5.0) + 15.0).abs() < 1.0e-5);

    // A ratio of 1 never changes anything.
    let off = CompressorParams {
        ratio: 1.0,
        ..peak_params()
    };
    assert_eq!(off.static_gain_db(0.0), 0.0);

    // A 12 dB knee starts 6 dB under the threshold, meets the hard curve
    // 6 dB over it, and is a quarter of the way there at the threshold.
    let soft = CompressorParams {
        knee_db: 12.0,
        ..peak_params()
    };
    assert_eq!(soft.static_gain_db(-26.0), 0.0);
    assert!((soft.static_gain_db(-14.0) - hard.static_gain_db(-14.0)).abs() < 1.0e-5);
    assert!((soft.static_gain_db(-20.0) + 0.75 * 1.5).abs() < 1.0e-5);
    assert!(soft.static_gain_db(-23.0) < 0.0 && soft.static_gain_db(-23.0) > -0.5);
}

#[test]
fn signals_under_the_threshold_pass_untouched() {
    let mut compressor: Compressor = prepared(&peak_params(), RATE);
    let input = noise(1, 0.05, 8_000);
    let output = run_mono(&mut compressor, &input);
    for (output, input) in output.iter().zip(&input) {
        assert!((output - input).abs() < 1.0e-7);
    }
}

/// Gain in dB the compressor applies to each sample of a steady signal
/// that steps from `from_db` to `to_db` after one second, for the four
/// seconds after the step.
fn step_response(params: &CompressorParams, from_db: f32, to_db: f32, rate: f32) -> Vec<f64> {
    let second = rate as usize;
    let mut input = steady(from_db, second);
    input.extend(steady(to_db, 4 * second));
    let mut compressor: Compressor = prepared(params, rate);
    let output = run_mono(&mut compressor, &input);
    output[second..]
        .iter()
        .zip(&input[second..])
        .map(|(output, input)| db(f64::from(output.abs()) / f64::from(input.abs())))
        .collect()
}

/// Milliseconds until the gain has covered 63% of the way from where it
/// starts to where it ends.
fn time_to_63_percent(gains: &[f64], rate: f32) -> f64 {
    let (start, end) = (gains[0], gains[gains.len() - 1]);
    let target = start + (end - start) * (1.0 - (-1.0_f64).exp());
    let rising = end > start;
    let position = gains
        .iter()
        .position(|gain| {
            if rising {
                *gain >= target
            } else {
                *gain <= target
            }
        })
        .unwrap();
    position as f64 * 1_000.0 / f64::from(rate)
}

#[test]
fn attack_and_release_take_their_set_times_at_every_sample_rate() {
    for rate in RATES {
        for (attack_ms, release_ms) in [(1.0, 20.0), (10.0, 100.0), (50.0, 400.0)] {
            let params = CompressorParams {
                attack_ms,
                release_ms,
                ..peak_params()
            };
            let attack = step_response(&params, -40.0, -4.0, rate);
            // On the first sample of the step the gain has barely moved.
            assert!(attack[0] > -1.0);
            assert!((attack[attack.len() - 1] + 12.0).abs() < 0.01);
            let measured = time_to_63_percent(&attack, rate);
            assert!(
                (measured / f64::from(attack_ms) - 1.0).abs() < 0.05,
                "attack {attack_ms} ms at {rate}: took {measured} ms"
            );

            let release = step_response(&params, -4.0, -40.0, rate);
            assert!(release[release.len() - 1].abs() < 0.01);
            let measured = time_to_63_percent(&release, rate);
            assert!(
                (measured / f64::from(release_ms) - 1.0).abs() < 0.05,
                "release {release_ms} ms at {rate}: took {measured} ms"
            );
        }
    }
}

#[test]
fn rms_detection_reads_a_sine_three_decibels_under_its_peak() {
    let rms_params = CompressorParams {
        detector: DetectorMode::Rms,
        attack_ms: 20.0,
        release_ms: 200.0,
        ..peak_params()
    };
    for peak_db in [-12.0_f32, -6.0, 0.0] {
        let mut compressor: Compressor = prepared(&rms_params, RATE);
        let input = sine(1_000.0, 10.0_f32.powf(peak_db / 20.0), 96_000, RATE);
        let output = run_mono(&mut compressor, &input);
        let measured = db(rms(&output[72_000..]) / rms(&input[72_000..]));
        let expected = f64::from(rms_params.static_gain_db(peak_db - 3.01));
        assert!(
            (measured - expected).abs() < 0.15,
            "{peak_db} dB peak: {measured} against {expected}"
        );
    }
    // The same sine in peak mode is compressed harder, since its peaks are
    // 3 dB over its RMS level.
    let peak_gain = {
        let mut compressor: Compressor = prepared(&peak_params(), RATE);
        let input = sine(1_000.0, 0.5, 96_000, RATE);
        let output = run_mono(&mut compressor, &input);
        db(rms(&output[72_000..]) / rms(&input[72_000..]))
    };
    let rms_gain = {
        let mut compressor: Compressor = prepared(&rms_params, RATE);
        let input = sine(1_000.0, 0.5, 96_000, RATE);
        let output = run_mono(&mut compressor, &input);
        db(rms(&output[72_000..]) / rms(&input[72_000..]))
    };
    assert!(peak_gain < rms_gain - 0.5, "{peak_gain} against {rms_gain}");
}

#[test]
fn makeup_gain_is_added_after_compression() {
    let params = CompressorParams {
        makeup_db: 7.0,
        ..peak_params()
    };
    assert!((settled_gain_db(&params, -40.0) - 7.0).abs() < 0.01);
    assert!((settled_gain_db(&params, -8.0) - (7.0 - 9.0)).abs() < 0.03);
    assert_eq!(params.total_makeup_db(), 7.0);
}

#[test]
fn auto_makeup_restores_half_of_what_full_scale_loses() {
    let params = CompressorParams {
        auto_makeup: true,
        makeup_db: 1.0,
        ..peak_params()
    };
    // Full scale is 20 dB over the threshold and loses 15 dB at 4:1.
    assert!((params.total_makeup_db() - 8.5).abs() < 1.0e-4);
    assert!((settled_gain_db(&params, -40.0) - 8.5).abs() < 0.01);
    assert!((settled_gain_db(&params, 0.0) - (8.5 - 15.0)).abs() < 0.03);
}

#[test]
fn mix_blends_the_dry_signal_back_in() {
    let input = steady(-8.0, 48_000);
    let dry = CompressorParams {
        mix: 0.0,
        ..peak_params()
    };
    let mut compressor: Compressor = prepared(&dry, RATE);
    assert!(run_mono(&mut compressor, &input) == input);

    let half = CompressorParams {
        mix: 0.5,
        ..peak_params()
    };
    // Fully wet this signal settles 9 dB down. Half of that and half of
    // the dry signal add up.
    let wet_gain = 10.0_f64.powf(-9.0 / 20.0);
    let expected = db(0.5 + 0.5 * wet_gain);
    assert!((settled_gain_db(&half, -8.0) - expected).abs() < 0.03);
}

#[test]
fn both_channels_get_the_gain_the_louder_one_needs() {
    let mut compressor: Compressor = prepared(&peak_params(), RATE);
    let mut left = steady(-8.0, 48_000);
    let mut right = steady(-40.0, 48_000);
    let (in_left, in_right) = (left.clone(), right.clone());
    run(&mut compressor, &mut left, &mut right, 256);
    let left_gain = db(rms(&left[40_000..]) / rms(&in_left[40_000..]));
    let right_gain = db(rms(&right[40_000..]) / rms(&in_right[40_000..]));
    assert!((left_gain + 9.0).abs() < 0.03);
    assert!((right_gain - left_gain).abs() < 1.0e-4);
}

#[test]
fn the_meter_reports_the_deepest_reduction_since_it_was_read() {
    let mut compressor: Compressor = prepared(&peak_params(), RATE);
    let meter = compressor.meter();
    assert_eq!(meter.take_db(), 0.0);

    let quiet = steady(-40.0, 4_800);
    run_mono(&mut compressor, &quiet);
    assert_eq!(meter.take_db(), 0.0);

    let loud = steady(-8.0, 48_000);
    run_mono(&mut compressor, &loud);
    assert!((meter.take_db() - 9.0).abs() < 0.03);
    // Reading clears it. The reduction is still releasing, so the next
    // short block reads a little less, and silence reads less again.
    run_mono(&mut compressor, &quiet[..480]);
    let releasing = meter.take_db();
    assert!(releasing > 1.0 && releasing < 9.0, "{releasing}");
    run_mono(&mut compressor, &steady(-40.0, 96_000));
    meter.take_db();
    run_mono(&mut compressor, &quiet);
    assert!(meter.take_db() < 0.01);
}

#[test]
fn a_drum_like_burst_is_caught_by_the_attack_and_let_go_by_the_release() {
    // Bursts of a loud tone over a quiet one, like a drum over a pad.
    let frames = 48_000;
    let input: Vec<f32> = (0..frames)
        .map(|n| {
            let tone = (n as f32 * 0.05).sin();
            let burst = n % 12_000 < 2_400;
            tone * if burst { 0.8 } else { 0.05 }
        })
        .collect();
    let params = CompressorParams {
        attack_ms: 2.0,
        release_ms: 30.0,
        ..peak_params()
    };
    let mut compressor: Compressor = prepared(&params, RATE);
    let output = run_mono(&mut compressor, &input);
    let level = |signal: &[f32], from: usize, to: usize| rms(&signal[from..to]);
    // Well into a burst the gain has come down by close to the static
    // amount: -1.9 dB peak is 18 dB over, so about 13.5 dB.
    let burst_gain = db(level(&output, 13_200, 14_400) / level(&input, 13_200, 14_400));
    assert!((-14.0..=-11.0).contains(&burst_gain), "{burst_gain}");
    // The quiet part just after a burst is still held down, and by the end
    // of the gap it is back to unity.
    let after = db(level(&output, 14_500, 15_000) / level(&input, 14_500, 15_000));
    assert!(after < -6.0, "{after}");
    let recovered = db(level(&output, 22_000, 24_000) / level(&input, 22_000, 24_000));
    assert!(recovered.abs() < 0.3, "{recovered}");
}
