//! Utility effects through real routing, automation, edits and rendering.
use crate::support::{Rig, idle_limiter, impulse, left, level, peak, right, rms, run, sine};
use windfall_dsp::{DcBlockParams, DistortionParams, EffectKind, EffectParams, StereoMatrixParams};
use windfall_engine::{RenderOptions, render};
use windfall_ipc::PlayMode;
use windfall_project::{AutomationTarget, TrackId};

const RATE: u32 = 48_000;
const KINDS: [EffectKind; 7] = [
    EffectKind::Balance,
    EffectKind::DcBlock,
    EffectKind::ChannelMute,
    EffectKind::Polarity,
    EffectKind::StereoMatrix,
    EffectKind::SoftClipper,
    EffectKind::Distortion,
];

fn matrix(left_delay_ms: f32, right_delay_ms: f32) -> EffectParams {
    EffectParams::StereoMatrix(StereoMatrixParams {
        left_delay_ms,
        right_delay_ms,
        ..Default::default()
    })
}
fn configured(kind: EffectKind) -> EffectParams {
    if kind == EffectKind::StereoMatrix {
        matrix(2.0, 2.0)
    } else {
        kind.default_params()
    }
}

#[test]
fn utility_effects_r2_live_asymmetric_insertion_primes_both_outputs() {
    for block in [1, 137, 511] {
        let mut rig = Rig::new();
        let track = rig.track();
        let channel = rig.channel_on(level(RATE, 1.0, 1.0), track);
        rig.steps(channel, &[0]);
        let (mut processor, controller) = rig.processor(RATE);
        controller.play();
        run(&mut processor, 4092, block);
        rig.effect(track, matrix(0.0, 50.0));
        controller.set_project(&rig.project, &rig.pool);
        assert_eq!(controller.latency_frames(), 0);
        let audio = run(&mut processor, 4096, block);
        for (frame, sample) in right(&audio).iter().enumerate() {
            assert_eq!(
                *sample, 1.0,
                "unprimed live insertion at {frame}, block {block}"
            );
        }
    }
}

#[test]
fn utility_effects_r2_longer_delay_during_insertion_extends_output_priming() {
    let mut rig = Rig::new();
    let track = rig.track();
    let channel = rig.channel_on(level(RATE, 1.0, 1.0), track);
    rig.steps(channel, &[0]);
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    run(&mut processor, 4092, 137);
    let id = rig.effect(track, matrix(0.0, 5.0));
    controller.set_project(&rig.project, &rig.pool);
    let mut audio = run(&mut processor, 100, 7);
    rig.effect_mut(id).params = matrix(0.0, 50.0);
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 4000, 137));
    assert!(
        audio.iter().all(|sample| *sample == 1.0),
        "longer insertion target exposed an unprimed output"
    );
    assert_eq!(controller.latency_frames(), 0);
}

#[test]
fn utility_effects_r2_insertion_priming_extension_keeps_shared_pdc_in_step() {
    let mut rig = Rig::new();
    let (first, second) = (rig.track(), rig.track());
    let tone = sine(RATE, 173.0, 0.25);
    let up = rig.channel_on(tone.clone(), first);
    let down = rig.channel_on(crate::support::inverted(&tone), second);
    rig.steps(up, &[0]);
    rig.steps(down, &[0]);
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    run(&mut processor, 4092, 137);
    let id = rig.effect(first, matrix(0.0, 5.0));
    controller.set_project(&rig.project, &rig.pool);
    let mut audio = run(&mut processor, 100, 7);
    rig.effect_mut(id).params = matrix(2.0, 50.0);
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 4000, 137));
    assert!(
        peak(&left(&audio)) < 1e-6,
        "extended insertion wait desynchronized shared PDC"
    );
    assert!(
        peak(&right(&audio)) > 0.2,
        "the intentional stereo delay must eventually be heard"
    );
    assert_eq!(controller.latency_frames(), 96);
}

