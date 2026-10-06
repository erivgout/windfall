//! Offline rendering: length, determinism and agreement with playback.

use windfall_core::AudioBuffer;
use windfall_engine::{RenderOptions, render};
use windfall_ipc::PlayMode;
use windfall_project::Envelope;

use crate::support::{Rig, impulse, left, level, sine, sounding_frames};

const RATE: u32 = 48_000;

/// A project with resampling, an envelope, swing and a cut in it, so a
/// render has plenty of ways to differ if anything depended on block size.
fn song() -> Rig {
    let mut rig = Rig::new();
    rig.project.settings.tempo_bpm = 128.0;
    rig.project.settings.swing = 0.3;
    let bass = rig.channel(sine(44_100, 110.0, 0.5));
    rig.sampler_mut(bass).envelope = Some(Envelope::default());
    rig.steps(bass, &[0, 3, 6, 10]);
    let hat = rig.channel(sine(RATE, 6_000.0, 0.2));
    rig.sampler_mut(hat).cut_self = true;
    rig.steps(hat, &[0, 1, 2, 3, 4, 6, 8, 9, 12, 14]);
    let lane = rig.playlist_track();
    let pattern = rig.first_pattern();
    rig.clip(lane, pattern, 0, 3_840);
    rig.clip(lane, pattern, 3_840, 2_000).offset = 700;
    rig
}

fn render_all(rig: &Rig, options: &RenderOptions) -> AudioBuffer {
    render(&rig.project, &rig.pool, options, &mut |_| true)
}

fn same_bits(a: &AudioBuffer, b: &AudioBuffer) -> bool {
    a.frames() == b.frames()
        && a.samples()
            .iter()
            .zip(b.samples())
            .all(|(a, b)| a.to_bits() == b.to_bits())
}

#[test]
fn rendering_twice_gives_the_same_bits() {
    let rig = song();
    for mode in [PlayMode::Pattern, PlayMode::Song] {
        let options = RenderOptions {
            mode,
            pattern_loops: 2,
            tail_secs: 0.5,
            ..RenderOptions::default()
        };
        let first = render_all(&rig, &options);
        let second = render_all(&rig, &options);
        assert!(first.samples().iter().any(|sample| *sample != 0.0));
        assert!(same_bits(&first, &second), "{mode:?} mode");
    }
}

#[test]
fn the_block_size_does_not_change_a_render() {
    let rig = song();
    for mode in [PlayMode::Pattern, PlayMode::Song] {
        let options = |block_frames| RenderOptions {
            mode,
            pattern_loops: 2,
            tail_secs: 0.25,
            block_frames,
            ..RenderOptions::default()
        };
        let reference = render_all(&rig, &options(128));
        for block_frames in [1, 7, 64, 1_024, 100_000] {
            let audio = render_all(&rig, &options(block_frames));
            assert!(
                same_bits(&audio, &reference),
                "{mode:?} mode in blocks of {block_frames}"
            );
        }
    }
}

#[test]
fn a_render_is_what_playback_sounds_like() {
    let rig = song();
    let rendered = render_all(
        &rig,
        &RenderOptions {
            pattern_loops: 3,
            ..RenderOptions::default()
        },
    );
    let played = rig.play(RATE, rendered.frames(), 480);
    assert!(rendered.samples() == played);
}

#[test]
fn pattern_mode_renders_the_loops_and_then_only_the_tail() {
    let mut rig = Rig::new();
    let click = rig.channel(impulse(RATE));
    rig.steps(click, &[0, 15]);
    let ring = rig.channel(level(RATE, 0.25, 1.0));
    rig.note(ring, 3_600, 240);
    let options = RenderOptions {
        pattern_loops: 2,
        tail_secs: 1.0,
        ..RenderOptions::default()
    };
    let audio = render_all(&rig, &options);
    // Two bars at 120 bpm, 96000 frames each, and a second of tail.
    assert_eq!(audio.frames(), 2 * 96_000 + 48_000);
    assert_eq!(audio.sample_rate(), RATE);

    let left = left(audio.samples());
    // The clicks play in both loops and not a third time in the tail.
    let clicks: Vec<usize> = (0..left.len()).filter(|&frame| left[frame] > 0.9).collect();
    assert_eq!(clicks, vec![0, 90_000, 96_000, 186_000]);
    // The last note rings on into the tail for its whole second.
    assert_eq!(left[2 * 96_000 + 10_000], 0.25);
    assert_eq!(
        sounding_frames(audio.samples()).last(),
        Some(&(186_000 + 47_999))
    );
}

#[test]
fn song_mode_renders_to_the_end_of_the_last_clip() {
    let mut rig = Rig::new();
    let click = rig.channel(impulse(RATE));
    rig.steps(click, &[0, 4]);
    let lane = rig.playlist_track();
    let pattern = rig.first_pattern();
    rig.clip(lane, pattern, 960, 1_920);
    // Looping is a transport setting and has no place in an export.
    let options = RenderOptions {
        mode: PlayMode::Song,
        tail_secs: 0.5,
        ..RenderOptions::default()
    };
    let audio = render_all(&rig, &options);
    // The song is 2880 ticks, 72000 frames, long.
    assert_eq!(audio.frames(), 72_000 + 24_000);
    assert_eq!(sounding_frames(audio.samples()), vec![24_000, 48_000]);
}

#[test]
fn the_chosen_pattern_is_the_one_rendered() {
    let mut rig = Rig::new();
    let click = rig.channel(impulse(RATE));
    rig.steps(click, &[0]);
    let second = rig.pattern(4);
    rig.note_in(second, click, 480, 240);
    let options = RenderOptions {
        pattern: Some(second),
        ..RenderOptions::default()
    };
    let audio = render_all(&rig, &options);
    assert_eq!(audio.frames(), 24_000);
    assert_eq!(sounding_frames(audio.samples()), vec![12_000]);
}

#[test]
fn a_render_at_another_sample_rate_keeps_the_timing() {
    let mut rig = Rig::new();
    let click = rig.channel(impulse(44_100));
    rig.steps(click, &[4]);
    let options = RenderOptions {
        sample_rate: 44_100,
        ..RenderOptions::default()
    };
    let audio = render_all(&rig, &options);
    assert_eq!(audio.frames(), 88_200);
    assert_eq!(sounding_frames(audio.samples()), vec![22_050]);
}
