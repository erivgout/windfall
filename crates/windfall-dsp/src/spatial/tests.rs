use super::*;
use crate::{Effect, ParamSet};

const RATE: f32 = 8000.0;
fn sine(hz: f32, len: usize) -> Vec<f32> {
    (0..len)
        .map(|i| (std::f32::consts::TAU * hz * i as f32 / RATE).sin() * 0.2)
        .collect()
}

#[test]
fn spatial_band_impulse_is_delayed_and_complementary_bands_recombine() {
    let mut fx = BandDelay::default();
    fx.prepare(RATE, 128);
    fx.set_params(&BandDelayParams {
        time_ms: [10.0; 3],
        feedback: [0.0; 3],
        pan: [0.0; 3],
        mix: 1.0,
        ..Default::default()
    });
    let mut l = vec![0.0; 400];
    let mut r = l.clone();
    l[0] = 1.0;
    r[0] = 1.0;
    fx.process(&mut l, &mut r);
    for (i, x) in l.iter().enumerate() {
        let expected = if i == 80 { 1.0 } else { 0.0 };
        assert!((x - expected).abs() < 2e-6, "sample {i}: {x}");
    }
    assert_eq!(l, r);
    assert_eq!(fx.latency_samples(), 0); // intentional effect delay, no PDC
}

#[test]
fn spatial_vintage_chorus_returns_a_delayed_toned_impulse() {
    let mut fx = VintageChorus::default();
    fx.prepare(RATE, 128);
    fx.set_params(&VintageChorusParams {
        delay_ms: 10.0,
        depth_ms: 0.0,
        rate_hz: 0.0,
        mix: 1.0,
        ..Default::default()
    });
    let mut l = vec![0.0; 500];
    let mut r = l.clone();
    l[0] = 1.0;
    fx.process(&mut l, &mut r);
    assert!(l[..80].iter().all(|x| *x == 0.0));
    assert!(l[80..150].iter().any(|x| x.abs() > 0.05));
    assert!(r.iter().all(|x| *x == 0.0));
}

#[test]
fn spatial_room_has_early_reflections_and_a_decaying_short_tail() {
    let mut fx = Room::default();
    fx.prepare(RATE, 128);
    fx.set_params(&RoomParams {
        pre_delay_ms: 0.0,
        mix: 1.0,
        decay_s: 0.25,
        ..Default::default()
    });
    let mut l = vec![0.0; RATE as usize];
    let mut r = l.clone();
    l[0] = 1.0;
    fx.process(&mut l, &mut r);
    assert!(l[..20].iter().all(|x| *x == 0.0));
    assert!(l[20..200].iter().any(|x| x.abs() > 0.02));
    let early: f32 = l[..2000].iter().chain(&r[..2000]).map(|x| x * x).sum();
    let late: f32 = l[6000..].iter().chain(&r[6000..]).map(|x| x * x).sum();
    assert!(early > 0.01 && late < early * 1e-4, "{early} -> {late}");
}

fn feedback_test<E: Effect + Default>(params: E::Params) {
    let mut fx = E::default();
    fx.prepare(RATE, 127);
    fx.set_params(&params);
    let mut l = [0.0; 127];
    let mut r = [0.0; 127];
    let mut peak = 0.0_f32;
    for block in 0..400 {
        l.fill(0.0);
        r.fill(0.0);
        if block == 0 {
            l[0] = 1.0;
            r[0] = -0.5;
        }
        fx.process(&mut l, &mut r);
        for x in l.iter().chain(&r) {
            assert!(x.is_finite());
            peak = peak.max(x.abs());
        }
    }
    assert!(peak > 0.001 && peak < 20.0, "unstable peak {peak}");
}
#[test]
fn spatial_feedback_is_finite_at_extreme_signed_settings() {
    for sign in [-1.0, 1.0] {
        feedback_test::<StackedFlanger>(StackedFlangerParams {
            feedback: sign * f32::MAX,
            delay_ms: 6.0,
            depth_ms: 8.0,
            rate1_hz: 5.0,
            rate2_hz: 4.2,
            tone_hz: 20_000.0,
            mix: 1.0,
            ..Default::default()
        });
        feedback_test::<VintagePhaser>(VintagePhaserParams {
            feedback: sign * f32::MAX,
            rate_hz: 5.0,
            min_hz: 20.0,
            max_hz: 10_000.0,
            color_hz: 12_000.0,
            mix: 1.0,
            ..Default::default()
        });
        feedback_test::<BandDelay>(BandDelayParams {
            feedback: [sign * f32::MAX; 3],
            time_ms: [5.0, 9.0, 13.0],
            mix: 1.0,
            ..Default::default()
        });
    }
    feedback_test::<Room>(RoomParams {
        decay_s: f32::MAX,
        damping: 0.0,
        mix: 1.0,
        ..Default::default()
    });
}

