//! Modulation through the real graph, automation, export and callback path.

use crate::support::Rig;
use windfall_core::AudioBuffer;
use windfall_dsp::{ChorusParams, EffectKind, EffectParams, FlangerParams, PhaserParams};
use windfall_engine::{RenderOptions, StemMode, StemOptions, render, render_stems, stems};
use windfall_ipc::PlayMode;
use windfall_project::{AutomationTarget, EffectId, TrackId};

const KINDS: [EffectKind; 3] = [EffectKind::Chorus, EffectKind::Flanger, EffectKind::Phaser];
const RATE: u32 = 48_000;

fn fixture(params: EffectParams) -> (Rig, TrackId, EffectId) {
    let mut rig = Rig::new();
    let track = rig.track();
    let data = (0..24_000)
        .flat_map(|n| {
            [
                (n as f32 * 0.08).sin() * 0.1,
                (n as f32 * 0.13).sin() * 0.07,
            ]
        })
        .collect();
    let channel = rig.channel_on(AudioBuffer::from_interleaved(RATE, 2, data), track);
    rig.steps(channel, &[0]);
    let effect = rig.effect(track, params);
    let lane = rig.playlist_track();
    let pattern = rig.first_pattern();
    rig.clip(lane, pattern, 0, 960);
    (rig, track, effect)
}

fn guarded(processor: &mut windfall_engine::Processor, frames: usize, block: usize) -> Vec<f32> {
    let mut audio = vec![0.0; frames * 2];
    for out in audio.chunks_mut(block * 2) {
        assert_eq!(
            crate::realtime::allocator_calls(|| processor.process(out)),
            0
        );
    }
    audio
}

#[test]
fn all_controls_automate_with_identical_live_offline_and_both_stems() {
    for kind in KINDS {
        let (mut rig, track, effect) = fixture(kind.default_params());
        for index in 0..kind.descriptors().len() {
            let automation = rig.automation(
                AutomationTarget::EffectParam {
                    track,
                    effect,
                    param: index as u32,
                },
                &[(0, 0.1), (240, 0.9), (480, 0.25), (720, 0.75), (960, 0.5)],
            );
            let lane = rig.playlist_track();
            rig.automation_clip(lane, automation, 0, 960);
        }
        let options = RenderOptions {
            mode: PlayMode::Song,
            block_frames: 101,
            ..Default::default()
        };
        let exported = render(&rig.project, &rig.pool, &options, &mut |_| true);
        let (mut processor, controller) = rig.song_processor(RATE);
        controller.play();
        assert_eq!(
            exported.samples(),
            guarded(&mut processor, exported.frames(), 137)
        );
        assert_eq!(
            exported.samples(),
            rig.play_song(RATE, exported.frames(), 1)
        );
        assert_eq!(controller.latency_frames(), 0);
        for mode in [StemMode::TrackOutputs, StemMode::ToMaster] {
            let stem_options = StemOptions {
                mode,
                tracks: Some(vec![track]),
                include_mix: true,
                numbered: false,
            };
            let list = stems(&rig.project, &stem_options).unwrap();
            let mut audio = vec![Vec::new(); list.len()];
            render_stems(
                &rig.project,
                &rig.pool,
                &options,
                &stem_options,
                &mut |i, b| {
                    audio[i].extend_from_slice(b);
                    true
                },
                &mut |_| true,
            )
            .unwrap();
            for audio in audio {
                assert_eq!(audio, exported.samples());
            }
        }
    }
}

fn difference(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f32::max)
}

#[test]
fn every_individual_automation_index_changes_routed_audio_and_is_partition_invariant() {
    for kind in KINDS {
        let (rig, _, _) = fixture(kind.default_params());
        let options = RenderOptions {
            mode: PlayMode::Song,
            ..Default::default()
        };
        let baseline = render(&rig.project, &rig.pool, &options, &mut |_| true);
        for index in 0..kind.descriptors().len() {
            let (mut rig, track, effect) = fixture(kind.default_params());
            let automation = rig.automation(
                AutomationTarget::EffectParam {
                    track,
                    effect,
                    param: index as u32,
                },
                &[(0, 0.1), (240, 0.9), (480, 0.2), (720, 0.8)],
            );
            let lane = rig.playlist_track();
            rig.automation_clip(lane, automation, 0, 960);
            let result = render(&rig.project, &rig.pool, &options, &mut |_| true);
            assert!(
                difference(result.samples(), baseline.samples()) > 1e-5,
                "{kind:?} index {index} is inert"
            );
            let (mut processor, controller) = rig.song_processor(RATE);
            controller.play();
            assert_eq!(
                result.samples(),
                guarded(&mut processor, result.frames(), 7)
            );
        }
    }
}

