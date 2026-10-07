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
            },
            960,
            None,
            synthetic,
        )
        .unwrap();
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
