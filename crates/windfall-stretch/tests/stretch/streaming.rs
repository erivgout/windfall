//! The contract of the streaming interface: what it takes, when it puts
//! it out, and that nothing a host does to it makes it misbehave.

use windfall_stretch::{MAX_TIME_RATIO, MIN_TIME_RATIO, Quality, Stretcher};

use crate::support::{
    RATE, Rng, assert_finite, chord, clicks, db, drum_loop, mean, noise, peak, rms, run, sine,
    square, steepest, stretcher, tone,
};

/// A stereo test signal with notes, hits and a gap of silence in it.
fn music(frames: usize) -> Vec<Vec<f32>> {
    let notes = chord(&[220.0, 330.0, 554.37, 880.0], 0.5, frames, RATE);
    let drums = drum_loop(128.0, 64, RATE);
    let hiss = noise(9, 0.05, frames);
    let mut left = Vec::with_capacity(frames);
    let mut right = Vec::with_capacity(frames);
    for n in 0..frames {
        let quiet = (frames / 2..frames / 2 + 12_000).contains(&n);
        let hit = drums[n % drums.len()] * 0.4;
        let (l, r) = (
            notes[n] * 0.8 + hit + hiss[n],
            notes[n] * 0.3 + hit - hiss[n],
        );
        left.push(if quiet { 0.0 } else { l });
        right.push(if quiet { 0.0 } else { r });
    }
    vec![left, right]
}

#[test]
fn input_is_taken_at_one_over_the_time_ratio() {
    for ratio in [0.25, 0.5, 0.8, 1.0, 1.000_01, 1.37, 2.0, 4.0] {
        let mut stretcher = stretcher(1, ratio, 0.0, Quality::Fast);
        let input = [vec![0.0_f32; 16]];
        let mut rng = Rng::new(3);
        let (mut produced, mut taken) = (0_usize, 0_usize);
        let mut output = vec![0.0_f32; 4_096];
        while produced < 400_000 {
            let frames = rng.between(1, 4_096);
            let needed = stretcher.input_frames_needed(frames);
            let feed: [&[f32]; 1] = [&input[0]];
            let got = stretcher.process(&feed, &mut [&mut output[..frames]]);
            assert_eq!(got, needed, "ratio {ratio}");
            produced += frames;
            taken += got;
            // The clock carries the fraction, so the count never strays.
            let exact = produced as f64 / ratio;
            assert!(
                (taken as f64 - exact).abs() <= 0.5 + 1e-6,
                "ratio {ratio}: {taken} of {exact}"
            );
        }
    }
}

#[test]
fn the_time_ratio_and_pitch_are_forced_into_range() {
    let mut stretcher = Stretcher::new(2, RATE);
    assert_eq!((stretcher.channels(), stretcher.sample_rate()), (2, RATE));
    assert_eq!(stretcher.quality(), Quality::Standard);
    assert_eq!(
        (stretcher.time_ratio(), stretcher.pitch_semitones()),
        (1.0, 0.0)
    );
    for (set, expected) in [
        (0.0, MIN_TIME_RATIO),
        (-3.0, MIN_TIME_RATIO),
        (1e9, MAX_TIME_RATIO),
        (f64::NAN, 1.0),
        (f64::INFINITY, 1.0),
        (1.5, 1.5),
    ] {
        stretcher.set_time_ratio(set);
        assert_eq!(stretcher.time_ratio(), expected, "{set}");
    }
    for (set, expected) in [
        (100.0, 24.0),
        (-100.0, -24.0),
        (f64::NAN, 0.0),
        (-7.5, -7.5),
    ] {
        stretcher.set_pitch_semitones(set);
        assert_eq!(stretcher.pitch_semitones(), expected, "{set}");
    }
    assert!(!stretcher.formant_preservation());
    stretcher.set_formant_preservation(true);
    assert!(stretcher.formant_preservation());
}

