//! Effects on mixer tracks: where they sit in the signal, what they keep
//! across edits, how they come and go, and how paths of different latency
//! stay lined up.

use windfall_core::db_to_gain;
use windfall_dsp::{
    CompressorParams, DelayParams, EqParams, LimiterParams, NoteDivision, ReverbParams,
};
use windfall_project::{ChannelId, EffectId, EffectParams, Send, TrackId};

use crate::support::{
    Rig, idle_limiter, impulse, inverted, largest_step, left, level, limiter, peak, plain_synth,
    right, rms, run, sine, sounding_frames,
};

const RATE: u32 = 48_000;

/// Frames an effect takes to fade into or out of a chain: 5 ms.
const SPLICE_FRAMES: usize = 240;

/// Frames an effect takes to be switched off or on, or to follow its mix:
/// 10 ms.
const SWITCH_FRAMES: usize = 480;

/// Frames a voice takes to fade out, or in, when a plan change moves it:
/// 4 ms.
const FADE_FRAMES: usize = 192;

/// The pitch of the test tone. Its period is no whole number of frames, so
/// the tone never looks the same after a delay as before it.
const TONE_HZ: f64 = 173.0;

/// The most that tone moves from one frame to the next at half scale.
const SINE_SLOPE: f32 = 0.5 * TONE_HZ as f32 * std::f32::consts::TAU / RATE as f32;

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-4,
        "got {actual}, expected {expected}"
    );
}

/// A channel on a track of its own that holds a constant level from the
/// first frame of playback.
fn steady(rig: &mut Rig, value: f32) -> (ChannelId, TrackId) {
    let track = rig.track();
    let channel = rig.channel_on(level(RATE, value, 4.0), track);
    rig.steps(channel, &[0]);
    (channel, track)
}

/// A reverb that rings for a good while, with none of the dry signal in it.
fn long_reverb() -> EffectParams {
    EffectParams::Reverb(ReverbParams {
        decay_s: 3.0,
        mix: 1.0,
        ..ReverbParams::default()
    })
}

/// A project where one click on the first frame rings on in a reverb, and
/// a second channel plays silence.
fn ringing_rig() -> (Rig, TrackId, ChannelId, EffectId) {
    let mut rig = Rig::new();
    let track = rig.track();
    let click = rig.channel_on(impulse(RATE), track);
    rig.steps(click, &[0]);
    let reverb = rig.effect(track, long_reverb());
    let other = rig.channel(level(RATE, 0.0, 0.01));
    rig.steps(other, &[2, 6]);
    (rig, track, other, reverb)
}

#[test]
fn a_tracks_effects_come_before_its_fader() {
    // A constant full-scale level, through a limiter that holds it to a
    // half, on a track whose fader is at a half.
    let mut rig = Rig::new();
    let (_, track) = steady(&mut rig, 1.0);
    rig.effect(track, limiter(-6.0206));
    rig.track_mut(track).volume = 0.5;
    let audio = left(&rig.play(RATE, 9_600, 128));
    // Were the fader first, the limiter would have nothing to do and a
    // half would come out.
    assert_close(audio[9_000], 0.25);

    // The meter is after both.
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    run(&mut processor, 9_600, 128);
    controller.frame();
    run(&mut processor, 256, 128);
    let meters = controller.frame().meters;
    assert_close(meters[2], 0.25);
    assert_close(meters[0], 0.25);
}

#[test]
fn effects_run_in_the_order_of_the_chain() {
    let level_after = |boost_first: bool| {
        let mut rig = Rig::new();
        let (_, track) = steady(&mut rig, 0.2);
        let hold = limiter(-12.0);
        let boost = EffectParams::Limiter(LimiterParams {
            ceiling_db: 0.0,
            input_gain_db: 12.0,
            ..LimiterParams::default()
        });
        let chain = if boost_first {
            [boost, hold]
        } else {
            [hold, boost]
        };
        for effect in chain {
            rig.effect(track, effect);
        }
        left(&rig.play(RATE, 9_600, 128))[9_000]
    };
    // Under the ceiling it passes the first limiter untouched and is then
    // made 12 dB louder.
    assert_close(level_after(false), 0.2 * db_to_gain(12.0));
    // Made louder first, it is over the ceiling and is held to it.
    assert_close(level_after(true), db_to_gain(-12.0));
}

