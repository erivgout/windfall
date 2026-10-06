//! The building blocks every factory sound is made of: noise, oscillators,
//! envelopes, filters, and the finishing step that turns a raw render into a
//! tidy one-shot.

use crate::math::{db_to_gain, exp, sin_turns, tan_turns};

/// Sample rate of every factory sound, as a float for the synthesis code.
pub const SAMPLE_RATE: f64 = crate::SAMPLE_RATE as f64;

/// A raw or finished render, one `Vec` of samples per channel.
#[derive(Debug, Clone, PartialEq)]
pub enum Audio {
    Mono(Vec<f64>),
    Stereo(Vec<f64>, Vec<f64>),
}

impl Audio {
    /// The samples of each channel: one for mono, left then right for stereo.
    pub fn channels(&self) -> Vec<&[f64]> {
        match self {
            Audio::Mono(mono) => vec![mono],
            Audio::Stereo(left, right) => vec![left, right],
        }
    }

    fn channels_mut(&mut self) -> Vec<&mut Vec<f64>> {
        match self {
            Audio::Mono(mono) => vec![mono],
            Audio::Stereo(left, right) => vec![left, right],
        }
    }

    /// Length in frames. Both channels of a stereo render are equally long.
    pub fn frames(&self) -> usize {
        self.channels()[0].len()
    }

    /// Largest absolute sample value on any channel.
    pub fn peak(&self) -> f64 {
        self.channels()
            .iter()
            .flat_map(|channel| channel.iter())
            .fold(0.0, |peak, sample| sample.abs().max(peak))
    }
}

/// Renders `seconds` of sound by calling `sample` with the time of each
/// frame in seconds.
pub fn synth(seconds: f64, mut sample: impl FnMut(f64) -> f64) -> Vec<f64> {
    let frames = (seconds * SAMPLE_RATE).round() as usize;
    (0..frames)
        .map(|frame| sample(frame as f64 / SAMPLE_RATE))
        .collect()
}

/// Adds `gain` times `other` to `target`, sample by sample.
pub fn mix_into(target: &mut [f64], other: &[f64], gain: f64) {
    for (out, sample) in target.iter_mut().zip(other) {
        *out += gain * sample;
    }
}

/// Deterministic white noise (xorshift64*). The same seed always gives the
/// same sequence, on every platform.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// A generator that produces the sequence belonging to `seed`.
    pub fn new(seed: u64) -> Self {
        // One splitmix64 round, so that small neighbouring seeds start far
        // apart and a zero seed cannot stall the generator.
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        Self((z ^ (z >> 31)) | 1)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// The next white noise sample, uniform in -1..1.
    pub fn white(&mut self) -> f64 {
        const SCALE: f64 = 2.0 / (1u64 << 53) as f64;
        (self.next_u64() >> 11) as f64 * SCALE - 1.0
    }
}

/// Exponential decay from 1 with time constant `tau` seconds.
pub fn decay(t: f64, tau: f64) -> f64 {
    exp(-t / tau)
}

/// A decay that follows `decay(t, tau)` at first and then bends down ever
/// faster. It is 9 dB under the plain decay at `knee` seconds and inaudible
/// by about two and a half times that. A plain exponential never quite ends;
/// this one lets a sound hold its body and still stop.
pub fn decay_to(t: f64, tau: f64, knee: f64) -> f64 {
    let bend = t / knee;
    exp(-t / tau - bend * bend)
}

/// Rise from 0 toward 1 with time constant `tau` seconds.
pub fn attack(t: f64, tau: f64) -> f64 {
    1.0 - exp(-t / tau)
}

/// A decay that begins at `start` seconds and is silent before it.
pub fn burst(t: f64, start: f64, tau: f64) -> f64 {
    if t < start {
        0.0
    } else {
        decay(t - start, tau)
    }
}

/// An oscillator phase. Each call to a waveform method returns one sample
/// and advances by `freq` hertz, so the pitch can change every sample.
#[derive(Debug, Clone, Default)]
pub struct Osc {
    phase: f64,
}

