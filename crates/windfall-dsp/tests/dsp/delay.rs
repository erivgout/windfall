//! The delay: echoes land on the right sample, at the right level, on the
//! right side.

use windfall_dsp::{Delay, DelayMode, DelayParams, Effect, NoteDivision};

use crate::support::{
    RATE, RATES, db, impulse, noise, peak, prepared, rms, run, silence, sine, steepest, tone_level,
    windowed_magnitudes,
};

/// Echoes with nothing in the way: no sync, filters wide open, fully wet.
fn clean(time_ms: f32, feedback: f32) -> DelayParams {
    DelayParams {
        sync: false,
        time_ms,
        feedback,
        low_cut_hz: 20.0,
        high_cut_hz: 20_000.0,
        saturation: 0.0,
        mix: 1.0,
        ..DelayParams::default()
    }
}

fn impulse_response(delay: &mut Delay, frames: usize) -> (Vec<f32>, Vec<f32>) {
    let mut left = impulse(frames, 0, 1.0);
    let mut right = impulse(frames, 0, 1.0);
    run(delay, &mut left, &mut right, 256);
    (left, right)
}

/// A short 1 kHz tone that fades in and out. Unlike a single spike it has
/// no energy at the extremes of the spectrum, so the feedback filters,
/// even wide open, leave its level alone.
fn burst(frames: usize, length: usize, level: f32, rate: f32) -> Vec<f32> {
    let mut signal = sine(1_000.0, level, length, rate);
    for (n, sample) in signal.iter_mut().enumerate() {
        *sample *= 0.5 - 0.5 * (std::f32::consts::TAU * n as f32 / length as f32).cos();
    }
    signal.resize(frames, 0.0);
    signal
}

/// The position of the loudest sample within `reach` of `around`.
fn loudest_near(response: &[f32], around: usize, reach: usize) -> usize {
    let from = around.saturating_sub(reach);
    let to = (around + reach + 1).min(response.len());
    (from..to)
        .max_by(|a, b| response[*a].abs().total_cmp(&response[*b].abs()))
        .unwrap()
}

#[test]
fn echoes_are_spaced_by_exactly_the_delay_time_at_every_sample_rate() {
    for rate in RATES {
        for time_ms in [1.0_f32, 10.0, 125.0, 333.3, 2_000.0] {
            let mut delay: Delay = prepared(&clean(time_ms, 0.5), rate);
            let spacing = (time_ms * rate / 1_000.0).round() as usize;
            let (left, right) = impulse_response(&mut delay, spacing * 4 + 10);
            assert!(left == right);
            // The first echo is an exact copy of the input, and nothing
            // comes out before it.
            assert_eq!(left.iter().position(|sample| *sample != 0.0), Some(spacing));
            assert_eq!(left[spacing], 1.0);
            assert_eq!(left[..2 * spacing].iter().filter(|s| **s != 0.0).count(), 1);
            // Later echoes have each been through the feedback filters
            // once more. Wide open, the high cut still holds a signal back
            // by about half a sample per pass, so an echo may sit a sample
            // late, never early.
            for echo in 2..=3 {
                let reach = (spacing / 2).clamp(1, 20);
                let found = loudest_near(&left, echo * spacing, reach);
                assert!(
                    found >= echo * spacing && found < echo * spacing + echo,
                    "echo {echo} of {time_ms} ms at {rate} is at {found}"
                );
                assert!(left[found].abs() > 0.05);
            }
        }
    }
}

#[test]
fn each_echo_is_quieter_by_the_feedback() {
    for feedback in [0.25_f32, 0.5, 0.8, 0.95] {
        let mut delay: Delay = prepared(&clean(20.0, feedback), RATE);
        let mut left = burst(960 * 12, 480, 0.5, RATE);
        let mut right = left.clone();
        let input_level = rms(&left[..960]);
        run(&mut delay, &mut left, &mut right, 256);
        let level = |echo: usize| rms(&left[echo * 960..(echo + 1) * 960]);
        // The first echo is the input, untouched.
        assert!((level(1) / input_level - 1.0).abs() < 1.0e-6);
        for echo in 1..10 {
            let ratio = level(echo + 1) / level(echo);
            assert!(
                (ratio / f64::from(feedback) - 1.0).abs() < 0.01,
                "feedback {feedback}, echo {echo}: ratio {ratio}"
            );
        }
    }
    // No feedback, one echo.
    let mut delay: Delay = prepared(&clean(20.0, 0.0), RATE);
    let (left, _) = impulse_response(&mut delay, 960 * 4);
    assert_eq!(left.iter().filter(|sample| **sample != 0.0).count(), 1);
}

