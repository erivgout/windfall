//! Streamed renders and stems: what each stream holds, that the streams
//! line up with the mix, and that they are the same bits every time.

use windfall_core::AudioBuffer;
use windfall_dsp::{CompressorParams, DelayParams, EqParams, ReverbParams};
use windfall_engine::{
    RenderOptions, Stem, StemMode, StemOptions, Streamed, render, render_stems, render_streaming,
    stems,
};
use windfall_ipc::PlayMode;
use windfall_project::{EffectParams, Send, TrackId};

use crate::rendering::{crowd, echoing, scored, song, studio, tape};
use crate::support::{
    Rig, frame_of_tick, idle_limiter, left, limiter, peak, plain_synth, rms, sine, sounding_frames,
};

const RATE: u32 = 48_000;

/// The streams of a stem render, gathered whole.
struct Streams {
    list: Vec<Stem>,
    /// The audio of each stream, interleaved stereo.
    audio: Vec<Vec<f32>>,
    outcome: Streamed,
}

impl Streams {
    /// The audio of the stream of this track, or of the mix for `None`.
    fn of(&self, track: Option<TrackId>) -> &[f32] {
        let found = self.list.iter().position(|stem| stem.track == track);
        &self.audio[found.expect("the stream was rendered")]
    }
}

fn stem_options(mode: StemMode) -> StemOptions {
    StemOptions {
        mode,
        tracks: None,
        include_mix: true,
        numbered: false,
    }
}

fn render_streams(rig: &Rig, options: &RenderOptions, stem_options: &StemOptions) -> Streams {
    let list = stems(&rig.project, stem_options).unwrap();
    let mut audio = vec![Vec::new(); list.len()];
    let outcome = render_stems(
        &rig.project,
        &rig.pool,
        options,
        stem_options,
        &mut |stream, block| {
            assert_eq!(block.len() % 2, 0);
            audio[stream].extend_from_slice(block);
            true
        },
        &mut |_| true,
    )
    .unwrap();
    Streams {
        list,
        audio,
        outcome,
    }
}

fn same_bits(a: &[f32], b: &[f32]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.to_bits() == b.to_bits())
}

/// The largest difference between two signals of one length.
fn apart(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    let differences = a.iter().zip(b).map(|(a, b)| (a - b).abs());
    differences.fold(0.0, f32::max)
}

/// The streams of these tracks added up, sample by sample.
fn sum(streams: &Streams, tracks: &[TrackId]) -> Vec<f32> {
    let mut total = vec![0.0; streams.of(None).len()];
    for &track in tracks {
        for (total, sample) in total.iter_mut().zip(streams.of(Some(track))) {
            *total += sample;
        }
    }
    total
}

fn song_options() -> RenderOptions {
    RenderOptions {
        mode: PlayMode::Song,
        tail_secs: 0.5,
        ..RenderOptions::default()
    }
}

