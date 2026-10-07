//! Offline rendering: length, determinism and agreement with playback.

use windfall_core::{AudioBuffer, db_to_gain};
use windfall_dsp::{CompressorParams, DelayParams, EqParams, ReverbParams};
use windfall_engine::{RenderOptions, TAIL_SILENCE_DB, render};
use windfall_ipc::{PlayMode, TransportPatch};
use windfall_project::{EffectParams, Envelope, Send, TrackId};

use crate::support::{
    Rig, automate_everything, idle_limiter, impulse, left, level, limiter, peak, plain_synth, run,
    sine, sounding_frames,
};

const RATE: u32 = 48_000;

/// A project with resampling, an envelope, swing and a cut in it, so a
/// render has plenty of ways to differ if anything depended on block size.
pub fn song() -> Rig {
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

/// A project with more notes inside one buffer than the engine takes from
/// the sequencer in one go, and far more than there are voices, so a render
/// goes through several rounds of notes and steals voices all the time.
pub fn crowd() -> Rig {
    let mut rig = Rig::new();
    rig.project.settings.tempo_bpm = 522.0;
    let pattern = rig.first_pattern();
    rig.pattern_mut(pattern).length_steps = 4;
    let pad = rig.channel(sine(44_100, 220.0, 0.05));
    rig.sampler_mut(pad).envelope = Some(Envelope {
        attack_ms: 1.0,
        decay_ms: 5.0,
        sustain: 0.5,
        release_ms: 8.0,
    });
    for index in 0..1_300_u32 {
        let note = rig.note(pad, 0, 100 + index % 7 * 40);
        note.key = 48 + (index % 24) as u8;
        // Every third note is silent and must not take a voice.
        note.velocity = (index % 3) as f32 * 0.002;
        note.pan = (index % 5) as f32 * 0.4 - 0.8;
    }
    let hat = rig.channel(sine(RATE, 3_000.0, 0.02));
    rig.sampler_mut(hat).cut_self = true;
    for index in 0..1_200_u32 {
        rig.note(hat, 10 + index % 3, 30).velocity = 0.001;
    }
    let click = rig.channel(impulse(RATE));
    rig.steps(click, &[0, 1, 3]);
    rig.note(click, 40, 240);

    // Two clips that overlap, so the song finds its notes out of order.
    let lane = rig.playlist_track();
    let other_lane = rig.playlist_track();
    rig.clip(lane, pattern, 0, 1_500);
    rig.clip(other_lane, pattern, 5, 1_000).offset = 950;
    rig
}

/// A project with every kind of effect and a synth in it, on tracks that
/// feed each other, so that latency is compensated on several paths.
pub fn studio() -> Rig {
    let mut rig = Rig::new();
    rig.project.settings.tempo_bpm = 200.0;
    rig.project.settings.swing = 0.2;
    let pattern = rig.first_pattern();
    rig.pattern_mut(pattern).length_steps = 8;
    let (drums, keys, space) = (rig.track(), rig.track(), rig.track());
    rig.track_mut(drums).sends.push(Send {
        target: space,
        gain: 0.5,
    });
    rig.track_mut(keys).sends.push(Send {
        target: space,
        gain: 0.7,
    });

    let kick = rig.channel_on(sine(44_100, 70.0, 0.15), drums);
    rig.steps(kick, &[0, 3, 4, 7]);
    let hat = rig.channel_on(sine(RATE, 6_000.0, 0.05), drums);
    rig.sampler_mut(hat).cut_self = true;
    rig.steps(hat, &[0, 1, 2, 3, 4, 5, 6, 7]);
    rig.effect(drums, EffectParams::Eq(EqParams::default()));
    rig.effect(drums, EffectParams::Compressor(CompressorParams::default()));
    rig.effect(drums, limiter(-3.0));

    let mut pad = plain_synth();
    pad.unison_voices = 3;
    pad.amp_envelope.release_ms = 80.0;
    pad.filter.cutoff_hz = 2_000.0;
    let synth = rig.synth_on(pad, keys);
    for (index, key) in [57, 60, 64, 60].into_iter().enumerate() {
        rig.note(synth, index as u32 * 480, 400).key = key;
    }
    rig.note(synth, 100, 1_700).key = 45;

    rig.effect(
        space,
        EffectParams::Delay(DelayParams {
            feedback: 0.3,
            ..DelayParams::default()
        }),
    );
    rig.effect(
        space,
        EffectParams::Reverb(ReverbParams {
            decay_s: 0.4,
            ..ReverbParams::default()
        }),
    );
    rig.effect(TrackId::MASTER, limiter(-1.0));

    let lane = rig.playlist_track();
    rig.clip(lane, pattern, 0, 1_920);
    rig.clip(lane, pattern, 1_920, 1_000).offset = 300;
    rig
}

/// The studio with audio clips on its playlist: pitched, reversed, faded,
/// started part way in, overlapping, at other sample rates, and on tracks
/// with effects and with latency.
pub fn tape() -> Rig {
    let mut rig = studio();
    let (drums, space) = (
        rig.project.mixer.tracks[1].id,
        rig.project.mixer.tracks[3].id,
    );
    let (lane, other) = (rig.playlist_track(), rig.playlist_track());
    let riser = rig.audio_clip(lane, sine(44_100, 330.0, 0.6), drums, 100, 1_500);
    rig.audio_mut(riser, |audio| {
        audio.pitch = 3.5;
        audio.gain = 0.4;
        audio.pan = -0.3;
        audio.fade_in = 200;
        audio.fade_out = 300;
    });
    rig.clip_mut(riser).offset = 77;
    let reversed = rig.audio_clip(lane, sine(RATE, 440.0, 0.3), space, 901, 2_000);
    rig.audio_mut(reversed, |audio| {
        audio.reverse = true;
        audio.pitch = -7.0;
        audio.gain = 0.5;
    });
    let bed = rig.audio_clip(other, sine(32_000, 220.0, 2.0), TrackId::MASTER, 0, 2_920);
    rig.audio_mut(bed, |audio| {
        audio.gain = 0.3;
        audio.fade_out = 1_000;
    });
    // A short one that starts between two frames and is cut by its length.
    rig.audio_clip(other, sine(RATE, 1_000.0, 0.5), drums, 1_333, 120);
    rig
}

/// The tape with an automation of every kind of target on its playlist,
/// the tempo among them.
pub fn scored() -> Rig {
    let mut rig = tape();
    let synth = rig.project.channels[2].id;
    automate_everything(&mut rig, synth);
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
    let rigs = [
        ("song", song()),
        ("crowd", crowd()),
        ("studio", studio()),
        ("tape", tape()),
        ("scored", scored()),
    ];
    for (name, rig) in rigs {
        for mode in [PlayMode::Pattern, PlayMode::Song] {
            let options = |block_frames| RenderOptions {
                mode,
                pattern_loops: 2,
                tail_secs: 0.25,
                block_frames,
                ..RenderOptions::default()
            };
            let reference = render_all(&rig, &options(128));
            assert!(reference.samples().iter().any(|sample| *sample != 0.0));
            for block_frames in [1, 7, 64, 256, 1_024, 100_000] {
                let audio = render_all(&rig, &options(block_frames));
                assert!(
                    same_bits(&audio, &reference),
                    "{name} in {mode:?} mode in blocks of {block_frames}"
                );
            }
        }
    }
}

#[test]
fn a_crowded_render_is_what_playback_sounds_like() {
    let rig = crowd();
    let rendered = render_all(
        &rig,
        &RenderOptions {
            pattern_loops: 3,
            ..RenderOptions::default()
        },
    );
    let played = rig.play(RATE, rendered.frames(), 480);
    assert!(rendered.samples() == played);
    // The click on tick 40 comes 230 frames in, after both crowds.
    assert!(left(rendered.samples())[230] > 0.9);
}

#[test]
fn a_render_through_effects_is_what_playback_sounds_like_without_the_latency() {
    let rig = studio();
    let (_, controller) = rig.processor(RATE);
    // The drums are 240 frames behind for their limiter and the synth 12,
    // and the master's limiter adds another 240.
    let latency = controller.latency_frames() as usize;
    assert_eq!(latency, 240 + 240);
    // The song, which ends by itself in playback as it does in a render. A
    // pattern would loop on, and a limiter that looks ahead would already
    // be turning the end of one pass down for the start of the next.
    let options = RenderOptions {
        mode: PlayMode::Song,
        tail_secs: 0.5,
        ..RenderOptions::default()
    };
    let rendered = render_all(&rig, &options);
    let (mut processor, controller) = rig.processor(RATE);
    controller.set_transport(TransportPatch {
        mode: Some(PlayMode::Song),
        ..TransportPatch::default()
    });
    controller.play();
    let played = run(&mut processor, rendered.frames() + latency, 480);
    assert!(rendered.samples() == &played[latency * 2..]);
    // Nothing of it comes out before the latency is up, but for the
    // whisper with which the synth's filters lead a note in.
    assert!(peak(&played[..latency * 2]) < 1e-4);
    assert!(peak(rendered.samples()) > 0.1);
}

#[test]
fn a_render_with_audio_clips_is_what_playback_sounds_like_without_the_latency() {
    let rig = tape();
    let options = RenderOptions {
        mode: PlayMode::Song,
        tail_secs: 0.5,
        ..RenderOptions::default()
    };
    let rendered = render_all(&rig, &options);
    // The clips add to what the studio plays without them.
    let without = render_all(&studio(), &options);
    assert_eq!(rendered.frames(), without.frames());
    assert!(rendered.samples() != without.samples());

    let (mut processor, controller) = rig.song_processor(RATE);
    let latency = controller.latency_frames() as usize;
    assert_eq!(latency, 240 + 240);
    controller.play();
    let played = run(&mut processor, rendered.frames() + latency, 480);
    assert!(rendered.samples() == &played[latency * 2..]);
}

#[test]
fn a_render_with_automation_is_what_playback_sounds_like_without_the_latency() {
    let rig = scored();
    let options = RenderOptions {
        mode: PlayMode::Song,
        tail_secs: 0.5,
        ..RenderOptions::default()
    };
    let rendered = render_all(&rig, &options);
    // The automation is heard, and it moves the length of the song: the
    // tempo does not stay at the stored 200.
    let without = render_all(&tape(), &options);
    assert_ne!(rendered.frames(), without.frames());
    assert!(peak(rendered.samples()) > 0.1);

    let (mut processor, controller) = rig.song_processor(RATE);
    let latency = controller.latency_frames() as usize;
    controller.play();
    let played = run(&mut processor, rendered.frames() + latency, 480);
    assert!(rendered.samples() == &played[latency * 2..]);
    // Rendered twice, it is the same to the bit.
    assert!(same_bits(&rendered, &render_all(&rig, &options)));
    // The stored project plays as it did before the automation was put
    // on it: nothing of it was written into the plan.
    let mut muted = scored();
    for clip in &mut muted.project.playlist.clips {
        clip.muted |= matches!(
            clip.content,
            windfall_project::ClipContent::Automation { .. }
        );
    }
    assert!(same_bits(&render_all(&muted, &options), &without));
}

#[test]
fn a_limiter_that_looks_ahead_does_not_shift_a_render() {
    // Quiet enough that the mix stays inside full scale.
    let quiet = || {
        let mut rig = song();
        for channel in &mut rig.project.channels {
            channel.volume = 0.5;
        }
        rig
    };
    for mode in [PlayMode::Pattern, PlayMode::Song] {
        let options = RenderOptions {
            mode,
            pattern_loops: 2,
            tail_secs: 0.25,
            ..RenderOptions::default()
        };
        let reference = render_all(&quiet(), &options);
        assert!(peak(reference.samples()) > 0.3);
        assert!(peak(reference.samples()) < 1.0);

        // A limiter on the master with nothing to do: its ceiling is at
        // 0 dB and it adds no gain. All that is left of it is its
        // look-ahead, and the render takes that off its front.
        for lookahead_ms in [0.1, 5.0, 20.0] {
            let mut rig = quiet();
            rig.effect(TrackId::MASTER, idle_limiter(lookahead_ms));
            let audio = render_all(&rig, &options);
            assert_eq!(audio.frames(), reference.frames());
            let apart = audio.samples().iter().zip(reference.samples());
            let apart = apart.map(|(a, b)| (a - b).abs()).fold(0.0, f32::max);
            assert!(
                apart < 1e-6,
                "{mode:?} mode, {lookahead_ms} ms: off by {apart}"
            );
        }

        // One that does work holds the peak and moves nothing either: the
        // first sound is on the frame it was on.
        let mut rig = quiet();
        rig.effect(TrackId::MASTER, limiter(-12.0));
        let audio = render_all(&rig, &options);
        assert_eq!(audio.frames(), reference.frames());
        assert!(peak(audio.samples()) <= db_to_gain(-12.0) + 1e-6);
        assert_eq!(
            sounding_frames(audio.samples())[0],
            sounding_frames(reference.samples())[0]
        );
    }
}

#[test]
fn a_render_at_another_rate_takes_off_the_latency_of_that_rate() {
    let mut rig = Rig::new();
    let click = rig.channel(impulse(44_100));
    rig.steps(click, &[4]);
    rig.effect(TrackId::MASTER, idle_limiter(10.0));
    let options = RenderOptions {
        sample_rate: 44_100,
        ..RenderOptions::default()
    };
    let audio = render_all(&rig, &options);
    assert_eq!(audio.frames(), 88_200);
    assert_eq!(sounding_frames(audio.samples()), vec![22_050]);
}

/// A click on the last step of the bar, frame 90000, into a reverb.
pub fn echoing() -> Rig {
    let mut rig = Rig::new();
    let track = rig.track();
    let click = rig.channel_on(impulse(RATE), track);
    rig.steps(click, &[15]);
    rig.effect(
        track,
        EffectParams::Reverb(ReverbParams {
            decay_s: 1.0,
            mix: 0.5,
            ..ReverbParams::default()
        }),
    );
    rig
}

#[test]
fn an_automatic_tail_lasts_as_long_as_the_sound() {
    let rig = echoing();
    let options = |auto_tail, block_frames| RenderOptions {
        tail_secs: 20.0,
        auto_tail,
        block_frames,
        ..RenderOptions::default()
    };
    let audio = render_all(&rig, &options(true, 1_024));
    // One bar is 96000 frames. The reverb rings on past it, but for
    // nowhere near the twenty seconds the tail may take.
    assert!(audio.frames() > 96_000 + 12_000, "{}", audio.frames());
    assert!(audio.frames() < 96_000 + 5 * 48_000, "{}", audio.frames());

    // It is the start of what a tail of the full length gives, cut right
    // after the last frame that is not silent.
    let whole = render_all(&rig, &options(false, 1_024));
    assert_eq!(whole.frames(), 96_000 + 20 * 48_000);
    let cut = audio.samples().len();
    assert!(audio.samples() == &whole.samples()[..cut]);
    let silence = db_to_gain(TAIL_SILENCE_DB);
    assert!(peak(&audio.samples()[cut - 2..]) >= silence);
    assert!(peak(&whole.samples()[cut..]) < silence);

    // The block size has no say in where that is.
    for block_frames in [7, 64, 4_096, 100_000, 10_000_000] {
        let other = render_all(&rig, &options(true, block_frames));
        assert!(same_bits(&other, &audio), "blocks of {block_frames}");
    }
}

#[test]
fn an_automatic_tail_ends_with_the_sound_whatever_the_limit() {
    // A reverb set to four seconds goes on putting out numbers that are
    // not zero, and far too small to hear, for most of a minute. It sits
    // on a track of its own, so the master is fed those numbers too, and
    // a compressor on the master lets go of the tail while it dies away.
    let mut rig = Rig::new();
    let track = rig.track();
    let click = rig.channel_on(impulse(RATE), track);
    rig.steps(click, &[0]);
    rig.effect(
        track,
        EffectParams::Reverb(ReverbParams {
            decay_s: 4.0,
            mix: 0.5,
            ..ReverbParams::default()
        }),
    );
    rig.effect(
        TrackId::MASTER,
        EffectParams::Compressor(CompressorParams::default()),
    );
    let options = |tail_secs, auto_tail, block_frames| RenderOptions {
        tail_secs,
        auto_tail,
        block_frames,
        ..RenderOptions::default()
    };

    let whole = render_all(&rig, &options(30.0, false, 1_024));
    assert_eq!(whole.frames(), 96_000 + 30 * 48_000);
    let silence = db_to_gain(TAIL_SILENCE_DB);
    let frames = whole.samples().as_chunks::<2>().0;
    let loud = |frame: &[f32; 2]| frame[0].abs() >= silence || frame[1].abs() >= silence;
    let end = frames.iter().rposition(loud).unwrap() + 1;
    // The sound outlasts the bar by over a second, and the numbers that
    // are not zero outlast the sound by most of the thirty seconds.
    assert!(end > 96_000 + 48_000 + 2_400, "the sound ends on {end}");
    assert!(end < 96_000 + 8 * 48_000, "the sound ends on {end}");
    let last_number = whole.samples().iter().rposition(|sample| *sample != 0.0);
    assert!(last_number.unwrap() / 2 > 96_000 + 20 * 48_000);

    // A second past the end of the sound is limit enough.
    let tight = (end - 96_000) as f32 / 48_000.0 + 1.0;
    for tail_secs in [tight, 10.0, 30.0, 120.0] {
        for block_frames in [64, 1_024, 100_000] {
            let audio = render_all(&rig, &options(tail_secs, true, block_frames));
            assert_eq!(
                audio.frames(),
                end,
                "a limit of {tail_secs} s in blocks of {block_frames}"
            );
            assert!(audio.samples() == &whole.samples()[..end * 2]);
        }
    }

    // A limit that runs out while the reverb still sounds keeps all of it.
    let cut = render_all(&rig, &options(1.0, true, 1_024));
    assert_eq!(cut.frames(), 96_000 + 48_000);
}

#[test]
fn an_automatic_tail_is_no_longer_than_the_tail_asked_for() {
    let rig = echoing();
    let options = RenderOptions {
        tail_secs: 0.25,
        auto_tail: true,
        ..RenderOptions::default()
    };
    assert_eq!(render_all(&rig, &options).frames(), 96_000 + 12_000);
    let none = RenderOptions {
        tail_secs: 0.0,
        auto_tail: true,
        ..RenderOptions::default()
    };
    assert_eq!(render_all(&rig, &none).frames(), 96_000);
}

#[test]
fn an_automatic_tail_of_a_dry_project_ends_with_its_last_sound() {
    // A second of sound that starts on the last step of the bar, frame
    // 90000.
    let mut rig = Rig::new();
    let ring = rig.channel(level(RATE, 0.25, 1.0));
    rig.steps(ring, &[15]);
    let options = RenderOptions {
        tail_secs: 10.0,
        auto_tail: true,
        ..RenderOptions::default()
    };
    assert_eq!(render_all(&rig, &options).frames(), 90_000 + 48_000);

    // With nothing left to ring there is no tail at all, and a silent
    // project is as long as its pattern.
    rig.pattern_mut(rig.first_pattern()).lanes.clear();
    rig.steps(ring, &[0]);
    assert_eq!(render_all(&rig, &options).frames(), 96_000);
    assert_eq!(render_all(&Rig::new(), &options).frames(), 96_000);
}

#[test]
fn an_automatic_tail_waits_for_an_echo_that_is_still_to_come() {
    // A click on the last step and one echo of it a whole note later: two
    // seconds of silence lie between the two.
    let mut rig = Rig::new();
    let track = rig.track();
    let click = rig.channel_on(impulse(RATE), track);
    rig.steps(click, &[15]);
    rig.effect(
        track,
        EffectParams::Delay(DelayParams {
            sync: false,
            time_ms: 2_000.0,
            feedback: 0.0,
            mix: 0.5,
            ..DelayParams::default()
        }),
    );
    let options = RenderOptions {
        tail_secs: 30.0,
        auto_tail: true,
        ..RenderOptions::default()
    };
    let audio = render_all(&rig, &options);
    let frames = sounding_frames(audio.samples());
    assert_eq!(frames[0], 90_000);
    let echo = *frames.last().unwrap();
    assert!(
        echo.abs_diff(90_000 + 96_000) < 200,
        "the echo is on {echo}"
    );
    assert!(audio.frames() < 90_000 + 96_000 + 2_000);
}

#[test]
fn a_render_starts_at_the_levels_of_the_project() {
    // The master is 6 dB down and panned, and so are the track and the
    // channel under it. The very first frame has to come out that way.
    let mut rig = Rig::new();
    let track = rig.track();
    let channel = rig.channel_on(level(RATE, 1.0, 1.0), track);
    rig.steps(channel, &[0]);
    rig.track_mut(TrackId::MASTER).volume = 0.5;
    rig.track_mut(TrackId::MASTER).pan = -0.5;
    rig.track_mut(track).volume = 0.5;
    rig.channel_mut(channel).volume = 0.5;
    rig.channel_mut(channel).pan = 0.5;

    let audio = render_all(&rig, &RenderOptions::default());
    let frames = audio.samples().as_chunks::<2>().0;
    // A full-scale sample at 0.125, with the channel's pan halving the left
    // side and the master's pan halving the right.
    assert_eq!(frames[0], [0.0625, 0.0625]);
    assert!(
        frames[..RATE as usize]
            .iter()
            .all(|frame| *frame == frames[0])
    );

    // A muted master is silent from the first frame.
    rig.track_mut(TrackId::MASTER).muted = true;
    let audio = render_all(&rig, &RenderOptions::default());
    assert!(audio.samples().iter().all(|sample| *sample == 0.0));
}

#[test]
fn a_note_held_past_the_end_of_a_render_ends_on_its_own_tick_in_the_tail() {
    // The note starts on the last beat of the bar and is held for two
    // beats, so it ends one beat, 24000 frames, into the tail.
    let mut rig = Rig::new();
    let channel = rig.channel(level(RATE, 0.5, 4.0));
    rig.sampler_mut(channel).envelope = Some(Envelope {
        attack_ms: 0.0,
        decay_ms: 0.0,
        sustain: 1.0,
        release_ms: 1.0,
    });
    rig.note(channel, 2_880, 1_920);
    let options = RenderOptions {
        tail_secs: 1.0,
        ..RenderOptions::default()
    };
    let audio = left(render_all(&rig, &options).samples());
    let held: Vec<usize> = (0..audio.len())
        .filter(|&frame| audio[frame] == 0.5)
        .collect();
    assert_eq!(held, (72_000..=120_000).collect::<Vec<_>>());
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