fn modulation_test<E: Effect + Default>(params: E::Params) {
    let mut fx = E::default();
    fx.prepare(RATE, 800);
    fx.set_params(&params);
    // 500 Hz repeats exactly every 16 samples. Both blocks have identical
    // steady input phase and are past delay/filter startup.
    let mut l = sine(500.0, 16_000);
    let mut r = l.clone();
    for (l, r) in l.chunks_mut(800).zip(r.chunks_mut(800)) {
        fx.process(l, r);
    }
    let change: f32 = l[8000..8800]
        .iter()
        .zip(&l[12_000..12_800])
        .map(|(a, b)| (a - b).abs())
        .sum();
    assert!(change > 1.0, "no measurable modulation: {change}");
}
#[test]
fn spatial_choruses_and_phaser_modulate_a_steady_sine_across_blocks() {
    modulation_test::<VintageChorus>(VintageChorusParams::default());
    modulation_test::<HyperChorus>(HyperChorusParams::default());
    modulation_test::<VintagePhaser>(VintagePhaserParams::default());
    modulation_test::<StackedFlanger>(StackedFlangerParams::default());
}

#[test]
fn spatial_enhancer_locks_low_sine_more_mono_than_high_sine() {
    let measure = |hz| {
        let mut fx = StereoEnhancer::default();
        fx.prepare(RATE, 128);
        fx.set_params(&StereoEnhancerParams {
            bass_hz: 300.0,
            width: 1.5,
            ..Default::default()
        });
        let mut l = sine(hz, 8000);
        let mut r: Vec<_> = l.iter().map(|x| x * 0.25).collect();
        fx.process(&mut l, &mut r);
        let side: f32 = l[2000..]
            .iter()
            .zip(&r[2000..])
            .map(|(l, r)| (l - r).powi(2))
            .sum();
        let mid: f32 = l[2000..]
            .iter()
            .zip(&r[2000..])
            .map(|(l, r)| (l + r).powi(2))
            .sum();
        side / mid
    };
    let low = measure(40.0);
    let high = measure(2000.0);
    assert!(low < high * 0.01 && high > 0.1, "low {low}, high {high}");
}

#[test]
fn spatial_spreader_preserves_mono_fold_down_including_live_edits() {
    let mut fx = Spreader::default();
    fx.prepare(RATE, 1);
    let l = sine(321.0, 4000);
    let r = sine(731.0, 4000);
    for i in 0..l.len() {
        if i == 1733 {
            fx.set_params(&SpreaderParams {
                width: 2.0,
                haas_ms: 27.0,
                amount: 1.0,
                mix: 0.8,
            });
        }
        let (mut ol, mut or) = ([l[i]], [r[i]]);
        fx.process(&mut ol, &mut or);
        assert!((ol[0] + or[0] - l[i] - r[i]).abs() < 1e-6);
    }
}

fn partition<E: Effect>(mut whole: E, mut pieces: E, edit: E::Params) {
    let (mut l, mut r) = (sine(271.0, 6000), sine(809.0, 6000));
    l[1200..].fill(0.0);
    r[1200..].fill(0.0);
    let (mut pl, mut pr) = (l.clone(), r.clone());
    // Same edit at the same frame, exercised inside a ringing tail.
    for (start, end) in [(0, 1777), (1777, 6000)] {
        if start != 0 {
            whole.set_params(&edit);
            pieces.set_params(&edit);
        }
        whole.process(&mut l[start..end], &mut r[start..end]);
        let sizes = [1, 31, 128, 7, 53];
        let mut at = start;
        let mut n = 0;
        while at < end {
            let to = (at + sizes[n % sizes.len()]).min(end);
            pieces.process(&mut pl[at..to], &mut pr[at..to]);
            at = to;
            n += 1;
        }
    }
    assert_eq!(l, pl);
    assert_eq!(r, pr);
}
#[test]
fn spatial_room_is_exactly_partition_invariant_with_tail_automation() {
    let (mut a, mut b) = (Room::default(), Room::default());
    for fx in [&mut a, &mut b] {
        fx.prepare(RATE, 6000);
    }
    partition(
        a,
        b,
        RoomParams {
            size: 0.8,
            decay_s: 1.2,
            pre_delay_ms: 17.0,
            mix: 0.8,
            ..Default::default()
        },
    );
}

#[test]
fn spatial_existing_echo_bank_substitute_is_partition_invariant() {
    use crate::echo_bank::{EchoBank, EchoBankParams};
    let mut p = EchoBankParams {
        dry: 0.2,
        wet: 0.8,
        ..Default::default()
    };
    for (i, unit) in p.units.iter_mut().enumerate() {
        unit.enabled = i < 4;
        unit.sync = false;
        unit.time_ms = 11.0 + i as f32 * 13.0;
        unit.next_send = if i < 3 { 0.4 } else { 0.0 };
        unit.feedback = 0.25;
        unit.output_pan = i as f32 * 0.15 - 0.4;
    }
    let (mut a, mut b) = (EchoBank::default(), EchoBank::default());
    for fx in [&mut a, &mut b] {
        fx.prepare(RATE, 6000);
        fx.set_params(&p);
    }
    p.units[1].time_ms = 80.0;
    p.units[2].output_pan = -0.5;
    partition(a, b, p);
}