impl Osc {
    /// Starts at phase zero, where a sine is zero and rising.
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts at `phase` turns instead of zero.
    pub fn at(phase: f64) -> Self {
        Self { phase }
    }

    fn advance(&mut self, step: f64) {
        self.phase += step;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
    }

    /// Sine wave.
    pub fn sine(&mut self, freq: f64) -> f64 {
        let out = sin_turns(self.phase);
        self.advance(freq / SAMPLE_RATE);
        out
    }

    /// Band-limited square wave.
    pub fn square(&mut self, freq: f64) -> f64 {
        let step = freq / SAMPLE_RATE;
        let falling = if self.phase < 0.5 {
            self.phase + 0.5
        } else {
            self.phase - 0.5
        };
        let naive = if self.phase < 0.5 { 1.0 } else { -1.0 };
        let out = naive + poly_blep(self.phase, step) - poly_blep(falling, step);
        self.advance(step);
        out
    }

    /// Band-limited rising sawtooth wave.
    pub fn saw(&mut self, freq: f64) -> f64 {
        let step = freq / SAMPLE_RATE;
        let out = 2.0 * self.phase - 1.0 - poly_blep(self.phase, step);
        self.advance(step);
        out
    }
}

/// Correction that rounds off the step of a square or saw wave over the two
/// samples around it. A hard step holds frequencies above half the sample
/// rate, which fold back down as harsh, out-of-tune tones.
fn poly_blep(phase: f64, step: f64) -> f64 {
    if phase < step {
        let x = phase / step;
        2.0 * x - x * x - 1.0
    } else if phase > 1.0 - step {
        let x = (phase - 1.0) / step;
        x * x + 2.0 * x + 1.0
    } else {
        0.0
    }
}

/// One resonance of a struck object: a decaying sine whose pitch starts high
/// and settles, the way a drum head does right after the hit.
#[derive(Debug, Clone)]
pub struct Mode {
    osc: Osc,
    freq: f64,
    tau: f64,
    bend: f64,
    bend_tau: f64,
}

impl Mode {
    /// A mode at `freq` hertz that decays with time constant `tau` seconds
    /// and is silent after about five times that.
    pub fn new(freq: f64, tau: f64) -> Self {
        Self {
            osc: Osc::new(),
            freq,
            tau,
            bend: 0.0,
            bend_tau: 1.0,
        }
    }

    /// Starts the pitch `amount` times higher (0.5 means 50% sharp) and lets
    /// it settle with time constant `tau` seconds.
    pub fn bend(mut self, amount: f64, tau: f64) -> Self {
        self.bend = amount;
        self.bend_tau = tau;
        self
    }

    /// The next sample. `t` is the time since the strike in seconds.
    pub fn tick(&mut self, t: f64) -> f64 {
        let freq = self.freq * (1.0 + self.bend * decay(t, self.bend_tau));
        self.osc.sine(freq) * decay_to(t, self.tau, 4.0 * self.tau)
    }
}

/// Adds up a set of modes, each scaled by the gain paired with it.
pub fn ring(modes: &mut [(f64, Mode)], t: f64) -> f64 {
    modes
        .iter_mut()
        .map(|(gain, mode)| *gain * mode.tick(t))
        .sum()
}

/// Ratios between the six square waves of a [`Metal`] cluster. None is a
/// simple fraction of another, so their harmonics never line up into a pitch.
const METAL_RATIOS: [f64; 6] = [1.0, 1.4071, 1.7411, 2.2882, 2.7061, 3.5713];

/// Level of each square wave in a [`Metal`] cluster, about one over the
/// square root of its ratio. A higher square puts stronger, sparser
/// harmonics into the treble, and at equal levels the top one would be heard
/// as a buzzing pitch. These levels give all six the same power up there.
const METAL_GAINS: [f64; 6] = [1.0, 0.843, 0.758, 0.661, 0.608, 0.529];

