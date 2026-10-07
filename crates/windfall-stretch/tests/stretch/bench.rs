//! The figures the crate documentation quotes: how much faster than real
//! time each preset runs, and what each does to test signals.
//!
//! Ignored by default, since the speeds only mean something in a release
//! build on a quiet machine:
//!
//! ```text
//! cargo test -p windfall-stretch --release -- --ignored --nocapture realtime_factors
//! cargo test -p windfall-stretch --release -- --ignored --nocapture quality_figures
//! ```

use std::time::Instant;

use windfall_stretch::{Quality, Stretcher};

use crate::quality::{click_figures, middle};
use crate::support::{
    RATE, band_ripple_db, cents, chord, clicks, db, frequency_near, noise, rms, run, sine,
    spectral_flatness, steepest, stream_mono, stretcher, tone_level,
};

const BLOCK: usize = 256;
const SECONDS: usize = 20;

/// Seconds of output produced per second of computing in blocks of
/// [`BLOCK`] frames, and the longest a single block took in milliseconds.
/// The best of a few runs, to keep other programs out of the figures.
fn measure(stretcher: &mut Stretcher, source: &[Vec<f32>]) -> (f64, f64) {
    let channels = source.len();
    let mut output = vec![vec![0.0_f32; BLOCK]; channels];
    let blocks = SECONDS * RATE as usize / BLOCK;
    let (mut best, mut longest) = (f64::MAX, f64::MAX);
    for _ in 0..3 {
        stretcher.reset();
        let mut position = 0;
        let mut worst = 0.0_f64;
        let start = Instant::now();
        for _ in 0..blocks {
            if position + 4 * BLOCK + 8 > source[0].len() {
                position = 0;
            }
            let input = [&source[0][position..], &source[channels - 1][position..]];
            let mut into: [&mut [f32]; 2] = [&mut [], &mut []];
            for (into, channel) in into.iter_mut().zip(output.iter_mut()) {
                *into = channel;
            }
            let began = Instant::now();
            position += stretcher.process(&input[..channels], &mut into[..channels]);
            worst = worst.max(began.elapsed().as_secs_f64());
        }
        best = best.min(start.elapsed().as_secs_f64());
        longest = longest.min(worst);
    }
    std::hint::black_box(&output);
    (SECONDS as f64 / best, longest * 1_000.0)
}

#[test]
#[ignore = "a benchmark: run it in release mode with --nocapture"]
fn realtime_factors() {
    println!(
        "\nRealtime factor at 48 kHz, blocks of {BLOCK}: seconds of output per second of CPU,"
    );
    println!("the longest single block in ms, and what a seek costs in ms");
    let settings = [
        ("ratio 1", 1.0, 0.0, false),
        ("ratio 1.25", 1.25, 0.0, false),
        ("ratio 0.8", 0.8, 0.0, false),
        ("pitch +3", 1.0, 3.0, false),
        ("ratio 1.25, pitch +3", 1.25, 3.0, false),
        ("the same, formants kept", 1.25, 3.0, true),
    ];
    for channels in [1, 2] {
        let source: Vec<Vec<f32>> = (0..channels)
            .map(|channel| {
                let notes = chord(
                    &[220.0, 330.0, 550.0, 1_760.0],
                    0.4,
                    20 * RATE as usize,
                    RATE,
                );
                let hiss = noise(channel as u32 + 1, 0.2, notes.len());
                notes
                    .iter()
                    .zip(&hiss)
                    .map(|(note, hiss)| note + hiss)
                    .collect()
            })
            .collect();
        for quality in Quality::ALL {
            println!("  {quality:?}, {channels} channel(s)");
            for (name, ratio, pitch, formants) in settings {
                let mut stretcher = stretcher(channels, ratio, pitch, quality);
                stretcher.set_formant_preservation(formants);
                let (factor, longest) = measure(&mut stretcher, &source);

                let frames = stretcher.seek_frames();
                let pre_roll: Vec<&[f32]> =
                    source.iter().map(|channel| &channel[..frames]).collect();
                let mut seek = f64::MAX;
                for _ in 0..5 {
                    let began = Instant::now();
                    stretcher.seek(&pre_roll);
                    seek = seek.min(began.elapsed().as_secs_f64());
                }
                println!(
                    "    {name:<26} {factor:>7.0}x   block {longest:>6.3} ms   seek {:>6.3} ms",
                    seek * 1_000.0
                );
            }
        }
    }
}

