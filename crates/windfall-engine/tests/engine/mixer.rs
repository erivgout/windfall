//! Mixer tracks: gain, pan, mute, solo, routing, sends, meters, and smooth
//! changes while playing.

use windfall_core::db_to_gain;
use windfall_engine::PREVIEW_GAIN_DB;
use windfall_project::{ChannelId, Send, TrackId};

use crate::support::{Rig, largest_step, left, level, right, run, sine};

const RATE: u32 = 48_000;

/// Frames a gain or pan change takes to arrive: 5 ms.
const RAMP_FRAMES: usize = 240;

/// Frames a voice of a removed channel takes to fade out: 4 ms.
const FADE_FRAMES: usize = 192;

/// Adds a channel on its own mixer track that holds a constant level from
/// the first frame of playback.
fn steady(rig: &mut Rig, value: f32) -> (ChannelId, TrackId) {
    let track = rig.track();
    let channel = rig.channel_on(level(RATE, value, 4.0), track);
    rig.steps(channel, &[0]);
    (channel, track)
}

/// The level on each side of the master once playback has settled.
fn master(rig: &Rig) -> (f32, f32) {
    let audio = rig.play(RATE, 256, 256);
    (audio[510], audio[511])
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-6,
        "got {actual}, expected {expected}"
    );
}

#[test]
fn channel_track_and_master_gains_multiply() {
    let mut rig = Rig::new();
    let (channel, track) = steady(&mut rig, 0.5);
    rig.channel_mut(channel).volume = 0.5;
    rig.track_mut(track).volume = 1.5;
    rig.track_mut(TrackId::MASTER).volume = 0.8;
    let (left, right) = master(&rig);
    assert_close(left, 0.3);
    assert_close(right, 0.3);
}

#[test]
fn pan_attenuates_only_the_far_side() {
    let mut rig = Rig::new();
    let (channel, track) = steady(&mut rig, 0.5);
    rig.channel_mut(channel).pan = -1.0;
    assert_eq!(master(&rig), (0.5, 0.0));

    rig.channel_mut(channel).pan = 0.0;
    rig.track_mut(track).pan = 0.5;
    assert_eq!(master(&rig), (0.25, 0.5));

    rig.track_mut(TrackId::MASTER).pan = -0.5;
    assert_eq!(master(&rig), (0.25, 0.25));
}

#[test]
fn mute_silences_a_channel_or_a_track() {
    let mut rig = Rig::new();
    let (quiet, quiet_track) = steady(&mut rig, 0.25);
    let (_, _) = steady(&mut rig, 0.5);

    rig.channel_mut(quiet).muted = true;
    assert_eq!(master(&rig), (0.5, 0.5));

    rig.channel_mut(quiet).muted = false;
    rig.track_mut(quiet_track).muted = true;
    assert_eq!(master(&rig), (0.5, 0.5));

    rig.track_mut(TrackId::MASTER).muted = true;
    assert_eq!(master(&rig), (0.0, 0.0));
}

#[test]
fn channel_solo_leaves_only_soloed_channels() {
    let mut rig = Rig::new();
    let (first, _) = steady(&mut rig, 0.125);
    let (second, _) = steady(&mut rig, 0.25);
    let (_, _) = steady(&mut rig, 0.5);
    assert_eq!(master(&rig).0, 0.875);

    rig.channel_mut(first).solo = true;
    assert_eq!(master(&rig).0, 0.125);

    rig.channel_mut(second).solo = true;
    assert_eq!(master(&rig).0, 0.375);

    // Mute wins over solo.
    rig.channel_mut(first).muted = true;
    assert_eq!(master(&rig).0, 0.25);
}

#[test]
fn track_solo_keeps_what_feeds_it_and_its_way_to_the_master() {
    let mut rig = Rig::new();
    let (_, source) = steady(&mut rig, 0.125);
    let (_, other) = steady(&mut rig, 0.25);
    let (_, bus) = steady(&mut rig, 0.5);
    rig.track_mut(source).output = Some(bus);
    assert_eq!(master(&rig).0, 0.875);

    // The bus carries the soloed track to the master, and keeps passing
    // what plays straight into it.
    rig.track_mut(source).solo = true;
    assert_eq!(master(&rig).0, 0.625);
    rig.track_mut(source).solo = false;

    // Soloing the bus keeps the track that feeds it.
    rig.track_mut(bus).solo = true;
    assert_eq!(master(&rig).0, 0.625);
    rig.track_mut(bus).solo = false;

    rig.track_mut(other).solo = true;
    assert_eq!(master(&rig).0, 0.25);

    // Mute wins over solo.
    rig.track_mut(other).muted = true;
    assert_eq!(master(&rig).0, 0.0);
}