/// Six square waves at unrelated pitches. High-passed, their thousands of
/// clashing harmonics are the metallic core of the hats and cymbals.
#[derive(Debug, Clone)]
pub struct Metal {
    oscs: [Osc; 6],
    base: f64,
}

impl Metal {
    /// A cluster whose lowest square wave is at `base` hertz.
    pub fn new(base: f64) -> Self {
        // Spread the starting phases so the six edges do not all land on the
        // first sample.
        let oscs = std::array::from_fn(|index| Osc::at(index as f64 * 0.149));
        Self { oscs, base }
    }

    /// The next sample. At `ring = 0` the six squares are simply added,
    /// which is dense and bright. At `ring = 1` they are multiplied in pairs
    /// instead: multiplying makes sum and difference tones, which sounds
    /// thinner and more clangorous. Values in between blend the two.
    pub fn tick(&mut self, ring: f64) -> f64 {
        let base = self.base;
        let s: [f64; 6] =
            std::array::from_fn(|index| self.oscs[index].square(base * METAL_RATIOS[index]));
        let sum: f64 = s.iter().zip(METAL_GAINS).map(|(s, gain)| s * gain).sum();
        let product = s[0] * s[3] + s[1] * s[4] + s[2] * s[5];
        (1.0 - ring) * sum + ring * 2.0 * product
    }
}

/// State-variable filter (trapezoidal, after Andrew Simper's design). One
/// instance gives low-pass, band-pass or high-pass; call one of them per
/// sample. It stays stable when the cutoff changes every sample.
#[derive(Debug, Clone)]
pub struct Svf {
    k: f64,
    a1: f64,
    a2: f64,
    a3: f64,
    ic1: f64,
    ic2: f64,
}

impl Svf {
    /// A filter at `cutoff` hertz. `q` sets the resonance: 0.707 is flat,
    /// higher values ring.
    pub fn new(cutoff: f64, q: f64) -> Self {
        let mut filter = Self {
            k: 1.0 / q,
            a1: 0.0,
            a2: 0.0,
            a3: 0.0,
            ic1: 0.0,
            ic2: 0.0,
        };
        filter.set_cutoff(cutoff);
        filter
    }

    /// Moves the filter to `cutoff` hertz without disturbing its state.
    pub fn set_cutoff(&mut self, cutoff: f64) {
        let g = tan_turns(cutoff / SAMPLE_RATE / 2.0);
        self.a1 = 1.0 / (1.0 + g * (g + self.k));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
    }

    /// Advances one sample and returns (low, band, high).
    fn tick(&mut self, input: f64) -> (f64, f64, f64) {
        let v3 = input - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        (v2, v1, input - self.k * v1 - v2)
    }

    /// Low-pass: keeps what is below the cutoff, 12 dB per octave.
    pub fn lowpass(&mut self, input: f64) -> f64 {
        self.tick(input).0
    }

    /// Band-pass: keeps what is around the cutoff, with a peak gain of `q`.
    pub fn bandpass(&mut self, input: f64) -> f64 {
        self.tick(input).1
    }

    /// High-pass: keeps what is above the cutoff, 12 dB per octave.
    pub fn highpass(&mut self, input: f64) -> f64 {
        self.tick(input).2
    }
}

/// How far below its peak a sound has to fall before the rest is cut off.
const TRIM_DB: f64 = -60.0;

/// Longest fade applied to the end of a sound, in frames (10 ms).
const FADE_OUT_FRAMES: usize = 480;

/// Fade applied to the start of a sound, in frames (an eighth of a
/// millisecond). Short enough to leave the transient alone, long enough that
/// the first sample is zero.
const FADE_IN_FRAMES: usize = 6;

/// Cutoff of the filter that removes DC offset, in hertz.
const DC_BLOCK_HZ: f64 = 10.0;

