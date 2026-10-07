//! Audio clips on the playlist: where they start, which frames of their
//! audio they play, and what shapes their level.

use std::ops::Range;

use windfall_core::{AudioBuffer, db_to_gain};
use windfall_dsp::DelayParams;
use windfall_engine::{RenderOptions, render, render_reporting};
use windfall_ipc::{PlayMode, TransportPatch};
use windfall_project::{ClipId, EffectParams, Send, TrackId};

use crate::support::{
    Rig, counted, counting, idle_limiter, impulse, largest_step, left, level, limiter, peak, right,
    run, sounding_frames,
};

const RATE: u32 = 48_000;

/// Frames in a tick at 120 bpm and 48 kHz.
const TICK: usize = 25;

/// Frames of the fade the engine puts wherever it cuts a clip's audio:
/// 3 ms.
const DECLICK: usize = 144;

/// The level of that fade on its `step`th frame of going up, or with
/// `step` frames left of going down.
fn declick(step: usize) -> f32 {
    if step < DECLICK {
        step as f32 / DECLICK as f32
    } else {
        1.0
    }
}

/// A project with one audio clip that plays a [`counting`] buffer of
/// `frames` frames into the master.
fn one_clip(frames: usize, start: u32, length: u32) -> (Rig, ClipId) {
    let mut rig = Rig::new();
    let lane = rig.playlist_track();
    let audio = counting(RATE, frames);
    let clip = rig.audio_clip(lane, audio, TrackId::MASTER, start, length);
    (rig, clip)
}

/// Checks that every frame of `range` holds exactly what `expected` says
/// it does.
#[track_caller]
fn assert_frames(signal: &[f32], range: Range<usize>, expected: impl Fn(usize) -> f32) {
    let frames = signal.iter().enumerate();
    for (frame, sample) in frames.take(range.end).skip(range.start) {
        assert_eq!(*sample, expected(frame), "frame {frame}");
    }
}

fn silent(signal: &[f32]) -> bool {
    signal.iter().all(|sample| *sample == 0.0)
}

#[test]
fn a_clip_starts_on_the_frame_of_its_tick_and_plays_its_audio_as_it_is() {
    // A tenth of a second of audio, a beat into the song.
    let (rig, _) = one_clip(4_800, 960, 3_840);
    let out = rig.play_song(RATE, 60_000, 480);
    let (left, right) = (left(&out), right(&out));
    assert_eq!(left, right);

    let start = 960 * TICK;
    assert!(silent(&left[..start]));
    // The audio comes out frame for frame from its first frame on, with
    // 3 ms of fade where it ends.
    for frame in 0..4_800 {
        let fade = declick(4_800 - frame);
        assert_eq!(left[start + frame], counted(frame) * fade, "frame {frame}");
    }
    assert_eq!(left[start], counted(0));
    assert!(silent(&left[start + 4_800..]));
}

