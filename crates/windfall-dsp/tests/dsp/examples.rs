//! Renders example audio from every processor to WAV files, for looking
//! at with other tools: spectrograms, statistics, or ears.
//!
//! Ignored by default. Name a folder and run it:
//!
//! ```text
//! WINDFALL_DSP_RENDER_DIR=/tmp/windfall-dsp cargo test -p windfall-dsp --release \
//!     -- --ignored render_examples
//! ```

use std::path::Path;

use windfall_dsp::{
    Compressor, CompressorParams, CutSlope, Delay, DelayMode, DelayParams, Effect, EqParams,
    FilterSlope, Instrument, LfoShape, Limiter, LimiterParams, ParametricEq, Reverb, ReverbParams,
    SubtractiveSynth, SynthParams, VoiceMode, Waveform,
};

use crate::support::{impulse, noise, prepared, prepared_instrument, render, run, write_wav};

const RATE: f32 = 48_000.0;

/// A sine that sweeps from 20 Hz to 20 kHz, an octave at a time.
fn sweep(seconds: f32, level: f32) -> Vec<f32> {
    let frames = (seconds * RATE) as usize;
    let octaves = (20_000.0_f64 / 20.0).log2();
    let mut phase = 0.0_f64;
    (0..frames)
        .map(|n| {
            let frequency = 20.0 * 2.0_f64.powf(octaves * n as f64 / frames as f64);
            phase += std::f64::consts::TAU * frequency / f64::from(RATE);
            (phase.sin() * f64::from(level)) as f32
        })
        .collect()
}

/// A crude drum loop: a decaying low thump and bursts of noise.
fn drums(seconds: f32) -> Vec<f32> {
    let frames = (seconds * RATE) as usize;
    let hiss = noise(77, 1.0, frames);
    (0..frames)
        .map(|n| {
            let beat = n % 24_000;
            let since = beat as f32 / RATE;
            let kick = (std::f32::consts::TAU * (45.0 + 90.0 * (-since * 30.0).exp()) * since)
                .sin()
                * (-since * 9.0).exp();
            let off = (n + 12_000) % 24_000;
            let snare = hiss[n] * (-(off as f32 / RATE) * 25.0).exp() * 0.6;
            let hat = hiss[n] * (-((n % 6_000) as f32 / RATE) * 90.0).exp() * 0.2;
            0.9 * kick + snare + hat
        })
        .collect()
}

fn through<E: Effect + Default>(
    params: &E::Params,
    input: &[f32],
    tail: usize,
) -> (Vec<f32>, Vec<f32>) {
    let mut effect: E = prepared(params, RATE);
    let mut left = input.to_vec();
    left.resize(input.len() + tail, 0.0);
    let mut right = left.clone();
    run(&mut effect, &mut left, &mut right, 256);
    (left, right)
}

fn save(folder: &Path, name: &str, (left, right): &(Vec<f32>, Vec<f32>)) {
    write_wav(&folder.join(format!("{name}.wav")), left, right, RATE);
}

/// Plays `notes` (start in seconds, key, length in seconds) and returns
/// the result with `tail` seconds after the last note.
fn play(params: &SynthParams, notes: &[(f32, u8, f32)], seconds: f32) -> (Vec<f32>, Vec<f32>) {
    let mut synth: SubtractiveSynth = prepared_instrument(params, RATE);
    let mut events: Vec<(usize, u8, bool)> = notes
        .iter()
        .flat_map(|(start, key, length)| {
            let on = (start * RATE) as usize;
            let off = ((start + length) * RATE) as usize;
            [(on, *key, true), (off, *key, false)]
        })
        .collect();
    events.sort();
    let frames = (seconds * RATE) as usize;
    let (mut left, mut right) = (Vec::new(), Vec::new());
    let mut position = 0;
    for (at, key, on) in events {
        let at = at.min(frames);
        let (l, r) = render(&mut synth, at - position);
        left.extend(l);
        right.extend(r);
        position = at;
        if on {
            synth.note_on(key, 0.9);
        } else {
            synth.note_off(key);
        }
    }
    let (l, r) = render(&mut synth, frames - position);
    left.extend(l);
    right.extend(r);
    (left, right)
}

