//! Export: what is refused, what runs, and what it writes.

use std::path::Path;
use std::sync::mpsc;

use windfall_ipc::{BitDepth, ExportFormat, ExportOptions, PlayMode, TransportPatch};
use windfall_project::{ClipContent, ClipInit, Command, PatternId, PlaylistTrackId};

use super::{Rig, SAMPLE_RATE, rms};
use crate::events::Event;
use crate::sync::lock;

fn options(rig: &Rig, name: &str) -> ExportOptions {
    ExportOptions {
        path: rig.file(name),
        format: ExportFormat::Wav,
        bit_depth: BitDepth::Float32,
        sample_rate: SAMPLE_RATE,
        mode: PlayMode::Pattern,
        pattern_loops: 1,
        tail_secs: 0.0,
    }
}

fn kick_on_the_beat(rig: &Rig) {
    for step in [0, 4, 8, 12] {
        rig.session
            .dispatch(
                Command::ToggleStep {
                    pattern: rig.pattern(),
                    channel: rig.channel(0),
                    step,
                },
                None,
            )
            .unwrap();
    }
}

#[test]
fn an_export_that_cannot_work_is_refused_before_it_starts() {
    let rig = Rig::new();
    let session = &rig.session;
    let refused = |change: &dyn Fn(&mut ExportOptions)| {
        let mut options = options(&rig, "refused.wav");
        change(&mut options);
        session.export_audio(options).unwrap_err()
    };

    assert_eq!(
        refused(&|o| o.path = String::new()),
        "Choose where to save the file."
    );
    assert_eq!(
        refused(&|o| o.path = "beat.wav".to_owned()),
        "\"beat.wav\" is not a full path"
    );
    assert_eq!(
        refused(&|o| o.pattern_loops = 0),
        "Render the pattern at least once."
    );
    assert_eq!(
        refused(&|o| o.sample_rate = 12),
        "12 Hz is not a sample rate Windfall can export."
    );
    assert_eq!(
        refused(&|o| o.tail_secs = f32::NAN),
        "The tail must be zero seconds or longer."
    );
    assert_eq!(
        refused(&|o| o.mode = PlayMode::Song),
        "The playlist is empty, so there is no song to export."
    );
    // A hundred thousand bars would need tens of gigabytes of memory.
    let error = refused(&|o| o.pattern_loops = 100_000);
    assert!(error.contains("minutes long"), "{error}");

    assert!(rig.events.take().is_empty());
    assert!(!Path::new(&rig.file("refused.wav")).exists());
    // None of the refusals left the session thinking an export is running.
    session.export_audio(options(&rig, "fine.wav")).unwrap();
    assert_eq!(rig.events.wait_for_export().error, None);
}

#[test]
fn a_second_export_is_refused_while_one_runs_and_edits_do_not_reach_it() {
    let rig = Rig::new();
    let session = &rig.session;
    kick_on_the_beat(&rig);

    // Holds the export at its first progress event until the test lets go.
    let (reached, wait_reached) = mpsc::channel();
    let (release, wait_release) = mpsc::channel::<()>();
    let mut held = false;
    *lock(&rig.events.hook) = Some(Box::new(move |event| {
        if matches!(event, Event::ExportProgress(_)) && !held {
            held = true;
            reached.send(()).unwrap();
            wait_release.recv().unwrap();
        }
    }));

    let first = options(&rig, "first.wav");
    session.export_audio(first.clone()).unwrap();
    wait_reached.recv().unwrap();

    assert_eq!(
        session
            .export_audio(options(&rig, "second.wav"))
            .unwrap_err(),
        "An export is already running."
    );

    // Editing while the export runs is safe: it has its own copy.
    session
        .dispatch(
            Command::ClearLane {
                pattern: rig.pattern(),
                channel: rig.channel(0),
            },
            None,
        )
        .unwrap();
    session.project_new();

    release.send(()).unwrap();
    let done = rig.events.wait_for_export();
    assert_eq!(done.error, None);
    assert_eq!(done.path, first.path);
    assert!(!Path::new(&rig.file("second.wav")).exists());
    let audio = windfall_codec::decode_file(&first.path).unwrap();
    assert!(rms(audio.samples()) > 0.02, "the edit reached the export");

    // And now another may run.
    rig.events.take();
    session.export_audio(options(&rig, "second.wav")).unwrap();
    assert_eq!(rig.events.wait_for_export().error, None);
    assert!(Path::new(&rig.file("second.wav")).is_file());
}

