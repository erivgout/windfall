//! Real processor navigation and linear region rendering, without an audio device.
use windfall_engine::{RenderOptions, StemOptions, render, render_stems, render_streaming_checked};
use windfall_ipc::{PlayMode, StemMode, TransportPatch};
use windfall_project::{MarkerKind, TickRange, TimelineMarker, TrackId};

use crate::realtime::allocator_calls;
use crate::support::{Rig, counted, counting, left, run};

const RATE: u32 = 48_000;

fn tape() -> Rig {
    let mut rig = Rig::new();
    rig.project.settings.tempo_bpm = 120.0; // exactly 25 frames per tick
    let lane = rig.playlist_track();
    let clip = rig.audio_clip(lane, counting(RATE, 100_000), TrackId::MASTER, 0, 3_840);
    rig.clip_mut(clip).offset = 20;
    rig
}

fn marker(rig: &mut Rig, tick: u32, kind: MarkerKind) {
    let id = rig.project.next_id;
    rig.project.next_id += 1;
    rig.project.playlist.timeline.markers.push(TimelineMarker {
        id,
        tick,
        name: "Navigation".into(),
        kind,
    });
}

#[test]
fn timeline_skip_pause_resume_and_seek_drive_the_processor_without_callback_allocations() {
    let mut rig = tape();
    marker(&mut rig, 13, MarkerKind::Skip { end: 31 });
    marker(&mut rig, 53, MarkerKind::Pause);
    rig.project.check().unwrap();
    let (mut processor, controller) = rig.song_processor(RATE);
    controller.play();
    let mut audio = vec![0.0; (13 + 22) * 25 * 2];
    assert_eq!(allocator_calls(|| processor.process(&mut audio)), 0);
    assert_eq!(left(&audio)[200], counted(20 * 25 + 200));
    // The skipped source resumes at its own offset, on the first boundary frame.
    assert_eq!(left(&audio)[(13 + 8) * 25], counted((20 + 31 + 8) * 25));
    assert!(!controller.transport().playing);
    assert_eq!(controller.frame().tick, 53.0);
    controller.play();
    let resumed = run(&mut processor, 300, 37);
    assert!(controller.transport().playing);
    assert_eq!(left(&resumed)[200], counted((20 + 53) * 25 + 200));
    controller.seek(53.0); // an explicit seek re-arms the pause
    let mut out = [0.0; 2];
    assert_eq!(allocator_calls(|| processor.process(&mut out)), 0);
    assert!(!controller.transport().playing);
    assert_eq!(controller.navigation_overflows(), 0);
}

#[test]
fn timeline_loop_marker_and_selected_region_precedence_stop_and_seek_are_real() {
    let mut rig = tape();
    marker(&mut rig, 11, MarkerKind::Loop { end: 23 });
    let (mut processor, controller) = rig.song_processor(RATE);
    controller.set_transport(TransportPatch {
        loop_song: Some(true),
        ..Default::default()
    });
    controller.play();
    let audio = left(&run(&mut processor, 34 * 25, 61));
    assert_eq!(audio[31 * 25], counted((20 + 19) * 25));
    controller
        .set_timeline_region(Some(TickRange { start: 31, end: 47 }))
        .unwrap();
    controller.seek(0.0); // clamped to selection start
    let audio = left(&run(&mut processor, 29 * 25, 79));
    assert_eq!(audio[27 * 25], counted((20 + 42) * 25));
    controller.set_transport(TransportPatch {
        loop_song: Some(false),
        ..Default::default()
    });
    run(&mut processor, 20 * 25, 53);
    assert!(!controller.transport().playing);
    assert_eq!(controller.frame().tick, 47.0);
    assert!(
        controller
            .set_timeline_region(Some(TickRange { start: 2, end: 2 }))
            .is_err()
    );
    assert_eq!(
        controller.timeline_region(),
        Some(TickRange { start: 31, end: 47 })
    );
}

#[test]
fn timeline_tiny_region_loop_has_an_observable_bounded_callback_limit() {
    let rig = tape();
    let (mut processor, controller) = rig.song_processor(RATE);
    controller
        .set_timeline_region(Some(TickRange { start: 1, end: 2 }))
        .unwrap();
    controller.set_transport(TransportPatch {
        loop_song: Some(true),
        ..Default::default()
    });
    controller.play();
    let mut audio = vec![0.0; 4_096 * 2];
    assert_eq!(allocator_calls(|| processor.process(&mut audio)), 0);
    assert!(!controller.transport().playing);
    assert_eq!(controller.navigation_overflows(), 1);
    assert_eq!(controller.frame().tick, 2.0);
}