#[test]
fn utility_effects_r2_zero_to_delay_keeps_all_slot_policies_cancelling() {
    for (mix, enabled) in [(1.0, true), (0.5, true), (0.0, true), (1.0, false)] {
        let mut rig = Rig::new();
        let first = rig.track();
        let second = rig.track();
        let tone = sine(RATE, 173.0, 0.25);
        let up = rig.channel_on(tone.clone(), first);
        let down = rig.channel_on(crate::support::inverted(&tone), second);
        rig.steps(up, &[0]);
        rig.steps(down, &[0]);
        let effect = rig.effect(first, matrix(0.0, 0.0));
        rig.effect_mut(effect).mix = mix;
        rig.effect_mut(effect).enabled = enabled;
        let (mut processor, controller) = rig.processor(RATE);
        controller.play();
        let mut audio = run(&mut processor, 4092, 137);
        rig.effect_mut(effect).params = matrix(2.0, 2.0);
        controller.set_project(&rig.project, &rig.pool);
        assert_eq!(controller.latency_frames(), 96);
        audio.extend(run(&mut processor, 1000, 29));
        assert!(
            peak(&audio) < 1e-6,
            "zero-to-delay PDC residual {}, mix {mix}, enabled {enabled}",
            peak(&audio)
        );
    }
}

#[test]
fn utility_effects_r2_upstream_edit_reaches_pdc_after_the_downstream_delay() {
    let mut rig = Rig::new();
    let first = rig.track();
    let second = rig.track();
    let tone = sine(RATE, 500.0, 0.25);
    let up = rig.channel_on(tone.clone(), first);
    let down = rig.channel_on(crate::support::inverted(&tone), second);
    rig.steps(up, &[0]);
    rig.steps(down, &[0]);
    let upstream = rig.effect(first, matrix(1.0, 1.0));
    rig.effect(first, matrix(1.0, 1.0));
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 4092, 137);
    rig.effect_mut(upstream).params = matrix(2.0, 2.0);
    controller.set_project(&rig.project, &rig.pool);
    assert_eq!(controller.latency_frames(), 144);
    audio.extend(run(&mut processor, 1000, 29));
    assert!(
        peak(&audio) < 1e-6,
        "downstream PDC residual {}",
        peak(&audio)
    );
}

#[test]
fn utility_effects_r2_overlapping_serial_edits_cancel_across_tracks_and_blocks() {
    for routed in [false, true] {
        for (mix, enabled) in [(1.0, true), (0.5, true), (0.0, true), (1.0, false)] {
            let mut rig = Rig::new();
            let (first, second, bus) = (rig.track(), rig.track(), rig.track());
            if routed {
                rig.track_mut(first).output = Some(bus);
            }
            let tone = sine(RATE, 500.0, 0.25);
            let up = rig.channel_on(tone.clone(), first);
            let down = rig.channel_on(crate::support::inverted(&tone), second);
            rig.steps(up, &[0]);
            rig.steps(down, &[0]);
            let upstream = rig.effect(first, matrix(1.0, 1.0));
            let downstream = rig.effect(if routed { bus } else { first }, matrix(1.0, 1.0));
            for id in [upstream, downstream] {
                rig.effect_mut(id).mix = mix;
                rig.effect_mut(id).enabled = enabled;
            }
            let (mut processor, controller) = rig.processor(RATE);
            controller.play();
            let mut audio = run(&mut processor, 4092, 137);
            for (a, b, frames, block) in [
                (2.0, 1.0, 120, 29),
                (0.5, 2.0, 37, 1),
                (1.0, 0.0, 17, 7),
                (0.0, 0.0, 61, 3),
                (0.75, 1.5, 3, 1),
                (2.0, 3.0, 1000, 137),
            ] {
                rig.effect_mut(upstream).params = matrix(a, a);
                rig.effect_mut(downstream).params = matrix(b, b);
                controller.set_project(&rig.project, &rig.pool);
                assert_eq!(controller.latency_frames(), ((a + b) * 48.0).round() as u32);
                audio.extend(run(&mut processor, frames, block));
            }
            assert!(
                peak(&audio) < 1e-6,
                "overlapping serial PDC residual {}, routed {routed}, mix {mix}, enabled {enabled}",
                peak(&audio)
            );
            // Cancellation must come from matching audible transfers. The
            // settled chain itself still produces the requested 5 ms delay.
            rig.track_mut(second).muted = true;
            controller.set_project(&rig.project, &rig.pool);
            let mut solo = run(&mut processor, 1000, 29);
            assert!(peak(&solo) > 0.2);
            controller.stop();
            run(&mut processor, 1000, 7);
            controller.seek(0.0);
            controller.play();
            solo = run(&mut processor, 1000, 137);
            let reference = rig.play(RATE, 1000, 1);
            assert_eq!(
                solo, reference,
                "reset/replay must reuse the final settled chain"
            );
        }
    }
}