#[test]
fn the_output_routes_a_track_into_another() {
    let mut rig = Rig::new();
    let (_, source) = steady(&mut rig, 0.5);
    let bus = rig.track();
    rig.track_mut(source).output = Some(bus);
    rig.track_mut(bus).volume = 0.5;
    rig.track_mut(bus).pan = 1.0;
    assert_eq!(master(&rig), (0.0, 0.25));

    // A track with no output reaches nothing, but still shows on its meter.
    rig.track_mut(bus).output = None;
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let audio = run(&mut processor, 256, 256);
    assert!(audio.iter().all(|sample| *sample == 0.0));
    // Master, then the source track, then the bus.
    assert_eq!(
        controller.frame().meters,
        vec![0.0, 0.0, 0.5, 0.5, 0.0, 0.25]
    );
}

#[test]
fn sends_copy_the_post_fader_signal() {
    let mut rig = Rig::new();
    let (_, source) = steady(&mut rig, 0.5);
    let effects = rig.track();
    rig.track_mut(source).volume = 0.5;
    rig.track_mut(source).sends.push(Send {
        target: effects,
        gain: 0.5,
    });
    // 0.25 directly, plus half of that through the other track.
    assert_eq!(master(&rig), (0.375, 0.375));

    // With no output, the track is heard only through its send.
    rig.track_mut(source).output = None;
    assert_eq!(master(&rig), (0.125, 0.125));
}

#[test]
fn meters_report_each_tracks_peak_and_reset_when_read() {
    let mut rig = Rig::new();
    let (channel, track) = steady(&mut rig, 0.5);
    rig.channel_mut(channel).pan = 0.5;
    rig.track_mut(track).volume = 0.5;
    let (_, _) = steady(&mut rig, -0.25);
    rig.track_mut(TrackId::MASTER).volume = 2.0;

    let (mut processor, controller) = rig.processor(RATE);
    assert_eq!(controller.frame().meters, vec![0.0; 6]);
    controller.play();
    run(&mut processor, 1_000, 100);
    let meters = controller.frame().meters;
    // The master: (0.125 - 0.25) * 2 on the left, (0.25 - 0.25) * 2 on the
    // right. Then the two tracks, after their faders.
    assert_eq!(meters, vec![0.25, 0.0, 0.125, 0.25, 0.25, 0.25]);

    // Nothing was processed since, so the peaks are gone.
    assert_eq!(controller.frame().meters, vec![0.0; 6]);
    run(&mut processor, 100, 100);
    assert_eq!(controller.frame().meters[2..4], [0.125, 0.25]);
}

#[test]
fn a_fader_move_glides_instead_of_stepping() {
    let mut rig = Rig::new();
    let (channel, track) = steady(&mut rig, 0.5);
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 1_000, 128);

    rig.channel_mut(channel).volume = 0.2;
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 1_000, 128));
    let signal = left(&audio);
    assert_eq!(signal[999], 0.5);
    assert_close(signal[1_000 + RAMP_FRAMES / 2], 0.3);
    assert_close(signal[1_000 + RAMP_FRAMES], 0.1);
    assert!(largest_step(&signal[1..]) <= 0.4 / RAMP_FRAMES as f32 * 1.01);

    // Muting a track is a glide to silence too.
    rig.track_mut(track).muted = true;
    controller.set_project(&rig.project, &rig.pool);
    let signal = left(&run(&mut processor, 1_000, 128));
    assert_close(signal[0], 0.1);
    assert_eq!(signal[RAMP_FRAMES], 0.0);
    assert!(largest_step(&signal) <= 0.1 / RAMP_FRAMES as f32 * 1.01);
}