#[test]
fn timeline_region_is_linear_sample_aligned_and_identical_in_buffer_streams_and_stems() {
    let mut rig = tape();
    rig.project.settings.tempo_bpm = 137.0; // fractional frame endpoints
    marker(&mut rig, 19, MarkerKind::Skip { end: 37 });
    marker(&mut rig, 61, MarkerKind::Pause);
    let options = RenderOptions {
        mode: PlayMode::Song,
        region: Some(TickRange { start: 7, end: 83 }),
        ..Default::default()
    };
    let baseline = render(&rig.project, &rig.pool, &options, &mut |_| true);
    let fpt = 60.0 * f64::from(RATE) / (137.0 * 960.0);
    assert_eq!(
        baseline.frames(),
        (83.0 * fpt).ceil() as usize - (7.0 * fpt).ceil() as usize
    );
    assert!(baseline.samples().iter().any(|s| *s > 0.0));
    rig.project.playlist.timeline.markers.clear();
    assert_eq!(
        baseline.samples(),
        render(&rig.project, &rig.pool, &options, &mut |_| true).samples()
    );
    for block_frames in [1, 37, 1_024] {
        let options = RenderOptions {
            block_frames,
            ..options.clone()
        };
        let mut streamed = Vec::new();
        let outcome = render_streaming_checked(
            &rig.project,
            &rig.pool,
            &options,
            &mut |block| {
                streamed.extend_from_slice(block);
                true
            },
            &mut |_| true,
        )
        .unwrap();
        assert!(outcome.completed);
        assert_eq!(streamed, baseline.samples());
        let mut mix = Vec::new();
        render_stems(
            &rig.project,
            &rig.pool,
            &options,
            &StemOptions {
                mode: StemMode::TrackOutputs,
                tracks: Some(Vec::new()),
                include_mix: true,
                numbered: false,
            },
            &mut |_, block| {
                mix.extend_from_slice(block);
                true
            },
            &mut |_| true,
        )
        .unwrap();
        assert_eq!(mix, baseline.samples());
    }
}

#[test]
fn timeline_regions_with_tempo_ramps_all_clip_sources_offsets_pdc_and_tails_match_live_and_streaming()
 {
    let mut rig = crate::rendering::tape();
    let lane = rig.playlist_track();
    let tempo = rig.automation(
        windfall_project::AutomationTarget::Tempo,
        &[(0, (60.0 - 10.0) / 512.0), (4000, (180.0 - 10.0) / 512.0)],
    );
    rig.automation_clip(lane, tempo, 0, 4000);
    let range = TickRange {
        start: 131,
        end: 2039,
    };
    let options = RenderOptions {
        mode: PlayMode::Song,
        region: Some(range),
        tail_secs: 0.2,
        ..Default::default()
    };
    let rendered = render(&rig.project, &rig.pool, &options, &mut |_| true);
    assert!(crate::support::peak(rendered.samples()) > 0.01);
    let seconds = |tick: u32| {
        60.0 / 960.0 * 4000.0 / 120.0 * (1.0 + 120.0 / 60.0 * f64::from(tick) / 4000.0).ln()
    };
    let body = (seconds(range.end) * f64::from(RATE)).ceil() as usize
        - (seconds(range.start) * f64::from(RATE)).ceil() as usize;
    assert_eq!(rendered.frames(), body + 9600);
    let (mut processor, controller) = rig.song_processor(RATE);
    let latency = controller.latency_frames() as usize;
    assert_eq!(latency, 480);
    controller.set_timeline_region(Some(range)).unwrap();
    controller.seek(f64::from(range.start));
    controller.play();
    let played = run(&mut processor, rendered.frames() + latency, 37);
    assert_eq!(rendered.samples(), &played[latency * 2..]);
    assert!(!controller.transport().playing);
    assert_eq!(controller.frame().tick, f64::from(range.end));
    for auto_tail in [false, true] {
        let options = RenderOptions {
            auto_tail,
            block_frames: 79,
            ..options.clone()
        };
        let buffer = render(&rig.project, &rig.pool, &options, &mut |_| true);
        let mut streamed = Vec::new();
        render_streaming_checked(
            &rig.project,
            &rig.pool,
            &options,
            &mut |block| {
                streamed.extend_from_slice(block);
                true
            },
            &mut |_| true,
        )
        .unwrap();
        assert_eq!(buffer.samples(), streamed);
    }
}

