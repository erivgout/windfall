//! Region publication guards and actual WAV export through the native session.
use super::{Rig, SAMPLE_RATE, factory_file, rms};
use crate::events::Event;
use crate::session::ClipPlace;
use crate::session::recording::{CaptureHandle, Sink};
use crate::sync::lock;
use std::{fs, path::Path, sync::mpsc, time::Duration};
use windfall_engine::{RenderOptions, render};
use windfall_ipc::{
    BitDepth, ExportFormat, ExportOptions, PlayMode, RecordingSource, TimelinePlaybackState,
    TransportPatch,
};
use windfall_project::{Command, TickRange, TrackId};

struct Capture;
impl CaptureHandle for Capture {
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
fn capture(_: RecordingSource, _: u32, mut sink: Sink) -> Result<Box<dyn CaptureHandle>, String> {
    sink(&[0.25; 960])?;
    Ok(Box::new(Capture))
}

fn refuse_transport(rig: &Rig, guard: TimelinePlaybackState) {
    let timeline = rig.session.timeline_state();
    let transport = rig.session.transport_state();
    let tick = rig.session.controller().frame().tick;
    let document = rig.session.document_snapshot();
    assert!(
        rig.session
            .timeline_transport_set(
                TransportPatch {
                    mode: Some(PlayMode::Song),
                    loop_song: Some(true),
                    ..Default::default()
                },
                guard
            )
            .is_err()
    );
    assert!(rig.session.timeline_transport_seek(711.0, guard).is_err());
    assert!(rig.session.timeline_transport_play(guard).is_err());
    assert_eq!(rig.session.timeline_state(), timeline);
    assert_eq!(rig.session.transport_state(), transport);
    assert_eq!(rig.session.controller().frame().tick, tick);
    let after = rig.session.document_snapshot();
    assert_eq!(after.project, document.project);
    assert_eq!(after.revision, document.revision);
    assert_eq!(after.dirty, document.dirty);
    assert_eq!(after.history, document.history);
}

#[test]
fn timeline_request_ordering_and_safe_limit_refuse_old_arm_clear_without_mutation() {
    let mut rig = Rig::new();
    rig.session
        .add_audio_clip_from_file(
            &factory_file("Bass/Bass Sub.wav"),
            ClipPlace {
                track: None,
                start: 0,
                mixer_track: None,
            },
        )
        .unwrap();
    let source = rig.session.timeline_state();
    let region = Some(TickRange {
        start: 17,
        end: 839,
    });
    let arm = rig
        .session
        .timeline_region_request(region, source.generation, source.revision, Some(10))
        .unwrap();
    rig.session
        .timeline_region_request(None, source.generation, source.revision, Some(12))
        .unwrap();
    assert!(
        rig.session
            .timeline_region_request(region, source.generation, source.revision, Some(11))
            .is_err()
    );
    refuse_transport(&rig, arm);
    let rearm = rig
        .session
        .timeline_region_request(
            Some(TickRange {
                start: 91,
                end: 210,
            }),
            source.generation,
            source.revision,
            Some(14),
        )
        .unwrap();
    assert!(
        rig.session
            .timeline_region_request(None, source.generation, source.revision, Some(13))
            .is_err()
    );
    assert_eq!(rig.session.timeline_state(), rearm);
    refuse_transport(&rig, TimelinePlaybackState { region, ..rearm });
    refuse_transport(
        &rig,
        TimelinePlaybackState {
            request: 13,
            ..rearm
        },
    );
    rig.session
        .timeline_transport_set(
            TransportPatch {
                mode: Some(PlayMode::Song),
                loop_song: Some(false),
                ..Default::default()
            },
            rearm,
        )
        .unwrap();
    rig.session.timeline_transport_seek(91.0, rearm).unwrap();
    assert!(rig.session.timeline_transport_play(rearm).unwrap().playing);
    assert!(rms(&rig.run(600)) > 1e-5);
    rig.session.transport_stop();
    let max = super::super::timeline::MAX_TIMELINE_REQUEST;
    rig.session
        .timeline_region_request(None, source.generation, source.revision, Some(max - 1))
        .unwrap();
    assert_eq!(
        rig.session
            .timeline_region(region, source.generation, source.revision)
            .unwrap()
            .request,
        max
    );
    let exhausted = rig.session.timeline_state();
    assert!(
        rig.session
            .timeline_region_request(region, source.generation, source.revision, Some(max + 1))
            .is_err()
    );
    assert!(
        rig.session
            .timeline_region(region, source.generation, source.revision)
            .is_err()
    );
    assert_eq!(rig.session.timeline_state(), exhausted);
}

#[test]
fn timeline_transport_guards_refuse_edited_new_open_and_recording_sources_before_mutation() {
    let rig = Rig::new();
    let saved = rig
        .session
        .project_save(Some(&rig.file("guarded-song")))
        .unwrap();
    for change in 0..3 {
        let source = rig.session.timeline_state();
        let guard = rig
            .session
            .timeline_region(
                Some(TickRange {
                    start: 17,
                    end: 839,
                }),
                source.generation,
                source.revision,
            )
            .unwrap();
        match change {
            0 => {
                rig.session
                    .dispatch(Command::AddPattern { name: None }, None)
                    .unwrap();
            }
            1 => {
                rig.session.project_new().unwrap();
            }
            _ => {
                rig.session.project_open(&saved).unwrap();
            }
        }
        refuse_transport(&rig, guard);
    }
    let source = rig.session.timeline_state();
    let guard = rig
        .session
        .timeline_region(
            Some(TickRange {
                start: 17,
                end: 839,
            }),
            source.generation,
            source.revision,
        )
        .unwrap();
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
            capture,
        )
        .unwrap();
    let timeline = rig.session.timeline_state();
    assert!(
        rig.session
            .timeline_region_request(
                None,
                source.generation,
                source.revision,
                Some(timeline.request + 1)
            )
            .is_err()
    );
    refuse_transport(&rig, guard);
    rig.session.recording_cancel();
    // Unguarded legacy transport retains its accepted signature and behavior.
    rig.session
        .transport_set(TransportPatch {
            mode: Some(PlayMode::Pattern),
            ..Default::default()
        })
        .unwrap();
    rig.session.transport_seek(27.0);
    assert!(rig.session.transport_play().unwrap().playing);
}

