//! Independent signal references, separate from registry/lifecycle acceptance.

use windfall_dsp::{
    AnyEffect, ChorusParams, EffectKind, EffectParams, EffectSlot, FlangerParams, PhaserParams,
};

const RATE: f32 = 48_000.0;
const KINDS: [EffectKind; 3] = [EffectKind::Chorus, EffectKind::Flanger, EffectKind::Phaser];

fn process(params: EffectParams, input: &[f32], block: usize) -> Vec<f32> {
    let mut fx = AnyEffect::new(&params);
    fx.prepare(RATE, 512);
    let mut left = input.to_vec();
    let mut right = input.to_vec();
    for (l, r) in left.chunks_mut(block).zip(right.chunks_mut(block)) {
        fx.process(l, r);
    }
    left
}

fn difference(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f32::max)
}

#[test]
fn chorus_static_impulse_is_three_distinct_normalized_taps_and_no_crossfeed() {
    // Frozen phases 0, 1/3, 2/3 give three independently calculable taps.
    let p = ChorusParams {
        delay_ms: 10.0,
        depth_ms: 8.0,
        rate1_hz: 0.0,
        rate2_hz: 0.0,
        rate3_hz: 0.0,
        mix: 1.0,
        ..Default::default()
    };
    let mut input = vec![0.0; 2500];
    input[0] = 1.0;
    let output = process(EffectParams::Chorus(p), &input, 137);
    let mut expected = vec![0.0_f32; input.len()];
    for phase in [0.0, 1.0 / 3.0, 2.0 / 3.0] {
        let d = (10.0 + 4.0 * (1.0 + (std::f64::consts::TAU * phase).sin())) * 48.0;
        let n = d.floor() as usize;
        let t = d.fract();
        // Lagrange-like Catmull-Rom impulse weights expanded in f64.
        let weights = [
            -0.5 * t + t * t - 0.5 * t * t * t,
            1.0 - 2.5 * t * t + 1.5 * t * t * t,
            0.5 * t + 2.0 * t * t - 1.5 * t * t * t,
            -0.5 * t * t + 0.5 * t * t * t,
        ];
        for (i, w) in weights.into_iter().enumerate() {
            expected[n + i - 1] += (w / 3.0) as f32;
        }
    }
    assert!(
        difference(&output, &expected) < 2e-5,
        "reference error {}",
        difference(&output, &expected)
    );
    assert!((output.iter().sum::<f32>() - 1.0).abs() < 1e-5);
    let mut fx = AnyEffect::new(&EffectParams::Chorus(p));
    fx.prepare(RATE, 512);
    let mut right = vec![0.0; input.len()];
    fx.process(&mut input, &mut right);
    assert!(right.iter().all(|x| *x == 0.0));
}

#[test]
fn chorus_moving_taps_detune_and_stereo_phase_changes_only_right() {
    let input: Vec<_> = (0..48_000)
        .map(|n| (std::f32::consts::TAU * 440.0 * n as f32 / RATE).sin() * 0.2)
        .collect();
    let p = ChorusParams {
        mix: 1.0,
        stereo_phase: 0.0,
        ..Default::default()
    };
    let mut fx = AnyEffect::new(&EffectParams::Chorus(p));
    fx.prepare(RATE, 512);
    let mut left = input.clone();
    let mut right = input.clone();
    fx.process(&mut left, &mut right);
    assert_eq!(left, right);
    let p = ChorusParams {
        stereo_phase: 0.25,
        ..p
    };
    let mut fx = AnyEffect::new(&EffectParams::Chorus(p));
    fx.prepare(RATE, 512);
    let mut left2 = input.clone();
    let mut right2 = input.clone();
    fx.process(&mut left2, &mut right2);
    assert_eq!(left, left2);
    assert!(difference(&left2[2000..], &right2[2000..]) > 0.1);
    let static_p = ChorusParams { depth_ms: 0.0, ..p };
    let frozen = process(EffectParams::Chorus(static_p), &input, 137);
    assert!(difference(&left2[2000..], &frozen[2000..]) > 0.1);
    // An independent continuously delayed sine reference, accurate at 440 Hz.
    for (n, &sample) in left2.iter().enumerate().skip(2000) {
        let expected: f64 = [0.6, 0.8, 1.1]
            .into_iter()
            .enumerate()
            .map(|(v, hz)| {
                let phase = v as f64 / 3.0 + n as f64 * hz / 48_000.0;
                let d = (12.0 + 2.0 * (1.0 + (std::f64::consts::TAU * phase).sin())) * 48.0;
                0.2 * (std::f64::consts::TAU * 440.0 * (n as f64 - d) / 48_000.0).sin() / 3.0
            })
            .sum();
        assert!((f64::from(sample) - expected).abs() < 0.0003);
    }
}

