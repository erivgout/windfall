//! Review regressions exercised through prepared processors and slots.
use windfall_dsp::{AnyEffect, EffectParams, EffectSlot, LimiterParams, StereoMatrixParams};

fn matrix(left_delay_ms: f32, right_delay_ms: f32) -> EffectParams {
    EffectParams::StereoMatrix(StereoMatrixParams {
        left_delay_ms,
        right_delay_ms,
        ..Default::default()
    })
}

fn run_slot(slot: &mut EffectSlot, left: &mut [f32], right: &mut [f32], blocks: &[usize]) {
    let mut offset = 0;
    for size in blocks.iter().copied().cycle() {
        if offset == left.len() {
            break;
        }
        let end = (offset + size).min(left.len());
        slot.process(&mut left[offset..end], &mut right[offset..end]);
        offset = end;
    }
}

#[test]
fn r2_bypass_fade_reports_the_wet_impulse_tail_until_it_is_inaudible() {
    for blocks in [&[1000][..], &[1, 7, 137][..]] {
        for delay in [0.0, 2.0] {
            for mix in [0.5, 1.0] {
                for mix_off in [false, true] {
                    let mut slot = EffectSlot::new(AnyEffect::new(&matrix(delay, 5.0)));
                    slot.prepare(48_000.0, 137);
                    slot.set_mix(mix);
                    slot.process(&mut [1.0], &mut [1.0]);
                    if mix_off {
                        slot.set_mix(0.0);
                    } else {
                        slot.set_enabled(false);
                    }
                    let tail = slot.tail_samples();
                    let gap = slot.gap_samples();
                    assert!(tail >= 240, "wet bypass tail reported {tail}");
                    assert!(gap >= 240, "wet bypass gap reported {gap}");
                    let (mut l, mut r) = (vec![0.0; 1000], vec![0.0; 1000]);
                    run_slot(&mut slot, &mut l, &mut r, blocks);
                    assert!(
                        (r[239] - mix * 0.5).abs() < 1e-5,
                        "right impulse must remain audible: {}",
                        r[239]
                    );
                    assert!(
                        l[tail..]
                            .iter()
                            .chain(&r[tail..])
                            .all(|sample| *sample == 0.0)
                    );
                    assert!(
                        l[gap..]
                            .iter()
                            .chain(&r[gap..])
                            .all(|sample| *sample == 0.0)
                    );
                    let dry = (delay * 48.0).round() as usize;
                    assert_eq!(slot.tail_samples(), dry);
                    assert_eq!(slot.gap_samples(), dry);
                }
            }
        }
    }
}

#[test]
fn asymmetric_matrix_wake_primes_both_outputs_before_bypass_or_mix_fade() {
    for blocks in [&[4096][..], &[1, 137, 29, 511, 3][..]] {
        for mix_wake in [false, true] {
            let mut slot = EffectSlot::new(AnyEffect::new(&matrix(0.0, 50.0)));
            slot.prepare(48_000.0, 511);
            for _ in 0..2 {
                let (mut l, mut r) = (vec![1.0; 4000], vec![1.0; 4000]);
                run_slot(&mut slot, &mut l, &mut r, blocks);
                if mix_wake {
                    slot.set_mix(0.0);
                } else {
                    slot.set_enabled(false);
                }
                l.fill(1.0);
                r.fill(1.0);
                run_slot(&mut slot, &mut l, &mut r, blocks);
                if mix_wake {
                    slot.set_mix(0.5);
                } else {
                    slot.set_enabled(true);
                }
                l.fill(1.0);
                r.fill(1.0);
                run_slot(&mut slot, &mut l, &mut r, blocks);
                assert_eq!(slot.latency_samples(), 0, "shared PDC must stay zero");
                for (n, (&l, &r)) in l.iter().zip(&r).enumerate() {
                    assert_eq!(l, 1.0, "left at {n}");
                    assert_eq!(r, 1.0, "unprimed right at {n}, mix wake {mix_wake}");
                }
                slot.reset();
            }
        }
    }
}