/// Drums, a bass and keys, each on a track of its own that plays straight
/// into a master with nothing on it. The tracks have effects, faders and
/// pans of their own, a limiter and a synth put two of them behind the
/// third, and the bass is an audio clip.
fn band() -> (Rig, [TrackId; 3]) {
    let mut rig = Rig::new();
    rig.project.settings.tempo_bpm = 140.0;
    rig.project.settings.swing = 0.15;
    let (drums, bass, keys) = (rig.track(), rig.track(), rig.track());
    for (track, name) in [(drums, "Drums"), (bass, "Bass"), (keys, "Keys")] {
        rig.track_mut(track).name = name.to_owned();
    }
    rig.track_mut(drums).volume = 0.8;
    rig.track_mut(drums).pan = -0.3;
    rig.track_mut(bass).volume = 1.1;
    rig.track_mut(keys).volume = 0.6;
    rig.track_mut(keys).pan = 0.4;

    let kick = rig.channel_on(sine(44_100, 70.0, 0.15), drums);
    rig.steps(kick, &[0, 4, 8, 12]);
    let hat = rig.channel_on(sine(RATE, 6_000.0, 0.05), drums);
    rig.sampler_mut(hat).cut_self = true;
    rig.steps(hat, &[0, 2, 3, 4, 6, 8, 10, 11, 12, 14]);
    rig.effect(drums, EffectParams::Eq(EqParams::default()));
    rig.effect(drums, EffectParams::Compressor(CompressorParams::default()));

    let synth = rig.synth_on(plain_synth(), keys);
    for (index, key) in [57, 60, 64, 67].into_iter().enumerate() {
        rig.note(synth, index as u32 * 960, 700).key = key;
    }
    rig.effect(keys, limiter(-9.0));

    let (lane, tape) = (rig.playlist_track(), rig.playlist_track());
    let pattern = rig.first_pattern();
    rig.clip(lane, pattern, 0, 3_840);
    rig.clip(lane, pattern, 3_840, 1_920);
    let line = rig.audio_clip(tape, sine(32_000, 55.0, 1.5), bass, 480, 4_000);
    rig.audio_mut(line, |audio| {
        audio.gain = 0.5;
        audio.fade_in = 100;
        audio.fade_out = 600;
    });
    (rig, [drums, bass, keys])
}

#[test]
fn a_streamed_render_is_the_render_block_by_block() {
    let rigs = [
        ("song", song()),
        ("crowd", crowd()),
        ("studio", studio()),
        ("tape", tape()),
        ("scored", scored()),
        ("echoing", echoing()),
    ];
    for (name, rig) in rigs {
        for mode in [PlayMode::Pattern, PlayMode::Song] {
            for auto_tail in [false, true] {
                let options = |block_frames| RenderOptions {
                    mode,
                    pattern_loops: 2,
                    tail_secs: if auto_tail { 4.0 } else { 0.25 },
                    auto_tail,
                    block_frames,
                    ..RenderOptions::default()
                };
                let whole = render(&rig.project, &rig.pool, &options(1_024), &mut |_| true);
                for block_frames in [7, 64, 1_000, 1_024, 100_000] {
                    let label = format!(
                        "{name} in {mode:?} mode, automatic tail {auto_tail}, blocks of {block_frames}"
                    );
                    let mut streamed = Vec::new();
                    let mut calls = 0;
                    let outcome = render_streaming(
                        &rig.project,
                        &rig.pool,
                        &options(block_frames),
                        &mut |block| {
                            streamed.extend_from_slice(block);
                            true
                        },
                        &mut |fraction| {
                            calls += 1;
                            assert!(fraction > 0.0 && fraction <= 1.0);
                            true
                        },
                    );
                    assert!(outcome.completed, "{label}");
                    assert!(calls > 0, "{label}");
                    assert_eq!(outcome.frames as usize, whole.frames(), "{label}");
                    assert_eq!(outcome.dropped_clips, 0, "{label}");
                    assert!(same_bits(&streamed, whole.samples()), "{label}");
                }
            }
        }
    }
}

#[test]
fn track_output_stems_add_up_to_the_mix_when_nothing_comes_after_them() {
    let (rig, tracks) = band();
    let options = song_options();
    let streams = render_streams(&rig, &options, &stem_options(StemMode::TrackOutputs));
    let names: Vec<&str> = streams.list.iter().map(|stem| stem.name.as_str()).collect();
    assert_eq!(names, ["Mix", "Drums", "Bass", "Keys"]);

    // The mix is the render, to the bit, and every stem is as long.
    let whole = render(&rig.project, &rig.pool, &options, &mut |_| true);
    let mix = streams.of(None);
    assert!(same_bits(mix, whole.samples()));
    assert_eq!(streams.outcome.frames as usize, whole.frames());
    for track in tracks {
        assert_eq!(streams.of(Some(track)).len(), mix.len());
        assert!(rms(streams.of(Some(track))) > 0.01);
    }

    // The keys are 252 frames behind the drums inside the engine, and
    // only stems that are lined up to the frame add up like this: to the
    // last bit or two of a sum of three.
    let total = sum(&streams, &tracks);
    assert!(peak(mix) > 0.3);
    let off = apart(&total, mix);
    assert!(
        off <= 4.0 * f32::EPSILON,
        "the stems are off the mix by {off}"
    );

    // Leave one out and they are far off.
    assert!(apart(&sum(&streams, &tracks[..2]), mix) > 0.05);
}

