//! The parametric equaliser, measured against filter theory.

use windfall_dsp::blocks::biquad::BiquadCoeffs;
use windfall_dsp::{CutSlope, Effect, EqBand, EqParams, ParametricEq};

use crate::support::{
    RATE, RATES, db, impulse, magnitudes, noise, peak, prepared, rms, run, run_mono,
};

const FFT_SIZE: usize = 1 << 17;

/// The equaliser's gain in dB at each of `frequencies`, measured from its
/// impulse response.
fn measured_response(params: &EqParams, rate: f32, frequencies: &[f32]) -> Vec<f64> {
    let mut eq: ParametricEq = prepared(params, rate);
    let response = run_mono(&mut eq, &impulse(FFT_SIZE, 0, 1.0));
    let spectrum = magnitudes(&response, FFT_SIZE);
    frequencies
        .iter()
        .map(|frequency| {
            // Frequencies are chosen on bin centres, so no interpolation.
            let bin = (frequency * FFT_SIZE as f32 / rate).round() as usize;
            db(spectrum[bin])
        })
        .collect()
}

/// Frequencies spread evenly on a log scale, each moved onto the centre of
/// an FFT bin.
fn probe_frequencies(rate: f32, low: f32, high: f32, count: usize) -> Vec<f32> {
    (0..count)
        .map(|step| {
            let frequency = low * (high / low).powf(step as f32 / (count - 1) as f32);
            let bin = (frequency * FFT_SIZE as f32 / rate).round().max(1.0);
            bin * rate / FFT_SIZE as f32
        })
        .collect()
}

fn predicted_response(params: &EqParams, rate: f32, frequencies: &[f32]) -> Vec<f32> {
    let mut gains = vec![0.0; frequencies.len()];
    params.magnitude_response_db(rate, frequencies, &mut gains);
    gains
}

fn flat() -> EqParams {
    EqParams::default()
}

fn band(frequency_hz: f32, gain_db: f32, q: f32) -> EqBand {
    EqBand {
        enabled: true,
        frequency_hz,
        gain_db,
        q,
    }
}

#[test]
fn measured_response_matches_the_predicted_curve() {
    let mut settings = Vec::new();
    let mut everything = flat();
    everything.low_cut.enabled = true;
    everything.low_cut.frequency_hz = 60.0;
    everything.low_cut.slope = CutSlope::Db24;
    everything.low_shelf = band(150.0, 5.0, 0.707);
    everything.peak1 = band(500.0, -8.0, 3.0);
    everything.peak2 = band(1_500.0, 6.0, 1.0);
    everything.peak3 = band(4_000.0, 12.0, 8.0);
    everything.high_shelf = band(9_000.0, -6.0, 0.9);
    everything.high_cut.enabled = true;
    everything.high_cut.frequency_hz = 15_000.0;
    everything.high_cut.slope = CutSlope::Db48;
    everything.high_cut.q = 1.5;
    everything.output_gain_db = -2.5;
    settings.push(everything);

    let mut narrow = flat();
    narrow.peak2 = band(1_000.0, 24.0, 18.0);
    settings.push(narrow);

    let mut shelves = flat();
    shelves.low_shelf = band(300.0, -24.0, 2.0);
    shelves.high_shelf = band(2_000.0, 24.0, 0.1);
    settings.push(shelves);

    let mut steep = flat();
    steep.low_cut.enabled = true;
    steep.low_cut.frequency_hz = 400.0;
    steep.low_cut.slope = CutSlope::Db48;
    steep.low_cut.q = 4.0;
    settings.push(steep);

    for params in &settings {
        for rate in RATES {
            let frequencies = probe_frequencies(rate, 25.0, 0.45 * rate, 60);
            let measured = measured_response(params, rate, &frequencies);
            let predicted = predicted_response(params, rate, &frequencies);
            for ((frequency, measured), predicted) in
                frequencies.iter().zip(&measured).zip(&predicted)
            {
                // Far down a steep cut the impulse response is below what
                // single precision output can carry.
                if *predicted > -90.0 {
                    assert!(
                        (measured - f64::from(*predicted)).abs() < 0.05,
                        "{frequency} Hz at {rate}: measured {measured}, predicted {predicted}"
                    );
                }
            }
        }
    }
}

#[test]
fn predicted_curve_is_the_cookbook_response() {
    // The prediction itself, checked against coefficients built here by
    // hand, so that prediction and audio cannot be wrong in the same way.
    let mut params = flat();
    params.peak1 = band(700.0, 7.0, 2.5);
    params.low_shelf = band(120.0, -4.0, 0.707);
    params.high_cut.enabled = true;
    params.high_cut.frequency_hz = 5_000.0;
    params.output_gain_db = 1.5;
    let frequencies = [40.0, 120.0, 700.0, 1_000.0, 5_000.0, 12_000.0];
    let predicted = predicted_response(&params, RATE, &frequencies);
    let sections = [
        BiquadCoeffs::peak(700.0, 2.5, 7.0, RATE),
        BiquadCoeffs::low_shelf(120.0, 0.707, -4.0, RATE),
        BiquadCoeffs::low_pass(5_000.0, std::f32::consts::FRAC_1_SQRT_2, RATE),
    ];
    for (frequency, predicted) in frequencies.iter().zip(predicted) {
        let expected: f64 = 1.5
            + sections
                .iter()
                .map(|section| db(section.magnitude(*frequency, RATE)))
                .sum::<f64>();
        assert!(
            (f64::from(predicted) - expected).abs() < 1.0e-3,
            "{frequency} Hz"
        );
    }
}