#[test]
fn the_master_has_effects_like_any_other_track() {
    let mut rig = Rig::new();
    steady(&mut rig, 0.5);
    steady(&mut rig, 0.5);
    rig.effect(TrackId::MASTER, limiter(-12.0));
    let audio = rig.play(RATE, 9_600, 128);
    assert_close(left(&audio)[9_000], db_to_gain(-12.0));
    assert_close(right(&audio)[9_000], db_to_gain(-12.0));
}

#[test]
fn an_edit_elsewhere_leaves_a_ringing_effect_exactly_as_it_was() {
    let (mut rig, _, other, _) = ringing_rig();
    let untouched = rig.play(RATE, 30_000, 128);
    assert!(rms(&left(&untouched)[20_000..]) > 1e-4, "the reverb rings");

    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 10_000, 128);
    // Edits that do not concern the reverb or what plays into it: toggled
    // steps, another channel's fader, a new channel on a new track with an
    // effect of its own, and an effect that is switched off on the master.
    rig.steps(other, &[1, 3]);
    rig.channel_mut(other).volume = 0.3;
    let track = rig.track();
    rig.channel_on(level(RATE, 0.0, 0.01), track);
    rig.effect(track, EffectParams::Eq(EqParams::default()));
    let bypassed = rig.effect(TrackId::MASTER, long_reverb());
    rig.effect_mut(bypassed).enabled = false;
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 10_000, 128));
    // And again, with nothing changed at all.
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 10_000, 128));

    assert!(audio == untouched, "the edit disturbed a ringing effect");
}

#[test]
fn changing_a_setting_keeps_what_the_effect_remembers() {
    let (mut rig, _, _, reverb) = ringing_rig();
    let untouched = left(&rig.play(RATE, 40_000, 128));

    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 10_000, 128);
    let EffectParams::Reverb(params) = &mut rig.effect_mut(reverb).params else {
        panic!("not a reverb");
    };
    params.decay_s = 0.3;
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 30_000, 128));
    let audio = left(&audio);

    // The tail carries on from where it was: a reverb built anew would be
    // silent, since the click that set it ringing is long over.
    assert_eq!(audio[..10_000], untouched[..10_000]);
    let just_after = 10_000..12_000;
    assert!(rms(&audio[just_after.clone()]) > 0.5 * rms(&untouched[just_after]));
    // And it now dies away as fast as the new setting says.
    let later = 30_000..40_000;
    assert!(rms(&audio[later.clone()]) < 0.05 * rms(&untouched[later]));
}

#[test]
fn an_effect_moved_to_another_track_keeps_ringing_there() {
    let (mut rig, _, _, reverb) = ringing_rig();
    let untouched = left(&rig.play(RATE, 30_000, 128));

    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 10_000, 128);
    // The same effect, id and all, now sits on the master.
    let slot = rig.remove_effect(reverb);
    rig.track_mut(TrackId::MASTER).effects.push(slot);
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 20_000, 128));
    let audio = left(&audio);

    // It fades in at its new place over 5 ms, and from then on its tail
    // is the very one it had: with every fader at unity the reverb sounds
    // the same on either track.
    for (offset, frame) in (10_000..10_000 + SPLICE_FRAMES).enumerate() {
        let faded = untouched[frame] * offset as f32 / SPLICE_FRAMES as f32;
        assert!((audio[frame] - faded).abs() < 1e-6, "frame {frame}");
    }
    let after = 10_000 + SPLICE_FRAMES;
    assert_eq!(audio[after..], untouched[after..]);
    assert!(rms(&audio[after..]) > 1e-4);
}