#[test]
fn utility_effects_r2_matrix_edits_stay_aligned_through_a_fixed_limiter() {
    let mut rig = Rig::new();
    let (first, second) = (rig.track(), rig.track());
    let tone = sine(RATE, 500.0, 0.25);
    let up = rig.channel_on(tone.clone(), first);
    let down = rig.channel_on(crate::support::inverted(&tone), second);
    rig.steps(up, &[0]);
    rig.steps(down, &[0]);
    let matrix_id = rig.effect(first, matrix(0.0, 0.0));
    rig.effect(first, idle_limiter(1.0));
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 4092, 137);
    for (ms, frames) in [(2.0, 120), (0.5, 37), (1.0, 1000)] {
        rig.effect_mut(matrix_id).params = matrix(ms, ms);
        controller.set_project(&rig.project, &rig.pool);
        audio.extend(run(&mut processor, frames, 29));
    }
    assert!(
        peak(&audio) < 1e-6,
        "limiter downstream PDC residual {}",
        peak(&audio)
    );
}

#[test]
fn utility_effects_r2_matrix_splices_preserve_existing_fixed_delay_history() {
    for place in 0..=2 {
        let mut rig = Rig::new();
        let (first, second) = (rig.track(), rig.track());
        let tone = sine(RATE, 173.0, 0.25);
        let up = rig.channel_on(tone.clone(), first);
        let down = rig.channel_on(crate::support::inverted(&tone), second);
        rig.steps(up, &[0]);
        rig.steps(down, &[0]);
        rig.effect(first, idle_limiter(1.0));
        rig.effect(first, idle_limiter(1.0));
        let (mut processor, controller) = rig.processor(RATE);
        controller.play();
        let mut audio = run(&mut processor, 4092, 137);
        let matrix_id = rig.effect(first, matrix(2.0, 2.0));
        let slot = rig.remove_effect(matrix_id);
        rig.track_mut(first).effects.insert(place, slot);
        controller.set_project(&rig.project, &rig.pool);
        audio.extend(run(&mut processor, 1000, 29));
        assert!(
            peak(&audio) < 1e-6,
            "matrix insertion at {place} residual {}",
            peak(&audio)
        );
        assert_eq!(controller.latency_frames(), 192);
        rig.remove_effect(matrix_id);
        controller.set_project(&rig.project, &rig.pool);
        audio.extend(run(&mut processor, 1000, 7));
        assert!(
            peak(&audio) < 1e-6,
            "matrix removal at {place} residual {}",
            peak(&audio)
        );
        assert_eq!(controller.latency_frames(), 96);
    }
}

