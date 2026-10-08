//! Measurements of the seven utility transfers, latency and live state.
use crate::support::{RATES, fft, impulse, noise, prepared, rms, run, sine};
use windfall_dsp::*;

const UTILITIES: [EffectKind; 7] = [
    EffectKind::Balance,
    EffectKind::DcBlock,
    EffectKind::ChannelMute,
    EffectKind::Polarity,
    EffectKind::StereoMatrix,
    EffectKind::SoftClipper,
    EffectKind::Distortion,
];

fn close(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} != {expected}, tolerance {tolerance}"
    );
}

#[test]
fn neutral_utilities_preserve_impulses_and_stereo_samples_exactly() {
    for rate in RATES {
        for kind in [
            EffectKind::Balance,
            EffectKind::ChannelMute,
            EffectKind::Polarity,
            EffectKind::StereoMatrix,
        ] {
            let mut effect = AnyEffect::new(&kind.default_params());
            effect.prepare(rate, 128);
            let (mut l, mut r) = (noise(1, 0.5, 1024), impulse(1024, 37, -0.75));
            let expected = (l.clone(), r.clone());
            effect.process(&mut l, &mut r);
            assert_eq!((l, r), expected, "{kind:?}");
            assert_eq!(effect.latency_samples(), 0);
        }
    }
}

#[test]
fn balance_gain_pan_and_zero_have_measured_endpoints() {
    for (pan, lg, rg) in [
        (-1.0, 2.0, 0.0),
        (-0.5, 2.0, 1.0),
        (0.0, 2.0, 2.0),
        (0.5, 1.0, 2.0),
        (1.0, 0.0, 2.0),
    ] {
        let mut effect: Balance = prepared(&BalanceParams { gain: 2.0, pan }, 48_000.0);
        let (mut l, mut r) = (vec![0.25; 4], vec![-0.5; 4]);
        effect.process(&mut l, &mut r);
        assert_eq!(l, vec![0.25 * lg; 4]);
        assert_eq!(r, vec![-0.5 * rg; 4]);
    }
    let mut effect: Balance = prepared(
        &BalanceParams {
            gain: 0.0,
            pan: 0.7,
        },
        48_000.0,
    );
    let (mut l, mut r) = (vec![1.0; 4], vec![-1.0; 4]);
    effect.process(&mut l, &mut r);
    assert!(l.iter().chain(&r).all(|s| *s == 0.0));
}

#[test]
fn mute_and_polarity_cover_signed_channel_truth_tables() {
    for left in [false, true] {
        for right in [false, true] {
            let mut mute: ChannelMute = prepared(&ChannelMuteParams { left, right }, 48_000.0);
            let mut polarity: Polarity = prepared(&PolarityParams { left, right }, 48_000.0);
            let (mut l, mut r) = (vec![0.25, -0.75], vec![-0.5, 0.125]);
            mute.process(&mut l, &mut r);
            assert_eq!(
                l,
                if left {
                    vec![0.0; 2]
                } else {
                    vec![0.25, -0.75]
                }
            );
            assert_eq!(
                r,
                if right {
                    vec![0.0; 2]
                } else {
                    vec![-0.5, 0.125]
                }
            );
            let (mut l, mut r) = (vec![0.25, -0.75], vec![-0.5, 0.125]);
            polarity.process(&mut l, &mut r);
            assert_eq!(
                l,
                if left {
                    vec![-0.25, 0.75]
                } else {
                    vec![0.25, -0.75]
                }
            );
            assert_eq!(
                r,
                if right {
                    vec![0.5, -0.125]
                } else {
                    vec![-0.5, 0.125]
                }
            );
        }
    }
}

