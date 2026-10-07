//! Actual CLAP fixtures through the desktop owner, session and export path.
use super::{Rig, SAMPLE_RATE, rms};
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};
use windfall_engine::plugins::{PluginFactory, PluginTransport};
use windfall_ipc::{BitDepth, ExportFormat, ExportOptions, PlayMode};
use windfall_project::{Command, EffectId, PluginTarget, Project, TrackId};

fn library() -> &'static Path {
    static LIBRARY: OnceLock<PathBuf> = OnceLock::new();
    LIBRARY.get_or_init(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let target = root.join("target/plugin-session-fixtures");
        let output = std::process::Command::new(env!("CARGO"))
            .args(["build", "--manifest-path"])
            .arg(root.join("crates/windfall-plugin-host/test-plugins/Cargo.toml"))
            .arg("--target-dir")
            .arg(&target)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = std::process::Command::new(env!("CARGO"))
            .args([
                "build",
                "-p",
                "windfall-desktop",
                "--bin",
                "windfall-desktop",
            ])
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        target.join("debug/windfall_test_plugins.dll")
    })
}
pub(super) fn manager(rig: &Rig) -> (std::sync::Arc<crate::plugins::PluginManager>, String) {
    let folder = rig.folder.path().join("plugins");
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("windfall-test.clap");
    std::fs::copy(library(), &path).unwrap();
    let scanner =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../target/debug/windfall-desktop.exe");
    assert!(scanner.is_file(), "desktop scanner helper must be built");
    let manager = crate::plugins::PluginManager::fixture(rig.folder.path(), &path, &scanner);
    rig.session.install_plugins(manager.clone());
    (manager, path.to_string_lossy().into_owned())
}
fn transport() -> PluginTransport {
    PluginTransport {
        playing: false,
        tempo_bpm: 120.0,
        position_beats: 0.0,
        position_seconds: 0.0,
        numerator: 4,
        denominator: 4,
    }
}

#[test]
fn native_session_instrument_effect_export_and_state_round_trip() {
    let mut rig = Rig::new();
    let (manager, path) = manager(&rig);
    let binding = manager
        .binding(
            &path,
            "org.windfall.test.sine",
            PluginTarget::Instrument {
                channel: windfall_project::ChannelId(0),
            },
        )
        .unwrap();
    let added = rig
        .session
        .dispatch(Command::AddPluginInstrument { plugin: binding }, None)
        .unwrap();
    let channel = windfall_project::ChannelId(added.created[0]);
    rig.session
        .dispatch(
            Command::SetPluginParam {
                target: PluginTarget::Instrument { channel },
                id: 1,
                value: 0.7,
            },
            None,
        )
        .unwrap();
    rig.session
        .dispatch(
            Command::ToggleStep {
                pattern: rig.pattern(),
                channel,
                step: 0,
            },
            None,
        )
        .unwrap();
    let binding = manager
        .binding(
            &path,
            "org.windfall.test.gain",
            PluginTarget::Effect {
                effect: EffectId(0),
            },
        )
        .unwrap();
    let added = rig
        .session
        .dispatch(
            Command::AddPluginEffect {
                track: TrackId(0),
                plugin: binding,
            },
            None,
        )
        .unwrap();
    let effect = EffectId(*added.created.last().unwrap());
    rig.session
        .dispatch(
            Command::SetPluginParam {
                target: PluginTarget::Effect { effect },
                id: 7,
                value: 0.5,
            },
            None,
        )
        .unwrap();
    rig.run(256); // Stopped buffers select installed playback instances.
    assert_eq!(rig.session.controller().latency_frames(), 32);
    rig.session.transport_play().unwrap();
    assert!(rms(&rig.run(12_000)) > 0.05);
    let captured = manager.runtime.capture(rig.project()).unwrap();
    assert!(
        captured
            .plugins
            .iter()
            .all(|plugin| plugin.state.starts_with(b"WFPS"))
    );
    let options = ExportOptions {
        path: rig.file("plugins.wav"),
        format: ExportFormat::Wav,
        bit_depth: BitDepth::Float32,
        sample_rate: SAMPLE_RATE,
        mode: PlayMode::Pattern,
        tail_secs: 0.0,
        ..Default::default()
    };
    rig.session.export_audio(options.clone()).unwrap();
    assert_eq!(rig.events.wait_for_export().error, None);
    assert!(
        rms(windfall_codec::decode_file(&options.path)
            .unwrap()
            .samples())
            > 0.01
    );
    assert_eq!(
        manager.runtime.capture(rig.project()).unwrap().plugins,
        captured.plugins,
        "export instances cannot replace playback state"
    );
    let saved = rig.file("plugins.windfall");
    rig.session.project_save(Some(&saved)).unwrap();
    rig.session.project_open(&saved).unwrap();
    rig.run(256);
    let reopened = rig.project();
    assert_eq!(reopened.plugins, captured.plugins);
    rig.session.transport_play().unwrap();
    assert!(rms(&rig.run(12_000)) > 0.05);

    let mut missing = reopened;
    let retained = missing
        .plugins
        .iter_mut()
        .find(|plugin| plugin.target == PluginTarget::Instrument { channel })
        .unwrap();
    retained.path = rig.file("missing.clap");
    let retained = retained.clone();
    let missing_file = rig.file("missing.windfall");
    windfall_project::file::save(&missing, Path::new(&missing_file)).unwrap();
    rig.session.project_open(&missing_file).unwrap();
    rig.session.transport_play().unwrap();
    let audio = rig.run(24_000);
    assert!(
        audio[audio.len() - 512..]
            .iter()
            .all(|sample| *sample == 0.0)
    );
    assert_eq!(
        rig.project().plugin(PluginTarget::Instrument { channel }),
        Some(&retained)
    );
    assert!(
        manager
            .state()
            .instances
            .iter()
            .any(|instance| instance.target == retained.target)
    );
}