#[test]
fn a_hit_on_the_first_frame_of_the_audio_is_not_faded_and_a_cut_into_it_is() {
    // A full-scale frame at the very start of a tenth of a second, the
    // way a drum loop begins, and half scale from there on.
    let mut audio = vec![0.5; 4_800];
    audio[0] = 1.0;
    let audio = AudioBuffer::from_interleaved(RATE, 1, audio);
    let mut rig = Rig::new();
    let lane = rig.playlist_track();
    let clip = rig.audio_clip(lane, audio, TrackId::MASTER, 960, 3_840);
    let start = 960 * TICK;

    // The clip starts where its audio does: the hit comes out whole.
    let out = left(&rig.play_song(RATE, 30_000, 480));
    assert!(silent(&out[..start]));
    assert_eq!(out[start], 1.0);
    assert!(
        out[start + 1..start + 4_800 - DECLICK]
            .iter()
            .all(|s| *s == 0.5)
    );
    // The same when the song is played from the clip's own tick, and each
    // time a looping song comes around to it.
    let (mut processor, controller) = rig.song_processor(RATE);
    controller.set_transport(TransportPatch {
        loop_song: Some(true),
        ..TransportPatch::default()
    });
    controller.seek(960.0);
    controller.play();
    let out = left(&run(&mut processor, 130_000, 480));
    assert_eq!(out[0], 1.0);
    // Four beats on the song is over, and a beat into the next time
    // around the clip starts again.
    assert_eq!(out[96_000 + start], 1.0);
    assert_eq!(out[96_000 + start + 1], 0.5);

    // A start anywhere else cuts into the audio, and is faded over 3 ms:
    // with an offset of a tick, 25 frames,
    rig.clip_mut(clip).offset = 1;
    let out = left(&rig.play_song(RATE, 30_000, 480));
    for frame in 0..DECLICK {
        assert_eq!(
            out[start + frame],
            0.5 * declick(frame + 1),
            "frame {frame}"
        );
    }
    assert_eq!(out[start + DECLICK], 0.5);
    // reversed, which starts on the last frame of the file,
    rig.clip_mut(clip).offset = 0;
    rig.audio_mut(clip, |audio| audio.reverse = true);
    let out = left(&rig.play_song(RATE, 30_000, 480));
    assert_eq!(out[start], 0.5 * declick(1));
    assert_eq!(out[start + DECLICK], 0.5);
    // and when playback begins inside the clip, a frame in.
    rig.audio_mut(clip, |audio| audio.reverse = false);
    let (mut processor, controller) = rig.song_processor(RATE);
    controller.seek(960.0 + 1.0 / TICK as f64);
    controller.play();
    let out = left(&run(&mut processor, 4_800, 480));
    assert_eq!(out[0], 0.5 * declick(1));
    assert_eq!(out[DECLICK], 0.5);
}

#[test]
fn the_frame_a_clip_starts_on_does_not_depend_on_the_buffer_size() {
    // A tick that falls between two frames: at 133 bpm a tick is 22.56
    // frames long.
    let (mut rig, _) = one_clip(4_800, 77, 3_840);
    rig.project.settings.tempo_bpm = 133.0;
    let reference = rig.play_song(RATE, 9_600, 9_600);
    let start = crate::support::frame_of_tick(77, 13_300, RATE);
    assert_eq!(sounding_frames(&reference)[0], start);
    for block in [1, 7, 64, 256, 1_000] {
        assert!(
            rig.play_song(RATE, 9_600, block) == reference,
            "blocks of {block}"
        );
    }
}

#[test]
fn playback_that_begins_inside_a_clip_begins_at_the_right_place_in_its_audio() {
    let (rig, _) = one_clip(48_000, 960, 3_840);
    let (mut processor, controller) = rig.song_processor(RATE);
    // Forty ticks into the clip, which is a thousand frames into its audio.
    controller.seek(1_000.0);
    controller.play();
    let out = left(&run(&mut processor, 4_800, 480));
    for (frame, sample) in out.iter().enumerate() {
        let expected = counted(1_000 + frame) * declick(frame + 1);
        assert_eq!(*sample, expected, "frame {frame}");
    }

    // A seek while it plays fades what was playing out over 3 ms and what
    // is there now in: tick 1920 is 24000 frames into the audio.
    controller.seek(1_920.0);
    let out = left(&run(&mut processor, 4_800, 480));
    for (frame, sample) in out.iter().enumerate() {
        let before = if frame < DECLICK {
            counted(5_800 + frame) * declick(DECLICK - frame)
        } else {
            0.0
        };
        let after = counted(24_000 + frame) * declick(frame + 1);
        assert_eq!(*sample, before + after, "frame {frame}");
    }

    // Past the end of the audio there is nothing to play.
    controller.seek(960.0 + 1_930.0);
    run(&mut processor, DECLICK, 48);
    assert!(silent(&run(&mut processor, 4_800, 480)));
}

#[test]
fn an_offset_skips_as_much_audio_as_those_ticks_last_at_the_projects_tempo() {
    let (mut rig, clip) = one_clip(96_000, 0, 1_920);
    rig.clip_mut(clip).offset = 480;
    // A beat at 120 bpm is a quarter of a second, 12000 frames.
    let out = left(&rig.play_song(RATE, 9_600, 480));
    assert_frames(&out, DECLICK..9_600, |frame| counted(12_000 + frame));

    // At half the tempo the same beat is twice as long.
    rig.project.settings.tempo_bpm = 60.0;
    let out = left(&rig.play_song(RATE, 9_600, 480));
    assert_frames(&out, DECLICK..9_600, |frame| counted(24_000 + frame));

    // An offset past the end of the audio leaves nothing to play.
    rig.clip_mut(clip).offset = 3_840;
    assert!(silent(&rig.play_song(RATE, 9_600, 480)));
}