#[test]
fn chorus_partition_noop_reset_and_zero_mix_are_exact() {
    let input: Vec<_> = (0..5000).map(|n| (n as f32 * 0.13).sin() * 0.2).collect();
    let params = EffectKind::Chorus.default_params();
    assert_eq!(process(params, &input, 1), process(params, &input, 512));
    let mut fx = AnyEffect::new(&params);
    fx.prepare(RATE, 512);
    let mut a = input.clone();
    let mut b = input.clone();
    for (l, r) in a.chunks_mut(7).zip(b.chunks_mut(7)) {
        fx.set_params(&params);
        fx.process(l, r);
    }
    assert_eq!(a, process(params, &input, 137));
    fx.reset();
    fx.process(&mut [], &mut []);
    let mut a2 = input.clone();
    let mut b2 = input.clone();
    fx.process(&mut a2, &mut b2);
    assert_eq!(a, a2);
    let p = ChorusParams {
        mix: 0.0,
        ..Default::default()
    };
    assert_eq!(process(EffectParams::Chorus(p), &input, 137), input);
}

type Complex = (f64, f64);
fn add(a: Complex, b: Complex) -> Complex {
    (a.0 + b.0, a.1 + b.1)
}
fn mul(a: Complex, b: Complex) -> Complex {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}
fn scale(a: Complex, s: f64) -> Complex {
    (a.0 * s, a.1 * s)
}
fn div(a: Complex, b: Complex) -> Complex {
    let d = b.0 * b.0 + b.1 * b.1;
    ((a.0 * b.0 + a.1 * b.1) / d, (a.1 * b.0 - a.0 * b.1) / d)
}

/// Independently expanded transfer function at a fixed corner/delay. Product
/// allpass or delay builders are deliberately absent from the reference.
fn magnitude(params: EffectParams, hz: f64) -> f64 {
    let w = std::f64::consts::TAU * hz / 48_000.0;
    let z = (w.cos(), -w.sin());
    let (wet, mix, polarity) = match params {
        EffectParams::Flanger(p) => {
            let d = f64::from(p.delay_ms) * 48.0;
            assert_eq!(d, d.round());
            let delay = ((w * d).cos(), -(w * d).sin());
            let pole = 0.9 * f64::from(p.damping);
            let lowpass = div((1.0 - pole, 0.0), add((1.0, 0.0), scale(z, -pole)));
            let forward = mul(delay, lowpass);
            (
                div(
                    forward,
                    add((1.0, 0.0), scale(forward, -f64::from(p.feedback))),
                ),
                f64::from(p.mix),
                if p.invert_wet { -1.0 } else { 1.0 },
            )
        }
        EffectParams::Phaser(p) => {
            let g = (std::f64::consts::PI * f64::from(p.min_hz) / 48_000.0).tan();
            let a = (g - 1.0) / (g + 1.0);
            let ap = div(add((a, 0.0), z), add((1.0, 0.0), scale(z, a)));
            let cascade = (0..6).fold((1.0, 0.0), |h, _| mul(h, ap));
            (
                div(
                    cascade,
                    add((1.0, 0.0), scale(mul(z, cascade), -f64::from(p.feedback))),
                ),
                f64::from(p.mix),
                1.0,
            )
        }
        _ => unreachable!(),
    };
    let output = add((1.0 - mix, 0.0), scale(wet, mix * polarity));
    output.0.hypot(output.1)
}