#[test]
fn utility_effects_r2_sampler_matrix_and_hosted_delay_keep_pdc_and_plugin_ownership() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use windfall_engine::plugins::{HostedEffect, HostedInstrument, PluginFactory};
    use windfall_project::{PluginBinding, PluginTarget};

    struct HostedDelay {
        ring: [[f32; 2]; 32],
        write: usize,
    }
    impl HostedEffect for HostedDelay {
        fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
            for (left, right) in left.iter_mut().zip(right) {
                let old = self.ring[self.write];
                self.ring[self.write] = [*left, *right];
                self.write = (self.write + 1) % 32;
                [*left, *right] = old;
            }
        }
        fn set_param(&mut self, _: u32, _: f32) {}
        fn set_tempo(&mut self, _: f32) {}
        fn latency(&self) -> usize {
            32
        }
        fn tail(&self) -> usize {
            32
        }
    }
    #[derive(Debug)]
    struct Factory(AtomicUsize);
    impl PluginFactory for Factory {
        fn effect(
            &self,
            _: &PluginBinding,
            _: u32,
            _: usize,
        ) -> Result<Box<dyn HostedEffect>, String> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Ok(Box::new(HostedDelay {
                ring: [[0.0; 2]; 32],
                write: 0,
            }))
        }
        fn instrument(
            &self,
            _: &PluginBinding,
            _: u32,
            _: usize,
        ) -> Result<Box<dyn HostedInstrument>, String> {
            Err("effect fixture".into())
        }
    }
    for enabled in [true, false] {
        let mut rig = Rig::new();
        let (first, second) = (rig.track(), rig.track());
        let tone = sine(RATE, 173.0, 0.25);
        let up = rig.channel_on(tone.clone(), first);
        let down = rig.channel_on(crate::support::inverted(&tone), second);
        rig.steps(up, &[0]);
        rig.steps(down, &[0]);
        let matrix_id = rig.effect(first, matrix(0.0, 0.0));
        rig.effect_mut(matrix_id).mix = 0.5;
        let plugin = rig.effect(first, EffectKind::Balance.default_params());
        rig.effect_mut(plugin).enabled = enabled;
        rig.project.plugins.push(PluginBinding {
            target: PluginTarget::Effect { effect: plugin },
            format: "clap".into(),
            path: "prepared-delay-fixture.clap".into(),
            id: "utility-delay".into(),
            name: "Prepared delay".into(),
            state: vec![],
            parameters: vec![],
        });
        let factory = Arc::new(Factory(AtomicUsize::new(0)));
        rig.pool.set_plugin_factory(factory.clone());
        let (mut processor, controller) = rig.processor(RATE);
        controller.play();
        let checked = |processor: &mut windfall_engine::Processor, frames: usize, block: usize| {
            let mut out = [0.0; 137 * 2];
            let mut maximum = 0.0_f32;
            let mut remaining = frames;
            while remaining > 0 {
                let count = remaining.min(block);
                let audio = &mut out[..count * 2];
                assert_eq!(
                    crate::realtime::allocator_calls(|| processor.process(audio)),
                    0
                );
                maximum = maximum.max(peak(audio));
                remaining -= count;
            }
            maximum
        };
        let mut residual = checked(&mut processor, 4092, 137);
        for (ms, frames, block) in [(2.0, 120, 29), (0.5, 37, 1), (1.0, 1000, 137)] {
            rig.effect_mut(matrix_id).params = matrix(ms, ms);
            controller.set_project(&rig.project, &rig.pool);
            assert_eq!(controller.latency_frames(), (ms * 48.0).round() as u32 + 32);
            residual = residual.max(checked(&mut processor, frames, block));
            controller.frame();
        }
        assert!(
            residual < 1e-6,
            "hosted delay PDC residual {residual}, enabled {enabled}"
        );
        assert_eq!(
            factory.0.load(Ordering::Relaxed),
            1,
            "delay edits must reuse the plugin"
        );
    }
}