#[test]
fn a_removed_effect_fades_out_over_five_milliseconds() {
    let (mut rig, _, _, reverb) = ringing_rig();
    let untouched = left(&rig.play(RATE, 12_000, 128));

    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 10_000, 128);
    rig.remove_effect(reverb);
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 1_000, 128));
    // The next plan lets go of the effect for good.
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 1_000, 128));
    let audio = left(&audio);

    for (offset, frame) in (10_000..10_000 + SPLICE_FRAMES).enumerate() {
        let left_of_fade = (SPLICE_FRAMES - offset) as f32 / SPLICE_FRAMES as f32;
        let faded = untouched[frame] * left_of_fade;
        assert!((audio[frame] - faded).abs() < 1e-6, "frame {frame}");
    }
    assert!(
        audio[10_000 + SPLICE_FRAMES..]
            .iter()
            .all(|sample| *sample == 0.0)
    );
}

#[test]
fn an_effect_removed_from_a_silent_track_is_gone_at_once() {
    // Nothing has sounded through the track, so there is nothing to fade.
    let mut rig = Rig::new();
    let track = rig.track();
    let late = rig.channel_on(level(RATE, 1.0, 1.0), track);
    rig.steps(late, &[1]);
    let effect = rig.effect(track, limiter(-12.0));
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 5_990, 128);
    rig.remove_effect(effect);
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 1_000, 128));
    let audio = left(&audio);
    // The note on frame 6000 comes out untouched from its first frame.
    assert!(audio[..6_000].iter().all(|sample| *sample == 0.0));
    assert!(audio[6_000..].iter().all(|sample| *sample == 1.0));
}

/// A project where the test tone plays for three seconds on a track of
/// its own.
fn tone_rig() -> (Rig, ChannelId, TrackId) {
    let mut rig = Rig::new();
    let track = rig.track();
    let tone = rig.channel_on(sine(RATE, TONE_HZ, 3.0), track);
    rig.steps(tone, &[0]);
    (rig, tone, track)
}

#[test]
fn an_effect_added_under_sound_fades_in_without_a_click() {
    let (mut rig, _, track) = tone_rig();
    let dry = left(&rig.play(RATE, 12_000, 128));

    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 5_000, 128);
    assert_eq!(controller.latency_frames(), 0);
    rig.effect(track, idle_limiter(5.0));
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 7_000, 128));
    assert_eq!(controller.latency_frames(), 240);
    let audio = left(&audio);

    // Untouched while the limiter's look-ahead fills, which takes 240
    // frames, and from the end of the fade after that the same sine 240
    // frames late.
    assert_eq!(audio[..5_240], dry[..5_240]);
    let after = 5_240 + SPLICE_FRAMES;
    for frame in after..12_000 {
        assert!(
            (audio[frame] - dry[frame - 240]).abs() < 1e-6,
            "frame {frame}"
        );
    }
    // In between, a crossfade from the one to the other. Put in at once,
    // the limiter would open with 240 frames of silence, a jump of up to
    // half of full scale at either end.
    let fade = SINE_SLOPE + 1.0 / SPLICE_FRAMES as f32;
    assert!(largest_step(&audio[1..]) <= fade);
    assert!(rms(&audio[5_240..after]) > 0.1);
}

#[test]
fn an_effect_on_a_track_nothing_sounds_through_is_there_from_its_first_frame() {
    // The limiter arrives ten frames before the first note of its track,
    // with nothing to fade in from.
    let mut rig = Rig::new();
    let track = rig.track();
    let late = rig.channel_on(level(RATE, 1.0, 1.0), track);
    rig.steps(late, &[1]);
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 5_990, 128);
    rig.effect(track, limiter(-12.0));
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 3_000, 128));
    let audio = left(&audio);
    // Nothing until the note comes out of the look-ahead, and never more
    // than the ceiling after.
    assert!(audio[..6_240].iter().all(|sample| *sample == 0.0));
    assert!(audio[6_240] > 0.2);
    assert!(peak(&audio) <= db_to_gain(-12.0) + 1e-6);
}

