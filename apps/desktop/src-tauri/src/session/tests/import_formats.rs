//! Compressed files through the same browser/sample/playlist flow as WAV.

use std::fs;

use windfall_codec::{Encoder, EncoderSettings, FlacBitDepth, Mp3Channels, Mp3Settings};
use windfall_ipc::{PlayMode, TransportPatch};
use windfall_project::{ChannelId, ClipId, SampleId};

use super::{Rig, SAMPLE_RATE, rms};
use crate::session::ClipPlace;

const FRAMES: usize = SAMPLE_RATE as usize / 2;

fn formats() -> [EncoderSettings; 3] {
    [
        EncoderSettings::Flac {
            depth: FlacBitDepth::Int24,
            level: 5,
        },
        EncoderSettings::Vorbis { quality: 6.0 },
        EncoderSettings::Mp3 {
            settings: Mp3Settings {
                channels: Mp3Channels::Mono,
                ..Mp3Settings::default()
            },
        },
    ]
}

fn tone_file(rig: &Rig, settings: &EncoderSettings) -> String {
    let file = rig.file(&format!("Tone.{}", settings.format().extension()));
    let samples: Vec<_> = (0..FRAMES)
        .map(|frame| {
            (std::f32::consts::TAU * 440.0 * frame as f32 / SAMPLE_RATE as f32).sin() * 0.25
        })
        .collect();
    let mut encoder = Encoder::open(&file, settings, SAMPLE_RATE, 1).unwrap();
    encoder.write(&samples).unwrap();
    encoder.finalize().unwrap();
    file
}

fn place() -> ClipPlace {
    ClipPlace {
        track: None,
        start: 960,
        mixer_track: None,
    }
}

#[test]
fn compressed_samples_preview_play_and_survive_save_reopen() {
    for settings in formats() {
        let mut rig = Rig::new();
        let file = tone_file(&rig, &settings);
        let info = rig.session.sample_info(&file).unwrap();
        assert_eq!(info.name, "Tone");
        assert_eq!((info.sample_rate, info.channels), (SAMPLE_RATE, 1));
        assert_eq!(info.frames, FRAMES as u64);
        assert!((info.duration_secs - 0.5).abs() < 1e-9);
        assert!(info.peaks.iter().any(|peak| *peak > 0.1));

        let before = rig.session.document_snapshot().history.entries.len();
        let added = rig.session.add_channel_from_file(&file, None).unwrap();
        let [sample, channel, _track] = added.created[..] else {
            panic!("expected sample, channel and mixer track");
        };
        assert!(rig.has_audio(SampleId(sample)));
        assert_eq!(
            rig.session.document_snapshot().history.entries.len(),
            before + 1
        );
        rig.session.audition_note_on(ChannelId(channel), 60, 1.0);
        assert!(rms(&rig.run(4_800)) > 0.05, "{settings:?}");
        rig.session.audition_note_off(ChannelId(channel), 60);

        let saved = rig
            .session
            .project_save(Some(&rig.file("Compressed.windfall")))
            .unwrap();
        let project = rig.project();
        rig.session.project_new().unwrap();
        rig.session.project_open(&saved).unwrap();
        assert_eq!(rig.project(), project);
        assert!(rig.has_audio(SampleId(sample)));
        rig.session.audition_note_on(ChannelId(channel), 60, 1.0);
        assert!(rms(&rig.run(4_800)) > 0.05, "reopened {settings:?}");
    }
}

#[test]
fn compressed_playlist_clips_have_gapless_duration_and_one_undo_step() {
    for settings in formats() {
        let mut rig = Rig::new();
        let file = tone_file(&rig, &settings);
        let before = rig.project();
        let added = rig
            .session
            .add_audio_clip_from_file(&file, place())
            .unwrap();
        let clip_id = ClipId(*added.created.last().unwrap());
        let imported = rig.project();
        let clip = imported
            .playlist
            .clips
            .iter()
            .find(|clip| clip.id == clip_id)
            .unwrap();
        assert_eq!((clip.start, clip.length), (960, 960), "{settings:?}");
        assert_eq!(rig.session.document_snapshot().history.entries.len(), 1);

        rig.session
            .transport_set(TransportPatch {
                mode: Some(PlayMode::Song),
                loop_song: Some(false),
                ..TransportPatch::default()
            })
            .unwrap();
        rig.session.transport_play().unwrap();
        let out = rig.run(SAMPLE_RATE as usize);
        assert!(out[..FRAMES * 2].iter().all(|sample| *sample == 0.0));
        assert!(rms(&out[FRAMES * 2..]) > 0.05, "{settings:?}");
        rig.session.transport_stop();

        rig.session.undo().unwrap();
        let mut undone = rig.project();
        // Allocated IDs stay reserved after undo.
        undone.next_id = before.next_id;
        assert_eq!(undone, before);
        rig.session.redo().unwrap();
        assert_eq!(rig.project(), imported);
    }
}

#[test]
fn broken_compressed_files_do_not_change_the_project_or_publish_edits() {
    for settings in formats() {
        let rig = Rig::new();
        let file = tone_file(&rig, &settings);
        let bytes = fs::read(&file).unwrap();
        fs::write(&file, &bytes[..8]).unwrap();
        let before = rig.session.document_snapshot();
        rig.events.take();
        assert!(rig.session.add_channel_from_file(&file, None).is_err());
        assert!(
            rig.session
                .add_audio_clip_from_file(&file, place())
                .is_err()
        );
        assert_eq!(rig.session.document_snapshot(), before);
        assert!(rig.events.take().is_empty());
    }
}
