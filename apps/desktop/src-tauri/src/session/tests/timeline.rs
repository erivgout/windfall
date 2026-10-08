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

#[test]
fn timeline_chained_unpublished_successors_cancel_the_live_native_owner_only() {
    for scenario in 0..3 {
        let ordinary = scenario == 1;
        let mut rig = Rig::new();
        let a = seed_selection(&rig);
        // B and C are prepared off State and do not become native authority
        // before publication. A can still commit while their requests wait.
        let b_request = a.request + 1;
        let c_request = a.request + 2;
        rig.session.timeline_transport_play(a).unwrap();
        assert_eq!(play_owner(&rig), a.request);
        assert!(rms(&rig.run(600)) > 1e-5);
        if ordinary {
            rig.session.transport_seek(99.0);
        }
        if scenario == 2 {
            rig.run(25_000);
            assert!(!rig.session.transport_state().playing);
            assert_eq!(play_owner(&rig), a.request);
        }
        let tick = rig.session.controller().frame().tick;
        let document = rig.session.document_snapshot();
        let cleared = rig
            .session
            .timeline_region_request(None, a.generation, a.revision, Some(a.request + 3), Some(a))
            .unwrap();
        assert_eq!(cleared.region, None);
        assert_eq!(rig.session.transport_state().playing, ordinary);
        assert_eq!(play_owner(&rig), 0);
        if scenario == 2 {
            // Naturally stopped ownership is retired without a second Stop:
            // the accepted endpoint/automation hold remains available to tails.
            assert_eq!(rig.session.controller().frame().tick, tick);
        }
        for stale in [c_request, b_request] {
            assert!(
                rig.session
                    .timeline_region_request(
                        a.region,
                        a.generation,
                        a.revision,
                        Some(stale),
                        Some(a)
                    )
                    .is_err()
            );
            assert_eq!(rig.session.timeline_state(), cleared);
        }
        if !ordinary {
            rig.run(2000);
            assert!(rms(&rig.run(600)) < 1e-7);
        }
        let after = rig.session.document_snapshot();
        assert_eq!(after.project, document.project);
        assert_eq!(after.revision, document.revision);
        assert_eq!(after.history, document.history);
        assert_eq!(after.dirty, document.dirty);
        let rearm = rig
            .session
            .timeline_region_request(
                a.region,
                a.generation,
                a.revision,
                Some(a.request + 4),
                None,
            )
            .unwrap();
        rig.session.timeline_transport_seek(17.0, rearm).unwrap();
        rig.session.timeline_transport_play(rearm).unwrap();
        assert_eq!(play_owner(&rig), rearm.request);
        assert!(rms(&rig.run(600)) > 1e-5);
    }
}

fn play_owner(rig: &Rig) -> u64 {
    let _state = rig.session.state();
    rig.session
        .inner
        .timeline_play_request
        .load(std::sync::atomic::Ordering::Relaxed)
}

fn seed_selection(rig: &Rig) -> TimelinePlaybackState {
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
    let arm = rig
        .session
        .timeline_region_request(
            Some(TickRange {
                start: 17,
                end: 839,
            }),
            source.generation,
            source.revision,
            Some(10),
            None,
        )
        .unwrap();
    rig.session
        .timeline_transport_set(
            TransportPatch {
                mode: Some(PlayMode::Song),
                loop_song: Some(false),
                ..Default::default()
            },
            arm,
        )
        .unwrap();
    rig.session.timeline_transport_seek(17.0, arm).unwrap();
    arm
}

#[test]
fn timeline_cancel_before_play_commit_refuses_old_play_and_permits_later_valid_play() {
    let mut rig = Rig::new();
    let arm = seed_selection(&rig);
    rig.session
        .timeline_region_request(None, arm.generation, arm.revision, Some(11), Some(arm))
        .unwrap();
    assert!(rig.session.timeline_transport_play(arm).is_err());
    assert!(!rig.session.transport_state().playing);
    assert_eq!(play_owner(&rig), 0);
    let newer = rig
        .session
        .timeline_region_request(
            Some(TickRange {
                start: 91,
                end: 210,
            }),
            arm.generation,
            arm.revision,
            Some(12),
            None,
        )
        .unwrap();
    rig.session.timeline_transport_seek(91.0, newer).unwrap();
    assert!(rig.session.timeline_transport_play(newer).unwrap().playing);
    assert_eq!(play_owner(&rig), 12);
    assert!(rms(&rig.run(600)) > 1e-5);
    // A late stale cancellation cannot remove the newer region or its Play.
    assert!(
        rig.session
            .timeline_region_request(None, arm.generation, arm.revision, Some(11), Some(arm))
            .is_err()
    );
    assert_eq!(rig.session.timeline_state(), newer);
    assert!(rig.session.transport_state().playing);
    assert_eq!(play_owner(&rig), 12);
}

