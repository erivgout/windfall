//! Independent equations and scoped realtime evidence for the E3 source APIs.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::f64::consts::PI;
use std::time::Instant;
use windfall_dsp::echo_bank::*;
use windfall_dsp::frequency_delay::*;
use windfall_dsp::{Effect, NoteDivision, ParamKind, ParamSet};

struct WatchedAllocator;
thread_local! { static WATCH: Cell<bool> = const { Cell::new(false) }; static CALLS: Cell<usize> = const { Cell::new(0) }; }
fn charge() {
    WATCH.with(|on| {
        if on.get() {
            CALLS.with(|n| n.set(n.get() + 1));
        }
    });
}
unsafe impl GlobalAlloc for WatchedAllocator {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        charge();
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        charge();
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        charge();
        unsafe { System.realloc(p, l, n) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        charge();
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static ALLOCATOR: WatchedAllocator = WatchedAllocator;
fn watched(f: impl FnOnce()) {
    struct Off;
    impl Drop for Off {
        fn drop(&mut self) {
            WATCH.with(|x| x.set(false));
        }
    }
    CALLS.with(|n| n.set(0));
    WATCH.with(|on| on.set(true));
    let off = Off;
    f();
    drop(off);
    assert_eq!(CALLS.with(Cell::get), 0, "callback allocator calls");
}
fn run<E: Effect>(e: &mut E, l: &mut [f32], r: &mut [f32], blocks: &[usize]) {
    let mut at = 0;
    for n in blocks.iter().copied().cycle() {
        if at == l.len() {
            break;
        }
        let end = (at + n).min(l.len());
        e.process(&mut l[at..end], &mut r[at..end]);
        at = end;
    }
}
fn bank_params() -> EchoBankParams {
    let mut p = EchoBankParams {
        dry: 0.0,
        wet: 1.0,
        ..Default::default()
    };
    for (i, u) in p.units.iter_mut().enumerate() {
        u.enabled = i == 0;
        u.sync = false;
        u.time_ms = 5.0;
        u.feedback = 0.0;
    }
    p
}
fn close(actual: f32, expected: f64, tolerance: f64) {
    assert!(
        (f64::from(actual) - expected).abs() < tolerance,
        "{actual} vs {expected}"
    );
}
fn check_params<P: ParamSet + serde::Serialize + serde::de::DeserializeOwned>(
    expected_count: usize,
) {
    let defaults = P::default();
    assert_eq!(P::descriptors().len(), expected_count);
    let json = serde_json::to_value(defaults).unwrap();
    let empty: P = serde_json::from_str("{}").unwrap();
    assert_eq!(defaults, empty);
    assert_eq!(serde_json::from_value::<P>(json.clone()).unwrap(), defaults);
    for (i, d) in P::descriptors().iter().enumerate() {
        assert_eq!(P::index_of(d.id), Some(i));
        close(defaults.get(i).unwrap(), f64::from(d.default), 1e-6);
        let mut field = &json;
        for part in d.id.split('.') {
            field = if let Ok(index) = part.parse::<usize>() {
                &field[index]
            } else {
                &field[part]
            };
        }
        match d.kind {
            ParamKind::Choice => {
                assert_eq!(field.as_str().unwrap(), d.choices[d.default as usize].value)
            }
            ParamKind::Toggle => assert_eq!(field.as_bool().unwrap(), d.default >= 0.5),
            _ => close(field.as_f64().unwrap() as f32, f64::from(d.default), 1e-6),
        }
        for value in [
            d.min,
            d.max,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            -1e30,
            1e30,
        ] {
            let mut p = defaults;
            assert!(p.set(i, value));
            let p = p.sanitized();
            let v = p.get(i).unwrap();
            assert!(v.is_finite() && v >= d.min && v <= d.max);
            for j in 0..expected_count {
                if j != i {
                    assert_eq!(p.get(j), defaults.get(j));
                }
            }
        }
    }
    let mut p = defaults;
    assert!(!p.set(expected_count, 1.0));
    assert_eq!(p.get(expected_count), None);
}
#[test]
fn parameter_contract_has_all_stable_nested_controls_and_legacy_defaults() {
    check_params::<EchoBankParams>(220);
    check_params::<FrequencyDelayParams>(71);
    assert_eq!(EchoBankParams::index_of("units.0.timeMs"), Some(8));
    assert_eq!(FrequencyDelayParams::index_of("bands.15.pan"), Some(69));
}

#[test]
fn bank_parallel_serial_signed_graph_matches_independent_discrete_paths() {
    let mut e = EchoBank::default();
    e.prepare(8000.0, 137);
    let mut p = bank_params();
    for i in 0..8 {
        p.units[i].enabled = true;
        p.units[i].time_ms = (i + 1) as f32;
        p.units[i].output_gain = 0.125;
    }
    e.set_params(&p);
    assert_eq!(e.warm_up_samples(), 64);
    let (mut l, mut r) = (vec![0.0; 512], vec![0.0; 512]);
    l[0] = 1.0;
    r[0] = 0.25;
    run(&mut e, &mut l, &mut r, &[1, 7, 137]);
    for n in 0..512 {
        let expected = if (8..=64).contains(&n) && n % 8 == 0 {
            0.125
        } else {
            0.0
        };
        close(l[n], expected, 1e-6);
        close(r[n], expected * 0.25, 1e-6);
    }
    e.reset();
    for i in 0..8 {
        p.units[i].input_gain = if i == 0 { 1.0 } else { 0.0 };
        p.units[i].output_gain = if i == 7 { 1.0 } else { 0.0 };
        p.units[i].next_send = if i < 7 { -1.0 } else { 0.0 };
    }
    e.set_params(&p);
    assert_eq!(e.warm_up_samples(), 288);
    l.fill(0.0);
    r.fill(0.0);
    l[0] = 1.0;
    run(&mut e, &mut l, &mut r, &[512]);
    // Eight serial times: 8*(1+...+8)=288; seven negative links.
    for (n, x) in l.iter().enumerate() {
        close(*x, if n == 288 { -1.0 } else { 0.0 }, 1e-6);
    }
}
#[test]
fn bank_echo_recurrence_stereo_offset_and_feedback_modes_are_distinct() {
    let mut e = EchoBank::default();
    e.prepare(8000.0, 512);
    for mode in [
        EchoFeedbackMode::Off,
        EchoFeedbackMode::Normal,
        EchoFeedbackMode::Inverted,
        EchoFeedbackMode::PingPong,
    ] {
        let mut p = bank_params();
        p.units[0].feedback = 0.5;
        p.units[0].feedback_mode = mode;
        e.reset();
        e.set_params(&p);
        let (mut l, mut r) = (vec![0.0; 241], vec![0.0; 241]);
        l[0] = 1.0;
        run(&mut e, &mut l, &mut r, &[7, 1, 137]);
        for k in 1..=6 {
            let side = match mode {
                EchoFeedbackMode::Inverted => 1,
                EchoFeedbackMode::PingPong => (k - 1) % 2,
                _ => 0,
            };
            let amplitude = if mode == EchoFeedbackMode::Off && k > 1 {
                0.0
            } else {
                0.5_f64.powi((k - 1) as i32)
            };
            close(l[k * 40], if side == 0 { amplitude } else { 0.0 }, 1e-6);
            close(r[k * 40], if side == 1 { amplitude } else { 0.0 }, 1e-6);
        }
    }
    let mut p = bank_params();
    p.units[0].stereo_offset_ms = 3.0;
    e.reset();
    e.set_params(&p);
    let (mut l, mut r) = (vec![0.0; 100], vec![0.0; 100]);
    l[0] = 1.0;
    r[0] = 0.5;
    run(&mut e, &mut l, &mut r, &[100]);
    assert_eq!(l[40], 1.0);
    assert_eq!(r[64], 0.5);
    assert_eq!(e.latency_samples(), 0);
}
#[test]
fn bank_sync_full_range_tempo_and_exact_boundaries_at_one_hz() {
    let mut e = EchoBank::default();
    e.prepare(1.0, 128);
    let mut p = bank_params();
    p.units[0].sync = true;
    p.units[0].division = NoteDivision::Whole;
    p.units[0].stereo_offset_ms = 1000.0;
    e.set_tempo(10.0);
    e.set_params(&p);
    let (mut l, mut r) = ([0.0; 64], [0.0; 64]);
    l[0] = 1.0;
    r[0] = 1.0;
    e.process(&mut l, &mut r);
    assert_eq!(l[24], 1.0);
    assert_eq!(r[25], 1.0);
    e.reset();
    e.set_tempo(120.0);
    l.fill(0.0);
    r.fill(0.0);
    l[0] = 1.0;
    r[0] = 1.0;
    e.process(&mut l, &mut r);
    assert_eq!(l[2], 1.0);
    assert_eq!(r[3], 1.0);
    for division in [
        NoteDivision::Whole,
        NoteDivision::Quarter,
        NoteDivision::EighthTriplet,
        NoteDivision::ThirtySecond,
    ] {
        p.units[0].division = division;
        e.reset();
        e.set_params(&p);
        e.set_tempo(10.0);
        l.fill(0.0);
        r.fill(0.0);
        l[0] = 1.0;
        e.process(&mut l, &mut r);
        assert_eq!(l[(division.beats() * 6.0).round().max(1.0) as usize], 1.0);
    }
}

/// Direct-form difference equation derived from the published trapezoidal
/// one-pole transfer, independent of the production integrator state.
fn reference_bands(rate: f64, width: f64, steep: bool, len: usize) -> Vec<Vec<f64>> {
    let mut cumulative = vec![vec![0.0; len]; 15];
    for (i, low) in cumulative.iter_mut().enumerate() {
        let center = (20.0_f64 * 20000.0).sqrt();
        let f = center * ((20.0 * 1000.0_f64.powf((i + 1) as f64 / 16.0)) / center).powf(width);
        let g = (PI * (f / rate).clamp(1e-5, 0.49)).tan();
        let c = g / (1.0 + g);
        let a = 1.0 - 2.0 * c;
        let mut first = vec![0.0; len];
        for n in 0..len {
            first[n] = c * (if n == 0 { 1.0 } else { 0.0 })
                + c * (if n == 1 { 1.0 } else { 0.0 })
                + if n > 0 { a * first[n - 1] } else { 0.0 };
        }
        for n in 0..len {
            low[n] = if steep {
                c * first[n]
                    + if n > 0 {
                        c * first[n - 1] + a * low[n - 1]
                    } else {
                        0.0
                    }
            } else {
                first[n]
            };
        }
    }
    (0..16)
        .map(|i| {
            (0..len)
                .map(|n| {
                    if i == 0 {
                        cumulative[0][n]
                    } else if i == 15 {
                        (if n == 0 { 1.0 } else { 0.0 }) - cumulative[14][n]
                    } else {
                        cumulative[i][n] - cumulative[i - 1][n]
                    }
                })
                .collect()
        })
        .collect()
}
#[test]
fn all_sixteen_band_impulses_levels_pans_delays_match_independent_reference() {
    let mut e = FrequencyDelay::default();
    e.prepare(8000.0, 137);
    for split in [FrequencySplit::Gentle, FrequencySplit::Steep] {
        for width in [0.5, 1.0, 2.0] {
            let reference = reference_bands(
                8000.0,
                f64::from(width),
                split == FrequencySplit::Steep,
                1024,
            );
            for (selected, expected_band) in reference.iter().enumerate() {
                let mut p = FrequencyDelayParams {
                    split,
                    bandwidth: width,
                    ..Default::default()
                };
                for b in &mut p.bands {
                    b.level = 0.0;
                }
                p.bands[selected].level = 0.75;
                p.bands[selected].delay_ms = (selected + 1) as f32;
                p.bands[selected].pan = if selected % 2 == 0 { -1.0 } else { 1.0 };
                e.reset();
                e.set_params(&p);
                let (mut l, mut r) = (vec![0.0; 1024], vec![0.0; 1024]);
                l[0] = 1.0;
                r[0] = 1.0;
                run(&mut e, &mut l, &mut r, &[1, 137, 7]);
                let delay = (selected + 1) * 8;
                for n in 0..1024 {
                    let expected = if n >= delay {
                        0.75 * expected_band[n - delay]
                    } else {
                        0.0
                    };
                    close(l[n], if selected % 2 == 0 { expected } else { 0.0 }, 2e-5);
                    close(r[n], if selected % 2 == 1 { expected } else { 0.0 }, 2e-5);
                }
            }
        }
    }
}
#[derive(Clone, Copy)]
struct Complex {
    re: f64,
    im: f64,
}
impl Complex {
    fn add(self, b: Self) -> Self {
        Self {
            re: self.re + b.re,
            im: self.im + b.im,
        }
    }
    fn sub(self, b: Self) -> Self {
        Self {
            re: self.re - b.re,
            im: self.im - b.im,
        }
    }
    fn mul(self, b: Self) -> Self {
        Self {
            re: self.re * b.re - self.im * b.im,
            im: self.re * b.im + self.im * b.re,
        }
    }
    fn scale(self, v: f64) -> Self {
        Self {
            re: self.re * v,
            im: self.im * v,
        }
    }
    fn div(self, b: Self) -> Self {
        let d = b.re * b.re + b.im * b.im;
        Self {
            re: (self.re * b.re + self.im * b.im) / d,
            im: (self.im * b.re - self.re * b.im) / d,
        }
    }
}
fn crossover_response(index: usize, hz: f64, rate: f64, steep: bool) -> Complex {
    let f = 20.0 * 1000.0_f64.powf((index + 1) as f64 / 16.0);
    let g = (PI * (f / rate).clamp(1e-5, 0.49)).tan();
    let c = g / (1.0 + g);
    let z = Complex {
        re: (-2.0 * PI * hz / rate).cos(),
        im: (-2.0 * PI * hz / rate).sin(),
    };
    let one = Complex { re: 1.0, im: 0.0 };
    let h = one.add(z).scale(c).div(one.sub(z.scale(1.0 - 2.0 * c)));
    if steep { h.mul(h) } else { h }
}
#[test]
fn band_tone_dft_matches_analytic_complex_gain_and_musical_delay_phase() {
    let rate = 48000.0;
    let hz = 1500.0;
    let len = 16384;
    let mut e = FrequencyDelay::default();
    e.prepare(rate as f32, 511);
    for steep in [false, true] {
        for selected in 0..16 {
            let mut p = FrequencyDelayParams {
                split: if steep {
                    FrequencySplit::Steep
                } else {
                    FrequencySplit::Gentle
                },
                ..Default::default()
            };
            for b in &mut p.bands {
                b.level = 0.0;
            }
            p.bands[selected].level = 1.0;
            p.bands[selected].delay_ms = 3.0;
            e.reset();
            e.set_params(&p);
            let mut l: Vec<f32> = (0..len)
                .map(|n| (2.0 * PI * hz * n as f64 / rate).cos() as f32)
                .collect();
            let mut r = vec![0.0; len];
            run(&mut e, &mut l, &mut r, &[137, 1, 511]);
            let one = Complex { re: 1.0, im: 0.0 };
            let h = if selected == 0 {
                crossover_response(0, hz, rate, steep)
            } else if selected == 15 {
                one.sub(crossover_response(14, hz, rate, steep))
            } else {
                crossover_response(selected, hz, rate, steep).sub(crossover_response(
                    selected - 1,
                    hz,
                    rate,
                    steep,
                ))
            };
            let phase = -2.0 * PI * hz * 144.0 / rate;
            let expected = h.mul(Complex {
                re: phase.cos(),
                im: phase.sin(),
            });
            let mut measured = Complex { re: 0.0, im: 0.0 };
            for (n, x) in l.iter().enumerate().skip(8192) {
                let angle = -2.0 * PI * hz * n as f64 / rate;
                measured = measured.add(
                    Complex {
                        re: angle.cos(),
                        im: angle.sin(),
                    }
                    .scale(f64::from(*x) * 2.0 / 8192.0),
                );
            }
            assert!(
                (measured.re - expected.re).abs() < 3e-4
                    && (measured.im - expected.im).abs() < 3e-4,
                "band {selected} steep {steep}: {},{} vs {},{}",
                measured.re,
                measured.im,
                expected.re,
                expected.im
            );
            assert!(r.iter().all(|x| *x == 0.0));
        }
    }
}
#[test]
fn recombination_is_unity_during_bandwidth_and_split_automation_at_all_rates() {
    for rate in [1.0, 8000.0, 48000.0, 384000.0] {
        let mut e = FrequencyDelay::default();
        e.prepare(rate, 137);
        let mut p = FrequencyDelayParams::default();
        e.set_params(&p);
        for step in 0..32 {
            p.bandwidth = if step % 2 == 0 { 0.5 } else { 2.0 };
            p.split = if step % 3 == 0 {
                FrequencySplit::Steep
            } else {
                FrequencySplit::Gentle
            };
            e.set_params(&p);
            let mut l: [f32; 137] =
                std::array::from_fn(|n| ((step * 137 + n) as f32 * 0.123).sin() * 0.25);
            let mut r: [f32; 137] =
                std::array::from_fn(|n| ((step * 137 + n) as f32 * 0.071).cos() * 0.5);
            let (reference_l, reference_r) = (l, r);
            e.process(&mut l, &mut r);
            for n in 0..137 {
                close(l[n], f64::from(reference_l[n]), 1e-6);
                close(r[n], f64::from(reference_r[n]), 1e-6);
            }
        }
        assert_eq!(e.latency_samples(), 0);
    }
}
#[test]
fn equal_band_times_recombine_to_delayed_unity_and_scale_mirroring_is_real() {
    let mut e = FrequencyDelay::default();
    e.prepare(8000.0, 137);
    for (delay, scale, short, expected) in [
        (10.0, 1.0, false, 80),
        (10.0, -1.0, false, 7920),
        (1000.0, 1.0, false, 8000),
        (1000.0, 1.0, true, 800),
        (10.0, 0.0, false, 0),
    ] {
        let mut p = FrequencyDelayParams {
            scale,
            short_range: short,
            ..Default::default()
        };
        for b in &mut p.bands {
            b.delay_ms = delay;
        }
        e.reset();
        e.set_params(&p);
        let (mut l, mut r) = (vec![0.0; 8100], vec![0.0; 8100]);
        l[0] = 1.0;
        r[0] = -0.25;
        run(&mut e, &mut l, &mut r, &[1, 7, 137]);
        for n in 0..8100 {
            close(l[n], if n == expected { 1.0 } else { 0.0 }, 1e-5);
            close(r[n], if n == expected { -0.25 } else { 0.0 }, 1e-5);
        }
    }
}
#[test]
fn frequency_feedback_recurrence_including_zero_time_is_causal() {
    let mut e = FrequencyDelay::default();
    e.prepare(8000.0, 137);
    for ms in [0.0, 1.0] {
        let mut p = FrequencyDelayParams {
            feedback: 0.5,
            ..Default::default()
        };
        for b in &mut p.bands {
            b.delay_ms = ms;
        }
        e.reset();
        e.set_params(&p);
        let (mut l, mut r) = ([0.0; 64], [0.0; 64]);
        l[0] = 1.0;
        e.process(&mut l, &mut r);
        for (n, sample) in l.iter().enumerate() {
            let expected = if ms == 0.0 {
                0.5_f64.powi(n as i32)
            } else if n >= 8 && n % 8 == 0 {
                0.5_f64.powi((n / 8 - 1) as i32)
            } else {
                0.0
            };
            close(*sample, expected, 2e-5);
        }
    }
}

fn extreme_domain<E: Effect>(e: &mut E) {
    let defaults = E::Params::default();
    e.set_params(&defaults);
    for (index, d) in E::Params::descriptors().iter().enumerate() {
        for v in [d.min, d.max, f32::NAN] {
            let mut p = defaults;
            p.set(index, v);
            e.set_params(&p);
            let (mut l, mut r) = ([f32::MAX; 32], [f32::NEG_INFINITY; 32]);
            e.process(&mut l, &mut r);
            assert!(l.iter().chain(&r).all(|x| x.is_finite()));
            e.set_params(&p);
            e.set_tempo(f32::NAN);
            e.process(&mut [], &mut []);
            e.reset();
        }
    }
}
#[test]
fn full_descriptor_domain_extreme_inputs_and_realtime_callbacks_at_all_rates() {
    for rate in [1.0, 8000.0, 48000.0, 384000.0] {
        let mut bank = EchoBank::default();
        bank.prepare(rate, 32);
        let mut freq = FrequencyDelay::default();
        freq.prepare(rate, 32);
        watched(|| {
            extreme_domain(&mut bank);
            extreme_domain(&mut freq);
        });
        let mut p = bank_params();
        for u in &mut p.units {
            u.enabled = true;
            u.time_ms = 1.0;
            u.feedback = 1.0;
            u.next_send = 1.0;
            u.filter.mode = EchoFilterMode::Peak;
            u.filter.q = 10.0;
            u.filter.gain_db = 18.0;
            u.filter.gain = 2.0;
            u.filter.sections = 3;
            u.feedback_filter = u.filter;
        }
        bank.set_params(&p);
        let mut q = FrequencyDelayParams {
            feedback: 1.0,
            ..Default::default()
        };
        for b in &mut q.bands {
            b.delay_ms = 0.0;
        }
        freq.set_params(&q);
        watched(|| {
            for _ in 0..128 {
                let (mut bl, mut br) = ([f32::MAX; 32], [-1000.0; 32]);
                bank.process(&mut bl, &mut br);
                assert!(
                    bl.iter()
                        .chain(&br)
                        .all(|x| x.is_finite() && x.abs() <= 1512.0)
                );
                let (mut l, mut r) = ([f32::MAX; 32], [-1000.0; 32]);
                freq.process(&mut l, &mut r);
                assert!(l.iter().chain(&r).all(|x| x.is_finite()));
                bank.reset();
                freq.reset();
                bank.set_tempo(10.0);
                bank.set_tempo(1000.0);
            }
        });
        println!(
            "rate={rate} bank_bytes={} unit_history_bytes={} frequency_bytes={}",
            bank.prepared_bytes(),
            2 * ((25.0 * rate).ceil() as usize + 1) * 4,
            freq.prepared_bytes()
        );
    }
}
#[test]
fn preparation_rate_policy_and_reprepare_charge_only_retained_storage() {
    let mut bank = EchoBank::default();
    let mut freq = FrequencyDelay::default();
    for (requested, actual) in [
        (f32::NAN, 48000.0),
        (f32::INFINITY, 48000.0),
        (-1.0, 1.0),
        (0.0, 1.0),
        (1e30, 384000.0),
        (8000.0, 8000.0),
        (1.0, 1.0),
    ] {
        bank.prepare(requested, 1);
        freq.prepare(requested, 1);
        assert_eq!(bank.actual_sample_rate(), actual);
        assert_eq!(freq.actual_sample_rate(), actual);
        assert_eq!(
            bank.prepared_bytes(),
            EchoBank::preparation_bytes(requested)
        );
        assert_eq!(
            freq.prepared_bytes(),
            FrequencyDelay::preparation_bytes(requested)
        );
        assert_eq!(
            bank.prepared_bytes(),
            std::mem::size_of::<EchoBank>() + 16 * ((25.0 * actual).ceil() as usize + 1) * 4
        );
        assert_eq!(
            freq.prepared_bytes(),
            std::mem::size_of::<FrequencyDelay>() + 32 * (actual.ceil() as usize + 1) * 4
        );
    }
}

fn automation<E: Effect>(e: &mut E, rate: f32, blocks: &[usize]) -> (Vec<f32>, Vec<f32>) {
    e.prepare(rate, 137);
    let mut p = E::Params::default();
    e.set_params(&p);
    let mut l = Vec::new();
    let mut r = Vec::new();
    for step in 0..40 {
        for (i, d) in E::Params::descriptors().iter().enumerate() {
            p.set(i, if step % 2 == 0 { d.min } else { d.max });
        }
        e.set_params(&p);
        e.set_tempo(if step % 2 == 0 { 10.0 } else { 1000.0 });
        e.process(&mut [], &mut []);
        let mut a: [f32; 137] =
            std::array::from_fn(|n| ((step * 137 + n) as f32 * 0.03).sin() * 0.1);
        let mut b: [f32; 137] =
            std::array::from_fn(|n| ((step * 137 + n) as f32 * 0.07).cos() * 0.15);
        run(e, &mut a, &mut b, blocks);
        l.extend(a);
        r.extend(b);
    }
    (l, r)
}
#[test]
fn automation_noops_and_reset_are_block_partition_invariant() {
    let mut a = EchoBank::default();
    let x = automation(&mut a, 8000.0, &[137]);
    let mut b = EchoBank::default();
    assert_eq!(x, automation(&mut b, 8000.0, &[1, 7, 29]));
    let mut c = FrequencyDelay::default();
    let y = automation(&mut c, 8000.0, &[137]);
    let mut d = FrequencyDelay::default();
    assert_eq!(y, automation(&mut d, 8000.0, &[1, 7, 29]));
    a.reset();
    b.reset();
    let p = bank_params();
    a.set_params(&p);
    b.set_params(&p);
    let (mut l, mut r) = ([0.0; 512], [0.0; 512]);
    l[0] = 1.0;
    let (mut rl, mut rr) = (l, r);
    a.process(&mut l, &mut r);
    for n in 0..512 {
        b.set_params(&p);
        b.set_tempo(120.0);
        b.process(&mut rl[n..n + 1], &mut rr[n..n + 1]);
    }
    assert_eq!((l, r), (rl, rr));
}
#[test]
fn rapid_time_retarget_preserves_tone_continuity_and_latest_target_readiness() {
    let mut e = FrequencyDelay::default();
    e.prepare(48000.0, 137);
    let mut p = FrequencyDelayParams::default();
    e.set_params(&p);
    let mut previous = 0.0_f32;
    let mut worst = 0.0_f32;
    for step in 0..200 {
        for b in &mut p.bands {
            b.delay_ms = if step % 2 == 0 { 0.5 } else { 1.0 };
        }
        if step > 10 {
            e.set_params(&p);
        }
        let mut l: [f32; 137] = std::array::from_fn(|n| {
            (2.0 * std::f32::consts::PI * 1000.0 * (step * 137 + n) as f32 / 48000.0).sin() * 0.25
        });
        let mut r = l;
        e.process(&mut l, &mut r);
        for x in l {
            worst = worst.max((x - previous).abs());
            previous = x;
        }
    }
    assert!(worst < 0.04, "tone step {worst}");
    for _ in 0..30 {
        e.process(&mut [0.0; 137], &mut [0.0; 137]);
    }
    assert_eq!(e.latency_transition_samples_remaining(), 0);
    e.reset();
    e.set_params(&p);
    assert_eq!(e.warm_up_samples(), 48);
    let (mut l, mut r) = ([0.0; 100], [0.0; 100]);
    l[0] = 1.0;
    e.process(&mut l, &mut r);
    close(l[48], 1.0, 1e-5);
}
#[test]
fn conservative_tail_includes_old_taps_and_feedback_crossings() {
    let mut bank = EchoBank::default();
    bank.prepare(1.0, 128);
    let mut p = bank_params();
    p.units[0].time_ms = 24000.0;
    bank.set_params(&p);
    bank.process(&mut [1.0], &mut [0.0]);
    p.units[0].time_ms = 1.0;
    bank.set_params(&p);
    assert!(bank.tail_samples() >= 24);
    assert!(bank.gap_samples() >= 24);
    assert!(bank.delay_readiness_samples() >= 24);
    p.units[0].feedback = 1.0;
    bank.set_params(&p);
    assert_eq!(bank.tail_samples(), usize::MAX);
    p.units[0].feedback = 0.0;
    bank.set_params(&p);
    bank.process(&mut [0.0; 128], &mut [0.0; 128]);
    let tail = bank.tail_samples();
    assert_ne!(tail, usize::MAX);
    let (mut l, mut r) = (vec![0.0; tail + 32], vec![0.0; tail + 32]);
    bank.process(&mut l, &mut r);
    assert!(l[tail..].iter().chain(&r[tail..]).all(|x| x.abs() < 1e-6));
    let mut freq = FrequencyDelay::default();
    freq.prepare(1.0, 128);
    let q = FrequencyDelayParams {
        feedback: 1.0,
        ..Default::default()
    };
    freq.set_params(&q);
    assert_eq!(freq.tail_samples(), usize::MAX);
    freq.set_params(&FrequencyDelayParams::default());
    freq.process(&mut [0.0; 128], &mut [0.0; 128]);
    assert_ne!(freq.tail_samples(), usize::MAX);
}

#[test]
fn every_bank_filter_response_order_and_extreme_loop_combination_stays_finite() {
    let modes = [
        EchoFilterMode::Off,
        EchoFilterMode::Lowpass,
        EchoFilterMode::Bandpass,
        EchoFilterMode::Notch,
        EchoFilterMode::Highpass,
        EchoFilterMode::LowShelf,
        EchoFilterMode::Peak,
        EchoFilterMode::HighShelf,
    ];
    for rate in [1.0, 8000.0, 48000.0, 384000.0] {
        let mut bank = EchoBank::default();
        bank.prepare(rate, 128);
        for mode in modes {
            for sections in 1..=3 {
                for high in [false, true] {
                    let mut p = bank_params();
                    for u in &mut p.units {
                        u.enabled = true;
                        u.time_ms = 1.0;
                        u.feedback = 1.0;
                        u.next_send = -1.0;
                        u.filter = EchoFilterParams {
                            mode,
                            sections,
                            frequency_hz: if high { 20000.0 } else { 20.0 },
                            q: if high { 10.0 } else { 0.5 },
                            gain_db: if high { 18.0 } else { -18.0 },
                            gain: if high { 2.0 } else { 0.1 },
                        };
                        u.feedback_filter = u.filter;
                        u.filter_post = high;
                    }
                    bank.reset();
                    bank.set_params(&p);
                    watched(|| {
                        for block in 0..16 {
                            let mut l: [f32; 128] = std::array::from_fn(|n| {
                                if (block * 128 + n) % 7 < 3 {
                                    1000.0
                                } else {
                                    -1000.0
                                }
                            });
                            let mut r = l.map(|x| -x);
                            bank.process(&mut l, &mut r);
                            assert!(
                                l.iter()
                                    .chain(&r)
                                    .all(|x| x.is_finite() && x.abs() <= 1512.0)
                            );
                        }
                    });
                }
            }
        }
    }
}
#[test]
fn bank_filter_lowpass_cascade_matches_analytic_transfer_and_changes_echoes() {
    let rate = 48000.0;
    let hz = 3000.0;
    let len = 16384;
    let mut bank = EchoBank::default();
    bank.prepare(rate as f32, 137);
    let z = Complex {
        re: (-2.0 * PI * hz / rate).cos(),
        im: (-2.0 * PI * hz / rate).sin(),
    };
    let one = Complex { re: 1.0, im: 0.0 };
    let plus = one.add(z);
    let minus = one.sub(z);
    let g = (PI * 1000.0 / rate).tan();
    let k = 2.0_f64.sqrt();
    // Bilinear transform of 1/(s^2+k*s+1).
    let h = plus.mul(plus).scale(g * g).div(
        minus
            .mul(minus)
            .add(minus.mul(plus).scale(k * g))
            .add(plus.mul(plus).scale(g * g)),
    );
    let phase = -2.0 * PI * hz * 48.0 / rate;
    for sections in 1..=3 {
        for post in [false, true] {
            let mut p = bank_params();
            p.units[0].time_ms = 1.0;
            p.units[0].filter.mode = EchoFilterMode::Lowpass;
            p.units[0].filter.sections = sections;
            p.units[0].filter_post = post;
            bank.reset();
            bank.set_params(&p);
            let mut l: Vec<f32> = (0..len)
                .map(|n| (0.1 * (2.0 * PI * hz * n as f64 / rate).cos()) as f32)
                .collect();
            let mut r = vec![0.0; len];
            run(&mut bank, &mut l, &mut r, &[137, 1]);
            let mut expected = Complex {
                re: phase.cos(),
                im: phase.sin(),
            };
            for _ in 0..sections {
                expected = expected.mul(h);
            }
            let mut measured = Complex { re: 0.0, im: 0.0 };
            for (n, x) in l.iter().enumerate().skip(8192) {
                let angle = -2.0 * PI * hz * n as f64 / rate;
                measured = measured.add(
                    Complex {
                        re: angle.cos(),
                        im: angle.sin(),
                    }
                    .scale(f64::from(*x) * 20.0 / 8192.0),
                );
            }
            assert!(
                (measured.re - expected.re).abs() < 3e-4
                    && (measured.im - expected.im).abs() < 3e-4
            );
            assert!(r.iter().all(|x| *x == 0.0));
        }
    }
}
#[test]
fn deselected_frequency_band_passes_unaffected_and_wakes_with_live_history() {
    let mut e = FrequencyDelay::default();
    e.prepare(8000.0, 137);
    let mut p = FrequencyDelayParams::default();
    for b in &mut p.bands {
        b.enabled = false;
        b.level = 0.0;
        b.delay_ms = 1000.0;
        b.pan = 1.0;
    }
    e.set_params(&p);
    for _ in 0..70 {
        let (mut l, mut r) = ([0.25; 137], [-0.5; 137]);
        e.process(&mut l, &mut r);
        for n in 0..137 {
            close(l[n], 0.25, 1e-6);
            close(r[n], -0.5, 1e-6);
        }
    }
    for b in &mut p.bands {
        b.enabled = true;
        b.level = 1.0;
        b.pan = 0.0;
    }
    e.set_params(&p);
    let (mut l, mut r) = ([0.25; 137], [-0.5; 137]);
    e.process(&mut l, &mut r);
    for n in 0..137 {
        let t = ((n + 1) as f64 / 80.0).min(1.0);
        // Enable, level and pan each have their own 80-frame signal ramp.
        close(l[n], 0.25 * (1.0 - t + t * t * t), 1e-5);
        close(r[n], -0.5 * (1.0 - t + t * t), 1e-5);
    }
}
#[test]
fn rapid_bank_delay_feedback_wet_edits_keep_history_and_reset_latest_target() {
    let mut e = EchoBank::default();
    e.prepare(48000.0, 137);
    let mut p = bank_params();
    p.units[0].time_ms = 1.0;
    e.set_params(&p);
    let mut previous = 0.0_f32;
    let mut worst = 0.0_f32;
    for step in 0..200 {
        if step > 10 {
            p.units[0].time_ms = if step % 2 == 0 { 2.0 } else { 1.0 };
            p.units[0].stereo_offset_ms = if step % 3 == 0 { 1.0 } else { 0.0 };
            p.units[0].feedback = if step % 2 == 0 { 0.0 } else { 0.5 };
            p.wet = if step % 3 == 0 { 0.5 } else { 1.0 };
            e.set_params(&p);
        }
        let mut l: [f32; 137] = std::array::from_fn(|n| {
            (2.0 * std::f32::consts::PI * 500.0 * (step * 137 + n) as f32 / 48000.0).sin() * 0.1
        });
        let mut r = l;
        e.process(&mut l, &mut r);
        if step > 10 {
            for x in l {
                worst = worst.max((x - previous).abs());
                previous = x;
            }
        } else {
            previous = l[136];
        }
    }
    assert!(worst < 0.02, "bank step {worst}");
    p = bank_params();
    p.units[0].time_ms = 3.0;
    p.units[0].stereo_offset_ms = 2.0;
    e.set_params(&p);
    e.reset();
    let (mut l, mut r) = ([0.0; 300], [0.0; 300]);
    l[0] = 1.0;
    r[0] = 0.25;
    e.process(&mut l, &mut r);
    assert_eq!(l[144], 1.0);
    assert_eq!(r[240], 0.25);
}
#[test]
fn short_history_wait_is_causal_and_old_tap_bounds_survive_a_pending_edit() {
    let mut e = FrequencyDelay::default();
    e.prepare(8000.0, 137);
    let mut p = FrequencyDelayParams::default();
    e.set_params(&p);
    e.process(&mut [1.0; 8], &mut [1.0; 8]);
    for b in &mut p.bands {
        b.delay_ms = 1000.0;
    }
    e.set_params(&p);
    assert_eq!(e.delay_readiness_samples(), 8000);
    assert!(e.latency_transition_samples_remaining() >= 7992);
    let (mut l, mut r) = (vec![1.0; 8400], vec![1.0; 8400]);
    run(&mut e, &mut l, &mut r, &[1, 137, 7]);
    for x in l.iter().chain(&r) {
        close(*x, 1.0, 1e-5);
    }
    assert_eq!(e.latency_transition_samples_remaining(), 0);
    for b in &mut p.bands {
        b.delay_ms = 0.0;
    }
    e.set_params(&p);
    assert_eq!(e.delay_readiness_samples(), 8000);
    e.process(&mut [1.0; 400], &mut [1.0; 400]);
    assert_eq!(e.delay_readiness_samples(), 0);
}

#[test]
fn bank_feedback_filter_recurrence_is_independent_and_first_echo_is_unfiltered() {
    let mut bank = EchoBank::default();
    bank.prepare(8000.0, 137);
    let mut p = bank_params();
    p.units[0].time_ms = 5.0;
    p.units[0].feedback = 0.5;
    p.units[0].feedback_filter.mode = EchoFilterMode::Lowpass;
    bank.set_params(&p);
    let (mut l, mut r) = (vec![0.0; 1024], vec![0.0; 1024]);
    l[0] = 1.0;
    run(&mut bank, &mut l, &mut r, &[1, 137, 7]);
    assert_eq!(l[40], 1.0);
    let g = (PI * 1000.0 / 8000.0).tan();
    let k = 2.0_f64.sqrt();
    let d = 1.0 + k * g + g * g;
    let b = [g * g / d, 2.0 * g * g / d, g * g / d];
    let a = [2.0 * (g * g - 1.0) / d, (1.0 - k * g + g * g) / d];
    let mut expected = vec![0.0; 1024];
    let mut filtered = vec![0.0; 1024];
    for n in 0..1024 {
        expected[n] =
            if n == 40 { 1.0 } else { 0.0 } + if n >= 40 { 0.5 * filtered[n - 40] } else { 0.0 };
        filtered[n] = b[0] * expected[n]
            + if n > 0 {
                b[1] * expected[n - 1] - a[0] * filtered[n - 1]
            } else {
                0.0
            }
            + if n > 1 {
                b[2] * expected[n - 2] - a[1] * filtered[n - 2]
            } else {
                0.0
            };
        close(l[n], expected[n], 2e-5);
    }
    assert!(r.iter().all(|x| *x == 0.0));
}
#[test]
fn bank_input_output_pan_separation_and_signed_level_have_real_transfers() {
    let mut bank = EchoBank::default();
    bank.prepare(8000.0, 137);
    for pan in [-1.0_f32, 0.0, 1.0] {
        for separation in [0.0, 1.0] {
            let mut p = bank_params();
            p.units[0].input_pan = pan;
            p.units[0].output_pan = -pan;
            p.units[0].input_gain = 2.0;
            p.units[0].output_gain = -0.5;
            p.units[0].separation = separation;
            bank.reset();
            bank.set_params(&p);
            let (mut l, mut r) = ([0.0; 100], [0.0; 100]);
            l[0] = 0.5;
            r[0] = 0.25;
            bank.process(&mut l, &mut r);
            let il = 1.0 * (1.0 - pan.max(0.0));
            let ir = 0.5 * (1.0 + pan.min(0.0));
            let mid = (il + ir) * 0.5;
            let side = (il - ir) * 0.5 * separation;
            close(
                l[40],
                f64::from(-0.5 * (mid + side) * (1.0 - (-pan).max(0.0))),
                1e-6,
            );
            close(
                r[40],
                f64::from(-0.5 * (mid - side) * (1.0 + (-pan).min(0.0))),
                1e-6,
            );
        }
    }
}
#[test]
fn full_frequency_time_bound_recombines_at_actual_sample_boundaries_all_rates() {
    for rate in [1.0, 8000.0, 48000.0, 384000.0] {
        let mut e = FrequencyDelay::default();
        e.prepare(rate, 137);
        let mut p = FrequencyDelayParams::default();
        for b in &mut p.bands {
            b.delay_ms = 1000.0;
        }
        e.set_params(&p);
        let delay = rate as usize;
        let (mut l, mut r) = (vec![0.0; delay + 64], vec![0.0; delay + 64]);
        l[0] = 1.0;
        r[0] = -0.5;
        watched(|| run(&mut e, &mut l, &mut r, &[1, 137, 511]));
        for n in 0..l.len() {
            close(l[n], if n == delay { 1.0 } else { 0.0 }, 1e-5);
            close(r[n], if n == delay { -0.5 } else { 0.0 }, 1e-5);
        }
    }
}
#[test]
fn new_types_derive_ts_without_application_exports() {
    use ts_rs::TS;
    let config = ts_rs::Config::default();
    assert!(EchoBankParams::decl(&config).contains("units"));
    assert!(EchoUnitParams::decl(&config).contains("feedbackFilter"));
    assert!(FrequencyDelayParams::decl(&config).contains("bands"));
    assert!(FrequencyBandParams::decl(&config).contains("delayMs"));
}

#[test]
fn full_bank_time_and_offset_bound_emit_exact_impulses_at_all_actual_rates() {
    for rate in [1.0, 8000.0, 48000.0, 384000.0] {
        let mut e = EchoBank::default();
        e.prepare(rate, 137);
        let mut p = bank_params();
        p.units[0].sync = true;
        p.units[0].division = NoteDivision::Whole;
        p.units[0].stereo_offset_ms = 1000.0;
        e.set_tempo(10.0);
        e.set_params(&p);
        let left_delay = 24 * rate as usize;
        let right_delay = 25 * rate as usize;
        let (mut l, mut r) = (vec![0.0; right_delay + 32], vec![0.0; right_delay + 32]);
        l[0] = 1.0;
        r[0] = -0.5;
        watched(|| run(&mut e, &mut l, &mut r, &[137, 511]));
        for n in 0..l.len() {
            assert_eq!(l[n], if n == left_delay { 1.0 } else { 0.0 });
            assert_eq!(r[n], if n == right_delay { -0.5 } else { 0.0 });
        }
        p.units[0].sync = false;
        p.units[0].time_ms = 1.0;
        p.units[0].stereo_offset_ms = 0.0;
        e.set_params(&p);
        watched(|| e.reset());
        let (mut l, mut r) = ([0.0; 1024], [0.0; 1024]);
        l[0] = 0.25;
        r[0] = 0.5;
        watched(|| e.process(&mut l, &mut r));
        let delay = (rate / 1000.0).round().max(1.0) as usize;
        for n in 0..1024 {
            assert_eq!(l[n], if n == delay { 0.25 } else { 0.0 });
            assert_eq!(r[n], if n == delay { 0.5 } else { 0.0 });
        }
        println!("full bank delay/offset impulse accepted at rate={rate}");
    }
}

#[test]
#[ignore = "scoped release CPU observation; no audio-device guarantee"]
fn release_cpu_constant_and_automated_worst_case() {
    for rate in [8000.0, 48000.0, 384000.0] {
        let mut bank = EchoBank::default();
        bank.prepare(rate, 64);
        let mut p = bank_params();
        for u in &mut p.units {
            u.enabled = true;
            u.feedback = 1.0;
            u.time_ms = 1.0;
            u.next_send = 1.0;
            u.filter.mode = EchoFilterMode::Peak;
            u.filter.sections = 3;
            u.filter.q = 10.0;
            u.filter.gain_db = 18.0;
            u.feedback_filter = u.filter;
        }
        bank.set_params(&p);
        let mut freq = FrequencyDelay::default();
        freq.prepare(rate, 64);
        let mut q = FrequencyDelayParams {
            feedback: 1.0,
            split: FrequencySplit::Steep,
            ..Default::default()
        };
        for (i, b) in q.bands.iter_mut().enumerate() {
            b.delay_ms = (i + 1) as f32;
        }
        freq.set_params(&q);
        // Prime every 1/2-ms destination before timing active fades.
        for _ in 0..32 {
            bank.process(&mut [0.1; 64], &mut [-0.1; 64]);
            freq.process(&mut [0.1; 64], &mut [-0.1; 64]);
        }
        for automated in [false, true] {
            let start = Instant::now();
            watched(|| {
                for block in 0..256 {
                    if automated {
                        for u in &mut p.units {
                            u.time_ms = if block % 2 == 0 { 1.0 } else { 2.0 };
                            u.filter.frequency_hz = if block % 2 == 0 { 20.0 } else { 20000.0 };
                            u.filter.q = if block % 2 == 0 { 0.5 } else { 10.0 };
                        }
                        bank.set_params(&p);
                        bank.set_tempo(if block % 2 == 0 { 10.0 } else { 1000.0 });
                    }
                    bank.process(&mut [0.1; 64], &mut [-0.1; 64]);
                }
            });
            let bank_time = start.elapsed();
            let start = Instant::now();
            watched(|| {
                for block in 0..256 {
                    if automated {
                        q.bandwidth = if block % 2 == 0 { 0.5 } else { 2.0 };
                        for b in &mut q.bands {
                            b.delay_ms = if block % 2 == 0 { 1.0 } else { 2.0 };
                        }
                        freq.set_params(&q);
                    }
                    freq.process(&mut [0.1; 64], &mut [-0.1; 64]);
                }
            });
            println!(
                "CPU rate={rate} automated={automated} frames=16384 bank_us={} frequency_us={} bank_bytes={} frequency_bytes={} taps<=32/64",
                bank_time.as_micros(),
                start.elapsed().as_micros(),
                bank.prepared_bytes(),
                freq.prepared_bytes()
            );
        }
    }
}
