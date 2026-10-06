//! The transport, the realtime feed, the audio device and previews.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use windfall_ipc::{AudioSettings, PlayMode, RealtimeFrame, TransportPatch};
use windfall_project::{ClipContent, ClipInit, Command, PatternId, PlaylistTrackId};

use super::{FakeDevice, Rig, SAMPLE_RATE, factory_file, rms};
use crate::events::Event;
use crate::session::FRAME_INTERVAL;
use crate::sync::lock;

fn step_on(rig: &Rig, channel: usize, step: u32) {
    rig.session
        .dispatch(
            Command::ToggleStep {
                pattern: rig.pattern(),
                channel: rig.channel(channel),
                step,
            },
            None,
        )
        .unwrap();
}

#[test]
fn the_transport_is_announced_when_it_changes_and_only_then() {
    let rig = Rig::new();
    let session = &rig.session;

    assert!(session.transport_play().playing);
    assert!(session.transport_play().playing);
    assert!(!session.transport_toggle().playing);
    assert!(!session.transport_stop().playing);
    assert!(session.transport_toggle().playing);
    let song = session
        .transport_set(TransportPatch {
            mode: Some(PlayMode::Song),
            loop_song: Some(true),
            ..TransportPatch::default()
        })
        .unwrap();
    assert_eq!(session.transport_state(), song);
    // Setting what is already set changes nothing.
    session.transport_set(TransportPatch::default()).unwrap();
    session.transport_seek(480.0);

    let states = rig.take_transport_states();
    let seen: Vec<(bool, PlayMode, bool)> = states
        .iter()
        .map(|state| (state.playing, state.mode, state.loop_song))
        .collect();
    assert_eq!(
        seen,
        [
            (true, PlayMode::Pattern, false),
            (false, PlayMode::Pattern, false),
            (true, PlayMode::Pattern, false),
            (true, PlayMode::Song, true),
        ]
    );
    assert_eq!(states.last(), Some(&song));
}

#[test]
fn the_transport_cannot_be_pointed_at_a_pattern_that_does_not_exist() {
    let rig = Rig::new();
    let before = rig.session.transport_state();
    let error = rig
        .session
        .transport_set(TransportPatch {
            mode: Some(PlayMode::Song),
            pattern: Some(PatternId(999)),
            ..TransportPatch::default()
        })
        .unwrap_err();
    assert_eq!(error, "pattern 999 does not exist");
    assert_eq!(rig.session.transport_state(), before);
    assert!(rig.events.take().is_empty());
}

#[test]
fn stopping_returns_the_playhead_to_where_playback_started() {
    let mut rig = Rig::new();
    let session = rig.session.clone();
    session.transport_seek(960.0);
    session.transport_play();
    rig.run(SAMPLE_RATE as usize / 2);
    // Half a second at 120 bpm is one beat.
    let frame = session.realtime_tick();
    assert!((frame.tick - 1_920.0).abs() < 1.0, "tick {}", frame.tick);

    session.transport_stop();
    rig.run(480);
    assert_eq!(session.realtime_tick().tick, 960.0);
}

#[test]
fn every_subscriber_gets_the_same_frame_and_a_closed_one_is_dropped() {
    let mut rig = Rig::new();
    let session = rig.session.clone();
    step_on(&rig, 0, 0);

    let inbox = |open: bool| {
        let frames = Arc::new(Mutex::new(Vec::<RealtimeFrame>::new()));
        let sink = frames.clone();
        let sender = Box::new(move |frame: &RealtimeFrame| {
            lock(&sink).push(frame.clone());
            open
        });
        (frames, sender)
    };
    let (main, to_main) = inbox(true);
    let (detached, to_detached) = inbox(true);
    let (closing, to_closing) = inbox(false);
    session.subscribe_realtime("main", to_main);
    session.subscribe_realtime("mixer", to_detached);
    session.subscribe_realtime("closing", to_closing);

    session.transport_play();
    rig.run(4_800);
    let first = session.realtime_tick();
    assert!(first.playing);
    assert!(first.meters[0] > 0.1, "{:?}", first.meters);
    assert_eq!(first.xruns, 0);

    // The meters are peaks since the last frame: read once, they are gone.
    // Both windows must therefore be sent the one frame that was read.
    assert_eq!(*lock(&main), std::slice::from_ref(&first));
    assert_eq!(*lock(&detached), std::slice::from_ref(&first));
    assert_eq!(*lock(&closing), [first]);

    rig.run(4_800);
    let second = session.realtime_tick();
    assert_eq!(lock(&main).len(), 2);
    assert_eq!(lock(&main)[1], second);
    assert_eq!(lock(&detached).len(), 2);
    assert_eq!(
        lock(&closing).len(),
        1,
        "a closed window is not sent to again"
    );

    // A window that subscribes again replaces its old channel.
    let (reloaded, to_reloaded) = inbox(true);
    session.subscribe_realtime("main", to_reloaded);
    session.unsubscribe_realtime("mixer");
    session.realtime_tick();
    assert_eq!(lock(&main).len(), 2);
    assert_eq!(lock(&detached).len(), 2);
    assert_eq!(lock(&reloaded).len(), 1);
}