#[test]
fn gain_mute_and_polarity_changes_arrive_in_five_ms_without_a_step() {
    for rate in RATES {
        for kind in [
            EffectKind::Balance,
            EffectKind::ChannelMute,
            EffectKind::Polarity,
        ] {
            let mut effect = AnyEffect::new(&kind.default_params());
            effect.prepare(rate, 512);
            let (mut l, mut r) = (vec![1.0], vec![1.0]);
            effect.process(&mut l, &mut r);
            let mut params = kind.default_params();
            params.set(
                0,
                if kind == EffectKind::Balance {
                    0.0
                } else {
                    1.0
                },
            );
            effect.set_params(&params);
            let frames = (rate * 0.005).round() as usize;
            let (mut l, mut r) = (vec![1.0; frames + 2], vec![1.0; frames + 2]);
            effect.process(&mut l, &mut r);
            let target = if kind == EffectKind::Polarity {
                -1.0
            } else {
                0.0
            };
            close(l[0], 1.0 + (target - 1.0) / frames as f32, 1e-6);
            assert_eq!(l[frames - 1], target);
            assert!(
                l.windows(2)
                    .all(|s| (s[1] - s[0]).abs() <= 2.01 / frames as f32)
            );
        }
    }
}

#[test]
fn dc_rejection_cutoff_and_step_response_are_rate_independent() {
    for rate in RATES {
        for cutoff_hz in [1.0, 5.0, 40.0] {
            let mut effect: DcBlock = prepared(&DcBlockParams { cutoff_hz }, rate);
            let frames = (rate * 2.0) as usize;
            let (mut l, mut r) = (vec![0.4; frames], vec![-0.7; frames]);
            run(&mut effect, &mut l, &mut r, 137);
            let pole = (-std::f32::consts::TAU * cutoff_hz / rate).exp();
            close(l[0], 0.4 * (1.0 + pole) * 0.5, 1e-6);
            let at = (rate * 0.01) as usize;
            // The coefficient is f32, but the filter's state is f64. An f32
            // powi can accumulate rounding in repeated squaring, exceeding
            // this bound even when the actual recurrence is accurate.
            let step = |pole: f64| f64::from(0.4_f32) * (1.0 + pole) * 0.5 * pole.powi(at as i32);
            close(l[at], step(f64::from(pole)) as f32, 2e-6);

            // Independently check the physical f64 transfer. These poles
            // lie in [0.5, 1), where one f32 coefficient ULP is EPSILON/2.
            // Propagate that quantum through the monotone closed-form step;
            // it bounds coefficient rounding/exp precision, not state error.
            let ideal_pole =
                (-std::f64::consts::TAU * f64::from(cutoff_hz) / f64::from(rate)).exp();
            let quantum = f64::from(f32::EPSILON) * 0.5;
            assert!((0.5..1.0).contains(&ideal_pole));
            assert!((f64::from(pole) - ideal_pole).abs() <= quantum);
            let ideal_step = step(ideal_pole);
            let coefficient_error = (step(ideal_pole + quantum) - ideal_step)
                .max(ideal_step - step(ideal_pole - quantum));
            assert!(
                (f64::from(l[at]) - ideal_step).abs() <= coefficient_error + 2e-6,
                "{rate}/{cutoff_hz}: physical step {}, expected {ideal_step}, coefficient budget {coefficient_error}",
                l[at]
            );
            assert!(
                l[frames - 1].abs() < 1e-5 && r[frames - 1].abs() < 1e-5,
                "{rate}/{cutoff_hz}"
            );
            assert_eq!(effect.latency_samples(), 0);
            // At the corner, settled sinusoidal gain is -3.01 dB.
            effect.reset();
            let frames = (rate * 8.0) as usize;
            let input = sine(cutoff_hz, 0.5, frames, rate);
            let (mut l, mut r) = (input.clone(), input.clone());
            run(&mut effect, &mut l, &mut r, 257);
            let start = (rate * 4.0) as usize;
            let gain = 20.0 * (rms(&l[start..]) / rms(&input[start..])).log10();
            assert!((gain + 3.0103).abs() < 0.02, "{rate}/{cutoff_hz}: {gain}");
            effect.reset();
            let tail = effect.tail_samples();
            let (mut l, mut r) = (impulse(tail + 1, 0, 1.0), impulse(tail + 1, 0, -1.0));
            run(&mut effect, &mut l, &mut r, 128);
            assert_eq!(l[tail], 0.0);
            assert_eq!(r[tail], 0.0);
        }
    }
}

