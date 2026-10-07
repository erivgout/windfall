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
