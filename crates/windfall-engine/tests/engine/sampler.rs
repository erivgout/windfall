//! Sampler voices: pitch, resampling, envelopes, cuts, stealing, live notes
//! and previews.

use windfall_core::{AudioBuffer, TICKS_PER_STEP, db_to_gain};
use windfall_engine::{PREVIEW_GAIN_DB, Processor};
use windfall_project::{ChannelId, Envelope, MAX_ENVELOPE_MS, MAX_PATTERN_STEPS};

use crate::support::{
    Rig, fall, frequency, impulse, largest_step, left, level, peak, run, sine, sounding_frames,
};

const RATE: u32 = 48_000;

/// Frames a cut, stolen or stopped voice takes to fade out: 4 ms.
const FADE_FRAMES: usize = 192;

/// An envelope that is fully open while the note is held and closes over
/// `release_ms` after it.
fn gate(release_ms: f32) -> Envelope {
    Envelope {
        attack_ms: 0.0,
        decay_ms: 0.0,
        sustain: 1.0,
        release_ms,
    }
}

#[test]
fn pitch_follows_the_key_relative_to_the_root() {
    for (key, tune, expected) in [
        (60, 0.0, 220.0),
        (72, 0.0, 440.0),
        (48, 0.0, 110.0),
        (60, 12.0, 440.0),
        (67, 0.0, 329.628),
        (60, -0.5, 213.737),
    ] {
        let mut rig = Rig::new();
        let channel = rig.channel(sine(RATE, 220.0, 1.0));
        rig.sampler_mut(channel).tune = tune;
        rig.note(channel, 0, 240).key = key;
        let audio = left(&rig.play(RATE, 19_200, 256));
        let measured = frequency(&audio, RATE);
        assert!(
            (measured - expected).abs() < 0.2,
            "key {key} tuned {tune}: {measured} Hz, expected {expected}"
        );
    }
}

#[test]
fn the_root_key_plays_the_sample_at_its_recorded_pitch() {
    let mut rig = Rig::new();
    let channel = rig.channel(sine(RATE, 300.0, 1.0));
    rig.sampler_mut(channel).root_key = 72;
    rig.note(channel, 0, 240).key = 72;
    let audio = left(&rig.play(RATE, 19_200, 256));
    assert!((frequency(&audio, RATE) - 300.0).abs() < 0.2);
}

#[test]
fn a_sample_keeps_its_pitch_and_level_at_any_output_rate() {
    for output_rate in [22_050, 44_100, 48_000, 96_000] {
        let mut rig = Rig::new();
        let channel = rig.channel(sine(44_100, 441.0, 1.0));
        rig.note(channel, 0, 240);
        let frames = output_rate as usize / 2;
        let audio = left(&rig.play(output_rate, frames, 256));
        let measured = frequency(&audio, output_rate);
        assert!(
            (measured - 441.0).abs() < 0.2,
            "{measured} Hz at an output rate of {output_rate}"
        );
        assert!((peak(&audio) - 0.5).abs() < 0.005);
    }
}

#[test]
fn resampling_follows_the_curve_between_samples() {
    // A 5 kHz sine has under nine samples a cycle at 44.1 kHz. Joining them
    // with straight lines would miss the true wave by up to 0.03 here.
    let mut rig = Rig::new();
    let channel = rig.channel(sine(44_100, 5_000.0, 0.5));
    rig.note(channel, 0, 240);
    let audio = left(&rig.play(RATE, 9_600, 256));
    let ideal = sine(RATE, 5_000.0, 0.2);
    let worst = audio[8..]
        .iter()
        .zip(&ideal.samples()[8..])
        .map(|(got, wanted)| (got - wanted).abs())
        .fold(0.0, f32::max);
    assert!(worst < 0.006, "off by up to {worst}");
}

#[test]
fn a_sample_at_the_output_rate_passes_through_untouched() {
    let source = sine(RATE, 997.0, 0.25);
    let mut rig = Rig::new();
    let channel = rig.channel(source.clone());
    rig.note(channel, 0, 240);
    let audio = left(&rig.play(RATE, 12_000, 128));
    assert_eq!(audio, source.samples());
}

