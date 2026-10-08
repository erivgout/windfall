//! Authored references: bilinear transforms of continuous transfer functions,
//! evaluated with double-precision direct-form history, not the product SVF.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::f64::consts::{PI, TAU};
use std::hint::black_box;
use std::time::Instant;

use windfall_dsp::filter_family::{
    BassShelf, BassShelfParams, FastLowpass, FastLowpassParams, SelectableFilter,
    SelectableFilterMode as Mode, SelectableFilterParams,
};
use windfall_dsp::{Effect, ParamSet};

// The same thread-local counting guard used by tests/dsp/realtime.rs. This
// auto-discovered binary is independent of the registry-owned test harness.
thread_local! {
    static WATCHING: Cell<bool> = const { Cell::new(false) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
}

struct CountingAllocator;

fn count() {
    if WATCHING.try_with(Cell::get).unwrap_or(false) {
        let _ = CALLS.try_with(|calls| calls.set(calls.get() + 1));
    }
}

// SAFETY: forward each allocation unchanged; bookkeeping uses plain TLS cells.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: caller upholds the allocator contract.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: caller upholds the allocator contract.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        count();
        // SAFETY: caller upholds the allocator contract.
        unsafe { System.dealloc(pointer, layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count();
        // SAFETY: caller upholds the allocator contract.
        unsafe { System.realloc(pointer, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn allocator_calls(work: impl FnOnce()) -> usize {
    struct Stop;
    impl Drop for Stop {
        fn drop(&mut self) {
            WATCHING.set(false);
        }
    }
    CALLS.set(0);
    WATCHING.set(true);
    let stop = Stop;
    work();
    drop(stop);
    CALLS.get()
}

#[derive(Clone, Copy)]
struct Reference {
    b: [f64; 3],
    a: [f64; 3],
    x: [f64; 2],
    y: [f64; 2],
}

impl Reference {
    fn new(b: [f64; 3], a: [f64; 3]) -> Self {
        Self {
            b: b.map(|v| v / a[0]),
            a: a.map(|v| v / a[0]),
            x: [0.0; 2],
            y: [0.0; 2],
        }
    }
    fn tick(&mut self, input: f32) -> f64 {
        let input = f64::from(input);
        let out = self.b[0] * input + self.b[1] * self.x[0] + self.b[2] * self.x[1]
            - self.a[1] * self.y[0]
            - self.a[2] * self.y[1];
        self.x = [input, self.x[0]];
        self.y = [out, self.y[0]];
        out
    }
    fn magnitude(&self, hz: f64, rate: f64) -> f64 {
        let omega = TAU * hz / rate;
        let power = |c: [f64; 3]| {
            let real = c[0] + c[1] * omega.cos() + c[2] * (2.0 * omega).cos();
            let imag = -c[1] * omega.sin() - c[2] * (2.0 * omega).sin();
            real * real + imag * imag
        };
        (power(self.b) / power(self.a)).sqrt()
    }
}

fn gain(hz: f64, rate: f64) -> f64 {
    (PI * (hz / rate).clamp(1.0e-5, 0.49)).tan()
}

// Numerator polynomials L=g²(1+z^-1)², B=g(1-z^-2),
// H=(1-z^-1)². Denominator H+kB+L. No product coefficient API.
fn response(mode: Mode, hz: f64, q: f64, db: f64, rate: f64) -> Reference {
    let mut g = gain(hz, rate);
    let mut k = 1.0 / q;
    let a = 10.0_f64.powf(db / 40.0);
    let (m0, m1, m2) = match mode {
        Mode::Lowpass => (0.0, 0.0, 1.0),
        Mode::Highpass => (1.0, -k, -1.0),
        Mode::Bandpass => (0.0, k, 0.0),
        Mode::Notch => (1.0, -k, 0.0),
        Mode::LowShelf => {
            g /= a.sqrt();
            (1.0, k * (a - 1.0), a * a - 1.0)
        }
        Mode::Peak => {
            k /= a;
            (1.0, k * (a * a - 1.0), 0.0)
        }
        Mode::HighShelf => {
            g *= a.sqrt();
            (a * a, k * (1.0 - a) * a, 1.0 - a * a)
        }
    };
    let d = [
        1.0 + k * g + g * g,
        2.0 * (g * g - 1.0),
        1.0 - k * g + g * g,
    ];
    let band = [g, 0.0, -g];
    let low = [g * g, 2.0 * g * g, g * g];
    Reference::new(
        std::array::from_fn(|i| m0 * d[i] + m1 * band[i] + m2 * low[i]),
        d,
    )
}

fn bass_response(hz: f64, db: f64, rate: f64) -> Reference {
    let boost = 10.0_f64.powf(db / 20.0);
    let g = gain(hz, rate) / boost.sqrt();
    Reference::new(
        [1.0 + boost * g, boost * g - 1.0, 0.0],
        [1.0 + g, g - 1.0, 0.0],
    )
}

fn prepared<E: Effect + Default>(params: E::Params, rate: f32) -> E {
    let mut effect = E::default();
    effect.prepare(rate, 512);
    effect.set_params(&params);
    effect
}

fn noise(seed: u32, n: usize) -> Vec<f32> {
    let mut state = seed;
    (0..n)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state as f64 / u32::MAX as f64 * 2.0 - 1.0) as f32 * 0.1
        })
        .collect()
}

fn impulse<E: Effect>(mut effect: E, n: usize) -> Vec<f32> {
    let mut left = vec![0.0; n];
    let mut right = vec![0.0; n];
    left[0] = 1.0;
    effect.process(&mut left, &mut right);
    assert!(right.iter().all(|&v| v == 0.0), "stereo crossfeed");
    left
}

fn check_impulse<E: Effect>(effect: E, mut reference: Reference, tolerance: f64) {
    let samples = impulse(effect, 16_384);
    let mut maximum = 0.0_f64;
    for (n, actual) in samples.iter().enumerate() {
        let expected = reference.tick(if n == 0 { 1.0 } else { 0.0 });
        maximum = maximum.max((f64::from(*actual) - expected).abs());
        assert!(actual.is_finite());
    }
    assert!(
        maximum < tolerance,
        "impulse maximum error {maximum}, limit {tolerance}"
    );
}

#[test]
fn independent_impulse_references_cover_every_mode_and_filter_path() {
    for rate in [8_000.0_f32, 44_100.0, 48_000.0, 96_000.0, 384_000.0] {
        for hz in [20.0_f32, 1_000.0, 20_000.0] {
            for q in [0.5, std::f32::consts::FRAC_1_SQRT_2, 10.0] {
                check_impulse(
                    prepared::<FastLowpass>(FastLowpassParams { cutoff_hz: hz, q }, rate),
                    response(Mode::Lowpass, hz.into(), q.into(), 0.0, rate.into()),
                    0.000_1,
                );
                for mode in Mode::ALL {
                    for db in [-18.0_f32, 0.0, 18.0] {
                        check_impulse(
                            prepared::<SelectableFilter>(
                                SelectableFilterParams {
                                    mode,
                                    frequency_hz: hz,
                                    q,
                                    gain_db: db,
                                },
                                rate,
                            ),
                            response(mode, hz.into(), q.into(), db.into(), rate.into()),
                            0.002,
                        );
                    }
                }
            }
        }
        for hz in [40.0_f32, 150.0, 1_000.0] {
            for db in [0.0_f32, 6.0, 18.0] {
                check_impulse(
                    prepared::<BassShelf>(
                        BassShelfParams {
                            frequency_hz: hz,
                            gain_db: db,
                        },
                        rate,
                    ),
                    bass_response(hz.into(), db.into(), rate.into()),
                    0.000_1,
                );
            }
        }
    }
}

fn check_sweep<E: Effect>(
    mut effect: E,
    mut left_reference: Reference,
    mut right_reference: Reference,
) {
    let rate = 48_000.0;
    let count = 48_000;
    let mut phase = 0.0;
    let mut left: Vec<f32> = (0..count)
        .map(|n| {
            let hz = 20.0 * (20_000.0_f64 / 20.0).powf(n as f64 / (count - 1) as f64);
            phase += TAU * hz / rate;
            phase.sin() as f32 * 0.01
        })
        .collect();
    let mut right = noise(0x79251, count);
    let expected_l: Vec<f64> = left
        .iter()
        .map(|&sample| left_reference.tick(sample))
        .collect();
    let expected_r: Vec<f64> = right
        .iter()
        .map(|&sample| right_reference.tick(sample))
        .collect();
    effect.process(&mut left, &mut right);
    for (actual, expected) in left
        .iter()
        .zip(&expected_l)
        .chain(right.iter().zip(&expected_r))
    {
        assert!(
            (f64::from(*actual) - expected).abs() < 0.000_05,
            "sweep/reference {actual}/{expected}"
        );
    }
}

#[test]
fn logarithmic_sweeps_and_independent_stereo_noise_match_reference_histories() {
    for mode in Mode::ALL {
        let params = SelectableFilterParams {
            mode,
            frequency_hz: 1_000.0,
            q: 4.0,
            gain_db: 12.0,
        };
        let reference = response(mode, 1_000.0, 4.0, 12.0, 48_000.0);
        check_sweep(
            prepared::<SelectableFilter>(params, 48_000.0),
            reference,
            reference,
        );
    }
    let reference = response(Mode::Lowpass, 1_000.0, 10.0, 0.0, 48_000.0);
    check_sweep(
        prepared::<FastLowpass>(
            FastLowpassParams {
                cutoff_hz: 1_000.0,
                q: 10.0,
            },
            48_000.0,
        ),
        reference,
        reference,
    );
    let reference = bass_response(150.0, 18.0, 48_000.0);
    check_sweep(
        prepared::<BassShelf>(
            BassShelfParams {
                frequency_hz: 150.0,
                gain_db: 18.0,
            },
            48_000.0,
        ),
        reference,
        reference,
    );
}

fn tail_check<E: Effect>(mut effect: E) {
    let (mut left, mut right) = ([1.0], [-0.5]);
    effect.process(&mut left, &mut right);
    let frames = effect.tail_samples();
    assert!(frames > 0 && frames < 25_000_000, "tail {frames}");
    let mut loudest = 0.0_f32;
    let mut n = 0;
    while n < frames + 512 {
        let count = 127.min(frames + 512 - n);
        let (mut l, mut r) = ([0.0; 127], [0.0; 127]);
        effect.process(&mut l[..count], &mut r[..count]);
        for i in 0..count {
            assert!(l[i].is_finite() && r[i].is_finite());
            if n + i >= frames {
                loudest = loudest.max(l[i].abs()).max(r[i].abs());
            }
        }
        n += count;
    }
    assert!(loudest < 1.0e-18, "residual after reported tail {loudest}");
}

#[test]
fn reported_tails_cover_low_cutoff_high_resonance_and_nyquist_histories() {
    for rate in [1.0_f32, 48_000.0, 384_000.0] {
        for hz in [20.0, 20_000.0] {
            tail_check(prepared::<FastLowpass>(
                FastLowpassParams {
                    cutoff_hz: hz,
                    q: 10.0,
                },
                rate,
            ));
        }
        for mode in [Mode::Lowpass, Mode::Peak, Mode::LowShelf, Mode::HighShelf] {
            tail_check(prepared::<SelectableFilter>(
                SelectableFilterParams {
                    mode,
                    frequency_hz: 20.0,
                    q: 10.0,
                    gain_db: 18.0,
                },
                rate,
            ));
        }
        tail_check(prepared::<BassShelf>(
            BassShelfParams {
                frequency_hz: 40.0,
                gain_db: 18.0,
            },
            rate,
        ));
    }
}

#[test]
fn shelf_and_peak_zero_gain_settle_to_exact_unity_after_active_history() {
    for mode in [Mode::LowShelf, Mode::Peak, Mode::HighShelf] {
        let mut effect = prepared::<SelectableFilter>(
            SelectableFilterParams {
                mode,
                frequency_hz: 20.0,
                q: 10.0,
                gain_db: 18.0,
            },
            48_000.0,
        );
        let (mut l, mut r) = (noise(11, 2000), noise(18, 2000));
        effect.process(&mut l, &mut r);
        effect.set_params(&SelectableFilterParams {
            mode,
            frequency_hz: 20_000.0,
            q: 0.5,
            gain_db: 0.0,
        });
        let input_l = noise(43, 2000);
        let input_r = noise(17, 2000);
        let (mut l, mut r) = (input_l.clone(), input_r.clone());
        effect.process(&mut l, &mut r);
        assert_eq!(l[239..], input_l[239..]);
        assert_eq!(r[239..], input_r[239..]);
    }
}

fn dft(samples: &[f32], hz: f64, rate: f64) -> f64 {
    let (mut real, mut imag) = (0.0, 0.0);
    for (n, &sample) in samples.iter().enumerate() {
        let phase = TAU * hz * n as f64 / rate;
        real += f64::from(sample) * phase.cos();
        imag -= f64::from(sample) * phase.sin();
    }
    real.hypot(imag)
}

#[test]
fn impulse_dft_matches_independent_transfer_over_the_spectrum() {
    let rate = 48_000.0;
    for mode in Mode::ALL {
        let params = SelectableFilterParams {
            mode,
            frequency_hz: 1_000.0,
            q: 2.0,
            gain_db: 12.0,
        };
        let output = impulse(prepared::<SelectableFilter>(params, rate), 32_768);
        let expected = response(mode, 1_000.0, 2.0, 12.0, rate.into());
        for hz in [
            0.0, 20.0, 100.0, 500.0, 1_000.0, 2_000.0, 8_000.0, 20_000.0, 24_000.0,
        ] {
            let measured = dft(&output, hz, rate.into());
            let reference = expected.magnitude(hz, rate.into());
            assert!(
                (measured - reference).abs() < 0.000_3,
                "{mode:?}/{hz}: {measured}/{reference}"
            );
        }
    }
    let output = impulse(
        prepared::<FastLowpass>(
            FastLowpassParams {
                cutoff_hz: 500.0,
                ..Default::default()
            },
            rate,
        ),
        32_768,
    );
    let slope =
        20.0 * (dft(&output, 4_000.0, rate.into()) / dft(&output, 2_000.0, rate.into())).log10();
    assert!((-13.0..-11.5).contains(&slope), "rolloff {slope} dB/octave");
    let output = impulse(
        prepared::<BassShelf>(
            BassShelfParams {
                frequency_hz: 150.0,
                gain_db: 18.0,
            },
            rate,
        ),
        32_768,
    );
    for hz in [0.0, 20.0, 150.0, 1_000.0, 10_000.0, 24_000.0] {
        let measured = dft(&output, hz, rate.into());
        let expected = bass_response(150.0, 18.0, rate.into()).magnitude(hz, rate.into());
        assert!(
            (measured - expected).abs() < 0.000_3,
            "bass/{hz}: {measured}/{expected}"
        );
    }
}

fn tone_gain<E: Effect>(mut effect: E, hz: f64, rate: f64) -> f64 {
    let count = rate as usize * 2;
    let input: Vec<f32> = (0..count)
        .map(|n| (TAU * hz * n as f64 / rate).sin() as f32 * 0.01)
        .collect();
    let mut left = input.clone();
    let mut right = vec![0.0; count];
    effect.process(&mut left, &mut right);
    let input_power: f64 = input[count / 2..]
        .iter()
        .map(|&v| f64::from(v).powi(2))
        .sum();
    let output_power: f64 = left[count / 2..]
        .iter()
        .map(|&v| f64::from(v).powi(2))
        .sum();
    (output_power / input_power).sqrt()
}

#[test]
fn cutoff_resonance_unity_bandpass_and_bass_headroom_are_measured() {
    for rate in [44_100.0_f32, 48_000.0, 96_000.0] {
        for q in [0.5, std::f32::consts::FRAC_1_SQRT_2, 10.0] {
            let value = tone_gain(
                prepared::<FastLowpass>(
                    FastLowpassParams {
                        cutoff_hz: 1_000.0,
                        q,
                    },
                    rate,
                ),
                1_000.0,
                rate.into(),
            );
            assert!((value - f64::from(q)).abs() < 0.001, "{rate}/{q}: {value}");
            let value = tone_gain(
                prepared::<SelectableFilter>(
                    SelectableFilterParams {
                        mode: Mode::Bandpass,
                        q,
                        ..Default::default()
                    },
                    rate,
                ),
                1_000.0,
                rate.into(),
            );
            assert!((value - 1.0).abs() < 0.0001);
        }
    }
    let bass = |hz| {
        tone_gain(
            prepared::<BassShelf>(
                BassShelfParams {
                    frequency_hz: 150.0,
                    gain_db: 18.0,
                },
                48_000.0,
            ),
            hz,
            48_000.0,
        )
    };
    assert!(20.0 * bass(5.0).log10() > 17.9);
    assert!((20.0 * bass(150.0).log10() - 9.0).abs() < 0.001);
    assert!(20.0 * bass(12_000.0).log10() < 0.004);
    // No output limiter conceals the boost.
    let mut effect = prepared::<BassShelf>(
        BassShelfParams {
            gain_db: 18.0,
            ..Default::default()
        },
        48_000.0,
    );
    let mut left = vec![0.25; 48_000];
    let mut right = vec![-0.25; 48_000];
    effect.process(&mut left, &mut right);
    assert!((left[47_999] - 0.25 * 10.0_f32.powf(18.0 / 20.0)).abs() < 0.0001);
    assert!(left[47_999] > 1.9 && right[47_999] < -1.9);
}

fn partition_check<E: Effect + Default>(settings: &[E::Params]) {
    let mut whole = prepared::<E>(settings[0], 48_000.0);
    let mut split = prepared::<E>(settings[0], 48_000.0);
    let input_l = noise(19, 1003);
    let input_r = noise(137, 1003);
    for (step, params) in settings.iter().cycle().take(60).enumerate() {
        whole.set_params(params);
        split.set_params(params);
        let (mut wl, mut wr) = (input_l.clone(), input_r.clone());
        let (mut sl, mut sr) = (input_l.clone(), input_r.clone());
        whole.process(&mut wl, &mut wr);
        let mut n = 0;
        while n < sl.len() {
            let length = [1, 7, 63, 64, 129, 251][(n + step) % 6].min(sl.len() - n);
            // Duplicate automation writes, empty blocks and irrelevant tempo
            // notifications must not restart or advance an ongoing ramp.
            split.set_params(params);
            split.set_tempo(127.0);
            split.process(&mut [], &mut []);
            split.process(&mut sl[n..n + length], &mut sr[n..n + length]);
            n += length;
        }
        assert_eq!(wl, sl, "left partition {step}");
        assert_eq!(wr, sr, "right partition {step}");
    }
}

#[test]
fn automation_noops_and_flush_are_frame_counted_across_arbitrary_blocks() {
    partition_check::<FastLowpass>(&[
        FastLowpassParams {
            cutoff_hz: 20.0,
            q: 10.0,
        },
        FastLowpassParams {
            cutoff_hz: 20_000.0,
            q: 0.5,
        },
        FastLowpassParams::default(),
    ]);
    let settings: Vec<_> = Mode::ALL
        .into_iter()
        .enumerate()
        .map(|(i, mode)| SelectableFilterParams {
            mode,
            frequency_hz: if i % 2 == 0 { 20.0 } else { 20_000.0 },
            q: if i % 2 == 0 { 10.0 } else { 0.5 },
            gain_db: if i % 2 == 0 { -18.0 } else { 18.0 },
        })
        .collect();
    partition_check::<SelectableFilter>(&settings);
    partition_check::<BassShelf>(&[
        BassShelfParams::default(),
        BassShelfParams {
            frequency_hz: 40.0,
            gain_db: 18.0,
        },
        BassShelfParams {
            frequency_hz: 1_000.0,
            gain_db: 6.0,
        },
    ]);
}

#[test]
fn mode_edits_crossfade_live_responses_and_retarget_without_a_dry_gap() {
    let mut changing = prepared::<SelectableFilter>(SelectableFilterParams::default(), 48_000.0);
    let mut references: Vec<_> = Mode::ALL
        .map(|mode| {
            prepared::<SelectableFilter>(
                SelectableFilterParams {
                    mode,
                    ..Default::default()
                },
                48_000.0,
            )
        })
        .into_iter()
        .collect();
    let input = noise(0x5931, 2000);
    let mut weights = [0.0_f64; 7];
    weights[0] = 1.0;
    let mut step = [0.0; 7];
    let mut target = 0;
    let mut remaining = 0;
    for (n, x) in input.into_iter().enumerate() {
        if [300, 350, 501, 800, 850, 1100, 1500].contains(&n) {
            target = (target + 1) % 7;
            changing.set_params(&SelectableFilterParams {
                mode: Mode::ALL[target],
                ..Default::default()
            });
            step =
                std::array::from_fn(|i| (if i == target { 1.0 } else { 0.0 } - weights[i]) / 240.0);
            remaining = 240;
        }
        if remaining > 0 {
            remaining -= 1;
            weights = if remaining == 0 {
                std::array::from_fn(|i| if i == target { 1.0 } else { 0.0 })
            } else {
                std::array::from_fn(|i| weights[i] + step[i])
            };
        }
        let (mut left, mut right) = ([x], [0.0]);
        changing.process(&mut left, &mut right);
        let mut expected = 0.0;
        for (i, reference) in references.iter_mut().enumerate() {
            let (mut l, mut r) = ([x], [0.0]);
            reference.process(&mut l, &mut r);
            expected += f64::from(l[0]) * weights[i];
        }
        assert!(
            (f64::from(left[0]) - expected).abs() < 0.000_002,
            "frame {n}"
        );
    }
}

fn reset_check<E: Effect + Default>(params: E::Params) {
    let mut effect = prepared::<E>(E::Params::default(), 48_000.0);
    let (mut l, mut r) = (noise(4, 1000), noise(9, 1000));
    effect.process(&mut l, &mut r);
    effect.set_params(&params);
    effect.reset();
    let mut fresh = prepared::<E>(params, 48_000.0);
    let (mut l, mut r) = (noise(19, 1000), noise(21, 1000));
    let (mut fl, mut fr) = (l.clone(), r.clone());
    effect.process(&mut l, &mut r);
    fresh.process(&mut fl, &mut fr);
    assert_eq!(l, fl);
    assert_eq!(r, fr);
    effect.reset();
    let (mut l, mut r) = ([0.0; 1000], [0.0; 1000]);
    effect.process(&mut l, &mut r);
    assert_eq!(l, [0.0; 1000]);
    assert_eq!(r, [0.0; 1000]);
    assert_eq!(effect.latency_samples(), 0);
    assert_eq!(effect.warm_up_samples(), 0);
    assert!(effect.tail_samples() >= effect.gap_samples());
}

#[test]
fn reset_clears_histories_and_snaps_latest_settings_without_startup_glides() {
    reset_check::<FastLowpass>(FastLowpassParams {
        cutoff_hz: 20.0,
        q: 10.0,
    });
    for mode in Mode::ALL {
        reset_check::<SelectableFilter>(SelectableFilterParams {
            mode,
            frequency_hz: 20.0,
            q: 10.0,
            gain_db: 18.0,
        });
    }
    reset_check::<BassShelf>(BassShelfParams {
        frequency_hz: 40.0,
        gain_db: 18.0,
    });
}

#[test]
fn zero_boost_is_exact_unity_and_live_bass_history_is_available_after_raise() {
    let input = noise(938, 10_000);
    let mut effect = prepared::<BassShelf>(BassShelfParams::default(), 48_000.0);
    let (mut left, mut right) = (input.clone(), input.clone());
    effect.process(&mut left, &mut right);
    assert_eq!(left, input);
    assert_eq!(right, input);
    // Fill the hidden lowpass with DC at unity before raising boost. At the
    // first ramp frame its history must already contain the current input.
    let (mut left, mut right) = ([0.1; 48_000], [-0.1; 48_000]);
    effect.process(&mut left, &mut right);
    effect.set_params(&BassShelfParams {
        gain_db: 18.0,
        ..Default::default()
    });
    let (mut left, mut right) = ([0.1], [-0.1]);
    effect.process(&mut left, &mut right);
    let expected = 0.1 * (1.0 + (10.0_f32.powf(18.0 / 20.0) - 1.0) / 240.0);
    assert!((left[0] - expected).abs() < 0.000_001);
    assert_eq!(left[0], -right[0]);
}

fn parameter_contract<P: ParamSet + serde::Serialize + serde::de::DeserializeOwned>() {
    let default = P::default();
    assert_eq!(default.sanitized(), default);
    let json = serde_json::to_value(default).unwrap();
    for (i, descriptor) in P::descriptors().iter().enumerate() {
        assert_eq!(default.get(i), Some(descriptor.default));
        assert_eq!(P::index_of(descriptor.id), Some(i));
        assert!(
            json.get(descriptor.id).is_some(),
            "missing {}",
            descriptor.id
        );
        for value in [
            f32::MIN,
            f32::MAX,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ] {
            let mut edited = default;
            assert!(edited.set(i, value));
            let actual = edited.get(i).unwrap();
            assert!(actual.is_finite() && actual >= descriptor.min && actual <= descriptor.max);
        }
    }
    let mut params = default;
    assert!(!params.set(P::descriptors().len(), 1.0));
    assert_eq!(params.get(P::descriptors().len()), None);
    assert_eq!(serde_json::from_value::<P>(json).unwrap(), default);
    assert_eq!(serde_json::from_str::<P>("{}").unwrap(), default);
}

#[test]
fn descriptors_sanitization_serialization_and_automation_indices_are_stable() {
    parameter_contract::<FastLowpassParams>();
    parameter_contract::<SelectableFilterParams>();
    parameter_contract::<BassShelfParams>();
    for (index, mode) in Mode::ALL.into_iter().enumerate() {
        let mut params = SelectableFilterParams::default();
        params.set(0, index as f32);
        assert_eq!(params.mode, mode);
        assert_eq!(
            serde_json::to_value(mode).unwrap(),
            SelectableFilterParams::descriptors()[0].choices[index].value
        );
    }
    assert_eq!(
        FastLowpassParams {
            cutoff_hz: f32::NAN,
            q: f32::INFINITY
        }
        .sanitized(),
        FastLowpassParams::default()
    );
    assert_eq!(
        BassShelfParams {
            frequency_hz: f32::NAN,
            gain_db: f32::NEG_INFINITY
        }
        .sanitized(),
        BassShelfParams::default()
    );
}

fn stress<E: Effect + Default>(settings: &[E::Params], rate: f32) {
    let mut effect = prepared::<E>(settings[0], rate);
    let (mut left, mut right) = ([0.0; 127], [0.0; 127]);
    for step in 0..500 {
        effect.set_params(&settings[step % settings.len()]);
        for n in 0..127 {
            left[n] = ((n * 37 + step * 11) % 257) as f32 / 257.0 * 0.02 - 0.01;
            right[n] = if step % 3 == 0 { 0.0 } else { -left[n] };
        }
        effect.process(&mut left, &mut right);
        for sample in left.iter().chain(&right) {
            assert!(sample.is_finite(), "nonfinite at rate {rate}, step {step}");
            assert!(
                sample.abs() < 100.0,
                "unstable {sample} at rate {rate}, step {step}"
            );
        }
    }
}

#[test]
fn tiny_invalid_and_normal_rates_and_all_control_extrema_stay_finite() {
    let settings: Vec<_> = Mode::ALL
        .into_iter()
        .flat_map(|mode| {
            [20.0, 20_000.0].into_iter().flat_map(move |hz| {
                [0.5, 10.0].into_iter().flat_map(move |q| {
                    [-18.0, 18.0]
                        .into_iter()
                        .map(move |db| SelectableFilterParams {
                            mode,
                            frequency_hz: hz,
                            q,
                            gain_db: db,
                        })
                })
            })
        })
        .collect();
    for rate in [
        f32::NAN,
        f32::INFINITY,
        -1.0,
        0.0,
        1.0,
        2.0,
        8.0,
        1_000.0,
        8_000.0,
        44_100.0,
        48_000.0,
        96_000.0,
        384_000.0,
        f32::MAX,
    ] {
        stress::<SelectableFilter>(&settings, rate);
        stress::<FastLowpass>(
            &[
                FastLowpassParams {
                    cutoff_hz: 20.0,
                    q: 10.0,
                },
                FastLowpassParams {
                    cutoff_hz: 20_000.0,
                    q: 0.5,
                },
            ],
            rate,
        );
        stress::<BassShelf>(
            &[
                BassShelfParams {
                    frequency_hz: 40.0,
                    gain_db: 18.0,
                },
                BassShelfParams {
                    frequency_hz: 1_000.0,
                    gain_db: 0.0,
                },
            ],
            rate,
        );
    }
}

#[test]
fn guarded_callbacks_never_allocate_reallocate_or_free() {
    let mut fast = prepared::<FastLowpass>(FastLowpassParams::default(), 48_000.0);
    let mut selectable = prepared::<SelectableFilter>(SelectableFilterParams::default(), 48_000.0);
    let mut bass = prepared::<BassShelf>(BassShelfParams::default(), 48_000.0);
    let (mut left, mut right) = ([0.1; 512], [-0.1; 512]);
    let calls = allocator_calls(|| {
        for n in 0..1000 {
            fast.set_params(&FastLowpassParams {
                cutoff_hz: if n % 2 == 0 { 20.0 } else { 20_000.0 },
                q: 10.0,
            });
            selectable.set_params(&SelectableFilterParams {
                mode: Mode::ALL[n % 7],
                frequency_hz: 20.0,
                q: 10.0,
                gain_db: 18.0,
            });
            bass.set_params(&BassShelfParams {
                frequency_hz: 40.0,
                gain_db: (n % 19) as f32,
            });
            let count = [1, 7, 64, 127, 512][n % 5];
            fast.process(&mut left[..count], &mut right[..count]);
            selectable.process(&mut left[..count], &mut right[..count]);
            bass.process(&mut left[..count], &mut right[..count]);
            fast.set_tempo(110.0);
            selectable.set_tempo(110.0);
            bass.set_tempo(110.0);
            if n % 31 == 0 {
                fast.reset();
                selectable.reset();
                bass.reset();
            }
            black_box((
                fast.latency_samples(),
                fast.tail_samples(),
                fast.gap_samples(),
                selectable.latency_samples(),
                selectable.tail_samples(),
                selectable.gap_samples(),
                bass.latency_samples(),
                bass.tail_samples(),
                bass.gap_samples(),
            ));
            left[0] = 0.1;
            right[0] = -0.1;
        }
    });
    assert_eq!(calls, 0);
    let mut kept = None;
    assert_eq!(allocator_calls(|| kept = Some(vec![1_u8; 64])), 1);
    assert_eq!(allocator_calls(|| drop(kept.take())), 1);
}

fn benchmark<E: Effect + Default>(label: &str, params: E::Params, automate: bool) {
    let mut effect = prepared::<E>(params, 48_000.0);
    let input_l = noise(891, 128);
    let input_r = noise(453, 128);
    let (mut left, mut right) = ([0.0; 128], [0.0; 128]);
    let iterations = 30_000;
    let start = Instant::now();
    for n in 0..iterations {
        left.copy_from_slice(&input_l);
        right.copy_from_slice(&input_r);
        if automate {
            let mut next = params;
            let index = n % E::Params::descriptors().len();
            let info = &E::Params::descriptors()[index];
            next.set(index, if n % 2 == 0 { info.min } else { info.max });
            effect.set_params(&next);
        }
        effect.process(black_box(&mut left), black_box(&mut right));
        black_box((&left, &right));
    }
    let elapsed = start.elapsed().as_secs_f64();
    println!(
        "{label} automation={automate}: {:.2} M stereo frames/s, {:.3} us/128 block, {:.3}% single-core walltime at 48k; size {} bytes; tail {} frames",
        iterations as f64 * 128.0 / elapsed / 1e6,
        elapsed * 1e6 / iterations as f64,
        elapsed / (iterations as f64 * 128.0 / 48_000.0) * 100.0,
        std::mem::size_of::<E>(),
        effect.tail_samples()
    );
}

#[test]
#[ignore = "headless release throughput measurement, not an audio-device deadline test"]
fn scoped_cpu_throughput() {
    for automate in [false, true] {
        benchmark::<FastLowpass>(
            "Fast lowpass",
            FastLowpassParams {
                cutoff_hz: 1_000.0,
                q: 10.0,
            },
            automate,
        );
        benchmark::<SelectableFilter>(
            "Selectable filter",
            SelectableFilterParams {
                mode: Mode::LowShelf,
                gain_db: 18.0,
                ..Default::default()
            },
            automate,
        );
        benchmark::<BassShelf>(
            "Bass shelf",
            BassShelfParams {
                gain_db: 18.0,
                ..Default::default()
            },
            automate,
        );
    }
}