/// Plays `input` with changes of ratio and pitch at fixed output frames,
/// asking for `sizes` frames at a time.
fn played(input: &[Vec<f32>], quality: Quality, sizes: &[usize]) -> Vec<Vec<f32>> {
    let mut stretcher = stretcher(input.len(), 1.3, 0.0, quality);
    let mut position = 0;
    let mut output: Vec<Vec<f32>> = vec![Vec::new(); input.len()];
    let steps: [(usize, f64, f64, bool); 5] = [
        (30_000, 1.3, 0.0, false),
        (17_001, 0.7, 4.0, false),
        (25_013, 2.5, -3.0, true),
        (9_999, 1.0, 0.0, true),
        (20_000, 1.0, 12.0, false),
    ];
    for (frames, ratio, pitch, formants) in steps {
        stretcher.set_time_ratio(ratio);
        stretcher.set_pitch_semitones(pitch);
        stretcher.set_formant_preservation(formants);
        let part = run(&mut stretcher, input, &mut position, frames, sizes);
        for (output, part) in output.iter_mut().zip(part) {
            output.extend(part);
        }
    }
    output
}

#[test]
fn the_output_does_not_depend_on_how_it_is_divided() {
    let input = music(160_000);
    for quality in Quality::ALL {
        let whole = played(&input, quality, &[1 << 20]);
        assert!(rms(&whole[0]) > 0.05 && rms(&whole[1]) > 0.02);
        let mut rng = Rng::new(11);
        let ragged: Vec<usize> = (0..97).map(|_| rng.between(1, 3_000)).collect();
        for sizes in [&[512][..], &[64], &[1, 7, 480, 33, 2_048, 129], &ragged] {
            assert!(
                played(&input, quality, sizes) == whole,
                "{quality:?} in blocks of {sizes:?}"
            );
        }
    }
}

#[test]
fn the_same_input_gives_the_same_output_every_time() {
    let input = music(120_000);
    // A ratio past 2 draws on the stretcher's random numbers.
    for (ratio, pitch) in [(1.2, 2.0), (3.0, 0.0)] {
        let mut first = stretcher(2, ratio, pitch, Quality::Standard);
        let once = run(&mut first, &input, &mut 0, 60_000, &[512]);
        let mut second = stretcher(2, ratio, pitch, Quality::Standard);
        assert!(run(&mut second, &input, &mut 0, 60_000, &[512]) == once);
        // A reset stretcher is a new one.
        first.reset();
        assert!(run(&mut first, &input, &mut 0, 60_000, &[512]) == once);
    }
}

#[test]
fn unity_settings_pass_the_input_through_untouched() {
    let input = music(150_000);
    for quality in Quality::ALL {
        let mut stretcher = stretcher(2, 1.0, 0.0, quality);
        let latency = stretcher.latency();
        let delay = latency.input + latency.output;
        assert_eq!(latency.output_frames(1.0), delay as f64);
        let output = run(&mut stretcher, &input, &mut 0, 140_000, &[512]);
        for (input, output) in input.iter().zip(&output) {
            assert!(output[..delay].iter().all(|sample| sample.abs() < 1e-5));
            let worst = input
                .iter()
                .zip(&output[delay..])
                .fold(0.0_f32, |worst, (a, b)| worst.max((a - b).abs()));
            // -90 dB against a signal that peaks near full scale.
            assert!(worst < 3e-5, "{quality:?}: off by {worst:e}");
        }
    }
}

#[test]
fn the_latency_says_where_the_input_comes_out() {
    let period = 24_000;
    let input = vec![clicks(period, period, 8 * period, RATE)];
    for quality in Quality::ALL {
        for (ratio, pitch) in [(0.5, 0.0), (0.8, 0.0), (1.0, 5.0), (1.25, 0.0), (2.0, -4.0)] {
            let mut stretcher = stretcher(1, ratio, pitch, quality);
            let lead = stretcher.latency().output_frames(ratio);
            let frames = (7.0 * period as f64 * ratio) as usize;
            let output = run(&mut stretcher, &input, &mut 0, frames, &[512]).remove(0);
            for click in 1..6 {
                let expected = lead + (click * period) as f64 * ratio;
                let from = (expected - period as f64 * ratio / 2.0) as usize;
                let to = (expected + period as f64 * ratio / 2.0) as usize;
                let loudest = (from..to)
                    .max_by(|a, b| output[*a].abs().total_cmp(&output[*b].abs()))
                    .unwrap_or(from);
                // A click is a millisecond of noise; its loudest sample
                // is within that of its start.
                let off = loudest as f64 - expected;
                assert!(
                    (-24.0..72.0).contains(&off),
                    "{quality:?} {ratio} {pitch}: {off} frames"
                );
            }
        }
    }
}

