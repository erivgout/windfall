//! The reverb, judged by measuring its impulse response.

use windfall_dsp::blocks::biquad::{Biquad, BiquadCoeffs};
use windfall_dsp::{Reverb, ReverbParams};

use crate::support::{
    RATE, RATES, assert_finite, db, impulse, mean, noise, peak, prepared, rms, run, silence,
    windowed_magnitudes,
};

/// The tail on its own: fully wet, no early reflections, nothing filtered
/// out on the way in, and no treble damping.
fn bare(decay_s: f32, size: f32) -> ReverbParams {
    ReverbParams {
        size,
        decay_s,
        pre_delay_ms: 0.0,
        damping: 0.0,
        early_level: 0.0,
        low_cut_hz: 20.0,
        high_cut_hz: 20_000.0,
        mix: 1.0,
        ..ReverbParams::default()
    }
}

fn impulse_response(params: &ReverbParams, rate: f32, seconds: f32) -> (Vec<f32>, Vec<f32>) {
    let frames = (seconds * rate) as usize;
    let mut reverb: Reverb = prepared(params, rate);
    let mut left = impulse(frames, 0, 1.0);
    let mut right = impulse(frames, 0, 1.0);
    run(&mut reverb, &mut left, &mut right, 256);
    (left, right)
}

/// Decay time in seconds from an impulse response: Schroeder's backward
/// integration, then the slope between 5 and 35 dB down, scaled to 60 dB.
fn rt60(response: &[f32], rate: f32) -> f64 {
    let mut remaining = vec![0.0_f64; response.len() + 1];
    for index in (0..response.len()).rev() {
        remaining[index] = remaining[index + 1] + f64::from(response[index]).powi(2);
    }
    let total = remaining[0];
    let reach = |level_db: f64| {
        remaining
            .iter()
            .position(|energy| 10.0 * (energy / total).max(1.0e-30).log10() <= level_db)
            .expect("the response is too short to measure") as f64
    };
    2.0 * (reach(-35.0) - reach(-5.0)) / f64::from(rate)
}

/// Keeps a band around `frequency`, two octaves wide at the most.
fn band_limited(signal: &[f32], frequency: f32, rate: f32) -> Vec<f32> {
    let coeffs = BiquadCoeffs::band_pass(frequency, 2.0, rate);
    let (mut first, mut second) = (Biquad::default(), Biquad::default());
    signal
        .iter()
        .map(|sample| second.tick(&coeffs, first.tick(&coeffs, f64::from(*sample))) as f32)
        .collect()
}

#[test]
fn decay_time_matches_the_setting() {
    for decay_s in [0.4_f32, 1.8, 5.0] {
        for size in [0.0_f32, 0.5, 1.0] {
            let (left, right) = impulse_response(&bare(decay_s, size), RATE, decay_s * 1.6 + 0.5);
            for (side, response) in [("left", &left), ("right", &right)] {
                let measured = rt60(response, RATE);
                assert!(
                    (measured / f64::from(decay_s) - 1.0).abs() < 0.1,
                    "decay {decay_s} s at size {size}, {side}: measured {measured} s"
                );
            }
        }
    }
}

#[test]
fn decay_time_is_the_same_at_every_sample_rate() {
    for rate in RATES {
        let (left, _) = impulse_response(&bare(1.2, 0.6), rate, 2.5);
        let measured = rt60(&left, rate);
        assert!((measured / 1.2 - 1.0).abs() < 0.1, "{measured} s at {rate}");
    }
}

