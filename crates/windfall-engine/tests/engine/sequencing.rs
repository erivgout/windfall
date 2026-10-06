//! Where notes land: exact frames, loops, swing, song mode, tempo changes.

use windfall_core::TICKS_PER_STEP;
use windfall_ipc::{PlayMode, TransportPatch};
use windfall_project::Envelope;

use crate::support::{Rig, frame_of_tick, impulse, left, level, run, sine, sounding_frames};

const BAR: u64 = 16 * TICKS_PER_STEP as u64;

/// A project with one impulse channel playing the given steps.
fn impulse_rig(sample_rate: u32, steps: &[u32]) -> Rig {
    let mut rig = Rig::new();
    let channel = rig.channel(impulse(sample_rate));
    rig.steps(channel, steps);
    rig
}

fn song_mode(loop_song: bool) -> TransportPatch {
    TransportPatch {
        mode: Some(PlayMode::Song),
        pattern: None,
        loop_song: Some(loop_song),
    }
}

#[test]
fn steps_land_on_the_exact_frame_through_loop_wraps() {
    let steps = [0, 3, 4, 7, 10, 15];
    // Tempo in hundredths of a bpm, and the sample rate.
    let cases = [
        (12_000, 48_000),
        (14_000, 48_000),
        (9_300, 44_100),
        // Step 7 of the second pass is exactly frame 220500 here, and
        // floating point puts it a hair past that.
        (6_900, 44_100),
        (12_750, 96_000),
        (17_400, 22_050),
        (6_133, 32_000),
        (52_200, 8_000),
    ];
    for (bpm_times_100, sample_rate) in cases {
        let mut rig = impulse_rig(sample_rate, &steps);
        rig.project.settings.tempo_bpm = bpm_times_100 as f64 / 100.0;

        let loops = 3;
        let expected: Vec<usize> = (0..loops)
            .flat_map(|pass| {
                steps.iter().map(move |step| {
                    let tick = pass * BAR + u64::from(step * TICKS_PER_STEP);
                    frame_of_tick(tick, bpm_times_100, sample_rate)
                })
            })
            .collect();
        let frames = frame_of_tick(loops * BAR, bpm_times_100, sample_rate);

        for block in [7, 64, 1000] {
            let audio = rig.play(sample_rate, frames, block);
            assert_eq!(
                sounding_frames(&audio),
                expected,
                "{} bpm at {sample_rate} Hz in blocks of {block}",
                bpm_times_100 as f64 / 100.0
            );
        }
    }
}

#[test]
fn every_buffer_size_gives_bit_identical_output() {
    let sample_rate = 48_000;
    let mut rig = Rig::new();
    rig.project.settings.tempo_bpm = 133.0;
    rig.project.settings.swing = 0.6;
    let bus = rig.track();
    let effects = rig.track();
    rig.track_mut(bus).pan = -0.3;
    rig.track_mut(bus).sends.push(windfall_project::Send {
        target: effects,
        gain: 0.5,
    });

    // A resampled, pitched voice.
    let lead = rig.channel_on(sine(44_100, 330.0, 0.4), bus);
    rig.sampler_mut(lead).envelope = Some(Envelope {
        attack_ms: 3.0,
        decay_ms: 40.0,
        sustain: 0.6,
        release_ms: 30.0,
    });
    rig.channel_mut(lead).pan = 0.2;
    for (index, start) in [0, 250, 700, 1500, 2890].into_iter().enumerate() {
        let note = rig.note(lead, start, 300);
        note.key = 55 + index as u8 * 3;
        note.pan = index as f32 * 0.2 - 0.4;
        note.velocity = 0.5 + index as f32 * 0.1;
    }
    // A choked one-shot.
    let hat = rig.channel_on(sine(48_000, 5_000.0, 0.3), effects);
    rig.sampler_mut(hat).cut_self = true;
    rig.steps(hat, &[0, 1, 2, 3, 5, 7, 8, 11, 13, 14, 15]);
    // An exact-frame marker.
    let click = rig.channel(impulse(sample_rate));
    rig.steps(click, &[0, 4, 8, 12, 15]);

    let frames = sample_rate as usize * 4;
    let reference = rig.play(sample_rate, frames, 128);
    assert!(sounding_frames(&reference).len() > frames / 2);
    for block in [1, 7, 64, 1024, 4096] {
        let audio = rig.play(sample_rate, frames, block);
        let same = audio
            .iter()
            .zip(&reference)
            .all(|(a, b)| a.to_bits() == b.to_bits());
        assert!(same, "blocks of {block} frames gave different audio");
    }
}