#[test]
fn pan_and_send_changes_glide_too() {
    let mut rig = Rig::new();
    let (channel, track) = steady(&mut rig, 0.5);
    let effects = rig.track();
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 500, 128);

    rig.channel_mut(channel).pan = 1.0;
    rig.track_mut(track).sends.push(Send {
        target: effects,
        gain: 1.0,
    });
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 500, 128));
    let (left, right) = (left(&audio), right(&audio));
    // The left side fades away while the new send fades in on the right.
    assert_eq!((left[499], right[499]), (0.5, 0.5));
    assert_eq!((left[999], right[999]), (0.0, 1.0));
    // Two glides at once never move the signal by more than its full
    // level over the length of one glide.
    assert!(largest_step(&left[1..]) <= 1.0 / RAMP_FRAMES as f32 * 1.01);
    assert!(largest_step(&right[1..]) <= 1.0 / RAMP_FRAMES as f32 * 1.01);
}

#[test]
fn the_output_gain_comes_after_the_master_meter() {
    let mut rig = Rig::new();
    steady(&mut rig, 0.5);
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 500, 128);
    controller.set_output_gain(0.0);
    audio.extend(run(&mut processor, 500, 128));
    let signal = left(&audio);
    assert_eq!(signal[499], 0.5);
    assert!(
        signal[500 + RAMP_FRAMES..]
            .iter()
            .all(|sample| *sample == 0.0)
    );
    assert!(largest_step(&signal[1..]) <= 0.5 / RAMP_FRAMES as f32 * 1.01);

    // Nothing is heard, yet the engine still mixes and meters.
    controller.frame();
    run(&mut processor, 500, 128);
    assert_eq!(controller.frame().meters[..2], [0.5, 0.5]);
}

/// A project where a 200 Hz sine plays for three seconds next to a channel
/// of clicks. The sine moves at most `SINE_SLOPE` from one frame to the next.
fn sine_rig() -> (Rig, ChannelId, ChannelId) {
    let mut rig = Rig::new();
    let tone = rig.channel(sine(RATE, 200.0, 3.0));
    rig.steps(tone, &[0]);
    let clicks = rig.channel(level(RATE, 0.0, 0.01));
    rig.steps(clicks, &[2, 6]);
    (rig, tone, clicks)
}

const SINE_SLOPE: f32 = 0.5 * 200.0 * std::f32::consts::TAU / RATE as f32;

#[test]
fn a_plan_swap_leaves_sounding_voices_exactly_as_they_were() {
    let (mut rig, _, clicks) = sine_rig();
    let untouched = rig.play(RATE, 20_000, 128);

    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 5_000, 128);
    // Edits that do not concern the sine: toggled steps, another channel's
    // fader, a new channel and a new mixer track.
    rig.steps(clicks, &[1, 3]);
    rig.channel_mut(clicks).volume = 0.3;
    let track = rig.track();
    rig.channel_on(level(RATE, 0.0, 0.01), track);
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 15_000, 128));

    assert!(audio == untouched, "the swap disturbed a sounding voice");
    assert_eq!(controller.frame().voices, 1);
}

#[test]
fn a_plan_swap_that_changes_a_sounding_channel_does_not_click() {
    let (mut rig, tone, _) = sine_rig();
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 5_000, 128);

    rig.channel_mut(tone).volume = 0.6;
    rig.channel_mut(tone).pan = -0.4;
    rig.project.settings.tempo_bpm = 97.0;
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 5_000, 128));

    // The sine never moves faster than its own slope plus the glide of
    // volume and pan together, which at its steepest covers 0.8 of the
    // sine's level over the length of one glide.
    let glide = 0.5 * 0.8 / RAMP_FRAMES as f32;
    assert!(largest_step(&left(&audio)[1..]) <= SINE_SLOPE + glide);
    assert!(largest_step(&right(&audio)[1..]) <= SINE_SLOPE + glide);
    // And it is still sounding, at its new level.
    let after = &left(&audio)[6_000..];
    assert_close(
        after
            .iter()
            .fold(0.0, |peak, sample| peak.max(sample.abs())),
        0.3,
    );
    assert_eq!(controller.frame().voices, 1);
}