#[test]
fn damping_shortens_the_treble_and_leaves_the_rest() {
    let damped = ReverbParams {
        damping: 1.0,
        ..bare(2.0, 0.5)
    };
    let (left, _) = impulse_response(&damped, RATE, 4.0);
    let low = rt60(&band_limited(&left, 400.0, RATE), RATE);
    let high = rt60(&band_limited(&left, 12_000.0, RATE), RATE);
    assert!((low / 2.0 - 1.0).abs() < 0.2, "low band decays in {low} s");
    // At full damping the top of the spectrum lasts a tenth as long. A
    // band this wide still hears some of what is below it.
    assert!(high < 0.45, "high band decays in {high} s");

    let (open, _) = impulse_response(&bare(2.0, 0.5), RATE, 4.0);
    let high = rt60(&band_limited(&open, 12_000.0, RATE), RATE);
    assert!(
        (high / 2.0 - 1.0).abs() < 0.25,
        "undamped high band: {high} s"
    );

    // Halfway, the treble lasts about half as long.
    let half = ReverbParams {
        damping: 0.5,
        ..bare(2.0, 0.5)
    };
    let (left, _) = impulse_response(&half, RATE, 4.0);
    let high = rt60(&band_limited(&left, 12_000.0, RATE), RATE);
    assert!(
        (0.8..=1.5).contains(&high),
        "half-damped high band: {high} s"
    );
}

/// Share of samples in a window that stand out from the window's own
/// spread, relative to what random noise would give. Sparse echoes read
/// near 0, a fully dense tail reads 1 (Abel and Huang, "A simple, robust
/// measure of reverberation echo density", AES 2006).
fn echo_density(window: &[f32]) -> f64 {
    let spread = rms(window);
    let outside = window
        .iter()
        .filter(|sample| f64::from(sample.abs()) > spread)
        .count();
    // erfc(1 / sqrt(2)): the share of a normal distribution beyond one
    // standard deviation.
    outside as f64 / window.len() as f64 / 0.3173
}

#[test]
fn the_tail_becomes_dense_quickly() {
    for size in [0.2_f32, 0.5, 1.0] {
        let params = ReverbParams {
            diffusion: 0.8,
            ..bare(2.0, size)
        };
        let (left, right) = impulse_response(&params, RATE, 1.0);
        for response in [&left, &right] {
            let at = |ms: usize| echo_density(&response[ms * 48..(ms + 25) * 48]);
            assert!(at(250) > 0.8, "size {size}: density {} at 250 ms", at(250));
            assert!(at(600) > 0.85, "size {size}: density {} at 600 ms", at(600));
        }
        // Density builds up: it is never higher at the very start.
        let start = echo_density(&left[..25 * 48]);
        assert!(start < echo_density(&left[400 * 48..425 * 48]) + 0.05);
    }
}

#[test]
fn diffusion_smears_the_first_echoes() {
    let dense = ReverbParams {
        diffusion: 1.0,
        ..bare(2.0, 1.0)
    };
    let sparse = ReverbParams {
        diffusion: 0.0,
        ..bare(2.0, 1.0)
    };
    let early = |params: &ReverbParams| {
        let (left, _) = impulse_response(params, RATE, 0.3);
        echo_density(&left[60 * 48..160 * 48])
    };
    assert!(
        early(&dense) > early(&sparse) + 0.2,
        "{} against {}",
        early(&dense),
        early(&sparse)
    );
}

/// The late tail with its decay undone, so that it can be examined as a
/// steady signal.
fn levelled_tail(
    response: &[f32],
    decay_s: f32,
    rate: f32,
    from_s: f32,
    frames: usize,
) -> Vec<f32> {
    let start = (from_s * rate) as usize;
    response[start..start + frames]
        .iter()
        .enumerate()
        .map(|(n, sample)| {
            let seconds = n as f64 / f64::from(rate);
            (f64::from(*sample) * 10.0_f64.powf(3.0 * seconds / f64::from(decay_s))) as f32
        })
        .collect()
}

/// The power spectrum of `signal` averaged over half-overlapping windows
/// of `size` samples (Welch's method). Bin `k` is at `k * rate / size` Hz.
fn averaged_spectrum(signal: &[f32], size: usize) -> Vec<f64> {
    let mut total = vec![0.0; size / 2];
    let mut windows = 0;
    for start in (0..=signal.len() - size).step_by(size / 2) {
        let spectrum = windowed_magnitudes(&signal[start..start + size]);
        for (sum, magnitude) in total.iter_mut().zip(spectrum) {
            *sum += magnitude * magnitude;
        }
        windows += 1;
    }
    total.iter().map(|sum| sum / f64::from(windows)).collect()
}

