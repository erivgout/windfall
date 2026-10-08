//! Private fault injection exercises the actual production staging path.
use super::*;
use crate::echo_bank::support::{allocation_test, stage_histories};
use crate::echo_bank::*;
use crate::frequency_delay::*;
use crate::{Effect, NoteDivision, ParamSet};

#[test]
fn checked_capacity_overflow_and_budget_boundaries_never_allocate() {
    let before = allocation_test::snapshot();
    for result in [
        PreparationRequirements::checked::<1>(1.0, usize::MAX, 0, 0),
        PreparationRequirements::checked::<1>(1.0, usize::MAX / 2, 0, 0),
        PreparationRequirements::checked::<1>(1.0, isize::MAX as usize / 4, 0, 0),
        PreparationRequirements::checked::<{ usize::MAX }>(1.0, 0, 0, 0),
        PreparationRequirements::checked::<1>(1.0, 0, usize::MAX, 0),
        PreparationRequirements::checked::<1>(1.0, 0, 0, usize::MAX),
        PreparationRequirements::with_heap::<{ usize::MAX }>(1.0, 0, 0, 0, 0),
    ] {
        assert!(matches!(
            result,
            Err(PreparationError::CapacityOverflow { .. })
        ));
    }
    assert!(matches!(
        stage_histories::<2>(1.0, usize::MAX, 0, 0, PreparationBudget::UNLIMITED),
        Err(PreparationError::CapacityOverflow {
            maximum_delay_samples: usize::MAX,
            history_count: 2
        })
    ));
    // Test the largest per-Vec pointer-offset domain without reserving it.
    let largest =
        PreparationRequirements::checked::<1>(1.0, isize::MAX as usize / 4 - 1, 0, 0).unwrap();
    assert_eq!(largest.retained_bytes, (isize::MAX as usize / 4) * 4);
    let req = PreparationRequirements::checked::<16>(1.0, 25, 123, 456).unwrap();
    let exact = PreparationBudget {
        retained_bytes: req.retained_bytes,
        peak_bytes: req.peak_bytes,
    };
    assert_eq!(req.admit(exact), Ok(()));
    assert_eq!(
        req.admit(PreparationBudget {
            retained_bytes: exact.retained_bytes - 1,
            ..exact
        }),
        Err(PreparationError::RetainedBudgetExceeded {
            required_bytes: exact.retained_bytes,
            budget_bytes: exact.retained_bytes - 1
        })
    );
    assert_eq!(
        req.admit(PreparationBudget {
            peak_bytes: exact.peak_bytes - 1,
            ..exact
        }),
        Err(PreparationError::PeakBudgetExceeded {
            required_bytes: exact.peak_bytes,
            budget_bytes: exact.peak_bytes - 1
        })
    );
    assert!(matches!(
        stage_histories::<16>(
            1.0,
            25,
            123,
            456,
            PreparationBudget {
                peak_bytes: exact.peak_bytes - 1,
                ..exact
            }
        ),
        Err(PreparationError::PeakBudgetExceeded { .. })
    ));
    assert_eq!(allocation_test::snapshot(), before);
}

fn bank_params() -> EchoBankParams {
    let mut p = EchoBankParams {
        dry: 0.1,
        wet: 0.8,
        ..Default::default()
    };
    for (i, u) in p.units.iter_mut().enumerate() {
        u.enabled = true;
        u.sync = i == 7;
        u.division = NoteDivision::ThirtySecond;
        u.time_ms = 1.0 + i as f32;
        u.feedback = 0.6;
        u.feedback_mode = if i % 2 == 0 {
            EchoFeedbackMode::PingPong
        } else {
            EchoFeedbackMode::Normal
        };
        u.output_gain = if i % 2 == 0 { -0.1 } else { 0.1 };
        u.next_send = 0.2;
        u.filter.mode = EchoFilterMode::Lowpass;
        u.filter.frequency_hz = 100.0 * (i + 1) as f32;
        u.filter.sections = 3;
        u.feedback_filter.mode = EchoFilterMode::Highpass;
        u.feedback_filter.frequency_hz = 200.0;
    }
    p.sanitized()
}
fn frequency_params() -> FrequencyDelayParams {
    let mut p = FrequencyDelayParams {
        feedback: 0.5,
        bandwidth: 1.3,
        split: FrequencySplit::Steep,
        ..Default::default()
    };
    for (i, b) in p.bands.iter_mut().enumerate() {
        b.delay_ms = (i + 1) as f32 * 1.37;
        b.level = (i + 1) as f32 / 16.0;
        b.pan = (i as f32 - 7.0) / 8.0;
    }
    p
}
fn input(seed: usize) -> ([f32; 127], [f32; 127]) {
    (
        std::array::from_fn(|n| ((seed + n) as f32 * 0.13).sin() * 0.3),
        std::array::from_fn(|n| ((seed + n) as f32 * 0.071).cos() * 0.2),
    )
}