#[test]
fn synced_time_follows_the_tempo() {
    let mut params = clean(100.0, 0.3);
    params.sync = true;
    for (division, bpm, expected) in [
        (NoteDivision::Quarter, 120.0, 24_000),
        (NoteDivision::Eighth, 120.0, 12_000),
        (NoteDivision::EighthDotted, 120.0, 18_000),
        (NoteDivision::QuarterTriplet, 90.0, 21_333),
        (NoteDivision::Sixteenth, 174.0, 4_138),
        (NoteDivision::Whole, 240.0, 48_000),
        (NoteDivision::ThirtySecond, 60.0, 6_000),
    ] {
        params.division = division;
        let mut delay: Delay = prepared(&params, RATE);
        delay.set_tempo(bpm);
        let (left, _) = impulse_response(&mut delay, expected + 100);
        assert_eq!(
            left.iter().position(|sample| *sample != 0.0),
            Some(expected),
            "{division:?} at {bpm} bpm"
        );
    }

    // Without a tempo from the host it assumes 120.
    params.division = NoteDivision::Eighth;
    let mut delay: Delay = prepared(&params, RATE);
    let (left, _) = impulse_response(&mut delay, 12_100);
    assert_eq!(left.iter().position(|sample| *sample != 0.0), Some(12_000));

    // A tempo that is not a number is ignored, and absurd ones are held
    // to something playable.
    delay.reset();
    delay.set_tempo(f32::NAN);
    delay.set_tempo(0.0);
    let (left, _) = impulse_response(&mut delay, 200_000);
    assert!(left.iter().all(|sample| sample.is_finite()));
}

#[test]
fn a_tempo_change_moves_the_echoes_without_a_click() {
    let mut params = clean(100.0, 0.4);
    params.sync = true;
    params.division = NoteDivision::Sixteenth;
    params.mix = 0.5;
    let mut delay: Delay = prepared(&params, RATE);
    delay.set_tempo(120.0);
    let mut left = sine(200.0, 0.5, 96_000, RATE);
    let mut right = left.clone();
    for (index, (left, right)) in left.chunks_mut(128).zip(right.chunks_mut(128)).enumerate() {
        if index % 60 == 30 {
            delay.set_tempo(if index % 120 == 30 { 137.0 } else { 93.0 });
        }
        delay.process(left, right);
    }
    let natural = std::f32::consts::TAU * 200.0 / RATE * peak(&left);
    assert!(steepest(&left[9_600..]) < 2.0 * natural + 0.002);
}

#[test]
fn ping_pong_bounces_from_left_to_right() {
    let mut params = clean(10.0, 0.6);
    params.mode = DelayMode::PingPong;
    let mut delay: Delay = prepared(&params, RATE);
    // Only the left channel has input. Ping-pong sums the input to mono,
    // so a one-sided source is half as loud going in.
    let mut left = burst(480 * 8, 240, 1.0, RATE);
    let mut right = silence(480 * 8);
    let input_level = rms(&left[..480]);
    run(&mut delay, &mut left, &mut right, 256);
    let level = |side: &[f32], echo: usize| rms(&side[echo * 480..(echo + 1) * 480]);
    // Odd echoes are on the left, even ones on the right, and each hop
    // loses the feedback.
    let mut expected = 0.5 * input_level;
    for echo in 1..=6 {
        let (here, there) = if echo % 2 == 1 {
            (&left, &right)
        } else {
            (&right, &left)
        };
        assert!(
            (level(here, echo) / expected - 1.0).abs() < 0.01,
            "echo {echo} is at {}, expected {expected}",
            level(here, echo)
        );
        // The other side holds only the dying ring of the echo before.
        assert!(
            level(there, echo) < expected * 0.01,
            "echo {echo} leaks across"
        );
        expected *= 0.6;
    }

    // Input on the right alone starts on the left too.
    delay.reset();
    let mut left = silence(2_000);
    let mut right = impulse(2_000, 0, 1.0);
    run(&mut delay, &mut left, &mut right, 256);
    assert_eq!(left[480], 0.5);
    assert_eq!(right[480], 0.0);
}

