//! Independent references: nearest discrete code, analytic capture times,
//! closed-form two-pole impulse/DTFT, and oversampled memoryless transfers.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::f64::consts::TAU;
use std::hint::black_box;
use std::time::Instant;
use ts_rs::TS;
use windfall_dsp::lofi::{LOFI_MAX_RATIO, Lofi, LofiParams};
use windfall_dsp::{Effect, ParamKind, ParamSet};

thread_local! {
    static WATCH: Cell<bool> = const { Cell::new(false) };
    static CALLS: Cell<[usize; 4]> = const { Cell::new([0; 4]) };
}
struct Allocator;
fn count(kind: usize) {
    if WATCH.try_with(Cell::get).unwrap_or(false) {
        let _ = CALLS.try_with(|cell| {
            let mut counts = cell.get();
            counts[kind] += 1;
            cell.set(counts);
        });
    }
}
// SAFETY: each call forwards the original layout/pointer unchanged to System.
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count(0);
        // SAFETY: the caller supplies a valid layout.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count(1);
        // SAFETY: the caller supplies a valid layout.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count(2);
        // SAFETY: the caller upholds the allocator contract.
        unsafe { System.realloc(pointer, layout, size) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        count(3);
        // SAFETY: the caller upholds the allocator contract.
        unsafe { System.dealloc(pointer, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

fn guarded(work: impl FnOnce()) -> [usize; 4] {
    struct Stop;
    impl Drop for Stop {
        fn drop(&mut self) {
            WATCH.set(false);
        }
    }
    CALLS.set([0; 4]);
    WATCH.set(true);
    let stop = Stop;
    work();
    drop(stop);
    CALLS.get()
}

fn prepared(params: LofiParams, rate: f32) -> Lofi {
    let mut effect = Lofi::default();
    effect.prepare(rate, 512);
    effect.set_params(&params);
    effect
}
fn render(params: LofiParams, rate: f32, input: &[f32], block: usize) -> Vec<f32> {
    let mut effect = prepared(params, rate);
    let mut left = input.to_vec();
    let mut right = vec![0.0; input.len()];
    for (l, r) in left.chunks_mut(block).zip(right.chunks_mut(block)) {
        effect.process(l, r);
    }
    left
}
fn noise(seed: u64, count: usize) -> Vec<f32> {
    let mut state = seed;
    (0..count)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            ((state >> 40) as f32 / 8_388_608.0 - 1.0) * 0.8
        })
        .collect()
}

// Independent nearest-code decision: compare distances to neighboring codes.
// No product quantizer, ramp, filter or coefficient API is called by references.
fn nearest(input: f64, bits: u8) -> f64 {
    let end = 2.0_f64.powi(i32::from(bits) - 1) - 1.0;
    let magnitude = input.abs().min(1.0);
    let below = (magnitude * end).floor();
    let above = (below + 1.0).min(end);
    let code = if magnitude - below / end < above / end - magnitude {
        below
    } else {
        above
    };
    input.signum() * code / end
}

#[test]
fn parameter_api_defaults_indices_json_ts_and_sanitization() {
    let ids = [
        "bits",
        "quantize",
        "rateRatio",
        "driveDb",
        "distortion",
        "cutoffHz",
        "filter",
        "mix",
        "outputDb",
    ];
    let defaults = LofiParams::default();
    let json = serde_json::to_value(defaults).unwrap();
    assert_eq!(serde_json::from_str::<LofiParams>("{}").unwrap(), defaults);
    assert_eq!(
        serde_json::from_value::<LofiParams>(json.clone()).unwrap(),
        defaults
    );
    assert!(serde_json::from_str::<LofiParams>(r#"{"bits":256}"#).is_err());
    assert!(serde_json::from_str::<LofiParams>(r#"{"bits":2.5}"#).is_err());
    let ts = LofiParams::decl(&ts_rs::Config::default());
    assert_eq!(LofiParams::NAME, "Lo-fi reduction");
    assert_eq!(LofiParams::descriptors().len(), ids.len());
    for (index, info) in LofiParams::descriptors().iter().enumerate() {
        assert_eq!(info.id, ids[index]);
        assert_eq!(LofiParams::index_of(info.id), Some(index));
        assert_eq!(defaults.get(index), Some(info.default));
        assert_eq!(json[info.id].as_f64().unwrap() as f32, info.default);
        assert!(ts.contains(info.id), "missing TS field {}: {ts}", info.id);
        let mut p = defaults;
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(p.set(index, invalid));
            assert_eq!(p.get(index), Some(info.default));
        }
        p.set(index, -f32::MAX);
        assert_eq!(p.get(index), Some(info.min));
        p.set(index, f32::MAX);
        assert_eq!(p.get(index), Some(info.max));
        if info.kind == ParamKind::Float {
            p.set(index, info.min + 0.37 * (info.max - info.min));
            assert_ne!(p.get(index), Some(info.default));
        }
    }
    let mut p = defaults;
    assert!(!p.set(9, 0.0));
    assert_eq!(p.get(9), None);
    assert_eq!(LofiParams::index_of("missing"), None);
    p.bits = 0;
    p.mix = f32::NAN;
    p.cutoff_hz = f32::INFINITY;
    let clean = p.sanitized();
    assert_eq!(clean.bits, 2);
    assert_eq!(clean.mix, defaults.mix);
    assert_eq!(clean.cutoff_hz, defaults.cutoff_hz);
    p.bits = 255;
    assert_eq!(p.sanitized().bits, 16);
    p = LofiParams::neutral();
    assert!(p.approach(&defaults, 0.5));
    assert_eq!(p.bits, defaults.bits);
    for _ in 0..100 {
        p.approach(&defaults, 0.5);
    }
    assert_eq!(p, defaults);
}

#[test]
fn every_bit_level_signed_codes_rounding_endpoints_saturation_and_headroom() {
    for bits in 2..=16 {
        let end = (1_i32 << (bits - 1)) - 1;
        let mut input: Vec<f32> = (-end..=end).map(|k| k as f32 / end as f32).collect();
        input.extend([
            -f32::MAX,
            -8.0,
            -1.0,
            -0.5,
            -0.0,
            0.0,
            0.5,
            1.0,
            8.0,
            f32::MAX,
        ]);
        for code in [0, 1, end / 2, end - 1] {
            let threshold = (f64::from(code) + 0.5) / f64::from(end);
            for offset in [-2.0e-7, 0.0, 2.0e-7] {
                input.extend([(threshold + offset) as f32, -(threshold + offset) as f32]);
            }
        }
        let p = LofiParams {
            bits,
            quantize: 1.0,
            ..LofiParams::neutral()
        };
        let output = render(p, 48_000.0, &input, 137);
        for (index, (&x, &y)) in input.iter().zip(&output).enumerate() {
            let expected = nearest(f64::from(x), bits) as f32;
            assert!(
                (y - expected).abs() < 2.0e-7,
                "bits={bits} index={index} input={x} got={y} expected={expected}"
            );
            assert!(y.abs() <= 1.0);
            if index < (2 * end + 1) as usize {
                assert!(
                    (y - x).abs() < 2.0e-7,
                    "missing code bits={bits} index={index}"
                );
            }
        }
    }
    assert_eq!(
        render(
            LofiParams {
                bits: 2,
                quantize: 1.0,
                ..LofiParams::neutral()
            },
            48_000.0,
            &[-0.5, 0.5],
            1
        ),
        [-1.0, 1.0]
    );
    let input = [-f32::MAX, -8.0, -1.0, -0.0, 0.0, 1.0, 8.0, f32::MAX];
    let neutral = render(LofiParams::neutral(), 48_000.0, &input, 7);
    assert_eq!(
        neutral.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
        input.map(f32::to_bits)
    );
    let partial = render(
        LofiParams {
            bits: 2,
            quantize: 0.25,
            ..LofiParams::neutral()
        },
        48_000.0,
        &[4.0, -4.0],
        1,
    );
    assert_eq!(partial, [3.25, -3.25]);
}

#[test]
fn integer_fractional_clock_capture_counts_and_irregular_partitions() {
    let input: Vec<f32> = (0..100_003).map(|n| n as f32 * 0.000_01 + 1.0).collect();
    for ratio in [1.0, 2.0, 3.0, 7.0, 64.0, 1.25, 2.5, 3.75, 7.1, 63.9] {
        let p = LofiParams {
            rate_ratio: ratio,
            ..LofiParams::neutral()
        };
        let expected: Vec<f32> = (0..input.len())
            .map(|n| {
                let capture =
                    ((n as f64 / f64::from(ratio)).floor() * f64::from(ratio)).ceil() as usize;
                input[capture]
            })
            .collect();
        for block in [1, 7, 64, 137, 512] {
            let output = render(p, 48_000.0, &input, block);
            assert_eq!(output, expected, "clock ratio={ratio} block={block}");
            let captures = 1 + output.windows(2).filter(|pair| pair[0] != pair[1]).count();
            let expected_count = ((input.len() - 1) as f64 / f64::from(ratio)).floor() as usize + 1;
            assert_eq!(
                captures, expected_count,
                "captures ratio={ratio} block={block}"
            );
        }
    }
}

#[test]
fn stereo_impulses_noise_and_antiphase_are_independent() {
    let p = LofiParams::default();
    let input_l = noise(0x1234_5678, 8192);
    let input_r = noise(0x7654_3210, 8192);
    let expected_l = render(p, 48_000.0, &input_l, 64);
    let expected_r = render(p, 48_000.0, &input_r, 7);
    let mut effect = prepared(p, 48_000.0);
    let mut left = input_l.clone();
    let mut right = input_r;
    effect.process(&mut left, &mut right);
    assert_eq!(left, expected_l);
    assert_eq!(right, expected_r);
    effect.reset();
    left = input_l.clone();
    right = input_l.iter().map(|x| -x).collect();
    effect.process(&mut left, &mut right);
    assert!(
        left.iter()
            .zip(&right)
            .all(|(l, r)| (*l + *r).abs() < 1.0e-7)
    );
    for side in 0..2 {
        effect.reset();
        left.fill(0.0);
        right.fill(0.0);
        if side == 0 {
            left[0] = 1.0;
        } else {
            right[0] = 1.0;
        }
        effect.process(&mut left, &mut right);
        let (active, silent) = if side == 0 {
            (&left, &right)
        } else {
            (&right, &left)
        };
        assert!(active.iter().any(|x| *x != 0.0));
        assert!(silent.iter().all(|x| *x == 0.0));
    }
}

fn effective_rate(rate: f32) -> f64 {
    if rate.is_finite() {
        f64::from(rate).clamp(1.0, 384_000.0)
    } else {
        48_000.0
    }
}
fn filter_pole(cutoff: f32, rate: f32) -> f64 {
    let rate = effective_rate(rate);
    (-TAU * f64::from(cutoff).min(0.45 * rate) / rate).exp()
}
fn spectral(samples: &[f32], frequency: f64, rate: f64) -> (f64, f64) {
    samples
        .iter()
        .enumerate()
        .fold((0.0, 0.0), |(re, im), (n, x)| {
            let phase = TAU * frequency * n as f64 / rate;
            (
                re + f64::from(*x) * phase.cos(),
                im - f64::from(*x) * phase.sin(),
            )
        })
}

#[test]
fn filter_impulse_dc_spectra_and_cutoff_extremes_at_all_rates() {
    for rate in [
        44_100.0,
        48_000.0,
        96_000.0,
        192_000.0,
        384_000.0,
        1.0,
        0.001,
        0.0,
        -1.0,
        f32::NAN,
        f32::INFINITY,
    ] {
        for cutoff in [20.0, 1000.0, 20_000.0] {
            let pole = filter_pole(cutoff, rate);
            let alpha = 1.0 - pole;
            let count = (24.0 / -pole.ln()).ceil() as usize + 128;
            let mut input = vec![0.0; count];
            input[0] = 1.0;
            let p = LofiParams {
                cutoff_hz: cutoff,
                filter: 1.0,
                ..LofiParams::neutral()
            };
            let output = render(p, rate, &input, 137);
            for (n, &y) in output.iter().enumerate() {
                // Convolution of two geometric impulse sequences, not a state recurrence.
                let expected = alpha * alpha * (n + 1) as f64 * pole.powi(n as i32);
                assert!(
                    (f64::from(y) - expected).abs() < 2.0e-7,
                    "impulse rate={rate} cutoff={cutoff} n={n} y={y} ref={expected}"
                );
                assert!(y.is_finite() && (0.0..=1.0).contains(&y));
            }
            let dc: f64 = output.iter().map(|x| f64::from(*x)).sum();
            assert!(
                (dc - 1.0).abs() < 2.0e-6,
                "DC rate={rate} cutoff={cutoff} sum={dc}"
            );
            let fs = effective_rate(rate);
            for hz in [0.0, f64::from(cutoff).min(fs * 0.45), fs * 0.1, fs * 0.4] {
                let (re, im) = spectral(&output, hz, fs);
                let measured = re.hypot(im);
                // |alpha/(1-p*z^-1)|² for the two cascaded real poles.
                let expected =
                    alpha * alpha / (1.0 + pole * pole - 2.0 * pole * (TAU * hz / fs).cos());
                assert!(
                    (measured - expected).abs() < 3.0e-6,
                    "spectrum rate={rate} cutoff={cutoff} hz={hz} got={measured} ref={expected}"
                );
            }
            let steady = render(p, rate, &vec![0.75; count], 64);
            assert!(
                (steady.last().unwrap() - 0.75).abs() < 1.0e-6,
                "steady DC rate={rate} cutoff={cutoff}"
            );
        }
    }
}

#[test]
fn moving_filter_remains_bounded_and_mix_keeps_headroom() {
    for rate in [44_100.0, 48_000.0, 96_000.0, 192_000.0, 1.0] {
        let mut effect = prepared(LofiParams::neutral(), rate);
        let mut p = LofiParams {
            filter: 1.0,
            ..LofiParams::neutral()
        };
        for n in 0..2048 {
            p.cutoff_hz = if n % 2 == 0 { 20.0 } else { 20_000.0 };
            effect.set_params(&p);
            let mut left = [if n % 3 == 0 { -4.0 } else { 4.0 }; 17];
            let mut right = left.map(|x| -x);
            effect.process(&mut left, &mut right);
            assert!(left.iter().all(|x| x.is_finite() && x.abs() <= 4.0));
            assert!(left.iter().zip(right).all(|(l, r)| (*l + r).abs() < 1.0e-7));
        }
        let dry = render(
            LofiParams {
                mix: 0.0,
                output_db: 0.0,
                ..LofiParams::default()
            },
            rate,
            &[8.0, -8.0],
            1,
        );
        assert_eq!(dry, [8.0, -8.0]);
    }
}

fn memoryless(input: f64, p: LofiParams) -> f64 {
    let driven = input * 10.0_f64.powf(f64::from(p.drive_db) / 20.0);
    let saturated = driven / (1.0 + driven.abs());
    let shaped = input * (1.0 - f64::from(p.distortion)) + saturated * f64::from(p.distortion);
    shaped * (1.0 - f64::from(p.quantize)) + nearest(shaped, p.bits) * f64::from(p.quantize)
}

#[test]
fn drive_transfer_quantization_and_intentional_alias_spectra() {
    for drive in [0.0, 6.0, 36.0] {
        for distortion in [0.0, 0.25, 1.0] {
            let p = LofiParams {
                drive_db: drive,
                distortion,
                ..LofiParams::neutral()
            };
            let input = [-8.0, -1.0, -0.25, 0.0, 0.25, 1.0, 8.0];
            let output = render(p, 48_000.0, &input, 1);
            for (&x, y) in input.iter().zip(output) {
                assert!(
                    (f64::from(y) - memoryless(f64::from(x), p)).abs() < 5.0e-7,
                    "drive={drive} distortion={distortion} x={x} y={y}"
                );
                if distortion == 1.0 {
                    assert!(y.abs() < 1.0);
                }
            }
        }
    }
    // Exact-bin input at 9/64 of 48k: its third harmonic (20.25k) is below
    // Nyquist, and its fifth (33.75k) folds to 14.25k at the native rate.
    let count = 4096;
    let fs = 48_000.0;
    let fundamental = fs * 9.0 / 64.0;
    let input: Vec<f32> = (0..count)
        .map(|n| (0.8 * (TAU * fundamental * n as f64 / fs).sin()) as f32)
        .collect();
    for p in [
        LofiParams {
            drive_db: 24.0,
            distortion: 1.0,
            ..LofiParams::neutral()
        },
        LofiParams {
            bits: 3,
            quantize: 1.0,
            ..LofiParams::neutral()
        },
    ] {
        let output = render(p, fs as f32, &input, 137);
        let reference: Vec<f32> = input
            .iter()
            .map(|x| memoryless(f64::from(*x), p) as f32)
            .collect();
        assert!(
            output
                .iter()
                .zip(&reference)
                .all(|(a, b)| (*a - *b).abs() < 2.0e-7)
        );
        for bin in [576, 1216, 1728, 1920] {
            let hz = bin as f64 * fs / count as f64;
            let actual = spectral(&output, hz, fs);
            let expected = spectral(&reference, hz, fs);
            assert!(
                (actual.0 - expected.0).hypot(actual.1 - expected.1) / (count as f64) < 2.0e-7,
                "native alias bin={bin} p={p:?}"
            );
        }
        let folded = spectral(&output, 14_250.0, fs);
        let folded_amplitude = 2.0 * folded.0.hypot(folded.1) / count as f64;
        assert!(
            folded_amplitude > 0.01,
            "intentional folded fifth absent: {folded_amplitude} p={p:?}"
        );
        // Independent 16x analytic input at the same physical frequency; no
        // product processor is used at high rate and no native alias suppression
        // is claimed. Isolate the unfolded fifth and the native folded bin.
        let high_fs = fs * 16.0;
        let high: Vec<f32> = (0..count * 16)
            .map(|n| memoryless(0.8 * (TAU * fundamental * n as f64 / high_fs).sin(), p) as f32)
            .collect();
        let high_fold = spectral(&high, 14_250.0, high_fs);
        let high_fifth = spectral(&high, 33_750.0, high_fs);
        let high_fold_amplitude = 2.0 * high_fold.0.hypot(high_fold.1) / high.len() as f64;
        let high_fifth_amplitude = 2.0 * high_fifth.0.hypot(high_fifth.1) / high.len() as f64;
        assert!(
            high_fold_amplitude < 0.005,
            "high-rate folded component={high_fold_amplitude}"
        );
        assert!(
            high_fifth_amplitude > 0.01,
            "high-rate fifth={high_fifth_amplitude}"
        );
    }
}

#[test]
fn rate_reduction_has_measurable_unfiltered_aliases() {
    // Native 9k sampled at 12k aliases to 3k. A 4-frame hold has its own
    // sinc envelope; reference is the analytic capture-time sine, no DSP clock.
    let fs = 48_000.0;
    let input: Vec<f32> = (0..8192)
        .map(|n| (TAU * 9000.0 * n as f64 / fs).sin() as f32)
        .collect();
    let p = LofiParams {
        rate_ratio: 4.0,
        ..LofiParams::neutral()
    };
    let output = render(p, fs as f32, &input, 7);
    let expected: Vec<f32> = (0..input.len()).map(|n| input[n / 4 * 4]).collect();
    assert_eq!(output, expected);
    let (re, im) = spectral(&output, 3000.0, fs);
    assert!(2.0 * re.hypot(im) / output.len() as f64 > 0.85);
}

#[test]
fn smoothing_noops_fast_retargets_empty_reset_and_step_bounds() {
    let mut effect = prepared(LofiParams::neutral(), 48_000.0);
    effect.process(&mut [], &mut []);
    let p = LofiParams {
        mix: 0.0,
        output_db: -6.0,
        ..LofiParams::neutral()
    };
    effect.set_params(&p);
    let mut l = [1.0];
    let mut r = [1.0];
    effect.process(&mut l, &mut r);
    assert!(
        (l[0] - 10.0_f32.powf(-6.0 / 20.0)).abs() < 1.0e-7,
        "empty process consumed fresh state"
    );
    effect.reset();
    effect.set_params(&LofiParams::neutral());
    effect.process(&mut [0.75], &mut [0.75]);
    let target = LofiParams {
        quantize: 1.0,
        bits: 2,
        ..LofiParams::neutral()
    };
    effect.set_params(&target);
    let mut last = 0.75;
    for n in 1..=240 {
        // Repeated writes, tempo and empty calls cannot prolong the ramp.
        effect.set_params(&target);
        effect.set_tempo(if n % 2 == 0 { 20.0 } else { f32::NAN });
        effect.process(&mut [], &mut []);
        l = [0.75];
        r = [0.75];
        effect.process(&mut l, &mut r);
        assert!(
            (l[0] - last).abs() <= 0.02,
            "resolution step n={n} previous={last} current={}",
            l[0]
        );
        last = l[0];
    }
    assert_eq!(last, 1.0, "ramp did not land in 240 frames");
    // Retarget mix from its current position, with an independently linear
    // amplitude reference. All nonlinear settings have already settled.
    let target = LofiParams { mix: 0.0, ..target };
    effect.set_params(&target);
    for n in 1..=120 {
        l = [0.75];
        r = [0.75];
        effect.process(&mut l, &mut r);
        let expected = 1.0 - 0.25 * n as f32 / 240.0;
        assert!(
            (l[0] - expected).abs() < 2.0e-6,
            "mix ramp n={n} y={} ref={expected}",
            l[0]
        );
    }
    let target = LofiParams { mix: 1.0, ..target };
    effect.set_params(&target);
    for n in 1..=240 {
        l = [0.75];
        r = [0.75];
        effect.process(&mut l, &mut r);
        let expected = 0.875 + 0.125 * n as f32 / 240.0;
        assert!(
            (l[0] - expected).abs() < 3.0e-6,
            "retarget n={n} y={} ref={expected}",
            l[0]
        );
    }
    effect.set_params(&LofiParams::default());
    effect.process(&mut [0.3; 11], &mut [-0.3; 11]);
    effect.reset();
    let mut a = [0.0; 512];
    let mut b = a;
    effect.process(&mut a, &mut b);
    assert_eq!(a, [0.0; 512]);
    assert_eq!(b, a);
    let mut fresh = prepared(LofiParams::default(), 48_000.0);
    a = [0.3; 512];
    b = [-0.3; 512];
    let mut c = a;
    let mut d = b;
    effect.reset();
    effect.process(&mut a, &mut b);
    fresh.process(&mut c, &mut d);
    assert_eq!(a, c);
    assert_eq!(b, d);
}

#[test]
fn resolution_morph_all_depth_pairs_keeps_authored_control_step_bound() {
    for input in [0.001, 0.12, 0.25, 0.5, 0.75, 0.99, 1.5] {
        for from in 2..=16 {
            for to in 2..=16 {
                let p = LofiParams {
                    bits: from,
                    quantize: 1.0,
                    ..LofiParams::neutral()
                };
                let mut effect = prepared(p, 48_000.0);
                let mut l = [input];
                let mut r = [-input];
                effect.process(&mut l, &mut r);
                let mut last = l[0];
                effect.set_params(&LofiParams { bits: to, ..p });
                for n in 1..=240 {
                    l = [input];
                    r = [-input];
                    effect.process(&mut l, &mut r);
                    assert!(
                        (l[0] - last).abs() <= 0.02,
                        "pair {from}->{to} x={input} n={n} previous={last} current={}",
                        l[0]
                    );
                    assert!((l[0] + r[0]).abs() < 1.0e-7);
                    last = l[0];
                }
                assert!(
                    (f64::from(last) - nearest(f64::from(input), to)).abs() < 2.0e-7,
                    "pair did not land {from}->{to} x={input}"
                );
            }
        }
    }
}

#[test]
fn drive_arrives_in_five_ms_at_representative_rates_with_analytic_ramp_reference() {
    for rate in [44_100.0, 48_000.0, 96_000.0, 192_000.0] {
        let mut effect = prepared(LofiParams::neutral(), rate);
        effect.process(&mut [0.4], &mut [-0.4]);
        let target = LofiParams {
            drive_db: 36.0,
            distortion: 1.0,
            ..LofiParams::neutral()
        };
        effect.set_params(&target);
        let frames = (0.005_f64 * f64::from(rate)).round() as usize;
        let target_gain = 10.0_f64.powf(36.0 / 20.0);
        let mut previous = 0.4;
        for n in 1..=frames + 1 {
            let t = (n as f64 / frames as f64).min(1.0);
            let driven = 0.4 * (1.0 + (target_gain - 1.0) * t);
            let expected = 0.4 * (1.0 - t) + driven / (1.0 + driven) * t;
            let mut left = [0.4];
            let mut right = [-0.4];
            effect.process(&mut left, &mut right);
            assert!(
                (f64::from(left[0]) - expected).abs() < 3.0e-6,
                "drive ramp rate={rate} n={n} y={} ref={expected}",
                left[0]
            );
            assert!(
                (left[0] - previous).abs() <= 0.02,
                "drive step rate={rate} n={n}"
            );
            previous = left[0];
        }
    }
}

fn automated(block: usize, noops: bool) -> (Vec<f32>, Vec<f32>) {
    let mut effect = prepared(LofiParams::default(), 48_000.0);
    let mut p = LofiParams::default();
    let mut left = noise(54321, 8192);
    let mut right = noise(12345, 8192);
    let mut frame = 0;
    while frame < left.len() {
        if frame % 23 == 0 {
            let index = frame / 23 % 9;
            let info = &LofiParams::descriptors()[index];
            p.set(
                index,
                if frame / 23 % 2 == 0 {
                    info.min
                } else {
                    info.max
                },
            );
            effect.set_params(&p);
        }
        if frame == 2990 {
            effect.reset();
        }
        if noops {
            effect.set_params(&p);
            effect.process(&mut [], &mut []);
            effect.set_tempo(120.0);
        }
        let next_event = ((frame / 23 + 1) * 23).min(if frame < 2990 { 2990 } else { 8192 });
        let end = (frame + block).min(next_event).min(left.len());
        effect.process(&mut left[frame..end], &mut right[frame..end]);
        frame = end;
    }
    (left, right)
}

#[test]
fn deterministic_bit_parity_1_7_64_137_512_with_automation_and_resets() {
    let expected = automated(1, false);
    assert!(
        expected
            .0
            .iter()
            .chain(&expected.1)
            .all(|x| x.is_finite() && x.abs() <= 4.0)
    );
    for block in [1, 7, 64, 137, 512] {
        for noops in [false, true] {
            let actual = automated(block, noops);
            assert_eq!(
                actual.0.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                expected.0.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                "left block={block} noops={noops}"
            );
            assert_eq!(
                actual.1.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                expected.1.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
                "right block={block} noops={noops}"
            );
        }
    }
}

#[test]
fn conservative_frozen_tail_and_gap_cover_maximal_finite_headroom() {
    for rate in [1.0, 44_100.0, 48_000.0, 96_000.0, 192_000.0, 384_000.0] {
        let p = LofiParams {
            rate_ratio: LOFI_MAX_RATIO,
            cutoff_hz: 20.0,
            filter: 1.0,
            ..LofiParams::neutral()
        };
        let mut effect = prepared(p, rate);
        assert_eq!(effect.latency_samples(), 0);
        assert_eq!(effect.warm_up_samples(), 0);
        assert_eq!(effect.gap_samples(), effect.tail_samples());
        let frozen = effect.tail_samples();
        let mut l = [f32::MAX];
        let mut r = [-f32::MAX];
        effect.process(&mut l, &mut r);
        assert!(l[0].is_finite() && r[0].is_finite());
        let mut processed = 0;
        while processed < frozen + 512 {
            let count = (frozen + 512 - processed).min(137);
            let mut left = [0.0; 137];
            let mut right = left;
            effect.process(&mut left[..count], &mut right[..count]);
            for (n, y) in left[..count].iter().enumerate() {
                assert!(y.is_finite());
                if processed + n >= frozen {
                    assert!(
                        y.abs() < 3.1623e-5,
                        "tail rate={rate} frame={} y={y}",
                        processed + n
                    );
                }
            }
            processed += count;
        }
        for index in 0..9 {
            let mut p = p;
            p.set(index, LofiParams::descriptors()[index].max);
            effect.set_params(&p);
            assert_eq!(
                effect.tail_samples(),
                frozen,
                "tail was not frozen index={index}"
            );
        }
    }
}

#[test]
fn callback_allocation_zero_for_all_extremes_and_fast_automation() {
    assert_eq!(
        guarded(|| {
            let mut processor = black_box(Lofi::default());
            processor.prepare(384_000.0, usize::MAX);
            black_box(processor);
        }),
        [0; 4],
        "construction/maximum prepare fixed-storage contract"
    );
    let mut effect = prepared(LofiParams::default(), 192_000.0);
    let mut left = [0.75; 137];
    let mut right = [-0.75; 137];
    let calls = guarded(|| {
        for n in 0..2048 {
            let mut p = LofiParams::neutral();
            for (index, info) in LofiParams::descriptors().iter().enumerate() {
                p.set(
                    index,
                    if (n >> index) & 1 == 0 {
                        info.min
                    } else {
                        info.max
                    },
                );
            }
            effect.set_params(&p);
            effect.set_params(&p);
            if n % 17 == 0 {
                effect.reset();
            }
            effect.set_tempo(140.0);
            left.fill(0.75);
            right.fill(-0.75);
            effect.process(&mut left, &mut right);
            effect.process(&mut [], &mut []);
        }
    });
    assert_eq!(calls, [0; 4], "alloc/zeroed/realloc/free calls");
    // Positive control exercises all FOUR allocator entry points, including
    // zeroed allocation and freeing; a guard unable to see frees is insufficient.
    let layout = Layout::from_size_align(64, 8).unwrap();
    let observed = guarded(|| {
        // SAFETY: valid layouts, successful pointers and matching deallocation.
        unsafe {
            let first = black_box(std::alloc::alloc(black_box(layout)));
            assert!(!first.is_null());
            let first = black_box(std::alloc::realloc(black_box(first), layout, 128));
            assert!(!first.is_null());
            std::alloc::dealloc(black_box(first), Layout::from_size_align(128, 8).unwrap());
            let zero = std::alloc::alloc_zeroed(layout);
            assert!(!zero.is_null());
            black_box(std::slice::from_raw_parts(zero, 64));
            std::alloc::dealloc(zero, layout);
        }
    });
    assert_eq!(observed, [1, 1, 1, 2]);
    assert!(!std::mem::needs_drop::<Lofi>());
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "non-finite lo-fi input")]
fn invalid_signal_is_visible_in_debug_direct_dsp_contract() {
    prepared(LofiParams::default(), 48_000.0).process(&mut [f32::NAN], &mut [0.0]);
}

#[test]
#[cfg(not(debug_assertions))]
fn invalid_signal_remains_visible_in_release() {
    let mut effect = prepared(LofiParams::default(), 48_000.0);
    let mut left = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY];
    let mut right = [0.0; 3];
    effect.process(&mut left, &mut right);
    assert!(left[0].is_nan());
    assert_eq!(left[1], f32::INFINITY);
    assert_eq!(left[2], f32::NEG_INFINITY);
    assert_eq!(right, [0.0; 3]);
}

#[test]
#[ignore = "optional release headless throughput; explicitly invoke, no hardware deadline claim"]
fn scoped_cpu_throughput() {
    for rate in [44_100.0, 48_000.0, 96_000.0, 192_000.0] {
        for automate in [false, true] {
            let mut effect = prepared(LofiParams::default(), rate);
            let mut p = LofiParams::default();
            let mut l = [0.0; 128];
            let mut r = l;
            let source_l: [f32; 128] = noise(12, 128).try_into().unwrap();
            let source_r: [f32; 128] = noise(34, 128).try_into().unwrap();
            let iterations = 30_000;
            let start = Instant::now();
            for n in 0..iterations {
                if automate {
                    let index = n % 9;
                    let info = &LofiParams::descriptors()[index];
                    p.set(index, if n % 2 == 0 { info.min } else { info.max });
                    effect.set_params(black_box(&p));
                }
                l.copy_from_slice(&source_l);
                r.copy_from_slice(&source_r);
                effect.process(black_box(&mut l), black_box(&mut r));
                black_box((&l, &r));
            }
            let elapsed = start.elapsed().as_secs_f64();
            println!(
                "rate={rate} automated={automate}: {:.3} M stereo frames/s, {:.3} us/128 frames, {:.3}% walltime at host rate; fixed {} bytes, params {} bytes, prepare heap 0 bytes; frozen tail {} frames",
                iterations as f64 * 128.0 / elapsed / 1e6,
                elapsed * 1e6 / iterations as f64,
                elapsed * f64::from(rate) / (iterations as f64 * 128.0) * 100.0,
                std::mem::size_of::<Lofi>(),
                std::mem::size_of::<LofiParams>(),
                effect.tail_samples()
            );
        }
    }
}
