//! Public playback and offline rendering keep absolute onsets under local meters.
#[allow(dead_code)]
#[path = "engine/support.rs"]
mod support;

use support::{Rig, impulse, sounding_frames};
use windfall_engine::{RenderOptions, render};
use windfall_ipc::PlayMode;
use windfall_project::{
    MarkerKind, MeterChange, MeterChangeId, TimeSignature, TimelineMarker, TimelineMarkerId,
};

#[test]
fn pattern_and_song_local_meters_do_not_retime_live_or_offline_audio() {
    let mut rig = Rig::new();
    rig.project.settings.tempo_bpm = 120.0;
    let channel = rig.channel(impulse(48_000));
    rig.project.patterns[0].length_steps = 32;
    rig.note(channel, 120, 120);
    rig.note(channel, 4100, 120);
    let track = rig.playlist_track();
    rig.clip(track, rig.first_pattern(), 960, 7680);
    let before = rig.project.clone();
    let expected_pattern = [120 * 25, 4100 * 25];
    let expected_song = [(960 + 120) * 25, (960 + 4100) * 25];
    let baseline_pattern = rig.play(48_000, 150_000, 64);
    let baseline_song = rig.play_song(48_000, 150_000, 64);
    assert_eq!(sounding_frames(&baseline_pattern), expected_pattern);
    assert_eq!(sounding_frames(&baseline_song), expected_song);
    let local_meter_id = rig.project.next_id;
    rig.project.next_id += 3;
    rig.project.patterns[0].time_signature = Some(TimeSignature {
        numerator: 3,
        denominator: 4,
    });
    rig.project.patterns[0].timeline.meters.push(MeterChange {
        id: MeterChangeId(local_meter_id),
        tick: 4001,
        signature: TimeSignature {
            numerator: 7,
            denominator: 8,
        },
    });
    rig.project.patterns[0]
        .timeline
        .markers
        .push(TimelineMarker {
            id: TimelineMarkerId(local_meter_id + 1),
            tick: 4100,
            name: "Pattern phrase".into(),
            kind: MarkerKind::Named,
        });
    rig.project.playlist.timeline.meters.push(MeterChange {
        id: MeterChangeId(local_meter_id + 2),
        tick: 4001,
        signature: TimeSignature {
            numerator: 5,
            denominator: 4,
        },
    });
    rig.project.check().unwrap();
    for block in [7, 1024] {
        assert_eq!(rig.play(48_000, 150_000, block), baseline_pattern);
        assert_eq!(rig.play_song(48_000, 150_000, block), baseline_song);
    }
    for (mode, expected, ticks) in [
        (PlayMode::Pattern, expected_pattern, 7680),
        (PlayMode::Song, expected_song, 8640),
    ] {
        let options = RenderOptions {
            mode,
            sample_rate: 48_000,
            block_frames: 37,
            ..Default::default()
        };
        let audio = render(&rig.project, &rig.pool, &options, &mut |_| true);
        assert_eq!(audio.frames(), ticks * 25);
        assert_eq!(sounding_frames(audio.samples()), expected);
        assert_eq!(
            audio.samples(),
            render(&before, &rig.pool, &options, &mut |_| true).samples()
        );
    }
    assert_eq!(rig.project.patterns[0].lanes, before.patterns[0].lanes);
    assert_eq!(rig.project.playlist.clips, before.playlist.clips);
}