#[test]
fn switching_an_effect_off_and_mixing_it_crossfade() {
    let mut rig = Rig::new();
    let (_, track) = steady(&mut rig, 1.0);
    let effect = rig.effect(track, limiter(-12.0));
    let held = db_to_gain(-12.0);
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 9_600, 128);

    rig.effect_mut(effect).enabled = false;
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 2_400, 128));
    rig.effect_mut(effect).enabled = true;
    rig.effect_mut(effect).mix = 0.5;
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 2_400, 128));
    let audio = left(&audio);

    assert_close(audio[9_599], held);
    // Off, the signal passes untouched, still delayed by the look-ahead so
    // nothing downstream has to move.
    assert_close(audio[11_999], 1.0);
    assert_eq!(controller.latency_frames(), 240);
    // Half mixed, half of each.
    assert_close(audio[14_399], 0.5 + 0.5 * held);
    // Each change takes 10 ms in a straight line.
    let step = (1.0 - held) / SWITCH_FRAMES as f32;
    assert!(largest_step(&audio[9_000..]) <= step * 1.01);
}

#[test]
fn a_limiter_on_one_track_does_not_put_it_behind_the_others() {
    // Two clicks on the same tick, one through a limiter that looks 5 ms
    // ahead and one through nothing at all.
    let mut rig = Rig::new();
    let (slow, fast) = (rig.track(), rig.track());
    let behind = rig.channel_on(impulse(RATE), slow);
    let ahead = rig.channel_on(impulse(RATE), fast);
    rig.channel_mut(behind).volume = 0.25;
    rig.channel_mut(ahead).volume = 0.5;
    rig.steps(behind, &[1]);
    rig.steps(ahead, &[1]);
    rig.effect(slow, idle_limiter(5.0));

    let (mut processor, controller) = rig.processor(RATE);
    assert_eq!(controller.latency_frames(), 240);
    controller.play();
    let audio = run(&mut processor, 12_000, 128);
    // Step 1 is frame 6000. Both clicks come out together, 240 frames on.
    assert_eq!(sounding_frames(&audio), [6_240]);
    assert_eq!(left(&audio)[6_240], 0.75);
}

#[test]
fn sends_nested_tracks_and_the_master_all_stay_lined_up() {
    // One click goes through a track with a 5 ms limiter into a bus with a
    // 2 ms limiter. A second goes through a plain track that plays into
    // the master and sends to the bus. A third plays straight into the
    // master, which has a 1 ms limiter of its own.
    let mut rig = Rig::new();
    let (slow, plain, bus) = (rig.track(), rig.track(), rig.track());
    rig.track_mut(slow).output = Some(bus);
    rig.track_mut(plain).sends.push(Send {
        target: bus,
        gain: 0.5,
    });
    rig.effect(slow, idle_limiter(5.0));
    rig.effect(bus, idle_limiter(2.0));
    rig.effect(TrackId::MASTER, idle_limiter(1.0));
    for (track, volume) in [(slow, 0.125), (plain, 0.25), (TrackId::MASTER, 0.0625)] {
        let click = rig.channel_on(impulse(RATE), track);
        rig.channel_mut(click).volume = volume;
        rig.steps(click, &[1]);
    }

    let (mut processor, controller) = rig.processor(RATE);
    assert_eq!(controller.latency_frames(), 240 + 96 + 48);
    controller.play();
    let audio = run(&mut processor, 12_000, 100);
    // Every path takes as long as the slowest: one frame holds it all, the
    // plain track's click twice, once directly and once through its send.
    assert_eq!(sounding_frames(&audio), [6_000 + 384]);
    assert_eq!(left(&audio)[6_384], 0.125 + 0.25 + 0.125 + 0.0625);
}

/// A project where a sine and the same sine upside down play on two tracks
/// of their own. As long as the two tracks are lined up to the sample,
/// nothing comes out.
fn cancelling_rig() -> (Rig, TrackId, TrackId) {
    let mut rig = Rig::new();
    let (first, second) = (rig.track(), rig.track());
    let tone = sine(RATE, TONE_HZ, 3.0);
    let upside_down = inverted(&tone);
    let up = rig.channel_on(tone, first);
    let down = rig.channel_on(upside_down, second);
    rig.steps(up, &[0]);
    rig.steps(down, &[0]);
    (rig, first, second)
}