#[test]
fn render_and_speculative_instances_cannot_take_editor_or_state_ownership() {
    let rig = Rig::new();
    let (manager, path) = manager(&rig);
    let target = PluginTarget::Effect {
        effect: EffectId(900),
    };
    let mut binding = manager
        .binding(&path, "org.windfall.test.gain", target)
        .unwrap();
    binding
        .parameters
        .iter_mut()
        .find(|param| param.id == 7)
        .unwrap()
        .value = 0.5;
    let mut first = manager.runtime.effect(&binding, SAMPLE_RATE, 256).unwrap();
    first.transport(transport());
    first.process(&mut [1.0; 64], &mut [1.0; 64]);
    let mut project = Project::new("Ownership");
    project.plugins = vec![binding.clone()];
    let first_state = manager.runtime.capture(project.clone()).unwrap().plugins[0]
        .state
        .clone();
    let mut changed = binding.clone();
    changed
        .parameters
        .iter_mut()
        .find(|param| param.id == 7)
        .unwrap()
        .value = 0.9;
    let mut speculative = manager.runtime.effect(&changed, SAMPLE_RATE, 256).unwrap();
    speculative.process(&mut [1.0; 64], &mut [1.0; 64]);
    let staged = manager.runtime.prepare_document();
    let mut rejected = staged.effect(&changed, SAMPLE_RATE, 256).unwrap();
    rejected.process(&mut [1.0; 64], &mut [1.0; 64]);
    assert!(
        manager
            .runtime
            .editor_binding(target, Some(binding.clone()), false)
            .is_ok(),
        "a refused staged replacement must leave the installed editor usable"
    );
    drop(rejected);
    drop(staged);
    assert_eq!(
        manager.runtime.capture(project.clone()).unwrap().plugins[0].state,
        first_state
    );
    let render = manager.runtime.render_factory().unwrap();
    let mut export = render.effect(&changed, SAMPLE_RATE, 256).unwrap();
    export.transport(transport());
    export.process(&mut [1.0; 64], &mut [1.0; 64]);
    assert_eq!(
        manager.runtime.capture(project.clone()).unwrap().plugins[0].state,
        first_state
    );
    manager.runtime.retry(); // A replacement document invalidates previous tokens.
    assert_eq!(
        manager.runtime.capture(project.clone()).unwrap().plugins[0].state,
        binding.state
    );
    assert!(manager.runtime.editor(target, true).is_err());
    let second_path = rig.folder.path().join("plugins/second.clap");
    std::fs::copy(library(), &second_path).unwrap();
    changed.path = second_path.to_string_lossy().into_owned();
    manager.runtime.approve(&[windfall_ipc::PluginEntry {
        path: changed.path.clone(),
        id: changed.id.clone(),
        format: "clap".into(),
        name: changed.name.clone(),
        vendor: "Fixture".into(),
        instrument: false,
        usable: true,
        error: None,
    }]);
    let mut replacement = manager.runtime.effect(&changed, SAMPLE_RATE, 256).unwrap();
    replacement.transport(transport());
    replacement.process(&mut [1.0; 64], &mut [1.0; 64]);
    assert_eq!(
        manager.runtime.capture(project).unwrap().plugins[0].state,
        binding.state,
        "equal numeric targets with different bindings cannot capture each other's state"
    );
    assert!(
        manager
            .runtime
            .editor_binding(target, Some(binding), false)
            .is_err()
    );
    let mut missing = changed.clone();
    missing.path = rig.file("missing.clap");
    let mut unavailable = manager.runtime.effect(&missing, SAMPLE_RATE, 256).unwrap();
    assert!(
        manager
            .runtime
            .editor_binding(target, Some(changed.clone()), false)
            .is_ok(),
        "speculative failure cannot deselect an installed instance"
    );
    unavailable.transport(transport());
    let mut left = [0.25; 64];
    let mut right = left;
    unavailable.process(&mut left, &mut right);
    assert_eq!(left, [0.25; 64], "missing effects bypass their input");
    assert!(
        manager
            .runtime
            .editor_binding(target, Some(changed.clone()), false)
            .is_err(),
        "an installed missing instance clears stale editor ownership"
    );
    let staged = manager.runtime.prepare_document();
    let mut accepted = staged.effect(&changed, SAMPLE_RATE, 256).unwrap();
    accepted.transport(transport());
    staged.install_document();
    assert!(
        manager
            .runtime
            .editor_binding(target, Some(changed), false)
            .is_ok(),
        "an accepted staged replacement publishes its own revision"
    );
}