#[test]
fn every_stem_starts_where_the_song_starts_whatever_is_behind() {
    // A click on the second beat of each of three tracks. One track has a
    // limiter that looks 5 ms ahead, one has one that looks 20 ms ahead,
    // and the master has one of its own.
    let click = || AudioBuffer::from_interleaved(RATE, 1, vec![0.25]);
    let mut rig = Rig::new();
    let (plain, near, far) = (rig.track(), rig.track(), rig.track());
    for track in [plain, near, far] {
        let channel = rig.channel_on(click(), track);
        rig.steps(channel, &[4]);
    }
    rig.effect(near, idle_limiter(5.0));
    rig.effect(far, idle_limiter(20.0));
    rig.effect(TrackId::MASTER, idle_limiter(10.0));
    let beat = frame_of_tick(960, 12_000, RATE);

    for (mode, block_frames) in [
        (StemMode::TrackOutputs, 1_024),
        (StemMode::TrackOutputs, 100),
        (StemMode::ToMaster, 1_024),
    ] {
        let options = RenderOptions {
            block_frames,
            ..RenderOptions::default()
        };
        let streams = render_streams(&rig, &options, &stem_options(mode));
        assert_eq!(streams.outcome.frames, 96_000);
        for track in [None, Some(plain), Some(near), Some(far)] {
            let audio = left(streams.of(track));
            assert_eq!(audio.len(), 96_000, "{mode:?} {track:?}");
            // The click is on its frame and nowhere else. A limiter with
            // nothing to do passes it on to within its own rounding.
            let level = if track.is_none() { 0.75 } else { 0.25 };
            assert!((audio[beat] - level).abs() < 1e-5, "{mode:?} {track:?}");
            let around = sounding_frames(streams.of(track));
            assert!(
                around.iter().all(|&frame| frame == beat),
                "{mode:?} {track:?}: sound on frames {:?}",
                &around[..around.len().min(8)]
            );
        }
    }
}

/// Two dry tracks that play into a bus with a reverb on it.
fn hall() -> (Rig, [TrackId; 3]) {
    let mut rig = Rig::new();
    let (lead, pad, bus) = (rig.track(), rig.track(), rig.track());
    for (track, name) in [(lead, "Lead"), (pad, "Pad"), (bus, "Hall")] {
        rig.track_mut(track).name = name.to_owned();
    }
    rig.track_mut(lead).output = Some(bus);
    rig.track_mut(pad).output = Some(bus);
    rig.track_mut(pad).volume = 0.7;
    let stab = rig.channel_on(sine(RATE, 440.0, 0.1), lead);
    rig.steps(stab, &[0, 6]);
    let wash = rig.channel_on(sine(44_100, 220.0, 0.2), pad);
    rig.steps(wash, &[3, 9]);
    rig.effect(
        bus,
        EffectParams::Reverb(ReverbParams {
            decay_s: 0.6,
            mix: 0.4,
            ..ReverbParams::default()
        }),
    );
    (rig, [lead, pad, bus])
}