#[test]
fn without_an_envelope_the_whole_sample_plays_whatever_the_note_length() {
    let mut rig = Rig::new();
    let channel = rig.channel(level(RATE, 0.5, 0.5));
    rig.note(channel, 0, 1);
    let audio = rig.play(RATE, 30_000, 256);
    assert_eq!(sounding_frames(&audio), (0..24_000).collect::<Vec<_>>());
}

#[test]
fn with_an_envelope_the_note_length_gates_the_sample() {
    let mut rig = Rig::new();
    let channel = rig.channel(level(RATE, 0.5, 2.0));
    // 10 ms of release is 480 frames.
    rig.sampler_mut(channel).envelope = Some(gate(10.0));
    // One step at 120 bpm is 6000 frames.
    rig.note(channel, 0, TICKS_PER_STEP);
    let audio = left(&rig.play(RATE, 12_000, 256));

    // The frame the note ends on still plays at the full level.
    assert!(audio[..=6_000].iter().all(|sample| *sample == 0.5));
    // The release is an exponential curve: a tenth of the way through it
    // the level has halved, and half way through it is 30 dB down.
    assert!((audio[6_048] - 0.5 * fall(48, 480)).abs() < 1e-6);
    assert!((audio[6_048] - 0.25).abs() < 1e-3);
    assert!((audio[6_240] - 0.5 * fall(240, 480)).abs() < 1e-6);
    assert!(audio[6_240] < 0.5 * 0.032);
    // It reaches silence exactly when its time is up, and not before.
    assert!(audio[6_479] > 0.0);
    assert!(audio[6_480..].iter().all(|sample| *sample == 0.0));
    // It is steepest at the start and never steps further than it does
    // there.
    assert!(largest_step(&audio[1..]) <= 0.5 * (1.0 - fall(1, 480)) * 1.01);
}

#[test]
fn a_release_of_zero_still_closes_without_a_click() {
    let mut rig = Rig::new();
    let channel = rig.channel(level(RATE, 0.5, 2.0));
    rig.sampler_mut(channel).envelope = Some(gate(0.0));
    rig.note(channel, 0, TICKS_PER_STEP);
    let audio = left(&rig.play(RATE, 7_000, 256));
    assert_eq!(audio[5_999], 0.5);
    // The shortest release is a millisecond, 48 frames, along the same
    // curve as any other.
    assert!((audio[6_024] - 0.5 * fall(24, 48)).abs() < 1e-6);
    assert!(audio[6_047] > 0.0);
    assert!(audio[6_048..].iter().all(|sample| *sample == 0.0));
    // No frame drops by more than the curve's first step, which is under a
    // seventh of the level. Cutting the note off would drop all of it.
    assert!(largest_step(&audio[1..]) <= 0.5 * (1.0 - fall(1, 48)) * 1.01);
    assert!(largest_step(&audio[1..]) < 0.5 / 7.0);
}

#[test]
fn the_envelope_shapes_attack_decay_and_sustain() {
    let mut rig = Rig::new();
    let channel = rig.channel(level(RATE, 1.0, 2.0));
    // 480 frames up, 480 frames down to half.
    rig.sampler_mut(channel).envelope = Some(Envelope {
        attack_ms: 10.0,
        decay_ms: 10.0,
        sustain: 0.5,
        release_ms: 10.0,
    });
    rig.note(channel, 0, 960);
    let audio = left(&rig.play(RATE, 12_000, 256));
    // The attack is a straight line that arrives on its last frame.
    assert_eq!(audio[0], 0.0);
    assert!((audio[240] - 0.5).abs() < 1e-6);
    assert!((audio[479] - 479.0 / 480.0).abs() < 1e-6);
    assert_eq!(audio[480], 1.0);
    // The decay is an exponential curve down to the sustain level: half of
    // the way there after a tenth of its time.
    assert!((audio[480 + 48] - (0.5 + 0.5 * fall(48, 480))).abs() < 1e-6);
    assert!((audio[480 + 48] - 0.75).abs() < 1e-3);
    assert!((audio[720] - (0.5 + 0.5 * fall(240, 480))).abs() < 1e-6);
    assert!(audio[959] > 0.5);
    assert!(audio[960..10_000].iter().all(|sample| *sample == 0.5));
    assert!(largest_step(&audio[480..]) <= 0.5 * (1.0 - fall(1, 480)) * 1.01);
}