#[test]
#[ignore]
fn probe() {
    use crate::support::{peak, steepest};
    // DC under a pitch shift: does it stay bounded?
    for quality in Quality::ALL {
        let mut stretcher = stretcher(1, 1.3, -12.0, quality);
        let input = vec![vec![1.0_f32; 480_000]];
        let output = run(&mut stretcher, &input, &mut 0, 600_000, &[512]).remove(0);
        let peaks: Vec<String> = output
            .chunks(60_000)
            .map(|part| format!("{:.2}", peak(part)))
            .collect();
        println!("{quality:?} DC peaks per 1.25 s: {}", peaks.join(" "));
    }
    // Jumps: where is the steepest step?
    for quality in Quality::ALL {
        let input = vec![sine(300.0, 0.5, 600_000, RATE)];
        let mut stretcher = stretcher(1, 1.0, 0.0, quality);
        let mut position = 0;
        run(&mut stretcher, &input, &mut position, 20_000, &[512]);
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
            let part = run(&mut stretcher, &input, &mut position, 24_000, &[512]).remove(0);
            let at = (1..part.len())
                .max_by(|a, b| {
                    (part[*a] - part[a - 1])
                        .abs()
                        .total_cmp(&(part[*b] - part[b - 1]).abs())
                })
                .unwrap();
            let lows: Vec<String> = part
                .chunks(2_400)
                .map(|chunk| format!("{:+.1}", db(rms(chunk) / 0.3536)))
                .collect();
            println!(
                "{quality:?} to {ratio}/{pitch}: steepest {:.4} at {at}, peak {:.3}, levels {}",
                steepest(&part),
                peak(&part),
                lows.join(" ")
            );
        }
    }
    // The end of a sine: what follows it?
    for quality in Quality::ALL {
        for ratio in [0.5, 1.0, 1.7] {
            let input = vec![sine(440.0, 0.5, 48_000, RATE)];
            let mut stretcher = stretcher(1, ratio, 0.0, quality);
            let lead = stretcher.latency().output_frames(ratio).ceil() as usize;
            let body = (48_000.0 * ratio) as usize;
            let output =
                run(&mut stretcher, &input, &mut 0, lead + body + 24_000, &[512]).remove(0);
            let after: Vec<String> = output[lead + body..]
                .chunks(480)
                .take(12)
                .map(|chunk| format!("{:.0}", db(rms(chunk) / 0.3536)))
                .collect();
            let before: Vec<String> = output[lead + body - 2_400..lead + body]
                .chunks(480)
                .map(|chunk| format!("{:.1}", db(rms(chunk) / 0.3536)))
                .collect();
            println!(
                "{quality:?} {ratio}: before the end {} | after, per 10 ms: {}",
                before.join(" "),
                after.join(" ")
            );
        }
    }
    // The chord at twice the length.
    let notes = [440.0, 554.37, 659.26, 880.0, 1_108.73];
    for quality in Quality::ALL {
        for ratio in [1.5, 1.75, 2.0, 3.0] {
            let output = stream_mono(&chord(&notes, 0.8, 144_000, RATE), ratio, 0.0, quality);
            let part = middle(&output);
            let levels: Vec<String> = notes
                .iter()
                .map(|note| format!("{:+.2}", db(tone_level(part, *note, RATE) / 0.16)))
                .collect();
            println!("{quality:?} chord at {ratio}: {}", levels.join(" "));
        }
    }
}