#[test]
fn timeline_regions_refuse_stale_edits_replacement_invalid_bounds_and_active_recording_atomically()
{
    let rig = Rig::new();
    let saved = rig.session.project_save(Some(&rig.file("opened"))).unwrap();
    let state = rig.session.timeline_state();
    let region = Some(TickRange {
        start: 17,
        end: 839,
    });
    rig.session
        .timeline_region(region, state.generation, state.revision)
        .unwrap();
    assert_eq!(rig.session.document_snapshot().revision, state.revision);
    assert!(
        rig.session
            .timeline_region(
                Some(TickRange { start: 3, end: 3 }),
                state.generation,
                state.revision
            )
            .is_err()
    );
    assert_eq!(rig.session.timeline_state().region, region);
    rig.session
        .dispatch(Command::AddPattern { name: None }, None)
        .unwrap();
    assert!(
        rig.session
            .timeline_region(None, state.generation, state.revision)
            .is_err()
    );
    assert_eq!(rig.session.timeline_state().region, region);
    let fresh = rig.session.timeline_state();
    let guarded_export = ExportOptions {
        path: rig.file("stale-region.wav"),
        region,
        region_generation: Some(state.generation),
        region_revision: Some(state.revision),
        mode: PlayMode::Song,
        ..Default::default()
    };
    assert!(
        rig.session
            .export_audio(guarded_export.clone())
            .unwrap_err()
            .contains("changed")
    );
    assert!(!Path::new(&guarded_export.path).exists());
    let partial_guard = ExportOptions {
        region_revision: None,
        ..guarded_export.clone()
    };
    assert!(
        rig.session
            .export_audio(partial_guard)
            .unwrap_err()
            .contains("both")
    );
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
            capture,
        )
        .unwrap();
    let before = rig.session.document_snapshot();
    assert!(
        rig.session
            .timeline_region(None, fresh.generation, fresh.revision)
            .is_err()
    );
    assert_eq!(rig.session.timeline_state().region, region);
    assert_eq!(rig.session.document_snapshot().project, before.project);
    rig.session.recording_cancel();
    rig.session.project_new().unwrap();
    assert_eq!(rig.session.timeline_state().region, None);
    assert!(
        rig.session
            .timeline_region(region, fresh.generation, fresh.revision)
            .is_err()
    );
    assert!(
        rig.session
            .export_audio(ExportOptions {
                region_generation: Some(fresh.generation),
                region_revision: Some(fresh.revision),
                ..guarded_export
            })
            .unwrap_err()
            .contains("changed")
    );
    let before_open = rig.session.timeline_state();
    rig.session.project_open(&saved).unwrap();
    assert_eq!(rig.session.timeline_state().region, None);
    assert!(
        rig.session
            .export_audio(ExportOptions {
                path: rig.file("stale-open.wav"),
                region,
                region_generation: Some(before_open.generation),
                region_revision: Some(before_open.revision),
                ..Default::default()
            })
            .unwrap_err()
            .contains("changed")
    );
    assert!(!Path::new(&rig.file("stale-open.wav")).exists());
}