/// Stretches `input` from its start and, separately, from a seek to the
/// frame that is `at` output frames in, and returns both from there on,
/// `frames` frames each.
fn continuous_and_sought(
    input: &[Vec<f32>],
    ratio: f64,
    pitch: f64,
    quality: Quality,
    at: usize,
    frames: usize,
) -> (Vec<Vec<f32>>, Vec<Vec<f32>>) {
    let mut continuous = stretcher(input.len(), ratio, pitch, quality);
    let mut position = 0;
    run(&mut continuous, input, &mut position, at, &[512]);
    let expected = run(&mut continuous, input, &mut position, frames, &[512]);

    // The input the continuous run had taken when it reached `at`.
    let mut sought = stretcher(input.len(), ratio, pitch, quality);
    let mut end = sought.input_frames_needed(at);
    let start = end.saturating_sub(sought.seek_frames());
    let pre_roll: Vec<&[f32]> = input.iter().map(|channel| &channel[start..end]).collect();
    sought.seek(&pre_roll);
    let found = run(&mut sought, input, &mut end, frames, &[512]);
    assert_eq!(end, position, "both runs end on the same input frame");
    (expected, found)
}

#[test]
fn a_seek_at_unity_continues_sample_for_sample() {
    let input = music(200_000);
    for quality in Quality::ALL {
        let (expected, found) = continuous_and_sought(&input, 1.0, 0.0, quality, 96_000, 30_000);
        for (expected, found) in expected.iter().zip(&found) {
            let worst = expected
                .iter()
                .zip(found)
                .fold(0.0_f32, |worst, (a, b)| worst.max((a - b).abs()));
            assert!(worst < 3e-5, "{quality:?}: off by {worst:e}");
        }
    }
}

#[test]
fn a_seek_sounds_like_playing_through() {
    // Steady notes: after a seek each one is there from the first frame,
    // at its level, and stays there. Phases differ, since they depend on
    // everything the stretcher has heard.
    let notes = [220.0, 440.0, 660.0, 1_000.0];
    let input = vec![chord(&notes, 0.8, 300_000, RATE)];
    for quality in Quality::ALL {
        for (ratio, pitch) in [(0.75, 0.0), (1.25, 0.0), (1.0, 4.0), (1.6, -3.0)] {
            let at = (2.0 * f64::from(RATE) * ratio) as usize;
            let (expected, found) =
                continuous_and_sought(&input, ratio, pitch, quality, at, 48_000);
            let factor = 2.0_f64.powf(pitch / 12.0);
            for window in [0..4_800, 4_800..9_600, 24_000..33_600] {
                let whole = db(rms(&found[0][window.clone()]) / rms(&expected[0][window.clone()]));
                assert!(
                    whole.abs() < 0.5,
                    "{quality:?} {ratio} {pitch} {window:?}: {whole:+.2} dB"
                );
            }
            for note in notes {
                let level = |signal: &[f32]| tone(&signal[..9_600], note * factor, RATE).0;
                let off = db(level(&found[0]) / level(&expected[0]));
                assert!(
                    off.abs() < 1.0,
                    "{quality:?} {ratio} {pitch} {note} Hz: {off:+.2} dB"
                );
            }
        }
    }
}

#[test]
fn a_seek_puts_a_hit_where_playing_through_puts_it() {
    let period = 12_000;
    let input = vec![clicks(1_000, period, 40 * period, RATE)];
    for quality in Quality::ALL {
        for ratio in [0.75, 1.0, 1.25, 1.5] {
            // The pre-roll ends 30 ms after a click, which the stretcher
            // has then taken and not yet put out: it must still come out
            // whole.
            let at = ((15 * period + 1_000 + 1_440) as f64 * ratio) as usize;
            let frames = (3.0 * period as f64 * ratio) as usize;
            let (expected, found) = continuous_and_sought(&input, ratio, 0.0, quality, at, frames);
            let loudest = |signal: &[f32]| {
                (0..signal.len())
                    .max_by(|a, b| signal[*a].abs().total_cmp(&signal[*b].abs()))
                    .unwrap_or(0) as i64
            };
            let hit = (period as f64 * ratio) as usize;
            for index in 0..2 {
                let window = index * hit..(index + 1) * hit;
                let (a, b) = (&expected[0][window.clone()], &found[0][window]);
                assert!(
                    (loudest(a) - loudest(b)).abs() <= 48,
                    "{quality:?} {ratio} hit {index}"
                );
                let level = db(rms(b) / rms(a));
                assert!(
                    level.abs() < 1.5,
                    "{quality:?} {ratio} hit {index}: {level:+.2} dB"
                );
            }
        }
    }
}