#[test]
fn timeline_cancellation_does_not_stop_superseding_ordinary_transport_or_settled_play() {
    for ordinary in 0..5 {
        let mut rig = Rig::new();
        let arm = seed_selection(&rig);
        rig.session.timeline_transport_play(arm).unwrap();
        assert_eq!(play_owner(&rig), arm.request);
        match ordinary {
            0 => {
                rig.session.transport_play().unwrap();
            }
            1 => {
                rig.session.transport_seek(99.0);
            }
            2 => {
                rig.session
                    .transport_set(TransportPatch {
                        loop_song: Some(true),
                        ..Default::default()
                    })
                    .unwrap();
            }
            3 => {
                rig.session.transport_stop();
                assert_eq!(play_owner(&rig), 0);
                rig.session.transport_play().unwrap();
            }
            _ => {} // Clear after settled selected Play carries no cancellation.
        }
        let normal = rig.session.transport_state();
        let tick = rig.session.controller().frame().tick;
        assert!(normal.playing);
        if ordinary < 4 {
            assert_eq!(play_owner(&rig), 0);
        }
        rig.session
            .timeline_region_request(
                None,
                arm.generation,
                arm.revision,
                Some(11),
                (ordinary < 4).then_some(arm),
            )
            .unwrap();
        assert_eq!(rig.session.transport_state(), normal);
        assert_eq!(rig.session.controller().frame().tick, tick);
        assert_eq!(rig.session.timeline_state().region, None);
        assert!(rms(&rig.run(600)) > 1e-5);
    }
}

#[test]
fn timeline_cancel_refusals_preserve_owner_and_full_canonical_state_until_fresh_retry() {
    let rig = Rig::new();
    let arm = seed_selection(&rig);
    rig.session.timeline_transport_play(arm).unwrap();
    let transport = rig.session.transport_state();
    let tick = rig.session.controller().frame().tick;
    let document = rig.session.document_snapshot();
    assert!(
        rig.session
            .transport_set(TransportPatch {
                pattern: Some(windfall_project::PatternId(u32::MAX)),
                ..Default::default()
            })
            .is_err()
    );
    for (range, request, cancel) in [
        (None, arm.request, arm),
        (Some(TickRange { start: 3, end: 3 }), arm.request + 1, arm),
        (
            None,
            arm.request + 1,
            TimelinePlaybackState {
                request: super::super::timeline::MAX_TIMELINE_REQUEST + 1,
                ..arm
            },
        ),
    ] {
        assert!(
            rig.session
                .timeline_region_request(
                    range,
                    arm.generation,
                    arm.revision,
                    Some(request),
                    Some(cancel)
                )
                .is_err()
        );
        assert_eq!(rig.session.timeline_state(), arm);
        assert_eq!(rig.session.transport_state(), transport);
        assert_eq!(rig.session.controller().frame().tick, tick);
        assert_eq!(play_owner(&rig), arm.request);
    }
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
    assert!(
        rig.session
            .timeline_region_request(None, arm.generation, arm.revision, Some(11), Some(arm))
            .is_err()
    );
    assert!(
        rig.session
            .transport_set(TransportPatch {
                loop_song: Some(true),
                ..Default::default()
            })
            .is_err()
    );
    rig.session.transport_seek(111.0); // recording refusal remains a legacy no-op
    assert_eq!(rig.session.timeline_state(), arm);
    assert_eq!(rig.session.transport_state(), transport);
    assert_eq!(rig.session.controller().frame().tick, tick);
    assert_eq!(play_owner(&rig), arm.request);
    let after = rig.session.document_snapshot();
    assert_eq!(after.project, document.project);
    assert_eq!(after.revision, document.revision);
    assert_eq!(after.history, document.history);
    assert_eq!(after.dirty, document.dirty);
    rig.session.recording_cancel();
    rig.session
        .dispatch(Command::AddPattern { name: None }, None)
        .unwrap();
    assert!(
        rig.session
            .timeline_region_request(None, arm.generation, arm.revision, Some(11), Some(arm))
            .is_err()
    );
    assert_eq!(play_owner(&rig), arm.request);
    assert_eq!(rig.session.transport_state(), transport);
    let fresh = rig.session.timeline_state();
    assert_eq!(fresh.region, arm.region);
    assert_eq!(fresh.request, arm.request);
    let edited = rig.session.document_snapshot();
    rig.session
        .timeline_region_request(
            None,
            fresh.generation,
            fresh.revision,
            Some(11),
            Some(TimelinePlaybackState {
                revision: fresh.revision,
                ..arm
            }),
        )
        .unwrap();
    assert!(!rig.session.transport_state().playing);
    assert_eq!(rig.session.timeline_state().region, None);
    assert_eq!(play_owner(&rig), 0);
    let after = rig.session.document_snapshot();
    assert_eq!(after.project, edited.project);
    assert_eq!(after.revision, edited.revision);
    assert_eq!(after.history, edited.history);
    assert_eq!(after.dirty, edited.dirty);
}