#[test]
fn a_note_just_before_the_loop_point_plays_on_the_frame_of_the_wrap() {
    // At 522 bpm and 8 kHz a tick is less than a frame. With a pattern of 9
    // steps, its last tick and the first tick of the next pass fall on the
    // same frame.
    let (bpm_times_100, sample_rate) = (52_200, 8_000);
    let length = 9 * u64::from(TICKS_PER_STEP);
    let wrap = frame_of_tick(length, bpm_times_100, sample_rate);
    assert_eq!(frame_of_tick(length - 1, bpm_times_100, sample_rate), wrap);

    let mut rig = Rig::new();
    rig.project.settings.tempo_bpm = 522.0;
    let pattern = rig.first_pattern();
    rig.pattern_mut(pattern).length_steps = 9;
    let channel = rig.channel(impulse(sample_rate));
    rig.note(channel, 0, 1);
    rig.note(channel, 700, 1);
    rig.note(channel, length as u32 - 1, 1);

    let passes = 4;
    let frames = frame_of_tick(passes * length, bpm_times_100, sample_rate);
    let mut expected = vec![0.0; frames];
    for pass in 0..passes {
        for tick in [0, 700, length - 1] {
            let frame = frame_of_tick(pass * length + tick, bpm_times_100, sample_rate);
            if frame < frames {
                expected[frame] += 1.0;
            }
        }
    }
    // The shared frame carries both notes.
    assert_eq!(expected[wrap], 2.0);
    for block in [1, 7, 64, 128, 1024] {
        let audio = left(&rig.play(sample_rate, frames, block));
        assert_eq!(audio, expected, "blocks of {block}");
    }
}

#[test]
fn swing_delays_every_second_step() {
    let sample_rate = 48_000;
    let mut rig = impulse_rig(sample_rate, &[0, 1, 2, 3, 15]);
    // 120 bpm at 48 kHz is 25 frames a tick, so a step is 6000 frames.
    for (swing, late) in [(0.0, 0), (0.5, 1000), (1.0, 2000)] {
        rig.project.settings.swing = swing;
        let audio = rig.play(sample_rate, 96_000, 256);
        assert_eq!(
            sounding_frames(&audio),
            vec![0, 6_000 + late, 12_000, 18_000 + late, 90_000 + late],
            "swing {swing}"
        );
    }
}

#[test]
fn notes_at_or_after_the_pattern_length_do_not_play() {
    let sample_rate = 48_000;
    let mut rig = impulse_rig(sample_rate, &[2, 8, 12]);
    let pattern = rig.first_pattern();
    rig.pattern_mut(pattern).length_steps = 8;
    // The pattern loops every 8 steps, 48000 frames, and only step 2 plays.
    let audio = rig.play(sample_rate, 144_000, 512);
    assert_eq!(sounding_frames(&audio), vec![12_000, 60_000, 108_000]);
}

#[test]
fn the_playhead_follows_playback_and_wraps_with_the_pattern() {
    let sample_rate = 48_000;
    let rig = impulse_rig(sample_rate, &[0]);
    let (mut processor, controller) = rig.processor(sample_rate);
    assert!(!controller.frame().playing);

    controller.play();
    assert!(controller.transport().playing);
    run(&mut processor, 12_000, 128);
    let frame = controller.frame();
    assert!(frame.playing);
    assert!((frame.tick - 480.0).abs() < 1e-6, "tick {}", frame.tick);

    // One bar is 96000 frames, so this is 100 ticks into the second pass.
    run(&mut processor, 84_000 + 2_500, 128);
    let tick = controller.frame().tick;
    assert!((tick - 100.0).abs() < 1e-6, "tick {tick}");

    controller.stop();
    run(&mut processor, 128, 128);
    let frame = controller.frame();
    assert!(!frame.playing && !controller.transport().playing);
    assert_eq!(frame.tick, 0.0);
}

#[test]
fn seeking_jumps_without_playing_what_was_skipped() {
    let sample_rate = 48_000;
    let rig = impulse_rig(sample_rate, &[0, 4, 8, 12]);
    let (mut processor, controller) = rig.processor(sample_rate);
    controller.play();
    let mut audio = run(&mut processor, 1_000, 100);
    // Jump to one tick before step 8, 25 frames ahead of it.
    controller.seek(1919.0);
    audio.extend(run(&mut processor, 30_000, 100));
    assert_eq!(sounding_frames(&audio), vec![0, 1_025, 25_025]);

    // A note a hair behind the place jumped to stays unplayed: step 8 is
    // skipped and step 12 is next. After the pattern wraps, every step
    // plays again, the ones before the jump included.
    controller.seek(1920.05);
    let audio = run(&mut processor, 60_000, 100);
    assert_eq!(sounding_frames(&audio), vec![23_999, 47_999]);

    // A jump far past the end of the pattern lands inside it.
    controller.seek(1e9 + 100.0);
    run(&mut processor, 100, 100);
    let frame = controller.frame();
    assert!(frame.playing);
    assert!(frame.tick < 3_840.0, "the playhead is at {}", frame.tick);

    // A seek made while stopped is where the next play starts, and where
    // stop comes back to.
    controller.stop();
    controller.seek(960.0);
    run(&mut processor, 100, 100);
    assert_eq!(controller.frame().tick, 960.0);
    controller.play();
    let audio = run(&mut processor, 100, 100);
    assert_eq!(sounding_frames(&audio), vec![0]);
    controller.stop();
    run(&mut processor, 100, 100);
    assert_eq!(controller.frame().tick, 960.0);
}