#[test]
fn the_tail_does_not_ring() {
    // A metallic reverb concentrates its energy in a few frequencies that
    // stand out from their neighbours for the whole length of the tail.
    // Undo the decay, average the spectrum of the tail over time at a
    // resolution of 12 Hz, and compare each frequency with the average of
    // those around it.
    //
    // Random noise measured this way peaks at about 2 times its local
    // average and scatters by about 1.1 dB. A real room scatters by 3 dB
    // or so. A ringing resonance would stand 10 or more times above its
    // surroundings.
    for (size, modulation) in [
        (0.5_f32, 0.25_f32),
        (0.0, 0.25),
        (0.0, 0.0),
        (1.0, 0.0),
        (0.3, 1.0),
    ] {
        let decay_s = 2.0;
        let params = ReverbParams {
            modulation,
            ..bare(decay_s, size)
        };
        let (left, right) = impulse_response(&params, RATE, 2.0);
        for response in [&left, &right] {
            let size_fft = 4_096;
            let tail = levelled_tail(response, decay_s, RATE, 0.3, 65_536);
            let power = averaged_spectrum(&tail, size_fft);
            let reach = 16;
            // 200 Hz to 8 kHz.
            let (mut highest, mut scatter, mut bins) = (0.0_f64, 0.0_f64, 0.0_f64);
            for bin in 17..683 {
                let around = &power[bin - reach..=bin + reach];
                let local = around.iter().sum::<f64>() / around.len() as f64;
                let ratio = power[bin] / local;
                highest = highest.max(ratio);
                scatter += (10.0 * ratio.log10()).powi(2);
                bins += 1.0;
            }
            let scatter = (scatter / bins).sqrt();
            assert!(
                highest < 6.0,
                "size {size}, modulation {modulation}: a peak {highest} times its surroundings"
            );
            assert!(
                scatter < 3.0,
                "size {size}, modulation {modulation}: the spectrum scatters by {scatter} dB"
            );
            let power = averaged_spectrum(&tail, 65_536);
            let size_fft = 65_536;

            // Third-octave bands should all hold similar energy: no wide
            // holes or humps either.
            let mut levels = Vec::new();
            let mut low = 250.0_f64;
            while low < 8_000.0 {
                let high = low * 2.0_f64.powf(1.0 / 3.0);
                let (from, to) = (
                    (low * size_fft as f64 / 48_000.0) as usize,
                    (high * size_fft as f64 / 48_000.0) as usize,
                );
                let energy = power[from..to].iter().sum::<f64>() / (to - from) as f64;
                levels.push(10.0 * energy.log10());
                low = high;
            }
            let highest = levels.iter().cloned().fold(f64::MIN, f64::max);
            let lowest = levels.iter().cloned().fold(f64::MAX, f64::min);
            assert!(
                highest - lowest < 6.0,
                "size {size}: third-octave bands span {} dB",
                highest - lowest
            );

            // And sample by sample the tail is distributed like noise.
            let spread = rms(&tail);
            let fourth: f64 =
                tail.iter().map(|s| f64::from(*s).powi(4)).sum::<f64>() / tail.len() as f64;
            let kurtosis = fourth / spread.powi(4);
            assert!(
                (2.5..=3.8).contains(&kurtosis),
                "size {size}: kurtosis {kurtosis}"
            );
        }
    }
}

#[test]
fn left_and_right_tails_are_different_and_width_folds_them_together() {
    let (left, right) = impulse_response(&bare(2.0, 0.5), RATE, 1.5);
    let (left, right) = (&left[9_600..], &right[9_600..]);
    let correlation: f64 = left
        .iter()
        .zip(right)
        .map(|(l, r)| f64::from(*l) * f64::from(*r))
        .sum::<f64>()
        / (rms(left) * rms(right) * left.len() as f64);
    assert!(correlation.abs() < 0.25, "correlation {correlation}");
    // Both sides are equally loud.
    assert!(db(rms(left) / rms(right)).abs() < 1.5);

    let mono = ReverbParams {
        width: 0.0,
        ..bare(2.0, 0.5)
    };
    let (left, right) = impulse_response(&mono, RATE, 1.0);
    assert!(left == right);
    assert!(rms(&left) > 1.0e-4);
}

