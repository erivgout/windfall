//! Native plugin refresh/control must not invalidate an owned recording take.
use super::Rig;
use crate::session::recording::{CaptureHandle, Sink};
use windfall_ipc::RecordingSource;

struct FakeCapture;
impl CaptureHandle for FakeCapture {
    fn frames(&self) -> u64 {
        480
    }
    fn failed(&self) -> bool {
        false
    }
    fn finish(self: Box<Self>) -> Result<u64, String> {
        Ok(480)
    }
}
fn synthetic(
    _: RecordingSource,
    rate: u32,
    mut sink: Sink,
) -> Result<Box<dyn CaptureHandle>, String> {
    assert_eq!(rate, 48_000);
    sink(&[0.25; 960])?;
    Ok(Box::new(FakeCapture))
}
fn start_take(rig: &Rig) {
    rig.session
        .recording_start_with(
            RecordingSource {
                host: "Fake".into(),
                device: "Fake".into(),
                left: 0,
                right: None,
                alignment: None,
                loop_recording: None,
                monitor: None,
                armed_tracks: None,
                mixer_tap: None,
            },
            960,
            None,
            synthetic,
        )
        .unwrap();
}

#[cfg(windows)]
#[test]
fn session_worker_retries_coalesced_native_dirty_after_recording() {
    use windfall_project::{Command, EffectId, PluginTarget, TrackId};
    let mut rig = Rig::new();
    let (runtime, binding) = crate::plugins::vst3_fixture(7);
    let manager = crate::plugins::PluginManager::fixture_runtime(rig.folder.path(), runtime);
    rig.session.install_plugins(manager.clone());
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
    let target = PluginTarget::Effect {
        effect: EffectId(*added.created.last().unwrap()),
    };
    rig.session
        .dispatch(
            Command::SetPluginParam {
                target,
                id: 7,
                value: 0.875,
            },
            None,
        )
        .unwrap();
    rig.run(256);
    start_take(&rig);
    let before = rig.session.document_snapshot();
    manager.attach(rig.session.downgrade());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !manager
        .state()
        .error
        .is_some_and(|error| error.contains("recording"))
    {
        assert!(
            std::time::Instant::now() < deadline,
            "worker did not retain the dirty request"
        );
        rig.run(256);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert_eq!(rig.session.document_snapshot(), before);
    rig.session.recording_cancel();
    loop {
        rig.run(256);
        if rig
            .project()
            .plugin(target)
            .unwrap()
            .parameters
            .iter()
            .any(|param| param.id == 7 && param.value == 0.375)
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "retained dirty capture did not retry"
        );
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert_ne!(
        rig.project().plugin(target).unwrap().state,
        before.project.plugin(target).unwrap().state
    );
}

#[cfg(windows)]
#[test]
fn native_vst3_dirty_restart_and_deactivation_edits_wait_for_recording_then_capture() {
    use windfall_project::{Command, EffectId, PluginTarget, TrackId};
    let mut rig = Rig::new();
    let (runtime, binding) = crate::plugins::vst3_fixture(7);
    let manager =
        crate::plugins::PluginManager::fixture_runtime(rig.folder.path(), runtime.clone());
    rig.session.install_plugins(manager);
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
    let target = PluginTarget::Effect {
        effect: EffectId(*added.created.last().unwrap()),
    };
    rig.session
        .dispatch(
            Command::SetPluginParam {
                target,
                id: 7,
                value: 0.875,
            },
            None,
        )
        .unwrap();
    rig.run(256);
    start_take(&rig);
    let before = rig.session.document_snapshot();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let request = loop {
        if let Some(request) = runtime
            .drain()
            .into_iter()
            .find(|request| matches!(request.update, crate::plugins::Update::Capture { .. }))
        {
            break request;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "native dirty request was lost"
        );
        rig.run(256);
        std::thread::sleep(std::time::Duration::from_millis(2));
    };
    assert!(
        rig.session
            .capture_plugin_update(&runtime, request.clone())
            .unwrap_err()
            .contains("recording")
    );
    assert_eq!(rig.session.document_snapshot(), before);
    assert!(
        runtime.drain().is_empty(),
        "coalesce the dirty identity while retained by session"
    );
    rig.session.recording_cancel();
    let capture_runtime = runtime.clone();
    let task = rig
        .session
        .background(move |session| session.capture_plugin_update(&capture_runtime, request));
    while !task.is_finished() {
        assert!(std::time::Instant::now() < deadline);
        rig.run(128);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(
        matches!(
            task.join().unwrap().unwrap(),
            crate::plugins::CaptureOutcome::Accepted { restart: true, .. }
        ),
        "restart is retained until captured"
    );
    let project = rig.project();
    let plugin = project.plugin(target).unwrap();
    assert_eq!(
        plugin
            .parameters
            .iter()
            .find(|param| param.id == 7)
            .unwrap()
            .value,
        0.375,
        "capture includes the native parameter rescan produced by deactivation"
    );
    assert_ne!(plugin.state, before.project.plugin(target).unwrap().state);
    assert!(!rig.session.recording_state().active);
    rig.run(512);
    std::thread::sleep(std::time::Duration::from_millis(30));
    assert!(
        runtime.drain().is_empty(),
        "deactivation dirty is covered, not rescheduled forever"
    );
    assert_eq!(
        rig.session.controller().latency_frames(),
        64,
        "restart installs new compensation after capturing the deactivation latency change"
    );
    assert!(
        runtime.errors().is_empty(),
        "successful replacement clears the recovery error"
    );
}
#[test]
fn active_take_refuses_plugin_refresh_and_control_without_changing_document_or_take() {
    let rig = Rig::new();
    start_take(&rig);
    let document = rig.session.document_snapshot();
    let take = rig.session.recording_state();
    let error = rig.session.refresh_plugins().unwrap_err();
    assert!(error.contains("recording"), "{error}");
    let mut ran = false;
    let error = rig
        .session
        .while_recording_idle(|| {
            ran = true;
            Ok(42)
        })
        .unwrap_err();
    assert!(error.contains("recording"), "{error}");
    assert!(!ran, "refused plugin control must not execute its closure");
    assert_eq!(rig.session.document_snapshot(), document);
    assert_eq!(rig.session.recording_state(), take);
    rig.session.recording_cancel();
    rig.session.refresh_plugins().unwrap();
    assert_eq!(
        rig.session
            .while_recording_idle(|| {
                ran = true;
                Ok(42)
            })
            .unwrap(),
        42
    );
    assert!(ran);
    assert_eq!(rig.session.document_snapshot(), document);
    assert!(!rig.session.recording_state().active);
}

#[cfg(windows)]
#[test]
fn owned_take_refuses_save_and_backup_native_state_capture() {
    let rig = Rig::new();
    let path = rig.file("recording-state.windfall");
    rig.session.project_save(Some(&path)).unwrap();
    rig.session
        .dispatch(
            windfall_project::Command::UpdateSettings {
                patch: windfall_project::SettingsPatch {
                    tempo_bpm: Some(137.0),
                    ..Default::default()
                },
            },
            None,
        )
        .unwrap();
    start_take(&rig);
    let snapshot = rig.session.document_snapshot();
    let take = rig.session.recording_state();
    assert!(
        rig.session
            .project_save(None)
            .unwrap_err()
            .contains("recording")
    );
    assert!(
        rig.session
            .write_backup("2026-10-07_12-00-00")
            .unwrap_err()
            .contains("recording")
    );
    assert_eq!(rig.session.document_snapshot(), snapshot);
    assert_eq!(rig.session.recording_state(), take);
    rig.session.recording_cancel();
    assert!(
        rig.session
            .write_backup("2026-10-07_12-00-01")
            .unwrap()
            .is_some()
    );
}

#[cfg(windows)]
#[test]
fn refused_refresh_preserves_live_clap_revision_and_captured_state_until_discard() {
    use windfall_engine::plugins::PluginFactory;
    use windfall_project::{Command, EffectId, PluginTarget, TrackId};
    let mut rig = Rig::new();
    let (manager, path) = super::plugins::manager(&rig);
    let plugin = manager
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
                plugin,
            },
            None,
        )
        .unwrap();
    let effect = EffectId(*added.created.last().unwrap());
    let target = PluginTarget::Effect { effect };
    rig.session
        .dispatch(
            Command::SetPluginParam {
                target,
                id: 7,
                value: 0.3,
            },
            None,
        )
        .unwrap();
    rig.run(512);
    let revision = manager.runtime.revision();
    let captured = manager.runtime.capture(rig.project()).unwrap().plugins;
    assert_eq!(captured.len(), 1);
    assert!(captured[0].state.starts_with(b"WFPS"));
    start_take(&rig);
    let document = rig.session.document_snapshot();
    let take = rig.session.recording_state();
    assert!(
        rig.session
            .refresh_plugins()
            .unwrap_err()
            .contains("recording")
    );
    let mut controlled = false;
    assert!(
        rig.session
            .while_recording_idle(|| {
                controlled = true;
                manager.runtime.editor_binding(
                    target,
                    Some(document.project.plugins[0].clone()),
                    false,
                )
            })
            .unwrap_err()
            .contains("recording")
    );
    assert!(!controlled);
    assert!(
        rig.session
            .dispatch_plugin_update(
                &manager.runtime,
                (
                    revision,
                    manager.runtime.selected_token(target).unwrap(),
                    crate::plugins::binding_identity(&document.project.plugins[0]),
                ),
                Command::SetPluginParam {
                    target,
                    id: 7,
                    value: 0.9,
                },
                None,
            )
            .unwrap_err()
            .contains("recording")
    );
    assert_eq!(
        manager.runtime.revision(),
        revision,
        "refused refresh cannot invalidate playback ownership"
    );
    assert_eq!(
        manager.runtime.capture(rig.project()).unwrap().plugins,
        captured
    );
    assert_eq!(rig.session.document_snapshot(), document);
    assert_eq!(rig.session.recording_state(), take);
    rig.session.recording_cancel();
    rig.session.refresh_plugins().unwrap();
    assert_ne!(manager.runtime.revision(), revision);
    rig.run(512);
    rig.session
        .while_recording_idle(|| {
            controlled = true;
            manager
                .runtime
                .editor_binding(target, Some(document.project.plugins[0].clone()), false)
        })
        .unwrap();
    assert!(controlled);
    assert_eq!(
        manager.runtime.capture(rig.project()).unwrap().plugins,
        captured
    );
    assert_eq!(rig.session.document_snapshot(), document);
    assert!(!rig.session.recording_state().active);
}