#[test]
fn a_bus_is_a_stem_of_its_own_and_its_tracks_stay_dry() {
    let (rig, [lead, pad, bus]) = hall();
    let options = RenderOptions {
        tail_secs: 1.0,
        ..RenderOptions::default()
    };
    let streams = render_streams(&rig, &options, &stem_options(StemMode::TrackOutputs));
    let mix = streams.of(None);

    // Everything goes through the bus, so the bus is the mix.
    assert!(apart(streams.of(Some(bus)), mix) <= f32::EPSILON);
    // The lead's stem is its two stabs and nothing after them: no reverb.
    let stab = RATE as usize / 10;
    let second = frame_of_tick(6 * 240, 12_000, RATE);
    let dry = sounding_frames(streams.of(Some(lead)));
    assert_eq!(dry[0], 1);
    assert!(
        dry.iter()
            .all(|&frame| frame < stab || (second..second + stab).contains(&frame))
    );
    // The mix rings on where the dry tracks are silent, so the dry stems
    // do not add up to it.
    let ringing = (second + stab + 4_800) * 2;
    assert!(rms(&mix[ringing..ringing + 9_600]) > 1e-3);
    assert!(apart(&sum(&streams, &[lead, pad]), mix) > 0.01);
}

#[test]
fn to_master_stems_carry_what_the_bus_does_to_them_and_add_up_to_the_mix() {
    let (rig, [lead, pad, bus]) = hall();
    let options = RenderOptions {
        tail_secs: 1.0,
        ..RenderOptions::default()
    };
    let streams = render_streams(&rig, &options, &stem_options(StemMode::ToMaster));
    // The bus has nothing playing straight into it, so it is not a stem
    // of this kind unless it is asked for.
    let names: Vec<&str> = streams.list.iter().map(|stem| stem.name.as_str()).collect();
    assert_eq!(names, ["Mix", "Lead", "Pad"]);
    let mix = streams.of(None);
    let whole = render(&rig.project, &rig.pool, &options, &mut |_| true);
    assert!(same_bits(mix, whole.samples()));

    // The lead's stem rings on after its second stab: it has its share of
    // the reverb.
    let stab = RATE as usize / 10;
    let ringing = (frame_of_tick(6 * 240, 12_000, RATE) + stab + 4_800) * 2;
    assert!(rms(&streams.of(Some(lead))[ringing..ringing + 9_600]) > 1e-4);
    // A reverb is linear, so the two stems add up to the mix, to within
    // the rounding of a reverb's worth of sums.
    let off = apart(&sum(&streams, &[lead, pad]), mix);
    assert!(peak(mix) > 0.2);
    assert!(off < 2e-6, "the stems are off the mix by {off}");

    // Asked for, the bus is a stem too, and a silent one.
    let asked = StemOptions {
        tracks: Some(vec![bus]),
        include_mix: false,
        ..stem_options(StemMode::ToMaster)
    };
    let alone = render_streams(&rig, &options, &asked);
    assert_eq!(alone.audio.len(), 1);
    assert_eq!(alone.audio[0].len(), mix.len());
    assert_eq!(peak(&alone.audio[0]), 0.0);
}

/// A vocal that sends to an echo, and a bass that does not. All three
/// tracks play into the master.
fn sending() -> (Rig, [TrackId; 3]) {
    let mut rig = Rig::new();
    let (vocal, bass, echo) = (rig.track(), rig.track(), rig.track());
    rig.track_mut(vocal).sends.push(Send {
        target: echo,
        gain: 0.6,
    });
    let word = rig.channel_on(sine(RATE, 660.0, 0.08), vocal);
    rig.steps(word, &[0, 8]);
    let note = rig.channel_on(sine(RATE, 55.0, 0.3), bass);
    rig.steps(note, &[4, 12]);
    rig.effect(
        echo,
        EffectParams::Delay(DelayParams {
            feedback: 0.4,
            mix: 1.0,
            ..DelayParams::default()
        }),
    );
    (rig, [vocal, bass, echo])
}

