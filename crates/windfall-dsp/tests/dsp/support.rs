//! Signals, measurements and drivers shared by the tests.

use std::f64::consts::TAU;

use windfall_dsp::blocks::noise::Rng;
use windfall_dsp::{Effect, Instrument, ParamKind, ParamScale, ParamSet};

pub const RATE: f32 = 48_000.0;

/// The sample rates every rate-dependent test covers.
pub const RATES: [f32; 3] = [44_100.0, 48_000.0, 96_000.0];

pub fn sine(frequency: f32, level: f32, frames: usize, rate: f32) -> Vec<f32> {
    (0..frames)
        .map(|n| (level as f64 * (TAU * frequency as f64 * n as f64 / rate as f64).sin()) as f32)
        .collect()
}

pub fn noise(seed: u32, level: f32, frames: usize) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    (0..frames).map(|_| rng.bipolar() * level).collect()
}

/// A square wave that sits at plus or minus `level`, so its absolute value
/// is constant.
pub fn square(period: usize, level: f32, frames: usize) -> Vec<f32> {
    (0..frames)
        .map(|n| {
            if (n / (period / 2)).is_multiple_of(2) {
                level
            } else {
                -level
            }
        })
        .collect()
}

pub fn impulse(frames: usize, at: usize, level: f32) -> Vec<f32> {
    let mut signal = vec![0.0; frames];
    signal[at] = level;
    signal
}

pub fn silence(frames: usize) -> Vec<f32> {
    vec![0.0; frames]
}

pub fn rms(signal: &[f32]) -> f64 {
    let power: f64 = signal.iter().map(|s| f64::from(*s) * f64::from(*s)).sum();
    (power / signal.len().max(1) as f64).sqrt()
}

pub fn peak(signal: &[f32]) -> f32 {
    signal.iter().fold(0.0, |peak, s| peak.max(s.abs()))
}

pub fn mean(signal: &[f32]) -> f64 {
    signal.iter().map(|s| f64::from(*s)).sum::<f64>() / signal.len().max(1) as f64
}

pub fn db(ratio: f64) -> f64 {
    20.0 * ratio.max(1.0e-30).log10()
}

/// The largest change between two neighbouring samples.
pub fn steepest(signal: &[f32]) -> f32 {
    signal.windows(2).fold(0.0, |steepest, pair| {
        steepest.max((pair[1] - pair[0]).abs())
    })
}

pub fn assert_finite(signal: &[f32], what: &str) {
    if let Some(position) = signal.iter().position(|s| !s.is_finite()) {
        panic!("{what}: sample {position} is {}", signal[position]);
    }
}

/// True if any sample is a subnormal number: too small to be audible, but
/// slow to compute with.
pub fn has_subnormals(signal: &[f32]) -> bool {
    signal.iter().any(|s| s.is_subnormal())
}