#[test]
fn moving_a_look_ahead_under_sound_keeps_every_path_lined_up() {
    let (mut rig, first, second) = cancelling_rig();
    let effect = rig.effect(first, idle_limiter(5.0));
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 5_000, 128);
    assert_eq!(controller.latency_frames(), 240);

    for (lookahead_ms, latency) in [(15.0, 720), (1.0, 48), (20.0, 960)] {
        rig.effect_mut(effect).params = idle_limiter(lookahead_ms);
        controller.set_project(&rig.project, &rig.pool);
        assert_eq!(controller.latency_frames(), latency);
        audio.extend(run(&mut processor, 5_000, 128));
    }
    // The limiter crossfades to its new look-ahead and the other track's
    // delay crossfades with it, so the two cancel throughout.
    assert!(
        peak(&audio) < 1e-6,
        "the paths came apart: {}",
        peak(&audio)
    );

    // Each of them alone is anything but silent.
    rig.track_mut(second).muted = true;
    assert!(peak(&rig.play(RATE, 5_000, 128)) > 0.4);
}

#[test]
fn a_limiter_that_comes_and_goes_under_sound_keeps_every_path_lined_up() {
    let (mut rig, first, _) = cancelling_rig();
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 5_000, 128);
    let effect = rig.effect(first, idle_limiter(5.0));
    controller.set_project(&rig.project, &rig.pool);
    assert_eq!(controller.latency_frames(), 240);
    audio.extend(run(&mut processor, 5_000, 128));
    rig.remove_effect(effect);
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 5_000, 128));
    assert_eq!(controller.latency_frames(), 0);

    // The limiter runs unheard until its look-ahead is full and then fades
    // in, and the delay that makes up for it on the other track waits as
    // long and fades with it. On the way out the two fade together again.
    // So the tracks cancel throughout: no hole, no jump, no echo.
    assert!(
        peak(&audio) < 1e-6,
        "the paths came apart: {}",
        peak(&audio)
    );

    // And the tone itself moves over to its delay and back without a step.
    let (mut rig, _, track) = tone_rig();
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 5_000, 128);
    let effect = rig.effect(track, idle_limiter(5.0));
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 5_000, 128));
    rig.remove_effect(effect);
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 5_000, 128));
    let audio = left(&audio);
    assert!(largest_step(&audio[1..]) <= SINE_SLOPE + 1.0 / SPLICE_FRAMES as f32);
    assert!(rms(&audio[10_000..10_000 + FADE_FRAMES]) > 0.1);
}

#[test]
fn the_latency_reported_is_that_of_the_project_just_set() {
    // A synth, 12 frames behind, into a master whose limiter looks 240
    // frames ahead.
    let mut rig = Rig::new();
    let keys = rig.track();
    let synth = rig.synth_on(plain_synth(), keys);
    rig.note(synth, 0, 960);
    rig.effect(TrackId::MASTER, idle_limiter(5.0));
    let (mut processor, controller) = rig.processor(RATE);
    assert_eq!(controller.latency_frames(), 252);
    controller.play();
    run(&mut processor, 4_800, 480);

    // Another project takes its place, with nothing in it that is late.
    // The synth of the first one is still fading out, and does not count.
    let mut plain = Rig::new();
    let click = plain.channel(impulse(RATE));
    plain.steps(click, &[0]);
    controller.set_project(&plain.project, &plain.pool);
    assert_eq!(controller.latency_frames(), 0);
    run(&mut processor, 480, 480);
    assert_eq!(controller.latency_frames(), 0);

    // And back: the figure is the new project's at once here too.
    controller.set_project(&rig.project, &rig.pool);
    assert_eq!(controller.latency_frames(), 252);

    // A synth that goes while a limiter stays leaves the limiter's figure.
    rig.project.channels.clear();
    rig.project.patterns[0].lanes.clear();
    controller.set_project(&rig.project, &rig.pool);
    assert_eq!(controller.latency_frames(), 240);
}

