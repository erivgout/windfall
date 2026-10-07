//! Real VST3 fixtures through the desktop native owner. Production additions
//! stay disabled: these tests exercise the ownership seam, not enablement.
use super::*;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use windfall_project::{Command, Document, EffectId, TrackId};

fn fixture(index: usize) -> (Arc<Runtime>, PluginBinding) {
    static FILE: OnceLock<PathBuf> = OnceLock::new();
    let path = FILE.get_or_init(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let target = root.join("target/vst3-ownership-fixture");
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
        let path = target.join("fixture.vst3");
        std::fs::copy(target.join("debug/windfall_test_plugins.dll"), &path).unwrap();
        path
    });
    let module = PluginHost::windfall().load(path).unwrap();
    let descriptor = module.descriptors()[index].clone();
    let runtime = Arc::new(Runtime::new().unwrap());
    // This capability exists only in cfg(test); it cannot be enabled through
    // IPC, preferences, a scanned catalog, or a saved production project.
    runtime
        .call(|owner| {
            owner.vst3_fixture = true;
            Ok(())
        })
        .unwrap();
    runtime.approve(&[windfall_ipc::PluginEntry {
        path: path.to_string_lossy().into_owned(),
        id: descriptor.id.clone(),
        format: "vst3".into(),
        name: descriptor.name.clone(),
        vendor: "Fixture".into(),
        instrument: index == 2,
        usable: true,
        error: None,
    }]);
    let binding = runtime
        .discover(PluginBinding {
            target: PluginTarget::Effect {
                effect: EffectId(900),
            },
            format: "vst3".into(),
            path: path.to_string_lossy().into_owned(),
            id: descriptor.id,
            name: descriptor.name,
            state: vec![],
            parameters: vec![],
        })
        .unwrap();
    (runtime, binding)
}

fn transport() -> windfall_engine::plugins::PluginTransport {
    windfall_engine::plugins::PluginTransport {
        playing: false,
        tempo_bpm: 120.0,
        position_beats: 0.0,
        position_seconds: 0.0,
        numerator: 4,
        denominator: 4,
    }
}

fn capture(
    runtime: &Arc<Runtime>,
    project: Project,
    audio: &mut dyn HostedEffect,
) -> Result<Project, String> {
    let runtime = runtime.clone();
    let task = std::thread::spawn(move || runtime.capture(project));
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !task.is_finished() {
        assert!(
            std::time::Instant::now() < deadline,
            "capture did not complete"
        );
        audio.control_boundary();
        std::thread::sleep(Duration::from_millis(1));
    }
    let result = task.join().unwrap();
    audio.control_boundary();
    result
}

#[test]
fn vst3_return_capture_reacquire_save_reopen_and_offline_render() {
    let (runtime, mut binding) = fixture(0);
    binding
        .parameters
        .iter_mut()
        .find(|p| p.id == 7)
        .unwrap()
        .value = 0.75;
    let mut document = Document::new(Project::new("VST3 ownership"));
    document
        .dispatch(
            Command::AddPluginEffect {
                track: TrackId(0),
                plugin: binding.clone(),
            },
            None,
        )
        .unwrap();
    binding = document.project().plugins[0].clone();
    let mut audio = runtime.effect(&binding, 48_000, 64).unwrap();
    audio.transport(transport());
    audio.set_param(7, 0.75); // still pending at the ownership boundary
    let token = runtime.selected_token(binding.target).unwrap();
    let saved = capture(&runtime, document.project().clone(), audio.as_mut()).unwrap();
    assert_ne!(saved.plugins[0].state, binding.state);
    assert_eq!(runtime.selected_token(binding.target), Some(token));
    assert_eq!(
        audio.latency(),
        17,
        "pause retains the installed compensation"
    );
    let mut left = [1.0; 64];
    let mut right = left;
    audio.process(&mut left, &mut right);
    assert_eq!(left, [1.5; 64]);
    // Once deferred edits reach process, the same semantic state has a
    // canonical encoding without deferred overrides. Compare export ownership
    // against this stable pre-export state, not the earlier pending encoding.
    let steady = capture(&runtime, document.project().clone(), audio.as_mut()).unwrap();

    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("fixture.windfall");
    windfall_project::file::save(&saved, &path).unwrap();
    let reopened = windfall_project::file::load(&path).unwrap();
    assert_eq!(reopened.plugins, saved.plugins);
    let render = runtime.render_factory().unwrap();
    let mut offline = render.effect(&reopened.plugins[0], 48_000, 64).unwrap();
    offline.transport(transport());
    left.fill(1.0);
    right.fill(1.0);
    offline.process(&mut left, &mut right);
    // The document's explicit parameter value takes precedence over opaque
    // state on reopen, matching CLAP's existing discovery/load policy.
    let value = reopened.plugins[0]
        .parameters
        .iter()
        .find(|p| p.id == 7)
        .unwrap()
        .value;
    assert_eq!(left, [value * 2.0; 64]);
    assert_eq!(
        runtime.selected_token(binding.target),
        Some(token),
        "offline instances cannot become playback owners"
    );
    let again = capture(&runtime, document.project().clone(), audio.as_mut()).unwrap();
    assert_eq!(again.plugins[0].state, steady.plugins[0].state);
    drop(offline);
    drop(audio);
    runtime
        .call(|owner| {
            assert!(owner.instances.is_empty());
            Ok(())
        })
        .unwrap();
}