#[test]
fn timeline_invalid_regions_and_cancelled_streams_never_call_sink_as_a_success() {
    let rig = tape();
    let invalid = RenderOptions {
        mode: PlayMode::Song,
        region: Some(TickRange { start: 8, end: 8 }),
        ..Default::default()
    };
    let mut called = false;
    assert!(
        render_streaming_checked(
            &rig.project,
            &rig.pool,
            &invalid,
            &mut |_| {
                called = true;
                true
            },
            &mut |_| true
        )
        .is_err()
    );
    assert!(!called);
    let options = RenderOptions {
        region: Some(TickRange { start: 8, end: 900 }),
        ..invalid
    };
    let cancelled = render_streaming_checked(
        &rig.project,
        &rig.pool,
        &options,
        &mut |_| false,
        &mut |_| true,
    )
    .unwrap();
    assert!(!cancelled.completed);
    assert!(cancelled.frames <= options.block_frames as u64);
}

#[test]
fn timeline_fractional_selected_start_keeps_marker_and_note_onsets_on_the_first_sample() {
    let mut rig = tape();
    rig.project.settings.tempo_bpm = 137.0;
    marker(&mut rig, 7, MarkerKind::Skip { end: 13 });
    marker(&mut rig, 23, MarkerKind::Pause);
    let per_tick = f64::from(RATE) * 60.0 / (137.0 * 960.0);
    let frames = (23.0 * per_tick).ceil() as usize - (13.0 * per_tick).ceil() as usize;
    let (mut processor, controller) = rig.song_processor(RATE);
    controller
        .set_timeline_region(Some(TickRange { start: 7, end: 83 }))
        .unwrap();
    controller.play();
    let mut audio = vec![0.0; frames * 2];
    assert_eq!(allocator_calls(|| processor.process(&mut audio)), 0);
    assert!(
        !controller.transport().playing,
        "the skip at the fractional start was lost"
    );
    assert_eq!(controller.frame().tick, 23.0);

    let mut notes = Rig::new();
    notes.project.settings.tempo_bpm = 137.0;
    let sampler = notes.channel(crate::support::level(RATE, 0.5, 1.0));
    notes.note(sampler, 7, 100);
    let lane = notes.playlist_track();
    let pattern = notes.first_pattern();
    notes.clip(lane, pattern, 0, 960);
    let options = RenderOptions {
        mode: PlayMode::Song,
        region: Some(TickRange { start: 7, end: 83 }),
        ..Default::default()
    };
    let rendered = render(&notes.project, &notes.pool, &options, &mut |_| true);
    assert!(
        crate::support::peak(rendered.samples()) > 0.4,
        "the note at the fractional start was lost"
    );
    let mut streamed = Vec::new();
    render_streaming_checked(
        &notes.project,
        &notes.pool,
        &options,
        &mut |block| {
            streamed.extend_from_slice(block);
            true
        },
        &mut |_| true,
    )
    .unwrap();
    assert_eq!(rendered.samples(), streamed);
}

#[test]
fn timeline_pause_at_natural_end_and_selected_end_loop_tie_are_not_retriggered() {
    let mut rig = tape();
    marker(&mut rig, 3840, MarkerKind::Pause);
    let (mut processor, controller) = rig.song_processor(RATE);
    controller.play();
    run(&mut processor, 3840 * 25 + 500, 53);
    assert!(!controller.transport().playing);
    assert_eq!(controller.frame().tick, 3840.0);
    controller
        .set_timeline_region(Some(TickRange {
            start: 3830,
            end: 3840,
        }))
        .unwrap();
    controller.set_transport(TransportPatch {
        loop_song: Some(true),
        ..Default::default()
    });
    controller.seek(0.0);
    controller.play();
    run(&mut processor, 24 * 25, 79);
    assert!(controller.transport().playing);
    assert_eq!(controller.frame().tick, 3834.0);
    assert_eq!(controller.navigation_overflows(), 0);
}
