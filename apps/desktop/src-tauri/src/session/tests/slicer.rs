//! Synthetic session verification; no real device or OS audio claims.
use super::Rig;
use crate::session::{
    ClipPlace,
    recording::{CaptureHandle, Sink},
};
use windfall_codec::{WavSampleFormat, write_wav};
use windfall_core::AudioBuffer;
use windfall_ipc::{RecordingSource, SliceOptions};
use windfall_project::{
    AudioClipPatch, AudioClipUpdate, ClipId, ClipPatch, ClipUpdate, Command, SettingsPatch,
};

fn source_clip(rig: &Rig) -> ClipId {
    let path = rig.folder.path().join("original.wav");
    let mut samples = vec![0.0; 48_000 * 4];
    for begin in [24_000, 48_000, 96_000] {
        samples[begin..begin + 2_400].fill(0.5);
    }
    write_wav(
        &path,
        &AudioBuffer::from_interleaved(48_000, 1, samples),
        WavSampleFormat::Int24,
    )
    .unwrap();
    let result = rig
        .session
        .add_audio_clip_from_file(
            path.to_str().unwrap(),
            ClipPlace {
                start: 120,
                track: None,
                mixer_track: None,
            },
        )
        .unwrap();
    let id = ClipId(*result.created.last().unwrap());
    rig.session
        .dispatch(
            Command::UpdateClips {
                updates: vec![ClipUpdate {
                    id,
                    patch: ClipPatch {
                        length: Some(2880),
                        offset: Some(240),
                        ..Default::default()
                    },
                }],
            },
            None,
        )
        .unwrap();
    rig.session
        .dispatch(
            Command::UpdateAudioClips {
                updates: vec![AudioClipUpdate {
                    id,
                    patch: AudioClipPatch {
                        reverse: Some(true),
                        pitch: Some(6.0),
                        gain: Some(0.7),
                        pan: Some(-0.3),
                        ..Default::default()
                    },
                }],
            },
            None,
        )
        .unwrap();
    id
}
fn grid() -> SliceOptions {
    SliceOptions::Grid { grid_ticks: 960 }
}
fn edit(rig: &Rig) {
    let name = if rig.project().settings.name == "Changed" {
        "Changed again"
    } else {
        "Changed"
    };
    rig.session
        .dispatch(
            Command::UpdateSettings {
                patch: SettingsPatch {
                    name: Some(name.into()),
                    ..Default::default()
                },
            },
            None,
        )
        .unwrap();
}
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
    sink(&[0.25; 480])?;
    Ok(Box::new(Capture))
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
            0,
            None,
            capture,
        )
        .unwrap();
}