#[test]
fn timeline_failed_ordinary_play_preserves_owner_after_natural_region_stop() {
    let mut rig = Rig::new();
    let arm = seed_selection(&rig);
    rig.session.timeline_transport_play(arm).unwrap();
    rig.run(25_000); // the processor naturally reaches selected end 839
    assert!(!rig.session.transport_state().playing);
    assert_eq!(play_owner(&rig), arm.request);
    let clips = rig
        .project()
        .playlist
        .clips
        .iter()
        .map(|clip| clip.id)
        .collect();
    rig.session
        .dispatch(Command::RemoveClips { clips }, None)
        .unwrap();
    let canonical = rig.session.timeline_state();
    let tick = rig.session.controller().frame().tick;
    let document = rig.session.document_snapshot();
    assert!(rig.session.transport_play().is_err());
    assert_eq!(rig.session.timeline_state(), canonical);
    assert_eq!(rig.session.controller().frame().tick, tick);
    assert_eq!(play_owner(&rig), arm.request);
    assert!(!rig.session.transport_state().playing);
    let after = rig.session.document_snapshot();
    assert_eq!(after.project, document.project);
    assert_eq!(after.revision, document.revision);
    assert_eq!(after.history, document.history);
    assert_eq!(after.dirty, document.dirty);
}

#[test]
fn timeline_cancelled_pending_play_committed_during_clear_query_stops_its_audio() {
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
    let arm = rig
        .session
        .timeline_region_request(
            Some(TickRange {
                start: 17,
                end: 839,
            }),
            source.generation,
            source.revision,
            Some(10),
            None,
        )
        .unwrap();
    rig.session
        .timeline_transport_set(
            TransportPatch {
                mode: Some(PlayMode::Song),
                loop_song: Some(false),
                ..Default::default()
            },
            arm,
        )
        .unwrap();
    rig.session.timeline_transport_seek(17.0, arm).unwrap();
    let document = rig.session.document_snapshot();
    // The source query waits off State. The cancelled Play may still commit
    // before the newer ordered clear reaches the native decision.
    let (queried, wait_query) = mpsc::channel();
    let (release, wait_release) = mpsc::channel();
    let session = rig.session.clone();
    let clear = std::thread::spawn(move || {
        let state = session.timeline_state();
        queried.send(()).unwrap();
        wait_release.recv().unwrap();
        session.timeline_region_request(None, state.generation, state.revision, Some(11), Some(arm))
    });
    wait_query.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(rig.session.timeline_transport_play(arm).unwrap().playing);
    assert!(rms(&rig.run(600)) > 1e-5);
    release.send(()).unwrap();
    clear.join().unwrap().unwrap();
    assert!(!rig.session.transport_state().playing);
    assert_eq!(rig.session.timeline_state().region, None);
    rig.run(2000); // drain the existing note/audio interrupt fade
    assert!(rms(&rig.run(600)) < 1e-7);
    let after = rig.session.document_snapshot();
    assert_eq!(after.project, document.project);
    assert_eq!(after.revision, document.revision);
    assert_eq!(after.history, document.history);
    assert_eq!(after.dirty, document.dirty);
}
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
        .timeline_region_request(region, source.generation, source.revision, Some(10), None)
        .unwrap();
    rig.session
        .timeline_region_request(None, source.generation, source.revision, Some(12), None)
        .unwrap();
    assert!(
        rig.session
            .timeline_region_request(region, source.generation, source.revision, Some(11), None)
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
            None,
        )
        .unwrap();
    assert!(
        rig.session
            .timeline_region_request(None, source.generation, source.revision, Some(13), None)
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
        .timeline_region_request(
            None,
            source.generation,
            source.revision,
            Some(max - 1),
            None,
        )
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
            .timeline_region_request(
                region,
                source.generation,
                source.revision,
                Some(max + 1),
                None
            )
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
                Some(timeline.request + 1),
                None,
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