#[test]
fn stereo_mode_keeps_each_side_to_itself() {
    let mut delay: Delay = prepared(&clean(10.0, 0.6), RATE);
    let mut left = impulse(4_000, 0, 1.0);
    let mut right = silence(4_000);
    run(&mut delay, &mut left, &mut right, 256);
    assert_eq!(peak(&right), 0.0);
    for echo in 1..=4 {
        let found = loudest_near(&left, echo * 480, 20);
        assert!(
            found >= echo * 480 && found < echo * 480 + echo,
            "echo {echo} at {found}"
        );
    }
}

#[test]
fn stereo_offset_delays_one_side() {
    for rate in RATES {
        for (offset_ms, late_right) in [(12.0_f32, true), (-30.0, false)] {
            let mut params = clean(50.0, 0.0);
            params.stereo_offset_ms = offset_ms;
            let mut delay: Delay = prepared(&params, rate);
            let (left, right) = impulse_response(&mut delay, (rate * 0.2) as usize);
            let first = |side: &[f32]| side.iter().position(|sample| *sample != 0.0).unwrap();
            let base = (50.0 * rate / 1_000.0).round() as usize;
            let late = ((50.0 + offset_ms.abs()) * rate / 1_000.0).round() as usize;
            let expected = if late_right {
                (base, late)
            } else {
                (late, base)
            };
            assert_eq!(
                (first(&left), first(&right)),
                expected,
                "{offset_ms} ms at {rate}"
            );
        }
    }
}

#[test]
fn feedback_filters_thin_and_darken_each_repeat() {
    let mut params = clean(50.0, 0.9);
    params.low_cut_hz = 400.0;
    params.high_cut_hz = 3_000.0;
    let mut delay: Delay = prepared(&params, RATE);
    let mut left = noise(5, 0.5, 2_000);
    left.resize(2_400 * 8, 0.0);
    let mut right = left.clone();
    run(&mut delay, &mut left, &mut right, 256);
    // The band of each echo that lies inside or outside the filters.
    let band = |echo: usize, from_hz: f64, to_hz: f64| {
        let spectrum = windowed_magnitudes(&left[echo * 2_400..echo * 2_400 + 2_048]);
        let bin = |hz: f64| (hz * 2_048.0 / 48_000.0) as usize;
        let power: f64 = spectrum[bin(from_hz)..bin(to_hz)]
            .iter()
            .map(|m| m * m)
            .sum();
        power.sqrt()
    };
    // The first echo is the input itself, unfiltered.
    let (low, mid, high) = (
        |echo| band(echo, 60.0, 200.0),
        |echo| band(echo, 900.0, 1_400.0),
        |echo| band(echo, 8_000.0, 16_000.0),
    );
    for echo in 1..5 {
        let mid_loss = db(mid(echo + 1) / mid(echo));
        let low_loss = db(low(echo + 1) / low(echo));
        let high_loss = db(high(echo + 1) / high(echo));
        assert!(
            (-2.5..=-0.5).contains(&mid_loss),
            "echo {echo}: mid {mid_loss}"
        );
        assert!(low_loss < -9.0, "echo {echo}: low {low_loss}");
        assert!(high_loss < -12.0, "echo {echo}: high {high_loss}");
    }
}

#[test]
fn saturation_squashes_loud_repeats_and_leaves_quiet_ones() {
    let level_of_second_echo = |input_level: f32, saturation: f32| {
        let mut params = clean(20.0, 0.9);
        params.saturation = saturation;
        let mut delay: Delay = prepared(&params, RATE);
        let mut left = sine(500.0, input_level, 480, RATE);
        left.resize(960 * 4, 0.0);
        let mut right = left.clone();
        run(&mut delay, &mut left, &mut right, 256);
        tone_level(&left[1_920..2_400], 500.0, RATE) / f64::from(input_level)
    };
    // Clean, the second echo is the feedback times the first.
    assert!((level_of_second_echo(1.0, 0.0) - 0.9).abs() < 0.02);
    // Saturated, a loud echo comes back much weaker, a quiet one nearly
    // as before.
    assert!(level_of_second_echo(1.0, 1.0) < 0.45);
    assert!(level_of_second_echo(0.02, 1.0) > 0.85);
    // And it adds overtones a clean delay does not have.
    let harmonic = |saturation: f32| {
        let mut params = clean(20.0, 0.9);
        params.saturation = saturation;
        let mut delay: Delay = prepared(&params, RATE);
        let mut left = sine(500.0, 1.0, 480, RATE);
        left.resize(960 * 4, 0.0);
        let mut right = left.clone();
        run(&mut delay, &mut left, &mut right, 256);
        tone_level(&left[1_920..2_400], 1_500.0, RATE)
    };
    assert!(harmonic(1.0) > 20.0 * harmonic(0.0));
}