#[test]
fn utility_effects_r2_causal_history_edits_splices_and_retirement_never_use_the_allocator() {
    let mut rig = Rig::new();
    let (first, second, bus) = (rig.track(), rig.track(), rig.track());
    rig.track_mut(first).output = Some(bus);
    let tone = sine(RATE, 500.0, 1.0);
    let up = rig.channel_on(tone.clone(), first);
    let down = rig.channel_on(crate::support::inverted(&tone), second);
    rig.steps(up, &[0]);
    rig.steps(down, &[0]);
    let upstream = rig.effect(first, matrix(0.0, 0.0));
    let downstream = rig.effect(bus, matrix(0.0, 0.0));
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut out = [0.0; 1024 * 2];
    let mut calls = 0;
    for _ in 0..5 {
        calls += crate::realtime::allocator_calls(|| processor.process(&mut out));
    }
    let mut inserted = None;
    for edit in 0..80 {
        for (index, id) in [upstream, downstream].into_iter().enumerate() {
            let delay = ((edit * 37 + index * 17) % 2401) as f32 / 48.0;
            rig.effect_mut(id).params = matrix(delay, delay);
            rig.effect_mut(id).mix = [0.0, 0.5, 1.0][edit % 3];
            rig.effect_mut(id).enabled = edit % 5 != 0;
        }
        if edit == 5 {
            inserted = Some(rig.effect(first, matrix(0.0, 50.0)));
        }
        if edit == 45 {
            rig.remove_effect(inserted.take().unwrap());
        }
        controller.set_project(&rig.project, &rig.pool);
        if edit == 20 {
            controller.stop();
        }
        if edit == 21 {
            controller.seek(0.0);
            controller.play();
        }
        let frames = [1, 7, 137, 511, 1024][edit % 5];
        calls += crate::realtime::allocator_calls(|| processor.process(&mut out[..frames * 2]));
        assert!(out[..frames * 2].iter().all(|sample| sample.is_finite()));
        // Retired plans and their prepared paths are collected on the control side.
        controller.frame();
    }
    rig.remove_effect(upstream);
    rig.remove_effect(downstream);
    controller.set_project(&rig.project, &rig.pool);
    for _ in 0..5 {
        calls += crate::realtime::allocator_calls(|| processor.process(&mut out));
    }
    controller.frame();
    controller.set_project(&rig.project, &rig.pool);
    calls += crate::realtime::allocator_calls(|| processor.process(&mut out));
    assert_eq!(
        calls, 0,
        "causal compensation touched alloc/realloc/free on the callback"
    );
}

#[test]
fn utility_effects_before_and_after_limiters_match_rendering_with_automatic_pdc() {
    for kind in KINDS {
        for utility_first in [true, false] {
            let mut rig = Rig::new();
            let track = rig.track();
            let channel = rig.channel_on(sine(RATE, 173.0, 0.3), track);
            rig.steps(channel, &[0]);
            let utility = configured(kind);
            let limiter = idle_limiter(1.0);
            for params in if utility_first {
                [utility, limiter]
            } else {
                [limiter, utility]
            } {
                rig.effect(track, params);
            }
            let lane = rig.playlist_track();
            let pattern = rig.first_pattern();
            rig.clip(lane, pattern, 0, 1920);
            let (_, controller) = rig.processor(RATE);
            let latency = 48 + utility.latency_samples(RATE as f32);
            assert_eq!(controller.latency_frames() as usize, latency, "{kind:?}");
            let options = RenderOptions {
                mode: PlayMode::Song,
                tail_secs: 0.05,
                block_frames: 97,
                ..Default::default()
            };
            let exported = render(&rig.project, &rig.pool, &options, &mut |_| true);
            let played = rig.play_song(RATE, exported.frames() + latency, 137);
            assert_eq!(
                exported.samples(),
                &played[latency * 2..],
                "{kind:?}/{utility_first}"
            );
            assert!(peak(exported.samples()) > 0.01);
            let options = RenderOptions {
                block_frames: 511,
                ..options
            };
            assert_eq!(
                exported.samples(),
                render(&rig.project, &rig.pool, &options, &mut |_| true).samples()
            );
        }
    }
}

#[test]
fn utility_effects_matrix_common_delay_aligns_plain_tracks_and_keeps_stereo_offset() {
    let mut rig = Rig::new();
    let track = rig.track();
    let plain = rig.track();
    let delayed = rig.channel_on(impulse(RATE), track);
    let direct = rig.channel_on(impulse(RATE), plain);
    rig.channel_mut(delayed).volume = 0.25;
    rig.channel_mut(direct).volume = 0.5;
    rig.steps(delayed, &[1]);
    rig.steps(direct, &[1]);
    rig.effect(track, matrix(2.0, 5.0));
    let (_, controller) = rig.processor(RATE);
    assert_eq!(controller.latency_frames(), 96);
    let audio = rig.play(RATE, 7000, 137);
    let (l, r) = (left(&audio), right(&audio));
    assert_eq!(l[6096], 0.75);
    assert_eq!(r[6096], 0.5);
    assert_eq!(r[6240], 0.25);
    assert_eq!(l.iter().filter(|s| **s != 0.0).count(), 1);
    assert_eq!(r.iter().filter(|s| **s != 0.0).count(), 2);
}