#[test]
fn gain_reduction_is_reported_for_each_compressor_and_limiter() {
    let mut rig = Rig::new();
    let (_, track) = steady(&mut rig, 1.0);
    rig.effect(track, EffectParams::Eq(EqParams::default()));
    let compressor = rig.effect(
        track,
        EffectParams::Compressor(CompressorParams {
            threshold_db: -20.0,
            ratio: 4.0,
            knee_db: 0.0,
            ..CompressorParams::default()
        }),
    );
    let squeeze = rig.effect(track, limiter(-20.0));
    let idle = rig.effect(TrackId::MASTER, idle_limiter(5.0));
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    // Past the start, where the limiter catches what the compressor is too
    // slow for and then lets go of it again.
    run(&mut processor, 72_000, 128);
    controller.frame();
    run(&mut processor, 4_800, 128);

    let frame = controller.frame();
    let read: Vec<(EffectId, f32)> = frame
        .gain_reductions
        .iter()
        .map(|reading| (reading.effect, reading.db))
        .collect();
    // In mixer order, the master first, and on a track in chain order. The
    // equaliser has no meter.
    let ids: Vec<EffectId> = read.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, [idle, compressor, squeeze]);
    // Full scale is 20 dB over the threshold, and at four to one 15 dB of
    // that are taken off. The limiter takes off the other 5.
    assert_eq!(read[0].1, 0.0);
    assert!((read[1].1 - 15.0).abs() < 0.5, "{}", read[1].1);
    assert!((read[2].1 - 5.0).abs() < 0.5, "{}", read[2].1);

    // Reading resets the meters, and they fill again as audio runs.
    let again = controller.frame();
    assert!(
        again
            .gain_reductions
            .iter()
            .all(|reading| reading.db == 0.0)
    );
    run(&mut processor, 256, 128);
    assert!(controller.frame().gain_reductions[1].db > 14.0);

    // An effect that leaves takes its meter along.
    rig.remove_effect(compressor);
    controller.set_project(&rig.project, &rig.pool);
    let ids: Vec<EffectId> = controller
        .frame()
        .gain_reductions
        .iter()
        .map(|reading| reading.effect)
        .collect();
    assert_eq!(ids, [idle, squeeze]);
}

#[test]
fn the_tempo_reaches_effects_that_follow_it() {
    // A delay of one quarter note with nothing but its echo coming out.
    let echo = EffectParams::Delay(DelayParams {
        sync: true,
        division: NoteDivision::Quarter,
        feedback: 0.0,
        mix: 1.0,
        ..DelayParams::default()
    });
    let mut rig = Rig::new();
    let track = rig.track();
    let click = rig.channel_on(impulse(RATE), track);
    rig.steps(click, &[0, 8]);
    rig.effect(track, echo);
    let loudest = |audio: &[f32]| {
        let peak = peak(audio);
        audio
            .iter()
            .position(|sample| sample.abs() == peak)
            .unwrap()
    };

    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 30_000, 128);
    // At 120 bpm a quarter note is 24000 frames.
    let first = loudest(&left(&audio));
    assert!(
        first.abs_diff(24_000) <= 2,
        "the echo came on frame {first}"
    );

    // The tempo halves on frame 30000, which is tick 1200. Step 8, on tick
    // 1920, is then 720 ticks of 50 frames away, on frame 66000, and its
    // echo a quarter note of 48000 frames after that.
    rig.project.settings.tempo_bpm = 60.0;
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 100_000, 128));
    let second = 66_000 + loudest(&left(&audio)[66_000..]);
    assert!(
        second.abs_diff(66_000 + 48_000) <= 2,
        "the echo came on frame {second}"
    );
}