#[test]
fn bands_hit_their_textbook_landmarks() {
    // A bell reaches its gain at its centre and leaves the far ends alone.
    let mut bell = flat();
    bell.peak2 = band(1_000.0, 9.0, 2.0);
    let frequencies = probe_frequencies(RATE, 30.0, 16_000.0, 3);
    let centre = probe_frequencies(RATE, 1_000.0, 1_000.1, 2)[0];
    let response = measured_response(&bell, RATE, &[frequencies[0], centre, frequencies[2]]);
    assert!(response[0].abs() < 0.05, "{response:?}");
    assert!((response[1] - 9.0).abs() < 0.05, "{response:?}");
    assert!(response[2].abs() < 0.1, "{response:?}");

    // A bell's bandwidth is its centre over its Q: with a Q of 2, half the
    // gain in dB is reached about a quarter octave either side.
    let half = predicted_response(&bell, RATE, &[780.0, 1_280.0]);
    assert!(
        (half[0] - 4.5).abs() < 0.3 && (half[1] - 4.5).abs() < 0.3,
        "{half:?}"
    );

    // Shelves reach their gain at the far end and half of it at the corner.
    let mut shelves = flat();
    shelves.low_shelf = band(200.0, 8.0, 0.707);
    shelves.high_shelf = band(6_000.0, -10.0, 0.707);
    let response = predicted_response(&shelves, RATE, &[10.0, 200.0, 1_100.0, 6_000.0, 23_000.0]);
    assert!((response[0] - 8.0).abs() < 0.05, "{response:?}");
    assert!((response[1] - 4.0).abs() < 0.1, "{response:?}");
    assert!(response[2].abs() < 0.5, "{response:?}");
    assert!((response[3] + 5.0).abs() < 0.1, "{response:?}");
    assert!((response[4] + 10.0).abs() < 0.05, "{response:?}");
}

#[test]
fn cut_filters_are_three_decibels_down_at_the_corner_and_fall_at_their_slope() {
    for (slope, per_octave) in [
        (CutSlope::Db12, 12.0),
        (CutSlope::Db24, 24.0),
        (CutSlope::Db48, 48.0),
    ] {
        let mut low_cut = flat();
        low_cut.low_cut.enabled = true;
        low_cut.low_cut.frequency_hz = 1_000.0;
        low_cut.low_cut.slope = slope;
        let frequencies = [125.0, 250.0, 1_000.0, 8_000.0];
        let response = predicted_response(&low_cut, RATE, &frequencies);
        assert!(
            (response[2] + 3.01).abs() < 0.02,
            "{slope:?} corner: {response:?}"
        );
        assert!(response[3].abs() < 0.05, "{slope:?} passband: {response:?}");
        let fall = response[1] - response[0];
        assert!(
            (fall - per_octave).abs() < 0.3,
            "{slope:?} falls {fall} dB per octave"
        );

        // The same holds for the audio itself, not just the prediction.
        let probes = probe_frequencies(RATE, 250.0, 1_000.0, 3);
        let measured = measured_response(&low_cut, RATE, &probes);
        let expected = predicted_response(&low_cut, RATE, &probes);
        for (measured, expected) in measured.iter().zip(expected) {
            assert!((measured - f64::from(expected)).abs() < 0.05);
        }

        let mut high_cut = flat();
        high_cut.high_cut.enabled = true;
        high_cut.high_cut.frequency_hz = 500.0;
        high_cut.high_cut.slope = slope;
        let response = predicted_response(&high_cut, RATE, &[20.0, 500.0, 2_000.0, 4_000.0]);
        assert!(response[0].abs() < 0.05);
        assert!(
            (response[1] + 3.01).abs() < 0.02,
            "{slope:?} corner: {response:?}"
        );
        let fall = response[2] - response[3];
        // Close to half the sample rate these filters fall a little faster
        // than their nominal slope.
        assert!(
            fall > per_octave - 0.3 && fall < per_octave * 1.1,
            "{slope:?} falls {fall} dB per octave"
        );
    }
}

#[test]
fn cut_resonance_adds_a_peak_at_the_corner() {
    let mut params = flat();
    params.low_cut.enabled = true;
    params.low_cut.frequency_hz = 200.0;
    params.low_cut.q = 4.0;
    for slope in [CutSlope::Db12, CutSlope::Db24, CutSlope::Db48] {
        params.low_cut.slope = slope;
        let response = predicted_response(&params, RATE, &[200.0, 2_000.0]);
        // A Q of 4 is 4 / 0.707 times the flat value: 15 dB more at the
        // corner than the flat filter's -3 dB.
        assert!((response[0] - 12.04).abs() < 0.1, "{slope:?}: {response:?}");
        assert!(response[1].abs() < 0.2);
    }
}