#[test]
fn an_export_that_fails_says_why_in_its_last_event() {
    let rig = Rig::new();
    let session = &rig.session;
    let nowhere = options(&rig, "no such folder/beat.wav");

    session.export_audio(nowhere.clone()).unwrap();
    let done = rig.events.wait_for_export();
    assert_eq!(done.path, nowhere.path);
    let error = done.error.unwrap();
    assert!(error.starts_with("Could not export to \""), "{error}");
    assert!(error.contains("beat.wav"), "{error}");

    rig.events.take();
    session.export_audio(options(&rig, "after.wav")).unwrap();
    assert_eq!(rig.events.wait_for_export().error, None);
}

#[test]
fn pattern_mode_exports_the_pattern_the_transport_has_selected() {
    let rig = Rig::new();
    let session = &rig.session;
    kick_on_the_beat(&rig);
    let empty = session
        .dispatch(Command::AddPattern { name: None }, None)
        .unwrap()
        .created[0];
    let export = |name: &str| {
        rig.events.take();
        let options = ExportOptions {
            sample_rate: 44_100,
            ..options(&rig, name)
        };
        session.export_audio(options.clone()).unwrap();
        assert_eq!(rig.events.wait_for_export().error, None);
        windfall_codec::decode_file(&options.path).unwrap()
    };

    let beat = export("beat.wav");
    // One bar at 120 bpm, at the rate asked for and not the engine's.
    assert_eq!((beat.sample_rate(), beat.frames()), (44_100, 88_200));
    assert!(rms(beat.samples()) > 0.02);

    session
        .transport_set(TransportPatch {
            pattern: Some(PatternId(empty)),
            ..TransportPatch::default()
        })
        .unwrap();
    let silence = export("silence.wav");
    assert_eq!(silence.frames(), 88_200);
    assert_eq!(rms(silence.samples()), 0.0);
}

#[test]
fn song_mode_exports_the_playlist_to_the_end_of_its_last_clip() {
    let rig = Rig::new();
    let session = &rig.session;
    kick_on_the_beat(&rig);
    let track = session
        .dispatch(Command::AddPlaylistTrack { name: None }, None)
        .unwrap()
        .created[0];
    let bar = rig.project().patterns[0].length_ticks();
    session
        .dispatch(
            Command::AddClips {
                clips: vec![ClipInit {
                    track: PlaylistTrackId(track),
                    // One silent bar, then three bars of the pattern.
                    start: bar,
                    length: Some(bar * 3),
                    content: ClipContent::Pattern {
                        pattern: rig.pattern(),
                    },
                }],
            },
            None,
        )
        .unwrap();

    let options = ExportOptions {
        mode: PlayMode::Song,
        // Loops are a pattern-mode setting and are ignored for a song.
        pattern_loops: 0,
        tail_secs: 0.25,
        bit_depth: BitDepth::Int16,
        ..options(&rig, "song.wav")
    };
    session.export_audio(options.clone()).unwrap();
    assert_eq!(rig.events.wait_for_export().error, None);

    let audio = windfall_codec::decode_file(&options.path).unwrap();
    let bar_frames = SAMPLE_RATE as usize * 2;
    assert_eq!(audio.frames(), bar_frames * 4 + SAMPLE_RATE as usize / 4);
    let bars = audio.samples().chunks(bar_frames * 2).collect::<Vec<_>>();
    // 16-bit silence still carries dither.
    assert!(rms(bars[0]) < 1e-4, "the first bar should be silent");
    for bar in &bars[1..4] {
        assert!(rms(bar) > 0.02);
    }
    // Exporting does not move the transport.
    assert!(!session.transport_state().playing);
    assert_eq!(session.transport_state().mode, PlayMode::Pattern);
}