fn tone(hz: f64, frames: usize) -> Vec<f32> {
    (0..frames)
        .map(|n| (0.02 * (std::f64::consts::TAU * hz * n as f64 / 48_000.0).sin()) as f32)
        .collect()
}
fn rms(x: &[f32]) -> f64 {
    (x.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
}

#[test]
fn flanger_and_phaser_match_analytic_tones_including_signed_feedback_and_polarity() {
    let mut worst = 0.0_f64;
    for feed in [-0.8, 0.0, 0.8] {
        for inverted in [false, true] {
            let p = EffectParams::Flanger(FlangerParams {
                delay_ms: 1.0,
                depth_ms: 0.0,
                rate_hz: 0.0,
                feedback: feed,
                damping: 0.4,
                invert_wet: inverted,
                mix: 0.5,
                ..Default::default()
            });
            for hz in [137.0, 500.0, 1300.0, 6000.0] {
                let input = tone(hz, 48_000);
                let output = process(p, &input, 137);
                let measured = rms(&output[24_000..]) / rms(&input[24_000..]);
                let expected = magnitude(p, hz);
                worst = worst.max((measured - expected).abs());
                assert!(
                    (measured - expected).abs() < 0.0001,
                    "{p:?} at {hz}: {measured} vs {expected}"
                );
            }
        }
        let p = EffectParams::Phaser(PhaserParams {
            min_hz: 1000.0,
            max_hz: 1000.0,
            rate_hz: 0.0,
            feedback: feed,
            mix: 0.5,
            ..Default::default()
        });
        for hz in [137.0, 500.0, 1000.0, 6000.0] {
            let input = tone(hz, 48_000);
            let output = process(p, &input, 137);
            let measured = rms(&output[24_000..]) / rms(&input[24_000..]);
            let expected = magnitude(p, hz);
            worst = worst.max((measured - expected).abs());
            assert!(
                (measured - expected).abs() < 0.0001,
                "{p:?} at {hz}: {measured} vs {expected}"
            );
        }
    }
    eprintln!("static analytic tone gain: worst absolute error {worst:e}");
}

#[test]
fn flanger_comb_and_three_phaser_notches_move_to_independently_predicted_frequencies() {
    for delay_ms in [1.0, 2.0, 4.0] {
        let p = EffectParams::Flanger(FlangerParams {
            delay_ms,
            depth_ms: 0.0,
            feedback: 0.0,
            damping: 0.0,
            mix: 0.5,
            ..Default::default()
        });
        let hz = 500.0 / f64::from(delay_ms);
        let input = tone(hz, 48_000);
        let output = process(p, &input, 64);
        assert!(rms(&output[12000..]) / rms(&input[12000..]) < 1e-4);
        assert!(rms(&process(p, &tone(hz * 0.65, 48_000), 64)[12000..]) > 0.005);
    }
    for corner in [250.0, 1000.0, 3000.0] {
        let p = EffectParams::Phaser(PhaserParams {
            min_hz: corner,
            max_hz: corner,
            rate_hz: 0.0,
            feedback: 0.0,
            mix: 0.5,
            ..Default::default()
        });
        let g = (std::f64::consts::PI * f64::from(corner) / 48_000.0).tan();
        for k in 0..3 {
            let angle = (2 * k + 1) as f64 * std::f64::consts::PI / 12.0;
            let notch = 48_000.0 / std::f64::consts::PI * (g * angle.tan()).atan();
            let input = tone(notch, 48_000);
            let output = process(p, &input, 113);
            assert!(
                rms(&output[12000..]) / rms(&input[12000..]) < 0.0002,
                "{corner} notch {notch}"
            );
            let adjacent = process(p, &tone(notch * 0.65, 48_000), 113);
            assert!(rms(&adjacent[12000..]) > 0.003);
        }
    }
}

#[test]
fn flanger_feedback_impulses_and_phaser_direct_form_impulse_are_independent_references() {
    let mut input = vec![0.0; 8192];
    input[0] = 0.1;
    for feed in [-0.8, 0.0, 0.8] {
        let p = EffectParams::Flanger(FlangerParams {
            delay_ms: 1.0,
            depth_ms: 0.0,
            feedback: feed,
            damping: 0.0,
            mix: 1.0,
            ..Default::default()
        });
        let output = process(p, &input, 7);
        let mut expected = vec![0.0; input.len()];
        for n in 1..input.len() / 48 {
            expected[n * 48] = 0.1 * feed.powi(n as i32 - 1);
        }
        assert!(difference(&output, &expected) < 1e-7);
        let p = EffectParams::Phaser(PhaserParams {
            min_hz: 1000.0,
            max_hz: 1000.0,
            rate_hz: 0.0,
            feedback: feed,
            mix: 1.0,
            ..Default::default()
        });
        let g = (std::f64::consts::PI * 1000.0 / 48_000.0).tan();
        let a = (g - 1.0) / (g + 1.0);
        let mut x = [0.0_f64; 6];
        let mut y = [0.0_f64; 6];
        let mut fb = 0.0;
        let mut expected = vec![0.0; input.len()];
        for (i, &input) in input.iter().enumerate() {
            let mut v = f64::from(input) + f64::from(feed) * fb;
            for stage in 0..6 {
                let next = a * v + x[stage] - a * y[stage];
                x[stage] = v;
                y[stage] = next;
                v = next;
            }
            fb = v;
            expected[i] = v as f32;
        }
        assert!(difference(&process(p, &input, 7), &expected) < 2e-6);
    }
}

#[test]
fn moving_flanger_matches_delayed_sine_and_phaser_sweep_is_real_and_stereo_independent() {
    let input = tone(440.0, 48_000);
    let p = FlangerParams {
        delay_ms: 1.0,
        depth_ms: 4.0,
        rate_hz: 0.5,
        feedback: 0.0,
        damping: 0.0,
        mix: 1.0,
        ..Default::default()
    };
    let output = process(EffectParams::Flanger(p), &input, 137);
    for (n, &sample) in output.iter().enumerate().skip(1000) {
        let lfo = (std::f64::consts::TAU * 0.5 * n as f64 / 48_000.0).sin();
        let d = (1.0 + 2.0 * (1.0 + lfo)) * 48.0;
        let reference = 0.02 * (std::f64::consts::TAU * 440.0 * (n as f64 - d) / 48_000.0).sin();
        assert!((f64::from(sample) - reference).abs() < 1e-5);
    }
    for kind in [EffectKind::Flanger, EffectKind::Phaser] {
        let mut params = kind.default_params();
        params.set(3, 0.0);
        let mut fx = AnyEffect::new(&params);
        fx.prepare(RATE, 512);
        let mut left = input.clone();
        let mut right = input.clone();
        fx.process(&mut left, &mut right);
        assert_eq!(left, right);
        params.set(3, 0.25);
        params.set(kind.descriptors().len() - 1, 1.0);
        let mut fx = AnyEffect::new(&params);
        fx.prepare(RATE, 512);
        let mut stereo_left = input.clone();
        let mut stereo_right = input.clone();
        fx.process(&mut stereo_left, &mut stereo_right);
        assert!(difference(&stereo_left[2000..], &stereo_right[2000..]) > 0.005);
        let mut fx = AnyEffect::new(&params);
        fx.prepare(RATE, 512);
        let mut isolated = input.clone();
        let mut zero = vec![0.0; input.len()];
        fx.process(&mut isolated, &mut zero);
        assert!(zero.iter().all(|x| *x == 0.0));
        let static_p = match params {
            EffectParams::Flanger(p) => EffectParams::Flanger(FlangerParams { depth_ms: 0.0, ..p }),
            EffectParams::Phaser(p) => EffectParams::Phaser(PhaserParams {
                min_hz: 1000.0,
                max_hz: 1000.0,
                ..p
            }),
            _ => unreachable!(),
        };
        assert!(difference(&isolated[2000..], &process(static_p, &input, 137)[2000..]) > 0.005);
    }
}

#[test]
fn every_live_ramp_noop_empty_reset_mix_bypass_and_callback_are_frame_clocked() {
    let input = tone(440.0, 9000);
    for kind in KINDS {
        let initial = kind.default_params();
        let mut target = initial;
        for (i, info) in kind.descriptors().iter().enumerate() {
            target.set(i, info.min + 0.7 * (info.max - info.min));
        }
        let run = |block: usize, repeat: bool| {
            let mut fx = AnyEffect::new(&initial);
            fx.prepare(RATE, 512);
            let mut left = input.clone();
            let mut right = input.clone();
            for (l, r) in left[..4000]
                .chunks_mut(block)
                .zip(right[..4000].chunks_mut(block))
            {
                fx.process(l, r);
            }
            fx.set_params(&target);
            fx.process(&mut [], &mut []);
            for (l, r) in left[4000..]
                .chunks_mut(block)
                .zip(right[4000..].chunks_mut(block))
            {
                if repeat {
                    fx.set_params(&target);
                }
                fx.process(l, r);
            }
            left
        };
        assert_eq!(run(1, true), run(512, false));
        assert_eq!(run(7, true), run(137, false));
        let mut fx = AnyEffect::new(&initial);
        fx.prepare(RATE, 512);
        let mut junk = input.clone();
        let mut other = input.clone();
        fx.process(&mut junk, &mut other);
        fx.set_params(&target);
        fx.reset();
        fx.process(&mut [], &mut []);
        let mut left = input.clone();
        let mut right = input.clone();
        fx.process(&mut left, &mut right);
        assert_eq!(left, process(target, &input, 7));
        let mut dry = target;
        dry.set(kind.descriptors().len() - 1, 0.0);
        assert_eq!(process(dry, &input, 137), input);
        let mut slot = EffectSlot::new(AnyEffect::new(&target));
        slot.prepare(RATE, 512);
        let mut l = [0.2; 512];
        let mut r = [-0.1; 512];
        assert_eq!(
            super::realtime::allocator_calls(|| {
                for n in 0..32 {
                    slot.set_params(&target);
                    slot.set_enabled(n % 8 < 4);
                    slot.set_mix((n % 5) as f32 / 4.0);
                    slot.set_tempo(120.0);
                    if n == 19 {
                        slot.reset();
                    }
                    slot.process(
                        &mut l[..[1, 7, 64, 512][n % 4]],
                        &mut r[..[1, 7, 64, 512][n % 4]],
                    );
                    let _ = (slot.latency_samples(), slot.tail_samples());
                }
            }),
            0
        );
        slot.set_enabled(false);
        for _ in 0..4 {
            l.fill(0.2);
            r.fill(-0.1);
            slot.process(&mut l, &mut r);
        }
        assert_eq!(l, [0.2; 512]);
        assert_eq!(r, [-0.1; 512]);
    }
}

#[test]
fn preparation_failure_extrema_and_tail_bounds_are_finite_bounded_and_observable() {
    for kind in KINDS {
        for rate in [
            1.0,
            8000.0,
            44_100.0,
            96_000.0,
            384_000.0,
            f32::MAX,
            f32::NAN,
        ] {
            let mut params = kind.default_params();
            for (i, info) in kind.descriptors().iter().enumerate() {
                params.set(i, info.max);
            }
            let mut fx = AnyEffect::new(&params);
            let mut l = [f32::INFINITY, f32::NAN, f32::MAX, -f32::MAX];
            let mut r = l;
            fx.process(&mut l, &mut r);
            assert_eq!(l, [0.0, 0.0, 1000.0, -1000.0]);
            fx.prepare(rate, 512);
            assert_eq!(fx.latency_samples(), 0);
            assert_eq!(params.latency_samples(rate), 0);
            assert_eq!(kind.max_latency_samples(rate), 0);
            for n in 0..64 {
                for (i, info) in kind.descriptors().iter().enumerate() {
                    params.set(i, if n % 2 == 0 { info.min } else { info.max });
                }
                fx.set_params(&params);
                l = [1000.0, -1000.0, 1e-39, 0.0];
                r = l;
                fx.process(&mut l, &mut r);
                assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() < 1e7));
            }
            assert!(fx.tail_samples() >= fx.warm_up_samples());
            assert!(fx.tail_samples() < 4_000_000);
        }
        let mut fx = AnyEffect::new(&kind.default_params());
        fx.prepare(RATE, 512);
        let tail = fx.tail_samples();
        let mut l = vec![0.0; tail + 1024];
        let mut r = l.clone();
        l[0] = 1.0;
        fx.process(&mut l, &mut r);
        assert!(l[tail..].iter().all(|v| v.abs() < 1e-6), "{kind:?} tail");
        assert!(r.iter().all(|v| *v == 0.0));
    }
}