#[test]
fn frequencies_mean_the_same_at_every_sample_rate() {
    let mut params = flat();
    params.low_cut.enabled = true;
    params.low_cut.frequency_hz = 100.0;
    params.low_shelf = band(250.0, 6.0, 0.707);
    params.peak1 = band(800.0, -10.0, 4.0);
    params.peak2 = band(2_000.0, 8.0, 2.0);
    let frequencies = [50.0, 100.0, 250.0, 700.0, 800.0, 900.0, 2_000.0, 3_000.0];
    let reference = predicted_response(&params, 48_000.0, &frequencies);
    for rate in [44_100.0, 96_000.0, 192_000.0] {
        let response = predicted_response(&params, rate, &frequencies);
        for ((frequency, response), reference) in frequencies.iter().zip(response).zip(&reference) {
            assert!(
                (response - reference).abs() < 0.25,
                "{frequency} Hz at {rate}: {response} against {reference}"
            );
        }
        // And the audio agrees with the prediction at that rate.
        let probes = probe_frequencies(rate, 50.0, 3_000.0, 12);
        let measured = measured_response(&params, rate, &probes);
        let predicted = predicted_response(&params, rate, &probes);
        for (measured, predicted) in measured.iter().zip(predicted) {
            assert!((measured - f64::from(predicted)).abs() < 0.05);
        }
    }
}

#[test]
fn disabled_bands_and_flat_bands_do_nothing() {
    let mut params = flat();
    params.low_shelf = EqBand {
        enabled: false,
        ..band(100.0, 12.0, 0.707)
    };
    params.peak1 = EqBand {
        enabled: false,
        ..band(1_000.0, -20.0, 5.0)
    };
    params.low_cut.frequency_hz = 5_000.0;
    params.high_cut.frequency_hz = 50.0;
    params.peak2 = band(3_000.0, 0.0, 10.0);
    let frequencies = [30.0, 100.0, 1_000.0, 3_000.0, 15_000.0];
    assert_eq!(predicted_response(&params, RATE, &frequencies), [0.0; 5]);

    let mut eq: ParametricEq = prepared(&params, RATE);
    let input = noise(5, 0.5, 4_000);
    assert!(run_mono(&mut eq, &input) == input);
    assert_eq!(eq.tail_samples(), 0);
}

#[test]
fn output_gain_scales_the_signal() {
    let mut params = flat();
    params.output_gain_db = -6.0;
    let mut eq: ParametricEq = prepared(&params, RATE);
    let input = noise(5, 0.5, 4_000);
    let output = run_mono(&mut eq, &input);
    assert!((db(rms(&output) / rms(&input)) + 6.0).abs() < 1.0e-3);
    assert_eq!(predicted_response(&params, RATE, &[1_000.0]), [-6.0]);
}

#[test]
fn a_fast_extreme_sweep_stays_bounded() {
    // A narrow 24 dB peak thrown across the whole spectrum on every block,
    // with full-scale noise going through it.
    for rate in RATES {
        let mut eq: ParametricEq = prepared(&flat(), rate);
        let mut loudest = 0.0_f32;
        for step in 0..2_000_u32 {
            let mut params = flat();
            let position = (step * 7 % 10) as f32 / 9.0;
            params.peak1 = band(20.0 * 1_000.0_f32.powf(position), 24.0, 18.0);
            params.peak2 = band(20_000.0 / 1_000.0_f32.powf(position), -24.0, 18.0);
            params.low_cut.enabled = step % 3 == 0;
            params.low_cut.frequency_hz = 20.0 + 5_000.0 * position;
            params.low_cut.q = 8.0;
            params.low_cut.slope =
                [CutSlope::Db12, CutSlope::Db24, CutSlope::Db48][step as usize % 3];
            eq.set_params(&params);
            let mut left = noise(step, 1.0, 32);
            let mut right = left.clone();
            eq.process(&mut left, &mut right);
            loudest = loudest.max(peak(&left));
        }
        // The loudest any setting here gets in the steady state is 24 dB
        // from the bell plus 21 dB from the cut's resonance: about 180.
        assert!(loudest < 400.0, "{loudest} at {rate}");
    }
}

#[test]
fn the_tail_covers_the_ringing_of_a_narrow_band() {
    let mut params = flat();
    params.peak1 = band(100.0, 18.0, 18.0);
    let mut eq: ParametricEq = prepared(&params, RATE);
    let tail = eq.tail_samples();
    // A narrow, boosted band at 100 Hz rings for most of a second.
    assert!(tail > 12_000 && tail < 96_000, "{tail}");
    let mut left = impulse(tail + 4_096, 0, 1.0);
    let mut right = left.clone();
    run(&mut eq, &mut left, &mut right, 256);
    let early = peak(&left[100..2_000]);
    let late = peak(&left[tail..]);
    assert!(
        late < early * 2.0e-3,
        "{late} after the tail, {early} at the start"
    );
}