#[test]
fn timeline_native_wav_region_readback_matches_renderer_through_pdc_and_tails() {
    let rig = Rig::new();
    rig.session
        .add_audio_clip_from_file(
            &factory_file("Bass/Bass Sub.wav"),
            ClipPlace {
                track: None,
                start: 0,
                mixer_track: None,
            },
        )
        .unwrap();
    rig.session
        .dispatch(
            Command::AddEffect {
                track: TrackId::MASTER,
                kind: windfall_project::EffectKind::Limiter,
                index: None,
            },
            None,
        )
        .unwrap();
    let range = TickRange {
        start: 83,
        end: 837,
    };
    let project = rig.project();
    let pool = rig.session.state().pool.clone();
    let options = RenderOptions {
        sample_rate: SAMPLE_RATE,
        mode: PlayMode::Song,
        region: Some(range),
        tail_secs: 0.05,
        ..Default::default()
    };
    let expected = render(&project, &pool, &options, &mut |_| true);
    assert!(rms(expected.samples()) > 0.01);
    let source = rig.session.timeline_state();
    let export = ExportOptions {
        region: Some(range),
        region_generation: Some(source.generation),
        region_revision: Some(source.revision),
        path: rig.file("region.wav"),
        format: ExportFormat::Wav,
        bit_depth: BitDepth::Float32,
        sample_rate: SAMPLE_RATE,
        mode: PlayMode::Song,
        tail_secs: options.tail_secs,
        auto_tail: false,
        ..Default::default()
    };
    rig.session.export_audio(export.clone()).unwrap();
    let done = rig.events.wait_for_export();
    assert_eq!(done.error, None);
    assert_ne!(done.cancelled, Some(true));
    let readback = windfall_codec::decode_file(&export.path).unwrap();
    assert_eq!(
        readback.frames(),
        (range.end - range.start) as usize * 25 + 2400
    );
    assert_eq!(readback.samples(), expected.samples());
    assert_eq!(rig.session.timeline_state().region, None); // exporting never changes live transport
}

#[test]
fn timeline_native_region_export_cancel_preserves_destination_after_project_replacement() {
    let rig = Rig::new();
    fs::create_dir(rig.file("region-output")).unwrap();
    let path = rig.file("region-output/cancel-region.wav");
    let original = b"existing destination";
    fs::write(&path, original).unwrap();
    let options = ExportOptions {
        region: Some(TickRange {
            start: 17,
            end: 3839,
        }),
        path: path.clone(),
        format: ExportFormat::Wav,
        bit_depth: BitDepth::Float32,
        sample_rate: SAMPLE_RATE,
        mode: PlayMode::Song,
        tail_secs: 0.0,
        auto_tail: false,
        ..Default::default()
    };
    let (reached, wait_reached) = mpsc::channel();
    let (release, wait_release) = mpsc::channel();
    let mut held = false;
    *lock(&rig.events.hook) = Some(Box::new(move |event| {
        if matches!(event, Event::ExportProgress(_)) && !held {
            held = true;
            reached.send(()).unwrap();
            wait_release.recv().unwrap();
        }
    }));
    rig.session.export_audio(options.clone()).unwrap();
    wait_reached.recv_timeout(Duration::from_secs(10)).unwrap();
    rig.session.project_new().unwrap();
    rig.session.export_cancel();
    release.send(()).unwrap();
    let done = rig.events.wait_for_export();
    assert_eq!(done.cancelled, Some(true));
    assert_eq!(done.error, None);
    assert_eq!(done.path, path);
    assert_eq!(fs::read(&path).unwrap(), original);
    // There are no staging files left beside the preserved destination.
    assert_eq!(
        fs::read_dir(Path::new(&path).parent().unwrap())
            .unwrap()
            .count(),
        1
    );
    rig.events.take();
    rig.session.export_audio(options).unwrap();
    assert_eq!(rig.events.wait_for_export().error, None);
    let silent = windfall_codec::decode_file(&path).unwrap();
    assert_eq!(silent.frames(), (3839 - 17) * 25);
    assert!(silent.samples().iter().all(|sample| *sample == 0.0));
}