#[test]
fn matrix_equations_and_mid_side_views_are_the_same_tested_matrix() {
    let source = (noise(31, 0.5, 128), noise(71, 0.5, 128));
    for mode in [
        MatrixMode::Stereo,
        MatrixMode::MidSide,
        MatrixMode::EncodeMidSide,
        MatrixMode::DecodeMidSide,
    ] {
        let p = StereoMatrixParams {
            mode,
            ll: 0.7,
            lr: -0.3,
            rl: 0.2,
            rr: 1.1,
            ..Default::default()
        };
        let mut effect: StereoMatrix = prepared(&p, 48_000.0);
        let (mut l, mut r) = source.clone();
        effect.process(&mut l, &mut r);
        for i in 0..l.len() {
            let (a, b) = if matches!(mode, MatrixMode::MidSide | MatrixMode::EncodeMidSide) {
                (
                    (source.0[i] + source.1[i]) * 0.5,
                    (source.0[i] - source.1[i]) * 0.5,
                )
            } else {
                (source.0[i], source.1[i])
            };
            let (a, b) = (p.ll * a + p.lr * b, p.rl * a + p.rr * b);
            let (a, b) = if matches!(mode, MatrixMode::MidSide | MatrixMode::DecodeMidSide) {
                (a + b, a - b)
            } else {
                (a, b)
            };
            close(l[i], a, 2e-7);
            close(r[i], b, 2e-7);
        }
    }
    let mut encode: StereoMatrix = prepared(
        &StereoMatrixParams {
            mode: MatrixMode::EncodeMidSide,
            ..Default::default()
        },
        48_000.0,
    );
    let mut decode: StereoMatrix = prepared(
        &StereoMatrixParams {
            mode: MatrixMode::DecodeMidSide,
            ..Default::default()
        },
        48_000.0,
    );
    let (mut l, mut r) = source.clone();
    encode.process(&mut l, &mut r);
    decode.process(&mut l, &mut r);
    for i in 0..l.len() {
        close(l[i], source.0[i], 6e-8);
        close(r[i], source.1[i], 6e-8);
    }
}

#[test]
fn matrix_width_extremes_keep_mono_and_cancel_antisymmetric_content() {
    for width in [-2.0, 0.0, 1.0, 2.0] {
        let mut effect: StereoMatrix = prepared(
            &StereoMatrixParams {
                mode: MatrixMode::MidSide,
                rr: width,
                ..Default::default()
            },
            48_000.0,
        );
        let (mut l, mut r) = (vec![0.25, 0.25], vec![0.25, -0.25]);
        effect.process(&mut l, &mut r);
        assert_eq!((l[0], r[0]), (0.25, 0.25));
        assert_eq!((l[1], r[1]), (0.25 * width, -0.25 * width));
        assert_eq!(l[1] + r[1], 0.0);
    }
}

#[test]
fn matrix_delays_report_common_latency_and_retain_channel_difference() {
    for rate in RATES {
        for (left_delay_ms, right_delay_ms) in [(0.0, 0.0), (2.0, 5.0), (50.0, 50.0), (50.0, 0.0)] {
            let p = StereoMatrixParams {
                left_delay_ms,
                right_delay_ms,
                ..Default::default()
            };
            let mut effect: StereoMatrix = prepared(&p, rate);
            let delays =
                [left_delay_ms, right_delay_ms].map(|ms| (ms * 0.001 * rate).round() as usize);
            assert_eq!(p.latency_samples(rate), delays[0].min(delays[1]));
            assert_eq!(effect.latency_samples(), p.latency_samples(rate));
            assert_eq!(effect.tail_samples(), delays[0].max(delays[1]));
            assert_eq!(effect.gap_samples(), effect.tail_samples());
            let frames = effect.tail_samples() + 2;
            let (mut l, mut r) = (impulse(frames, 0, 0.5), impulse(frames, 0, -0.25));
            run(&mut effect, &mut l, &mut r, 73);
            assert_eq!(l, impulse(frames, delays[0], 0.5));
            assert_eq!(r, impulse(frames, delays[1], -0.25));
        }
    }
}