#[test]
fn a_seek_with_no_pre_roll_starts_from_silence() {
    let input = music(60_000);
    let mut fresh = stretcher(2, 1.4, 2.0, Quality::Standard);
    let expected = run(&mut fresh, &input, &mut 0, 40_000, &[512]);
    let mut sought = stretcher(2, 1.4, 2.0, Quality::Standard);
    run(&mut sought, &input, &mut 20_000, 9_000, &[512]);
    sought.seek(&[]);
    assert!(run(&mut sought, &input, &mut 0, 40_000, &[512]) == expected);
    // Pre-roll that is all silence is the same thing.
    let silence = vec![0.0_f32; sought.seek_frames() + 99];
    sought.seek(&[&silence, &silence]);
    assert!(run(&mut sought, &input, &mut 0, 40_000, &[512]) == expected);
}

#[test]
fn the_end_of_the_input_rings_out_and_goes_silent() {
    let frames = 48_000;
    let input = vec![sine(440.0, 0.5, frames, RATE)];
    for quality in Quality::ALL {
        for ratio in [0.5, 1.0, 1.7] {
            let mut stretcher = stretcher(1, ratio, 0.0, quality);
            let latency = stretcher.latency();
            let lead = latency.output_frames(ratio).ceil() as usize;
            let body = (frames as f64 * ratio) as usize;
            let mut position = 0;
            let mut output = run(&mut stretcher, &input, &mut position, lead + body, &[512]);
            // The stretcher is ahead of what is heard by its latency, so
            // by now it has been given all of the input and more.
            assert!(position >= frames);
            // The last frame of input stays in sight for a block and an
            // interval of input, and the last block that saw it lasts a
            // block of output.
            let block = (quality.block_seconds() * f64::from(RATE)) as usize;
            let interval = (quality.interval_seconds() * f64::from(RATE)) as usize;
            let rings = ((block + interval + 1) as f64 * ratio) as usize + block + interval + 2;
            let mut rest = vec![0.0_f32; rings + 4_800];
            stretcher.flush(&mut [&mut rest]);
            output[0].extend(&rest);

            let output = &output[0];
            let last = &output[lead + body - 2_400..lead + body - 480];
            let level = db(rms(last) / 0.3536);
            assert!(
                level.abs() < 1.0,
                "{quality:?} {ratio}: {level:+.2} dB at the end"
            );
            let after = &output[lead + body + 480..];
            assert!(
                peak(after) < 0.05,
                "{quality:?} {ratio}: {} after the end",
                peak(after)
            );
            let silent = &output[body + rings..];
            assert!(
                silent.iter().all(|sample| *sample == 0.0),
                "{quality:?} {ratio}"
            );
        }
    }
}

#[test]
fn flush_is_processing_with_no_input() {
    let input = music(30_000);
    let mut flushed = stretcher(2, 1.2, 1.0, Quality::Fast);
    let mut processed = stretcher(2, 1.2, 1.0, Quality::Fast);
    for stretcher in [&mut flushed, &mut processed] {
        run(stretcher, &input, &mut 0, 20_000, &[512]);
    }
    let mut a = vec![vec![0.0_f32; 9_000]; 2];
    let mut b = a.clone();
    let mut into: Vec<&mut [f32]> = a.iter_mut().map(Vec::as_mut_slice).collect();
    flushed.flush(&mut into);
    let mut into: Vec<&mut [f32]> = b.iter_mut().map(Vec::as_mut_slice).collect();
    assert_eq!(processed.process(&[], &mut into), 7_500);
    assert!(a == b && rms(&a[0]) > 0.01);
}