#[test]
fn a_send_is_heard_in_the_stem_of_the_track_it_goes_to_or_comes_from() {
    let (rig, [vocal, bass, echo]) = sending();
    let options = RenderOptions {
        tail_secs: 2.0,
        auto_tail: true,
        ..RenderOptions::default()
    };
    let whole = render(&rig.project, &rig.pool, &options, &mut |_| true);
    assert!(whole.frames() > 96_000 && whole.frames() < 96_000 * 2);

    // As track outputs the echo is a stem of its own: the vocal's stem is
    // dry, the echo's is wet, and with the bass they are the mix.
    let outputs = render_streams(&rig, &options, &stem_options(StemMode::TrackOutputs));
    let mix = outputs.of(None);
    assert!(same_bits(mix, whole.samples()));
    assert!(rms(outputs.of(Some(echo))) > 1e-3);
    let off = apart(&sum(&outputs, &[vocal, bass, echo]), mix);
    assert!(off <= 4.0 * f32::EPSILON, "off by {off}");
    let word = RATE as usize * 8 / 100;
    let dry = sounding_frames(outputs.of(Some(vocal)));
    assert!(dry.iter().all(|&frame| frame % 48_000 < word));

    // To the master, the vocal's stem has its echo in it, and the echo
    // track, which nothing plays straight into, is no stem at all.
    let to_master = render_streams(&rig, &options, &stem_options(StemMode::ToMaster));
    assert_eq!(to_master.list.len(), 3);
    assert!(same_bits(to_master.of(None), whole.samples()));
    let wet = to_master.of(Some(vocal));
    assert_eq!(wet.len(), mix.len());
    let echoed = sum(&outputs, &[vocal, echo]);
    assert!(apart(wet, &echoed) < 1e-6);
    let off = apart(&sum(&to_master, &[vocal, bass]), mix);
    assert!(off < 1e-6, "off by {off}");
}

#[test]
fn a_part_of_the_tracks_gives_the_same_stems_as_all_of_them() {
    let (rig, [drums, bass, keys]) = band();
    let options = song_options();
    for mode in [StemMode::TrackOutputs, StemMode::ToMaster] {
        let all = render_streams(&rig, &options, &stem_options(mode));
        // Asked for back to front, and without the mix.
        let some = StemOptions {
            tracks: Some(vec![keys, drums]),
            include_mix: false,
            ..stem_options(mode)
        };
        let part = render_streams(&rig, &options, &some);
        let tracks: Vec<_> = part.list.iter().map(|stem| stem.track).collect();
        assert_eq!(tracks, [Some(drums), Some(keys)], "{mode:?}");
        for track in [drums, keys] {
            assert!(
                same_bits(part.of(Some(track)), all.of(Some(track))),
                "{mode:?}"
            );
        }
        assert_eq!(part.outcome, all.outcome, "{mode:?}");

        // The mix alone is a render.
        let only_mix = StemOptions {
            tracks: Some(Vec::new()),
            ..stem_options(mode)
        };
        let mix = render_streams(&rig, &options, &only_mix);
        assert_eq!(mix.audio.len(), 1);
        assert!(same_bits(&mix.audio[0], all.of(None)), "{mode:?}");
        let _ = bass;
    }
}

#[test]
fn the_block_size_does_not_change_a_stem() {
    let rigs = [("studio", studio()), ("tape", tape()), ("scored", scored())];
    for (name, rig) in rigs {
        for mode in [StemMode::TrackOutputs, StemMode::ToMaster] {
            let options = |block_frames| RenderOptions {
                mode: PlayMode::Song,
                tail_secs: 3.0,
                auto_tail: true,
                block_frames,
                ..RenderOptions::default()
            };
            let reference = render_streams(&rig, &options(1_024), &stem_options(mode));
            let whole = render(&rig.project, &rig.pool, &options(1_024), &mut |_| true);
            assert!(same_bits(reference.of(None), whole.samples()), "{name}");
            assert!(reference.audio.len() >= 3, "{name} {mode:?}");
            for audio in &reference.audio {
                assert_eq!(audio.len(), whole.samples().len(), "{name} {mode:?}");
            }
            // Twice the same, and the same in other blocks.
            for block_frames in [1_024, 7, 64, 333, 100_000] {
                let other = render_streams(&rig, &options(block_frames), &stem_options(mode));
                assert_eq!(other.outcome, reference.outcome);
                for (index, audio) in other.audio.iter().enumerate() {
                    assert!(
                        same_bits(audio, &reference.audio[index]),
                        "{name} {mode:?}: {} in blocks of {block_frames}",
                        reference.list[index].name
                    );
                }
            }
        }
    }
}

