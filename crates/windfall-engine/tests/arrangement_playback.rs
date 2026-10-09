//! Arrangement filtering through the public renderer and its engine plan.
use windfall_core::{AudioBuffer, PPQ};
use windfall_engine::{RenderOptions, SamplePool, render};
use windfall_ipc::PlayMode;
use windfall_project::arrangement::Arrangement;
use windfall_project::{
    ArrangementBook, Clip, ClipContent, ClipId, PlaylistTrack, PlaylistTrackId, Project, SampleId,
    TrackId,
};

fn fixture() -> (Project, SamplePool) {
    let mut project = Project::new("Windfall arrangement playback");
    project.settings.tempo_bpm = 120.0;
    for index in 0..2 {
        project.playlist.tracks.push(PlaylistTrack {
            id: PlaylistTrackId(50 + index),
            name: format!("Track {index}"),
            muted: false,
            solo: false,
            color: 0,
            height: 0,
        });
        project.playlist.clips.push(Clip {
            id: ClipId(60 + index),
            track: PlaylistTrackId(50 + index),
            start: index * PPQ,
            length: PPQ,
            offset: 0,
            muted: false,
            content: ClipContent::Audio {
                sample: SampleId(11),
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
            },
        });
    }
    let mut pool = SamplePool::new();
    pool.insert(
        SampleId(11),
        AudioBuffer::from_interleaved(48_000, 1, vec![0.25; 48_000]),
    );
    (project, pool)
}

fn select(project: &mut Project, clips: &[u32], tracks: &[u32]) {
    project.playlist.arrangement_book = ArrangementBook {
        active: Some(90),
        arrangements: vec![Arrangement {
            id: 90,
            name: "Verse".into(),
            clips: clips.iter().copied().map(ClipId).collect(),
            tracks: tracks.iter().copied().map(PlaylistTrackId).collect(),
        }],
        ..Default::default()
    };
}

fn audio(project: &Project, pool: &SamplePool) -> AudioBuffer {
    render(
        project,
        pool,
        &RenderOptions {
            mode: PlayMode::Song,
            sample_rate: 48_000,
            block_frames: 480,
            tail_secs: 0.0,
            ..Default::default()
        },
        &mut |_| true,
    )
}

#[test]
fn empty_book_plans_every_audible_clip() {
    let (mut project, pool) = fixture();
    let all = audio(&project, &pool);
    assert_eq!(all.frames(), 48_000);
    for frame in [12_000, 36_000] {
        assert!(all.samples()[frame * 2].abs() > 0.1);
    }
    project.playlist.clips[0].muted = true;
    let muted = audio(&project, &pool);
    assert_eq!(muted.frames(), all.frames());
    assert_eq!(muted.samples()[12_000 * 2], 0.0);
    assert!(muted.samples()[36_000 * 2].abs() > 0.1);
}

#[test]
fn active_arrangement_plans_only_its_clip_without_rewriting_positions() {
    let (mut project, pool) = fixture();
    let saved = project.playlist.clips.clone();
    select(&mut project, &[61], &[51]);
    let actual = audio(&project, &pool);
    let mut reference = project.clone();
    reference.playlist.arrangement_book = Default::default();
    reference
        .playlist
        .clips
        .retain(|clip| clip.id == ClipId(61));
    assert_eq!(actual.samples(), audio(&reference, &pool).samples());
    assert_eq!(actual.samples()[12_000 * 2], 0.0);
    assert!(actual.samples()[36_000 * 2].abs() > 0.1);
    assert_eq!(project.playlist.clips, saved);
}

#[test]
fn clip_on_a_track_outside_the_arrangement_is_silent() {
    let (mut project, pool) = fixture();
    select(&mut project, &[61], &[50]);
    assert_eq!(audio(&project, &pool).frames(), 0);
}

#[test]
fn empty_arrangement_lists_play_nothing_and_no_active_id_restores_all_clips() {
    let (mut project, pool) = fixture();
    for (clips, tracks) in [(&[][..], &[50][..]), (&[60][..], &[][..])] {
        select(&mut project, clips, tracks);
        assert_eq!(audio(&project, &pool).frames(), 0);
    }
    project.playlist.arrangement_book.active = None;
    assert_eq!(audio(&project, &pool).frames(), 48_000);
}

#[test]
fn hidden_solo_track_does_not_silence_active_clips_and_mute_still_wins() {
    let (mut project, pool) = fixture();
    project.playlist.tracks[1].solo = true;
    select(&mut project, &[60], &[50]);
    assert!(audio(&project, &pool).samples()[12_000 * 2].abs() > 0.1);
    project.playlist.tracks[0].muted = true;
    let muted = audio(&project, &pool);
    assert_eq!(muted.frames(), 24_000);
    assert!(muted.samples().iter().all(|&sample| sample == 0.0));
}