#[test]
fn vst3_instrument_engine_playback_state_reopen_and_wav_export_are_deterministic() {
    let (runtime, mut binding) = fixture(2);
    binding
        .parameters
        .iter_mut()
        .find(|p| p.id == 7)
        .unwrap()
        .value = 0.6;
    let mut document = Document::new(Project::new("VST3 instrument"));
    document
        .dispatch(Command::AddPluginInstrument { plugin: binding }, None)
        .unwrap();
    let channel = document.project().channels[0].id;
    let pattern = document.project().patterns[0].id;
    document
        .dispatch(
            Command::ToggleStep {
                pattern,
                channel,
                step: 0,
            },
            None,
        )
        .unwrap();
    let project = document.project().clone();
    let mut pool = windfall_engine::SamplePool::new();
    pool.set_plugin_factory(runtime.clone());
    let (mut processor, controller) = windfall_engine::Processor::new(48_000);
    controller.set_project(&project, &pool);
    controller.play();
    let mut output = [0.0; 512];
    processor.process(&mut output);
    assert!(output.iter().all(|sample| sample.is_finite()));
    assert!(output.iter().any(|sample| sample.abs() > 0.01));
    let token = runtime.selected_token(project.plugins[0].target).unwrap();
    let capture_runtime = runtime.clone();
    let capture_project = project.clone();
    let task = std::thread::spawn(move || capture_runtime.capture(capture_project));
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !task.is_finished() {
        assert!(std::time::Instant::now() < deadline);
        processor.process(&mut output);
        std::thread::sleep(Duration::from_millis(1));
    }
    let captured = task.join().unwrap().unwrap();
    processor.process(&mut output);
    assert_eq!(
        runtime.selected_token(project.plugins[0].target),
        Some(token)
    );
    let folder = tempfile::tempdir().unwrap();
    let saved = folder.path().join("instrument.windfall");
    windfall_project::file::save(&captured, &saved).unwrap();
    let reopened = windfall_project::file::load(&saved).unwrap();
    assert_eq!(captured.plugins, reopened.plugins);
    let options = windfall_engine::RenderOptions {
        tail_secs: 0.0,
        ..Default::default()
    };
    let first = windfall_engine::render(&captured, &pool, &options, &mut |_| true);
    let second = windfall_engine::render(&reopened, &pool, &options, &mut |_| true);
    assert_eq!(first.samples(), second.samples());
    assert!(first.samples().iter().any(|sample| sample.abs() > 0.01));
    let exported = folder.path().join("instrument.wav");
    windfall_codec::write_wav(&exported, &second, windfall_codec::WavSampleFormat::Float32)
        .unwrap();
    assert_eq!(
        windfall_codec::decode_file(&exported).unwrap().samples(),
        second.samples()
    );
    assert_eq!(
        runtime.selected_token(project.plugins[0].target),
        Some(token)
    );
}