#[test]
fn phaser_idle_tail_is_independent_per_channel_and_does_not_gate_continuous_input() {
    let params = EffectParams::Phaser(PhaserParams {
        min_hz: 20.0,
        max_hz: 20.0,
        rate_hz: 0.0,
        feedback: 0.85,
        mix: 1.0,
        ..Default::default()
    });
    let mut fx = AnyEffect::new(&params);
    fx.prepare(RATE, 512);
    let tail = fx.tail_samples();
    let mut left = vec![0.02; tail + 1024];
    let mut right = vec![0.0; left.len()];
    right[0] = 0.1;
    fx.process(&mut left, &mut right);
    assert!(right[tail..].iter().all(|v| *v == 0.0));
    assert!(left[tail..].iter().all(|v| v.abs() > 0.01));
}

fn phaser_reentry_params() -> PhaserParams {
    PhaserParams {
        min_hz: 20.0,
        max_hz: 20.0,
        rate_hz: 0.0,
        feedback: 0.85,
        mix: 1.0,
        ..Default::default()
    }
}

fn phaser_reentry_input(fade_age: usize) -> (Vec<f32>, usize) {
    let mut input = vec![1.0; 48_000];
    input.resize(48_000 + 192_000 + fade_age, 0.0);
    let reentry = input.len();
    input.resize(reentry + 960, 1e-6);
    (input, reentry)
}

