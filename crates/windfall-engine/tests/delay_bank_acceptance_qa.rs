//! Registered bank/band controls must reach native playback, history and files.
#[allow(dead_code)]
#[path = "engine/support.rs"]
mod support;

use support::{Rig, sine};
use windfall_dsp::echo_bank::{EchoBankParams, EchoFilterMode};
use windfall_dsp::frequency_delay::FrequencyDelayParams;
use windfall_engine::{Processor, RenderOptions, render};
use windfall_project::*;

fn audio(project: &Project, rig: &Rig) -> Vec<f32> {
    let result = render(project, &rig.pool, &RenderOptions::default(), &mut |_| true);
    assert!(result.samples().iter().all(|sample| sample.is_finite()));
    assert!(result.samples().iter().any(|sample| sample.abs() > 0.001));
    result.samples().to_vec()
}

fn distinct(a: &[f32], b: &[f32]) {
    // Tail length may change with delay edits; the scheduled song is shared.
    let energy = a.iter().map(|&x| f64::from(x).powi(2)).sum::<f64>();
    let difference = a
        .iter()
        .zip(b)
        .map(|(&x, &y)| f64::from(x - y).powi(2))
        .sum::<f64>();
    assert!(difference / energy > 0.0001, "the native audio must change");
}

fn set(doc: &mut Document, effect: EffectId, name: &str, value: f32) {
    let kind = doc
        .project()
        .mixer
        .tracks
        .iter()
        .find(|track| track.id == TrackId::MASTER)
        .unwrap()
        .effects
        .iter()
        .find(|slot| slot.id == effect)
        .unwrap()
        .params
        .kind();
    let param = kind
        .descriptors()
        .iter()
        .position(|info| info.id == name)
        .unwrap() as u32;
    doc.dispatch(
        Command::SetEffectParam {
            track: TrackId::MASTER,
            effect,
            param,
            value,
        },
        None,
    )
    .unwrap();
}

fn fixture(params: EffectParams) -> (Rig, Document, EffectId) {
    let mut rig = Rig::new();
    for hz in [440.0, 12_000.0] {
        let channel = rig.channel(sine(48_000, hz, 1.0));
        rig.note(channel, 240, 1920);
    }
    let mut doc = Document::new(rig.project.clone());
    let effect = EffectId(
        doc.dispatch(
            Command::AddEffect {
                track: TrackId::MASTER,
                kind: params.kind(),
                index: None,
            },
            None,
        )
        .unwrap()
        .created[0],
    );
    doc.dispatch(
        Command::SetEffectParams {
            track: TrackId::MASTER,
            effect,
            params,
        },
        None,
    )
    .unwrap();
    (rig, doc, effect)
}

fn roundtrip_and_live(doc: &Document, rig: &Rig, expected: &[f32]) {
    let path = std::env::temp_dir().join(format!(
        "windfall-bank-qa-{}-{}.windfall",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    file::save(doc.project(), &path).unwrap();
    let loaded = file::load(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    let effects = |project: &Project| {
        project
            .mixer
            .tracks
            .iter()
            .find(|track| track.id == TrackId::MASTER)
            .unwrap()
            .effects
            .clone()
    };
    assert_eq!(effects(&loaded), effects(doc.project()));
    assert_eq!(audio(&loaded, rig), expected);
    for block in [7, 64, 127] {
        let (mut processor, controller) = Processor::new(48_000);
        controller.set_project(&loaded, &rig.pool);
        controller.play();
        let latency = controller.latency_frames() as usize;
        let mut live = vec![0.0; (36_000 + latency) * 2];
        for chunk in live.chunks_mut(block * 2) {
            processor.process(chunk);
        }
        assert_eq!(
            &live[latency * 2..(latency + 36_000) * 2],
            &expected[..72_000]
        );
    }
}

#[test]
fn echo_bank_filters_and_serial_unit_send_change_native_audio_and_survive_history() {
    let mut params = EchoBankParams {
        dry: 0.0,
        wet: 1.0,
        ..Default::default()
    };
    params.units[0].sync = false;
    params.units[0].time_ms = 25.0;
    params.units[0].feedback = 0.0;
    params.units[1].enabled = true;
    params.units[1].sync = false;
    params.units[1].time_ms = 40.0;
    params.units[1].feedback = 0.0;
    params.units[1].input_gain = 0.0;
    params.units[1].filter.mode = EchoFilterMode::Lowpass;
    params.units[1].filter.frequency_hz = 900.0;
    let (rig, mut doc, effect) = fixture(EffectParams::EchoBank(params));
    let parallel = audio(doc.project(), &rig);
    set(&mut doc, effect, "units.0.nextSend", 0.8);
    let linked = audio(doc.project(), &rig);
    distinct(&parallel, &linked);
    set(&mut doc, effect, "units.1.filter.frequencyHz", 6000.0);
    let filtered = audio(doc.project(), &rig);
    distinct(&linked, &filtered);
    doc.undo().unwrap();
    assert_eq!(audio(doc.project(), &rig), linked);
    doc.redo().unwrap();
    assert_eq!(audio(doc.project(), &rig), filtered);
    for unit in 0..8 {
        for suffix in ["enabled", "timeMs", "filter.frequencyHz", "nextSend"] {
            assert!(
                EffectKind::EchoBank
                    .descriptors()
                    .iter()
                    .any(|info| info.id == format!("units.{unit}.{suffix}"))
            );
        }
    }
    roundtrip_and_live(&doc, &rig, &filtered);
}

#[test]
fn frequency_delay_has_sixteen_independent_native_band_controls_and_file_roundtrip() {
    let (rig, mut doc, effect) =
        fixture(EffectParams::FrequencyDelay(FrequencyDelayParams::default()));
    let baseline = audio(doc.project(), &rig);
    for band in 0..16 {
        for suffix in ["delayMs", "level", "pan", "enabled"] {
            assert!(
                EffectKind::FrequencyDelay
                    .descriptors()
                    .iter()
                    .any(|info| info.id == format!("bands.{band}.{suffix}"))
            );
        }
    }
    set(&mut doc, effect, "bands.7.delayMs", 35.0);
    let delayed = audio(doc.project(), &rig);
    distinct(&baseline, &delayed);
    set(&mut doc, effect, "bands.15.level", 0.2);
    let attenuated = audio(doc.project(), &rig);
    distinct(&delayed, &attenuated);
    set(&mut doc, effect, "bands.15.pan", -1.0);
    let panned = audio(doc.project(), &rig);
    distinct(&attenuated, &panned);
    let params = &doc
        .project()
        .mixer
        .tracks
        .iter()
        .find(|track| track.id == TrackId::MASTER)
        .unwrap()
        .effects
        .iter()
        .find(|slot| slot.id == effect)
        .unwrap()
        .params;
    let EffectParams::FrequencyDelay(params) = params else {
        panic!("effect type retained")
    };
    assert_eq!(params.bands[7].delay_ms, 35.0);
    assert_eq!(params.bands[15].level, 0.2);
    assert_eq!(params.bands[0], FrequencyDelayParams::default().bands[0]);
    doc.undo().unwrap();
    assert_eq!(audio(doc.project(), &rig), attenuated);
    doc.redo().unwrap();
    assert_eq!(audio(doc.project(), &rig), panned);
    roundtrip_and_live(&doc, &rig, &panned);
}
