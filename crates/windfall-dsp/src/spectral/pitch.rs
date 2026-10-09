use super::{audio, output, rate};
use crate::balance::Controls;
use crate::blocks::biquad::{Biquad, BiquadCoeffs};
use crate::blocks::delay_line::DelayLine;
use crate::blocks::math::smoothing_coefficient;
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

const GRAIN_SPAN: usize = 2048;
/// Fixed grain-centre/dry-path delay; wet grains span 2..2050 samples.
pub const PITCH_SHIFT_LATENCY: usize = 2 + GRAIN_SPAN / 2;
const TRACK_SIZE: usize = 256;
const TRACK_HOP: usize = 32;
const TRACK_LAGS: usize = 152;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct PitchShiftParams {
    /// -24..24 semitones; ratio = 2^(semitones/12).
    pub semitones: f32,
    pub mix: f32,
}
impl Default for PitchShiftParams {
    fn default() -> Self {
        Self {
            semitones: 0.0,
            mix: 1.0,
        }
    }
}
param_set!(PitchShiftParams, "Pitch Shift", {
    float [semitones] "semitones" "Shift" { Semitones, Linear, -24.0, 24.0, 0.0 }
    float [mix] "mix" "Mix" { Fraction, Linear, 0.0, 1.0, 1.0 }
});

/// Two cubic fractional read heads with complementary raised-cosine windows.
/// Their linked stereo phase advances by 1-ratio samples per input sample.
struct Grains {
    lines: [DelayLine; 2],
    phase: f64,
    extra_delay: usize,
}
impl Default for Grains {
    fn default() -> Self {
        Self {
            lines: std::array::from_fn(|_| DelayLine::new(GRAIN_SPAN + 2)),
            phase: 0.5,
            extra_delay: 0,
        }
    }
}
impl Grains {
    fn prepare(&mut self, extra_delay: usize) {
        self.extra_delay = extra_delay;
        self.lines = std::array::from_fn(|_| DelayLine::new(extra_delay + GRAIN_SPAN + 2));
        self.reset();
    }
    fn reset(&mut self) {
        for line in &mut self.lines {
            line.clear();
        }
        self.phase = 0.5;
    }
    fn tick(&mut self, input: [f32; 2], ratio: f32) -> ([f32; 2], [f32; 2]) {
        let p = self.phase as f32;
        let q = (p + 0.5).fract();
        let weight = 0.5 - 0.5 * (std::f32::consts::TAU * p).cos();
        let a = (self.extra_delay + 2) as f32 + p * GRAIN_SPAN as f32;
        let b = (self.extra_delay + 2) as f32 + q * GRAIN_SPAN as f32;
        let mut wet = [0.0; 2];
        let mut dry = [0.0; 2];
        // Collapse onto the fixed delay at unison, including after automation.
        let shifted = ((ratio - 1.0).abs() * 10_000.0).min(1.0);
        for ch in 0..2 {
            dry[ch] = self.lines[ch].tap(self.extra_delay + PITCH_SHIFT_LATENCY);
            let grain =
                self.lines[ch].tap_cubic(a) * weight + self.lines[ch].tap_cubic(b) * (1.0 - weight);
            wet[ch] = dry[ch] + (grain - dry[ch]) * shifted;
            self.lines[ch].push(input[ch]);
        }
        self.phase = (self.phase + (1.0 - f64::from(ratio)) / GRAIN_SPAN as f64).rem_euclid(1.0);
        (wet, dry)
    }
    fn latency(&self) -> usize {
        self.extra_delay + PITCH_SHIFT_LATENCY
    }
    fn tail(&self) -> usize {
        self.extra_delay + GRAIN_SPAN + 4
    }
}