#[test]
fn the_length_of_a_clip_cuts_its_audio_short_with_a_fade() {
    // Two seconds of audio in a clip of one beat, 24000 frames.
    let (rig, _) = one_clip(96_000, 0, 960);
    let out = left(&rig.play_song(RATE, 30_000, 480));
    assert_frames(&out, 0..24_000, |frame| {
        counted(frame) * declick(24_000 - frame)
    });
    assert!(out[23_999] != 0.0);
    assert!(silent(&out[24_000..]));
}

#[test]
fn gain_and_pan_set_the_level_of_each_side() {
    let (mut rig, clip) = one_clip(9_600, 0, 3_840);
    rig.audio_mut(clip, |audio| {
        audio.gain = 0.5;
        audio.pan = 0.5;
    });
    let out = rig.play_song(RATE, 4_800, 480);
    let (left, right) = (left(&out), right(&out));
    for frame in DECLICK..4_800 {
        // Panned half way right: the left side is turned down by half.
        assert_eq!(left[frame], counted(frame) * 0.25, "frame {frame}");
        assert_eq!(right[frame], counted(frame) * 0.5, "frame {frame}");
    }
}

#[test]
fn a_reversed_clip_plays_from_the_last_frame_back() {
    let (mut rig, clip) = one_clip(9_600, 0, 3_840);
    rig.audio_mut(clip, |audio| audio.reverse = true);
    let out = left(&rig.play_song(RATE, 12_000, 480));
    assert_frames(&out, DECLICK..9_600 - DECLICK, |frame| {
        counted(9_599 - frame)
    });
    assert!(silent(&out[9_600..]));

    // The offset skips from where it starts playing, the end of the file.
    rig.clip_mut(clip).offset = 96;
    let out = left(&rig.play_song(RATE, 12_000, 480));
    assert_frames(&out, DECLICK..7_200 - DECLICK, |frame| {
        counted(7_199 - frame)
    });
    assert!(silent(&out[7_200..]));
}

#[test]
fn pitch_changes_the_speed_like_a_tape() {
    // An octave up goes through the audio twice as fast.
    let (mut rig, clip) = one_clip(48_000, 0, 3_840);
    rig.audio_mut(clip, |audio| audio.pitch = 12.0);
    let out = left(&rig.play_song(RATE, 30_000, 480));
    assert_frames(&out, DECLICK..24_000 - DECLICK, |frame| counted(2 * frame));
    assert!(out[23_999] != 0.0);
    assert!(silent(&out[24_000..]));

    // The offset is counted in beats of the song, so at twice the speed it
    // skips twice the audio.
    rig.clip_mut(clip).offset = 96;
    let out = left(&rig.play_song(RATE, 4_800, 480));
    assert_frames(&out, DECLICK..4_800, |frame| counted(4_800 + 2 * frame));

    // An octave down takes two output frames for each frame of audio, and
    // fills in the ones between.
    rig.clip_mut(clip).offset = 0;
    rig.audio_mut(clip, |audio| audio.pitch = -12.0);
    let out = left(&rig.play_song(RATE, 9_600, 480));
    for frame in (DECLICK..9_598).step_by(2) {
        assert_eq!(out[frame], counted(frame / 2), "frame {frame}");
        let between = out[frame + 1];
        assert!(between > out[frame] && between < out[frame + 2]);
    }
}

#[test]
fn a_file_at_another_rate_plays_at_its_own_speed() {
    // A second of audio at 24 kHz is a second at 48 kHz: 48000 frames out.
    let mut rig = Rig::new();
    let lane = rig.playlist_track();
    rig.audio_clip(lane, counting(24_000, 24_000), TrackId::MASTER, 0, 3_840);
    let out = left(&rig.play_song(RATE, 60_000, 480));
    for frame in (DECLICK..47_000).step_by(2) {
        assert_eq!(out[frame], counted(frame / 2), "frame {frame}");
    }
    assert!(out[47_999] != 0.0);
    assert!(silent(&out[48_000..]));
}

