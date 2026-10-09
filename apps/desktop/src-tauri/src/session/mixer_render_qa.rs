//! Actual offline files written from captured selected/armed mixer track ids.
use super::Rig;
use crate::session::ClipPlace;
use windfall_codec::{WavSampleFormat, WavWriter, decode_file};
use windfall_ipc::{BitDepth, ExportFormat, ExportOptions, ExportStems, PlayMode, StemMode};
use windfall_project::{Command, MixerRecordMode, MixerRecording, MixerTrackPatch, TrackId};

fn fixture() -> (Rig, [TrackId; 2]) {
    let rig = Rig::new();
    let mut tracks = Vec::new();
    for (name, start, value, volume) in [("QA Early", 0, 0.2, 0.5), ("QA Late", 960, 0.35, 0.75)] {
        let id = TrackId(
            rig.session
                .dispatch(
                    Command::AddMixerTrack {
                        name: Some(name.into()),
                    },
                    None,
                )
                .unwrap()
                .created[0],
        );
        rig.session
            .dispatch(
                Command::UpdateMixerTrack {
                    id,
                    patch: MixerTrackPatch {
                        volume: Some(volume),
                        ..Default::default()
                    },
                },
                None,
            )
            .unwrap();
        let path = rig.file(&format!("{name}.wav"));
        let mut writer = WavWriter::create(&path, 48000, 2, WavSampleFormat::Float32).unwrap();
        writer.write(&[value; 960]).unwrap();
        writer.finalize().unwrap();
        rig.session
            .add_audio_clip_from_file(
                &path,
                ClipPlace {
                    start,
                    track: None,
                    mixer_track: Some(id),
                },
            )
            .unwrap();
        tracks.push(id);
    }
    rig.session
        .dispatch(
            Command::UpdateMixerTrack {
                id: TrackId::MASTER,
                patch: MixerTrackPatch {
                    volume: Some(1.0),
                    ..Default::default()
                },
            },
            None,
        )
        .unwrap();
    (rig, tracks.try_into().unwrap())
}
fn options(
    rig: &Rig,
    name: &str,
    tracks: Vec<TrackId>,
    include_mix: bool,
    mode: StemMode,
) -> ExportOptions {
    ExportOptions {
        path: rig.file(name),
        format: ExportFormat::Wav,
        bit_depth: BitDepth::Float32,
        sample_rate: 48000,
        mode: PlayMode::Song,
        pattern_loops: 1,
        tail_secs: 0.0,
        auto_tail: false,
        stems: Some(ExportStems {
            mode,
            tracks: Some(tracks),
            include_mix,
            numbered: true,
            folder: true,
        }),
        ..Default::default()
    }
}
#[test]
fn qa_selected_track_request_writes_only_its_isolated_post_fader_audio_file() {
    let (rig, [early, _]) = fixture();
    let before = rig.project();
    rig.session
        .export_audio(options(
            &rig,
            "selected.wav",
            vec![early],
            false,
            StemMode::TrackOutputs,
        ))
        .unwrap();
    let done = rig.events.wait_for_export();
    assert_eq!(done.error, None);
    let files = done.files.unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].track, Some(early));
    let audio = decode_file(&files[0].path).unwrap();
    assert_eq!(audio.frames() as u64, files[0].frames);
    assert!(audio.samples()[..960].iter().any(|sample| *sample > 0.05));
    assert!(audio.samples()[48000..].iter().all(|sample| *sample == 0.0));
    assert_eq!(rig.project(), before);
}

#[test]
fn qa_saved_armed_track_requests_write_distinct_stems_whose_sum_conserves_the_mix() {
    for mode in [StemMode::TrackOutputs, StemMode::ToMaster] {
        let (rig, [early, late]) = fixture();
        for id in [early, late] {
            rig.session
                .dispatch(
                    Command::UpdateMixerTrack {
                        id,
                        patch: MixerTrackPatch {
                            recording: Some(MixerRecording {
                                armed: true,
                                mode: MixerRecordMode::PostFader,
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                    },
                    None,
                )
                .unwrap();
        }
        let before = rig.project();
        let armed = before
            .mixer
            .tracks
            .iter()
            .filter(|track| {
                !track.current
                    && track.id != TrackId::MASTER
                    && track
                        .recording
                        .as_ref()
                        .is_some_and(|recording| recording.armed)
            })
            .map(|track| track.id)
            .collect();
        rig.session
            .export_audio(options(&rig, "armed.wav", armed, true, mode))
            .unwrap();
        let done = rig.events.wait_for_export();
        assert_eq!(done.error, None);
        let files = done.files.unwrap();
        assert_eq!(
            files.iter().map(|file| file.track).collect::<Vec<_>>(),
            [None, Some(early), Some(late)]
        );
        let audio: Vec<_> = files
            .iter()
            .map(|file| decode_file(&file.path).unwrap())
            .collect();
        assert!(
            audio
                .iter()
                .all(|stream| stream.frames() == audio[0].frames())
        );
        assert!(
            audio[1].samples()[..960]
                .iter()
                .any(|sample| *sample > 0.05)
        );
        assert!(
            audio[1].samples()[48000..]
                .iter()
                .all(|sample| *sample == 0.0)
        );
        assert!(
            audio[2].samples()[..48000]
                .iter()
                .all(|sample| *sample == 0.0)
        );
        assert!(
            audio[2].samples()[48000..]
                .iter()
                .any(|sample| *sample > 0.15)
        );
        for ((mix, first), second) in audio[0]
            .samples()
            .iter()
            .zip(audio[1].samples())
            .zip(audio[2].samples())
        {
            assert!((*mix - first - second).abs() < 1e-6);
        }
        assert_eq!(
            rig.project(),
            before,
            "offline rendering cannot mutate arms or source clips"
        );
    }
}

#[test]
fn qa_native_stem_admission_rejects_removed_ids_and_current_utility_before_writing() {
    let (rig, [early, _]) = fixture();
    rig.session
        .dispatch(Command::RemoveMixerTrack { id: early }, None)
        .unwrap();
    assert!(
        rig.session
            .export_audio(options(
                &rig,
                "removed.wav",
                vec![early],
                false,
                StemMode::TrackOutputs
            ))
            .is_err()
    );
    let current = TrackId(
        rig.session
            .dispatch(Command::EnsureCurrentMixerTrack, None)
            .unwrap()
            .created[0],
    );
    assert!(
        rig.session
            .export_audio(options(
                &rig,
                "current.wav",
                vec![current],
                false,
                StemMode::TrackOutputs
            ))
            .is_err()
    );
    assert!(!rig.folder.path().join("removed").exists());
    assert!(!rig.folder.path().join("current").exists());
}