#[test]
fn channels_that_are_missing_are_silent() {
    let source = music(40_000);
    let mut stretcher = stretcher(2, 1.1, 0.0, Quality::Fast);
    // Only the left channel is given, and three outputs are asked for.
    let mut output = vec![vec![1.0_f32; 30_000]; 3];
    let mut into: Vec<&mut [f32]> = output.iter_mut().map(Vec::as_mut_slice).collect();
    stretcher.process(&[&source[0]], &mut into);
    assert!(rms(&output[0][15_000..]) > 0.05);
    assert!(output[1].iter().all(|sample| *sample == 0.0));
    assert!(output[2].iter().all(|sample| *sample == 0.0));

    // Output slices of different lengths: the shortest decides.
    let (mut short, mut long) = (vec![0.0_f32; 100], vec![7.0_f32; 300]);
    let taken = stretcher.process(&[&source[0], &source[1]], &mut [&mut short, &mut long]);
    assert_eq!(taken, 91);
    assert!(long[100..].iter().all(|sample| *sample == 7.0));
    assert_eq!(stretcher.process(&[], &mut []), 0);
}

#[test]
fn any_number_of_channels_is_stretched() {
    for channels in [1, 2, 3, 6] {
        let input: Vec<Vec<f32>> = (0..channels)
            .map(|channel| sine(200.0 * (channel + 1) as f64, 0.5, 60_000, RATE))
            .collect();
        let mut stretcher = stretcher(channels, 1.5, 0.0, Quality::Fast);
        let output = run(&mut stretcher, &input, &mut 0, 80_000, &[512]);
        for (channel, output) in output.iter().enumerate() {
            let level = tone(&output[30_000..70_000], 200.0 * (channel + 1) as f64, RATE).0;
            assert!(
                db(level / 0.5).abs() < 0.5,
                "channel {channel} of {channels}: {level}"
            );
        }
    }
}

/// Inputs chosen to break things.
fn hostile_inputs(frames: usize) -> Vec<(&'static str, Vec<f32>, Vec<f32>)> {
    let mut rng = Rng::new(21);
    let spikes: Vec<f32> = (0..frames)
        .map(|_| {
            if rng.unipolar() < 0.01 {
                rng.bipolar().signum()
            } else {
                0.0
            }
        })
        .collect();
    let tiny: Vec<f32> = (0..frames)
        .map(|n| if n % 2 == 0 { 1.0e-39 } else { -3.0e-42 })
        .collect();
    let burst: Vec<f32> = (0..frames)
        .map(|n| if n < frames / 4 { rng.bipolar() } else { 0.0 })
        .collect();
    let broken: Vec<f32> = (0..frames)
        .map(|n| match n % 1_000 {
            17 => f32::NAN,
            400 => f32::INFINITY,
            600 => f32::NEG_INFINITY,
            900 => f32::MAX,
            _ => rng.bipolar() * 0.3,
        })
        .collect();
    let faint = noise(4, 1e-7, frames);
    vec![
        ("silence", vec![0.0; frames], vec![0.0; frames]),
        ("DC", vec![1.0; frames], vec![-1.0; frames]),
        ("square", square(64, 1.0, frames), square(50, 1.0, frames)),
        ("impulses", spikes.clone(), burst.clone()),
        (
            "noise at +24 dB",
            noise(1, 15.85, frames),
            noise(2, 15.85, frames),
        ),
        ("subnormal input", tiny.clone(), tiny),
        ("burst then silence", burst, spikes),
        ("noise at -140 dB", faint.clone(), faint),
        ("not numbers", broken.clone(), broken),
    ]
}