#[test]
fn every_buffer_size_gives_the_same_audio_through_effects() {
    let build = || {
        let mut rig = Rig::new();
        rig.project.settings.tempo_bpm = 140.0;
        let (drums, bus) = (rig.track(), rig.track());
        rig.track_mut(drums).sends.push(Send {
            target: bus,
            gain: 0.6,
        });
        let kick = rig.channel_on(sine(RATE, 80.0, 0.2), drums);
        rig.steps(kick, &[0, 3, 6]);
        rig.effect(drums, EffectParams::Eq(EqParams::default()));
        rig.effect(drums, EffectParams::Compressor(CompressorParams::default()));
        rig.effect(bus, EffectParams::Delay(DelayParams::default()));
        rig.effect(bus, EffectParams::Reverb(ReverbParams::default()));
        rig.effect(TrackId::MASTER, limiter(-6.0));
        rig
    };
    let play = |block: usize| {
        let mut rig = build();
        let (mut processor, controller) = rig.processor(RATE);
        controller.play();
        let mut audio = run(&mut processor, 8_000, block);
        // A fader, a setting, a new effect and a removed one, all while
        // sound is passing.
        rig.project.mixer.tracks[1].volume = 0.7;
        let reverb = rig.project.mixer.tracks[2].effects[1].id;
        rig.effect_mut(reverb).mix = 0.6;
        let added = rig.effect(TrackId(rig.project.mixer.tracks[1].id.0), idle_limiter(3.0));
        controller.set_project(&rig.project, &rig.pool);
        audio.extend(run(&mut processor, 8_000, block));
        rig.remove_effect(added);
        rig.remove_effect(reverb);
        controller.set_project(&rig.project, &rig.pool);
        audio.extend(run(&mut processor, 8_000, block));
        audio
    };
    let reference = play(128);
    assert!(peak(&reference) > 0.05);
    for block in [1, 7, 64, 256, 1_000, 24_000] {
        assert!(play(block) == reference, "blocks of {block} frames");
    }
}

// Voices that a change of plan takes out of their channel

#[test]
fn a_voice_that_loses_its_channel_finishes_through_its_tracks_effects() {
    // A full-scale level, held to a quarter by a limiter on its track.
    let mut rig = Rig::new();
    let (channel, track) = steady(&mut rig, 1.0);
    rig.effect(track, limiter(-12.0));
    let held = db_to_gain(-12.0);
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 9_600, 128);
    controller.frame();

    rig.project.channels.retain(|entry| entry.id != channel);
    rig.project.patterns[0].lanes.clear();
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 128, 128));
    assert_eq!(controller.frame().voices, 1);
    audio.extend(run(&mut processor, 128, 128));
    // What the limiter puts out from here on is the fading voice, 240
    // frames late. The voice is still inside the track, so the track's
    // meter shows it.
    controller.frame();
    audio.extend(run(&mut processor, 64, 64));
    assert!(controller.frame().meters[2] > 0.1);
    audio.extend(run(&mut processor, 2_080, 128));
    let audio = left(&audio);

    // It fades out under the limiter. Taken out of the track it would have
    // come out at full scale, four times as loud as it was heard.
    assert_close(audio[9_599], held);
    assert!(peak(&audio[9_600..]) <= held + 1e-6);
    assert!(largest_step(&audio[9_000..]) <= 1.0 / FADE_FRAMES as f32);
    assert!(audio[11_000..].iter().all(|sample| *sample == 0.0));
    assert_eq!(controller.frame().voices, 0);
}

#[test]
fn a_fading_voice_keeps_to_its_track_when_an_effect_is_further_along() {
    // The voice's own track is plain. The limiter is on the master.
    let mut rig = Rig::new();
    let (_, _) = steady(&mut rig, 1.0);
    rig.effect(TrackId::MASTER, limiter(-12.0));
    let held = db_to_gain(-12.0);
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 9_600, 128);
    controller.stop();
    // A plan arrives while the stopped note is fading out.
    rig.project.mixer.tracks[1].volume = 0.9;
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 2_400, 128));
    let audio = left(&audio);
    assert!(peak(&audio[9_600..]) <= held + 1e-6);
    assert!(largest_step(&audio[9_000..]) <= 1.0 / FADE_FRAMES as f32);
    assert!(audio[11_000..].iter().all(|sample| *sample == 0.0));
}

