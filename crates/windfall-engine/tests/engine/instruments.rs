//! Instrument channels: notes on their exact frames and held to their
//! ticks, the transport, notes played by hand, the channel's gain and pan,
//! and what happens across edits.

use windfall_dsp::{EffectParams, EqParams};
use windfall_ipc::{PlayMode, TransportPatch};
use windfall_project::{ChannelId, TrackId};

use crate::support::{
    Rig, impulse, largest_step, left, level, peak, plain_synth, right, rms, run, sounding_frames,
};

const RATE: u32 = 48_000;

/// Frames by which the synth's output lags its notes.
const LATENCY: usize = 12;

/// Frames within which the plain synth is silent after one of its notes
/// has ended: a millisecond of release, the fade that ends a voice, the
/// synth's latency, and the few samples its filters ring for.
const RELEASE_FRAMES: usize = 400;

/// A project with one channel that plays the plain synth into the master.
fn synth_rig() -> (Rig, ChannelId) {
    let mut rig = Rig::new();
    let synth = rig.synth_on(plain_synth(), TrackId::MASTER);
    (rig, synth)
}

/// The first and the last frame on which either side is not silent.
fn span(audio: &[f32]) -> (usize, usize) {
    let frames = sounding_frames(audio);
    let first = *frames.first().expect("something sounds");
    (first, *frames.last().expect("something sounds"))
}

/// The most a sine of this peak level at middle C, the default key, moves
/// from one frame to the next.
fn slope_of(level: f32) -> f32 {
    level * 261.63 * std::f32::consts::TAU / RATE as f32
}

#[test]
fn a_note_starts_on_its_frame_and_ends_on_its_tick() {
    // The note starts on tick 240, frame 6000, and is held for 480 ticks,
    // to frame 18000.
    let (mut rig, synth) = synth_rig();
    rig.note(synth, 240, 480);
    let audio = rig.play(RATE, 30_000, 128);
    let (first, last) = span(&audio);
    assert!(
        (6_000..=6_000 + 2 * LATENCY).contains(&first),
        "the note began on frame {first}"
    );
    assert!(
        (18_000..18_000 + RELEASE_FRAMES).contains(&last),
        "the note was over on frame {last}"
    );
    // In between it holds its level.
    let held = left(&audio);
    assert!(rms(&held[7_000..17_000]) > 0.2);
    assert!((peak(&held[7_000..12_000]) - peak(&held[12_000..17_000])).abs() < 1e-3);

    // However the output is cut into buffers, the audio is the same.
    for block in [1, 7, 64, 256, 1_024, 30_000] {
        assert!(rig.play(RATE, 30_000, block) == audio, "blocks of {block}");
    }
    let (_, controller) = rig.processor(RATE);
    assert_eq!(controller.latency_frames(), LATENCY as u32);
}

#[test]
fn a_note_one_frame_later_sounds_one_frame_later() {
    // At 48 kHz and 125 bpm a tick is 24 frames, so a note two ticks later
    // is 48 frames later, and so is every sample of it once both are held.
    let onset = |tick: u32| {
        let (mut rig, synth) = synth_rig();
        rig.project.settings.tempo_bpm = 125.0;
        rig.note(synth, tick, 960);
        span(&rig.play(RATE, 30_000, 256)).0
    };
    assert_eq!(onset(102), onset(100) + 48);
    assert_eq!(onset(100), onset(0) + 2_400);
}

/// Plays `rig` at 120 bpm, changes the tempo to `tempo` on frame
/// `change_at`, and returns the last frame that sounds.
fn end_across_a_tempo_change(rig: &mut Rig, change_at: usize, tempo: f64, frames: usize) -> usize {
    let mut outputs = Vec::new();
    for block in [7, 128, 1_024] {
        rig.project.settings.tempo_bpm = 120.0;
        let (mut processor, controller) = rig.processor(RATE);
        controller.play();
        let mut audio = run(&mut processor, change_at, block);
        rig.project.settings.tempo_bpm = tempo;
        controller.set_project(&rig.project, &rig.pool);
        audio.extend(run(&mut processor, frames - change_at, block));
        assert_eq!(controller.frame().voices, 0, "a note is stuck");
        outputs.push(audio);
    }
    assert!(
        outputs.iter().all(|audio| audio == &outputs[0]),
        "the buffer size changed the audio"
    );
    span(&outputs[0]).1
}