#[test]
fn spatial_other_new_circuits_are_partition_invariant() {
    macro_rules! check {
        ($effect:ty, $params:expr) => {{
            let (mut a, mut b) = (<$effect>::default(), <$effect>::default());
            a.prepare(RATE, 6000);
            b.prepare(RATE, 6000);
            partition(a, b, $params);
        }};
    }
    check!(
        VintageChorus,
        VintageChorusParams {
            depth_ms: 9.0,
            mix: 0.8,
            ..Default::default()
        }
    );
    check!(
        HyperChorus,
        HyperChorusParams {
            spread: 0.3,
            rate_hz: 3.0,
            ..Default::default()
        }
    );
    check!(
        VintagePhaser,
        VintagePhaserParams {
            feedback: -0.7,
            color_hz: 300.0,
            ..Default::default()
        }
    );
    check!(
        StackedFlanger,
        StackedFlangerParams {
            rate2_hz: 3.0,
            feedback: -0.65,
            ..Default::default()
        }
    );
    check!(
        BandDelay,
        BandDelayParams {
            time_ms: [11.0, 23.0, 17.0],
            ..Default::default()
        }
    );
    check!(
        Spreader,
        SpreaderParams {
            haas_ms: 25.0,
            amount: 0.9,
            ..Default::default()
        }
    );
    check!(
        StereoEnhancer,
        StereoEnhancerParams {
            bass_hz: 400.0,
            width: 2.0,
            ..Default::default()
        }
    );
}

fn params_check<P: ParamSet + serde::Serialize + serde::de::DeserializeOwned>() {
    let default = P::default();
    assert_eq!(default, default.sanitized());
    assert_eq!(
        default,
        serde_json::from_str::<P>(&serde_json::to_string(&default).unwrap()).unwrap()
    );
    assert_eq!(default, serde_json::from_str::<P>("{}").unwrap());
    for (i, d) in P::descriptors().iter().enumerate() {
        assert_eq!(default.get(i), Some(d.default), "{} {}", P::NAME, d.id);
        assert_eq!(P::index_of(d.id), Some(i));
        for value in [f32::NAN, f32::INFINITY, -f32::MAX, f32::MAX] {
            let mut p = default;
            assert!(p.set(i, value));
            let x = p.get(i).unwrap();
            assert!(x.is_finite() && x >= d.min && x <= d.max);
        }
    }
    assert_eq!(default.get(P::descriptors().len()), None);
}
#[test]
fn spatial_params_have_stable_descriptors_defaults_and_json() {
    params_check::<VintageChorusParams>();
    params_check::<HyperChorusParams>();
    params_check::<VintagePhaserParams>();
    params_check::<StackedFlangerParams>();
    params_check::<BandDelayParams>();
    params_check::<RoomParams>();
    params_check::<SpreaderParams>();
    params_check::<StereoEnhancerParams>();
}

fn hostile<E: Effect + Default>() {
    let mut fx = E::default();
    let (mut l, mut r) = ([f32::NAN, f32::MAX, f32::INFINITY, -f32::MAX], [0.0; 4]);
    fx.process(&mut l, &mut r);
    assert!(l.iter().chain(&r).all(|x| x.is_finite()));
    for sr in [f32::NAN, 1.0, 8000.0, 192_000.0] {
        fx.prepare(sr, 4);
        let mut params = E::Params::default();
        for i in 0..E::Params::descriptors().len() {
            params.set(i, f32::MAX);
        }
        fx.set_params(&params);
        fx.set_tempo(f32::NAN);
        l = [f32::NAN, f32::MAX, f32::INFINITY, -f32::MAX];
        fx.process(&mut l, &mut r);
        assert!(l.iter().chain(&r).all(|x| x.is_finite()));
        fx.reset();
        fx.process(&mut [], &mut []);
    }
}
#[test]
fn spatial_hostile_input_and_sample_rates_remain_finite() {
    hostile::<VintageChorus>();
    hostile::<HyperChorus>();
    hostile::<VintagePhaser>();
    hostile::<StackedFlanger>();
    hostile::<BandDelay>();
    hostile::<Room>();
    hostile::<Spreader>();
    hostile::<StereoEnhancer>();
}

fn tail_check<E: Effect + Default>() {
    let mut fx = E::default();
    fx.prepare(1000.0, 1);
    let mut l = [1.0];
    let mut r = [-0.5];
    fx.process(&mut l, &mut r);
    for _ in 0..fx.tail_samples() {
        l = [0.0];
        r = [0.0];
        fx.process(&mut l, &mut r);
    }
    for _ in 0..1000 {
        l = [0.0];
        r = [0.0];
        fx.process(&mut l, &mut r);
        assert_eq!(l, [0.0]);
        assert_eq!(r, [0.0]);
    }
}
#[test]
fn spatial_reported_tails_really_expire() {
    tail_check::<VintageChorus>();
    tail_check::<HyperChorus>();
    tail_check::<VintagePhaser>();
    tail_check::<StackedFlanger>();
    tail_check::<BandDelay>();
    tail_check::<Room>();
    tail_check::<Spreader>();
    tail_check::<StereoEnhancer>();
}