#[test]
fn the_longest_decay_still_arrives_at_the_sustain_level_on_time() {
    // A minute of decay from 1 to 0.95 at 48 kHz moves the level by
    // 0.000000017 a frame, less than a 32-bit float near 1 can tell apart.
    let mut rig = Rig::new();
    let channel = rig.channel(level(RATE, 1.0, 61.0));
    rig.sampler_mut(channel).envelope = Some(Envelope {
        attack_ms: 0.0,
        decay_ms: MAX_ENVELOPE_MS,
        sustain: 0.95,
        release_ms: 10.0,
    });
    // At 120 bpm the longest pattern lasts over two minutes.
    let pattern = rig.first_pattern();
    rig.pattern_mut(pattern).length_steps = MAX_PATTERN_STEPS;
    rig.note(channel, 0, 240_000);

    let decay = 60 * RATE as usize;
    let audio = left(&rig.play(RATE, decay + 1_000, 4_096));
    assert_eq!(audio[0], 1.0);
    assert!((audio[decay / 10] - (0.95 + 0.05 * fall(1, 10))).abs() < 1e-6);
    assert!((audio[decay / 10] - 0.975).abs() < 1e-4);
    assert!((audio[decay / 2] - (0.95 + 0.05 * fall(1, 2))).abs() < 1e-6);
    assert!(audio[..decay].is_sorted_by(|a, b| a >= b));
    assert!(audio[decay - RATE as usize] > 0.95);
    assert!(audio[decay..].iter().all(|sample| *sample == 0.95));
}

#[test]
fn cut_self_fades_out_the_note_before() {
    let mut rig = Rig::new();
    let channel = rig.channel(level(RATE, 0.5, 1.0));
    rig.steps(channel, &[0, 4]);

    // Without it the two notes overlap.
    let overlapping = left(&rig.play(RATE, 40_000, 256));
    assert_eq!(overlapping[30_000], 1.0);

    rig.sampler_mut(channel).cut_self = true;
    let cut = left(&rig.play(RATE, 40_000, 256));
    assert_eq!(cut[23_999], 0.5);
    assert_eq!(cut[30_000], 0.5);

    // What is left after taking the second note away is the first note
    // alone: full until the second starts, then a short straight fade.
    let first_alone: Vec<f32> = cut
        .iter()
        .enumerate()
        .map(|(frame, sample)| {
            if frame >= 24_000 {
                sample - 0.5
            } else {
                *sample
            }
        })
        .collect();
    assert!((first_alone[24_000] - 0.5).abs() < 1e-6);
    assert!((first_alone[24_000 + FADE_FRAMES / 2] - 0.25).abs() < 1e-3);
    assert!(
        first_alone[24_000 + FADE_FRAMES..]
            .iter()
            .all(|sample| sample.abs() < 1e-6)
    );
    assert!(largest_step(&first_alone[1..]) <= 0.5 / FADE_FRAMES as f32 * 1.01);
}

#[test]
fn notes_that_start_together_on_a_cut_self_channel_all_play() {
    let mut rig = Rig::new();
    let channel = rig.channel(level(RATE, 0.25, 1.0));
    rig.sampler_mut(channel).cut_self = true;
    for key in [60, 64, 67] {
        rig.note(channel, 0, 240).key = key;
    }
    let audio = left(&rig.play(RATE, 4_000, 256));
    assert!(audio.iter().all(|sample| *sample == 0.75));
}

#[test]
fn channels_in_one_cut_group_stop_each_other() {
    let mut rig = Rig::new();
    let open = rig.channel(level(RATE, 0.5, 2.0));
    let closed = rig.channel(level(RATE, 0.25, 2.0));
    let bystander = rig.channel(level(RATE, 0.125, 2.0));
    rig.steps(open, &[0]);
    rig.steps(closed, &[4, 8]);
    rig.steps(bystander, &[0]);

    let free = left(&rig.play(RATE, 60_000, 256));
    assert_eq!(free[30_000], 0.875);

    rig.sampler_mut(open).cut_group = 3;
    rig.sampler_mut(closed).cut_group = 3;
    rig.sampler_mut(bystander).cut_group = 4;
    let grouped = left(&rig.play(RATE, 60_000, 256));
    assert_eq!(grouped[23_999], 0.625);
    // The closed hat cut the open one, and nothing else.
    assert!((grouped[30_000] - 0.375).abs() < 1e-6);
    // A group does not cut its own channel: both closed hats sound.
    assert!((grouped[54_000] - 0.625).abs() < 1e-6);
    // The cut is a fade, on top of the step the new note makes by starting.
    assert!(largest_step(&grouped[24_001..47_000]) <= 0.5 / FADE_FRAMES as f32 * 1.01);
}

