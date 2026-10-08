//! Published lo-fi controls through actual routing, automation and renderers.

use crate::support::{Rig, peak};
use windfall_core::AudioBuffer;
use windfall_dsp::{EffectParams, FilterPlacement, LofiParams, RunRelation};
use windfall_engine::{RenderOptions, StemMode, StemOptions, render, render_stems, stems};
use windfall_ipc::PlayMode;
use windfall_project::{AutomationTarget, EffectId, TrackId};

const RATE: u32 = 48_000;

fn tone() -> AudioBuffer {
    AudioBuffer::from_interleaved(
        RATE,
        2,
        (0..RATE)
            .flat_map(|n| {
                let phase = std::f64::consts::TAU * f64::from(n) / f64::from(RATE);
                [
                    (0.3 * (phase * 701.0).sin()) as f32,
                    (-0.2 * (phase * 997.0).sin()) as f32,
                ]
            })
            .collect(),
    )
}

fn fixture(audio: AudioBuffer, params: Option<LofiParams>) -> (Rig, TrackId, Option<EffectId>) {
    let mut rig = Rig::new();
    let track = rig.track();
    let channel = rig.channel_on(audio, track);
    rig.steps(channel, &[0]);
    let effect = params.map(|p| rig.effect(track, EffectParams::Lofi(p)));
    (rig, track, effect)
}

fn song(params: LofiParams) -> (Rig, TrackId, EffectId) {
    let (mut rig, track, effect) = fixture(tone(), Some(params));
    let lane = rig.playlist_track();
    rig.clip(lane, rig.first_pattern(), 0, 960);
    (rig, track, effect.unwrap())
}

fn guarded_run(
    processor: &mut windfall_engine::Processor,
    frames: usize,
    block: usize,
) -> Vec<f32> {
    let mut audio = vec![0.0; frames * 2];
    for out in audio.chunks_mut(block * 2) {
        assert_eq!(
            crate::realtime::allocator_calls(|| processor.process(out)),
            0
        );
    }
    audio
}

fn difference(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f32::max)
}

#[test]
fn routed_quantized_replaced_and_held_signal_matches_an_integer_reference() {
    let params = LofiParams {
        bits: 4,
        quantize: 1.0,
        rate_ratio: 4.0,
        preserve_ms: 0.125,
        replace_ms: 0.0625,
        replacement: 1.0,
        replacement_value: 0.75,
        ..LofiParams::neutral()
    };
    let (dry_rig, _, _) = fixture(tone(), None);
    let dry = dry_rig.play(RATE, 8192, 127);
    let expected: Vec<f32> = (0..8192)
        .flat_map(|n| {
            let captured = n / 4 * 4;
            // Independent integer clock: six preserved, three replaced frames.
            [0, 1].map(|ch| {
                if captured % 9 >= 6 {
                    0.75
                } else {
                    (f64::from(dry[captured * 2 + ch]).clamp(-1.0, 1.0) * 7.0).round() as f32 / 7.0
                }
            })
        })
        .collect();
    let (rig, _, _) = fixture(tone(), Some(params));
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let wet = guarded_run(&mut processor, 8192, 137);
    assert!(difference(&wet, &expected) < 1e-6);
    assert_eq!(controller.latency_frames(), 0);
    for block in [1, 7, 64, 512] {
        assert_eq!(wet, rig.play(RATE, 8192, block));
    }

    // Moving the same processor to a bus changes its owner, not its signal.
    let (mut bus_rig, source, _) = fixture(tone(), None);
    let bus = bus_rig.track();
    bus_rig.track_mut(source).output = Some(bus);
    bus_rig.effect(bus, EffectParams::Lofi(params));
    assert_eq!(wet, bus_rig.play(RATE, 8192, 29));
    let (mut master_rig, _, _) = fixture(tone(), None);
    master_rig.effect(TrackId::MASTER, EffectParams::Lofi(params));
    assert_eq!(wet, master_rig.play(RATE, 8192, 31));
}

