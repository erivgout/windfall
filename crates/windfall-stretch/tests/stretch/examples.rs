//! Renders example audio to WAV files, for looking at with other tools:
//! spectrograms, statistics, or ears.
//!
//! Ignored by default. Name a folder and run it:
//!
//! ```text
//! WINDFALL_STRETCH_RENDER_DIR=/tmp/windfall-stretch cargo test -p windfall-stretch --release \
//!     -- --ignored render_examples
//! ```

use std::f64::consts::TAU;
use std::path::Path;

use windfall_core::AudioBuffer;
use windfall_stretch::{Quality, stretch};

use crate::support::{RATE, Rng, chord, clicks, drum_loop, noise, run, sine, stretcher, write_wav};

/// A sine that sweeps from 50 Hz to 12 kHz, an octave at a time.
fn sweep(frames: usize) -> Vec<f32> {
    let octaves = (12_000.0_f64 / 50.0).log2();
    let mut phase = 0.0_f64;
    (0..frames)
        .map(|n| {
            let frequency = 50.0 * 2.0_f64.powf(octaves * n as f64 / frames as f64);
            phase += TAU * frequency / f64::from(RATE);
            (phase.sin() * 0.5) as f32
        })
        .collect()
}

/// Plucked notes, each a handful of harmonics that die away, over the
/// drum loop: something like a bar of music.
fn song(bpm: f64, beats: usize) -> Vec<f32> {
    let mut signal = drum_loop(bpm, beats, RATE);
    let beat = 60.0 / bpm * f64::from(RATE);
    let mut rng = Rng::new(12);
    let scale = [110.0, 130.81, 146.83, 164.81, 196.0, 220.0, 261.63, 293.66];
    for half in 0..beats * 2 {
        let start = (half as f64 * beat / 2.0) as usize;
        let note = scale[rng.between(0, scale.len() - 1)] * if half % 3 == 0 { 2.0 } else { 1.0 };
        for n in start..signal.len().min(start + (beat * 1.5) as usize) {
            let time = (n - start) as f64 / f64::from(RATE);
            let pluck: f64 = (1..=6)
                .map(|harmonic| {
                    let decay = (-time * (2.0 + harmonic as f64)).exp();
                    (TAU * note * harmonic as f64 * time).sin() * decay / harmonic as f64
                })
                .sum();
            signal[n] = signal[n] * 0.999 + (pluck * 0.18) as f32;
        }
    }
    signal
}

fn save(folder: &Path, name: &str, signal: &[f32]) {
    write_wav(&folder.join(format!("{name}.wav")), &[signal], RATE);
}

#[test]
#[ignore = "writes WAV files to WINDFALL_STRETCH_RENDER_DIR"]
fn render_examples() {
    let Ok(folder) = std::env::var("WINDFALL_STRETCH_RENDER_DIR") else {
        panic!("set WINDFALL_STRETCH_RENDER_DIR to the folder to render into");
    };
    let folder = Path::new(&folder);
    std::fs::create_dir_all(folder).expect("could not make the folder");

    let seconds = 4 * RATE as usize;
    let sources = [
        ("sine", sine(440.0, 0.5, seconds, RATE)),
        (
            "chord",
            chord(&[220.0, 277.18, 329.63, 440.0, 554.37], 0.8, seconds, RATE),
        ),
        (
            "lowchord",
            chord(&[110.0, 138.59, 164.81], 0.8, seconds, RATE),
        ),
        ("sweep", sweep(seconds)),
        ("clicks", clicks(12_000, 24_000, seconds, RATE)),
        ("drums", drum_loop(120.0, 8, RATE)),
        ("song", song(120.0, 8)),
        ("noise", noise(3, 0.5, seconds)),
    ];
    let settings = [
        ("x050", 0.5, 0.0),
        ("x075", 0.75, 0.0),
        ("x125", 1.25, 0.0),
        ("x150", 1.5, 0.0),
        ("x200", 2.0, 0.0),
        ("x400", 4.0, 0.0),
        ("up12", 1.0, 12.0),
        ("up4", 1.0, 4.0),
        ("down5", 1.0, -5.0),
        ("down12", 1.0, -12.0),
        ("x130down3", 1.3, -3.0),
    ];
    for (name, source) in &sources {
        save(folder, name, source);
        let buffer = AudioBuffer::from_interleaved(RATE, 1, source.clone());
        for (label, ratio, pitch) in settings {
            for quality in Quality::ALL {
                let output = stretch(&buffer, ratio, pitch, quality);
                let quality = format!("{quality:?}").to_lowercase();
                save(
                    folder,
                    &format!("{name}_{label}_{quality}"),
                    output.samples(),
                );
            }
        }
    }

    // Both controls moving while the song plays.
    let source = vec![song(120.0, 16)];
    let mut stretcher = stretcher(1, 1.0, 0.0, Quality::Standard);
    let mut position = 0;
    let mut output = Vec::new();
    let steps = 12 * RATE as usize / 64;
    for step in 0..steps {
        let along = step as f64 / steps as f64;
        stretcher.set_time_ratio(2.0_f64.powf((along * 2.0 * TAU).sin()));
        stretcher.set_pitch_semitones(7.0 * (along * 3.0 * TAU).sin());
        output.extend(run(&mut stretcher, &source, &mut position, 64, &[64]).remove(0));
    }
    save(folder, "song_sweep", &output);
}