#[test]
fn rapid_matrix_retargets_preserve_the_audible_taps_and_finish_at_the_final_setting() {
    // This 1 kHz tone changes by at most 0.131 per sample. Changing a tap
    // through a 240-sample fade can add at most 2/240, not a full-scale step.
    let input: Vec<f32> = (0..6000)
        .map(|n| (std::f64::consts::TAU * n as f64 / 48.0).sin() as f32)
        .collect();
    let mut reference = std::collections::BTreeMap::new();
    for blocks in [&[6000][..], &[1, 137, 29, 511, 3][..]] {
        for common in [false, true] {
            for mix in [1.0, 0.5, 0.0] {
                let mut slot = EffectSlot::new(AnyEffect::new(&matrix(0.0, 0.0)));
                slot.prepare(48_000.0, 511);
                slot.set_mix(mix);
                let (mut l, mut r) = (input.clone(), input.clone());
                let mut at = 0;
                for (end, delay) in [
                    (4092, 0.5),
                    (4212, 1.0),
                    (4249, 0.25),
                    (4266, 1.5),
                    (4327, 0.75),
                    (6000, 0.75),
                ] {
                    run_slot(&mut slot, &mut l[at..end], &mut r[at..end], blocks);
                    slot.set_params(&matrix(if common { delay } else { 0.0 }, delay));
                    at = end;
                }
                let jump = (r[4212] - r[4211]).abs();
                assert!(
                    jump < 0.15,
                    "retarget step {jump}: {} -> {}, common={common}, mix={mix}",
                    r[4211],
                    r[4212]
                );
                for samples in [&l, &r] {
                    assert!(
                        samples[4092..]
                            .windows(2)
                            .all(|pair| (pair[1] - pair[0]).abs() < 0.15)
                    );
                }
                assert_eq!(slot.latency_samples(), if common { 36 } else { 0 });
                for n in 4567..6000 {
                    let dry = input[n - if common { 36 } else { 0 }];
                    let expected = if mix == 1.0 {
                        input[n - 36]
                    } else {
                        dry + (input[n - 36] - dry) * mix
                    };
                    assert_eq!(r[n], expected, "final right at {n}");
                }
                let key = (common, mix.to_bits());
                if let Some(expected) = reference.get(&key) {
                    assert_eq!(&(l, r), expected, "rapid retarget block invariance");
                } else {
                    reference.insert(key, (l, r));
                }
            }
        }
    }
}

#[test]
fn bypassed_matrix_retains_old_dry_tails_and_reset_uses_the_final_target() {
    let mut slot = EffectSlot::new(AnyEffect::new(&matrix(50.0, 50.0)));
    slot.prepare(48_000.0, 137);
    slot.set_enabled(false);
    let (mut l, mut r) = (vec![1.0; 3000], vec![1.0; 3000]);
    run_slot(&mut slot, &mut l, &mut r, &[1, 137]);
    slot.set_params(&matrix(0.0, 0.0));
    slot.process(&mut [1.0], &mut [1.0]);
    assert_eq!(slot.latency_samples(), 0);
    assert_eq!(slot.tail_samples(), 2400);
    assert_eq!(slot.gap_samples(), 2400);
    slot.set_params(&matrix(0.5, 0.5));
    slot.process(&mut [1.0], &mut [1.0]);
    slot.set_params(&matrix(1.0, 1.0));
    slot.reset();
    slot.set_enabled(true);
    assert_eq!(slot.latency_samples(), 48);
    assert_eq!(slot.tail_samples(), 48);
    let (mut l, mut r) = (vec![0.0; 100], vec![0.0; 100]);
    l[0] = 1.0;
    r[0] = 1.0;
    run_slot(&mut slot, &mut l, &mut r, &[1, 7, 137]);
    assert_eq!(l[48], 1.0);
    assert_eq!(l.iter().filter(|sample| **sample != 0.0).count(), 1);
    assert_eq!(l, r);
    slot.set_enabled(false);
    slot.set_params(&matrix(50.0, 50.0));
    assert_eq!(
        slot.tail_samples(),
        2400,
        "new dry target before processing"
    );
    assert_eq!(slot.gap_samples(), 2400);
}