#[test]
fn full_feedback_and_full_scale_input_never_run_away() {
    for saturation in [0.0_f32, 1.0] {
        for mode in [DelayMode::Stereo, DelayMode::PingPong] {
            let params = DelayParams {
                feedback: 5.0,
                saturation,
                mode,
                ..clean(5.0, 0.0)
            };
            let mut delay: Delay = prepared(&params, RATE);
            let mut left = noise(3, 1.0, 96_000);
            let mut right = noise(4, 1.0, 96_000);
            run(&mut delay, &mut left, &mut right, 256);
            // The feedback is held at 0.95, so the loop can gain at most
            // 1 / (1 - 0.95) = 20.
            let loudest = peak(&left).max(peak(&right));
            assert!(
                loudest < 20.0,
                "{mode:?}, saturation {saturation}: {loudest}"
            );
            // And it dies away when the input stops.
            let (mut left, mut right) = (silence(96_000), silence(96_000));
            run(&mut delay, &mut left, &mut right, 256);
            assert!(rms(&left[90_000..]) < 1.0e-3);
        }
    }
}

#[test]
fn changing_the_time_crossfades_without_a_click_or_a_pitch_bend() {
    let mut params = clean(40.0, 0.3);
    let mut delay: Delay = prepared(&params, RATE);
    let mut left = sine(300.0, 0.5, 96_000, RATE);
    let mut right = left.clone();
    for (index, (left, right)) in left.chunks_mut(64).zip(right.chunks_mut(64)).enumerate() {
        // New times arrive faster than the 30 ms crossfade can finish.
        if (200..400).contains(&index) && index % 5 == 0 {
            params.time_ms = 40.0 + (index % 37) as f32 * 3.1;
            delay.set_params(&params);
        }
        delay.process(left, right);
    }
    let natural = std::f32::consts::TAU * 300.0 / RATE * peak(&left);
    assert!(steepest(&left[4_800..]) < 2.0 * natural + 0.002);
    // Once the changes stop, what comes out is still a 300 Hz tone and
    // nothing else: no glide has left other pitches in the line.
    let settled = &left[96_000 - 16_384..];
    let tone = tone_level(settled, 300.0, RATE);
    assert!(tone > 0.3);
    for other in [200.0, 250.0, 350.0, 400.0, 600.0] {
        assert!(
            tone_level(settled, other, RATE) < tone * 1.0e-3,
            "{other} Hz"
        );
    }
}

#[test]
fn mix_runs_from_all_dry_to_all_wet() {
    let input = noise(8, 0.5, 6_000);
    let mut params = clean(10.0, 0.5);
    params.mix = 0.0;
    let mut delay: Delay = prepared(&params, RATE);
    let (mut left, mut right) = (input.clone(), input.clone());
    run(&mut delay, &mut left, &mut right, 256);
    assert!(left == input && right == input);

    params.mix = 1.0;
    let mut delay: Delay = prepared(&params, RATE);
    let (left, _) = impulse_response(&mut delay, 1_000);
    assert_eq!(left[0], 0.0);

    params.mix = 0.25;
    let mut delay: Delay = prepared(&params, RATE);
    let (left, _) = impulse_response(&mut delay, 1_000);
    assert!((left[0] - 0.75).abs() < 1.0e-6 && (left[480] - 0.25).abs() < 1.0e-6);
}

#[test]
fn the_tail_covers_the_echoes_down_to_sixty_decibels() {
    for (feedback, mode) in [
        (0.0_f32, DelayMode::Stereo),
        (0.5, DelayMode::Stereo),
        (0.9, DelayMode::PingPong),
    ] {
        let mut params = clean(15.0, feedback);
        params.mode = mode;
        let mut delay: Delay = prepared(&params, RATE);
        let tail = delay.tail_samples();
        let (left, right) = impulse_response(&mut delay, tail + 2_000);
        let after = peak(&left[tail..]).max(peak(&right[tail..]));
        assert!(
            after < 1.5e-3,
            "feedback {feedback}: {after} after {tail} samples"
        );
        // The tail is not wildly longer than it needs to be either.
        let needed = left
            .iter()
            .zip(&right)
            .rposition(|(l, r)| l.abs().max(r.abs()) > 1.0e-3)
            .unwrap();
        assert!(tail < needed * 2 + 4_000, "tail {tail}, needed {needed}");
    }
}