#[test]
fn a_tempo_change_moves_the_end_of_a_sounding_note() {
    // A quarter note, 960 ticks, would end on frame 24000 at 120 bpm. The
    // change comes on frame 6000, at tick 240, with 720 ticks to go.
    let (mut rig, synth) = synth_rig();
    let pattern = rig.first_pattern();
    rig.pattern_mut(pattern).length_steps = 64;
    rig.note(synth, 0, 960);

    // At 60 bpm a tick takes 50 frames, so the note ends on frame 42000.
    let end = end_across_a_tempo_change(&mut rig, 6_000, 60.0, 50_000);
    assert!((42_000..42_000 + RELEASE_FRAMES).contains(&end), "{end}");
    // At 240 bpm a tick takes 12.5 frames, so it ends on frame 15000.
    let end = end_across_a_tempo_change(&mut rig, 6_000, 240.0, 20_000);
    assert!((15_000..15_000 + RELEASE_FRAMES).contains(&end), "{end}");
}

#[test]
fn a_note_held_across_the_loop_point_ends_in_the_next_pass() {
    // A pattern of four steps is 24000 frames long. The note starts on
    // tick 720, frame 18000, and is held for 480 ticks, 240 of them in the
    // next pass, so it ends on frame 30000 and starts again on 42000.
    let (mut rig, synth) = synth_rig();
    let pattern = rig.first_pattern();
    rig.pattern_mut(pattern).length_steps = 4;
    rig.note(synth, 720, 480);
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let audio = run(&mut processor, 100_000, 128);
    let frames = sounding_frames(&audio);
    for pass in 0..3 {
        let (start, end) = (18_000 + pass * 24_000, 30_000 + pass * 24_000);
        let sounding: Vec<usize> = frames
            .iter()
            .copied()
            .filter(|frame| (start..start + 24_000).contains(frame))
            .collect();
        assert!((start..=start + 2 * LATENCY).contains(&sounding[0]));
        let last = sounding[sounding.len() - 1];
        assert!((end..end + RELEASE_FRAMES).contains(&last), "pass {pass}");
    }
    // Stopped in the middle of a note, nothing is left sounding.
    controller.stop();
    let audio = run(&mut processor, 2_000, 128);
    assert!(
        sounding_frames(&audio)
            .iter()
            .all(|frame| *frame < RELEASE_FRAMES)
    );
    assert_eq!(controller.frame().voices, 0);
}

#[test]
fn a_key_holds_one_note_and_ends_with_the_later_of_two() {
    // Two notes on one key: ticks 0 to 960 and 480 to 1440. The key comes
    // up when the second ends, on frame 36000, not when the first would
    // have, on frame 24000.
    let (mut rig, synth) = synth_rig();
    rig.note(synth, 0, 960);
    rig.note(synth, 480, 960);
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let audio = run(&mut processor, 50_000, 128);
    assert!(rms(&left(&audio)[24_500..35_500]) > 0.1);
    let (_, last) = span(&audio);
    assert!((36_000..36_000 + RELEASE_FRAMES).contains(&last), "{last}");
    assert_eq!(controller.frame().voices, 0);
}