#[test]
fn routed_stereo_impulses_match_independent_delays_feedback_and_allpass_recurrence() {
    for params in [
        EffectParams::Chorus(ChorusParams {
            delay_ms: 1.0,
            depth_ms: 0.0,
            mix: 1.0,
            ..Default::default()
        }),
        EffectParams::Flanger(FlangerParams {
            delay_ms: 1.0,
            depth_ms: 0.0,
            feedback: -0.8,
            damping: 0.0,
            mix: 1.0,
            ..Default::default()
        }),
        EffectParams::Phaser(PhaserParams {
            min_hz: 1000.0,
            max_hz: 1000.0,
            rate_hz: 0.0,
            feedback: 0.0,
            mix: 1.0,
            ..Default::default()
        }),
    ] {
        let mut rig = Rig::new();
        let track = rig.track();
        let mut data = vec![0.0; 8192 * 2];
        data[512 * 2] = 0.1;
        data[901 * 2 + 1] = -0.05;
        let channel = rig.channel_on(AudioBuffer::from_interleaved(RATE, 2, data), track);
        rig.steps(channel, &[0]);
        let dry = rig.play(RATE, 8192, 113);
        rig.effect(track, params);
        let (mut processor, controller) = rig.processor(RATE);
        controller.play();
        let wet = guarded(&mut processor, 8192, 137);
        let mut expected = vec![0.0; dry.len()];
        match params {
            EffectParams::Chorus(_) => {
                expected[48 * 2..].copy_from_slice(&dry[..dry.len() - 48 * 2])
            }
            EffectParams::Flanger(p) => {
                for n in 48 * 2..dry.len() {
                    expected[n] = dry[n - 48 * 2] + p.feedback * expected[n - 48 * 2];
                }
            }
            EffectParams::Phaser(_) => {
                let g = (std::f64::consts::PI * 1000.0 / 48_000.0).tan();
                let a = (g - 1.0) / (g + 1.0);
                let mut x = [[0.0_f64; 6]; 2];
                let mut y = x;
                for (n, &sample) in dry.iter().enumerate() {
                    let channel = n % 2;
                    let mut v = f64::from(sample);
                    for stage in 0..6 {
                        let next = a * v + x[channel][stage] - a * y[channel][stage];
                        x[channel][stage] = v;
                        y[channel][stage] = next;
                        v = next;
                    }
                    expected[n] = v as f32;
                }
            }
            _ => unreachable!(),
        }
        assert!(
            difference(&wet, &expected) < 2e-6,
            "{params:?}: routed reference error"
        );
        assert_eq!(wet, rig.play(RATE, 8192, 1));
        assert!(wet[..512 * 2].iter().all(|v| *v == 0.0));
        assert!(
            wet.as_chunks::<2>()
                .0
                .iter()
                .take(901)
                .all(|frame| frame[1] == 0.0)
        );
    }
}

#[test]
fn live_edits_noops_bypass_removal_restore_and_tail_keep_callback_ownership() {
    for kind in KINDS {
        let params = kind.default_params();
        let (mut rig, track, effect) = fixture(params);
        let (mut processor, controller) = rig.song_processor(RATE);
        controller.play();
        guarded(&mut processor, 4096, 137);
        controller.set_project(&rig.project, &rig.pool);
        assert_eq!(
            guarded(&mut processor, 1024, 7),
            rig.play_song(RATE, 5120, 31)[4096 * 2..]
        );
        let mut changed = params;
        for (i, info) in kind.descriptors().iter().enumerate() {
            changed.set(i, info.min + 0.7 * (info.max - info.min));
        }
        rig.effect_mut(effect).params = changed;
        controller.set_project(&rig.project, &rig.pool);
        assert!(
            guarded(&mut processor, 4096, 1)
                .iter()
                .all(|v| v.is_finite())
        );
        rig.effect_mut(effect).enabled = false;
        controller.set_project(&rig.project, &rig.pool);
        guarded(&mut processor, 4096, 137);
        let mut dry_project = rig.project.clone();
        dry_project
            .mixer
            .tracks
            .iter_mut()
            .for_each(|t| t.effects.clear());
        let options = RenderOptions {
            mode: PlayMode::Song,
            ..Default::default()
        };
        let dry = render(&dry_project, &rig.pool, &options, &mut |_| true);
        assert_eq!(
            guarded(&mut processor, 1024, 31),
            dry.samples()[13312 * 2..14336 * 2]
        );
        rig.effect_mut(effect).enabled = true;
        controller.set_project(&rig.project, &rig.pool);
        guarded(&mut processor, 4096, 7);
        let slot = rig.remove_effect(effect);
        controller.set_project(&rig.project, &rig.pool);
        guarded(&mut processor, 1024, 1);
        rig.track_mut(track).effects.push(slot);
        controller.set_project(&rig.project, &rig.pool);
        let restored = guarded(&mut processor, 4096, 137);
        assert!(restored.iter().all(|v| v.is_finite()));
        // Existing engine seeks retain FX memory and phase. DSP reset has a
        // separate exact fresh-instance assertion; seeking must remain finite.
        controller.seek(0.0);
        let sought = guarded(&mut processor, 8192, 7);
        assert!(sought.iter().all(|v| v.is_finite()));
        assert_eq!(controller.latency_frames(), 0);
    }
}

#[test]
fn last_frame_tail_is_exported_and_live_matches_without_a_fixed_latency() {
    for kind in KINDS {
        let (mut rig, _, _) = fixture(kind.default_params());
        let sample = rig.project.samples[0].id;
        let mut data = vec![0.0; 24000 * 2];
        data[23999 * 2] = 0.1;
        rig.pool
            .insert(sample, AudioBuffer::from_interleaved(RATE, 2, data));
        let options = RenderOptions {
            mode: PlayMode::Song,
            tail_secs: 5.0,
            auto_tail: true,
            block_frames: 101,
            ..Default::default()
        };
        let audio = render(&rig.project, &rig.pool, &options, &mut |_| true);
        assert!(audio.frames() > 24000, "{kind:?} tail omitted");
        assert!(audio.samples()[48000..].iter().any(|v| v.abs() > 1e-5));
        let (mut processor, controller) = rig.song_processor(RATE);
        controller.play();
        assert_eq!(
            audio.samples(),
            guarded(&mut processor, audio.frames(), 137)
        );
        assert!(audio.samples().iter().all(|v| v.is_finite()));
    }
}