#[test]
fn a_song_that_reaches_its_end_stops_and_the_ui_is_told() {
    let mut rig = Rig::new();
    let session = rig.session.clone();
    step_on(&rig, 0, 0);
    let track = session
        .dispatch(Command::AddPlaylistTrack { name: None }, None)
        .unwrap()
        .created[0];
    session
        .dispatch(
            Command::AddClips {
                clips: vec![ClipInit {
                    track: PlaylistTrackId(track),
                    start: 0,
                    length: None,
                    content: ClipContent::Pattern {
                        pattern: rig.pattern(),
                    },
                }],
            },
            None,
        )
        .unwrap();
    session
        .transport_set(TransportPatch {
            mode: Some(PlayMode::Song),
            loop_song: Some(false),
            ..TransportPatch::default()
        })
        .unwrap();
    assert!(session.transport_play().playing);
    rig.events.take();

    // One bar at 120 bpm is two seconds. Half-way through it still plays.
    rig.run(SAMPLE_RATE as usize);
    assert!(session.realtime_tick().playing);
    assert!(rig.events.take().is_empty());

    rig.run(SAMPLE_RATE as usize * 3 / 2);
    assert!(!session.realtime_tick().playing);
    let states = rig.take_transport_states();
    assert_eq!(states.len(), 1);
    assert!(!states[0].playing);
    assert_eq!(states[0].mode, PlayMode::Song);

    // Told once, not on every frame.
    session.realtime_tick();
    assert!(rig.events.take().is_empty());
}

#[test]
fn the_realtime_thread_ticks_sixty_times_a_second_and_ends_with_the_session() {
    let rig = Rig::new();
    let count = Arc::new(AtomicUsize::new(0));
    let counter = count.clone();
    rig.session.subscribe_realtime(
        "main",
        Box::new(move |_| {
            counter.fetch_add(1, Ordering::SeqCst);
            true
        }),
    );
    let thread = rig.session.spawn_realtime().unwrap();
    let started = Instant::now();
    std::thread::sleep(Duration::from_millis(600));
    let frames = count.load(Ordering::SeqCst) as f64;
    let expected = started.elapsed().as_secs_f64() / FRAME_INTERVAL.as_secs_f64();
    // A loaded test machine can hold a thread up, so the bounds are loose;
    // the real rate is measured against the running app.
    assert!(
        frames > expected * 0.7 && frames < expected * 1.1 + 2.0,
        "{frames} frames where about {expected:.0} were due"
    );

    drop(rig);
    thread.join().unwrap();
}

#[test]
fn configuring_the_engine_remembers_the_settings_and_announces_the_status() {
    let rig = Rig::new();
    let session = &rig.session;
    assert_eq!(session.engine_status().buffer_frames, 480);
    session.transport_play();
    rig.events.take();

    let settings = AudioSettings {
        host: None,
        device: None,
        sample_rate: Some(44_100),
        buffer_frames: Some(128),
    };
    let status = session.engine_configure(settings.clone());
    assert!(status.running);
    assert_eq!((status.sample_rate, status.buffer_frames), (44_100, 128));
    assert_eq!(session.engine_status(), status);
    assert!(
        rig.events
            .take()
            .contains(&Event::EngineStatus(status.clone()))
    );

    // A device that cannot be opened is a status, not a failed call.
    let failed = session.engine_configure(AudioSettings {
        device: Some("Unplugged".to_owned()),
        ..settings.clone()
    });
    assert!(!failed.running);
    assert!(failed.error.as_deref().unwrap().contains("Unplugged"));
    assert_eq!(rig.events.take(), [Event::EngineStatus(failed)]);

    session.engine_configure(settings);
    let rig = rig.restart();
    // The fake device opens with what the settings file holds.
    assert_eq!(rig.session.engine_status(), status);
}

#[test]
fn a_status_that_changes_by_itself_is_announced_once() {
    let rig = Rig::new();
    rig.session.poll_engine_status();
    assert!(rig.events.take().is_empty());

    let mut lost = FakeDevice::describe(&AudioSettings::default());
    lost.running = false;
    lost.error = Some("the device was unplugged".to_owned());
    *lock(&rig.device.status) = lost.clone();
    rig.session.poll_engine_status();
    rig.session.poll_engine_status();
    assert_eq!(rig.events.take(), [Event::EngineStatus(lost)]);

    let back = FakeDevice::describe(&AudioSettings::default());
    *lock(&rig.device.status) = back.clone();
    rig.session.poll_engine_status();
    assert_eq!(rig.events.take(), [Event::EngineStatus(back)]);
}

#[test]
fn a_preview_plays_a_file_without_the_transport_and_can_be_stopped() {
    let mut rig = Rig::new();
    let session = rig.session.clone();
    let crash = factory_file("Drums/Cymbals/Crash.wav");

    session.preview_play(&crash).unwrap();
    assert!(rms(&rig.run(4_800)) > 0.01);
    assert!(!session.transport_state().playing);
    // The master meter shows it.
    assert!(session.realtime_tick().meters[0] > 0.01);

    session.preview_stop();
    rig.run(4_800);
    assert!(rms(&rig.run(4_800)) < 1e-4, "the preview is still sounding");

    let error = session.preview_play(&rig.file("nothing.wav")).unwrap_err();
    assert!(error.contains("nothing.wav"), "{error}");
    assert!(rig.events.take().is_empty());
}