#[test]
fn a_voice_whose_track_goes_too_finishes_at_the_output() {
    // With the track gone its effects are gone, and the voice has nowhere
    // left to be heard but the output, at the gain its faders gave it.
    let mut rig = Rig::new();
    let (channel, track) = steady(&mut rig, 0.5);
    rig.effect(track, EffectParams::Eq(EqParams::default()));
    rig.track_mut(track).volume = 0.5;
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 9_600, 128);
    rig.project.channels.retain(|entry| entry.id != channel);
    rig.project.patterns[0].lanes.clear();
    rig.project.mixer.tracks.retain(|entry| entry.id != track);
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 1_000, 128));
    let audio = left(&audio);
    assert_close(audio[9_599], 0.25);
    for (offset, frame) in (9_600..9_600 + FADE_FRAMES).enumerate() {
        let left_of_fade = (FADE_FRAMES - offset) as f32 / FADE_FRAMES as f32;
        assert_close(audio[frame], 0.25 * left_of_fade);
    }
    assert!(
        audio[9_600 + FADE_FRAMES..]
            .iter()
            .all(|sample| *sample == 0.0)
    );
}

#[test]
fn a_channel_moved_to_another_track_crossfades_between_the_two() {
    // The tone moves from a track at unity to one at a quarter that also
    // has a limiter with 5 ms of look-ahead.
    let (mut rig, tone, _) = tone_rig();
    let quiet = rig.track();
    rig.track_mut(quiet).volume = 0.25;
    rig.effect(quiet, idle_limiter(5.0));
    let dry = left(&rig.play(RATE, 12_000, 128));

    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 6_000, 128);
    rig.channel_mut(tone).mixer_track = quiet;
    controller.set_project(&rig.project, &rig.pool);
    audio.extend(run(&mut processor, 6_000, 128));
    let audio = left(&audio);

    // Before, the tone as it is, delayed like everything else by the
    // limiter it does not go through. After, a quarter of it.
    assert_eq!(audio[..6_000], dry[..6_000]);
    for frame in 6_600..12_000 {
        assert!(
            (audio[frame] - 0.25 * dry[frame]).abs() < 1e-6,
            "frame {frame}"
        );
    }
    // In between it leaves the one as it arrives on the other. Switched at
    // once, the tone would jump by three quarters of where it stood, and
    // go silent for the length of the look-ahead.
    let fade = SINE_SLOPE + 1.0 / FADE_FRAMES as f32;
    assert!(largest_step(&audio[1..]) <= fade);
    assert_eq!(controller.frame().voices, 1);
}

#[test]
fn the_playhead_is_where_the_sound_being_heard_is() {
    // A limiter that looks 20 ms ahead puts the whole mix 960 frames
    // behind. The pattern is four steps, 24000 frames, long.
    let mut rig = Rig::new();
    let pattern = rig.first_pattern();
    rig.pattern_mut(pattern).length_steps = 4;
    let click = rig.channel(impulse(RATE));
    rig.steps(click, &[1]);
    rig.effect(TrackId::MASTER, idle_limiter(20.0));
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();

    // Until the start of the pattern has reached the output, the playhead
    // waits there.
    let mut audio = run(&mut processor, 480, 128);
    assert!(controller.frame().playing);
    assert_eq!(controller.frame().tick, 0.0);
    // The click on step 1, tick 240, is heard on frame 6960, and that is
    // the frame on which the playhead gets to it.
    audio.extend(run(&mut processor, 6_480, 128));
    assert_eq!(controller.frame().tick, 240.0);
    audio.extend(run(&mut processor, 40, 40));
    assert_eq!(sounding_frames(&audio), [6_960]);

    // The engine is already working on the second pass when the end of the
    // first is still on its way out.
    audio.extend(run(&mut processor, 24_480 - 7_000, 120));
    assert!((controller.frame().tick - 940.8).abs() < 1e-9);
    audio.extend(run(&mut processor, 730, 73));
    assert!((controller.frame().tick - 10.0).abs() < 1e-9);

    // Stopped, it is back where playback started.
    controller.stop();
    run(&mut processor, 128, 128);
    assert_eq!(controller.frame().tick, 0.0);
}