#[test]
fn removing_a_channel_fades_its_voices_out() {
    let (mut rig, tone, _) = sine_rig();
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 5_000, 128);
    assert_eq!(controller.frame().voices, 1);

    rig.project.channels.retain(|channel| channel.id != tone);
    for pattern in &mut rig.project.patterns {
        pattern.lanes.retain(|lane| lane.channel != tone);
    }
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 1_000, 128));

    let signal = left(&audio);
    assert!(
        signal[5_000 + FADE_FRAMES..]
            .iter()
            .all(|sample| *sample == 0.0)
    );
    assert!(signal[5_000 + FADE_FRAMES / 2].abs() > 0.0);
    assert!(largest_step(&signal[1..]) <= SINE_SLOPE + 0.5 / FADE_FRAMES as f32);
    assert_eq!(controller.frame().voices, 0);
}

#[test]
fn removing_a_mixer_track_sends_its_channels_to_the_master() {
    let mut rig = Rig::new();
    let (_, track) = steady(&mut rig, 0.5);
    rig.track_mut(track).volume = 0.5;
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 500, 128);

    rig.project.mixer.tracks.retain(|entry| entry.id != track);
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 500, 128));
    let signal = left(&audio);
    assert_eq!(signal[499], 0.25);
    assert_eq!(signal[999], 0.5);
    assert_eq!(controller.frame().meters.len(), 2);
}

/// Plays `rig` from the top on a processor that has done nothing else, and
/// returns the level of each side, which must be the same on every frame
/// from the very first.
fn constant_from_the_first_frame(rig: &Rig) -> (f32, f32) {
    let audio = rig.play(RATE, 1_000, 64);
    let frames = audio.as_chunks::<2>().0;
    assert!(
        frames.iter().all(|frame| *frame == frames[0]),
        "the level moved: it started at {:?} and came to {:?}",
        frames[0],
        frames[999]
    );
    (frames[0][0], frames[0][1])
}

/// A project that plays a constant full-scale level with a different gain
/// and pan on its channel, its track and the master. It comes out at
/// 0.0625 on both sides: three faders at a half, the channel's pan halving
/// the left and the master's pan halving the right.
fn leveled_rig() -> Rig {
    let mut rig = Rig::new();
    let (channel, track) = steady(&mut rig, 1.0);
    rig.channel_mut(channel).volume = 0.5;
    rig.channel_mut(channel).pan = 0.5;
    rig.track_mut(track).volume = 0.5;
    rig.track_mut(TrackId::MASTER).volume = 0.5;
    rig.track_mut(TrackId::MASTER).pan = -0.5;
    rig
}

#[test]
fn the_first_project_plays_at_its_own_levels_from_the_first_frame() {
    assert_eq!(
        constant_from_the_first_frame(&leveled_rig()),
        (0.0625, 0.0625)
    );
}

#[test]
fn mute_and_solo_of_the_first_project_hold_from_the_first_frame() {
    let mut rig = Rig::new();
    let (quiet, _) = steady(&mut rig, 0.25);
    let (_, loud_track) = steady(&mut rig, 0.5);
    assert_eq!(constant_from_the_first_frame(&rig), (0.75, 0.75));

    rig.channel_mut(quiet).muted = true;
    assert_eq!(constant_from_the_first_frame(&rig), (0.5, 0.5));
    rig.channel_mut(quiet).muted = false;

    rig.channel_mut(quiet).solo = true;
    assert_eq!(constant_from_the_first_frame(&rig), (0.25, 0.25));
    rig.channel_mut(quiet).solo = false;

    rig.track_mut(loud_track).muted = true;
    assert_eq!(constant_from_the_first_frame(&rig), (0.25, 0.25));
    rig.track_mut(loud_track).muted = false;

    rig.track_mut(loud_track).solo = true;
    assert_eq!(constant_from_the_first_frame(&rig), (0.5, 0.5));
    rig.track_mut(loud_track).solo = false;

    rig.track_mut(TrackId::MASTER).muted = true;
    assert_eq!(constant_from_the_first_frame(&rig), (0.0, 0.0));
}