#[test]
fn one_sided_input_still_fills_both_sides() {
    let mut reverb: Reverb = prepared(&bare(2.0, 0.5), RATE);
    let mut left = impulse(48_000, 0, 1.0);
    let mut right = silence(48_000);
    run(&mut reverb, &mut left, &mut right, 256);
    let balance = db(rms(&right[24_000..]) / rms(&left[24_000..]));
    assert!(balance.abs() < 3.0, "late balance {balance} dB");
    // At the very start the sound is on the side it came from.
    assert!(rms(&left[..2_400]) > rms(&right[..2_400]) * 1.5);
}

#[test]
fn no_offset_builds_up() {
    for params in [ReverbParams::default(), bare(10.0, 1.0)] {
        let wet = ReverbParams { mix: 1.0, ..params };
        let mut reverb: Reverb = prepared(&wet, RATE);
        let mut left = vec![1.0_f32; 192_000];
        let mut right = vec![1.0_f32; 192_000];
        run(&mut reverb, &mut left, &mut right, 256);
        assert!(
            mean(&left[168_000..]).abs() < 2.0e-3,
            "{}",
            mean(&left[168_000..])
        );
        assert!(mean(&right[168_000..]).abs() < 2.0e-3);
    }
    // With a short decay, what the constant's sudden start set ringing has
    // gone too, and nothing at all is left.
    let wet = ReverbParams {
        mix: 1.0,
        ..ReverbParams::default()
    };
    let mut reverb: Reverb = prepared(&wet, RATE);
    let mut left = vec![1.0_f32; 192_000];
    let mut right = vec![1.0_f32; 192_000];
    run(&mut reverb, &mut left, &mut right, 256);
    assert!(
        peak(&left[168_000..]) < 1.0e-3,
        "{}",
        peak(&left[168_000..])
    );
}

#[test]
fn every_corner_of_the_settings_is_finite_and_dies_away() {
    for corner in 0..64_u32 {
        let pick = |bit: u32, low: f32, high: f32| if corner >> bit & 1 == 0 { low } else { high };
        let params = ReverbParams {
            size: pick(0, 0.0, 1.0),
            decay_s: pick(1, 0.1, 20.0),
            pre_delay_ms: pick(2, 0.0, 250.0),
            damping: pick(3, 0.0, 1.0),
            diffusion: pick(4, 0.0, 1.0),
            modulation: pick(5, 0.0, 1.0),
            early_level: pick(0, 1.0, 0.0),
            low_cut_hz: pick(3, 20.0, 1_000.0),
            high_cut_hz: pick(4, 20_000.0, 1_000.0),
            width: pick(5, 1.0, 0.0),
            mix: 1.0,
        };
        let rate = RATES[corner as usize % RATES.len()];
        let mut reverb: Reverb = prepared(&params, rate);
        let burst = (rate * 0.25) as usize;
        let frames = (rate * 2.25) as usize;
        let mut left = noise(corner, 1.0, burst);
        left.resize(frames, 0.0);
        let mut right = left.clone();
        run(&mut reverb, &mut left, &mut right, 256);
        assert_finite(&left, &format!("{params:?}"));
        assert_finite(&right, &format!("{params:?}"));
        let quarter = (rate * 0.5) as usize;
        let earlier = rms(&left[burst + quarter..burst + 2 * quarter]);
        let later = rms(&left[burst + 3 * quarter..burst + 4 * quarter]);
        assert!(
            later < earlier * 0.95 || later < 1.0e-9,
            "{params:?}: {earlier} then {later}"
        );
        assert!(peak(&left) < 20.0, "{params:?}: peak {}", peak(&left));
    }
}