#[test]
fn a_crowd_of_overlapping_notes_leaves_none_stuck() {
    // Chords, repeats of one key and notes that run past the loop point,
    // at a tempo that puts many of them in one buffer.
    let (mut rig, synth) = synth_rig();
    rig.project.settings.tempo_bpm = 300.0;
    let pattern = rig.first_pattern();
    rig.pattern_mut(pattern).length_steps = 8;
    for index in 0..120_u32 {
        let note = rig.note(synth, index * 16, 30 + index % 7 * 200);
        note.key = 48 + (index % 13) as u8;
        note.velocity = 0.2 + (index % 5) as f32 * 0.2;
    }
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    // Two passes and a bit of the third.
    let audio = run(&mut processor, 40_000, 100);
    assert!(peak(&audio) > 0.1);
    assert!(controller.frame().voices > 0);

    // Empty the pattern: the notes that are sounding still end on their
    // own ticks, the longest of them 1230 ticks after it began.
    rig.project.patterns[0].lanes.clear();
    controller.set_project(&rig.project, &rig.pool);
    let audio = run(&mut processor, 20_000, 100);
    let (_, last) = span(&audio);
    assert!(
        last < 1_230 * 10 + RELEASE_FRAMES,
        "sound until frame {last}"
    );
    assert_eq!(controller.frame().voices, 0);
    assert!(run(&mut processor, 1_000, 100).iter().all(|s| *s == 0.0));
}

#[test]
fn stopping_silences_an_instrument_with_a_short_fade() {
    let (mut rig, synth) = synth_rig();
    rig.note(synth, 0, 3_000);
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 6_000, 128);
    assert_eq!(controller.frame().voices, 1);
    controller.stop();
    audio.extend(run(&mut processor, 2_000, 128));
    let audio = left(&audio);

    let level = peak(&audio[3_000..6_000]);
    assert!(level > 0.2);
    // Faded over 4 ms, not cut and not left to its release.
    assert!(largest_step(&audio) <= slope_of(level) + level / 150.0);
    assert!(audio[6_000 + RELEASE_FRAMES..].iter().all(|s| *s == 0.0));
    assert!(rms(&audio[6_000..6_100]) > 0.05);
    assert_eq!(controller.frame().voices, 0);

    // Playing again starts the note again.
    controller.play();
    let again = run(&mut processor, 3_000, 128);
    assert!(rms(&left(&again)[1_000..]) > 0.2);
}

#[test]
fn seeking_ends_the_notes_of_the_pattern_but_not_one_held_by_hand() {
    let (mut rig, synth) = synth_rig();
    rig.note(synth, 0, 3_000);
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    run(&mut processor, 3_000, 128);
    controller.note_on(synth, 72, 0.8);
    run(&mut processor, 3_000, 128);
    assert_eq!(controller.frame().voices, 2);

    // The playhead moves past the start of the pattern's note, so that
    // note is over and does not start again. The key held by hand is still
    // down.
    controller.seek(2_000.0);
    run(&mut processor, 3_000, 128);
    assert_eq!(controller.frame().voices, 1);
    let audio = run(&mut processor, 3_000, 128);
    assert!(rms(&left(&audio)) > 0.2);

    controller.note_off(synth, 72);
    let audio = run(&mut processor, 3_000, 128);
    assert!(span(&audio).1 < RELEASE_FRAMES);
    assert_eq!(controller.frame().voices, 0);
}

#[test]
fn a_clip_ends_the_note_it_cuts_short() {
    // The note is a whole bar long, but the clip shows only its first
    // beat, so it ends with the clip on tick 960, frame 24000.
    let (mut rig, synth) = synth_rig();
    rig.note(synth, 0, 3_840);
    let lane = rig.playlist_track();
    let pattern = rig.first_pattern();
    rig.clip(lane, pattern, 0, 960);
    rig.clip(lane, pattern, 7_680, 1);
    let (mut processor, controller) = rig.processor(RATE);
    controller.set_transport(TransportPatch {
        mode: Some(PlayMode::Song),
        ..TransportPatch::default()
    });
    controller.play();
    let audio = run(&mut processor, 60_000, 128);
    let (first, last) = span(&audio);
    assert!(first <= 2 * LATENCY);
    assert!((24_000..24_000 + RELEASE_FRAMES).contains(&last), "{last}");
}