#[test]
fn a_tempo_change_keeps_the_playhead_and_places_later_notes_exactly() {
    let sample_rate = 48_000;
    let mut rig = impulse_rig(sample_rate, &[0, 1, 2, 3, 4]);
    // The change comes 10000 frames in, at tick 400, part way through step 1.
    let change_at = 10_000;
    let expected = vec![
        0,
        6_000,
        // 80 ticks to step 2 at 150 bpm, which is 20 frames a tick.
        change_at + 80 * 20,
        change_at + 320 * 20,
        change_at + 560 * 20,
    ];
    let mut outputs = Vec::new();
    for block in [1, 7, 64, 128, 1024] {
        rig.project.settings.tempo_bpm = 120.0;
        let (mut processor, controller) = rig.processor(sample_rate);
        controller.play();
        let mut audio = run(&mut processor, change_at, block);
        assert!((controller.frame().tick - 400.0).abs() < 1e-9);

        rig.project.settings.tempo_bpm = 150.0;
        controller.set_project(&rig.project, &rig.pool);
        audio.extend(run(&mut processor, 100, block));
        let tick = controller.frame().tick;
        assert!((tick - 405.0).abs() < 1e-9, "the playhead jumped to {tick}");

        audio.extend(run(&mut processor, 20_000, block));
        assert_eq!(sounding_frames(&audio), expected, "blocks of {block}");
        outputs.push(audio);
    }
    assert!(outputs.iter().all(|audio| audio == &outputs[0]));
}

#[test]
fn a_tempo_change_on_the_frame_of_a_note_plays_it_once() {
    let sample_rate = 48_000;
    let mut rig = impulse_rig(sample_rate, &[0, 1, 2]);
    // Step 1 is due on frame 6000. A change one frame earlier slows the last
    // stretch before it, 0.04 ticks, from 1 frame to 1.25, so it lands a
    // frame later. A change on its frame or after it must not move it.
    for (change_at, second) in [(5_999, 6_001), (6_000, 6_000), (6_001, 6_000)] {
        rig.project.settings.tempo_bpm = 120.0;
        let (mut processor, controller) = rig.processor(sample_rate);
        controller.play();
        let mut audio = run(&mut processor, change_at, 512);
        rig.project.settings.tempo_bpm = 96.0;
        controller.set_project(&rig.project, &rig.pool);
        audio.extend(run(&mut processor, 12_000, 512));
        let sounding = sounding_frames(&audio);
        assert_eq!(sounding.len(), 3, "change at {change_at}: {sounding:?}");
        assert_eq!(sounding[..2], [0, second], "change at {change_at}");
    }
}

#[test]
fn song_mode_plays_clips_with_offset_length_and_mute() {
    let sample_rate = 48_000;
    let mut rig = Rig::new();
    let channel = rig.channel(impulse(sample_rate));
    let pattern = rig.first_pattern();
    rig.pattern_mut(pattern).length_steps = 4;
    rig.steps(channel, &[0, 1]);
    let other = rig.pattern(4);
    rig.note_in(other, channel, 2 * TICKS_PER_STEP, TICKS_PER_STEP);

    let lane = rig.playlist_track();
    let muted_lane = rig.playlist_track();
    rig.project.playlist.tracks[1].muted = true;
    // The first pattern from one step in, repeating, and ending on the
    // very tick a note would start.
    rig.clip(lane, pattern, 960, 1920).offset = TICKS_PER_STEP;
    rig.clip(lane, other, 4800, 960);
    rig.clip(lane, other, 5760, 960).muted = true;
    rig.clip(muted_lane, pattern, 0, 6720);

    let (mut processor, controller) = rig.processor(sample_rate);
    controller.set_transport(song_mode(false));
    controller.play();
    // The song is 7 beats, 168000 frames, long. One more block lets the
    // transport reach the end.
    let audio = run(&mut processor, 168_300, 300);

    // The first clip covers ticks 960..2880 and shows the pattern from tick
    // 240 on: step 1 at once, then steps 0 and 1 of the next repeat, then
    // step 0 of the one after. Its step 1 would fall on tick 2880, which is
    // where the clip ends, so it does not play.
    let ticks = [960, 1680, 1920, 2640, 4800 + 480];
    let expected: Vec<usize> = ticks.iter().map(|tick| tick * 25).collect();
    assert_eq!(sounding_frames(&audio), expected);

    // The song ended with its last clip, muted or not, and went back to the
    // start.
    let frame = controller.frame();
    assert!(!frame.playing);
    assert_eq!(frame.tick, 0.0);
    assert!(!controller.transport().playing);
}