#[test]
fn hostile_input_gives_finite_bounded_output() {
    let frames = 48_000;
    let settings = [
        (1.0, 0.0),
        (0.5, 0.0),
        (2.0, 0.0),
        (3.7, 0.0),
        (1.0, 12.0),
        (1.3, -12.0),
        (0.8, 7.0),
    ];
    for (name, left, right) in hostile_inputs(frames) {
        let loudest = peak(&left).max(peak(&right)).min(1e4);
        for quality in Quality::ALL {
            for (index, (ratio, pitch)) in settings.into_iter().enumerate() {
                let mut stretcher = stretcher(2, ratio, pitch, quality);
                stretcher.set_formant_preservation(index % 2 == 1);
                let input = vec![left.clone(), right.clone()];
                let wanted = (frames as f64 * ratio) as usize + 20_000;
                let output = run(&mut stretcher, &input, &mut 0, wanted, &[512]);
                for output in &output {
                    let what = format!("{name}, {quality:?}, ratio {ratio}, pitch {pitch}");
                    assert_finite(output, &what);
                    assert!(output.iter().all(|sample| !sample.is_subnormal()), "{what}");
                    // Overlapping blocks of a square or of noise can add
                    // up past the input's peak, but not far.
                    let limit = if name == "not numbers" {
                        1e5
                    } else {
                        loudest * 4.0
                    };
                    assert!(peak(output) <= limit, "{what}: peak {}", peak(output));
                    if matches!(name, "silence" | "subnormal input") {
                        assert!(output.iter().all(|sample| *sample == 0.0), "{what}");
                    }
                }
            }
        }
    }
}

#[test]
fn sound_comes_back_after_input_that_is_not_a_number() {
    let mut stretcher = stretcher(1, 1.25, 3.0, Quality::Standard);
    let poison = vec![vec![f32::NAN; 20_000]];
    let output = run(&mut stretcher, &poison, &mut 0, 20_000, &[512]);
    assert!(output[0].iter().all(|sample| *sample == 0.0));
    let input = vec![sine(500.0, 0.5, 96_000, RATE)];
    let output = run(&mut stretcher, &input, &mut 0, 96_000, &[512]);
    let wanted = 500.0 * 2.0_f64.powf(3.0 / 12.0);
    let level = tone(&output[0][40_000..90_000], wanted, RATE).0;
    assert!(db(level / 0.5).abs() < 0.5, "{level}");
}

#[test]
fn no_offset_is_added_to_a_signal_that_has_none() {
    let frames = 96_000;
    let signals = [
        ("noise", noise(8, 0.5, frames)),
        ("square", square(480, 0.8, frames)),
        ("drums", drum_loop(120.0, 4, RATE)),
    ];
    for (name, input) in signals {
        for (ratio, pitch) in [
            (0.6, 0.0),
            (1.5, 0.0),
            (1.0, -12.0),
            (1.0, 12.0),
            (1.4, 5.0),
        ] {
            let mut stretcher = stretcher(1, ratio, pitch, Quality::Standard);
            let wanted = (input.len() as f64 * ratio) as usize;
            let output = run(
                &mut stretcher,
                std::slice::from_ref(&input),
                &mut 0,
                wanted,
                &[512],
            )
            .remove(0);
            let part = &output[wanted / 4..];
            let offset = mean(part).abs();
            assert!(
                offset < 0.004 * rms(part).max(0.1),
                "{name} {ratio} {pitch}: offset {offset:e}"
            );
        }
    }
}

/// The level of `signal` over each stretch of 10 ms, in dB against
/// `reference`.
fn levels(signal: &[f32], reference: f64) -> (f64, f64) {
    let levels = signal
        .as_chunks::<480>()
        .0
        .iter()
        .map(|part| db(rms(part) / reference));
    levels.fold((f64::MAX, f64::MIN), |(low, high), level| {
        (low.min(level), high.max(level))
    })
}

#[test]
fn sweeping_the_ratio_and_the_pitch_does_not_click() {
    let amplitude = 0.5_f64;
    let input = vec![sine(440.0, amplitude, 600_000, RATE)];
    for quality in Quality::ALL {
        let mut stretcher = stretcher(1, 1.0, 0.0, quality);
        let mut position = 0;
        let mut output = Vec::new();
        let lead = stretcher.latency().output_frames(1.0) as usize + 4_800;
        output.extend(run(&mut stretcher, &input, &mut position, lead, &[64]).remove(0));
        // Four seconds of both controls moving all the time, set every 64
        // frames as an automation lane would.
        let steps = 4 * RATE as usize / 64;
        for step in 0..steps {
            let along = step as f64 / steps as f64;
            stretcher.set_time_ratio(2.0_f64.powf((along * 3.0 * std::f64::consts::TAU).sin()));
            stretcher.set_pitch_semitones(12.0 * (along * 2.0 * std::f64::consts::TAU).sin());
            output.extend(run(&mut stretcher, &input, &mut position, 64, &[64]).remove(0));
        }
        let moving = &output[lead..];
        // A sine of this level an octave up, the highest it gets, changes
        // by this much from one sample to the next at most. A click would
        // be a far bigger step.
        let steepest_tone = amplitude * std::f64::consts::TAU * 880.0 / f64::from(RATE);
        let found = f64::from(steepest(moving));
        assert!(
            found < 1.5 * steepest_tone,
            "{quality:?}: a step of {found} against {steepest_tone}"
        );
        let (low, high) = levels(moving, amplitude / 2.0_f64.sqrt());
        assert!(
            low > -3.0 && high < 1.5,
            "{quality:?}: level from {low:+.2} to {high:+.2} dB"
        );
    }
}

