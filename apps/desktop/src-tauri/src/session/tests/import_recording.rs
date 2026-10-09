//! Reviewed imports and in-flight project reads must preserve an active take.

use super::Rig;
use crate::session::recording::{CaptureHandle, Sink};
use windfall_flp::{Channel, ChannelKind, Note, Pattern};
use windfall_ipc::{FlpImportOptions, MidiImportOptions, RecordingSource};
use windfall_midi::{MidiNote, MidiSong, MidiTrack, WriteOptions};
use windfall_project::{Command, SettingsPatch};

#[path = "../../../../../../crates/windfall-flp/tests/common/mod.rs"]
// Keep this suite self-contained without changing the sibling FLP suite's helpers.
#[allow(clippy::duplicate_mod)]
mod fixtures;

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
    _source: RecordingSource,
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
        .expect("synthetic capture starts");
}

#[test]
fn recording_refuses_midi_confirmation_and_preserves_the_review_for_after_discard() {
    let rig = Rig::new();
    let path = rig.file("reviewed.mid");
    let song = MidiSong {
        tracks: vec![MidiTrack {
            name: Some("Reviewed MIDI".into()),
            notes: vec![MidiNote {
                start: 0,
                length: 960,
                key: 60,
                velocity: 100,
                release: 64,
                channel: 0,
            }],
            ..Default::default()
        }],
        length: 3840,
        ..Default::default()
    };
    windfall_midi::write_file(&path, &song, &WriteOptions::default()).unwrap();
    let review = rig
        .session
        .midi_preview(&path, MidiImportOptions::default())
        .unwrap();
    let before = rig.session.document_snapshot();
    start_take(&rig);
    let take = rig.session.recording_state();

    let error = rig.session.import_midi(review.token).unwrap_err();
    assert!(error.contains("Stop or cancel recording"), "{error}");
    assert_eq!(rig.session.document_snapshot(), before);
    assert_eq!(rig.session.recording_state(), take);

    rig.session.recording_cancel();
    assert_eq!(rig.session.document_snapshot(), before);
    rig.session.import_midi(review.token).unwrap();
    let imported = rig.session.document_snapshot();
    assert_eq!(
        imported.project.channels.len(),
        before.project.channels.len() + 1
    );
    assert_eq!(
        imported.project.channels.last().unwrap().name,
        "Reviewed MIDI"
    );
    assert_eq!(imported.history.cursor, before.history.cursor + 1);
}

#[test]
fn recording_refuses_flp_open_and_preserves_the_review_for_after_discard() {
    let rig = Rig::new();
    let mut project = fixtures::project(fixtures::MODERN, 96);
    project.channels.push(Channel {
        iid: 0,
        kind: ChannelKind::Generator,
        name: Some("Reviewed FL instrument".into()),
        ..Default::default()
    });
    project.patterns.push(Pattern {
        iid: 1,
        notes: vec![Note {
            channel: 0,
            length: 96,
            key: 60,
            velocity: 100,
            fine_pitch: 120,
            pan: 64,
            ..Default::default()
        }],
        ..Default::default()
    });
    let path = rig.file("reviewed.flp");
    std::fs::write(&path, fixtures::write(&project)).unwrap();
    let review = rig
        .session
        .flp_preview(&path, &FlpImportOptions::default())
        .unwrap();
    let before = rig.session.document_snapshot();
    start_take(&rig);
    let take = rig.session.recording_state();

    let error = rig.session.flp_open(review.token).unwrap_err();
    assert!(error.contains("Stop or cancel recording"), "{error}");
    assert_eq!(rig.session.document_snapshot(), before);
    assert_eq!(rig.session.recording_state(), take);

    rig.session.recording_cancel();
    assert_eq!(rig.session.document_snapshot(), before);
    let imported = rig.session.flp_open(review.token).unwrap();
    assert!(imported.dirty && imported.path.is_none());
    assert_eq!(imported.project.channels[0].name, "Reviewed FL instrument");
    assert_eq!(imported.project.patterns[0].lanes[0].notes.len(), 1);
}

#[test]
fn a_project_read_started_before_capture_cannot_install_during_the_take() {
    let rig = Rig::new();
    let path = rig
        .session
        .project_save(Some(&rig.file("saved.windfall")))
        .unwrap();
    let saved = rig.project();
    rig.session
        .dispatch(
            Command::UpdateSettings {
                patch: SettingsPatch {
                    name: Some("Current edited project".into()),
                    ..Default::default()
                },
            },
            None,
        )
        .unwrap();
    let before = rig.session.document_snapshot();
    assert_ne!(before.project, saved);
    let hold = rig.session.hold("open:install");
    let open_path = path.clone();
    let work = rig
        .session
        .background(move |session| session.project_open(&open_path));
    hold.wait();
    start_take(&rig);
    let take = rig.session.recording_state();
    hold.release();

    let error = work.join().expect("read completes").unwrap_err();
    assert!(error.contains("Stop or cancel recording"), "{error}");
    assert_eq!(rig.session.document_snapshot(), before);
    assert_eq!(rig.session.recording_state(), take);
    assert!(take.active);

    rig.session.recording_cancel();
    assert_eq!(rig.session.document_snapshot(), before);
    assert_eq!(rig.session.project_open(&path).unwrap().project, saved);
}