#[test]
#[ignore = "writes WAV files: set WINDFALL_DSP_RENDER_DIR and run with --ignored"]
fn render_examples() {
    let Ok(folder) = std::env::var("WINDFALL_DSP_RENDER_DIR") else {
        println!("WINDFALL_DSP_RENDER_DIR is not set, so nothing was written");
        return;
    };
    let folder = Path::new(&folder);
    std::fs::create_dir_all(folder).expect("could not create the folder");
    let click = impulse(48_000, 0, 1.0);

    // Equaliser: a sweep through a busy curve, and the curve's impulse
    // response.
    let mut eq = EqParams::default();
    eq.low_cut.enabled = true;
    eq.low_cut.frequency_hz = 80.0;
    eq.low_cut.slope = CutSlope::Db24;
    eq.low_shelf.gain_db = 4.0;
    eq.peak1.gain_db = -9.0;
    eq.peak1.q = 4.0;
    eq.peak2.gain_db = 6.0;
    eq.peak3.gain_db = 9.0;
    eq.peak3.q = 6.0;
    eq.high_shelf.gain_db = -5.0;
    eq.high_cut.enabled = true;
    eq.high_cut.frequency_hz = 12_000.0;
    save(
        folder,
        "eq-sweep",
        &through::<ParametricEq>(&eq, &sweep(10.0, 0.1), 0),
    );
    save(
        folder,
        "eq-impulse",
        &through::<ParametricEq>(&eq, &click, 0),
    );
    // The same sweep while the mid band is dragged around.
    let mut moving: ParametricEq = prepared(&eq, RATE);
    let mut left = sweep(10.0, 0.1);
    let mut right = left.clone();
    for (index, (left, right)) in left.chunks_mut(64).zip(right.chunks_mut(64)).enumerate() {
        let wobble = (index as f32 * 0.02).sin();
        eq.peak2.frequency_hz = 1_000.0 * 4.0_f32.powf(wobble);
        eq.peak2.gain_db = 12.0 * (index as f32 * 0.013).cos();
        moving.set_params(&eq);
        moving.process(left, right);
    }
    save(folder, "eq-sweep-automated", &(left, right));

    // Dynamics on drums.
    let loop_ = drums(4.0);
    save(folder, "drums-dry", &(loop_.clone(), loop_.clone()));
    let squash = CompressorParams {
        threshold_db: -24.0,
        ratio: 6.0,
        attack_ms: 5.0,
        release_ms: 120.0,
        auto_makeup: true,
        ..CompressorParams::default()
    };
    save(
        folder,
        "drums-compressed",
        &through::<Compressor>(&squash, &loop_, 0),
    );
    let loud = LimiterParams {
        input_gain_db: 12.0,
        ceiling_db: -1.0,
        ..LimiterParams::default()
    };
    save(
        folder,
        "drums-limited",
        &through::<Limiter>(&loud, &loop_, 0),
    );
    let hot_sweep = sweep(10.0, 1.0);
    save(
        folder,
        "limiter-sweep",
        &through::<Limiter>(&loud, &hot_sweep, 0),
    );

    // Reverb: impulse responses at a few settings, and drums through it.
    for (name, size, decay_s, damping) in [
        ("room", 0.15, 0.5, 0.4),
        ("default", 0.5, 1.8, 0.5),
        ("hall", 1.0, 5.0, 0.3),
        ("bright", 0.5, 3.0, 0.0),
    ] {
        let params = ReverbParams {
            size,
            decay_s,
            damping,
            mix: 1.0,
            ..ReverbParams::default()
        };
        let tail = (decay_s * 1.5 * RATE) as usize;
        save(
            folder,
            &format!("reverb-impulse-{name}"),
            &through::<Reverb>(&params, &click, tail),
        );
    }
    save(
        folder,
        "drums-reverb",
        &through::<Reverb>(&ReverbParams::default(), &loop_, 96_000),
    );

    // Delay.
    let dub = DelayParams {
        sync: true,
        feedback: 0.7,
        mode: DelayMode::PingPong,
        high_cut_hz: 3_000.0,
        low_cut_hz: 200.0,
        saturation: 0.6,
        mix: 0.5,
        ..DelayParams::default()
    };
    save(
        folder,
        "delay-impulse",
        &through::<Delay>(&dub, &click, 192_000),
    );
    save(
        folder,
        "drums-delay",
        &through::<Delay>(&dub, &loop_, 96_000),
    );

    // Synth: single notes of each waveform, a chord, a high sweep to show
    // aliasing, a filter sweep, a unison lead and a mono line.
    for waveform in [
        Waveform::Sine,
        Waveform::Triangle,
        Waveform::Saw,
        Waveform::Square,
        Waveform::Pulse,
    ] {
        let mut params = SynthParams::default();
        params.oscillators[0].waveform = waveform;
        params.oscillators[0].pulse_width = 0.2;
        let notes = [
            (0.0, 36, 0.4),
            (0.5, 60, 0.4),
            (1.0, 84, 0.4),
            (1.5, 108, 0.4),
        ];
        save(
            folder,
            &format!("synth-{waveform:?}").to_lowercase(),
            &play(&params, &notes, 2.2),
        );
    }
    let chord = [
        (0.0, 48, 1.5),
        (0.0, 55, 1.5),
        (0.0, 64, 1.5),
        (0.0, 67, 1.5),
        (0.02, 72, 1.5),
    ];
    save(
        folder,
        "synth-chord",
        &play(&SynthParams::default(), &chord, 2.5),
    );

    let mut climbing = SynthParams {
        glide_ms: 2_000.0,
        voice_mode: VoiceMode::Mono,
        ..SynthParams::default()
    };
    climbing.amp_envelope.sustain = 1.0;
    save(
        folder,
        "synth-saw-glide-to-top",
        &play(&climbing, &[(0.0, 60, 0.3), (0.2, 127, 3.0)], 3.5),
    );

    let mut pluck = SynthParams::default();
    pluck.filter.cutoff_hz = 200.0;
    pluck.filter.resonance = 0.6;
    pluck.filter.envelope_octaves = 5.0;
    pluck.filter.slope = FilterSlope::Db24;
    pluck.filter_envelope.decay_ms = 400.0;
    pluck.filter_envelope.sustain = 0.0;
    let line = [
        (0.0, 36, 0.4),
        (0.5, 48, 0.4),
        (1.0, 43, 0.4),
        (1.5, 39, 0.4),
    ];
    save(folder, "synth-filter-pluck", &play(&pluck, &line, 2.5));

    let mut lead = SynthParams {
        unison_voices: 7,
        unison_detune_cents: 25.0,
        ..SynthParams::default()
    };
    lead.oscillators[1].level = 0.6;
    lead.oscillators[1].coarse = -12;
    lead.oscillators[1].waveform = Waveform::Square;
    lead.filter.cutoff_hz = 6_000.0;
    lead.filter.drive = 0.4;
    lead.lfos[0].shape = LfoShape::Sine;
    lead.lfos[0].pitch_semitones = 0.15;
    save(folder, "synth-unison-chord", &play(&lead, &chord, 2.5));

    // Sixteen notes arriving at once on four voices, then all stopped.
    let mut crowded = SynthParams {
        polyphony: 4,
        ..SynthParams::default()
    };
    crowded.oscillators[0].waveform = Waveform::Sine;
    let burst: Vec<(f32, u8, f32)> = (0..16)
        .map(|n| (0.05 * n as f32, 48 + n as u8 * 2, 0.5))
        .collect();
    save(folder, "synth-voice-stealing", &play(&crowded, &burst, 2.0));
    println!("wrote examples to {}", folder.display());
}