/// In-place radix-2 FFT. The length must be a power of two.
pub fn fft(re: &mut [f64], im: &mut [f64]) {
    let length = re.len();
    assert!(length.is_power_of_two() && im.len() == length);
    let mut j = 0;
    for i in 1..length {
        let mut bit = length >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut span = 2;
    while span <= length {
        let angle = -TAU / span as f64;
        let (step_re, step_im) = (angle.cos(), angle.sin());
        for start in (0..length).step_by(span) {
            let (mut w_re, mut w_im) = (1.0, 0.0);
            for offset in 0..span / 2 {
                let (a, b) = (start + offset, start + offset + span / 2);
                let t_re = re[b] * w_re - im[b] * w_im;
                let t_im = re[b] * w_im + im[b] * w_re;
                re[b] = re[a] - t_re;
                im[b] = im[a] - t_im;
                re[a] += t_re;
                im[a] += t_im;
                (w_re, w_im) = (
                    w_re * step_re - w_im * step_im,
                    w_re * step_im + w_im * step_re,
                );
            }
        }
        span *= 2;
    }
}

/// Magnitudes of the first half of the spectrum of `signal`, zero-padded to
/// `size`. Bin `k` is at `k * rate / size` Hz.
pub fn magnitudes(signal: &[f32], size: usize) -> Vec<f64> {
    let mut re = vec![0.0; size];
    let mut im = vec![0.0; size];
    for (slot, sample) in re.iter_mut().zip(signal) {
        *slot = f64::from(*sample);
    }
    fft(&mut re, &mut im);
    (0..size / 2)
        .map(|k| (re[k] * re[k] + im[k] * im[k]).sqrt())
        .collect()
}

/// Like [`magnitudes`], with a Hann window over the signal and scaled so
/// that a sine of amplitude 1 reads 1 at its bin.
pub fn windowed_magnitudes(signal: &[f32]) -> Vec<f64> {
    let size = signal.len();
    assert!(size.is_power_of_two());
    let mut re: Vec<f64> = signal
        .iter()
        .enumerate()
        .map(|(n, sample)| {
            let window = 0.5 - 0.5 * (TAU * n as f64 / size as f64).cos();
            f64::from(*sample) * window
        })
        .collect();
    let mut im = vec![0.0; size];
    fft(&mut re, &mut im);
    (0..size / 2)
        .map(|k| (re[k] * re[k] + im[k] * im[k]).sqrt() * 4.0 / size as f64)
        .collect()
}

/// Like [`windowed_magnitudes`] with a four-term Blackman-Harris window,
/// whose side lobes are 92 dB down. A tone spreads over four bins either
/// side of its own and over nothing else, which is what looking for faint
/// components next to loud ones needs.
pub fn sharp_magnitudes(signal: &[f32]) -> Vec<f64> {
    let size = signal.len();
    assert!(size.is_power_of_two());
    const TERMS: [f64; 4] = [0.35875, 0.48829, 0.14128, 0.01168];
    let mut re: Vec<f64> = signal
        .iter()
        .enumerate()
        .map(|(n, sample)| {
            let turn = TAU * n as f64 / size as f64;
            let window = TERMS[0] - TERMS[1] * turn.cos() + TERMS[2] * (2.0 * turn).cos()
                - TERMS[3] * (3.0 * turn).cos();
            f64::from(*sample) * window
        })
        .collect();
    let mut im = vec![0.0; size];
    fft(&mut re, &mut im);
    (0..size / 2)
        .map(|k| (re[k] * re[k] + im[k] * im[k]).sqrt() * 2.0 / (size as f64 * TERMS[0]))
        .collect()
}

/// Amplitude of the component of `signal` at exactly `frequency`, using a
/// Hann window.
pub fn tone_level(signal: &[f32], frequency: f64, rate: f32) -> f64 {
    let length = signal.len() as f64;
    let (mut re, mut im, mut weight) = (0.0, 0.0, 0.0);
    for (n, sample) in signal.iter().enumerate() {
        let window = 0.5 - 0.5 * (TAU * n as f64 / length).cos();
        let phase = TAU * frequency * n as f64 / f64::from(rate);
        re += f64::from(*sample) * window * phase.cos();
        im += f64::from(*sample) * window * phase.sin();
        weight += window;
    }
    2.0 * (re * re + im * im).sqrt() / weight
}

/// The frequency of the strongest component of `signal` in Hz, found by
/// counting whole cycles between the first and last upward zero crossing.
/// Accurate to a tiny fraction of a cent for a steady tone.
pub fn frequency_of(signal: &[f32], rate: f32) -> f64 {
    let mut crossings = Vec::new();
    for n in 1..signal.len() {
        let (before, after) = (f64::from(signal[n - 1]), f64::from(signal[n]));
        if before < 0.0 && after >= 0.0 {
            crossings.push((n - 1) as f64 + before / (before - after));
        }
    }
    assert!(crossings.len() > 2, "no steady tone to measure");
    let cycles = (crossings.len() - 1) as f64;
    cycles * f64::from(rate) / (crossings[crossings.len() - 1] - crossings[0])
}

pub fn cents(frequency: f64, reference: f64) -> f64 {
    1200.0 * (frequency / reference).log2()
}

/// A prepared effect with `params` in effect from its first sample.
pub fn prepared<E: Effect + Default>(params: &E::Params, rate: f32) -> E {
    let mut effect = E::default();
    effect.prepare(rate, 512);
    effect.set_params(params);
    effect
}

/// Runs an effect over a stereo signal in blocks of `block` samples.
pub fn run<E: Effect>(effect: &mut E, left: &mut [f32], right: &mut [f32], block: usize) {
    for (left, right) in left.chunks_mut(block).zip(right.chunks_mut(block)) {
        effect.process(left, right);
    }
}

/// Runs a mono signal through both channels of an effect and returns the
/// left output.
pub fn run_mono<E: Effect>(effect: &mut E, signal: &[f32]) -> Vec<f32> {
    let mut left = signal.to_vec();
    let mut right = signal.to_vec();
    run(effect, &mut left, &mut right, 256);
    left
}

/// A prepared instrument with `params` in effect from its first sample.
pub fn prepared_instrument<I: Instrument + Default>(params: &I::Params, rate: f32) -> I {
    let mut instrument = I::default();
    instrument.prepare(rate, 512);
    instrument.set_params(params);
    instrument
}

/// Renders `frames` samples from an instrument and returns left and right.
pub fn render<I: Instrument>(instrument: &mut I, frames: usize) -> (Vec<f32>, Vec<f32>) {
    let mut left = vec![0.0; frames];
    let mut right = vec![0.0; frames];
    for (left, right) in left.chunks_mut(256).zip(right.chunks_mut(256)) {
        instrument.process(left, right);
    }
    (left, right)
}

/// Parameters with every control at a random place in its range. One time
/// in four a control sits on its minimum, its maximum or its default,
/// where edge cases live.
pub fn random_params<P: ParamSet>(rng: &mut Rng) -> P {
    let mut params = P::default();
    for (index, info) in P::descriptors().iter().enumerate() {
        let roll = rng.unipolar();
        let value = if roll < 0.08 {
            info.min
        } else if roll < 0.16 {
            info.max
        } else if roll < 0.25 {
            info.default
        } else {
            let position = rng.unipolar();
            match (info.kind, info.scale) {
                (ParamKind::Float, ParamScale::Logarithmic) => {
                    info.min * (info.max / info.min).powf(position)
                }
                _ => info.min + (info.max - info.min) * position,
            }
        };
        assert!(params.set(index, value));
    }
    params
}

/// Writes a stereo 32-bit float WAV file, for looking at renders with other
/// tools.
pub fn write_wav(path: &std::path::Path, left: &[f32], right: &[f32], rate: f32) {
    let frames = left.len().min(right.len());
    let data_bytes = (frames * 8) as u32;
    let mut bytes = Vec::with_capacity(44 + frames * 8);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    // Format 3 is IEEE float.
    bytes.extend_from_slice(&3_u16.to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&(rate as u32).to_le_bytes());
    bytes.extend_from_slice(&(rate as u32 * 8).to_le_bytes());
    bytes.extend_from_slice(&8_u16.to_le_bytes());
    bytes.extend_from_slice(&32_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_bytes.to_le_bytes());
    for (left, right) in left.iter().zip(right) {
        bytes.extend_from_slice(&left.to_le_bytes());
        bytes.extend_from_slice(&right.to_le_bytes());
    }
    std::fs::write(path, bytes).expect("could not write the WAV file");
}
