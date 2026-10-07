//! Prepared clip edits and history must preserve ownership of an active take.
use super::{Rig, factory_file};
use crate::session::ClipPlace;
use crate::session::recording::{CaptureHandle, Sink};
use windfall_ipc::RecordingSource;
use windfall_project::{
    AudioClipPatch, AudioClipUpdate, ClipId, ClipStretch, ClipStretchQuality, Command,
};

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
fn add_source_clip(rig: &Rig) -> ClipId {
    let added = rig
        .session
        .add_audio_clip_from_file(
            &factory_file("Bass/Bass Sub.wav"),
            ClipPlace {
                track: None,
                start: 0,
                mixer_track: None,
            },
        )
        .unwrap();
    ClipId(*added.created.last().unwrap())
}
fn spectral_command(id: ClipId) -> Command {
    Command::UpdateAudioClips {
        updates: vec![AudioClipUpdate {
            id,
            patch: AudioClipPatch {
                stretch: Some(ClipStretch::Spectral {
                    ratio: 1.5,
                    quality: ClipStretchQuality::Standard,
                    formants: true,
                }),
                pitch: Some(7.0),
                ..Default::default()
            },
        }],
    }
}
fn assert_document_preserved(rig: &Rig, before: &windfall_project::DocumentSnapshot) {
    let after = rig.session.document_snapshot();
    assert_eq!(after.project, before.project);
    assert_eq!(after.history, before.history);
    assert_eq!(after.dirty, before.dirty);
    assert!(rig.session.recording_state().active);
    assert_eq!(rig.session.recording_state().frames, 480);
}
#[test]
fn active_take_rejects_spectral_clip_preparation_without_changing_document_or_history() {
    let rig = Rig::new();
    let clip = add_source_clip(&rig);
    start_take(&rig);
    let before = rig.session.document_snapshot();
    let error = rig
        .session
        .prepare_clip_command(spectral_command(clip))
        .unwrap_err();
    assert!(error.contains("recording"), "{error}");
    assert_document_preserved(&rig, &before);
    rig.session.recording_cancel();
    rig.session
        .prepare_clip_command(spectral_command(clip))
        .unwrap();
    assert_ne!(rig.project().playlist, before.project.playlist);
}
#[test]
fn clip_prepared_before_capture_cannot_apply_during_take_and_can_be_retried_after_discard() {
    let rig = Rig::new();
    let clip = add_source_clip(&rig);
    let command = spectral_command(clip);
    let hold = rig.session.hold("clip:prepared");
    let pending = command.clone();
    let worker = rig
        .session
        .background(move |session| session.prepare_clip_command(pending));
    hold.wait();
    // Both document and recording ownership must be released during preparation.
    start_take(&rig);
    let before = rig.session.document_snapshot();
    hold.release();
    let error = worker.join().unwrap().unwrap_err();
    assert!(error.contains("recording"), "{error}");
    assert_document_preserved(&rig, &before);
    rig.session.recording_cancel();
    rig.session.prepare_clip_command(command).unwrap();
    assert_ne!(rig.project().playlist, before.project.playlist);
}
#[test]
fn undo_redo_and_history_jump_preserve_spectral_history_while_a_take_is_active() {
    let rig = Rig::new();
    let clip = add_source_clip(&rig);
    rig.session
        .prepare_clip_command(spectral_command(clip))
        .unwrap();
    let spectral = rig.session.document_snapshot();
    // Undo/jump backwards have a real spectral edit to remove.
    start_take(&rig);
    assert!(rig.session.undo().is_none());
    assert!(rig.session.redo().is_none());
    rig.session.history_jump(0);
    assert_document_preserved(&rig, &spectral);
    rig.session.recording_cancel();
    rig.session.undo().unwrap();
    let undone = rig.session.document_snapshot();
    assert_ne!(undone.project.playlist, spectral.project.playlist);
    // Redo/jump forwards have a real spectral edit to restore.
    start_take(&rig);
    assert!(rig.session.undo().is_none());
    assert!(rig.session.redo().is_none());
    rig.session.history_jump(spectral.history.cursor);
    assert_document_preserved(&rig, &undone);
    rig.session.recording_cancel();
    rig.session.redo().unwrap();
    assert_eq!(rig.project().playlist, spectral.project.playlist);
}