#[test]
fn utility_effects_matrix_latency_edits_keep_opposite_tracks_cancelling() {
    let mut rig = Rig::new();
    let first = rig.track();
    let second = rig.track();
    let tone = sine(RATE, 173.0, 1.0);
    let other = crate::support::inverted(&tone);
    let up = rig.channel_on(tone, first);
    let down = rig.channel_on(other, second);
    rig.steps(up, &[0]);
    rig.steps(down, &[0]);
    let effect = rig.effect(first, matrix(2.0, 2.0));
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 3000, 137);
    for ms in [5.0, 0.0, 50.0, 1.0] {
        rig.effect_mut(effect).params = matrix(ms, ms);
        controller.set_project(&rig.project, &rig.pool);
        assert_eq!(controller.latency_frames(), (ms * 48.0).round() as u32);
        audio.extend(run(&mut processor, 3000, 137));
    }
    assert!(peak(&audio) < 1e-6, "PDC diverged: {}", peak(&audio));
}

#[test]
fn utility_effects_rapid_matrix_latency_edits_keep_dry_wet_and_pdc_in_step() {
    for mix in [0.0, 0.5, 1.0] {
        let mut rig = Rig::new();
        let first = rig.track();
        let second = rig.track();
        let tone = sine(RATE, 1000.0, 1.0);
        let other = crate::support::inverted(&tone);
        let up = rig.channel_on(tone, first);
        let down = rig.channel_on(other, second);
        rig.steps(up, &[0]);
        rig.steps(down, &[0]);
        let effect = rig.effect(first, matrix(2.0, 2.0));
        rig.effect_mut(effect).mix = mix;
        let (mut processor, controller) = rig.processor(RATE);
        controller.play();
        let mut audio = run(&mut processor, 4092, 137);
        for (ms, frames, block) in [
            (0.5, 120, 29),
            (1.0, 37, 1),
            (0.25, 17, 7),
            (1.5, 61, 3),
            (0.75, 1000, 137),
        ] {
            rig.effect_mut(effect).params = matrix(ms, ms);
            controller.set_project(&rig.project, &rig.pool);
            assert_eq!(controller.latency_frames(), (ms * 48.0).round() as u32);
            audio.extend(run(&mut processor, frames, block));
        }
        assert!(
            peak(&audio) < 1e-6,
            "rapid matrix PDC diverged at mix {mix}: {}",
            peak(&audio)
        );
    }
}

#[test]
fn utility_effects_matrix_and_distortion_report_128_samples_even_when_bypassed() {
    for enabled in [false, true] {
        let mut rig = Rig::new();
        let track = rig.track();
        let channel = rig.channel_on(impulse(RATE), track);
        rig.steps(channel, &[1]);
        for params in [
            matrix(2.0, 5.0),
            EffectParams::Distortion(DistortionParams::default()),
        ] {
            let effect = rig.effect(track, params);
            rig.effect_mut(effect).enabled = enabled;
        }
        let (_, controller) = rig.processor(RATE);
        assert_eq!(controller.latency_frames(), 128);
        if !enabled {
            let audio = rig.play(RATE, 7000, 137);
            assert_eq!(left(&audio)[6128], 1.0);
            assert_eq!(right(&audio)[6128], 1.0);
            assert_eq!(audio.iter().filter(|sample| **sample != 0.0).count(), 2);
        }
    }
}