#[test]
fn notes_played_by_hand_sound_without_the_transport() {
    let (rig, synth) = synth_rig();
    let (mut processor, controller) = rig.processor(RATE);
    controller.note_on(synth, 60, 0.9);
    controller.note_on(synth, 64, 0.9);
    let mut audio = run(&mut processor, 4_800, 100);
    assert!(!controller.frame().playing);
    assert_eq!(controller.frame().voices, 2);
    assert!(span(&audio).0 <= 2 * LATENCY);

    controller.note_off(synth, 60);
    audio.extend(run(&mut processor, 4_800, 100));
    assert_eq!(controller.frame().voices, 1);
    assert!(rms(&left(&audio)[9_000..]) > 0.1);
    controller.note_off(synth, 64);
    let after = run(&mut processor, 4_800, 100);
    assert!(span(&after).1 < RELEASE_FRAMES);
    assert_eq!(controller.frame().voices, 0);

    // A key that is not down, and a channel that does not exist, are
    // nothing to let go of.
    controller.note_off(synth, 99);
    controller.note_off(ChannelId(4_000), 60);
    controller.note_on(ChannelId(4_000), 60, 1.0);
    assert!(run(&mut processor, 256, 100).iter().all(|s| *s == 0.0));
}

#[test]
fn the_channels_volume_pan_mute_and_solo_shape_an_instrument() {
    let build = || {
        let (mut rig, synth) = synth_rig();
        rig.note(synth, 0, 480);
        (rig, synth)
    };
    let (rig, _) = build();
    let full = rig.play(RATE, 14_000, 128);
    assert!(rms(&left(&full)) > 0.1);
    assert_eq!(left(&full), right(&full));

    let (mut rig, synth) = build();
    rig.channel_mut(synth).volume = 0.5;
    let half = rig.play(RATE, 14_000, 128);
    assert!(
        half.iter()
            .zip(&full)
            .all(|(half, full)| *half == full * 0.5)
    );

    // Pan uses the mixer's balance law: the far side goes down.
    rig.channel_mut(synth).volume = 1.0;
    rig.channel_mut(synth).pan = -0.5;
    let panned = rig.play(RATE, 14_000, 128);
    assert_eq!(left(&panned), left(&full));
    let halved: Vec<f32> = right(&full).iter().map(|sample| sample * 0.5).collect();
    assert_eq!(right(&panned), halved);

    rig.channel_mut(synth).pan = 0.0;
    rig.channel_mut(synth).muted = true;
    assert!(rig.play(RATE, 14_000, 128).iter().all(|s| *s == 0.0));
    rig.channel_mut(synth).muted = false;
    let other = rig.channel(level(RATE, 0.0, 0.1));
    rig.channel_mut(other).solo = true;
    assert!(rig.play(RATE, 14_000, 128).iter().all(|s| *s == 0.0));
    rig.channel_mut(synth).solo = true;
    assert!(rig.play(RATE, 14_000, 128) == full);
}

#[test]
fn a_fader_move_glides_under_a_sounding_instrument() {
    let (mut rig, synth) = synth_rig();
    rig.note(synth, 0, 3_000);
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 6_000, 128);
    rig.channel_mut(synth).volume = 0.25;
    rig.channel_mut(synth).pan = 0.5;
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 6_000, 128));
    let (left, right) = (left(&audio), right(&audio));

    let level = peak(&left[3_000..6_000]);
    // 5 ms from full level to a quarter, and on the left to an eighth.
    let glide = level * 0.875 / 240.0;
    assert!(largest_step(&left) <= slope_of(level) + glide * 1.05);
    assert!((peak(&left[7_000..]) - level * 0.125).abs() < 2e-3);
    assert!((peak(&right[7_000..]) - level * 0.25).abs() < 2e-3);
    assert_eq!(controller.frame().voices, 1);
}

#[test]
fn a_notes_velocity_and_pan_reach_the_instrument() {
    let play = |velocity: f32, pan: f32| {
        let (mut rig, synth) = synth_rig();
        rig.synth_mut(synth).amp_velocity = 1.0;
        let note = rig.note(synth, 0, 480);
        note.velocity = velocity;
        note.pan = pan;
        rig.play(RATE, 14_000, 128)
    };
    let loud = play(1.0, 0.0);
    let soft = play(0.5, 0.0);
    assert!(peak(&soft) < 0.8 * peak(&loud));
    assert!(peak(&soft) > 0.05);
    // A note with no velocity is silent.
    assert!(play(0.0, 0.0).iter().all(|s| *s == 0.0));
    // Per-note expression pans the synth voice independently of track pan.
    let panned = play(1.0, -1.0);
    assert_eq!(left(&panned), left(&loud));
    assert!(right(&panned).iter().all(|sample| *sample == 0.0));
    assert_eq!(left(&loud), right(&loud));
}