fn process_phaser_parts(
    fx: &mut AnyEffect,
    left: &mut [f32],
    right: &mut [f32],
    partitions: &[usize],
    repeated: Option<&EffectParams>,
) {
    assert_eq!(left.len(), right.len());
    assert!(!partitions.is_empty() && partitions.iter().all(|n| *n > 0));
    assert_eq!(
        super::realtime::allocator_calls(|| {
            let mut start = 0;
            let mut piece = 0;
            while start < left.len() {
                if let Some(params) = repeated {
                    fx.set_params(params);
                    fx.set_tempo(120.0);
                }
                // Empty calls are deliberately inserted at every boundary.
                fx.process(&mut [], &mut []);
                let end = (start + partitions[piece % partitions.len()]).min(left.len());
                fx.process(&mut left[start..end], &mut right[start..end]);
                let _ = (fx.latency_samples(), fx.tail_samples());
                start = end;
                piece += 1;
            }
        }),
        0
    );
}

fn phaser_static_wet_reference(input: &[f32], p: PhaserParams) -> Vec<f64> {
    // Independent direct-form allpasses, not the processor's lattice states.
    // This reference keeps evolving through silence/reentry with signed
    // feedback. The separate envelope assertions must not freeze or erase it.
    let g = (std::f64::consts::PI * f64::from(p.min_hz) / 48_000.0).tan();
    let a = (g - 1.0) / (g + 1.0);
    let mut previous_input = [0.0; 6];
    let mut previous_output = [0.0; 6];
    let mut feedback = 0.0;
    input
        .iter()
        .map(|input| {
            let mut wet = f64::from(*input) + f64::from(p.feedback) * feedback;
            for stage in 0..6 {
                let next = a * wet + previous_input[stage] - a * previous_output[stage];
                previous_input[stage] = wet;
                previous_output[stage] = next;
                wet = next;
            }
            feedback = wet;
            wet
        })
        .collect()
}

