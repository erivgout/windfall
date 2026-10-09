//! Delay's new modulation knobs must affect native audio and survive history.
#[allow(dead_code)]
#[path = "engine/support.rs"]
mod support;

use support::{Rig, sine};
use windfall_dsp::{DelayParams, ParamSet};
use windfall_engine::{Processor, RenderOptions, render};
use windfall_project::*;

fn audio(project: &Project, rig: &Rig, block: usize) -> Vec<f32> {
    let result = render(
        project,
        &rig.pool,
        &RenderOptions {
            block_frames: block,
            ..Default::default()
        },
        &mut |_| true,
    );
    assert!(result.samples().iter().all(|sample| sample.is_finite()));
    result.samples().to_vec()
}

#[test]
fn delay_modulation_controls_change_audio_persist_and_match_live_offline() {
    let mut rig = Rig::new();
    rig.project.settings.tempo_bpm = 120.0;
    let channel = rig.channel(sine(48_000, 440.0, 2.0));
    rig.note(channel, 240, 1920);
    let mut doc = Document::new(rig.project.clone());
    let track = TrackId::MASTER;
    let effect = EffectId(
        doc.dispatch(
            Command::AddEffect {
                track,
                kind: EffectKind::Delay,
                index: None,
            },
            None,
        )
        .unwrap()
        .created[0],
    );
    doc.dispatch(
        Command::SetEffectParams {
            track,
            effect,
            params: EffectParams::Delay(DelayParams {
                sync: false,
                time_ms: 60.0,
                feedback: 0.3,
                saturation: 0.2,
                mix: 1.0,
                ..Default::default()
            }),
        },
        None,
    )
    .unwrap();
    let baseline = audio(doc.project(), &rig, 64);
    let index = |name: &str| {
        DelayParams::descriptors()
            .iter()
            .position(|info| info.id == name)
            .unwrap() as u32
    };
    doc.dispatch(
        Command::SetEffectParam {
            track,
            effect,
            param: index("modRateHz"),
            value: 2.0,
        },
        None,
    )
    .unwrap();
    // Rate alone with zero depth must preserve the legacy exact sound.
    assert_eq!(audio(doc.project(), &rig, 7), baseline);
    doc.dispatch(
        Command::SetEffectParam {
            track,
            effect,
            param: index("modDepthMs"),
            value: 5.0,
        },
        None,
    )
    .unwrap();
    let moving = audio(doc.project(), &rig, 7);
    let energy = |pcm: &[f32]| {
        pcm.iter()
            .map(|&sample| f64::from(sample).powi(2))
            .sum::<f64>()
    };
    let difference: Vec<_> = moving.iter().zip(&baseline).map(|(a, b)| a - b).collect();
    assert!(energy(&moving) > 1.0);
    assert!(
        energy(&difference) / energy(&baseline) > 0.01,
        "modulation must change the actual native echo"
    );
    doc.undo().unwrap();
    assert_eq!(audio(doc.project(), &rig, 127), baseline);
    doc.redo().unwrap();
    assert_eq!(audio(doc.project(), &rig, 1000), moving);
    let path = std::env::temp_dir().join(format!(
        "windfall-delay-qa-{}-{}.windfall",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    file::save(doc.project(), &path).unwrap();
    let loaded = file::load(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    let params = &loaded
        .mixer
        .tracks
        .iter()
        .find(|item| item.id == track)
        .unwrap()
        .effects
        .iter()
        .find(|item| item.id == effect)
        .unwrap()
        .params;
    let EffectParams::Delay(params) = params else {
        panic!("saved effect retained its type")
    };
    assert_eq!((params.mod_rate_hz, params.mod_depth_ms), (2.0, 5.0));
    assert_eq!(audio(&loaded, &rig, 256), moving);
    let (mut processor, controller) = Processor::new(48_000);
    controller.set_project(&loaded, &rig.pool);
    controller.play();
    let latency = controller.latency_frames() as usize;
    let mut live = vec![0.0; (36_000 + latency) * 2];
    for chunk in live.chunks_mut(128 * 2) {
        processor.process(chunk);
    }
    assert_eq!(
        &live[latency * 2..(latency + 36_000) * 2],
        &moving[..72_000]
    );
}
