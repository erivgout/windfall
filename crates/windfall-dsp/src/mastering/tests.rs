use super::*;

fn prepared(params: StageStackParams) -> StageStack {
    let mut effect = StageStack::default();
    effect.prepare(48_000.0, 4096);
    effect.set_params(&params);
    effect
}

#[test]
fn parameters_match_descriptors_serde_and_typescript() {
    let defaults = StageStackParams::default();
    let json = serde_json::to_value(defaults).unwrap();
    assert_eq!(StageStackParams::NAME, "Stage Stack");
    let declaration = StageStackParams::decl(&ts_rs::Config::default());
    for (index, info) in StageStackParams::descriptors().iter().enumerate() {
        assert_eq!(defaults.get(index), Some(info.default));
        assert_eq!(StageStackParams::index_of(info.id), Some(index));
        assert_eq!(json[info.id].as_f64().unwrap() as f32, info.default);
        assert!(declaration.contains(info.id));
        let mut params = defaults;
        assert!(params.set(index, info.max + 100.0));
        assert_eq!(params.get(index), Some(info.max));
        assert!(params.set(index, info.min - 100.0));
        assert_eq!(params.get(index), Some(info.min));
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(params.set(index, invalid));
            assert_eq!(params.get(index), Some(info.default));
        }
    }
    assert_eq!(
        serde_json::from_value::<StageStackParams>(json).unwrap(),
        defaults
    );
    assert_eq!(defaults.get(4), None);
    assert!(!StageStackParams::default().set(4, 0.0));
}

#[test]
fn zero_mix_is_exact_unity_and_latency_is_zero() {
    let mut effect = prepared(StageStackParams {
        mix: 0.0,
        ..Default::default()
    });
    let original = [-0.0, -2.0, 0.0, 0.2, 1.0, 4.0, f32::MAX, -f32::MAX];
    let mut left = original;
    let mut right = original;
    effect.process(&mut left, &mut right);
    assert_eq!(left.map(f32::to_bits), original.map(f32::to_bits));
    assert_eq!(right.map(f32::to_bits), original.map(f32::to_bits));
    assert_eq!(effect.latency_samples(), 0);
    assert_eq!(effect.tail_samples(), 0);
    effect.set_params(&StageStackParams {
        drive_db: 36.0,
        mix: 0.0,
        ..Default::default()
    });
    effect.process(&mut left, &mut right);
    assert_eq!(left.map(f32::to_bits), original.map(f32::to_bits));
}

#[test]
fn saturator_is_soft_bounded_and_tone_adds_even_color() {
    for input in [0.1, 0.5, 1.0, 3.0, 10.0] {
        let positive = saturate(input, 1.0, 0.0);
        assert_eq!(positive, -saturate(-input, 1.0, 0.0));
        assert!(saturate(input + 0.01, 1.0, 0.0) > positive);
        let even = saturate(input, 1.0, 1.0) + saturate(-input, 1.0, 1.0);
        assert!(even > 0.0);
    }
    assert!((saturate(0.001, 1.0, 0.0) / 0.001 - 1.0).abs() < 1e-5);
    assert!(saturate(0.1, db_to_gain(36.0), 0.0) > saturate(0.1, 1.0, 0.0));
    assert_eq!(saturate(0.0, 63.0, 1.0), 0.0);
    for input in [f32::MAX, -f32::MAX, 1e20, -1e20] {
        assert!(saturate(input, db_to_gain(36.0), 1.0).abs() <= 2.125);
    }
}

#[test]
fn hot_sine_is_reduced_to_ceiling_without_delaying_first_sample() {
    let mut effect = prepared(StageStackParams {
        ceiling_db: -6.0,
        ..Default::default()
    });
    let input: Vec<f32> = (0..4096)
        .map(|n| (n as f32 * 0.13 + 0.7).sin() * 3.0)
        .collect();
    let mut left = input.clone();
    let mut right = input.clone();
    effect.process(&mut left, &mut right);
    let ceiling = db_to_gain(-6.0);
    assert!(left[0] > 0.0);
    assert!(left.iter().all(|sample| sample.abs() <= ceiling));
    let energy = |signal: &[f32]| signal.iter().map(|x| x * x).sum::<f32>();
    assert!(energy(&left) < energy(&input) * 0.2);
    assert_eq!(left, right);
}

