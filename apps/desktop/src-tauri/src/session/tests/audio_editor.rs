use super::Rig;
use crate::session::{
    ClipPlace,
    recording::{CaptureHandle, Sink},
};
use windfall_core::AudioBuffer;
use windfall_ipc::{AudioEditOperation as Op, AudioEditRequest, RecordingSource};
use windfall_project::{ClipContent, ClipId, Command, SampleId};

fn add(rig: &Rig) -> (ClipId, String, Vec<u8>) {
    let path = rig.file("original.wav");
    // Real Float32 fixture, with one frame per tick at the default 120 BPM.
    let source =
        AudioBuffer::from_interleaved(1920, 2, vec![0.1, -0.1, 0.2, -0.2, 0.3, -0.3, 0.4, -0.4]);
    windfall_codec::write_wav(&path, &source, windfall_codec::WavSampleFormat::Float32).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let result = rig
        .session
        .add_audio_clip_from_file(
            &path,
            ClipPlace {
                track: None,
                start: 100,
                mixer_track: None,
            },
        )
        .unwrap();
    (ClipId(*result.created.last().unwrap()), path, bytes)
}
fn request(token: u32, operation: Op) -> AudioEditRequest {
    AudioEditRequest {
        token,
        operation,
        start_frame: 1,
        end_frame: 3,
    }
}
fn derived_files(rig: &Rig) -> usize {
    let folder = rig.folder.path().join("recordings").join("Audio edits");
    std::fs::read_dir(folder).map_or(0, |entries| entries.count())
}
#[test]
fn audio_editor_exact_edit_undo_redo_save_and_fresh_reopen_preserve_source() {
    let rig = Rig::new();
    let (id, path, original) = add(&rig);
    let before = rig.session.document_snapshot();
    let preview = rig.session.audio_editor_open(id).unwrap();
    assert_eq!(preview.frames, 4);
    assert!(preview.peaks.iter().any(|p| *p < 0.0));
    let result = rig
        .session
        .audio_editor_apply(request(preview.token, Op::Reverse))
        .unwrap();
    assert_eq!(result.created.len(), 2);
    let sample = SampleId(result.created[0]);
    let edited = rig.project();
    assert_eq!(
        rig.session.document_snapshot().history.cursor,
        before.history.cursor + 1
    );
    assert_eq!(
        rig.session.state().pool.get(sample).unwrap().samples(),
        &[0.1, -0.1, 0.3, -0.3, 0.2, -0.2, 0.4, -0.4]
    );
    let edited_clip = edited
        .playlist
        .clips
        .iter()
        .find(|c| c.id.0 == result.created[1])
        .unwrap();
    assert_eq!(edited_clip.start, 100);
    assert_eq!(edited_clip.offset, 0);
    assert_eq!(std::fs::read(&path).unwrap(), original);
    rig.session.undo().unwrap();
    let mut restored = before.project.clone();
    restored.next_id = rig.project().next_id; // document ids are intentionally never reused
    assert_eq!(rig.project(), restored);
    assert_eq!(derived_files(&rig), 1); // undo history retains owned asset
    rig.session.redo().unwrap();
    assert_eq!(rig.project(), edited);
    let saved = rig.file("song.windfall");
    rig.session.project_save(Some(&saved)).unwrap();
    let rig = rig.restart();
    rig.session.project_open(&saved).unwrap();
    assert_eq!(rig.project(), edited);
    assert_eq!(
        rig.session.state().pool.get(sample).unwrap().samples(),
        &[0.1, -0.1, 0.3, -0.3, 0.2, -0.2, 0.4, -0.4]
    );
    assert_eq!(std::fs::read(path).unwrap(), original);
}
#[test]
fn audio_editor_trim_and_extract_placement_and_exact_samples() {
    for operation in [Op::Trim, Op::Extract] {
        let rig = Rig::new();
        let (id, _, _) = add(&rig);
        let before = rig.project();
        let preview = rig.session.audio_editor_open(id).unwrap();
        let result = rig
            .session
            .audio_editor_apply(request(preview.token, operation))
            .unwrap();
        let project = rig.project();
        let clip = project
            .playlist
            .clips
            .iter()
            .find(|c| c.id.0 == result.created[1])
            .unwrap();
        assert_eq!(clip.start, if operation == Op::Trim { 101 } else { 104 });
        assert_eq!(clip.length, 2);
        assert_eq!(
            project.playlist.clips.len(),
            before.playlist.clips.len() + usize::from(operation == Op::Extract)
        );
        assert_eq!(
            rig.session
                .state()
                .pool
                .get(SampleId(result.created[0]))
                .unwrap()
                .samples(),
            &[0.2, -0.2, 0.3, -0.3]
        );
        rig.session.undo().unwrap();
        let mut restored = before;
        restored.next_id = rig.project().next_id;
        assert_eq!(rig.project(), restored);
    }
}
#[test]
fn audio_editor_invalid_token_range_missing_source_and_write_failure_leave_document_untouched() {
    let rig = Rig::new();
    let (id, _, _) = add(&rig);
    let before = rig.session.document_snapshot();
    assert!(rig.session.audio_editor_open(ClipId(u32::MAX)).is_err());
    let preview = rig.session.audio_editor_open(id).unwrap();
    assert!(
        rig.session
            .audio_editor_apply(request(preview.token + 1, Op::Trim))
            .is_err()
    );
    let mut invalid = request(preview.token, Op::Trim);
    invalid.end_frame = 5;
    assert!(rig.session.audio_editor_apply(invalid).is_err());
    std::fs::create_dir_all(rig.folder.path().join("recordings")).unwrap();
    std::fs::write(rig.folder.path().join("recordings/Audio edits"), b"blocked").unwrap();
    assert!(
        rig.session
            .audio_editor_apply(request(preview.token, Op::Trim))
            .unwrap_err()
            .contains("folder")
    );
    assert_eq!(rig.session.document_snapshot(), before);
    let ClipContent::Audio { sample, .. } = before
        .project
        .playlist
        .clips
        .iter()
        .find(|c| c.id == id)
        .unwrap()
        .content
    else {
        unreachable!()
    };
    rig.session.state().pool.remove(sample);
    assert!(
        rig.session
            .audio_editor_open(id)
            .unwrap_err()
            .contains("not loaded")
    );
}
#[test]
fn audio_editor_document_source_and_generation_races_refuse_install_and_remove_orphan_output() {
    for change in 0..3 {
        let rig = Rig::new();
        let (id, _, _) = add(&rig);
        let preview = rig.session.audio_editor_open(id).unwrap();
        let hold = rig.session.hold("audio-editor:prepared");
        let job = rig
            .session
            .background(move |s| s.audio_editor_apply(request(preview.token, Op::Trim)));
        hold.wait();
        match change {
            0 => {
                rig.session
                    .dispatch(
                        Command::AddPlaylistTrack {
                            name: None,
                            index: None,
                        },
                        None,
                    )
                    .unwrap();
            }
            1 => {
                let mut state = rig.session.state();
                let sample = state
                    .document
                    .project()
                    .playlist
                    .clips
                    .iter()
                    .find(|c| c.id == id)
                    .unwrap()
                    .content
                    .sample()
                    .unwrap();
                state
                    .pool
                    .insert(sample, AudioBuffer::from_interleaved(1920, 2, vec![0.0; 8]));
            }
            _ => {
                rig.session.project_new().unwrap();
            }
        }
        let before = rig.session.document_snapshot();
        hold.release();
        assert!(job.join().unwrap().unwrap_err().contains("changed"));
        assert_eq!(rig.session.document_snapshot(), before);
        assert_eq!(derived_files(&rig), 0);
    }
}
struct FakeCapture;
impl CaptureHandle for FakeCapture {
    fn frames(&self) -> u64 {
        4
    }
    fn failed(&self) -> bool {
        false
    }
    fn finish(self: Box<Self>) -> Result<u64, String> {
        Ok(4)
    }
}
fn synthetic(_: RecordingSource, _: u32, mut sink: Sink) -> Result<Box<dyn CaptureHandle>, String> {
    sink(&[0.1; 8])?;
    Ok(Box::new(FakeCapture))
}
fn record(rig: &Rig) {
    rig.session
        .recording_start_with(
            RecordingSource {
                host: "fake".into(),
                device: "fake".into(),
                left: 0,
                right: None,
            },
            0,
            None,
            synthetic,
        )
        .unwrap();
}
#[test]
fn audio_editor_recording_before_and_during_work_refuses_without_changing_take_or_document() {
    let rig = Rig::new();
    let (id, _, _) = add(&rig);
    let preview = rig.session.audio_editor_open(id).unwrap();
    record(&rig);
    let before = rig.session.document_snapshot();
    assert!(
        rig.session
            .audio_editor_apply(request(preview.token, Op::Trim))
            .unwrap_err()
            .contains("recording")
    );
    assert!(
        rig.session
            .audio_editor_open(id)
            .unwrap_err()
            .contains("recording")
    );
    assert_eq!(rig.session.document_snapshot(), before);
    rig.session.recording_cancel();
    let preview = rig.session.audio_editor_open(id).unwrap();
    let hold = rig.session.hold("audio-editor:prepared");
    let job = rig
        .session
        .background(move |s| s.audio_editor_apply(request(preview.token, Op::Trim)));
    hold.wait();
    record(&rig);
    hold.release();
    assert!(job.join().unwrap().unwrap_err().contains("recording"));
    assert_eq!(rig.session.document_snapshot(), before);
    assert!(rig.session.recording_state().active);
    assert_eq!(rig.session.recording_state().frames, 4);
    assert_eq!(derived_files(&rig), 0);
    rig.session.recording_cancel();
}
#[test]
fn audio_editor_preview_race_and_discarded_token_are_refused() {
    let rig = Rig::new();
    let (id, _, _) = add(&rig);
    let hold = rig.session.hold("audio-editor:rendered");
    let job = rig.session.background(move |s| s.audio_editor_open(id));
    hold.wait();
    rig.session
        .dispatch(
            Command::AddPlaylistTrack {
                name: None,
                index: None,
            },
            None,
        )
        .unwrap();
    hold.release();
    assert!(job.join().unwrap().unwrap_err().contains("changed"));
    let preview = rig.session.audio_editor_open(id).unwrap();
    rig.session.audio_editor_discard(preview.token);
    assert!(
        rig.session
            .audio_editor_apply(request(preview.token, Op::Trim))
            .is_err()
    );
    assert_eq!(derived_files(&rig), 0);
}