#[test]
fn phaser_tail_fade_reentry_recovers_gain_without_a_history_jump() {
    let p = phaser_reentry_params();
    let params = EffectParams::Phaser(p);
    let (input, reentry) = phaser_reentry_input(240);
    let mut right_input = input.clone();
    right_input[reentry..reentry + 100].fill(0.0);
    let render = |parts: &[usize], repeated| {
        let mut fx = AnyEffect::new(&params);
        fx.prepare(RATE, 512);
        let mut left = input.clone();
        let mut right = right_input.clone();
        process_phaser_parts(&mut fx, &mut left, &mut right, parts, repeated);
        (left, right)
    };
    let (output, right) = render(&[1], None);
    assert_eq!(
        (output.clone(), right.clone()),
        render(&[7, 137, 1, 512, 31], Some(&params))
    );
    let previous = output[reentry - 1];
    let resumed = output[reentry];
    let jump = (resumed - previous).abs();
    eprintln!(
        "Phaser actual tail reentry: previous={previous:.10}, resumed={resumed:.10}, jump={jump:.10}"
    );
    assert!(jump < 1e-4, "tail recovery jumped by {jump}");
    assert!(
        (0.003..0.0034).contains(&previous),
        "old tail history was lost"
    );
    let mut maximum = 0.0_f64;
    for (actual, source, offset) in [(&output, &input, 0), (&right, &right_input, 100)] {
        let reference = phaser_static_wet_reference(source, p);
        let new_input = phaser_static_wet_reference(&source[reentry..], p);
        let starting_gain = 1.0 - (240 + offset) as f64 / 480.0;
        // Five seconds of high-feedback f32 processing accumulates history
        // rounding relative to f64. Normalize only the residual history at
        // the last quiet frame, leaving the new-input response unscaled.
        // The independent recurrence still evolves that history: holding it
        // fixed, clearing it early or snapping its gain cannot pass this test.
        let prior = reentry + offset - 1;
        let history_scale = f64::from(actual[prior]) / (reference[prior] * starting_gain);
        assert!((history_scale - 1.0).abs() < 0.01);
        for n in 0..960 {
            let gain = if n < offset {
                1.0 - (240 + n + 1) as f64 / 480.0
            } else {
                starting_gain + (1.0 - starting_gain) * ((n - offset + 1) as f64 / 480.0).min(1.0)
            };
            let wet = (reference[reentry + n] - new_input[n]) * history_scale + new_input[n];
            let error = (f64::from(actual[reentry + n]) - wet * gain).abs();
            maximum = maximum.max(error);
            assert!(
                error < 5e-7,
                "reentry offset {offset}, frame {n}: error {error}"
            );
        }
    }
    eprintln!("Phaser retained-history/envelope reference max error={maximum:.10}");
}