#[test]
fn an_edit_elsewhere_leaves_a_sounding_instrument_exactly_as_it_was() {
    let (mut rig, synth) = synth_rig();
    rig.note(synth, 0, 1_200);
    rig.note(synth, 2_000, 480).key = 67;
    let clicks = rig.channel(level(RATE, 0.0, 0.01));
    rig.steps(clicks, &[2]);
    let untouched = rig.play(RATE, 70_000, 128);

    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 10_000, 128);
    // A step, another channel's fader, a second synth on a new track, and
    // a note added later in the synth's own lane.
    rig.steps(clicks, &[5]);
    rig.channel_mut(clicks).volume = 0.5;
    let track = rig.track();
    rig.synth_on(plain_synth(), track);
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 10_000, 128));
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 50_000, 128));
    assert!(
        audio == untouched,
        "the edit disturbed a sounding instrument"
    );
}

#[test]
fn changing_a_setting_does_not_restart_the_instrument() {
    let (mut rig, synth) = synth_rig();
    rig.note(synth, 0, 3_000);
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 6_000, 128);
    rig.synth_mut(synth).gain = 0.25;
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 6_000, 128));
    let audio = left(&audio);

    let level = peak(&audio[3_000..6_000]);
    // The note carries on and settles at half its level. A synth built
    // anew would have gone silent: the note began long ago.
    assert!((peak(&audio[10_000..]) - level * 0.5).abs() < 2e-3);
    assert!(
        audio[6_000..]
            .chunks(200)
            .all(|piece| peak(piece) > level * 0.4)
    );
    assert!(largest_step(&audio) <= slope_of(level) * 1.05 + level / 200.0);
    assert_eq!(controller.frame().voices, 1);
}

#[test]
fn removing_an_instrument_channel_fades_it_out() {
    let (mut rig, synth) = synth_rig();
    rig.note(synth, 0, 3_000);
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 6_000, 128);
    rig.project.channels.clear();
    rig.project.patterns[0].lanes.clear();
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 1_000, 128));
    // The next plan lets go of the instrument for good.
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 1_000, 128));
    let audio = left(&audio);

    let level = peak(&audio[3_000..6_000]);
    assert!(largest_step(&audio) <= slope_of(level) + level / 150.0);
    assert!(rms(&audio[6_000..6_100]) > 0.05);
    assert!(audio[6_000 + RELEASE_FRAMES..].iter().all(|s| *s == 0.0));
    assert_eq!(controller.frame().voices, 0);
}

#[test]
fn a_sampler_on_the_same_track_waits_for_the_synth() {
    // The synth is 12 frames behind, so a click beside it is delayed by as
    // much, and the two stay lined up.
    let (mut rig, _) = synth_rig();
    let click = rig.channel(impulse(RATE));
    rig.steps(click, &[1]);
    let audio = rig.play(RATE, 12_000, 128);
    assert_eq!(sounding_frames(&audio), [6_000 + LATENCY]);
    assert_eq!(left(&audio)[6_000 + LATENCY], 1.0);
}

#[test]
fn an_instrument_runs_through_its_tracks_effects() {
    let mut rig = Rig::new();
    let track = rig.track();
    let synth = rig.synth_on(plain_synth(), track);
    rig.note(synth, 0, 480);
    let dry = rig.play(RATE, 14_000, 128);
    // A flat equaliser changes nothing; muting the track silences it.
    rig.effect(track, EffectParams::Eq(EqParams::default()));
    let equalised = rig.play(RATE, 14_000, 128);
    let apart = dry.iter().zip(&equalised).map(|(a, b)| (a - b).abs());
    assert!(apart.fold(0.0, f32::max) < 1e-4);
    rig.track_mut(track).muted = true;
    assert!(rig.play(RATE, 14_000, 128).iter().all(|s| *s == 0.0));
}
