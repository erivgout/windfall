//! How much faster than real time each processor runs.
//!
//! Ignored by default, since the numbers only mean something in a release
//! build on a quiet machine:
//!
//! ```text
//! cargo test -p windfall-dsp --release -- --ignored --nocapture realtime_factors
//! ```

use std::time::Instant;

use windfall_dsp::{
    AnyEffect, AnyInstrument, CompressorParams, DelayParams, EffectKind, EffectParams, EqParams,
    FilterSlope, InstrumentParams, LimiterParams, ReverbParams, SynthParams, Waveform,
};

use crate::support::noise;

const RATE: f32 = 48_000.0;
const BLOCK: usize = 256;
const SECONDS: usize = 20;

/// Seconds of audio produced per second of computing, taking the best of
/// a few runs to keep other programs out of the figure.
fn realtime_factor(mut process: impl FnMut(&mut [f32], &mut [f32])) -> f64 {
    let source_left = noise(1, 0.5, BLOCK);
    let source_right = noise(2, 0.5, BLOCK);
    let (mut left, mut right) = (vec![0.0; BLOCK], vec![0.0; BLOCK]);
    let blocks = SECONDS * RATE as usize / BLOCK;
    let mut best = f64::MAX;
    for _ in 0..3 {
        let start = Instant::now();
        for _ in 0..blocks {
            left.copy_from_slice(&source_left);
            right.copy_from_slice(&source_right);
            process(&mut left, &mut right);
        }
        best = best.min(start.elapsed().as_secs_f64());
    }
    std::hint::black_box((&left, &right));
    SECONDS as f64 / best
}

fn busy_eq() -> EqParams {
    let mut params = EqParams::default();
    params.low_cut.enabled = true;
    params.low_cut.slope = windfall_dsp::CutSlope::Db24;
    params.high_cut.enabled = true;
    params.low_shelf.gain_db = 3.0;
    params.peak1.gain_db = -4.0;
    params.peak2.gain_db = 5.0;
    params.peak3.gain_db = 2.0;
    params.high_shelf.gain_db = -3.0;
    params
}

fn synth_factor(params: SynthParams, voices: u8) -> f64 {
    let mut synth = AnyInstrument::new(&InstrumentParams::SubtractiveSynth(params));
    synth.prepare(RATE, BLOCK);
    for voice in 0..voices {
        synth.note_on(36 + voice * 3, 0.8);
    }
    realtime_factor(|left, right| synth.process(left, right))
}

#[test]
#[ignore = "a benchmark: run it in release mode with --nocapture"]
fn realtime_factors() {
    println!("\nRealtime factor at 48 kHz, blocks of {BLOCK}: seconds of audio per second of CPU");
    let effects = [
        (
            "Parametric EQ, defaults (flat)",
            EffectKind::Eq.default_params(),
        ),
        ("Parametric EQ, all 7 bands", EffectParams::Eq(busy_eq())),
        (
            "Compressor",
            EffectParams::Compressor(CompressorParams {
                threshold_db: -30.0,
                ..CompressorParams::default()
            }),
        ),
        (
            "Limiter",
            EffectParams::Limiter(LimiterParams {
                input_gain_db: 12.0,
                ..LimiterParams::default()
            }),
        ),
        ("Reverb", EffectParams::Reverb(ReverbParams::default())),
        (
            "Delay",
            EffectParams::Delay(DelayParams {
                saturation: 0.5,
                ..DelayParams::default()
            }),
        ),
    ];
    for (name, params) in effects {
        let mut effect = AnyEffect::new(&params);
        effect.prepare(RATE, BLOCK);
        let factor = realtime_factor(|left, right| effect.process(left, right));
        println!("  {name:<36} {factor:>9.0}x");
    }

    let default = SynthParams::default();
    let mut rich = SynthParams::default();
    rich.oscillators[1].level = 0.7;
    rich.oscillators[1].waveform = Waveform::Pulse;
    rich.oscillators[2].level = 0.5;
    rich.oscillators[2].waveform = Waveform::Triangle;
    rich.filter.cutoff_hz = 2_000.0;
    rich.filter.slope = FilterSlope::Db24;
    rich.filter.drive = 0.5;
    let mut stacked = rich;
    stacked.unison_voices = 7;
    let synths = [
        ("Synth, silent", default, 0),
        ("Synth, 1 voice, default patch", default, 1),
        ("Synth, 16 voices, default patch", default, 16),
        ("Synth, 16 voices, 3 osc, 24 dB, drive", rich, 16),
        ("Synth, 16 voices, same with 7 unison", stacked, 16),
    ];
    for (name, mut params, voices) in synths {
        params.polyphony = 32;
        params.amp_envelope.sustain = 1.0;
        let factor = synth_factor(params, voices);
        println!("  {name:<36} {factor:>9.0}x");
    }
}