#[test]
fn phaser_tail_reentry_control_noop_reset_and_dry_semantics_keep_the_frame_clock() {
    let p = phaser_reentry_params();
    let params = EffectParams::Phaser(p);
    let changed = EffectParams::Phaser(PhaserParams { mix: 0.25, ..p });
    let (input, reentry) = phaser_reentry_input(240);
    let reference = phaser_static_wet_reference(&input, p);
    let render = |parts: &[usize], repeated| {
        let mut fx = AnyEffect::new(&params);
        fx.prepare(RATE, 512);
        let mut left = input.clone();
        let mut right = input.clone();
        process_phaser_parts(
            &mut fx,
            &mut left[..reentry],
            &mut right[..reentry],
            parts,
            None,
        );
        fx.set_params(&changed);
        process_phaser_parts(
            &mut fx,
            &mut left[reentry..],
            &mut right[reentry..],
            parts,
            repeated,
        );
        (fx, left)
    };
    let (mut fx, output) = render(&[1], Some(&changed));
    let (_, irregular) = render(&[137, 7, 512, 1, 31], None);
    assert_eq!(output, irregular);
    for n in 0..960 {
        let progress = ((n + 1) as f64 / 480.0).min(1.0);
        let gain = 0.5 + 0.5 * progress;
        let mix = 1.0 - 0.75 * progress;
        let dry = f64::from(input[reentry + n]);
        let expected = dry + mix * (reference[reentry + n] * gain - dry);
        assert!((f64::from(output[reentry + n]) - expected).abs() < 1e-5);
    }

    // Reset halfway through another recovery must start at full gain with
    // cleared filter state, and snap the currently requested controls.
    let mut left = input[..reentry].to_vec();
    let mut right = left.clone();
    fx.reset();
    process_phaser_parts(&mut fx, &mut left, &mut right, &[137], None);
    let mut onset = [1e-6; 73];
    let mut other = onset;
    process_phaser_parts(&mut fx, &mut onset, &mut other, &[7], Some(&changed));
    assert_eq!(
        super::realtime::allocator_calls(|| {
            fx.reset();
            fx.set_params(&changed);
            fx.process(&mut [], &mut []);
        }),
        0
    );
    let mut reset_left = vec![0.02; 960];
    let mut reset_right = reset_left.clone();
    process_phaser_parts(
        &mut fx,
        &mut reset_left,
        &mut reset_right,
        &[7, 137],
        Some(&changed),
    );
    assert_eq!(reset_left, process(changed, &[0.02; 960], 1));

    let dry = EffectParams::Phaser(PhaserParams { mix: 0.0, ..p });
    let mut fx = AnyEffect::new(&dry);
    fx.prepare(RATE, 512);
    let mut left = input.clone();
    let mut right = input.clone();
    process_phaser_parts(&mut fx, &mut left, &mut right, &[1, 137, 7], Some(&dry));
    assert_eq!(left, input);
    assert_eq!(right, input);
}

#[test]
fn phaser_tail_reentry_after_state_clear_recovers_from_zero_gain() {
    let p = phaser_reentry_params();
    let params = EffectParams::Phaser(p);
    for fade_age in [480, 1000] {
        let (input, reentry) = phaser_reentry_input(fade_age);
        let mut fx = AnyEffect::new(&params);
        fx.prepare(RATE, 512);
        let mut left = input.clone();
        let mut right = input.clone();
        process_phaser_parts(
            &mut fx,
            &mut left,
            &mut right,
            &[1, 137, 7, 512],
            Some(&params),
        );
        assert_eq!(left[reentry - 1], 0.0);
        let reference = phaser_static_wet_reference(&input[reentry..], p);
        for (n, wet) in reference.into_iter().enumerate() {
            let gain = ((n + 1) as f64 / 480.0).min(1.0);
            assert!(
                (f64::from(left[reentry + n]) - wet * gain).abs() < 1e-9,
                "cleared tail, frame {n}"
            );
        }
    }
}