#[test]
fn vst3_no_callback_refuses_capture_without_deactivating_the_processor() {
    let (runtime, binding) = fixture(0);
    let mut audio = runtime.effect(&binding, 48_000, 64).unwrap();
    audio.transport(transport());
    let mut project = Project::new("Disconnected callback");
    project.plugins = vec![binding];
    let error = runtime.capture(project).unwrap_err();
    assert!(error.contains("block boundary"), "{error}");
    audio.control_boundary();
    let mut left = [1.0; 64];
    let mut right = left;
    audio.process(&mut left, &mut right);
    assert_eq!(left, [1.0; 64]);
    runtime
        .call(|owner| {
            assert!(
                owner
                    .instances
                    .values()
                    .all(|record| record.plugin.is_active())
            );
            Ok(())
        })
        .unwrap();
}

#[test]
fn vst3_late_return_and_retirement_are_serviced_by_the_owner() {
    let (runtime, binding) = fixture(0);
    let mut audio = runtime.effect(&binding, 48_000, 64).unwrap();
    audio.transport(transport());
    runtime
        .call(|owner| {
            owner.instances.values().next().unwrap().ownership.request();
            Ok(())
        })
        .unwrap();
    audio.control_boundary();
    // Model timeout cancellation after audio has already relinquished ownership.
    runtime
        .call(|owner| {
            owner.instances.values().next().unwrap().ownership.cancel();
            Ok(())
        })
        .unwrap();
    runtime.call(|_| Ok(())).unwrap(); // owner maintenance completes before next job
    audio.control_boundary();
    let mut left = [1.0; 64];
    let mut right = left;
    audio.process(&mut left, &mut right);
    assert_eq!(left, [1.0; 64]);
    // Also retire while a resume is queued but not yet reacquired.
    runtime
        .call(|owner| {
            owner.instances.values().next().unwrap().ownership.request();
            Ok(())
        })
        .unwrap();
    audio.control_boundary();
    runtime.call(|_| Ok(())).unwrap();
    drop(audio);
    runtime
        .call(|owner| {
            assert!(owner.instances.is_empty());
            Ok(())
        })
        .unwrap();
}

#[test]
fn vst3_speculation_revision_binding_and_failed_units_keep_their_roles() {
    let (runtime, binding) = fixture(0);
    let mut first = runtime.effect(&binding, 48_000, 64).unwrap();
    first.transport(transport());
    let token = runtime.selected_token(binding.target).unwrap();
    let staged = runtime.prepare_document();
    let speculative = staged.effect(&binding, 48_000, 64).unwrap();
    assert_eq!(runtime.selected_token(binding.target), Some(token));
    drop(speculative);
    drop(staged);
    let mut project = Project::new("Stale snapshot");
    project.plugins = vec![binding.clone()];
    let revision = runtime.document_revision();
    runtime.retry();
    assert_eq!(
        runtime
            .capture_at(project.clone(), revision)
            .unwrap()
            .plugins,
        project.plugins
    );
    assert!(runtime.editor(binding.target, false).is_err());

    let (failed_runtime, failed_binding) = fixture(4);
    let mut failed = failed_runtime.effect(&failed_binding, 48_000, 64).unwrap();
    failed.transport(transport());
    let mut left = [0.25; 64];
    let mut right = left;
    failed.process(&mut left, &mut right);
    project.plugins = vec![failed_binding];
    let error = capture(&failed_runtime, project, failed.as_mut()).unwrap_err();
    assert!(error.contains("failed plugin"), "{error}");
    failed.process(&mut left, &mut right);
    assert_eq!(left, [0.25; 64], "failed effects remain bypassed");
}

#[test]
fn production_vst3_gate_survives_a_successful_scan_and_saved_binding() {
    let (_, binding) = fixture(0);
    let runtime = Runtime::new().unwrap();
    runtime.approve(&[windfall_ipc::PluginEntry {
        path: binding.path.clone(),
        id: binding.id.clone(),
        format: "vst3".into(),
        name: binding.name.clone(),
        vendor: "Fixture".into(),
        instrument: false,
        usable: true,
        error: None,
    }]);
    assert!(
        runtime
            .discover(binding)
            .unwrap_err()
            .contains("not enabled")
    );
}