#[test]
fn pre_delay_holds_the_reverb_back() {
    for rate in RATES {
        for pre_delay_ms in [0.0_f32, 40.0, 250.0] {
            let params = ReverbParams {
                pre_delay_ms,
                early_level: 1.0,
                mix: 1.0,
                ..ReverbParams::default()
            };
            let (left, _) = impulse_response(&params, rate, 0.6);
            let first = left
                .iter()
                .position(|sample| sample.abs() > 1.0e-5)
                .unwrap();
            let first_ms = first as f32 * 1_000.0 / rate;
            // The first early reflection comes 7.1 ms after the pre-delay
            // at full size, and the default size is 0.875 of that.
            let expected = pre_delay_ms + 7.1 * 0.875;
            assert!(
                (first_ms - expected).abs() < 0.6,
                "pre-delay {pre_delay_ms} ms at {rate}: first sound at {first_ms} ms"
            );
        }
    }
}

#[test]
fn early_reflections_follow_their_level_and_the_size() {
    let with = |early_level: f32, size: f32| {
        let params = ReverbParams {
            early_level,
            size,
            pre_delay_ms: 0.0,
            mix: 1.0,
            ..ReverbParams::default()
        };
        impulse_response(&params, RATE, 0.2).0
    };
    // The window before the tail's first echo holds only early
    // reflections.
    let loud = with(1.0, 1.0);
    let half = with(0.5, 1.0);
    let none = with(0.0, 1.0);
    assert!(peak(&none[..1_200]) < 1.0e-6);
    let ratio = db(rms(&half[..1_200]) / rms(&loud[..1_200]));
    assert!((ratio + 6.02).abs() < 0.1, "{ratio}");
    // A smaller room brings them closer together.
    let first = |response: &[f32]| response.iter().position(|s| s.abs() > 1.0e-4).unwrap();
    let (large, small) = (first(&with(1.0, 1.0)), first(&with(1.0, 0.0)));
    assert!(
        (large as f32 / small as f32 - 6.0).abs() < 0.3,
        "{large} against {small}"
    );
}

#[test]
fn mix_runs_from_all_dry_to_all_wet() {
    let input = noise(11, 0.5, 12_000);
    let dry = ReverbParams {
        mix: 0.0,
        ..ReverbParams::default()
    };
    let mut reverb: Reverb = prepared(&dry, RATE);
    let (mut left, mut right) = (input.clone(), input.clone());
    run(&mut reverb, &mut left, &mut right, 256);
    assert!(left == input && right == input);

    // Fully wet with a pre-delay, the input itself is gone.
    let wet = ReverbParams {
        mix: 1.0,
        pre_delay_ms: 20.0,
        ..ReverbParams::default()
    };
    let (left, _) = impulse_response(&wet, RATE, 0.1);
    assert_eq!(peak(&left[..900]), 0.0);
}

#[test]
fn a_fully_wet_default_reverb_is_about_as_loud_as_its_input() {
    // A long tail holds more energy than a short one, but the reverb takes
    // part of that back, so the level stays within a few dB.
    for (decay_s, range) in [(1.8, -3.0..=1.0), (0.3, -4.0..=2.0), (10.0, -2.0..=4.0)] {
        let params = ReverbParams {
            decay_s,
            mix: 1.0,
            ..ReverbParams::default()
        };
        let mut reverb: Reverb = prepared(&params, RATE);
        let frames = 48_000 * 14;
        let mut left = noise(21, 0.25, frames);
        let mut right = noise(22, 0.25, frames);
        let input = rms(&left);
        run(&mut reverb, &mut left, &mut right, 256);
        let level = db(rms(&left[frames - 96_000..]) / input);

        assert!(range.contains(&level), "decay {decay_s} s: {level} dB");
    }
}

#[test]
fn low_cut_and_high_cut_shape_what_enters_the_tail() {
    let shaped = ReverbParams {
        low_cut_hz: 500.0,
        high_cut_hz: 2_000.0,
        ..bare(1.5, 0.5)
    };
    let open = bare(1.5, 0.5);
    let level = |params: &ReverbParams, frequency: f32| {
        let (left, _) = impulse_response(params, RATE, 1.0);
        rms(&band_limited(&left, frequency, RATE))
    };
    assert!(db(level(&shaped, 100.0) / level(&open, 100.0)) < -20.0);
    assert!(db(level(&shaped, 10_000.0) / level(&open, 10_000.0)) < -20.0);
    assert!(db(level(&shaped, 1_000.0) / level(&open, 1_000.0)).abs() < 3.0);
}
