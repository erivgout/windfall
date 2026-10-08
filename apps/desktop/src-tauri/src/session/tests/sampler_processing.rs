//! Barrier checks: preparation releases locks and cannot publish obsolete banks.
use super::Rig;
use windfall_core::AudioBuffer;
use windfall_project::{
    ChannelId, ClipStretchQuality, Command, SamplerKeyRange, SamplerPatch, SamplerStretch,
};

fn command(id: ChannelId) -> Command {
    Command::UpdateSampler {
        id,
        patch: SamplerPatch {
            stretch: Some(SamplerStretch::Spectral {
                ratio: 1.5,
                quality: ClipStretchQuality::Fast,
                formants: true,
                range: SamplerKeyRange {
                    first: 60,
                    last: 60,
                },
            }),
            ..Default::default()
        },
    }
}
fn prepare(rig: &Rig) {
    let request = rig.session.sampler_preparation_begin().unwrap();
    rig.session
        .prepare_sampler_command(command(rig.channel(0)), request)
        .unwrap();
}

#[test]
fn sampler_processing_apply_undo_redo_save_open_and_direct_dispatch_prepare() {
    let rig = Rig::new();
    let before = rig.session.document_snapshot();
    prepare(&rig);
    let after = rig.session.document_snapshot();
    assert_eq!(after.history.cursor, before.history.cursor + 1);
    assert!(
        rig.session
            .controller()
            .sampler_key_supported(rig.channel(0), 60)
    );
    assert!(
        !rig.session
            .controller()
            .sampler_key_supported(rig.channel(0), 59)
    );
    rig.session.undo().unwrap();
    assert_eq!(rig.project(), before.project);
    rig.session.redo().unwrap();
    assert_eq!(rig.project(), after.project);
    let path = rig.folder.path().join("sampler.windfall");
    rig.session
        .project_save(Some(path.to_str().unwrap()))
        .unwrap();
    rig.session.project_new().unwrap();
    rig.session.project_open(path.to_str().unwrap()).unwrap();
    assert_eq!(rig.project(), after.project);
    let project = rig.project();
    assert!(!rig.session.state().pool.needs_sampler_preparation(&project));
    rig.session
        .dispatch(
            Command::UpdateSampler {
                id: rig.channel(0),
                patch: SamplerPatch {
                    tune: Some(1.0),
                    ..Default::default()
                },
            },
            None,
        )
        .unwrap();
    let project = rig.project();
    assert!(!rig.session.state().pool.needs_sampler_preparation(&project));
}

#[test]
fn sampler_processing_cancellation_and_new_requests_preserve_project_pool_history_and_plan() {
    for supersede in [false, true] {
        let rig = Rig::new();
        let request = rig.session.sampler_preparation_begin().unwrap();
        let before = rig.session.document_snapshot();
        let retained = rig.session.state().pool.sampler_retained_bytes();
        let command = command(rig.channel(0));
        let hold = rig.session.hold("sampler:prepared");
        let worker = rig
            .session
            .background(move |session| session.prepare_sampler_command(command, request));
        hold.wait();
        assert_eq!(rig.session.document_snapshot(), before); // state lock is released
        if supersede {
            rig.session.sampler_preparation_begin().unwrap();
        } else {
            rig.session.sampler_preparation_cancel(request);
        }
        hold.release();
        assert!(worker.join().unwrap().unwrap_err().contains("cancelled"));
        assert_eq!(rig.session.document_snapshot(), before);
        assert_eq!(rig.session.state().pool.sampler_retained_bytes(), retained);
        assert!(
            rig.session
                .controller()
                .sampler_key_supported(rig.channel(0), 59)
        );
    }
}

#[test]
fn sampler_processing_generation_edit_and_source_guards_refuse_stale_publication() {
    for change in 0..3 {
        let rig = Rig::new();
        let request = rig.session.sampler_preparation_begin().unwrap();
        let pending = command(rig.channel(0));
        let hold = rig.session.hold("sampler:prepared");
        let worker = rig
            .session
            .background(move |session| session.prepare_sampler_command(pending, request));
        hold.wait();
        match change {
            0 => {
                rig.session.project_new().unwrap();
            }
            1 => {
                rig.session
                    .dispatch(
                        Command::UpdateSampler {
                            id: rig.channel(0),
                            patch: SamplerPatch {
                                gain: Some(0.5),
                                ..Default::default()
                            },
                        },
                        None,
                    )
                    .unwrap();
            }
            _ => {
                let mut state = rig.session.state();
                let sample = state.document.project().channels[0]
                    .source
                    .sample()
                    .unwrap();
                state.pool.insert(
                    sample,
                    AudioBuffer::from_interleaved(48_000, 1, vec![0.5; 50]),
                );
            }
        }
        let changed = rig.session.document_snapshot();
        hold.release();
        assert!(worker.join().unwrap().is_err());
        assert_eq!(rig.session.document_snapshot(), changed);
        assert_eq!(rig.session.state().pool.sampler_retained_bytes(), 0);
    }
}