#[test]
fn utility_effects_moves_preserve_processor_state_and_removal_retires_tails() {
    for kind in KINDS {
        let mut rig = Rig::new();
        let track = rig.track();
        let sample = if kind == EffectKind::DcBlock {
            impulse(RATE)
        } else {
            sine(RATE, 173.0, 1.0)
        };
        let channel = rig.channel_on(sample, track);
        rig.steps(channel, &[0]);
        let effect = rig.effect(track, configured(kind));
        let reference = rig.play(RATE, 12000, 128);
        let (mut processor, controller) = rig.processor(RATE);
        controller.play();
        let mut moved = run(&mut processor, 4000, 128);
        let slot = rig.remove_effect(effect);
        rig.track_mut(TrackId::MASTER).effects.push(slot);
        controller.set_project(&rig.project, &rig.pool);
        moved.extend(run(&mut processor, 8000, 128));
        // An effect moved to another track fades in for 5 ms. After that its
        // history is carried, including DC memory and both FIR delay lines.
        let settled = 4000 + 240 + configured(kind).latency_samples(RATE as f32);
        assert_eq!(
            &moved[settled * 2..],
            &reference[settled * 2..],
            "{kind:?} moved state"
        );
        rig.remove_effect(effect);
        controller.set_project(&rig.project, &rig.pool);
        let out = run(&mut processor, 4000, 128);
        assert!(out.iter().all(|x| x.is_finite()));
        controller.set_project(&rig.project, &rig.pool);
        let out = run(&mut processor, 1000, 128);
        assert_eq!(controller.latency_frames(), 0);
        if kind == EffectKind::DcBlock {
            assert_eq!(peak(&out), 0.0);
        } else {
            assert!(rms(&left(&out)) > 0.2);
        }
    }
}

#[test]
fn utility_effects_live_automation_changes_each_real_processor_and_exports_the_same_audio() {
    // Choose a non-latency setting for this test; common delay changes have
    // their own exact cancellation test above.
    for (kind, param, value) in [
        (EffectKind::Balance, 0, 0.0),
        (EffectKind::DcBlock, 0, 1.0),
        (EffectKind::ChannelMute, 0, 1.0),
        (EffectKind::Polarity, 0, 1.0),
        (EffectKind::StereoMatrix, 1, 0.5),
        (EffectKind::SoftClipper, 0, 0.0),
        (EffectKind::Distortion, 0, 1.0),
    ] {
        let mut rig = Rig::new();
        let track = rig.track();
        let channel = rig.channel_on(level(RATE, 0.5, 0.5), track);
        rig.steps(channel, &[0]);
        let effect = rig.effect(track, kind.default_params());
        let lane = rig.playlist_track();
        let pattern = rig.first_pattern();
        rig.clip(lane, pattern, 0, 960);
        let dry = rig.play_song(RATE, 24000, 137);
        let lane = rig.playlist_track();
        let automation = rig.automation(
            AutomationTarget::EffectParam {
                track,
                effect,
                param,
            },
            &[(0, value)],
        );
        rig.automation_clip(lane, automation, 0, 960);
        let options = RenderOptions {
            mode: PlayMode::Song,
            block_frames: 101,
            ..Default::default()
        };
        let exported = render(&rig.project, &rig.pool, &options, &mut |_| true);
        let (_, controller) = rig.processor(RATE);
        let latency = controller.latency_frames() as usize;
        let played = rig.play_song(RATE, exported.frames() + latency, 137);
        assert_eq!(
            exported.samples(),
            &played[latency * 2..],
            "{kind:?} automation parity"
        );
        // DC difference is most visible during the step decay, the others
        // after the control has settled. The effect must change actual audio.
        let range = if kind == EffectKind::DcBlock {
            250..1000
        } else {
            1000..8000
        };
        assert!(
            range
                .map(|i| (played[(i + latency) * 2] - dry[(i + latency) * 2]).abs())
                .fold(0.0_f32, f32::max)
                > 1e-3,
            "{kind:?} cosmetic automation"
        );
    }
}

#[test]
fn utility_effects_finite_tails_survive_auto_tail_rendering() {
    for params in [
        matrix(50.0, 10.0),
        EffectParams::DcBlock(DcBlockParams { cutoff_hz: 40.0 }),
        EffectParams::Distortion(DistortionParams::default()),
    ] {
        let mut rig = Rig::new();
        let track = rig.track();
        let lane = rig.playlist_track();
        rig.audio_clip(lane, level(RATE, 0.5, 0.002), track, 0, 4);
        rig.effect(track, params);
        let options = RenderOptions {
            mode: PlayMode::Song,
            tail_secs: 1.0,
            auto_tail: true,
            block_frames: 113,
            ..Default::default()
        };
        let exported = render(&rig.project, &rig.pool, &options, &mut |_| true);
        assert!(exported.frames() < 48025);
        assert!(exported.samples().iter().all(|x| x.is_finite()));
        assert!(peak(exported.samples()) > 1e-4, "{:?}", params.kind());
    }
}