#[test]
fn slicer_refuses_tempo_automation_without_document_or_source_mutation() {
    use windfall_project::AutomationTarget;
    let rig = Rig::new();
    let id = source_clip(&rig);
    rig.session.automate(AutomationTarget::Tempo).unwrap();
    let before = rig.session.document_snapshot();
    let path = rig.folder.path().join("original.wav");
    let bytes = std::fs::read(&path).unwrap();
    for options in [grid(), SliceOptions::Transients { sensitivity: 0.5 }] {
        assert!(
            rig.session
                .slice_analyze(id, options)
                .unwrap_err()
                .contains("tempo automation")
        );
    }
    assert_eq!(rig.session.document_snapshot(), before);
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn slicer_splits_linked_source_once_and_undo_redo_save_reopen_preserve_it() {
    let rig = Rig::new();
    let id = source_clip(&rig);
    let before = rig.session.document_snapshot();
    let path = rig.folder.path().join("original.wav");
    let bytes = std::fs::read(&path).unwrap();
    let review = rig.session.slice_analyze(id, grid()).unwrap();
    assert_eq!(rig.project(), before.project);
    let result = rig
        .session
        .slice_apply(review.token, vec![840, 1800])
        .unwrap();
    assert_eq!(result.created.len(), 3);
    let sliced = rig.session.document_snapshot();
    assert_eq!(sliced.history.cursor, before.history.cursor + 1);
    assert_eq!(
        sliced.history.entries.last().unwrap().label,
        "Slice audio clip"
    );
    assert_eq!(sliced.project.samples, before.project.samples);
    let original = before
        .project
        .playlist
        .clips
        .iter()
        .find(|c| c.id == id)
        .unwrap();
    let slices: Vec<_> = sliced
        .project
        .playlist
        .clips
        .iter()
        .filter(|c| result.created.contains(&c.id.0))
        .collect();
    assert_eq!(
        slices
            .iter()
            .map(|c| (c.start, c.length, c.offset))
            .collect::<Vec<_>>(),
        [(120, 840, 240), (960, 960, 1080), (1920, 1080, 2040)]
    );
    for c in slices {
        assert_eq!(c.content, original.content);
    }
    assert!(rig.session.slice_apply(review.token, vec![840]).is_err());
    rig.session.undo().unwrap();
    assert_eq!(rig.project().playlist, before.project.playlist);
    rig.session.redo().unwrap();
    assert_eq!(rig.project(), sliced.project);
    let save = rig.file("sliced.windfall");
    rig.session.project_save(Some(&save)).unwrap();
    rig.session.project_open(&save).unwrap();
    assert_eq!(rig.project().playlist, sliced.project.playlist);
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn slicer_refuses_stale_edit_undo_source_replacement_and_document_replacement() {
    let rig = Rig::new();
    let id = source_clip(&rig);
    let review = rig.session.slice_analyze(id, grid()).unwrap();
    edit(&rig);
    rig.session.undo().unwrap();
    let before = rig.session.document_snapshot();
    assert!(
        rig.session
            .slice_apply(review.token, vec![840])
            .unwrap_err()
            .contains("changed")
    );
    assert_eq!(rig.session.document_snapshot(), before);
    let review = rig.session.slice_analyze(id, grid()).unwrap();
    let sample = rig
        .project()
        .playlist
        .clips
        .iter()
        .find(|c| c.id == id)
        .unwrap()
        .content
        .sample()
        .unwrap();
    rig.session.state().pool.insert(
        sample,
        AudioBuffer::from_interleaved(48_000, 1, vec![0.0; 48_000 * 4]),
    );
    assert!(
        rig.session
            .slice_apply(review.token, vec![840])
            .unwrap_err()
            .contains("changed")
    );
    let review = rig.session.slice_analyze(id, grid()).unwrap();
    rig.session.project_new().unwrap();
    let before = rig.session.document_snapshot();
    assert!(rig.session.slice_apply(review.token, vec![840]).is_err());
    assert_eq!(rig.session.document_snapshot(), before);
}

#[test]
fn slicer_analysis_and_compilation_release_state_and_recheck_edits() {
    let rig = Rig::new();
    let id = source_clip(&rig);
    let held = rig.session.hold("slice:analyzed");
    let work = rig.session.background(move |s| s.slice_analyze(id, grid()));
    held.wait();
    edit(&rig);
    let before = rig.session.document_snapshot();
    held.release();
    assert!(work.join().unwrap().unwrap_err().contains("changed"));
    assert_eq!(rig.session.document_snapshot(), before);
    let review = rig.session.slice_analyze(id, grid()).unwrap();
    let held = rig.session.hold("slice:prepared");
    let work = rig
        .session
        .background(move |s| s.slice_apply(review.token, vec![840]));
    held.wait();
    edit(&rig);
    let before = rig.session.document_snapshot();
    held.release();
    assert!(work.join().unwrap().unwrap_err().contains("changed"));
    assert_eq!(rig.session.document_snapshot(), before);
}

#[test]
fn slicer_active_recording_and_recording_started_during_work_reject_without_mutation() {
    let rig = Rig::new();
    let id = source_clip(&rig);
    let review = rig.session.slice_analyze(id, grid()).unwrap();
    start_take(&rig);
    let before = rig.session.document_snapshot();
    assert!(rig.session.slice_analyze(id, grid()).is_err());
    assert!(rig.session.slice_apply(review.token, vec![840]).is_err());
    assert_eq!(rig.session.document_snapshot(), before);
    assert!(rig.session.recording_state().active);
    rig.session.recording_cancel();
    for point in ["slice:analyzed", "slice:prepared"] {
        let review = rig.session.slice_analyze(id, grid()).unwrap();
        let held = rig.session.hold(point);
        let work = rig.session.background(move |s| {
            if point == "slice:analyzed" {
                s.slice_analyze(id, grid()).map(|_| ())
            } else {
                s.slice_apply(review.token, vec![840]).map(|_| ())
            }
        });
        held.wait();
        start_take(&rig);
        let before = rig.session.document_snapshot();
        held.release();
        assert!(work.join().unwrap().is_err());
        assert_eq!(rig.session.document_snapshot(), before);
        assert!(rig.session.recording_state().active);
        rig.session.recording_cancel();
    }
}

#[test]
fn slicer_latest_token_discard_and_invalid_markers_do_not_mutate() {
    let rig = Rig::new();
    let id = source_clip(&rig);
    let old = rig.session.slice_analyze(id, grid()).unwrap();
    let review = rig.session.slice_analyze(id, grid()).unwrap();
    let before = rig.session.document_snapshot();
    assert!(rig.session.slice_apply(old.token, vec![840]).is_err());
    for markers in [vec![], vec![1], vec![840, 840], vec![1800, 840]] {
        assert!(rig.session.slice_apply(review.token, markers).is_err());
        assert_eq!(rig.session.document_snapshot(), before);
    }
    rig.session.slice_discard(old.token); // an obsolete close cannot discard a newer review
    let held = rig.session.hold("slice:prepared");
    let work = rig
        .session
        .background(move |s| s.slice_apply(review.token, vec![840]));
    held.wait();
    rig.session.slice_discard(review.token);
    held.release();
    assert!(work.join().unwrap().is_err());
    assert_eq!(rig.session.document_snapshot(), before);
}

#[test]
fn slicer_source_replacement_during_analysis_is_rejected() {
    let rig = Rig::new();
    let id = source_clip(&rig);
    let sample = rig
        .project()
        .playlist
        .clips
        .iter()
        .find(|c| c.id == id)
        .unwrap()
        .content
        .sample()
        .unwrap();
    let held = rig.session.hold("slice:analyzed");
    let work = rig.session.background(move |s| s.slice_analyze(id, grid()));
    held.wait();
    rig.session.state().pool.insert(
        sample,
        AudioBuffer::from_interleaved(48_000, 1, vec![0.0; 48_000 * 4]),
    );
    held.release();
    assert!(work.join().unwrap().unwrap_err().contains("changed"));
}

#[test]
fn slicer_a_newer_analysis_wins_when_older_work_finishes_later() {
    let rig = Rig::new();
    let id = source_clip(&rig);
    let held = rig.session.hold("slice:analyzed");
    let work = rig.session.background(move |s| s.slice_analyze(id, grid()));
    held.wait();
    let latest = rig.session.slice_analyze(id, grid()).unwrap();
    held.release();
    assert!(work.join().unwrap().is_err());
    rig.session.slice_apply(latest.token, vec![840]).unwrap();
}

#[test]
fn slicer_rechecks_source_identity_after_playback_plan_compilation() {
    let rig = Rig::new();
    let id = source_clip(&rig);
    let sample = rig
        .project()
        .playlist
        .clips
        .iter()
        .find(|c| c.id == id)
        .unwrap()
        .content
        .sample()
        .unwrap();
    let review = rig.session.slice_analyze(id, grid()).unwrap();
    let held = rig.session.hold("slice:prepared");
    let work = rig
        .session
        .background(move |s| s.slice_apply(review.token, vec![840]));
    held.wait();
    let before = rig.session.document_snapshot();
    rig.session.state().pool.insert(
        sample,
        AudioBuffer::from_interleaved(48_000, 1, vec![0.0; 48_000 * 4]),
    );
    held.release();
    assert!(work.join().unwrap().is_err());
    assert_eq!(rig.session.document_snapshot(), before);
}