#[test]
fn a_muted_track_is_a_silent_stem_and_a_soloed_one_the_only_one_heard() {
    let (mut rig, [drums, bass, keys]) = band();
    rig.track_mut(bass).muted = true;
    let options = song_options();
    for mode in [StemMode::TrackOutputs, StemMode::ToMaster] {
        let streams = render_streams(&rig, &options, &stem_options(mode));
        assert_eq!(peak(streams.of(Some(bass))), 0.0, "{mode:?}");
        assert!(peak(streams.of(Some(drums))) > 0.1, "{mode:?}");
        assert_eq!(streams.of(Some(bass)).len(), streams.of(None).len());
    }
    rig.track_mut(bass).muted = false;
    rig.track_mut(keys).solo = true;
    let streams = render_streams(&rig, &options, &stem_options(StemMode::TrackOutputs));
    assert_eq!(peak(streams.of(Some(drums))), 0.0);
    assert!(apart(streams.of(Some(keys)), streams.of(None)) <= f32::EPSILON);
}

#[test]
fn a_render_that_is_told_to_stop_stops() {
    let (rig, _) = band();
    let options = RenderOptions {
        block_frames: 1_000,
        ..song_options()
    };
    for mode in [StemMode::TrackOutputs, StemMode::ToMaster] {
        // Stopped by the progress callback after three blocks.
        let mut blocks = 0;
        let mut frames = 0;
        let outcome = render_stems(
            &rig.project,
            &rig.pool,
            &options,
            &stem_options(mode),
            &mut |_, block| {
                frames += block.len() / 2;
                true
            },
            &mut |_| {
                blocks += 1;
                blocks < 3
            },
        )
        .unwrap();
        assert!(!outcome.completed, "{mode:?}");
        assert_eq!(blocks, 3, "{mode:?}");
        assert!(frames <= 4 * 3_000, "{mode:?}");

        // Stopped by the sink, which is not called again.
        let mut calls = 0;
        let outcome = render_stems(
            &rig.project,
            &rig.pool,
            &options,
            &stem_options(mode),
            &mut |_, _| {
                calls += 1;
                calls < 5
            },
            &mut |_| true,
        )
        .unwrap();
        assert!(!outcome.completed, "{mode:?}");
        assert_eq!(calls, 5, "{mode:?}");
    }

    let mut calls = 0;
    let outcome = render_streaming(
        &rig.project,
        &rig.pool,
        &options,
        &mut |_| {
            calls += 1;
            false
        },
        &mut |_| true,
    );
    assert!(!outcome.completed);
    assert_eq!(calls, 1);
}

#[test]
fn progress_runs_from_nothing_to_all_of_it_over_every_pass() {
    let (rig, _) = band();
    for mode in [StemMode::TrackOutputs, StemMode::ToMaster] {
        let mut seen = Vec::new();
        render_stems(
            &rig.project,
            &rig.pool,
            &song_options(),
            &stem_options(mode),
            &mut |_, _| true,
            &mut |fraction| {
                seen.push(fraction);
                true
            },
        )
        .unwrap();
        assert!(seen.len() > 10, "{mode:?}");
        assert!(seen.windows(2).all(|pair| pair[0] <= pair[1]), "{mode:?}");
        assert!(seen[0] > 0.0 && seen[0] < 0.1, "{mode:?}");
        assert_eq!(seen.last(), Some(&1.0), "{mode:?}");
    }
}