#[test]
#[ignore = "prints the measurements behind the documented bounds"]
fn quality_figures() {
    let frames = 3 * RATE as usize;
    for quality in Quality::ALL {
        println!("\n== {quality:?}");
        println!("Sine, worst over 110, 440, 1000 and 3520 Hz: cents off, level in dB");
        for (ratio, pitch) in [
            (0.5, 0.0),
            (0.75, 0.0),
            (1.25, 0.0),
            (1.5, 0.0),
            (2.0, 0.0),
            (1.0, -12.0),
            (1.0, -5.0),
            (1.0, 7.0),
            (1.0, 12.0),
            (1.3, -3.0),
        ] {
            let (mut off, mut level) = (0.0_f64, 0.0_f64);
            for frequency in [110.0, 440.0, 1_000.0, 3_520.0] {
                let output =
                    stream_mono(&sine(frequency, 0.5, frames, RATE), ratio, pitch, quality);
                let part = middle(&output);
                let wanted = frequency * 2.0_f64.powf(pitch / 12.0);
                let found = frequency_near(part, wanted, 0.03, RATE);
                let cents = cents(found, wanted);
                let db = db(tone_level(part, found, RATE) / 0.5);
                off = if cents.abs() > off.abs() { cents } else { off };
                level = if db.abs() > level.abs() { db } else { level };
            }
            println!("  ratio {ratio:<4} pitch {pitch:>5}: {off:+.4} cents, {level:+.3} dB");
        }

        println!("Sine at 1 kHz: the largest step between two samples against the sine's own");
        let input = sine(1_000.0, 0.5, frames, RATE);
        let own = 0.5 * std::f64::consts::TAU * 1_000.0 / f64::from(RATE);
        let steps: Vec<String> = [0.5, 0.67, 0.75, 1.25, 1.5, 1.75, 2.0, 3.0]
            .iter()
            .map(|ratio| {
                let output = stream_mono(&input, *ratio, 0.0, quality);
                format!("{ratio}: {:.3}", f64::from(steepest(middle(&output))) / own)
            })
            .collect();
        println!("  {}", steps.join(", "));

        println!("Two notes a gap apart at 1 kHz, ratio 0.75: cents off and level of each");
        for gap in [20.0, 26.0, 32.0, 40.0, 50.0, 65.0] {
            let notes = [1_000.0, 1_000.0 + gap];
            let output = stream_mono(&chord(&notes, 0.8, frames, RATE), 0.75, 0.0, quality);
            let part = middle(&output);
            let figures: Vec<String> = notes
                .iter()
                .map(|note| {
                    let found = frequency_near(part, *note, 0.01, RATE);
                    let level = db(tone_level(part, found, RATE) / 0.4);
                    format!("{:+.2} cents {level:+.2} dB", cents(found, *note))
                })
                .collect();
            println!("  {gap:>4} Hz: {}", figures.join(", "));
        }

        println!("Clicks: pre-echo in dB, share in place, level in dB, frames off");
        let period = RATE as usize / 2;
        let input = clicks(period / 2, period, 4 * RATE as usize, RATE);
        for ratio in [0.5, 0.75, 0.9, 1.1, 1.25, 1.5, 2.0] {
            let output = stream_mono(&input, ratio, 0.0, quality);
            let (early, near, level, furthest) = click_figures(&output, period, ratio, &input);
            println!(
                "  ratio {ratio:<4}: {early:>6.1} dB, {:.2}%, {level:+.2} dB, {furthest:+}",
                near * 100.0
            );
        }

        println!("White noise: level in dB, ripple in third octaves in dB, flatness");
        let input = noise(3, 0.5, 4 * RATE as usize);
        println!(
            "  input: flatness {:.3}",
            spectral_flatness(&input, 4_096, RATE)
        );
        for (ratio, pitch) in [
            (0.5, 0.0),
            (0.75, 0.0),
            (1.25, 0.0),
            (1.5, 0.0),
            (2.0, 0.0),
            (1.0, 7.0),
        ] {
            let output = stream_mono(&input, ratio, pitch, quality);
            let part = middle(&output);
            println!(
                "  ratio {ratio:<4} pitch {pitch:>4}: {:+.2} dB, {:.2} dB, {:.3}",
                db(rms(part) / rms(&input)),
                band_ripple_db(part, RATE),
                spectral_flatness(part, 4_096, RATE),
            );
        }

        let mut stretcher = stretcher(1, 1.0, 0.0, quality);
        let delay = stretcher.latency().input + stretcher.latency().output;
        let output = run(
            &mut stretcher,
            std::slice::from_ref(&input),
            &mut 0,
            150_000,
            &[512],
        )
        .remove(0);
        let worst = input
            .iter()
            .zip(&output[delay..])
            .fold(0.0_f32, |worst, (a, b)| worst.max((a - b).abs()));
        println!("Unity: noise at -6 dBFS comes back within {worst:e}");
        println!("Latency: {:?}", stretcher.latency());
    }
}