#[test]
fn a_note_with_no_velocity_takes_no_voice_but_still_cuts() {
    let mut rig = Rig::new();
    let channel = rig.channel(level(RATE, 0.5, 1.0));
    rig.sampler_mut(channel).cut_self = true;
    rig.steps(channel, &[0]);
    for _ in 0..300 {
        rig.note(channel, 4 * TICKS_PER_STEP, TICKS_PER_STEP)
            .velocity = 0.0;
    }
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 24_000, 256);
    assert_eq!(controller.frame().voices, 1);

    // The silent notes arrive on frame 24000. They cut the note before
    // them, as any note on this channel does, and add nothing of their own.
    audio.extend(run(&mut processor, 64, 64));
    assert_eq!(controller.frame().voices, 1);
    audio.extend(run(&mut processor, 1_000, 256));
    assert_eq!(controller.frame().voices, 0);
    let audio = left(&audio);
    assert_eq!(audio[23_999], 0.5);
    assert!((audio[24_000 + FADE_FRAMES / 2] - 0.25).abs() < 1e-3);
    assert!(
        audio[24_000 + FADE_FRAMES..]
            .iter()
            .all(|sample| *sample == 0.0)
    );
}

#[test]
fn start_end_and_reverse_choose_what_plays() {
    let ramp: Vec<f32> = (1..=10).map(|value| value as f32 / 10.0).collect();
    let mut rig = Rig::new();
    let channel = rig.channel(AudioBuffer::from_interleaved(RATE, 1, ramp));
    rig.note(channel, 0, 240);
    rig.sampler_mut(channel).start = 0.2;
    rig.sampler_mut(channel).end = 0.7;
    let audio = left(&rig.play(RATE, 8, 8));
    assert_eq!(audio, [0.3, 0.4, 0.5, 0.6, 0.7, 0.0, 0.0, 0.0]);

    rig.sampler_mut(channel).reverse = true;
    let audio = left(&rig.play(RATE, 8, 8));
    assert_eq!(audio, [0.7, 0.6, 0.5, 0.4, 0.3, 0.0, 0.0, 0.0]);
}

#[test]
fn a_stereo_sample_keeps_its_sides() {
    let mut rig = Rig::new();
    let stereo = AudioBuffer::from_interleaved(RATE, 2, vec![0.1, -0.2, 0.3, -0.4]);
    let channel = rig.channel(stereo);
    rig.note(channel, 0, 240);
    let audio = rig.play(RATE, 3, 3);
    assert_eq!(audio, [0.1, -0.2, 0.3, -0.4, 0.0, 0.0]);
}

#[test]
fn velocity_gain_and_pan_set_the_level_of_each_side() {
    let mut rig = Rig::new();
    let channel = rig.channel(level(RATE, 0.5, 1.0));
    rig.sampler_mut(channel).gain = 1.5;
    rig.channel_mut(channel).volume = 0.8;
    rig.channel_mut(channel).pan = 0.25;
    let note = rig.note(channel, 0, 240);
    note.velocity = 0.5;
    note.pan = 0.25;
    let audio = rig.play(RATE, 100, 100);
    // 0.5 * 0.5 * 1.5 * 0.8, and a pan of 0.5 halves the left side.
    assert!((audio[0] - 0.15).abs() < 1e-6);
    assert!((audio[1] - 0.3).abs() < 1e-6);

    // Note pan and channel pan add up and stop at hard left.
    let mut rig = Rig::new();
    let channel = rig.channel(level(RATE, 0.5, 1.0));
    rig.channel_mut(channel).pan = -0.75;
    rig.note(channel, 0, 240).pan = -0.75;
    let audio = rig.play(RATE, 100, 100);
    assert_eq!((audio[0], audio[1]), (0.5, 0.0));
}

