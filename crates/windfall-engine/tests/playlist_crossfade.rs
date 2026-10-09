//! Playlist crossfades through the public renderer, without shared fixtures.

use windfall_core::AudioBuffer;
use windfall_engine::{RenderOptions, SamplePool, render};
use windfall_ipc::PlayMode;
use windfall_project::{Clip, ClipContent, ClipId, PlaylistTrackId, Project, SampleId, TrackId};

const RATE: u32 = 48_000;

fn clip(id: u32, track: u32, start: u32, length: u32, fades: (u32, u32)) -> Clip {
    Clip {
        id: ClipId(id),
        track: PlaylistTrackId(track),
        start,
        length,
        offset: 0,
        muted: false,
        content: ClipContent::Audio {
            sample: SampleId(1),
            mixer_track: TrackId::MASTER,
            output: Default::default(),
            normalize: false,
            gain: if track == 51 { 0.5 } else { 1.0 },
            pan: 0.0,
            fade_in: fades.0,
            fade_out: fades.1,
            reverse: false,
            pitch: 0.0,
            stretch: Default::default(),
        },
    }
}

#[test]
fn same_track_overlaps_render_equal_power_crossfades_and_keep_longer_fades() {
    let mut pool = SamplePool::new();
    pool.insert(
        SampleId(1),
        AudioBuffer::from_interleaved(RATE, 1, vec![0.5; RATE as usize * 4]),
    );
    let options = RenderOptions {
        mode: PlayMode::Song,
        sample_rate: RATE,
        block_frames: 480,
        ..Default::default()
    };
    let curve = |part: f64| (part * std::f64::consts::FRAC_PI_2).sin() as f32;
    for user_fade in [0, 1_440] {
        let mut project = Project::new("playlist crossfade");
        project.settings.tempo_bpm = 120.0;
        // The other playlist track shares the mixer destination and starts
        // between the pair. Input order deliberately differs from start order.
        project.playlist.clips = vec![
            clip(30, 50, 960, 1_920, (user_fade, 0)),
            clip(20, 51, 480, 2_880, (0, 0)),
            clip(10, 50, 0, 1_920, (0, user_fade)),
        ];
        let stored_clips = project.playlist.clips.clone();
        let audio = render(&project, &pool, &options, &mut |_| true);
        let fade_frames = f64::from(user_fade.max(960)) * 25.0;
        for frame in [24_144, 30_000, 36_000, 42_000, 47_000] {
            let up = curve((frame - 24_000) as f64 / fade_frames);
            let down = curve((48_000 - frame) as f64 / fade_frames);
            let expected = 0.5 * (up + down) + 0.25;
            for channel in 0..2 {
                let actual = audio.samples()[frame * 2 + channel];
                assert!(
                    (actual - expected).abs() < 1e-6,
                    "frame {frame}, channel {channel}, fade {user_fade}: {actual} != {expected}"
                );
            }
        }
        assert_eq!(project.playlist.clips, stored_clips);
    }
}
