use windfall_core::AudioBuffer;
use windfall_engine::{RenderOptions, SamplePool, render_streaming_checked};
use windfall_ipc::PlayMode;
use windfall_project::{
    Clip, ClipContent, ClipId, ClipInit, Command, Lane, Note, NoteId, PlaylistTrack,
    PlaylistTrackId, PlaylistTrackPatch, TrackId,
};

use super::{Rig, SAMPLE_RATE};
use crate::session::playlist_bounce::bounce_source;

fn audio(sample: windfall_project::SampleId) -> ClipContent {
    ClipContent::Audio {
        sample,
        mixer_track: TrackId::MASTER,
        output: Default::default(),
        normalize: false,
        gain: 1.0,
        pan: 0.0,
        fade_in: 0,
        fade_out: 0,
        reverse: false,
        pitch: 0.0,
        stretch: Default::default(),
    }
}

#[test]
fn bounce_span_renders_pattern_and_audio_and_excludes_other_playlist_tracks() {
    let rig = Rig::new();
    for pattern in [true, false] {
        let mut project = rig.project();
        let sample = project.samples[0].id;
        project.next_id = 1000;
        project.settings.tempo_bpm = 120.0;
        project.patterns[0].lanes = vec![Lane {
            channel: project.channels[0].id,
            notes: vec![Note {
                id: NoteId(901),
                start: 0,
                length: 960,
                key: windfall_project::DEFAULT_KEY,
                velocity: 1.0,
                pan: 0.0,
                expression: Default::default(),
            }],
        }];
        project.playlist.tracks = vec![
            PlaylistTrack {
                id: PlaylistTrackId(902),
                name: "Chosen".into(),
                muted: true,
                solo: false,
                color: 0,
                height: 0,
            },
            PlaylistTrack {
                id: PlaylistTrackId(903),
                name: "Other".into(),
                muted: false,
                solo: true,
                color: 0,
                height: 0,
            },
        ];
        project.playlist.clips = vec![
            Clip {
                id: ClipId(904),
                track: PlaylistTrackId(902),
                start: 960,
                length: 960,
                offset: 0,
                muted: false,
                content: if pattern {
                    ClipContent::Pattern {
                        pattern: project.patterns[0].id,
                    }
                } else {
                    audio(sample)
                },
            },
            Clip {
                id: ClipId(905),
                track: PlaylistTrackId(903),
                start: 960,
                length: 1920,
                offset: 0,
                muted: false,
                content: audio(sample),
            },
        ];
        project.check().unwrap();
        let live = project.clone();
        let (source, range, destination) = bounce_source(&project, &[ClipId(904)]).unwrap();
        assert_eq!(
            (range.start, range.end, destination),
            (960, 1920, PlaylistTrackId(902))
        );
        assert!(source.playlist.tracks[0].solo);
        assert!(!source.playlist.tracks[0].muted);
        assert!(!source.playlist.tracks[1].solo);
        let mut pool = SamplePool::new();
        pool.insert(
            sample,
            AudioBuffer::from_interleaved(SAMPLE_RATE, 1, vec![0.25; 96_000]),
        );
        let options = RenderOptions {
            region: Some(range),
            mode: PlayMode::Song,
            sample_rate: SAMPLE_RATE,
            ..Default::default()
        };
        let collect = |project: &windfall_project::Project| {
            let mut buffer = Vec::new();
            let result = render_streaming_checked(
                project,
                &pool,
                &options,
                &mut |block| {
                    buffer.extend_from_slice(block);
                    true
                },
                &mut |_| true,
            )
            .unwrap();
            assert!(result.completed);
            assert_eq!(result.frames, 24_000);
            buffer
        };
        let actual = collect(&source);
        assert!(actual.iter().any(|value| value.abs() > 0.01));
        let mut reference = source.clone();
        reference.playlist.clips[1].muted = true;
        assert_eq!(actual, collect(&reference));
        // Even with an audible clip on the other track, a silent chosen track stays silent.
        reference.playlist.clips[0].muted = true;
        reference.playlist.clips[1].muted = false;
        assert!(collect(&reference).iter().all(|value| *value == 0.0));
        assert_eq!(project, live);
    }
}