#[test]
fn a_missing_sample_plays_nothing() {
    let mut rig = Rig::new();
    let channel = rig.channel(impulse(RATE));
    rig.steps(channel, &[0, 1]);
    rig.pool = windfall_engine::SamplePool::new();
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let audio = run(&mut processor, 10_000, 256);
    assert!(sounding_frames(&audio).is_empty());
    assert_eq!(controller.frame().voices, 0);
}

#[test]
fn a_full_pool_steals_voices_with_a_fade() {
    let mut rig = Rig::new();
    let channel = rig.channel(level(RATE, 0.001, 1.0));
    for _ in 0..300 {
        rig.note(channel, 0, 240);
    }
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 64, 64);
    // 256 voices stay and the 44 that were stolen are on their way out.
    assert_eq!(controller.frame().voices, 300);
    audio.extend(run(&mut processor, 1_000, 64));
    assert_eq!(controller.frame().voices, 256);

    let audio = left(&audio);
    assert!((audio[0] - 0.3).abs() < 1e-4);
    assert!((audio[1_000] - 0.256).abs() < 1e-4);
    assert!(largest_step(&audio) <= 0.044 / FADE_FRAMES as f32 * 1.05);
}

#[test]
fn far_more_notes_than_voices_never_overflows_the_pool() {
    let mut rig = Rig::new();
    let channel = rig.channel(level(RATE, 0.001, 1.0));
    for index in 0..700 {
        rig.note(channel, (index / 350) * 240, 240);
    }
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 64, 64);
    // Every slot is taken: 256 voices and 64 fading out.
    assert_eq!(controller.frame().voices, 320);
    audio.extend(run(&mut processor, 12_000, 128));
    assert_eq!(controller.frame().voices, 256);
    assert!(audio.iter().all(|sample| sample.is_finite()));
    assert!((peak(&audio[20_000..]) - 0.256).abs() < 1e-3);
}

#[test]
fn stopping_fades_every_note_out() {
    let mut rig = Rig::new();
    let channel = rig.channel(level(RATE, 0.5, 2.0));
    rig.steps(channel, &[0]);
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let mut audio = run(&mut processor, 4_800, 256);
    controller.stop();
    audio.extend(run(&mut processor, 4_800, 256));
    let audio = left(&audio);
    assert_eq!(audio[4_799], 0.5);
    assert!(
        audio[4_800 + FADE_FRAMES..]
            .iter()
            .all(|sample| *sample == 0.0)
    );
    assert!(largest_step(&audio[1..]) <= 0.5 / FADE_FRAMES as f32 * 1.01);
    assert_eq!(controller.frame().voices, 0);
}

#[test]
fn live_notes_play_without_the_transport() {
    let mut rig = Rig::new();
    let one_shot = rig.channel(level(RATE, 0.5, 0.1));
    let held = rig.channel(level(RATE, 0.25, 2.0));
    rig.sampler_mut(held).envelope = Some(gate(10.0));
    let (mut processor, controller) = rig.processor(RATE);

    // A sampler with no envelope plays to the end, note-off or not.
    controller.note_on(one_shot, 60, 1.0);
    let mut audio = run(&mut processor, 1_000, 100);
    controller.note_off(one_shot, 60);
    audio.extend(run(&mut processor, 5_000, 100));
    assert_eq!(sounding_frames(&audio), (0..4_800).collect::<Vec<_>>());
    assert!(!controller.frame().playing);

    // One with an envelope sounds until the key comes up.
    controller.note_on(held, 60, 0.5);
    controller.note_on(held, 64, 0.5);
    let mut audio = run(&mut processor, 2_000, 100);
    assert_eq!(controller.frame().voices, 2);
    controller.note_off(held, 60);
    audio.extend(run(&mut processor, 2_000, 100));
    assert_eq!(controller.frame().voices, 1);
    controller.note_off(held, 64);
    audio.extend(run(&mut processor, 2_000, 100));
    assert_eq!(controller.frame().voices, 0);
    let audio = left(&audio);
    assert_eq!(audio[1_999], 0.25);
    assert_eq!(audio[3_999], 0.125);
    assert_eq!(audio[5_999], 0.0);

    // A channel that does not exist is ignored.
    controller.note_on(ChannelId(9_999), 60, 1.0);
    let audio = run(&mut processor, 500, 100);
    assert!(sounding_frames(&audio).is_empty());

    // Stop silences held notes as well.
    controller.note_on(held, 60, 1.0);
    run(&mut processor, 500, 100);
    assert_eq!(controller.frame().voices, 1);
    controller.stop();
    let audio = left(&run(&mut processor, 500, 100));
    assert_eq!(controller.frame().voices, 0);
    assert!(largest_step(&audio) <= 0.25 / FADE_FRAMES as f32 * 1.01);
}