#[test]
fn matrix_common_delay_automation_crossfades_in_step_with_slot_dry() {
    let p = StereoMatrixParams {
        left_delay_ms: 2.0,
        right_delay_ms: 2.0,
        ..Default::default()
    };
    let mut slot = EffectSlot::new(AnyEffect::new(&EffectParams::StereoMatrix(p)));
    slot.prepare(48_000.0, 128);
    slot.set_mix(0.5);
    let source = sine(173.0, 0.5, 8000, 48_000.0);
    let (mut l, mut r) = (source.clone(), source.clone());
    slot.process(&mut l[..4000], &mut r[..4000]);
    slot.set_params(&EffectParams::StereoMatrix(StereoMatrixParams {
        left_delay_ms: 5.0,
        right_delay_ms: 5.0,
        ..p
    }));
    slot.process(&mut l[4000..], &mut r[4000..]);
    assert_eq!(slot.latency_samples(), 240);
    for i in 4000..4240 {
        let old = (4240 - i) as f32 / 240.0;
        close(
            l[i],
            source[i - 240] + (source[i - 96] - source[i - 240]) * old,
            2e-7,
        );
    }
    assert_eq!(l[4240..], source[4000..7760]);
    assert_eq!(l, r);
}

#[test]
fn zero_latency_matrix_keeps_slot_dry_history_for_its_first_delayed_mix() {
    let mut slot = EffectSlot::new(AnyEffect::new(&EffectKind::StereoMatrix.default_params()));
    slot.prepare(48_000.0, 128);
    slot.set_mix(0.5);
    let source = sine(173.0, 0.5, 8000, 48_000.0);
    let (mut l, mut r) = (source.clone(), source.clone());
    slot.process(&mut l[..4000], &mut r[..4000]);
    slot.set_params(&EffectParams::StereoMatrix(StereoMatrixParams {
        left_delay_ms: 5.0,
        right_delay_ms: 5.0,
        ..Default::default()
    }));
    slot.process(&mut l[4000..], &mut r[4000..]);
    for i in 4000..4240 {
        let old = (4240 - i) as f32 / 240.0;
        close(
            l[i],
            source[i - 240] + (source[i] - source[i - 240]) * old,
            2e-7,
        );
    }
    assert_eq!(l[4240..], source[4000..7760]);
    assert_eq!(l, r);
}

fn slot_partitioned(kind: EffectKind, sizes: &[usize]) -> (Vec<f32>, Vec<f32>) {
    let mut slot = EffectSlot::new(AnyEffect::new(&configured_slot(kind)));
    slot.prepare(48_000.0, 512);
    let (mut l, mut r) = (noise(12, 0.3, 8000), noise(91, 0.3, 8000));
    let mut step = 0;
    for (event, (start, end)) in [
        (0, 501),
        (501, 1301),
        (1301, 2111),
        (2111, 4001),
        (4001, 5301),
        (5301, 8000),
    ]
    .into_iter()
    .enumerate()
    {
        match event {
            1 => slot.set_enabled(false),
            2 => {
                slot.set_params(&edited(kind));
                slot.set_enabled(true);
            }
            3 => slot.set_mix(0.3),
            4 => slot.reset(),
            5 => slot.set_mix(1.0),
            _ => {}
        }
        let mut at = start;
        while at < end {
            let next = (at + sizes[step % sizes.len()]).min(end);
            slot.process(&mut l[at..next], &mut r[at..next]);
            at = next;
            step += 1;
        }
    }
    (l, r)
}
fn configured_slot(kind: EffectKind) -> EffectParams {
    if kind == EffectKind::StereoMatrix {
        EffectParams::StereoMatrix(StereoMatrixParams {
            left_delay_ms: 1.0,
            right_delay_ms: 1.0,
            ..Default::default()
        })
    } else {
        kind.default_params()
    }
}
#[test]
fn utility_slots_are_partition_invariant_through_bypass_mix_reset_and_parameters() {
    for kind in UTILITIES {
        let reference = slot_partitioned(kind, &[8000]);
        for sizes in [&[1, 7, 137, 511, 3, 256][..], &[1][..]] {
            let actual = slot_partitioned(kind, sizes);
            for (side, (actual, expected)) in [
                ("left", (&actual.0, &reference.0)),
                ("right", (&actual.1, &reference.1)),
            ] {
                for (frame, (actual, expected)) in actual.iter().zip(expected).enumerate() {
                    assert_eq!(
                        actual.to_bits(),
                        expected.to_bits(),
                        "{kind:?} {side} frame {frame}, partitions {sizes:?}: {actual} != {expected}"
                    );
                }
            }
        }
    }
}