#[test]
fn fades_are_equal_power_curves_between_their_ticks() {
    // A steady level for four seconds, in a clip of one bar, 96000 frames.
    let mut rig = Rig::new();
    let lane = rig.playlist_track();
    let clip = rig.audio_clip(lane, level(RATE, 0.5, 4.0), TrackId::MASTER, 0, 3_840);
    rig.audio_mut(clip, |audio| {
        audio.fade_in = 480;
        audio.fade_out = 960;
    });
    let out = left(&rig.play_song(RATE, 100_000, 480));
    let curve = |part: f64| (part * std::f64::consts::FRAC_PI_2).sin() as f32;

    // In over half a beat, 12000 frames.
    for frame in [DECLICK, 3_000, 6_000, 9_000, 11_999] {
        let expected = 0.5 * curve(frame as f64 / 12_000.0);
        assert!((out[frame] - expected).abs() < 1e-6, "frame {frame}");
    }
    assert!((out[6_000] - 0.5 * std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-6);
    assert!(out[..12_000].is_sorted());
    assert_eq!(out[12_000], 0.5);
    assert!(out[12_000..72_000].iter().all(|sample| *sample == 0.5));

    // Out over the last beat, 24000 frames, to silence on the clip's end.
    for frame in [72_001, 78_000, 84_000, 90_000, 95_800] {
        let expected = 0.5 * curve((96_000 - frame) as f64 / 24_000.0);
        assert!((out[frame] - expected).abs() < 1e-6, "frame {frame}");
    }
    assert!(out[72_000..96_000].is_sorted_by(|a, b| a >= b));
    assert!(out[95_999] > 0.0 && out[95_999] < 1e-5);
    assert!(silent(&out[96_000..]));

    // The fades follow the tempo, as the clip's edges do: at twice the
    // tempo they take half the frames.
    rig.project.settings.tempo_bpm = 240.0;
    let out = left(&rig.play_song(RATE, 50_000, 480));
    assert!((out[3_000] - 0.5 * curve(0.5)).abs() < 1e-6);
    assert_eq!(out[6_000], 0.5);
    assert!((out[42_000] - 0.5 * curve(0.5)).abs() < 1e-6);
    assert!(silent(&out[48_000..]));
}

#[test]
fn two_clips_that_fade_into_each_other_keep_their_power() {
    // The same steady level fading out of one clip and into the next over
    // one beat. Two different sounds would keep their loudness; the same
    // sound twice is louder in the middle, by 3 dB.
    let mut rig = Rig::new();
    let (above, below) = (rig.playlist_track(), rig.playlist_track());
    let first = rig.audio_clip(above, level(RATE, 0.5, 4.0), TrackId::MASTER, 0, 1_920);
    let second = rig.audio_clip(below, level(RATE, 0.5, 4.0), TrackId::MASTER, 960, 1_920);
    rig.audio_mut(first, |audio| audio.fade_out = 960);
    rig.audio_mut(second, |audio| audio.fade_in = 960);
    let out = left(&rig.play_song(RATE, 72_000, 480));
    let fade = |frame: usize| (frame as f64 / 24_000.0 * std::f64::consts::FRAC_PI_2).sin() as f32;
    for frame in [24_000 + DECLICK, 30_000, 36_000, 42_000, 47_000] {
        let (up, down) = (fade(frame - 24_000), fade(48_000 - frame));
        assert!((up * up + down * down - 1.0).abs() < 1e-5);
        assert!(
            (out[frame] - 0.5 * (up + down)).abs() < 1e-6,
            "frame {frame}"
        );
    }
    assert!((out[36_000] - 0.5 * std::f32::consts::SQRT_2).abs() < 1e-6);
}

#[test]
fn a_muted_clip_or_track_is_silent_and_comes_in_mid_way_when_unmuted() {
    let (mut rig, clip) = one_clip(96_000, 0, 3_840);
    rig.clip_mut(clip).muted = true;
    assert!(silent(&rig.play_song(RATE, 9_600, 480)));
    rig.clip_mut(clip).muted = false;
    rig.project.playlist.tracks[0].muted = true;
    assert!(silent(&rig.play_song(RATE, 9_600, 480)));
    // In pattern mode the playlist does not play at all.
    rig.project.playlist.tracks[0].muted = false;
    assert!(silent(&rig.play(RATE, 9_600, 480)));

    // Unmuted while the song plays, the clip joins in where the song is.
    rig.clip_mut(clip).muted = true;
    let (mut processor, controller) = rig.song_processor(RATE);
    controller.play();
    assert!(silent(&run(&mut processor, 10_000, 500)));
    rig.clip_mut(clip).muted = false;
    controller.set_project(&rig.project, &rig.pool);
    let out = left(&run(&mut processor, 4_800, 480));
    for (frame, sample) in out.iter().enumerate() {
        let expected = counted(10_000 + frame) * declick(frame + 1);
        assert_eq!(*sample, expected, "frame {frame}");
    }

    // Muted again, it fades out over 3 ms instead of stopping dead.
    rig.clip_mut(clip).muted = true;
    controller.set_project(&rig.project, &rig.pool);
    let out = left(&run(&mut processor, 4_800, 480));
    for (frame, sample) in out.iter().enumerate() {
        let expected = counted(14_800 + frame) * declick(DECLICK.saturating_sub(frame));
        assert_eq!(*sample, expected, "frame {frame}");
    }
}

#[test]
fn a_clip_goes_through_its_mixer_track() {
    // The track is at half level and sends half of that to a bus.
    let mut rig = Rig::new();
    let lane = rig.playlist_track();
    let (track, bus) = (rig.track(), rig.track());
    rig.track_mut(track).volume = 0.5;
    rig.track_mut(track).sends.push(Send {
        target: bus,
        gain: 0.5,
    });
    let clip = rig.audio_clip(lane, counting(RATE, 9_600), track, 0, 3_840);
    let out = left(&rig.play_song(RATE, 4_800, 480));
    for (frame, sample) in out.iter().enumerate().skip(DECLICK) {
        let expected = counted(frame) * 0.75;
        assert!((sample - expected).abs() < 1e-9, "frame {frame}");
    }

    // A muted track silences it, and a limiter on the track holds it.
    rig.track_mut(track).muted = true;
    assert!(silent(&rig.play_song(RATE, 4_800, 480)));
    rig.track_mut(track).muted = false;
    rig.track_mut(track).volume = 1.0;
    rig.track_mut(track).sends.clear();
    let loud = rig.sample(level(RATE, 0.9, 1.0));
    let windfall_project::ClipContent::Audio { sample, .. } = &mut rig.clip_mut(clip).content
    else {
        panic!("not an audio clip");
    };
    *sample = loud;
    rig.effect(track, limiter(-12.0));
    let out = rig.play_song(RATE, 24_000, 480);
    assert!(peak(&out) <= db_to_gain(-12.0) + 1e-6);
    assert!(peak(&out[9_600..]) > db_to_gain(-12.5));
}

#[test]
fn a_clip_stays_lined_up_with_a_track_that_has_a_limiter() {
    // A click from the step sequencer, through a limiter that looks 5 ms
    // ahead, and the same click a thousand frames into an audio clip on a
    // track with nothing on it. Both belong on frame 24000.
    let mut rig = Rig::new();
    let (drums, tape) = (rig.track(), rig.track());
    rig.effect(drums, idle_limiter(5.0));
    let click = rig.channel_on(impulse(RATE), drums);
    rig.steps(click, &[4]);
    let (lane, other) = (rig.playlist_track(), rig.playlist_track());
    let pattern = rig.first_pattern();
    rig.clip(lane, pattern, 0, 3_840);
    let mut audio = vec![0.0; 4_800];
    audio[1_000] = 0.5;
    let audio = AudioBuffer::from_interleaved(RATE, 1, audio);
    rig.audio_clip(other, audio, tape, 920, 960);

    let options = RenderOptions {
        mode: PlayMode::Song,
        ..RenderOptions::default()
    };
    let rendered = render(&rig.project, &rig.pool, &options, &mut |_| true);
    assert_eq!(sounding_frames(rendered.samples()), [24_000]);
    assert!((left(rendered.samples())[24_000] - 1.5).abs() < 1e-5);

    // Played, both come out late by the limiter's 240 frames.
    let out = rig.play_song(RATE, 30_000, 480);
    assert_eq!(sounding_frames(&out), [24_240]);
    assert!((left(&out)[24_240] - 1.5).abs() < 1e-5);
}

#[test]
fn a_looping_song_cuts_a_clip_at_its_end_and_starts_it_again() {
    // Three seconds of audio in a clip of one bar, which is the song.
    let (rig, _) = one_clip(144_000, 0, 3_840);
    let (mut processor, controller) = rig.song_processor(RATE);
    controller.set_transport(TransportPatch {
        loop_song: Some(true),
        ..TransportPatch::default()
    });
    controller.play();
    let out = left(&run(&mut processor, 200_000, 480));
    for frame in (0..96_000).step_by(7) {
        let fade = declick(96_000 - frame);
        assert_eq!(out[frame], counted(frame) * fade, "frame {frame}");
    }
    assert_eq!(out[95_999], counted(95_999) * declick(1));
    // The second time around is the first again, and nothing of the first
    // is left sounding under it.
    assert!(out[96_000..192_000] == out[..96_000]);

    // Stopping fades the clip out in 3 ms and leaves nothing behind.
    controller.stop();
    let out = left(&run(&mut processor, 4_800, 480));
    assert!(out[..DECLICK].iter().all(|sample| *sample > 0.0));
    assert!(out[..DECLICK].is_sorted_by(|a, b| a > b));
    assert!(silent(&out[DECLICK..]));
    assert!(silent(&run(&mut processor, 48_000, 480)));
}

#[test]
fn a_tempo_change_moves_the_end_of_a_sounding_clip_and_not_its_audio() {
    let (mut rig, _) = one_clip(144_000, 0, 3_840);
    let (mut processor, controller) = rig.song_processor(RATE);
    controller.play();
    let out = left(&run(&mut processor, 24_000, 480));
    assert_eq!(out[20_000], counted(20_000));

    // A beat in, the tempo doubles. The audio carries on frame for frame.
    // The three beats left of the clip now take 12.5 frames a tick, so it
    // ends 36000 frames on.
    rig.project.settings.tempo_bpm = 240.0;
    controller.set_project(&rig.project, &rig.pool);
    let out = left(&run(&mut processor, 48_000, 480));
    assert_frames(&out, 0..36_000, |frame| {
        counted(24_000 + frame) * declick(36_000 - frame)
    });
    assert!(silent(&out[36_000..]));

    // Played again at the new tempo, the clip starts on its tick with the
    // start of its audio and is cut after a bar of 48000 frames.
    let out = left(&rig.play_song(RATE, 60_000, 480));
    for frame in (0..48_000).step_by(5) {
        let fade = declick(48_000 - frame);
        assert_eq!(out[frame], counted(frame) * fade, "frame {frame}");
    }
    assert!(silent(&out[48_000..]));
}

#[test]
fn an_edit_to_a_sounding_clip_glides_or_starts_it_again_where_it_belongs() {
    let mut rig = Rig::new();
    let lane = rig.playlist_track();
    let steady = rig.audio_clip(lane, level(RATE, 0.5, 4.0), TrackId::MASTER, 0, 3_840);
    let (mut processor, controller) = rig.song_processor(RATE);
    controller.play();
    run(&mut processor, 9_600, 480);

    // A new gain is reached in 5 ms, in a straight line.
    rig.audio_mut(steady, |audio| audio.gain = 0.5);
    controller.set_project(&rig.project, &rig.pool);
    let out = left(&run(&mut processor, 960, 480));
    assert!((out[0] - 0.5).abs() < 1e-6);
    assert_eq!(out[240], 0.25);
    assert!(largest_step(&out) <= 0.25 / 240.0 + 1e-6);
    assert!(out[..240].is_sorted_by(|a, b| a > b));

    // A clip that is moved is another sound: what was playing fades out,
    // and what the playhead is in now fades in from the right place.
    let (mut rig, clip) = one_clip(144_000, 0, 3_840);
    let (mut processor, controller) = rig.song_processor(RATE);
    controller.play();
    run(&mut processor, 24_000, 480);
    rig.clip_mut(clip).start = 480;
    controller.set_project(&rig.project, &rig.pool);
    let out = left(&run(&mut processor, 4_800, 480));
    for (frame, sample) in out.iter().enumerate() {
        let before = counted(24_000 + frame) * declick(DECLICK.saturating_sub(frame));
        let after = counted(12_000 + frame) * declick(frame + 1);
        assert_eq!(*sample, before + after, "frame {frame}");
    }

    // The same for a new pitch: an octave up, it is twice as far along.
    rig.audio_mut(clip, |audio| audio.pitch = 12.0);
    controller.set_project(&rig.project, &rig.pool);
    let out = left(&run(&mut processor, 4_800, 480));
    assert_frames(&out, DECLICK..4_800, |frame| counted(2 * (16_800 + frame)));

    // A clip that is deleted fades out.
    rig.project.playlist.clips.clear();
    controller.set_project(&rig.project, &rig.pool);
    let out = left(&run(&mut processor, 4_800, 480));
    assert!(out[..DECLICK].iter().all(|sample| *sample > 0.0));
    assert!(silent(&out[DECLICK..]));
}

#[test]
fn a_hundred_and_twenty_eight_clips_sound_at_once_and_the_rest_stay_silent() {
    // Each clip puts out a steady 1/256 for half a second.
    let mut rig = Rig::new();
    let lane = rig.playlist_track();
    for _ in 0..140 {
        rig.audio_clip(
            lane,
            level(RATE, 1.0 / 256.0, 0.5),
            TrackId::MASTER,
            0,
            3_840,
        );
    }
    let (mut processor, controller) = rig.song_processor(RATE);
    controller.play();
    let out = left(&run(&mut processor, 12_000, 480));
    assert!(out[DECLICK..].iter().all(|sample| *sample == 0.5));
    // The UI is told how many play and how many are left out, for as long
    // as those would have sounded.
    let frame = controller.frame();
    assert_eq!((frame.audio_clips, frame.dropped_clips), (128, 12));
    let out = left(&run(&mut processor, 36_000, 480));
    assert!(out[..12_000 - DECLICK].iter().all(|sample| *sample == 0.5));
    assert!(silent(&out[12_000..]));
    let frame = controller.frame();
    assert_eq!((frame.audio_clips, frame.dropped_clips), (0, 0));

    // Stopped and started again at once, the clips that are fading out do
    // not stand in the way of the ones that start: all 128 play again.
    let (mut processor, controller) = rig.song_processor(RATE);
    controller.play();
    run(&mut processor, 4_800, 480);
    controller.stop();
    controller.play();
    let out = left(&run(&mut processor, 4_800, 480));
    assert!(out[DECLICK..].iter().all(|sample| *sample == 0.5));
    let frame = controller.frame();
    assert_eq!((frame.audio_clips, frame.dropped_clips), (128, 12));
    // Stopped, nothing is left out of anything.
    controller.stop();
    run(&mut processor, 480, 480);
    let frame = controller.frame();
    assert_eq!((frame.audio_clips, frame.dropped_clips), (0, 0));
}

#[test]
fn the_clips_left_out_are_the_ones_that_start_last_and_then_the_newest() {
    // 127 clips at 1/256 on the first tick. Then, in the order they were
    // made: one at 1/8 that starts two beats later, and two more on the
    // first tick, at 1/16 and at 1/32.
    let mut rig = Rig::new();
    let lane = rig.playlist_track();
    let mut add = |value: f32, start: u32| {
        let audio = level(RATE, value, 2.0);
        rig.audio_clip(lane, audio, TrackId::MASTER, start, 3_840 - start)
    };
    for _ in 0..127 {
        add(1.0 / 256.0, 0);
    }
    let late = add(1.0 / 8.0, 1_920);
    let (kept, newest) = (add(1.0 / 16.0, 0), add(1.0 / 32.0, 0));
    assert!(late < kept && kept < newest);

    // Of the 129 that start together the one made last finds no slot, and
    // neither does the one that starts later: 127/256 and 1/16 are heard.
    let options = RenderOptions {
        mode: PlayMode::Song,
        ..RenderOptions::default()
    };
    let rendered = render_reporting(&rig.project, &rig.pool, &options, &mut |_| true);
    assert_eq!(rendered.dropped_clips, 2);
    let out = left(rendered.audio.samples());
    assert_eq!(out.len(), 96_000);
    let heard = 127.0 / 256.0 + 1.0 / 16.0;
    assert!(out[DECLICK..96_000 - DECLICK].iter().all(|s| *s == heard));

    // Playback leaves out the same two, and says so while each would be
    // sounding: one from the start, both from the third beat on.
    let (mut processor, controller) = rig.song_processor(RATE);
    controller.play();
    let played = run(&mut processor, 24_000, 480);
    let frame = controller.frame();
    assert_eq!((frame.audio_clips, frame.dropped_clips), (128, 1));
    let more = run(&mut processor, 48_000, 480);
    assert_eq!(controller.frame().dropped_clips, 2);
    let rest = run(&mut processor, 24_000, 480);
    let played = [played, more, rest].concat();
    assert!(rendered.audio.samples() == played);

    // The order the clips have in the project makes no difference.
    rig.project.playlist.clips.reverse();
    let again = render_reporting(&rig.project, &rig.pool, &options, &mut |_| true);
    assert_eq!(again.dropped_clips, 2);
    assert!(again.audio.samples() == rendered.audio.samples());

    // With room for everything, nothing is reported.
    rig.project.playlist.clips.truncate(100);
    let roomy = render_reporting(&rig.project, &rig.pool, &options, &mut |_| true);
    assert_eq!(roomy.dropped_clips, 0);
}

#[test]
fn a_clip_without_audio_is_silent_and_still_counts_for_the_length_of_the_song() {
    let (mut rig, clip) = one_clip(9_600, 960, 2_880);
    let windfall_project::ClipContent::Audio { sample, .. } = rig.clip_mut(clip).content else {
        panic!("not an audio clip");
    };
    rig.pool.remove(sample);
    let options = RenderOptions {
        mode: PlayMode::Song,
        ..RenderOptions::default()
    };
    let rendered = render(&rig.project, &rig.pool, &options, &mut |_| true);
    assert_eq!(rendered.frames(), 96_000);
    assert!(silent(rendered.samples()));
}

#[test]
fn an_automatic_tail_waits_for_what_a_clip_leaves_in_its_effects() {
    // A short clip at the very end of a one-bar song, into an echo half a
    // second long.
    let mut rig = Rig::new();
    let lane = rig.playlist_track();
    let track = rig.track();
    let mut burst = vec![0.0; 2_400];
    burst[1_200] = 0.5;
    let burst = AudioBuffer::from_interleaved(RATE, 1, burst);
    rig.audio_clip(lane, burst, track, 3_744, 96);
    let options = RenderOptions {
        mode: PlayMode::Song,
        tail_secs: 10.0,
        auto_tail: true,
        ..RenderOptions::default()
    };
    // Dry, the song ends with its last clip.
    let dry = render(&rig.project, &rig.pool, &options, &mut |_| true);
    assert_eq!(dry.frames(), 96_000);
    assert_eq!(sounding_frames(dry.samples()), [93_600 + 1_200]);

    rig.effect(
        track,
        EffectParams::Delay(DelayParams {
            sync: false,
            time_ms: 500.0,
            feedback: 0.0,
            mix: 0.5,
            ..DelayParams::default()
        }),
    );
    let wet = render(&rig.project, &rig.pool, &options, &mut |_| true);
    let frames = sounding_frames(wet.samples());
    assert_eq!(frames[0], 94_800);
    let echo = *frames.last().unwrap();
    assert!(
        echo.abs_diff(94_800 + 24_000) < 200,
        "the echo is on {echo}"
    );
    assert!(wet.frames() > 96_000 + 20_000 && wet.frames() < 96_000 + 30_000);
}