#[test]
fn asymmetric_matrix_wake_reaches_the_requested_wet_gain_with_shared_dry_delay() {
    let params = EffectParams::StereoMatrix(StereoMatrixParams {
        ll: 0.25,
        rr: 0.5,
        left_delay_ms: 2.0,
        right_delay_ms: 5.0,
        ..Default::default()
    });
    let mut slot = EffectSlot::new(AnyEffect::new(&params));
    slot.prepare(48_000.0, 137);
    let (mut l, mut r) = (vec![1.0; 3000], vec![1.0; 3000]);
    run_slot(&mut slot, &mut l, &mut r, &[1, 29, 137]);
    slot.set_mix(0.0);
    l.fill(1.0);
    r.fill(1.0);
    run_slot(&mut slot, &mut l, &mut r, &[1, 29, 137]);
    slot.set_mix(0.5);
    l.fill(1.0);
    r.fill(1.0);
    run_slot(&mut slot, &mut l, &mut r, &[1, 29, 137]);
    assert_eq!(slot.latency_samples(), 96);
    assert_eq!(&l[..240], &[1.0; 240]);
    assert_eq!(&r[..240], &[1.0; 240]);
    assert_eq!(l[2999], 0.625);
    assert_eq!(r[2999], 0.75);
    assert!(
        l.windows(2)
            .chain(r.windows(2))
            .all(|p| (p[1] - p[0]).abs() < 0.002)
    );
}

#[test]
fn matrix_delay_edits_during_wake_extend_priming_across_irregular_blocks() {
    for blocks in [&[4000][..], &[1, 7, 137][..]] {
        let mut slot = EffectSlot::new(AnyEffect::new(&matrix(0.0, 5.0)));
        slot.prepare(48_000.0, 137);
        let (mut l, mut r) = (vec![1.0; 4000], vec![1.0; 4000]);
        run_slot(&mut slot, &mut l, &mut r, blocks);
        slot.set_enabled(false);
        l.fill(1.0);
        r.fill(1.0);
        run_slot(&mut slot, &mut l, &mut r, blocks);
        slot.set_enabled(true);
        l.fill(1.0);
        r.fill(1.0);
        run_slot(&mut slot, &mut l[..100], &mut r[..100], blocks);
        slot.set_params(&matrix(0.0, 50.0));
        run_slot(&mut slot, &mut l[100..], &mut r[100..], blocks);
        for (frame, sample) in r.iter().enumerate() {
            assert_eq!(
                *sample, 1.0,
                "delay edit heard an unprimed channel at {frame}"
            );
        }
    }
}

#[test]
fn matrix_slot_repairs_preserve_the_limiters_existing_dry_transition_policy() {
    let input: Vec<f32> = (0..5000)
        .map(|n| 0.25 * (std::f64::consts::TAU * n as f64 / 48.0).sin() as f32)
        .collect();
    let render = |mix| {
        let params = |lookahead_ms| {
            EffectParams::Limiter(LimiterParams {
                ceiling_db: 0.0,
                lookahead_ms,
                ..Default::default()
            })
        };
        let mut slot = EffectSlot::new(AnyEffect::new(&params(2.0)));
        slot.prepare(48_000.0, 137);
        slot.set_mix(mix);
        let (mut l, mut r) = (input.clone(), input.clone());
        run_slot(&mut slot, &mut l[..4092], &mut r[..4092], &[137]);
        slot.set_params(&params(0.5));
        run_slot(&mut slot, &mut l[4092..4212], &mut r[4092..4212], &[137]);
        slot.set_params(&params(1.0));
        run_slot(&mut slot, &mut l[4212..], &mut r[4212..], &[137]);
        l
    };
    let wet = render(1.0);
    for mix in [0.0, 0.5] {
        for (frame, (wet, mixed)) in wet.iter().zip(render(mix)).enumerate() {
            assert_eq!(*wet, mixed, "limiter dry alignment at {frame}, mix {mix}");
        }
    }
}