fn seed(rig: &Rig) -> Vec<ClipId> {
    let session = &rig.session;
    let mut tracks = Vec::new();
    for name in ["Top", "Bottom"] {
        let result = session
            .dispatch(
                Command::AddPlaylistTrack {
                    name: Some(name.into()),
                    index: None,
                },
                None,
            )
            .unwrap();
        tracks.push(PlaylistTrackId(result.created[0]));
    }
    let sample = rig.project().samples[0].id;
    let result = session
        .dispatch(
            Command::AddClips {
                clips: vec![
                    ClipInit {
                        track: tracks[1],
                        start: 960,
                        length: Some(960),
                        offset: None,
                        muted: None,
                        content: ClipContent::Pattern {
                            pattern: rig.pattern(),
                        },
                    },
                    ClipInit {
                        track: tracks[0],
                        start: 1440,
                        length: Some(960),
                        offset: None,
                        muted: None,
                        content: audio(sample),
                    },
                ],
            },
            None,
        )
        .unwrap();
    session
        .dispatch(
            Command::UpdatePlaylistTrack {
                id: tracks[0],
                patch: PlaylistTrackPatch {
                    muted: Some(true),
                    solo: Some(false),
                    ..Default::default()
                },
            },
            None,
        )
        .unwrap();
    session
        .dispatch(
            Command::UpdatePlaylistTrack {
                id: tracks[1],
                patch: PlaylistTrackPatch {
                    solo: Some(true),
                    ..Default::default()
                },
            },
            None,
        )
        .unwrap();
    result.created.into_iter().map(ClipId).collect()
}

#[test]
fn bounce_commits_one_batch_and_undo_restores_sources_and_track_flags() {
    let rig = Rig::new();
    let selection = seed(&rig);
    let before = rig.session.document_snapshot();
    let result = rig
        .session
        .bounce_selected_clips(selection.clone())
        .unwrap();
    let after = rig.session.document_snapshot();
    assert_eq!(
        after.history.entries.len(),
        before.history.entries.len() + 1
    );
    assert_eq!(
        after.history.entries.last().unwrap().label,
        "Bounce selected clips"
    );
    assert_eq!(
        after.project.playlist.tracks,
        before.project.playlist.tracks
    );
    assert!(
        after
            .project
            .playlist
            .clips
            .iter()
            .filter(|clip| selection.contains(&clip.id))
            .all(|clip| clip.muted)
    );
    let clip = after
        .project
        .playlist
        .clips
        .iter()
        .find(|clip| clip.id.0 == *result.created.last().unwrap())
        .unwrap();
    assert_eq!(clip.id.0, *result.created.last().unwrap());
    assert_eq!(
        (clip.track, clip.start, clip.length, clip.muted),
        (before.project.playlist.tracks[0].id, 960, 1440, false)
    );
    let buffer = rig
        .session
        .state()
        .pool
        .get(clip.content.sample().unwrap())
        .unwrap()
        .clone();
    assert!(buffer.samples().iter().any(|value| value.abs() > 0.0001));
    rig.session.undo().unwrap();
    let mut restored = rig.project();
    restored.next_id = before.project.next_id;
    assert_eq!(restored, before.project);
    rig.session.redo().unwrap();
    assert_eq!(rig.project(), after.project);
}

#[test]
fn bounce_empty_or_stale_selection_leaves_document_unchanged() {
    let rig = Rig::new();
    let selection = seed(&rig);
    let before = rig.session.document_snapshot();
    for ids in [vec![], vec![ClipId(999_999)]] {
        assert!(rig.session.bounce_selected_clips(ids).is_err());
        assert_eq!(rig.session.document_snapshot(), before);
    }
    let hold = rig.session.hold("bounce:rendered");
    let worker = rig
        .session
        .background(move |session| session.bounce_selected_clips(selection));
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
    let edited = rig.session.document_snapshot();
    hold.release();
    assert!(
        worker
            .join()
            .unwrap()
            .unwrap_err()
            .contains("changed while bouncing")
    );
    assert_eq!(rig.session.document_snapshot(), edited);
}