#[test]
fn sampler_processing_budget_failure_preserves_old_settings_history_and_plan() {
    let rig = Rig::new();
    {
        let mut state = rig.session.state();
        let mut limited = windfall_engine::SamplePool::with_sampler_budget(1024);
        for (id, audio) in state.pool.iter() {
            limited.insert(id, audio.clone());
        }
        state.pool = limited;
    }
    let before = rig.session.document_snapshot();
    let request = rig.session.sampler_preparation_begin().unwrap();
    let error = rig
        .session
        .prepare_sampler_command(command(rig.channel(0)), request)
        .unwrap_err();
    assert!(error.contains("budget"));
    assert_eq!(rig.session.document_snapshot(), before);
    assert!(
        rig.session
            .controller()
            .sampler_key_supported(rig.channel(0), 59)
    );
    assert_eq!(rig.session.state().pool.sampler_retained_bytes(), 0);
}

struct FakeCapture;
impl crate::session::recording::CaptureHandle for FakeCapture {
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
fn start_take(rig: &Rig) {
    rig.session
        .recording_start_with(
            windfall_ipc::RecordingSource {
                host: "Fake".into(),
                device: "Fake".into(),
                left: 0,
                right: None,
            },
            960,
            None,
            |_, _, mut sink| {
                sink(&[0.25; 960])?;
                Ok(Box::new(FakeCapture))
            },
        )
        .unwrap();
}

#[test]
fn sampler_processing_recording_before_and_after_render_refuses_apply_and_history() {
    let rig = Rig::new();
    let request = rig.session.sampler_preparation_begin().unwrap();
    let pending = command(rig.channel(0));
    let hold = rig.session.hold("sampler:prepared");
    let worker = rig
        .session
        .background(move |session| session.prepare_sampler_command(pending, request));
    hold.wait();
    start_take(&rig);
    let before = rig.session.document_snapshot();
    hold.release();
    assert!(worker.join().unwrap().unwrap_err().contains("recording"));
    assert_eq!(rig.session.document_snapshot(), before);
    assert!(rig.session.sampler_preparation_begin().is_err());
    assert!(rig.session.undo().is_none());
    assert!(rig.session.redo().is_none());
    assert_eq!(rig.session.document_snapshot(), before);
    assert!(rig.session.recording_state().active);
    rig.session.recording_cancel();
    prepare(&rig);
    start_take(&rig);
    let after = rig.session.document_snapshot();
    assert!(rig.session.undo().is_none());
    rig.session.history_jump(0);
    assert_eq!(rig.session.document_snapshot(), after);
    rig.session.recording_cancel();
    rig.session.undo().unwrap();
}

#[test]
fn sampler_processing_open_and_history_prepare_off_lock_and_reject_project_replacement() {
    let rig = Rig::new();
    prepare(&rig);
    let path = rig.folder.path().join("prepared.windfall");
    rig.session
        .project_save(Some(path.to_str().unwrap()))
        .unwrap();
    rig.session.project_new().unwrap();
    let hold = rig.session.hold("sampler:install-prepared");
    let path = path.to_str().unwrap().to_owned();
    let worker = rig
        .session
        .background(move |session| session.project_open(&path));
    hold.wait();
    let before = rig.session.document_snapshot();
    // An edit proves the document lock is free and invalidates the open ticket.
    rig.session
        .dispatch(
            Command::UpdateSampler {
                id: rig.channel(0),
                patch: SamplerPatch {
                    gain: Some(0.75),
                    ..Default::default()
                },
            },
            None,
        )
        .unwrap();
    let changed = rig.session.document_snapshot();
    assert_ne!(changed, before);
    hold.release();
    assert!(worker.join().unwrap().is_err());
    assert_eq!(rig.session.document_snapshot(), changed);
    let hold = rig.session.hold("sampler:history-prepared");
    let worker = rig.session.background(|session| session.undo());
    hold.wait();
    rig.session.project_new().unwrap();
    let replacement = rig.session.document_snapshot();
    hold.release();
    assert!(worker.join().unwrap().is_none());
    assert_eq!(rig.session.document_snapshot(), replacement);
}

#[test]
fn sampler_processing_export_refuses_unpublished_source_variants() {
    let rig = Rig::new();
    prepare(&rig);
    {
        let mut state = rig.session.state();
        let sample = state.document.project().channels[0]
            .source
            .sample()
            .unwrap();
        state.pool.insert(
            sample,
            AudioBuffer::from_interleaved(48_000, 1, vec![0.5; 50]),
        );
    }
    let options = windfall_ipc::ExportOptions {
        path: rig.file("pending.wav"),
        sample_rate: 48_000,
        pattern_loops: 1,
        ..Default::default()
    };
    let before = rig.session.document_snapshot();
    let error = rig.session.export_audio(options).unwrap_err();
    assert!(error.contains("not prepared"));
    assert_eq!(rig.session.document_snapshot(), before);
    assert!(!rig.folder.path().join("pending.wav").exists());
}

#[test]
fn sampler_processing_background_reload_coalesces_sources_and_refuses_recording_publication() {
    use std::sync::atomic::Ordering;
    let rig = Rig::new();
    prepare(&rig);
    let reload = |value| {
        let mut state = rig.session.state();
        let sample = state.document.project().channels[0]
            .source
            .sample()
            .unwrap();
        state.pool.insert(
            sample,
            AudioBuffer::from_interleaved(48_000, 1, vec![value; 512]),
        );
        rig.session.push_project(&state);
    };
    let wait = || {
        let deadline = std::time::Instant::now() + super::PATIENCE;
        while rig.session.inner.preparing_samplers.load(Ordering::Acquire) {
            assert!(
                std::time::Instant::now() < deadline,
                "sampler worker did not finish"
            );
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    };
    let hold = rig.session.hold("sampler:background-prepared");
    reload(0.25);
    hold.wait();
    reload(0.5);
    hold.release();
    wait();
    let project = rig.project();
    assert!(!rig.session.state().pool.needs_sampler_preparation(&project));
    let hold = rig.session.hold("sampler:background-prepared");
    reload(0.75);
    hold.wait();
    start_take(&rig);
    let before = rig.session.document_snapshot();
    hold.release();
    wait();
    assert_eq!(rig.session.document_snapshot(), before);
    assert!(rig.session.state().pool.needs_sampler_preparation(&project));
    assert!(rig.session.recording_state().active);
    rig.session.recording_cancel();
}

#[test]
fn sampler_processing_background_reload_rejects_a_new_replacement_request() {
    use std::sync::atomic::Ordering;
    let rig = Rig::new();
    prepare(&rig);
    let retained = rig.session.state().pool.sampler_retained_bytes();
    let hold = rig.session.hold("sampler:background-prepared");
    {
        let mut state = rig.session.state();
        let sample = state.document.project().channels[0]
            .source
            .sample()
            .unwrap();
        state.pool.insert(
            sample,
            AudioBuffer::from_interleaved(48_000, 1, vec![0.25; 512]),
        );
        rig.session.push_project(&state);
    }
    hold.wait();
    let before = rig.session.document_snapshot();
    rig.session.begin_replacement();
    hold.release();
    let deadline = std::time::Instant::now() + super::PATIENCE;
    while rig.session.inner.preparing_samplers.load(Ordering::Acquire) {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(rig.session.document_snapshot(), before);
    let state = rig.session.state();
    assert!(
        state
            .pool
            .needs_sampler_preparation(state.document.project())
    );
    assert_eq!(state.pool.sampler_retained_bytes(), retained);
}

#[test]
fn sampler_processing_clip_worker_routes_source_misses_through_sampler_recording_guard() {
    use std::sync::atomic::Ordering;
    let rig = Rig::new();
    prepare(&rig);
    let retained = rig.session.state().pool.sampler_retained_bytes();
    let hold = rig.session.hold("sampler:background-prepared");
    {
        let mut state = rig.session.state();
        let sample = state.document.project().channels[0]
            .source
            .sample()
            .unwrap();
        state.pool.insert(
            sample,
            AudioBuffer::from_interleaved(48_000, 1, vec![0.25; 512]),
        );
        rig.session.queue_clip_preparation();
    }
    hold.wait();
    start_take(&rig);
    let before = rig.session.document_snapshot();
    hold.release();
    let deadline = std::time::Instant::now() + super::PATIENCE;
    while rig.session.inner.preparing_samplers.load(Ordering::Acquire)
        || rig.session.inner.preparing_clips.load(Ordering::Acquire)
    {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(rig.session.document_snapshot(), before);
    let state = rig.session.state();
    assert!(
        state
            .pool
            .needs_sampler_preparation(state.document.project())
    );
    assert_eq!(state.pool.sampler_retained_bytes(), retained);
    drop(state);
    rig.session.recording_cancel();
}

#[test]
fn sampler_processing_export_snapshot_retains_a_charged_bank_after_live_cache_eviction() {
    let mut rig = Rig::new();
    prepare(&rig);
    let retained = rig.session.state().pool.sampler_retained_bytes();
    let hold = rig.session.hold("sampler:export-snapshot");
    rig.session
        .export_audio(windfall_ipc::ExportOptions {
            path: rig.file("snapshot.wav"),
            mode: windfall_ipc::PlayMode::Pattern,
            tail_secs: 0.0,
            bit_depth: windfall_ipc::BitDepth::Float32,
            ..Default::default()
        })
        .unwrap();
    hold.wait();
    rig.session
        .dispatch(
            Command::UpdateSampler {
                id: rig.channel(0),
                patch: SamplerPatch {
                    stretch: Some(SamplerStretch::Tape),
                    ..Default::default()
                },
            },
            None,
        )
        .unwrap();
    for _ in 0..8 {
        rig.run(256);
        rig.session.controller().transport();
    }
    assert_eq!(rig.session.state().pool.sampler_retained_bytes(), retained);
    hold.release();
    assert!(rig.events.wait_for_export().error.is_none());
    let deadline = std::time::Instant::now() + super::PATIENCE;
    while rig.session.state().pool.sampler_retained_bytes() != 0 {
        assert!(
            std::time::Instant::now() < deadline,
            "export bank did not retire"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}