#[test]
fn utility_descriptors_fix_the_persisted_automation_index_contract() {
    for info in &EffectKind::StereoMatrix.descriptors()[1..5] {
        assert_eq!(info.unit, ParamUnit::None, "signed matrix coefficients");
    }
    assert_eq!(
        &EffectKind::ALL[..5],
        &[
            EffectKind::Eq,
            EffectKind::Compressor,
            EffectKind::Limiter,
            EffectKind::Reverb,
            EffectKind::Delay
        ]
    );
    for (kind, ids) in [
        (EffectKind::Balance, &["gain", "pan"][..]),
        (EffectKind::DcBlock, &["cutoffHz"][..]),
        (EffectKind::ChannelMute, &["left", "right"][..]),
        (EffectKind::Polarity, &["left", "right"][..]),
        (
            EffectKind::StereoMatrix,
            &[
                "mode",
                "ll",
                "lr",
                "rl",
                "rr",
                "leftDelayMs",
                "rightDelayMs",
            ][..],
        ),
        (EffectKind::SoftClipper, &["ceilingDb", "knee"][..]),
        (
            EffectKind::Distortion,
            &["driveDb", "shape", "outputDb"][..],
        ),
    ] {
        assert_eq!(
            kind.descriptors().iter().map(|i| i.id).collect::<Vec<_>>(),
            ids
        );
    }
}

#[test]
#[ignore = "timing measurement: run release utility_effects_realtime_factors with --ignored --nocapture"]
fn utility_effects_realtime_factors() {
    let input = noise(5, 0.8, 512);
    for kind in UTILITIES {
        let mut effect = AnyEffect::new(&kind.default_params());
        effect.prepare(48_000.0, 512);
        let (mut l, mut r) = (input.clone(), input.clone());
        let started = std::time::Instant::now();
        for _ in 0..1500 {
            l.copy_from_slice(&input);
            r.copy_from_slice(&input);
            effect.process(&mut l, &mut r);
            std::hint::black_box((&l, &r));
        }
        let elapsed = started.elapsed().as_secs_f64();
        println!(
            "{}: {:.2}x realtime for stereo 48 kHz / 512 frames ({elapsed:.4} s)",
            kind.name(),
            16.0 / elapsed
        );
    }
}

#[test]
fn soft_clipper_knee_is_linear_below_start_smooth_monotone_symmetric_and_bounded() {
    for ceiling_db in [-24.0, -6.0, 0.0] {
        for knee in [0.05, 0.5, 1.0] {
            let p = SoftClipperParams { ceiling_db, knee };
            let c = windfall_core::db_to_gain(ceiling_db);
            let start = c * (1.0 - knee);
            let end = c * (1.0 + knee);
            let eps = c * 1e-4;
            close(p.transfer(start * 0.9), start * 0.9, 1e-7);
            close(p.transfer(end), c, 1e-7);
            close(p.transfer(1000.0), c, 1e-7);
            let below = (p.transfer(start) - p.transfer(start - eps)) / eps;
            let above = (p.transfer(start + eps) - p.transfer(start)) / eps;
            assert!((below - above).abs() < 0.004);
            assert!((p.transfer(end) - p.transfer(end - eps)) / eps < 0.004);
            let mut last = 0.0;
            for n in 0..500 {
                let x = c * 3.0 * n as f32 / 499.0;
                let y = p.transfer(x);
                assert!(y >= last && y <= c);
                close(p.transfer(-x), -y, 0.0);
                last = y;
                let mut effect: SoftClipper = prepared(&p, 48_000.0);
                let (mut l, mut r) = ([x], [-x]);
                effect.process(&mut l, &mut r);
                assert_eq!(l[0], y);
                assert_eq!(r[0], -y);
            }
        }
    }
}

