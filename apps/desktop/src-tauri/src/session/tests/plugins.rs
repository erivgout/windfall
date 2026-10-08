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
    let scanner = std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(
            || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../target"),
            PathBuf::from,
        )
        .join("debug/windfall-desktop.exe");
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
fn r4_deduplicated_clap_instrument_document_adoption() {
    r4_document_adoption(false, true);
}
#[test]
fn r4_deduplicated_vst3_instrument_document_adoption() {
    r4_document_adoption(true, true);
}
#[test]
fn r4_deduplicated_clap_effect_document_adoption() {
    r4_document_adoption(false, false);
}
#[test]
fn r4_deduplicated_vst3_effect_document_adoption() {
    r4_document_adoption(true, false);
}

fn r4_document_adoption(vst3: bool, instrument: bool) {
    use windfall_project::{
        AutomationId, AutomationPoint, AutomationTarget, ClipContent, ClipInit,
    };
    let mut rig = Rig::new();
    let (manager, binding, id) = if vst3 {
        let (runtime, binding) = crate::plugins::vst3_fixture(if instrument { 2 } else { 0 });
        let manager = crate::plugins::PluginManager::fixture_runtime(rig.folder.path(), runtime);
        rig.session.install_plugins(manager.clone());
        (manager, binding, 7)
    } else {
        let (manager, path) = manager(&rig);
        let binding = manager
            .binding(
                &path,
                if instrument {
                    "org.windfall.test.sine"
                } else {
                    "org.windfall.test.gain"
                },
                if instrument {
                    PluginTarget::Instrument {
                        channel: windfall_project::ChannelId(0),
                    }
                } else {
                    PluginTarget::Effect {
                        effect: EffectId(0),
                    }
                },
            )
            .unwrap();
        (manager, binding, if instrument { 1 } else { 7 })
    };
    let added = rig
        .session
        .dispatch(
            if instrument {
                Command::AddPluginInstrument { plugin: binding }
            } else {
                Command::AddPluginEffect {
                    track: TrackId(0),
                    plugin: binding,
                }
            },
            None,
        )
        .unwrap();
    let channel = if instrument {
        windfall_project::ChannelId(added.created[0])
    } else {
        windfall_project::ChannelId(
            rig.session
                .dispatch(
                    Command::AddChannel {
                        name: Some("Native effect source".into()),
                        sample: None,
                        instrument: Some(windfall_project::InstrumentKind::SubtractiveSynth),
                        index: None,
                        mixer_track: None,
                    },
                    None,
                )
                .unwrap()
                .created[0],
        )
    };
    let target = if instrument {
        PluginTarget::Instrument { channel }
    } else {
        PluginTarget::Effect {
            effect: EffectId(*added.created.last().unwrap()),
        }
    };
    let automation_target = if instrument {
        AutomationTarget::InstrumentParam { channel, param: 0 }
    } else {
        AutomationTarget::EffectParam {
            track: TrackId(0),
            effect: EffectId(*added.created.last().unwrap()),
            param: 0,
        }
    };
    let range = rig.project().automation_range(&automation_target).unwrap();
    rig.session
        .dispatch(
            Command::SetPluginParam {
                target,
                id,
                value: 0.5,
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
    let track = rig
        .session
        .dispatch(
            Command::AddPlaylistTrack {
                name: None,
                index: None,
            },
            None,
        )
        .unwrap()
        .created[0];
    let automation = rig
        .session
        .dispatch(
            Command::AddAutomation {
                name: None,
                target: automation_target,
                points: Some(vec![
                    AutomationPoint {
                        tick: 0,
                        value: range.normalized(0.75),
                        curve: 0.0,
                        hold: true,
                    },
                    AutomationPoint {
                        tick: 96,
                        value: range.normalized(0.25),
                        curve: 0.0,
                        hold: true,
                    },
                ]),
            },
            None,
        )
        .unwrap()
        .created[0];
    let track = windfall_project::PlaylistTrackId(track);
    rig.session
        .dispatch(
            Command::AddClips {
                clips: vec![
                    ClipInit {
                        track,
                        start: 0,
                        length: Some(960),
                        offset: None,
                        muted: None,
                        content: ClipContent::Pattern {
                            pattern: rig.pattern(),
                        },
                    },
                    ClipInit {
                        track,
                        start: 0,
                        length: Some(960),
                        offset: None,
                        muted: None,
                        content: ClipContent::Automation {
                            automation: AutomationId(automation),
                        },
                    },
                ],
            },
            None,
        )
        .unwrap();
    rig.session
        .transport_set(windfall_ipc::TransportPatch {
            mode: Some(PlayMode::Song),
            ..Default::default()
        })
        .unwrap();
    rig.session.transport_play().unwrap();
    let mut out = [0.0; 960];
    assert_eq!(
        crate::test_alloc::allocator_calls(|| rig.processor.process(&mut out)),
        0
    );
    assert_eq!(manager.runtime.fixture_parameter(target, id), Some(0.75));
    let token = manager.runtime.selected_token(target).unwrap();
    rig.session
        .dispatch(
            Command::SetPluginParam {
                target,
                id,
                value: 0.75,
            },
            None,
        )
        .unwrap();
    // The installed value equals the automation cache: apply_plugin must
    // adopt its document generation even though set_param is deduplicated.
    assert_eq!(
        crate::test_alloc::allocator_calls(|| rig.processor.process(&mut [])),
        0
    );
    assert_eq!(
        crate::test_alloc::allocator_calls(|| {
            for _ in 0..10 {
                rig.processor.process(&mut out);
            }
        }),
        0
    );
    assert_eq!(manager.runtime.fixture_parameter(target, id), Some(0.25));
    assert!(
        rig.session
            .controller()
            .frame()
            .automated
            .iter()
            .any(|p| p.automation == AutomationId(automation) && p.value == range.normalized(0.25))
    );
    let runtime = manager.runtime.clone();
    let project = rig.project();
    let task = std::thread::spawn(move || runtime.capture(project));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !task.is_finished() {
        assert!(std::time::Instant::now() < deadline);
        assert_eq!(
            crate::test_alloc::allocator_calls(|| rig.processor.process(&mut [])),
            0
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let captured = task.join().unwrap().unwrap();
    let value = |p: &Project| {
        p.plugin(target)
            .unwrap()
            .parameters
            .iter()
            .find(|p| p.id == id)
            .unwrap()
            .value
    };
    assert_eq!(
        value(&captured),
        0.25,
        "{vst3}: deduplicated document suppressed newer automation"
    );
    assert_eq!(manager.runtime.selected_token(target), Some(token));
    let save_path = rig.file("deduplicated.windfall");
    let path = save_path.clone();
    let task = rig
        .session
        .background(move |session| session.project_save(Some(&path)));
    while !task.is_finished() {
        assert!(std::time::Instant::now() < deadline);
        assert_eq!(
            crate::test_alloc::allocator_calls(|| rig.processor.process(&mut [])),
            0
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    task.join().unwrap().unwrap();
    assert_eq!(
        value(&windfall_project::file::load(Path::new(&save_path)).unwrap()),
        0.25
    );
    let options = ExportOptions {
        path: rig.file("deduplicated.wav"),
        format: ExportFormat::Wav,
        bit_depth: BitDepth::Float32,
        sample_rate: SAMPLE_RATE,
        mode: PlayMode::Pattern,
        tail_secs: 0.0,
        ..Default::default()
    };
    let export = options.clone();
    let task = rig
        .session
        .background(move |session| session.export_audio(export));
    while !task.is_finished() {
        assert!(std::time::Instant::now() < deadline);
        assert_eq!(
            crate::test_alloc::allocator_calls(|| rig.processor.process(&mut [])),
            0
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    task.join().unwrap().unwrap();
    assert_eq!(rig.events.wait_for_export().error, None);
    assert_eq!(manager.runtime.selected_token(target), Some(token));
    let exported = windfall_codec::decode_file(&options.path).unwrap();
    rig.session.project_open(&save_path).unwrap();
    rig.run(256);
    assert_eq!(value(&rig.project()), 0.25);
    assert_eq!(manager.runtime.fixture_parameter(target, id), Some(0.25));
    rig.events.take();
    let reference = ExportOptions {
        path: rig.file("reopened.wav"),
        ..options
    };
    let export = reference.clone();
    let task = rig
        .session
        .background(move |session| session.export_audio(export));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !task.is_finished() {
        assert!(std::time::Instant::now() < deadline);
        assert_eq!(
            crate::test_alloc::allocator_calls(|| rig.processor.process(&mut [])),
            0
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    task.join().unwrap().unwrap();
    assert_eq!(rig.events.wait_for_export().error, None);
    assert_eq!(
        exported.samples(),
        windfall_codec::decode_file(&reference.path)
            .unwrap()
            .samples()
    );
}

#[test]
fn saturated_clap_parameter_edit_retries_without_replacing_playback_or_allocating() {
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
    let target = PluginTarget::Instrument { channel };
    rig.session
        .dispatch(
            Command::SetPluginParam {
                target,
                id: 1,
                value: 0.5,
            },
            None,
        )
        .unwrap();
    rig.run(256);
    let token = manager.runtime.selected_token(target).unwrap();
    let epoch = rig.session.controller().hardware_epoch();
    for _ in 0..1024 {
        assert!(
            rig.session
                .controller()
                .hardware_note(epoch, channel, 64, 100)
        );
    }
    assert_eq!(
        crate::test_alloc::allocator_calls(|| rig.processor.process(&mut [])),
        0
    );
    rig.session
        .dispatch(
            Command::SetPluginParam {
                target,
                id: 1,
                value: 0.75,
            },
            None,
        )
        .unwrap();
    assert_eq!(
        crate::test_alloc::allocator_calls(|| rig.processor.process(&mut [])),
        0
    );
    let mut out = [0.0; 256];
    assert_eq!(
        crate::test_alloc::allocator_calls(|| {
            for _ in 0..8 {
                rig.processor.process(&mut out);
            }
        }),
        0
    );
    assert_eq!(manager.runtime.selected_token(target), Some(token));
    assert_eq!(
        rig.project()
            .plugin(target)
            .unwrap()
            .parameters
            .iter()
            .find(|param| param.id == 1)
            .unwrap()
            .value,
        0.75
    );
    let native = manager.runtime.capture(rig.project()).unwrap();
    assert_eq!(
        native
            .plugin(target)
            .unwrap()
            .parameters
            .iter()
            .find(|param| param.id == 1)
            .unwrap()
            .value,
        0.75,
        "committed cached parameter must eventually reach native after saturation drains"
    );
    assert_eq!(manager.runtime.selected_token(target), Some(token));
}

#[test]
fn runtime_repair_saturated_controls_survive_capture_save_backup_zip_and_export() {
    for action in [
        "capture",
        "save",
        "backup",
        "zip",
        "export",
        "unobserved-save",
    ] {
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
        let target = PluginTarget::Instrument { channel };
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
        rig.session
            .dispatch(
                Command::SetPluginParam {
                    target,
                    id: 1,
                    value: 0.5,
                },
                None,
            )
            .unwrap();
        rig.run(256);
        let saved = rig.file("pending.windfall");
        rig.session.project_save(Some(&saved)).unwrap();
        rig.run(256);
        let token = manager.runtime.selected_token(target).unwrap();
        let epoch = rig.session.controller().hardware_epoch();
        for _ in 0..1024 {
            assert!(
                rig.session
                    .controller()
                    .hardware_note(epoch, channel, 64, 100)
            );
        }
        assert_eq!(
            crate::test_alloc::allocator_calls(|| rig.processor.process(&mut [])),
            0
        );
        rig.session
            .dispatch(
                Command::SetPluginParam {
                    target,
                    id: 1,
                    value: 0.75,
                },
                None,
            )
            .unwrap();
        if action != "unobserved-save" {
            assert_eq!(
                crate::test_alloc::allocator_calls(|| rig.processor.process(&mut [])),
                0
            );
        }
        let value = |project: &Project| {
            project
                .plugin(target)
                .unwrap()
                .parameters
                .iter()
                .find(|param| param.id == 1)
                .unwrap()
                .value
        };
        if action == "export" {
            let options = ExportOptions {
                path: rig.file("pending.wav"),
                format: ExportFormat::Wav,
                bit_depth: BitDepth::Float32,
                sample_rate: SAMPLE_RATE,
                mode: PlayMode::Pattern,
                tail_secs: 0.0,
                ..Default::default()
            };
            rig.session.export_audio(options.clone()).unwrap();
            assert_eq!(rig.events.wait_for_export().error, None);
            let first = windfall_codec::decode_file(&options.path).unwrap();
            assert_eq!(manager.runtime.selected_token(target), Some(token));
            rig.run(2048);
            rig.events.take();
            let mut reference = options;
            reference.path = rig.file("settled.wav");
            rig.session.export_audio(reference.clone()).unwrap();
            assert_eq!(rig.events.wait_for_export().error, None);
            let settled = windfall_codec::decode_file(&reference.path).unwrap();
            assert_eq!(
                first.samples(),
                settled.samples(),
                "export before draining must render committed intent"
            );
            continue;
        }
        let captured = match action {
            "capture" => manager.runtime.capture(rig.project()).unwrap(),
            "save" | "unobserved-save" => {
                rig.session.project_save(Some(&saved)).unwrap();
                windfall_project::file::load(Path::new(&saved)).unwrap()
            }
            "backup" => {
                let path = rig
                    .session
                    .write_backup("2026-10-07_21-00-00")
                    .unwrap()
                    .unwrap();
                windfall_project::file::load(&path).unwrap()
            }
            "zip" => {
                let zip = rig.file("pending.zip");
                rig.session.project_archive_save(&zip).unwrap();
                assert_eq!(manager.runtime.selected_token(target), Some(token));
                rig.session.project_archive_open(&zip).unwrap().project
            }
            _ => unreachable!(),
        };
        assert_eq!(
            value(&captured),
            0.75,
            "{action}: older native state overwrote committed intent"
        );
        if action != "zip" {
            assert_eq!(manager.runtime.selected_token(target), Some(token));
        }
        let reopened = rig.file("reopened.windfall");
        windfall_project::file::save(&captured, Path::new(&reopened)).unwrap();
        rig.session.project_open(&reopened).unwrap();
        rig.run(512);
        assert_eq!(value(&rig.project()), 0.75);
        assert_eq!(
            value(&manager.runtime.capture(rig.project()).unwrap()),
            0.75
        );
    }
}

#[test]
fn runtime_repair_vst3_bundle_binary_changes_require_rescan() {
    use std::io::Write;
    let rig = Rig::new();
    let bundle = rig.folder.path().join("Bundle.vst3");
    let inner = bundle.join("Contents/x86_64-win/Bundle.vst3");
    std::fs::create_dir_all(inner.parent().unwrap()).unwrap();
    std::fs::copy(library(), &inner).unwrap();
    let scanner = std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(
            || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../target"),
            PathBuf::from,
        )
        .join("debug/windfall-desktop.exe");
    let manager = crate::plugins::PluginManager::fixture(rig.folder.path(), &bundle, &scanner);
    let entry = manager
        .state()
        .entries
        .into_iter()
        .find(|entry| entry.format == "vst3" && !entry.instrument)
        .unwrap();
    let binding = manager
        .binding(
            &entry.path,
            &entry.id,
            PluginTarget::Effect {
                effect: EffectId(901),
            },
        )
        .unwrap();
    let directory = std::fs::metadata(&bundle).unwrap();
    std::fs::OpenOptions::new()
        .append(true)
        .open(&inner)
        .unwrap()
        .write_all(b"changed binary overlay")
        .unwrap();
    let after = std::fs::metadata(&bundle).unwrap();
    assert_eq!(directory.len(), after.len());
    assert_eq!(directory.modified().unwrap(), after.modified().unwrap());
    assert!(
        manager
            .runtime
            .discover(binding.clone())
            .unwrap_err()
            .contains("changed"),
        "the loaded binary must match the approved scan"
    );
    let rescanned = crate::plugins::PluginManager::fixture(
        &rig.folder.path().join("rescanned"),
        &bundle,
        &scanner,
    );
    assert!(rescanned.runtime.discover(binding.clone()).is_ok());
    std::fs::remove_file(&inner).unwrap();
    assert!(rescanned.runtime.discover(binding).is_err());
}

#[test]
fn production_vst3_catalog_enables_checked_roles_and_excludes_rejected_layouts() {
    let rig = Rig::new();
    let folder = rig.folder.path().join("vst3-scan");
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("windfall-test.vst3");
    std::fs::copy(library(), &path).unwrap();
    let target = std::env::var_os("CARGO_TARGET_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../target"),
        PathBuf::from,
    );
    let manager = crate::plugins::PluginManager::fixture(
        rig.folder.path(),
        &path,
        &target.join("debug/windfall-desktop.exe"),
    );
    let module = windfall_plugin_host::PluginHost::windfall()
        .load(&path)
        .unwrap();
    let descriptors = module.descriptors();
    let state = manager.state();
    for (index, instrument) in [(0, false), (2, true)] {
        let entry = state
            .entries
            .iter()
            .find(|entry| entry.id == descriptors[index].id)
            .unwrap();
        assert!(entry.usable, "{:?}", entry.error);
        assert_eq!(entry.format, "vst3");
        assert_eq!(entry.instrument, instrument);
        assert!(
            manager
                .binding(
                    &entry.path,
                    &entry.id,
                    if instrument {
                        PluginTarget::Instrument {
                            channel: windfall_project::ChannelId(900),
                        }
                    } else {
                        PluginTarget::Effect {
                            effect: EffectId(900),
                        }
                    }
                )
                .is_ok()
        );
    }
    // Layout rejection is omitted from the usable catalog. Only dangerous
    // scanner failures enter its separate blocklist.
    assert!(
        state
            .entries
            .iter()
            .all(|entry| entry.id != descriptors[1].id)
    );
    assert!(
        manager
            .binding(
                &path.to_string_lossy(),
                &descriptors[1].id,
                PluginTarget::Effect {
                    effect: EffectId(900)
                }
            )
            .is_err()
    );
}

#[test]
fn native_vst3_session_save_backup_reopen_editor_routing_and_offline_export() {
    let mut rig = Rig::new();
    let (runtime, binding) = crate::plugins::vst3_fixture(2);
    let manager =
        crate::plugins::PluginManager::fixture_runtime(rig.folder.path(), runtime.clone());
    rig.session.install_plugins(manager);
    let added = rig
        .session
        .dispatch(Command::AddPluginInstrument { plugin: binding }, None)
        .unwrap();
    let channel = windfall_project::ChannelId(added.created[0]);
    let target = PluginTarget::Instrument { channel };
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
    rig.session.transport_play().unwrap();
    assert!(rms(&rig.run(12_000)) > 0.01);
    runtime
        .editor_binding(
            target,
            Some(rig.project().plugin(target).unwrap().clone()),
            false,
        )
        .unwrap();
    assert!(runtime.editor(target, true).unwrap_err().contains("editor"));
    let saved = rig.file("vst3.windfall");
    let save_path = saved.clone();
    let task = rig
        .session
        .background(move |session| session.project_save(Some(&save_path)));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !task.is_finished() {
        assert!(std::time::Instant::now() < deadline);
        rig.run(256);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    task.join().unwrap().unwrap();
    let captured = windfall_project::file::load(Path::new(&saved))
        .unwrap()
        .plugins;
    rig.session
        .dispatch(
            Command::SetPluginParam {
                target,
                id: 7,
                value: 0.6,
            },
            None,
        )
        .unwrap();
    rig.run(256);
    let task = rig
        .session
        .background(|session| session.write_backup("2026-10-07_12-30-00"));
    while !task.is_finished() {
        assert!(std::time::Instant::now() < deadline);
        rig.run(256);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(task.join().unwrap().unwrap().is_some());
    rig.session.project_open(&saved).unwrap();
    rig.run(256);
    assert_eq!(rig.project().plugins, captured);
    rig.session.transport_play().unwrap();
    assert!(rms(&rig.run(12_000)) > 0.01);
    let options = ExportOptions {
        path: rig.file("vst3.wav"),
        format: ExportFormat::Wav,
        bit_depth: BitDepth::Float32,
        sample_rate: SAMPLE_RATE,
        mode: PlayMode::Pattern,
        tail_secs: 0.0,
        ..Default::default()
    };
    let export_options = options.clone();
    let task = rig
        .session
        .background(move |session| session.export_audio(export_options));
    while !task.is_finished() {
        assert!(std::time::Instant::now() < deadline);
        rig.run(256);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    task.join().unwrap().unwrap();
    assert_eq!(rig.events.wait_for_export().error, None);
    assert!(
        rms(windfall_codec::decode_file(&options.path)
            .unwrap()
            .samples())
            > 0.01
    );
    assert_eq!(rig.project().plugins, captured);
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
    assert_ne!(
        first_state, binding.state,
        "state capture must contain the processed native parameter change, not the discovery preset"
    );
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