#[test]
fn audio_editor_bakes_clip_view_once_and_preserves_routing_mute_and_fractional_tick_length() {
    use windfall_project::{AudioClipPatch, AudioClipUpdate, ClipPatch, ClipUpdate};
    let rig = Rig::new();
    let (id, path, original) = add(&rig);
    rig.session
        .prepare_clip_command(Command::Batch {
            label: Some("Clip view".into()),
            commands: vec![
                Command::UpdateClips {
                    updates: vec![ClipUpdate {
                        id,
                        patch: ClipPatch {
                            offset: Some(1),
                            length: Some(2),
                            muted: Some(true),
                            ..Default::default()
                        },
                    }],
                },
                Command::UpdateAudioClips {
                    updates: vec![AudioClipUpdate {
                        id,
                        patch: AudioClipPatch {
                            reverse: Some(true),
                            gain: Some(0.5),
                            pan: Some(1.0),
                            fade_in: Some(1),
                            ..Default::default()
                        },
                    }],
                },
            ],
        })
        .unwrap();
    let preview = rig.session.audio_editor_open(id).unwrap();
    let before = rig.project();
    let result = rig
        .session
        .audio_editor_apply(AudioEditRequest {
            token: preview.token,
            operation: Op::Reverse,
            start_frame: 0,
            end_frame: 2,
        })
        .unwrap();
    let sample = SampleId(result.created[0]);
    assert_eq!(
        rig.session.state().pool.get(sample).unwrap().samples(),
        &[0.0, -0.1, 0.0, -0.0]
    );
    let project = rig.project();
    let clip = project
        .playlist
        .clips
        .iter()
        .find(|c| c.id.0 == result.created[1])
        .unwrap();
    assert!(clip.muted);
    assert_eq!(clip.offset, 0);
    assert_eq!(clip.length, 2);
    let ClipContent::Audio {
        mixer_track,
        gain,
        pan,
        fade_in,
        fade_out,
        reverse,
        pitch,
        stretch,
        ..
    } = clip.content
    else {
        unreachable!()
    };
    assert_eq!(
        mixer_track,
        before
            .playlist
            .clips
            .iter()
            .find(|c| c.id == id)
            .map(
                |c| if let ClipContent::Audio { mixer_track, .. } = c.content {
                    mixer_track
                } else {
                    unreachable!()
                }
            )
            .unwrap()
    );
    assert_eq!(
        (gain, pan, fade_in, fade_out, reverse, pitch, stretch),
        (1.0, 0.0, 0, 0, false, 0.0, Default::default())
    );
    assert_eq!(std::fs::read(path).unwrap(), original);
    // Typical 44.1kHz source: a one-tick window is 22.96875 frames.
    let path = rig.file("fractional.wav");
    windfall_codec::write_wav(
        &path,
        &AudioBuffer::from_interleaved(44100, 1, vec![0.2; 23]),
        windfall_codec::WavSampleFormat::Float32,
    )
    .unwrap();
    let result = rig
        .session
        .add_audio_clip_from_file(
            &path,
            ClipPlace {
                track: None,
                start: 0,
                mixer_track: None,
            },
        )
        .unwrap();
    let id = ClipId(*result.created.last().unwrap());
    rig.session
        .prepare_clip_command(Command::UpdateClips {
            updates: vec![ClipUpdate {
                id,
                patch: ClipPatch {
                    length: Some(1),
                    ..Default::default()
                },
            }],
        })
        .unwrap();
    let preview = rig.session.audio_editor_open(id).unwrap();
    assert_eq!(preview.frames, 23);
    let result = rig
        .session
        .audio_editor_apply(AudioEditRequest {
            token: preview.token,
            operation: Op::Silence,
            start_frame: 0,
            end_frame: 23,
        })
        .unwrap();
    assert_eq!(
        rig.project()
            .playlist
            .clips
            .iter()
            .find(|c| c.id.0 == result.created[1])
            .unwrap()
            .length,
        1
    );
}

#[test]
fn audio_editor_tempo_automation_is_explicitly_refused() {
    use windfall_project::AutomationTarget;
    let rig = Rig::new();
    let (id, _, _) = add(&rig);
    rig.session.automate(AutomationTarget::Tempo).unwrap();
    let before = rig.session.document_snapshot();
    assert!(
        rig.session
            .audio_editor_open(id)
            .unwrap_err()
            .contains("tempo automation")
    );
    assert_eq!(rig.session.document_snapshot(), before);
}