#[test]
fn bounce_source_excludes_unselected_clips_on_selected_tracks() {
    let rig = Rig::new();
    let selection = seed(&rig);
    let mut project = rig.project();
    let mut other = project.playlist.clips[0].clone();
    other.id = ClipId(project.next_id);
    project.next_id += 1;
    other.start = 1200;
    project.playlist.clips.push(other.clone());
    project.playlist.clips.sort_by_key(Clip::sort_key);
    project.check().unwrap();
    let (source, _, _) = bounce_source(&project, &selection).unwrap();
    assert!(
        source
            .playlist
            .clips
            .iter()
            .find(|clip| clip.id == other.id)
            .unwrap()
            .muted
    );
    assert_eq!(source.playlist.clips.len(), project.playlist.clips.len());
}

#[test]
fn bounce_active_arrangement_keeps_print_audible_and_undoable() {
    let rig = Rig::new();
    let selection = seed(&rig);
    let tracks = rig
        .project()
        .playlist
        .tracks
        .iter()
        .map(|track| track.id)
        .collect();
    rig.session
        .dispatch(
            Command::AddArrangement {
                name: "Chosen layout".into(),
                clips: selection.clone(),
                tracks,
            },
            None,
        )
        .unwrap();
    let before = rig.session.document_snapshot();
    let result = rig
        .session
        .bounce_selected_clips(selection.clone())
        .unwrap();
    let after = rig.project();
    let printed = after
        .playlist
        .clips
        .iter()
        .find(|clip| clip.id.0 == *result.created.last().unwrap())
        .unwrap();
    let active = after
        .playlist
        .arrangement_book
        .arrangements
        .iter()
        .find(|arrangement| Some(arrangement.id) == after.playlist.arrangement_book.active)
        .unwrap();
    assert!(
        active.clips.contains(&printed.id),
        "the active arrangement must include the print"
    );
    assert!(
        matches!(
            printed.content,
            ClipContent::Audio {
                output: windfall_project::ClipAudioOutput::Direct,
                ..
            }
        ),
        "a full-mix print must bypass Master processing on playback"
    );
    rig.session.undo().unwrap();
    let mut restored = rig.project();
    restored.next_id = before.project.next_id;
    assert_eq!(restored, before.project);
    rig.session.redo().unwrap();
    assert_eq!(rig.project(), after);
}

#[test]
fn bounce_print_preserves_master_gain_on_playback() {
    let rig = Rig::new();
    let track = PlaylistTrackId(
        rig.session
            .dispatch(
                Command::AddPlaylistTrack {
                    name: None,
                    index: None,
                },
                None,
            )
            .unwrap()
            .created[0],
    );
    let sample = rig.project().samples[0].id;
    let clip = ClipId(
        rig.session
            .dispatch(
                Command::AddClips {
                    clips: vec![ClipInit {
                        track,
                        start: 0,
                        length: Some(960),
                        offset: None,
                        muted: None,
                        content: audio(sample),
                    }],
                },
                None,
            )
            .unwrap()
            .created[0],
    );
    rig.session
        .dispatch(
            Command::UpdateMixerTrack {
                id: TrackId::MASTER,
                patch: windfall_project::MixerTrackPatch {
                    volume: Some(0.25),
                    ..Default::default()
                },
            },
            None,
        )
        .unwrap();
    let options = RenderOptions {
        region: Some(windfall_project::TickRange { start: 0, end: 960 }),
        mode: PlayMode::Song,
        sample_rate: SAMPLE_RATE,
        tail_secs: 0.0,
        auto_tail: false,
        ..Default::default()
    };
    let render = || {
        let project = rig.project();
        let pool = rig.session.state().pool.clone();
        let mut samples = Vec::new();
        let result = render_streaming_checked(
            &project,
            &pool,
            &options,
            &mut |block| {
                samples.extend_from_slice(block);
                true
            },
            &mut |_| true,
        )
        .unwrap();
        assert!(result.completed);
        samples
    };
    let before = render();
    assert!(before.iter().any(|sample| sample.abs() > 0.0001));
    rig.session.bounce_selected_clips(vec![clip]).unwrap();
    let after = render();
    assert_eq!(before.len(), after.len());
    let error = before
        .iter()
        .zip(&after)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f32, f32::max);
    assert!(
        error < 0.00001,
        "the print must not apply Master gain twice: max error {error}"
    );
}