#[test]
fn a_looping_song_starts_over_at_the_end_of_the_last_clip() {
    let sample_rate = 48_000;
    let mut rig = impulse_rig(sample_rate, &[0, 6]);
    let lane = rig.playlist_track();
    let pattern = rig.first_pattern();
    rig.clip(lane, pattern, 480, 1920);

    // The song is 2400 ticks, 60000 frames, long.
    let mut outputs = Vec::new();
    for block in [1, 7, 64, 128, 1024] {
        let (mut processor, controller) = rig.processor(sample_rate);
        controller.set_transport(song_mode(true));
        controller.play();
        let audio = run(&mut processor, 180_000, block);
        let pass = [480 * 25, (480 + 1440) * 25];
        let expected: Vec<usize> = (0..3)
            .flat_map(|index| pass.map(|frame| frame + index * 60_000))
            .collect();
        assert_eq!(sounding_frames(&audio), expected, "blocks of {block}");
        assert!(controller.frame().playing);
        outputs.push(audio);
    }
    assert!(outputs.iter().all(|audio| audio == &outputs[0]));
}

#[test]
fn a_held_note_in_a_clip_ends_with_the_clip() {
    let sample_rate = 48_000;
    let mut rig = Rig::new();
    let channel = rig.channel(level(sample_rate, 0.5, 4.0));
    rig.sampler_mut(channel).envelope = Some(Envelope {
        attack_ms: 0.0,
        decay_ms: 0.0,
        sustain: 1.0,
        release_ms: 1.0,
    });
    // The note is a whole bar long but the clip shows only its first beat.
    rig.note(channel, 0, 3840);
    let lane = rig.playlist_track();
    let pattern = rig.first_pattern();
    rig.clip(lane, pattern, 0, 960);
    rig.clip(lane, pattern, 3840, 1);

    let (mut processor, controller) = rig.processor(sample_rate);
    controller.set_transport(song_mode(false));
    controller.play();
    let audio = run(&mut processor, 48_000, 256);
    let sounding = sounding_frames(&audio);
    // One beat is 24000 frames, and the release adds a millisecond, which
    // is 48 frames.
    assert_eq!(sounding.first(), Some(&0));
    let last = *sounding.last().expect("the note sounded");
    assert!(
        (24_040..=24_050).contains(&last),
        "the note ended on {last}"
    );
}

#[test]
fn switching_pattern_and_mode_while_playing() {
    let sample_rate = 48_000;
    let mut rig = impulse_rig(sample_rate, &[0, 4]);
    let channel = rig.project.channels[0].id;
    let second = rig.pattern(8);
    rig.note_in(second, channel, 6 * TICKS_PER_STEP, TICKS_PER_STEP);

    let (mut processor, controller) = rig.processor(sample_rate);
    controller.play();
    let mut audio = run(&mut processor, 30_000, 250);
    // The playhead, five steps in, carries over into the other pattern.
    controller.set_transport(TransportPatch {
        pattern: Some(second),
        ..TransportPatch::default()
    });
    audio.extend(run(&mut processor, 30_000, 250));
    assert_eq!(sounding_frames(&audio), vec![0, 24_000, 36_000]);
    assert_eq!(controller.transport().pattern, second);

    // An empty song has nothing to play, so playback ends.
    controller.set_transport(song_mode(false));
    run(&mut processor, 250, 250);
    let transport = controller.transport();
    assert_eq!(transport.mode, PlayMode::Song);
    assert!(!transport.playing);
}

#[test]
fn a_transport_pattern_missing_from_the_project_falls_back_to_the_first() {
    let sample_rate = 48_000;
    let rig = impulse_rig(sample_rate, &[1]);
    let (mut processor, controller) = rig.processor(sample_rate);
    assert_eq!(controller.transport().pattern, rig.first_pattern());
    controller.play();
    let audio = run(&mut processor, 10_000, 256);
    assert_eq!(sounding_frames(&audio), vec![6_000]);
}
