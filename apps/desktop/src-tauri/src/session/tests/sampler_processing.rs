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

#[test]
fn sampler_r1_identical_apply_recovers_recording_refused_reload_without_a_musical_edit() {
    use std::sync::atomic::Ordering;
    let mut rig = Rig::new();
    rig.session
        .dispatch(
            Command::ToggleStep {
                pattern: rig.pattern(),
                channel: rig.channel(0),
                step: 0,
            },
            None,
        )
        .unwrap();
    prepare(&rig);
    let hold = rig.session.hold("sampler:background-prepared");
    {
        let mut state = rig.session.state();
        let sample = state.document.project().channels[0]
            .source
            .sample()
            .unwrap();
        let data = (0..8192)
            .map(|frame| (std::f32::consts::TAU * 880.0 * frame as f32 / 48_000.0).sin() * 0.25)
            .collect();
        state
            .pool
            .insert(sample, AudioBuffer::from_interleaved(48_000, 1, data));
        rig.session.push_project(&state);
    }
    hold.wait();
    start_take(&rig);
    hold.release();
    let deadline = std::time::Instant::now() + super::PATIENCE;
    while rig.session.inner.preparing_samplers.load(Ordering::Acquire) {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    rig.session.recording_cancel();
    let before = rig.session.document_snapshot();
    assert!(
        rig.session
            .state()
            .pool
            .needs_sampler_preparation(&before.project)
    );
    prepare(&rig); // identical settings: runtime repair, no new musical undo step
    assert_eq!(rig.session.document_snapshot(), before);
    assert!(
        !rig.session
            .state()
            .pool
            .needs_sampler_preparation(&before.project)
    );
    let expected = {
        let state = rig.session.state();
        windfall_engine::render(
            &before.project,
            &state.pool,
            &windfall_engine::RenderOptions {
                pattern: Some(before.project.patterns[0].id),
                tail_secs: 0.0,
                ..Default::default()
            },
            &mut |_| true,
        )
    };
    rig.session.transport_play().unwrap();
    let playback = rig.run(4096);
    assert_eq!(playback, expected.samples()[..playback.len()]);
    assert!(playback.iter().any(|value| value.abs() > 0.01));
    rig.session.transport_stop();
    let path = rig.file("recovered.wav");
    rig.session
        .export_audio(windfall_ipc::ExportOptions {
            path: path.clone(),
            mode: windfall_ipc::PlayMode::Pattern,
            bit_depth: windfall_ipc::BitDepth::Float32,
            tail_secs: 0.0,
            ..Default::default()
        })
        .unwrap();
    assert!(rig.events.wait_for_export().error.is_none());
    assert_eq!(
        windfall_codec::decode_file(path).unwrap().samples(),
        expected.samples()
    );
    assert_eq!(rig.session.document_snapshot(), before);
}

#[test]
fn sampler_r1_history_attaches_the_exact_prepared_restored_asset_after_cache_replacement() {
    let mut rig = Rig::new();
    let file = rig.file("restored.wav");
    let write = |frames, level| {
        windfall_codec::write_wav(
            &file,
            &AudioBuffer::from_interleaved(48_000, 1, vec![level; frames]),
            windfall_codec::WavSampleFormat::Float32,
        )
        .unwrap();
    };
    write(4096, 0.25);
    let channel = rig.channel(0);
    let imported = rig
        .session
        .set_channel_sample_from_file(channel, &file)
        .unwrap();
    let sample = windfall_project::SampleId(imported.created[0]);
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
    prepare(&rig);
    let original = rig.session.state().pool.get(sample).unwrap().clone();
    let restored = rig.session.document_snapshot();
    let expected = {
        let state = rig.session.state();
        windfall_engine::render(
            &restored.project,
            &state.pool,
            &Default::default(),
            &mut |_| true,
        )
    };
    rig.session
        .dispatch(
            Command::Batch {
                label: Some("Remove spectral asset".into()),
                commands: vec![
                    Command::RemoveChannel { id: channel },
                    Command::RemoveSample { id: sample },
                ],
            },
            None,
        )
        .unwrap();
    assert!(!rig.session.state().pool.contains(sample));
    let hold = rig.session.hold("sampler:history-prepared");
    let worker = rig.session.background(|session| session.undo());
    hold.wait();
    write(8192, -0.125);
    rig.session.sample_info(&file).unwrap(); // public preview/info cache replaces A with B
    let replacement = rig
        .session
        .inner
        .cache
        .peek(std::path::Path::new(&file))
        .unwrap();
    assert_ne!(original.identity(), replacement.identity());
    assert!(!rig.session.state().pool.contains(sample));
    hold.release();
    assert!(worker.join().unwrap().is_some());
    assert_eq!(rig.project(), restored.project);
    let state = rig.session.state();
    assert_eq!(
        state.pool.get(sample).unwrap().identity(),
        original.identity()
    );
    assert!(!state.pool.needs_sampler_preparation(&restored.project));
    drop(state);
    rig.session.transport_play().unwrap();
    let playback = rig.run(4096);
    assert!(playback.iter().any(|x| x.abs() > 0.01));
    assert_eq!(playback, expected.samples()[..playback.len()]);
    rig.session.transport_stop();
    rig.session
        .export_audio(windfall_ipc::ExportOptions {
            path: rig.file("restored-export.wav"),
            mode: windfall_ipc::PlayMode::Pattern,
            tail_secs: 0.0,
            bit_depth: windfall_ipc::BitDepth::Float32,
            ..Default::default()
        })
        .unwrap();
    assert!(rig.events.wait_for_export().error.is_none());
    assert_eq!(
        windfall_codec::decode_file(rig.file("restored-export.wav"))
            .unwrap()
            .samples(),
        expected.samples()
    );
}

#[test]
fn sampler_r1_clip_apply_refuses_a_newly_loaded_sampler_source_and_keeps_its_bank() {
    use std::sync::atomic::Ordering;
    use windfall_project::{AudioClipPatch, AudioClipUpdate, ClipId, SampleId, SamplePath};
    let mut rig = Rig::new();
    let clip = rig
        .session
        .add_audio_clip_from_file(
            &super::factory_file("Bass/Bass Sub.wav"),
            crate::session::ClipPlace {
                track: None,
                start: 0,
                mixer_track: None,
            },
        )
        .unwrap();
    let clip = ClipId(*clip.created.last().unwrap());
    let file = rig.file("missing-spectral.wav");
    let sample = SampleId(rig.project().next_id);
    let channel = rig.channel(0);
    rig.session
        .dispatch(
            Command::Batch {
                label: None,
                commands: vec![
                    Command::AddSample {
                        name: "Missing".into(),
                        path: SamplePath::External(file.clone()),
                    },
                    Command::SetChannelSample {
                        id: channel,
                        sample: Some(sample),
                    },
                ],
            },
            None,
        )
        .unwrap();
    rig.wait_until_loaded_or_failed(sample);
    assert!(rig.session.state().failed.contains(&sample));
    prepare(&rig);
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
    let pending = Command::UpdateAudioClips {
        updates: vec![AudioClipUpdate {
            id: clip,
            patch: AudioClipPatch {
                reverse: Some(true),
                ..Default::default()
            },
        }],
    };
    let hold = rig.session.hold("clip:prepared");
    let worker = rig
        .session
        .background(move |session| session.prepare_clip_command(pending));
    hold.wait();
    windfall_codec::write_wav(
        &file,
        &AudioBuffer::from_interleaved(48_000, 1, vec![0.25; 4096]),
        windfall_codec::WavSampleFormat::Float32,
    )
    .unwrap();
    assert_eq!(rig.session.samples_reload(), 0);
    let deadline = std::time::Instant::now() + super::PATIENCE;
    while rig.session.inner.preparing_samplers.load(Ordering::Acquire) {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let before = rig.session.document_snapshot();
    let source = rig.session.state().pool.get(sample).unwrap().identity();
    let bytes = rig.session.state().pool.sampler_retained_bytes();
    assert!(
        !rig.session
            .state()
            .pool
            .needs_sampler_preparation(&before.project)
    );
    assert!(rig.session.controller().sampler_key_supported(channel, 60));
    rig.session.controller().note_on(channel, 60, 0.8);
    let playback = rig.run(8192);
    assert!(playback.iter().any(|x| x.abs() > 0.01));
    hold.release();
    assert!(
        worker
            .join()
            .unwrap()
            .unwrap_err()
            .contains("sample changed")
    );
    assert_eq!(rig.session.document_snapshot(), before);
    let state = rig.session.state();
    assert_eq!(state.pool.get(sample).unwrap().identity(), source);
    assert_eq!(state.pool.sampler_retained_bytes(), bytes);
    assert!(!state.pool.needs_sampler_preparation(&before.project));
    assert!(rig.session.controller().sampler_key_supported(channel, 60));
    drop(state);
    rig.session.controller().note_on(channel, 60, 0.8);
    assert_eq!(rig.run(8192), playback);
    let expected = {
        let state = rig.session.state();
        windfall_engine::render(
            &before.project,
            &state.pool,
            &Default::default(),
            &mut |_| true,
        )
    };
    rig.session
        .export_audio(windfall_ipc::ExportOptions {
            path: rig.file("latest-bank.wav"),
            mode: windfall_ipc::PlayMode::Pattern,
            bit_depth: windfall_ipc::BitDepth::Float32,
            ..Default::default()
        })
        .unwrap();
    assert!(rig.events.wait_for_export().error.is_none());
    assert_eq!(
        windfall_codec::decode_file(rig.file("latest-bank.wav"))
            .unwrap()
            .samples(),
        expected.samples()
    );
}

fn limited_candidate_fixture() -> Rig {
    let rig = Rig::new();
    {
        let mut state = rig.session.state();
        let mut pool = windfall_engine::SamplePool::with_sampler_budget(256 * 1024);
        for (id, audio) in state.pool.iter() {
            pool.insert(id, audio.clone());
        }
        let sample = state.document.project().channels[0]
            .source
            .sample()
            .unwrap();
        pool.insert(
            sample,
            AudioBuffer::from_interleaved(48_000, 1, vec![0.25; 4096]),
        );
        state.pool = pool;
    }
    prepare(&rig);
    rig
}

#[test]
fn sampler_r1_candidate_budget_noop_and_success_publish_atomically_with_exact_sources() {
    use windfall_project::{SampleId, SamplePath};
    for mode in ["budget", "noop", "success"] {
        let mut rig = limited_candidate_fixture();
        let channel = rig.channel(0);
        // A real redo branch must survive failed/no-op candidate preparation.
        rig.session
            .dispatch(
                Command::UpdateSampler {
                    id: channel,
                    patch: SamplerPatch {
                        gain: Some(0.5),
                        ..Default::default()
                    },
                },
                None,
            )
            .unwrap();
        rig.session.undo().unwrap();
        let before = rig.session.document_snapshot();
        let original = rig.session.state().pool.clone();
        let bytes = original.sampler_retained_bytes();
        rig.session.controller().note_on(channel, 60, 0.8);
        let playback = rig.run(8192);
        assert!(playback.iter().any(|x| x.abs() > 0.01));
        let old_sample = before.project.channels[0].source.sample().unwrap();
        let sample = if mode == "noop" {
            old_sample
        } else {
            SampleId(before.project.next_id)
        };
        let audio = AudioBuffer::from_interleaved(
            48_000,
            2,
            vec![-0.125; if mode == "success" { 2048 } else { 524288 }],
        );
        let file = rig.file("candidate.wav");
        windfall_codec::write_wav(&file, &audio, windfall_codec::WavSampleFormat::Float32).unwrap();
        let audio = rig
            .session
            .inner
            .cache
            .decode(std::path::Path::new(&file))
            .unwrap();
        let command = if mode == "noop" {
            Command::SetChannelSample {
                id: channel,
                sample: Some(sample),
            }
        } else {
            Command::Batch {
                label: Some("Candidate import".into()),
                commands: vec![
                    Command::AddSample {
                        name: "Candidate".into(),
                        path: SamplePath::External(file),
                    },
                    Command::SetChannelSample {
                        id: channel,
                        sample: Some(sample),
                    },
                ],
            }
        };
        let ticket = {
            let _recording = rig.session.recording_idle().unwrap();
            let state = rig.session.state();
            rig.session
                .sample_edit_ticket(&state, command, None, vec![(sample, audio.clone())])
                .unwrap()
        };
        let prepared = ticket.prepare(); // neither State nor recording is acquired by the helper
        assert_eq!(rig.session.document_snapshot(), before);
        assert!(rig.session.state().pool.same_sources(&original));
        if mode == "budget" {
            assert!(prepared.err().unwrap().contains("budget"));
        } else {
            let _recording = rig.session.recording_idle().unwrap();
            let mut state = rig.session.state();
            prepared.unwrap().commit(&mut state).unwrap();
            if mode == "success" {
                assert_eq!(state.pool.get(sample).unwrap().identity(), audio.identity());
                assert!(state.loaded.contains(&sample));
                assert!(!state.loading.contains(&sample));
                assert!(!state.failed.contains(&sample));
                assert!(
                    !state
                        .pool
                        .needs_sampler_preparation(state.document.project())
                );
                drop(state);
                drop(_recording);
                let after = rig.session.document_snapshot();
                assert_eq!(after.history.cursor, before.history.cursor + 1);
                rig.session.undo().unwrap();
                let mut undone = before.project.clone();
                undone.next_id = after.project.next_id; // allocated ids are never reused by undo
                assert_eq!(rig.project(), undone);
                rig.session.redo().unwrap();
                assert_eq!(rig.project(), after.project);
                assert!(
                    !rig.session
                        .state()
                        .pool
                        .needs_sampler_preparation(&after.project)
                );
                assert_eq!(
                    rig.session.state().pool.get(sample).unwrap().identity(),
                    audio.identity()
                );
                continue;
            }
        }
        assert_eq!(rig.session.document_snapshot(), before);
        assert!(rig.session.state().pool.same_sources(&original));
        assert_eq!(rig.session.state().pool.sampler_retained_bytes(), bytes);
        rig.session.controller().note_on(channel, 60, 0.8);
        assert_eq!(rig.run(8192), playback);
        rig.session
            .export_audio(windfall_ipc::ExportOptions {
                path: rig.file("candidate-refusal.wav"),
                mode: windfall_ipc::PlayMode::Pattern,
                ..Default::default()
            })
            .unwrap();
        assert!(rig.events.wait_for_export().error.is_none());
    }
}

#[test]
fn sampler_r1_candidate_guards_validate_the_original_map_and_pending_loads_before_dispatch() {
    for change in [
        "generation",
        "edits",
        "replacements",
        "addition",
        "removal",
        "replacement",
        "loading",
    ] {
        let rig = limited_candidate_fixture();
        let command = command(rig.channel(0));
        let ticket = {
            let state = rig.session.state();
            rig.session
                .sample_edit_ticket(&state, command, None, Vec::new())
                .unwrap()
        };
        let prepared = ticket.prepare().unwrap();
        {
            let mut state = rig.session.state();
            let id = state.document.project().channels[0]
                .source
                .sample()
                .unwrap();
            match change {
                "generation" => state.generation += 1,
                "edits" => state.edits += 1,
                "replacements" => state.replacements += 1,
                "addition" => {
                    state.pool.insert(
                        windfall_project::SampleId(999),
                        AudioBuffer::from_interleaved(48_000, 1, vec![]),
                    );
                }
                "removal" => {
                    state.pool.remove(id);
                }
                "replacement" => {
                    state.pool.insert(
                        id,
                        AudioBuffer::from_interleaved(48_000, 1, vec![0.125; 4096]),
                    );
                }
                "loading" => {
                    state.loading.insert(id);
                }
                _ => unreachable!(),
            }
        }
        let before = rig.session.document_snapshot();
        let pool = rig.session.state().pool.clone();
        let _recording = rig.session.recording_idle().unwrap();
        let mut state = rig.session.state();
        assert!(prepared.commit(&mut state).is_err(), "{change}");
        assert!(state.pool.same_sources(&pool));
        drop(state);
        assert_eq!(rig.session.document_snapshot(), before);
        assert!(
            rig.session
                .controller()
                .sampler_key_supported(rig.channel(0), 60)
        );
    }
}

fn pending_load_publication(publication: &str) {
    use std::sync::atomic::Ordering;
    use windfall_project::{SampleId, SamplePath};
    let rig = Rig::new();
    let file = rig.file("pending.wav");
    let write = |frames, level| {
        windfall_codec::write_wav(
            &file,
            &AudioBuffer::from_interleaved(48_000, 1, vec![level; frames]),
            windfall_codec::WavSampleFormat::Float32,
        )
        .unwrap();
    };
    write(4096, 0.25);
    let sample = SampleId(rig.project().next_id);
    let channel = rig.channel(1);
    let hold = rig.session.hold("samples:decoded");
    rig.session
        .dispatch(
            Command::Batch {
                label: None,
                commands: vec![
                    Command::AddSample {
                        name: "Pending".into(),
                        path: SamplePath::External(file.clone()),
                    },
                    Command::SetChannelSample {
                        id: channel,
                        sample: Some(sample),
                    },
                    command(channel),
                ],
            },
            None,
        )
        .unwrap();
    hold.wait();
    let decoded = rig
        .session
        .inner
        .cache
        .peek(std::path::Path::new(&file))
        .unwrap();
    // A preview updates the path cache while its old decoded job awaits State.
    write(8192, -0.125);
    rig.session.sample_info(&file).unwrap();
    let newer = rig
        .session
        .inner
        .cache
        .peek(std::path::Path::new(&file))
        .unwrap();
    assert_ne!(decoded.identity(), newer.identity());
    let before = rig.session.document_snapshot();
    match publication {
        "candidate" => {
            let refused = {
                let state = rig.session.state();
                rig.session
                    .sample_edit_ticket(
                        &state,
                        Command::UpdateSampler {
                            id: channel,
                            patch: SamplerPatch {
                                tune: Some(1.0),
                                ..Default::default()
                            },
                        },
                        None,
                        vec![(sample, newer.clone())],
                    )
                    .err()
                    .unwrap()
            };
            assert!(refused.contains("already loading"));
            assert_eq!(rig.session.document_snapshot(), before);
            let added = SampleId(before.project.next_id);
            let ticket = {
                let _recording = rig.session.recording_idle().unwrap();
                let state = rig.session.state();
                rig.session
                    .sample_edit_ticket(
                        &state,
                        Command::AddSample {
                            name: "Unrelated".into(),
                            path: SamplePath::External(rig.file("unrelated.wav")),
                        },
                        None,
                        vec![(
                            added,
                            AudioBuffer::from_interleaved(48_000, 1, vec![0.125; 1024]),
                        )],
                    )
                    .unwrap()
            };
            let prepared = ticket.prepare().unwrap();
            let _recording = rig.session.recording_idle().unwrap();
            prepared.commit(&mut rig.session.state()).unwrap();
        }
        "history" => {
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
            rig.session.undo().unwrap();
            assert_eq!(rig.project(), before.project);
        }
        _ => unreachable!(),
    }
    {
        let state = rig.session.state();
        assert!(
            state.loading.contains(&sample),
            "{publication} superseded an outstanding decode"
        );
        assert!(!state.loaded.contains(&sample));
        assert!(!state.pool.contains(sample));
        assert!(!rig.session.controller().sampler_key_supported(channel, 60));
    }
    hold.release();
    rig.wait_until_loaded_or_failed(sample);
    let deadline = std::time::Instant::now() + super::PATIENCE;
    while rig.session.inner.preparing_samplers.load(Ordering::Acquire) {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let state = rig.session.state();
    assert_eq!(
        state.pool.get(sample).unwrap().identity(),
        decoded.identity()
    );
    assert!(!state.loading.contains(&sample));
    assert!(
        !state
            .pool
            .needs_sampler_preparation(state.document.project())
    );
    assert!(rig.session.controller().sampler_key_supported(channel, 60));
}

#[test]
fn sampler_r1_pending_load_candidate_does_not_attach_a_newer_unrelated_cached_source() {
    pending_load_publication("candidate");
}

#[test]
fn sampler_r1_pending_load_history_does_not_attach_a_newer_unrelated_cached_source() {
    pending_load_publication("history");
}