#[test]
fn distortion_shapes_are_distinct_from_the_polynomial_soft_knee() {
    let soft = DistortionParams {
        drive_db: 0.0,
        shape: 0.0,
        output_db: 0.0,
    };
    let hard = DistortionParams { shape: 1.0, ..soft };
    close(soft.transfer(0.5), 1.0 / 3.0, 1e-7);
    assert_eq!(hard.transfer(0.5), 0.5);
    assert_eq!(hard.transfer(2.0), 1.0);
    close(soft.transfer(2.0), 2.0 / 3.0, 1e-7);
    assert_eq!(SoftClipperParams::default().transfer(0.5), 0.5);
    assert!(SoftClipperParams::default().transfer(1.0) > soft.transfer(1.0));
    for shape in [0.0, 0.5, 1.0] {
        let p = DistortionParams { shape, ..soft };
        let mut effect: Distortion = prepared(&p, 48_000.0);
        let (mut l, mut r) = (vec![0.5; 1000], vec![-0.5; 1000]);
        effect.process(&mut l, &mut r);
        close(l[999], p.transfer(0.5), 2e-5);
        close(r[999], -l[999], 0.0);
    }
}

#[test]
fn distortion_filters_have_measured_group_delay_and_finite_support() {
    for rate in RATES {
        let p = DistortionParams {
            drive_db: 0.0,
            shape: 1.0,
            output_db: 0.0,
        };
        let mut effect: Distortion = prepared(&p, rate);
        let (mut l, mut r) = (impulse(200, 10, 0.1), impulse(200, 10, -0.1));
        effect.process(&mut l, &mut r);
        assert_eq!(effect.latency_samples(), 32);
        assert_eq!(effect.tail_samples(), 64);
        let peak = l
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
            .unwrap()
            .0;
        assert_eq!(peak, 42);
        assert!(l[75..].iter().all(|s| *s == 0.0));
        close(l.iter().sum(), 0.1, 2e-6);
        for (l, r) in l.iter().zip(r) {
            assert_eq!(*l, -r);
        }
    }
}

fn spectrum(samples: &[f32]) -> Vec<f64> {
    let mut re: Vec<f64> = samples.iter().map(|v| f64::from(*v)).collect();
    let mut im = vec![0.0; re.len()];
    fft(&mut re, &mut im);
    let n = re.len() as f64;
    re.iter()
        .zip(im)
        .map(|(a, b)| (a * a + b * b).sqrt() * 2.0 / n)
        .collect()
}

#[test]
fn oversampled_distortion_reduces_alias_energy_against_a_64x_reference() {
    // Coherent ~0.189*Fs sine: only its fundamental belongs below Nyquist.
    // The 64x-rate reference measures the same nonlinearity before an ideal
    // spectral low-pass. Compare all baseband bins, excluding the wanted tone.
    const N: usize = 4096;
    const BIN: usize = 773;
    const HIGH: usize = 64;
    for rate in RATES {
        for drive_db in [6.0, 12.0, 24.0, 36.0] {
            for shape in [0.0, 0.5, 1.0] {
                let p = DistortionParams {
                    drive_db,
                    shape,
                    output_db: 0.0,
                };
                let input = sine(rate * BIN as f32 / N as f32, 0.8, N * 3, rate);
                let raw: Vec<f32> = input[N * 2..].iter().map(|x| p.transfer(*x)).collect();
                let mut effect: Distortion = prepared(&p, rate);
                let (mut l, mut r) = (input.clone(), input);
                run(&mut effect, &mut l, &mut r, 137);
                let measured = spectrum(&l[N * 2..]);
                let raw = spectrum(&raw);
                let high: Vec<f32> = (0..N * HIGH)
                    .map(|n| {
                        let x = (std::f64::consts::TAU * BIN as f64 * n as f64 / (N * HIGH) as f64)
                            .sin()
                            * 0.8;
                        p.transfer(x as f32)
                    })
                    .collect();
                let reference = spectrum(&high);
                let error = |s: &[f64]| {
                    (1..N / 2)
                        .filter(|i| *i != BIN)
                        .map(|i| (s[i] - reference[i]).powi(2))
                        .sum::<f64>()
                        .sqrt()
                };
                let (raw_error, error) = (error(&raw), error(&measured));
                let improvement = 20.0 * (raw_error / error).log10();
                println!(
                    "alias {rate} Hz drive={drive_db} shape={shape}: raw={raw_error:.6}, 4x={error:.6}, improvement={improvement:.2} dB"
                );
                assert!(
                    improvement >= 12.0,
                    "{rate}/{drive_db}/{shape}: {improvement} dB"
                );
                assert!(
                    (measured[BIN] / reference[BIN] - 1.0).abs() < 0.003,
                    "fundamental response"
                );
            }
        }
    }
}