#[test]
fn jumping_the_ratio_and_the_pitch_does_not_click() {
    let amplitude = 0.5_f64;
    let input = vec![sine(300.0, amplitude, 600_000, RATE)];
    for quality in Quality::ALL {
        let mut stretcher = stretcher(1, 1.0, 0.0, quality);
        let mut position = 0;
        let lead = stretcher.latency().output_frames(1.0) as usize + 4_800;
        let mut output = run(&mut stretcher, &input, &mut position, lead, &[512]).remove(0);
        for (ratio, pitch) in [
            (2.0, 0.0),
            (0.5, 0.0),
            (1.0, 7.0),
            (1.0, -12.0),
            (4.0, 12.0),
            (1.0, 0.0),
        ] {
            stretcher.set_time_ratio(ratio);
            stretcher.set_pitch_semitones(pitch);
            output.extend(run(&mut stretcher, &input, &mut position, 24_000, &[512]).remove(0));
        }
        let moving = &output[lead..];
        let steepest_tone = amplitude * std::f64::consts::TAU * 600.0 / f64::from(RATE);
        let found = f64::from(steepest(moving));
        assert!(
            found < 1.5 * steepest_tone,
            "{quality:?}: a step of {found} against {steepest_tone}"
        );
        let (low, high) = levels(moving, amplitude / 2.0_f64.sqrt());
        assert!(
            low > -6.0 && high < 2.0,
            "{quality:?}: level from {low:+.2} to {high:+.2} dB"
        );
    }
}

#[test]
fn prepare_changes_the_format_and_keeps_the_settings() {
    let mut stretcher = Stretcher::new(1, 44_100);
    stretcher.set_time_ratio(1.5);
    stretcher.set_pitch_semitones(-2.0);
    stretcher.set_formant_preservation(true);
    let before = stretcher.latency();
    stretcher.prepare(2, 96_000, Quality::High);
    assert_eq!((stretcher.channels(), stretcher.sample_rate()), (2, 96_000));
    assert_eq!(stretcher.quality(), Quality::High);
    assert_eq!(
        (stretcher.time_ratio(), stretcher.pitch_semitones()),
        (1.5, -2.0)
    );
    assert!(stretcher.formant_preservation());
    assert!(stretcher.latency().input > before.input * 2);
    // The settings are in effect at once: no glide from the defaults.
    assert_eq!(stretcher.input_frames_needed(3_000), 2_000);
}

#[test]
fn the_latency_is_half_a_block_each_way_at_any_rate() {
    for rate in [8_000, 22_050, 44_100, 48_000, 88_200, 96_000, 192_000] {
        for quality in Quality::ALL {
            let stretcher = Stretcher::with_quality(1, rate, quality);
            let latency = stretcher.latency();
            let block = (f64::from(rate) * quality.block_seconds()) as usize;
            assert_eq!(latency.input + latency.output, block, "{rate} {quality:?}");
            assert!(latency.input.abs_diff(latency.output) <= 1);
            assert_eq!(
                latency.input_frames(2.0),
                latency.input as f64 + latency.output as f64 / 2.0
            );
            // Pre-roll for a seek: the history, and the run before it.
            let interval = (f64::from(rate) * quality.interval_seconds()) as usize;
            let blocks = block.div_ceil(interval) + 1;
            assert_eq!(
                stretcher.seek_frames(),
                blocks * interval + block + interval + 1
            );
        }
    }
}