#[test]
fn append_only_contracts_and_authored_settings_roundtrip_with_every_index() {
    let old = [
        "eq",
        "compressor",
        "limiter",
        "reverb",
        "delay",
        "balance",
        "dcBlock",
        "channelMute",
        "polarity",
        "stereoMatrix",
        "softClipper",
        "distortion",
        "fastLowpass",
        "selectableFilter",
        "bassShelf",
    ];
    for (i, kind) in EffectKind::ALL[..15].iter().enumerate() {
        assert_eq!(*kind as usize, i);
        assert_eq!(serde_json::to_value(kind).unwrap(), old[i]);
    }
    // Other deliveries can append their kinds during parent composition.
    let modulation: Vec<_> = EffectKind::ALL[15..]
        .iter()
        .copied()
        .filter(|kind| KINDS.contains(kind))
        .collect();
    assert_eq!(modulation, KINDS);
    let examples = [
        include_str!("../../../../docs/examples/modulation/chorus-wide.json"),
        include_str!("../../../../docs/examples/modulation/flanger-negative-feedback.json"),
        include_str!("../../../../docs/examples/modulation/phaser-sweep.json"),
    ];
    for (kind, example) in KINDS.into_iter().zip(examples) {
        let defaults = kind.default_params();
        let encoded = serde_json::to_value(defaults).unwrap();
        assert_eq!(
            serde_json::from_value::<EffectParams>(encoded.clone()).unwrap(),
            defaults
        );
        assert_eq!(
            serde_json::from_value::<EffectParams>(serde_json::json!({"type":kind})).unwrap(),
            defaults
        );
        for (i, info) in kind.descriptors().iter().enumerate() {
            assert_eq!(defaults.get(i), Some(info.default));
            let mut changed = defaults;
            assert!(changed.set(i, info.max));
            assert_eq!(changed.get(i), Some(info.max));
            assert_eq!(changed, changed.sanitized());
        }
        let mut untouched = defaults;
        assert!(!untouched.set(kind.descriptors().len(), 0.0));
        assert_eq!(untouched, defaults);
        let example: EffectParams = serde_json::from_str(example).unwrap();
        assert_eq!(example.kind(), kind);
        assert_eq!(example.sanitized(), example);
        let mut fx = AnyEffect::new(&example);
        fx.prepare(RATE, 512);
        assert_eq!(fx.kind(), kind);
        assert!(fx.set_params(&example));
        assert!(!fx.set_params(&EffectKind::Eq.default_params()));
    }
}

#[test]
#[ignore = "CPU/memory observations; run serially with --nocapture"]
fn modulation_observations() {
    use std::time::Instant;
    for kind in KINDS {
        let mut effect = AnyEffect::new(&kind.default_params());
        for rate in [48_000.0, 384_000.0] {
            let bytes = super::realtime::allocated_bytes(|| effect.prepare(rate, 512));
            eprintln!(
                "{} prepare at {rate}: {bytes} new buffer bytes",
                kind.name()
            );
        }
        effect.prepare(RATE, 512);
        let mut left = [0.1; 256];
        let mut right = [-0.2; 256];
        let mut total = 0.0;
        let mut maximum = 0.0_f64;
        for _ in 0..1875 {
            left.fill(0.1);
            right.fill(-0.2);
            let start = Instant::now();
            effect.process(&mut left, &mut right);
            let elapsed = start.elapsed().as_secs_f64();
            total += elapsed;
            maximum = maximum.max(elapsed);
        }
        std::hint::black_box((&left, &right));
        eprintln!(
            "{}: 10 seconds stereo, elapsed DSP calls {:.4}s, {:.1}x realtime; max observed 256-frame call {:.3}ms",
            kind.name(),
            total,
            10.0 / total,
            maximum * 1000.0
        );
    }
}

#[test]
#[ignore = "writes original dry/wet listening fixtures to an explicitly named folder"]
fn modulation_render_examples() {
    let Some(folder) = std::env::var_os("WINDFALL_DSP_RENDER_DIR") else {
        panic!("set WINDFALL_DSP_RENDER_DIR for authored listening files");
    };
    let folder = std::path::Path::new(&folder);
    std::fs::create_dir_all(folder).unwrap();
    // Original sustained harmonic chord, followed by silence for the tail.
    let input: Vec<_> = (0..288_000)
        .map(|n| {
            if n >= 192_000 {
                return 0.0;
            }
            let envelope = (n as f32 / 2400.0).min(1.0) * ((192_000 - n) as f32 / 4800.0).min(1.0);
            [220.0, 277.1826, 329.6276]
                .iter()
                .map(|hz| {
                    let phase = std::f64::consts::TAU * hz * n as f64 / 48_000.0;
                    (phase.sin() + 0.3 * (2.0 * phase).sin() + 0.15 * (3.0 * phase).sin()) as f32
                        * 0.05
                        * envelope
                })
                .sum()
        })
        .collect();
    super::support::write_wav(&folder.join("dry-chord.wav"), &input, &input, RATE);
    for kind in KINDS {
        let mut fx = AnyEffect::new(&kind.default_params());
        fx.prepare(RATE, 512);
        let mut left = input.clone();
        let mut right = input.clone();
        for (l, r) in left.chunks_mut(256).zip(right.chunks_mut(256)) {
            fx.process(l, r);
        }
        super::support::write_wav(
            &folder.join(format!("{}.wav", kind.name().to_lowercase())),
            &left,
            &right,
            RATE,
        );
    }
}