macro_rules! rollback {
    ($processor:ty, $params:expr, $count:expr, $retarget:expr) => {{
        let (mut subject, mut reference) = (<$processor>::default(), <$processor>::default());
        let mut p = $params;
        for e in [&mut subject, &mut reference] {
            e.prepare(8000.0, 64);
            e.set_params(&p);
            for seed in 0..8 {
                let (mut l, mut r) = input(seed * 127);
                e.process(&mut l, &mut r);
            }
        }
        // Refuse every one of the independently allocated histories, including
        // the last reservation after ALL other replacements have succeeded.
        for failed in 0..$count {
            ($retarget)(&mut p, failed);
            for e in [&mut subject, &mut reference] {
                e.set_params(&p);
                e.set_tempo(121.0 + failed as f32);
                let (mut l, mut r) = input(37 * failed);
                e.process(&mut l[..37], &mut r[..37]);
            }
            let state = (
                subject.actual_sample_rate(),
                subject.prepared_bytes(),
                subject.params(),
                subject.warm_up_samples(),
                subject.gap_samples(),
                subject.tail_samples(),
                subject.delay_readiness_samples(),
                subject.latency_transition_samples_remaining(),
            );
            let before = allocation_test::snapshot();
            let fault = allocation_test::Fault::at(failed);
            let error = subject
                .try_prepare(48000.0, 1024, PreparationBudget::UNLIMITED)
                .unwrap_err();
            drop(fault);
            assert_eq!(
                error,
                PreparationError::ReservationFailed {
                    history_index: failed,
                    requested_bytes: (<$processor>::preparation_bytes(48000.0)
                        - size_of::<$processor>())
                        / $count,
                }
            );
            let after = allocation_test::snapshot();
            assert_eq!(
                after.0, before.0,
                "staged histories leaked or live histories were retired"
            );
            assert_eq!(after.1 - before.1, failed);
            assert_eq!(
                after.2 - before.2,
                failed,
                "partial staging must retire off-thread"
            );
            assert_eq!(
                (
                    subject.actual_sample_rate(),
                    subject.prepared_bytes(),
                    subject.params(),
                    subject.warm_up_samples(),
                    subject.gap_samples(),
                    subject.tail_samples(),
                    subject.delay_readiness_samples(),
                    subject.latency_transition_samples_remaining()
                ),
                state
            );
            assert_eq!(
                subject.preparation_status(),
                PreparationStatus {
                    is_prepared: true,
                    last_refusal: Some(PreparationRefusal {
                        sample_rate: 48000.0,
                        error
                    })
                }
            );
            for seed in 0..12 {
                let (mut sl, mut sr) = input(failed * 2000 + seed * 127);
                let (mut rl, mut rr) = (sl, sr);
                // Include no-ops/empty blocks while a filter/time/gain edit is
                // still running. Histories AND frame clocks must be unchanged.
                subject.process(&mut [], &mut []);
                subject.set_params(&p);
                subject.process(&mut sl, &mut sr);
                reference.process(&mut rl, &mut rr);
                assert_eq!(sl, rl, "left audio after reservation {failed}");
                assert_eq!(sr, rr, "right audio after reservation {failed}");
            }
        }
        // Legacy Effect callers cannot receive Result, but must see the same
        // latch and preserved source after a partially staged refusal.
        let before = allocation_test::snapshot();
        let fault = allocation_test::Fault::at($count - 1);
        subject.prepare(48000.0, 1);
        drop(fault);
        let after = allocation_test::snapshot();
        assert_eq!(after.0, before.0);
        assert_eq!(after.1 - before.1, $count - 1);
        assert_eq!(after.2 - before.2, $count - 1);
        assert_eq!(subject.actual_sample_rate(), 8000.0);
        assert!(matches!(
            subject.preparation_status().last_refusal,
            Some(PreparationRefusal {
                error: PreparationError::ReservationFailed { .. },
                ..
            })
        ));
        let (mut sl, mut sr) = input(101);
        let (mut rl, mut rr) = (sl, sr);
        subject.process(&mut sl, &mut sr);
        reference.process(&mut rl, &mut rr);
        assert_eq!(sl, rl);
        assert_eq!(sr, rr);
        let refusal = subject.preparation_status();
        subject.reset();
        reference.reset();
        assert_eq!(subject.preparation_status(), refusal);
        let (mut sl, mut sr) = input(207);
        let (mut rl, mut rr) = (sl, sr);
        subject.process(&mut sl, &mut sr);
        reference.process(&mut rl, &mut rr);
        assert_eq!(sl, rl);
        assert_eq!(sr, rr);
        assert_eq!(
            subject.try_prepare(1.0, 1, PreparationBudget::UNLIMITED),
            Ok(())
        );
        assert_eq!(
            subject.preparation_status(),
            PreparationStatus {
                is_prepared: true,
                last_refusal: None
            }
        );
        assert_eq!(subject.params(), p.sanitized());
    }};
}

