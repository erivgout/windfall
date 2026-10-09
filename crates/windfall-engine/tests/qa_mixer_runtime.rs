#[allow(dead_code)]
#[path = "engine/support.rs"]
mod support;

use support::{Rig, run};
use windfall_engine::{RenderOptions, render};
use windfall_ipc::{MetronomeSettings, PlayMode, TransportPatch};

fn silent_rig() -> Rig {
    let mut rig = Rig::new();
    rig.project.settings.tempo_bpm = 120.0;
    rig.project.settings.time_signature.numerator = 3;
    rig.project.settings.time_signature.denominator = 8;
    let lane = rig.playlist_track();
    let pattern = rig.first_pattern();
    rig.clip(lane, pattern, 0, 3_840);
    rig
}

fn live_clicks(mode: PlayMode, block: usize) -> Vec<f32> {
    let rig = silent_rig();
    let (mut processor, controller) = rig.processor(8_000);
    controller.set_transport(TransportPatch {
        mode: Some(mode),
        metronome: Some(MetronomeSettings {
            enabled: true,
            gain: 0.5,
            accent: true,
        }),
        ..Default::default()
    });
    controller.play();
    run(&mut processor, 8_000, block)
}

#[test]
fn qa_live_metronome_song_pattern_meter_and_block_invariance() {
    for mode in [PlayMode::Pattern, PlayMode::Song] {
        let audio = live_clicks(mode, 64);
        assert_eq!(audio, live_clicks(mode, 127));
        for beat in [0, 2_000, 4_000, 6_000] {
            assert!(
                audio[(beat + 1) * 2].abs() > 0.0,
                "missing {mode:?} beat {beat}"
            );
            assert!(
                audio[(beat + 200) * 2..(beat + 1_990) * 2]
                    .iter()
                    .all(|sample| *sample == 0.0)
            );
        }
        assert_eq!(&audio[2..400], &audio[12_002..12_400]);
        assert_ne!(&audio[2..400], &audio[4_002..4_400]);
    }
}

#[test]
fn qa_count_in_holds_playhead_then_starts_and_stop_cancels() {
    let rig = silent_rig();
    let (mut processor, controller) = rig.processor(8_000);
    controller.play_count_in(1);
    let preroll = run(&mut processor, 5_999, 127);
    assert!(preroll.iter().any(|sample| *sample != 0.0));
    assert_eq!(controller.count_in_remaining(), 1);
    assert_eq!(controller.frame().tick, 0.0);
    run(&mut processor, 1, 1);
    assert_eq!(controller.count_in_remaining(), 0);
    assert!(controller.transport().playing);
    let after = run(&mut processor, 100, 64);
    assert!(after.iter().all(|sample| *sample == 0.0));
    controller.stop();
    run(&mut processor, 256, 64);
    controller.play_count_in(2);
    run(&mut processor, 100, 64);
    assert!(controller.count_in_remaining() > 0);
    controller.stop();
    run(&mut processor, 256, 64);
    assert_eq!(controller.count_in_remaining(), 0);
    assert!(!controller.transport().playing);
}

#[test]
fn qa_runtime_click_is_excluded_from_offline_project_render() {
    let rig = silent_rig();
    assert!(
        live_clicks(PlayMode::Pattern, 64)
            .iter()
            .any(|sample| *sample != 0.0)
    );
    for mode in [PlayMode::Pattern, PlayMode::Song] {
        let audio = render(
            &rig.project,
            &rig.pool,
            &RenderOptions {
                sample_rate: 8_000,
                mode,
                ..Default::default()
            },
            &mut |_| true,
        );
        assert!(audio.frames() > 0);
        assert!(audio.samples().iter().all(|sample| *sample == 0.0));
    }
}