/// Turns a raw render into a finished one-shot: removes DC offset, cuts the
/// tail where it falls below -60 dB, fades both ends to exact zero, and
/// scales the peak to `peak_db` dBFS.
pub fn finish(mut audio: Audio, peak_db: f64) -> Audio {
    let pole = exp(-std::f64::consts::TAU * DC_BLOCK_HZ / SAMPLE_RATE);
    for channel in audio.channels_mut() {
        let mut previous_in = 0.0;
        let mut previous_out = 0.0;
        for sample in channel.iter_mut() {
            let out = *sample - previous_in + pole * previous_out;
            previous_in = *sample;
            previous_out = out;
            *sample = out;
        }
    }

    let threshold = audio.peak() * db_to_gain(TRIM_DB);
    let frames = audio
        .channels()
        .iter()
        .filter_map(|channel| channel.iter().rposition(|sample| sample.abs() > threshold))
        .max()
        .map_or(0, |last| last + 1);
    let fade_out = FADE_OUT_FRAMES.min(frames / 2);
    for channel in audio.channels_mut() {
        channel.truncate(frames);
        for (index, sample) in channel.iter_mut().take(FADE_IN_FRAMES).enumerate() {
            *sample *= smoothstep(index as f64 / FADE_IN_FRAMES as f64);
        }
        for (index, sample) in channel.iter_mut().rev().take(fade_out).enumerate() {
            *sample *= smoothstep(index as f64 / fade_out as f64);
        }
    }

    // Normalize last: the fades can shave the peak.
    let peak = audio.peak();
    if peak > 0.0 {
        let gain = db_to_gain(peak_db) / peak;
        for channel in audio.channels_mut() {
            for sample in channel.iter_mut() {
                *sample *= gain;
            }
        }
    }
    audio
}