#[test]
fn release_retains_reduction_and_stereo_link_then_reset_clears_it() {
    let params = StageStackParams::default();
    let mut effect = prepared(params);
    effect.process(&mut [8.0], &mut [0.1]);
    let held = effect.safety_gain;
    assert!(held < 0.6);
    let mut left = [0.1];
    let mut right = left;
    effect.process(&mut left, &mut right);
    assert!(effect.safety_gain > held && effect.safety_gain < 0.6);
    assert!(left[0] < saturate(0.1, 1.0, 0.0) * 0.6);
    assert_eq!(left, right);
    effect.reset();
    effect.process(&mut left, &mut right);
    assert_eq!(effect.safety_gain, 1.0);
    effect.process(&mut [8.0], &mut [0.0]);
    let mut silence = [0.0; 4096];
    let mut silence_right = silence;
    effect.process(&mut silence, &mut silence_right);
    assert!(effect.safety_gain > 0.999);
    assert_eq!(silence, [0.0; 4096]);
}

#[test]
fn arbitrary_splits_match_with_live_parameter_edits() {
    let mut whole = prepared(StageStackParams::default());
    let mut split = prepared(StageStackParams::default());
    for segment in 0..3 {
        if segment != 0 {
            let params = StageStackParams {
                drive_db: 12.0 * segment as f32,
                tone: 0.4 * segment as f32,
                ceiling_db: -6.0 * segment as f32,
                mix: 0.3 * segment as f32,
            };
            whole.set_params(&params);
            split.set_params(&params);
        }
        let mut left: Vec<f32> = (0..1337)
            .map(|n| ((n + segment * 1337) as f32 * 0.17).sin() * 4.0)
            .collect();
        let mut right: Vec<f32> = left.iter().map(|x| -x * 0.3).collect();
        let mut split_left = left.clone();
        let mut split_right = right.clone();
        whole.process(&mut left, &mut right);
        let mut offset = 0;
        for size in [1, 7, 53, 2, 239, 13, 512, 3, 507] {
            let end = (offset + size).min(left.len());
            split.process(&mut split_left[offset..end], &mut split_right[offset..end]);
            split.process(&mut [], &mut []);
            offset = end;
        }
        assert_eq!(offset, left.len());
        assert_eq!(left, split_left);
        assert_eq!(right, split_right);
    }
}

#[test]
fn extreme_audio_params_and_sample_rates_remain_finite() {
    for sample_rate in [1.0, 48_000.0, 384_000.0, f32::NAN, f32::INFINITY] {
        for mix in [0.0, 0.5, 1.0] {
            let mut effect = StageStack::default();
            effect.prepare(sample_rate, 16);
            effect.set_params(&StageStackParams {
                drive_db: 36.0,
                tone: 1.0,
                ceiling_db: -24.0,
                mix,
            });
            let mut left = [
                f32::NAN,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::MAX,
                -f32::MAX,
                0.0,
            ];
            let mut right = left;
            effect.process(&mut left, &mut right);
            assert!(left.iter().chain(&right).all(|sample| sample.is_finite()));
            assert_eq!(left[..3], [0.0; 3]);
            effect.set_params(&StageStackParams {
                drive_db: f32::NAN,
                tone: f32::INFINITY,
                ceiling_db: f32::NEG_INFINITY,
                mix: f32::NAN,
            });
            effect.process(&mut left, &mut right);
            assert!(left.iter().chain(&right).all(|sample| sample.is_finite()));
        }
    }
}

#[test]
fn lowering_ceiling_is_immediate_and_reset_preserves_targets() {
    let mut effect = prepared(StageStackParams::default());
    effect.process(&mut [4.0], &mut [4.0]);
    let params = StageStackParams {
        drive_db: 24.0,
        tone: 1.0,
        ceiling_db: -24.0,
        mix: 1.0,
    };
    effect.set_params(&params);
    let mut left = [4.0; 31];
    let mut right = left;
    effect.process(&mut left, &mut right);
    assert!(left.iter().all(|x| x.abs() <= db_to_gain(-24.0)));
    effect.reset();
    let mut fresh = prepared(params);
    effect.process(&mut left, &mut right);
    let mut expected_left = [db_to_gain(-24.0); 31];
    let mut expected_right = expected_left;
    fresh.process(&mut expected_left, &mut expected_right);
    assert_eq!(left, expected_left);
    assert_eq!(right, expected_right);
}