#[test]
fn a_live_note_is_pitched_by_its_key() {
    let mut rig = Rig::new();
    let channel = rig.channel(sine(RATE, 220.0, 1.0));
    let (mut processor, controller) = rig.processor(RATE);
    controller.note_on(channel, 72, 1.0);
    let audio = left(&run(&mut processor, 19_200, 256));
    assert!((frequency(&audio, RATE) - 440.0).abs() < 0.2);
}

#[test]
fn a_preview_plays_into_the_master_whatever_the_transport_does() {
    // No project is loaded at all.
    let (mut processor, controller) = Processor::new(RATE);
    controller.preview(sine(44_100, 441.0, 1.0));
    let mut audio = run(&mut processor, 9_600, 256);
    assert!((frequency(&left(&audio), RATE) - 441.0).abs() < 0.3);
    // The sine peaks at 0.5, and a preview plays 6 dB down.
    let heard = 0.5 * db_to_gain(PREVIEW_GAIN_DB);
    let frame = controller.frame();
    assert_eq!(frame.voices, 1);
    assert!((frame.meters[0] - heard).abs() < 0.005 && (frame.meters[1] - heard).abs() < 0.005);

    // Stopping the transport leaves it alone.
    controller.stop();
    audio = run(&mut processor, 4_800, 256);
    assert!((peak(&audio) - heard).abs() < 0.005);

    // Stopping the preview fades it out.
    controller.stop_preview();
    audio.extend(run(&mut processor, 1_000, 256));
    let tail = left(&audio);
    assert!(
        tail[4_800 + FADE_FRAMES..]
            .iter()
            .all(|sample| *sample == 0.0)
    );
    // The sine moves at most 0.015 a frame, and the fade adds a little.
    assert!(largest_step(&tail) <= heard * 441.0 * std::f32::consts::TAU / 48_000.0 + 0.0015);
    assert_eq!(controller.frame().voices, 0);
}

#[test]
fn a_preview_plays_six_decibels_down_so_it_does_not_clip_a_full_mix() {
    assert_eq!(PREVIEW_GAIN_DB, -6.0);
    let gain = db_to_gain(PREVIEW_GAIN_DB);
    assert!((gain - 0.501).abs() < 0.001);

    // A project playing at full scale, and a full-scale file previewed
    // over it.
    let mut rig = Rig::new();
    let channel = rig.channel(level(RATE, 1.0, 1.0));
    rig.steps(channel, &[0]);
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    controller.preview(level(RATE, 1.0, 1.0));
    let audio = run(&mut processor, 1_000, 128);
    assert!(audio.iter().all(|sample| *sample == 1.0 + gain));
    assert_eq!(controller.frame().meters, vec![1.0 + gain, 1.0 + gain]);
}

#[test]
fn a_new_preview_replaces_the_one_playing() {
    let (mut processor, controller) = Processor::new(RATE);
    controller.preview(level(RATE, 0.5, 1.0));
    run(&mut processor, 1_000, 100);
    controller.preview(level(RATE, 0.25, 1.0));
    let audio = left(&run(&mut processor, 1_000, 100));
    let gain = db_to_gain(PREVIEW_GAIN_DB);
    assert!((audio[0] - 0.75 * gain).abs() < 1e-6);
    assert!((audio[FADE_FRAMES / 2] - 0.5 * gain).abs() < 1e-6);
    assert!(
        audio[FADE_FRAMES..]
            .iter()
            .all(|sample| *sample == 0.25 * gain)
    );
    assert_eq!(controller.frame().voices, 1);
}