/// Smooth S-curve from 0 at `x = 0` to 1 at `x = 1`.
fn smoothstep(x: f64) -> f64 {
    x * x * (3.0 - 2.0 * x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(samples: &[f64]) -> f64 {
        (samples.iter().map(|sample| sample * sample).sum::<f64>() / samples.len() as f64).sqrt()
    }

    #[test]
    fn noise_is_reproducible_and_centered() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        let mut other = Rng::new(8);
        let first: Vec<f64> = (0..50_000).map(|_| a.white()).collect();
        assert!(first.iter().all(|&sample| sample == b.white()));
        assert!(first.iter().any(|&sample| sample != other.white()));
        assert!(first.iter().all(|sample| (-1.0..1.0).contains(sample)));
        let mean = first.iter().sum::<f64>() / first.len() as f64;
        assert!(mean.abs() < 0.01, "mean {mean}");
        // Uniform noise in -1..1 has an RMS of 1/sqrt(3).
        assert!((rms(&first) - 0.577).abs() < 0.01);
    }

    #[test]
    fn noise_sequence_is_pinned() {
        // Changing the generator would silently change every noisy sound.
        let mut rng = Rng::new(1);
        let first: Vec<u64> = (0..3).map(|_| rng.next_u64()).collect();
        assert_eq!(
            first,
            [
                0x4B46_A55D_F361_1B9B,
                0xD7E1_F141_0E76_3EF4,
                0x5F14_EC66_975F_9B06
            ]
        );
    }

    #[test]
    fn sine_oscillator_has_the_right_period() {
        let mut osc = Osc::new();
        let wave = synth(0.1, |_| osc.sine(480.0));
        // 480 Hz at 48 kHz repeats every 100 samples.
        assert_eq!(wave[0], 0.0);
        assert!((wave[25] - 1.0).abs() < 1e-9);
        assert!((wave[1_000] - wave[0]).abs() < 1e-9);
    }

    #[test]
    fn square_and_saw_stay_bounded_and_centered() {
        let mut square = Osc::new();
        let mut saw = Osc::new();
        let squares = synth(1.0, |_| square.square(441.3));
        let saws = synth(1.0, |_| saw.saw(441.3));
        for wave in [&squares, &saws] {
            assert!(wave.iter().all(|sample| sample.abs() <= 1.01));
            let mean = wave.iter().sum::<f64>() / wave.len() as f64;
            assert!(mean.abs() < 0.01, "mean {mean}");
        }
        assert!((rms(&squares) - 1.0).abs() < 0.05);
        assert!((rms(&saws) - 0.577).abs() < 0.03);
    }

    #[test]
    fn filter_passes_and_stops_the_right_bands() {
        let tone = |freq: f64| {
            let mut osc = Osc::new();
            synth(0.5, move |_| osc.sine(freq))
        };
        let through = |mut run: Box<dyn FnMut(f64) -> f64>, freq: f64| {
            let filtered: Vec<f64> = tone(freq).into_iter().map(&mut run).collect();
            rms(&filtered[12_000..]) / std::f64::consts::FRAC_1_SQRT_2
        };
        let lowpass = |freq| {
            let mut filter = Svf::new(1_000.0, 0.707);
            through(Box::new(move |x| filter.lowpass(x)), freq)
        };
        let highpass = |freq| {
            let mut filter = Svf::new(1_000.0, 0.707);
            through(Box::new(move |x| filter.highpass(x)), freq)
        };
        let bandpass = |freq| {
            let mut filter = Svf::new(1_000.0, 4.0);
            through(Box::new(move |x| filter.bandpass(x)), freq)
        };
        assert!((lowpass(100.0) - 1.0).abs() < 0.01);
        assert!((lowpass(1_000.0) - 0.707).abs() < 0.01);
        assert!(lowpass(10_000.0) < 0.01);
        assert!((highpass(10_000.0) - 1.0).abs() < 0.01);
        assert!(highpass(100.0) < 0.011);
        // The band-pass peaks at Q and falls away on both sides.
        assert!((bandpass(1_000.0) - 4.0).abs() < 0.05);
        assert!(bandpass(100.0) < 0.15);
        assert!(bandpass(10_000.0) < 0.15);
    }

    #[test]
    fn envelopes_have_the_documented_shapes() {
        assert_eq!(decay(0.0, 0.1), 1.0);
        assert!((decay(0.1, 0.1) - (-1.0f64).exp()).abs() < 1e-12);
        assert!(decay_to(0.05, 0.1, 0.2) < decay(0.05, 0.1));
        assert!(decay_to(0.6, 0.1, 0.2) < 1e-6);
        assert_eq!(attack(0.0, 0.01), 0.0);
        assert!(attack(0.1, 0.01) > 0.999);
        assert_eq!(burst(0.01, 0.02, 0.1), 0.0);
        assert_eq!(burst(0.02, 0.02, 0.1), 1.0);
    }

    #[test]
    fn finish_trims_fades_and_normalizes() {
        let mut osc = Osc::new();
        // A decaying tone riding on a DC offset, followed by a long silence.
        let raw = synth(2.0, |t| 0.05 + osc.sine(300.0) * decay(t, 0.05));
        let Audio::Mono(done) = finish(Audio::Mono(raw), -6.0) else {
            panic!("mono in, mono out");
        };
        let peak = done.iter().fold(0.0f64, |peak, s| s.abs().max(peak));
        assert!((peak - 0.501_187).abs() < 1e-5, "peak {peak}");
        assert_eq!(done[0], 0.0);
        assert_eq!(done[done.len() - 1], 0.0);
        // -60 dB of a 50 ms decay is 345 ms in; the DC step's own decay adds
        // a little.
        assert!((14_000..30_000).contains(&done.len()), "{}", done.len());
        let mean = done.iter().sum::<f64>() / done.len() as f64;
        assert!(mean.abs() < 2e-3, "mean {mean}");
    }

    #[test]
    fn finish_keeps_stereo_channels_the_same_length() {
        let mut osc = Osc::new();
        let left = synth(1.0, |t| osc.sine(500.0) * decay(t, 0.02));
        let right = synth(1.0, |t| osc.sine(700.0) * decay(t, 0.08));
        let Audio::Stereo(left, right) = finish(Audio::Stereo(left, right), -1.0) else {
            panic!("stereo in, stereo out");
        };
        assert_eq!(left.len(), right.len());
        assert_eq!(right[right.len() - 1], 0.0);
    }
}