#[test]
fn bank_every_reservation_failure_rolls_back_live_histories_filters_and_ramps() {
    rollback!(
        EchoBank,
        bank_params(),
        16,
        |p: &mut EchoBankParams, failed: usize| {
            p.wet = if failed.is_multiple_of(2) { 0.4 } else { 0.8 };
            for (i, u) in p.units.iter_mut().enumerate() {
                u.time_ms = 2.0 + i as f32 + (failed % 3) as f32;
                u.filter.frequency_hz = 200.0 + 50.0 * failed as f32;
            }
        }
    );
}
#[test]
fn frequency_every_reservation_failure_rolls_back_live_splitter_bands_and_clock() {
    rollback!(
        FrequencyDelay,
        frequency_params(),
        32,
        |p: &mut FrequencyDelayParams, failed: usize| {
            p.bandwidth = if failed.is_multiple_of(2) { 0.7 } else { 1.3 };
            p.wet = if failed.is_multiple_of(2) { 0.3 } else { 0.8 };
            for (i, b) in p.bands.iter_mut().enumerate() {
                b.delay_ms = (i + 1 + failed % 3) as f32 * 1.37;
            }
        }
    );
}

#[test]
fn initial_refusal_is_visible_and_remains_unprepared_until_success() {
    macro_rules! initial {
        ($processor:ty) => {{
            let mut e = <$processor>::default();
            assert_eq!(
                e.preparation_status(),
                PreparationStatus {
                    is_prepared: false,
                    last_refusal: None
                }
            );
            let before = (e.actual_sample_rate(), e.params(), e.prepared_bytes());
            let fault = allocation_test::Fault::at(0);
            e.prepare(f32::NAN, 0);
            drop(fault);
            assert!(!e.preparation_status().is_prepared);
            assert_eq!(
                e.preparation_status().last_refusal.unwrap().sample_rate,
                48000.0
            );
            assert_eq!(
                (e.actual_sample_rate(), e.params(), e.prepared_bytes()),
                before
            );
            let (mut l, mut r) = ([0.3, f32::NAN, 2000.0], [-0.1, f32::INFINITY, -2000.0]);
            e.process(&mut l, &mut r);
            assert_eq!(l, [0.3, 0.0, 1000.0]);
            assert_eq!(r, [-0.1, 0.0, -1000.0]);
            assert!(e.preparation_status().last_refusal.is_some());
            e.try_prepare(1.0, 0, PreparationBudget::UNLIMITED).unwrap();
            assert!(e.preparation_status().is_prepared);
            assert_eq!(e.preparation_status().last_refusal, None);
        }};
    }
    initial!(EchoBank);
    initial!(FrequencyDelay);
}