#[test]
fn all_sixteen_control_curves_agree_live_offline_and_in_both_stem_modes() {
    let params = LofiParams {
        replacement: 0.75,
        replacement_value: 0.2,
        resonance: 0.7,
        run_relation: RunRelation::Double,
        placement: FilterPlacement::Pre,
        ..LofiParams::default()
    };
    let (mut rig, track, effect) = song(params);
    assert_eq!(EffectParams::Lofi(params).kind().descriptors().len(), 16);
    for param in 0..16 {
        let automation = rig.automation(
            AutomationTarget::EffectParam {
                track,
                effect,
                param,
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
    let live = guarded_run(&mut processor, exported.frames(), 137);
    assert_eq!(exported.samples(), live);
    assert_eq!(controller.latency_frames(), 0);
    assert!(live.iter().all(|x| x.is_finite()));
    for block in [1, 7, 64, 512] {
        assert_eq!(live, rig.play_song(RATE, exported.frames(), block));
    }
    let (plain, _, _) = song(params);
    let static_audio = render(&plain.project, &plain.pool, &options, &mut |_| true);
    assert!(difference(exported.samples(), static_audio.samples()) > 1e-3);
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
            &mut |index, block| {
                audio[index].extend_from_slice(block);
                true
            },
            &mut |_| true,
        )
        .unwrap();
        for samples in audio {
            assert_eq!(samples, exported.samples(), "{mode:?}");
        }
    }
}

#[test]
fn live_noop_edits_bypass_mix_removal_and_restore_have_no_callback_allocator_calls() {
    let (mut rig, track, effect) = fixture(tone(), Some(LofiParams::default()));
    let effect = effect.unwrap();
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    guarded_run(&mut processor, 4096, 137);
    controller.set_project(&rig.project, &rig.pool);
    let unchanged = guarded_run(&mut processor, 1024, 29);
    let reference = rig.play(RATE, 5120, 31);
    assert_eq!(unchanged, reference[8192..]);
    for (index, info) in EffectParams::Lofi(LofiParams::default())
        .kind()
        .descriptors()
        .iter()
        .enumerate()
    {
        rig.effect_mut(effect).params.set(index, info.max);
        controller.set_project(&rig.project, &rig.pool);
        assert!(
            guarded_run(&mut processor, 257, 7)
                .iter()
                .all(|x| x.is_finite())
        );
    }
    rig.effect_mut(effect).enabled = false;
    controller.set_project(&rig.project, &rig.pool);
    guarded_run(&mut processor, 1024, 113);
    rig.effect_mut(effect).enabled = true;
    rig.effect_mut(effect).mix = 0.0;
    controller.set_project(&rig.project, &rig.pool);
    guarded_run(&mut processor, 1024, 1);
    rig.effect_mut(effect).mix = 1.0;
    controller.set_project(&rig.project, &rig.pool);
    guarded_run(&mut processor, 1024, 137);
    let slot = rig.remove_effect(effect);
    controller.set_project(&rig.project, &rig.pool);
    let start = 5120 + 16 * 257 + 3 * 1024;
    let removed = guarded_run(&mut processor, 4096, 137);
    let (dry, _, _) = fixture(tone(), None);
    let dry = dry.play(RATE, start + 4096, 101);
    assert_eq!(&removed[2048 * 2..], &dry[(start + 2048) * 2..]);
    rig.track_mut(track).effects.push(slot);
    controller.set_project(&rig.project, &rig.pool);
    let restored = guarded_run(&mut processor, 4096, 31);
    assert!(restored.iter().all(|x| x.is_finite()));
    assert!(difference(&restored[2048 * 2..], &dry[(start + 2048) * 2..]) > 1e-3);
    assert_eq!(controller.latency_frames(), 0);
}

#[test]
fn finite_replacement_and_resonant_ringout_survive_automatic_song_tail() {
    for placement in [FilterPlacement::Pre, FilterPlacement::Post] {
        let mut rig = Rig::new();
        let track = rig.track();
        let lane = rig.playlist_track();
        let mut samples = vec![0.0; 24_000 * 2];
        samples[23_999 * 2] = 0.5;
        rig.audio_clip(
            lane,
            AudioBuffer::from_interleaved(RATE, 2, samples),
            track,
            0,
            960,
        );
        rig.effect(
            track,
            EffectParams::Lofi(LofiParams {
                preserve_ms: 1.0,
                replace_ms: 2.0,
                replacement: 1.0,
                replacement_value: 0.3,
                filter: 1.0,
                resonance: 1.0,
                cutoff_hz: 1000.0,
                placement,
                ..LofiParams::neutral()
            }),
        );
        let options = RenderOptions {
            mode: PlayMode::Song,
            tail_secs: 1.0,
            auto_tail: true,
            block_frames: 101,
            ..Default::default()
        };
        let exported = render(&rig.project, &rig.pool, &options, &mut |_| true);
        assert!(exported.frames() > 24_000 && exported.frames() <= 72_000);
        assert!(peak(&exported.samples()[48_000..]) > 0.1);
        assert!(peak(&exported.samples()[50_000..]) < 1e-6);
        assert_eq!(
            exported.samples(),
            rig.play_song(RATE, exported.frames(), 137)
        );
    }
}