fn edited(kind: EffectKind) -> EffectParams {
    let mut p = kind.default_params();
    for (i, info) in kind.descriptors().iter().enumerate() {
        p.set(
            i,
            if info.default == info.max {
                info.min
            } else {
                info.max
            },
        );
    }
    p
}
fn partitioned(kind: EffectKind, sizes: &[usize]) -> (Vec<f32>, Vec<f32>) {
    let mut effect = AnyEffect::new(&kind.default_params());
    effect.prepare(48_000.0, 512);
    let (mut l, mut r) = (noise(17, 0.5, 8000), noise(99, 0.5, 8000));
    let events = [
        (0, None),
        (197, Some(edited(kind))),
        (901, Some(kind.default_params())),
        (1099, None),
        (2047, Some(edited(kind))),
    ];
    let mut step = 0;
    for (i, (start, params)) in events.iter().enumerate() {
        if let Some(p) = params {
            effect.set_params(p);
        } else {
            effect.reset();
        }
        let end = events.get(i + 1).map_or(l.len(), |e| e.0);
        let mut at = *start;
        while at < end {
            let next = (at + sizes[step % sizes.len()]).min(end);
            effect.process(&mut l[at..next], &mut r[at..next]);
            at = next;
            step += 1;
        }
    }
    (l, r)
}

#[test]
fn every_utility_is_bit_identical_across_irregular_partitions_and_resets() {
    for kind in UTILITIES {
        let reference = partitioned(kind, &[8000]);
        for sizes in [&[1][..], &[1, 7, 137, 511, 3, 256][..], &[128][..]] {
            assert_eq!(partitioned(kind, sizes), reference, "{kind:?} {sizes:?}");
        }
        let mut fresh = AnyEffect::new(&edited(kind));
        fresh.prepare(48_000.0, 512);
        let input = noise(41, 0.5, 3000);
        let (mut l, mut r) = (input.clone(), input.clone());
        fresh.process(&mut l, &mut r);
        fresh.reset();
        let (mut again_l, mut again_r) = (input.clone(), input);
        fresh.process(&mut again_l, &mut again_r);
        assert_eq!((l, r), (again_l, again_r), "{kind:?} reset");
    }
}

#[test]
fn direct_utilities_bound_bad_audio_and_extreme_controls_without_subnormals() {
    for rate in [f32::NAN, 1.0, 44_100.0, 96_000.0, f32::INFINITY] {
        for kind in UTILITIES {
            for value in [
                f32::NAN,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::MAX,
                -f32::MAX,
            ] {
                let mut p = kind.default_params();
                for i in 0..kind.descriptors().len() {
                    p.set(i, value);
                }
                let mut effect = AnyEffect::new(&p);
                effect.prepare(rate, 128);
                let seed = [
                    f32::NAN,
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                    f32::MAX,
                    -f32::MAX,
                    f32::from_bits(1),
                    -f32::from_bits(1),
                    0.5,
                ];
                let (mut l, mut r) = (seed.repeat(400), seed.repeat(400));
                effect.process(&mut l, &mut r);
                assert!(
                    l.iter()
                        .chain(&r)
                        .all(|s| s.is_finite() && !s.is_subnormal() && s.abs() <= 16_000.0),
                    "{kind:?} {rate}/{value}"
                );
                assert!(effect.latency_samples() <= kind.max_latency_samples(rate));
            }
        }
    }
}