#[test]
fn a_project_that_replaces_another_plays_at_its_own_levels_from_the_first_frame() {
    let mut first = Rig::new();
    let (channel, _) = steady(&mut first, 0.5);
    let (mut processor, controller) = first.processor(RATE);
    controller.play();
    run(&mut processor, 1_000, 128);

    // The next project is built the same way, so its channel, its track
    // and its master carry the very ids the first one used.
    let second = leveled_rig();
    assert_eq!(second.project.channels[0].id, channel);

    // Stop, load and play arrive together, as when a project is opened and
    // started by one action. The old note is still fading out.
    controller.stop();
    controller.set_project(&second.project, &second.pool);
    controller.play();
    let audio = run(&mut processor, 1_000, 64);
    let frames = audio.as_chunks::<2>().0;
    for (frame, sides) in frames.iter().enumerate() {
        // What is left of the first project's note: 0.5, down a straight
        // line over the fade, at the levels of its own project.
        let tail = 0.5 * FADE_FRAMES.saturating_sub(frame) as f32 / FADE_FRAMES as f32;
        for side in sides {
            assert!(
                (side - (0.0625 + tail)).abs() < 1e-6,
                "frame {frame} came out at {side}"
            );
        }
    }
    assert_eq!(frames[999], [0.0625, 0.0625]);
    assert_eq!(controller.frame().voices, 1);
}

#[test]
fn a_fader_nothing_is_heard_through_changes_at_once() {
    let mut rig = Rig::new();
    let (_, _) = steady(&mut rig, 0.5);
    // A second channel whose only note starts on step 1, frame 6000.
    let track = rig.track();
    let late = rig.channel_on(level(RATE, 0.25, 1.0), track);
    rig.steps(late, &[1]);
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 5_990, 128);

    // Ten frames before the note, its channel, its track and one of its
    // track's sends are set. None of them has anything to glide from.
    let effects = rig.track();
    rig.channel_mut(late).volume = 0.5;
    rig.track_mut(track).volume = 0.5;
    rig.track_mut(track).sends.push(Send {
        target: effects,
        gain: 1.0,
    });
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 1_000, 128));
    let signal = left(&audio);
    assert!(signal[..6_000].iter().all(|sample| *sample == 0.5));
    // 0.25 through two faders at a half, once directly and once more
    // through the send.
    assert!(signal[6_000..].iter().all(|sample| *sample == 0.5 + 0.125));
}

#[test]
fn a_preview_heard_through_the_master_makes_the_first_project_glide_in() {
    // The exception that shows the rule: something is sounding through the
    // master when the first project arrives, so the master fader glides.
    let (mut processor, controller) = windfall_engine::Processor::new(RATE);
    controller.preview(level(RATE, 1.0, 1.0));
    let mut audio = run(&mut processor, 500, 128);

    let mut rig = Rig::new();
    rig.track_mut(TrackId::MASTER).volume = 0.5;
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 500, 128));
    let signal = left(&audio);
    let preview = db_to_gain(PREVIEW_GAIN_DB);
    assert_close(signal[499], preview);
    assert_close(signal[500 + RAMP_FRAMES / 2], preview * 0.75);
    assert_close(signal[999], preview * 0.5);
    assert!(largest_step(&signal[1..]) <= preview * 0.5 / RAMP_FRAMES as f32 * 1.01);
}

#[test]
fn a_voice_that_loses_its_channel_fades_out_from_exactly_where_it_was() {
    // The voice is heard through a panned channel, a track with its own
    // fader, a send into a second track, and the master.
    let mut rig = Rig::new();
    let (channel, track) = steady(&mut rig, 0.5);
    let effects = rig.track();
    rig.channel_mut(channel).pan = -0.5;
    rig.track_mut(track).volume = 0.5;
    rig.track_mut(track).sends.push(Send {
        target: effects,
        gain: 0.5,
    });
    rig.track_mut(effects).pan = 1.0;
    rig.track_mut(TrackId::MASTER).volume = 0.8;
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let before = run(&mut processor, 256, 64);
    let heard = (before[510], before[511]);
    // The left side is 0.25 directly; the send, panned hard right, adds
    // nothing to it. The right side is 0.125 and half as much again.
    assert_close(heard.0, 0.2);
    assert_close(heard.1, 0.15);

    // The channel goes, and with it everything else changes too.
    rig.project.channels.clear();
    rig.project.mixer.tracks.truncate(1);
    rig.track_mut(TrackId::MASTER).volume = 0.1;
    controller.set_project(&rig.project, &rig.pool);
    let after = run(&mut processor, 256, 64);
    for (frame, sides) in after.as_chunks::<2>().0.iter().enumerate() {
        let left_of_fade = FADE_FRAMES.saturating_sub(frame) as f32 / FADE_FRAMES as f32;
        assert_close(sides[0], heard.0 * left_of_fade);
        assert_close(sides[1], heard.1 * left_of_fade);
    }
}