pub struct PitchShift {
    grains: Grains,
    controls: Controls<2>,
}
impl Default for PitchShift {
    fn default() -> Self {
        Self {
            grains: Grains::default(),
            controls: Controls::new([0.0, 1.0]),
        }
    }
}
impl Effect for PitchShift {
    type Params = PitchShiftParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.grains.prepare(0);
        self.controls.prepare(rate(sample_rate));
    }
    fn reset(&mut self) {
        self.grains.reset();
        self.controls.reset();
    }
    fn set_params(&mut self, params: &Self::Params) {
        let p = params.sanitized();
        self.controls.set([p.semitones, p.mix]);
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let [semitones, mix] = self.controls.tick();
            let (wet, dry) = self
                .grains
                .tick([audio(*l), audio(*r)], (semitones / 12.0).exp2());
            *l = output(dry[0] + (wet[0] - dry[0]) * mix);
            *r = output(dry[1] + (wet[1] - dry[1]) * mix);
        }
    }
    fn latency_samples(&self) -> usize {
        self.grains.latency()
    }
    fn tail_samples(&self) -> usize {
        self.grains.tail()
    }
    fn gap_samples(&self) -> usize {
        self.grains.tail()
    }
    fn warm_up_samples(&self) -> usize {
        self.grains.tail()
    }
    fn delay_readiness_samples(&self) -> usize {
        self.grains.tail()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum PitchScale {
    #[default]
    Chromatic,
    Major,
    Minor,
    Pentatonic,
}
impl PitchScale {
    fn mask(self) -> u16 {
        match self {
            Self::Chromatic => 0x0fff,
            Self::Major => 0x0ab5,
            Self::Minor => 0x05ad,
            Self::Pentatonic => 0x0295,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct PitchCorrectParams {
    /// Pitch class, C=0..B=11.
    pub root: u8,
    pub scale: PitchScale,
    /// Time constant in ms, 0..500; zero snaps on each voiced estimate.
    pub speed_ms: f32,
    /// Correction share, 0..1.
    pub amount: f32,
    /// A4 tuning, 400..480 Hz.
    pub tuning_hz: f32,
    pub mix: f32,
}
impl Default for PitchCorrectParams {
    fn default() -> Self {
        Self {
            root: 0,
            scale: PitchScale::Chromatic,
            speed_ms: 35.0,
            amount: 1.0,
            tuning_hz: 440.0,
            mix: 1.0,
        }
    }
}
param_set!(PitchCorrectParams, "Pitch Correct", {
    int [root] "root" "Root" { None, 0, 11, 0 }
    choice [scale] "scale" "Scale" { PitchScale, Chromatic, [Chromatic "chromatic" "Chromatic", Major "major" "Major", Minor "minor" "Minor", Pentatonic "pentatonic" "Pentatonic"] }
    float [speed_ms] "speedMs" "Speed" { Milliseconds, Linear, 0.0, 500.0, 35.0 }
    float [amount] "amount" "Amount" { Fraction, Linear, 0.0, 1.0, 1.0 }
    float [tuning_hz] "tuningHz" "Tuning" { Hertz, Linear, 400.0, 480.0, 440.0 }
    float [mix] "mix" "Mix" { Fraction, Linear, 0.0, 1.0, 1.0 }
});

/// Monophonic normalized autocorrelation at approximately 6 kHz. A bounded
/// 256-sample ring, 32-sample hop and <=151 lags cost no worker handoff/locks.
pub struct PitchCorrect {
    params: PitchCorrectParams,
    controls: Controls<4>,
    grains: Grains,
    sample_rate: f32,
    decimation: usize,
    decimation_cursor: usize,
    tracker_rate: f32,
    lowpass: [Biquad; 2],
    lowpass_coeffs: BiquadCoeffs,
    tracker: [f32; TRACK_SIZE],
    tracker_cursor: usize,
    tracked: usize,
    hop: usize,
    frequency: f32,
    correction: f32,
}
impl Default for PitchCorrect {
    fn default() -> Self {
        let mut effect = Self {
            params: PitchCorrectParams::default(),
            controls: Controls::new([35.0, 1.0, 440.0, 1.0]),
            grains: Grains::default(),
            sample_rate: 48_000.0,
            decimation: 8,
            decimation_cursor: 0,
            tracker_rate: 6_000.0,
            lowpass: [Biquad::default(); 2],
            lowpass_coeffs: BiquadCoeffs::IDENTITY,
            tracker: [0.0; TRACK_SIZE],
            tracker_cursor: 0,
            tracked: 0,
            hop: 0,
            frequency: 0.0,
            correction: 0.0,
        };
        effect.prepare(48_000.0, 1);
        effect
    }
}
impl PitchCorrect {
    fn estimate(&mut self) {
        let mut signal = [0.0; TRACK_SIZE];
        let mean = self.tracker.iter().sum::<f32>() / TRACK_SIZE as f32;
        for (i, sample) in signal.iter_mut().enumerate() {
            *sample = self.tracker[(self.tracker_cursor + i) % TRACK_SIZE] - mean;
        }
        let power = signal.iter().map(|v| v * v).sum::<f32>() / TRACK_SIZE as f32;
        if power < 1.0e-7 {
            self.frequency = 0.0;
            return;
        }
        let min_lag = (self.tracker_rate / 1000.0).floor().max(2.0) as usize;
        let max_lag = (self.tracker_rate / 60.0)
            .ceil()
            .min((TRACK_LAGS - 2) as f32) as usize;
        let mut correlation = [0.0; TRACK_LAGS];
        for lag in min_lag.saturating_sub(1)..=max_lag + 1 {
            let mut cross = 0.0;
            let mut a = 0.0;
            let mut b = 0.0;
            for i in 0..TRACK_SIZE - lag {
                cross += signal[i] * signal[i + lag];
                a += signal[i] * signal[i];
                b += signal[i + lag] * signal[i + lag];
            }
            correlation[lag] = cross / (a * b).sqrt().max(1.0e-12);
        }
        // Prefer the earliest convincing peak, rather than a longer multiple
        // of the period. Reject unvoiced/aperiodic input without holding a note.
        let mut best = 0;
        for lag in min_lag..=max_lag {
            if correlation[lag] >= 0.75
                && correlation[lag] >= correlation[lag - 1]
                && correlation[lag] > correlation[lag + 1]
            {
                best = lag;
                break;
            }
        }
        if best == 0 {
            self.frequency = 0.0;
            return;
        }
        let a = correlation[best - 1];
        let b = correlation[best];
        let c = correlation[best + 1];
        let denominator = a - 2.0 * b + c;
        let fraction = if denominator.abs() > 1.0e-6 {
            (0.5 * (a - c) / denominator).clamp(-0.5, 0.5)
        } else {
            0.0
        };
        self.frequency = self.tracker_rate / (best as f32 + fraction);
    }
    fn target(&self, tuning: f32) -> f32 {
        if self.frequency <= 0.0 {
            return 0.0;
        }
        let note = 69.0 + 12.0 * (self.frequency / tuning).log2();
        let centre = note.round() as i32;
        let mut nearest = centre;
        let mut distance = f32::INFINITY;
        for candidate in centre - 12..=centre + 12 {
            let class = (candidate - i32::from(self.params.root)).rem_euclid(12) as u32;
            if self.params.scale.mask() & (1_u16 << class) != 0 {
                let d = (candidate as f32 - note).abs();
                if d < distance {
                    nearest = candidate;
                    distance = d;
                }
            }
        }
        (nearest as f32 - note).clamp(-12.0, 12.0)
    }
}
impl Effect for PitchCorrect {
    type Params = PitchCorrectParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.sample_rate = rate(sample_rate);
        self.decimation = (self.sample_rate / 6000.0).round().max(1.0) as usize;
        self.tracker_rate = self.sample_rate / self.decimation as f32;
        self.lowpass_coeffs =
            BiquadCoeffs::low_pass(self.tracker_rate * 0.4, 0.707, self.sample_rate);
        self.grains.prepare(TRACK_SIZE * self.decimation);
        self.controls.prepare(self.sample_rate);
        self.reset();
    }
    fn reset(&mut self) {
        self.grains.reset();
        self.controls.reset();
        self.lowpass.fill(Biquad::default());
        self.tracker.fill(0.0);
        self.tracker_cursor = 0;
        self.decimation_cursor = 0;
        self.tracked = 0;
        self.hop = 0;
        self.frequency = 0.0;
        self.correction = 0.0;
    }
    fn set_params(&mut self, params: &Self::Params) {
        self.params = params.sanitized();
        let p = self.params;
        self.controls
            .set([p.speed_ms, p.amount, p.tuning_hz, p.mix]);
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let input = [audio(*l), audio(*r)];
            let mut filtered = f64::from(0.5 * (input[0] + input[1]));
            for section in &mut self.lowpass {
                filtered = section.tick(&self.lowpass_coeffs, filtered);
                section.flush();
            }
            self.decimation_cursor += 1;
            if self.decimation_cursor == self.decimation {
                self.decimation_cursor = 0;
                self.tracker[self.tracker_cursor] = output(filtered as f32);
                self.tracker_cursor = (self.tracker_cursor + 1) % TRACK_SIZE;
                self.tracked = (self.tracked + 1).min(TRACK_SIZE);
                self.hop += 1;
                if self.hop == TRACK_HOP {
                    self.hop = 0;
                    if self.tracked == TRACK_SIZE {
                        self.estimate();
                    }
                }
            }
            let [speed, amount, tuning, mix] = self.controls.tick();
            let target = self.target(tuning) * amount;
            self.correction +=
                (target - self.correction) * smoothing_coefficient(speed, self.sample_rate);
            let (wet, dry) = self.grains.tick(input, (self.correction / 12.0).exp2());
            *l = output(dry[0] + (wet[0] - dry[0]) * mix);
            *r = output(dry[1] + (wet[1] - dry[1]) * mix);
        }
    }
    fn latency_samples(&self) -> usize {
        self.grains.latency()
    }
    fn tail_samples(&self) -> usize {
        self.grains.tail()
    }
    fn gap_samples(&self) -> usize {
        self.grains.tail()
    }
    fn warm_up_samples(&self) -> usize {
        self.grains.tail()
    }
    fn delay_readiness_samples(&self) -> usize {
        self.grains.tail()
    }
}
