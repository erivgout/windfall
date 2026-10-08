//! A continuously swept, damped feedback comb. No tap crossfades: pitch motion
//! is intentional. Convex linear interpolation keeps the feedback contraction.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{Controls, advance, rate, signal, sine};
use crate::blocks::delay_line::DelayLine;
use crate::blocks::math::flush;
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};

const MAX_DELAY_MS: f32 = 20.0;
const MAX_FEEDBACK: f32 = 0.9;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct FlangerParams {
    /// Minimum delay, 0.1..=10 ms; default 1. At least one frame at any rate.
    pub delay_ms: f32,
    /// Sweep above the minimum, 0..=10 ms; default 3.
    pub depth_ms: f32,
    /// Free-running rate, 0..=5 Hz; default 0.25. Zero freezes phase.
    pub rate_hz: f32,
    /// Right-channel phase offset, cycles 0..=1; default 0.25.
    pub stereo_phase: f32,
    /// Signed feedback, -0.9..=0.9; default 0.5.
    pub feedback: f32,
    /// Feedback/wet lowpass amount, 0..=1; default 0.2. Pole is 0.9 * damping.
    pub damping: f32,
    /// Sine-to-triangle blend, 0..=1; default 0.
    pub shape: f32,
    /// Invert the wet output independently of the feedback sign; default false.
    pub invert_wet: bool,
    /// Wet share, 0..=1; default 0.5.
    pub mix: f32,
}

impl Default for FlangerParams {
    fn default() -> Self {
        Self {
            delay_ms: 1.0,
            depth_ms: 3.0,
            rate_hz: 0.25,
            stereo_phase: 0.25,
            feedback: 0.5,
            damping: 0.2,
            shape: 0.0,
            invert_wet: false,
            mix: 0.5,
        }
    }
}

param_set!(FlangerParams, "Flanger", {
    float [delay_ms] "delayMs" "Delay" { Milliseconds, Linear, 0.1, 10.0, 1.0 }
    float [depth_ms] "depthMs" "Depth" { Milliseconds, Linear, 0.0, 10.0, 3.0 }
    float [rate_hz] "rateHz" "Rate" { Hertz, Linear, 0.0, 5.0, 0.25 }
    float [stereo_phase] "stereoPhase" "Stereo phase (cycles)" { Fraction, Linear, 0.0, 1.0, 0.25 }
    float [feedback] "feedback" "Feedback" { Fraction, Linear, -MAX_FEEDBACK, MAX_FEEDBACK, 0.5 }
    float [damping] "damping" "Damping" { Fraction, Linear, 0.0, 1.0, 0.2 }
    float [shape] "shape" "Sine to triangle" { Fraction, Linear, 0.0, 1.0, 0.0 }
    toggle [invert_wet] "invertWet" "Invert wet" { false }
    float [mix] "mix" "Mix" { Fraction, Linear, 0.0, 1.0, 0.5 }
});

pub struct Flanger {
    params: FlangerParams,
    sample_rate: f32,
    controls: Controls<9>,
    lines: [DelayLine; 2],
    lowpass: [f32; 2],
    phase: f64,
    prepared: bool,
}

impl Default for Flanger {
    fn default() -> Self {
        Self {
            params: FlangerParams::default(),
            sample_rate: 48_000.0,
            controls: Controls::new(),
            lines: std::array::from_fn(|_| DelayLine::default()),
            lowpass: [0.0; 2],
            phase: 0.0,
            prepared: false,
        }
    }
}

impl Flanger {
    fn update(&mut self) {
        let p = self.params;
        self.controls.set([
            p.delay_ms,
            p.depth_ms,
            p.rate_hz,
            p.stereo_phase,
            p.feedback,
            p.damping,
            p.shape,
            if p.invert_wet { -1.0 } else { 1.0 },
            p.mix,
        ]);
    }

    fn horizon(&self) -> usize {
        (f64::from(MAX_DELAY_MS) * 0.001 * f64::from(self.sample_rate))
            .ceil()
            .max(1.0) as usize
            + 1
    }
}

impl Effect for Flanger {
    type Params = FlangerParams;

    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.sample_rate = rate(sample_rate);
        let horizon = self.horizon();
        self.lines = std::array::from_fn(|_| DelayLine::new(horizon));
        self.controls.prepare(self.sample_rate);
        self.prepared = true;
        self.reset();
    }

    fn reset(&mut self) {
        for line in &mut self.lines {
            line.clear();
        }
        self.lowpass = [0.0; 2];
        self.phase = 0.0;
        self.controls.reset();
        self.update();
    }

    fn set_params(&mut self, params: &FlangerParams) {
        self.params = params.sanitized();
        self.update();
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (left, right) in left.iter_mut().zip(right) {
            let input = [signal(*left), signal(*right)];
            if !self.prepared {
                [*left, *right] = input;
                continue;
            }
            let [
                delay,
                depth,
                hz,
                stereo,
                feedback,
                damping,
                shape,
                polarity,
                mix,
            ] = self.controls.tick();
            let mut output = [0.0; 2];
            for channel in 0..2 {
                let phase = self.phase + f64::from(stereo) * channel as f64;
                let triangle = (4.0 * ((phase + 0.75).fract() - 0.5).abs() - 1.0) as f32;
                let lfo = sine(phase) * (1.0 - shape) + triangle * shape;
                let tap = ((f64::from(delay) + f64::from(depth) * (0.5 + 0.5 * f64::from(lfo)))
                    * 0.001
                    * f64::from(self.sample_rate)) as f32;
                let delayed = self.lines[channel].tap_linear(tap.max(1.0));
                self.lowpass[channel] =
                    flush(delayed + 0.9 * damping * (self.lowpass[channel] - delayed));
                self.lines[channel].push(flush(input[channel] + feedback * self.lowpass[channel]));
                let wet = polarity * self.lowpass[channel];
                output[channel] = input[channel] + mix * (wet - input[channel]);
            }
            advance(&mut self.phase, hz, self.sample_rate);
            [*left, *right] = output;
        }
    }

    fn warm_up_samples(&self) -> usize {
        self.horizon() + 7
    }
    fn delay_readiness_samples(&self) -> usize {
        self.horizon() + 7
    }
    fn gap_samples(&self) -> usize {
        self.horizon() + 7
    }
    fn tail_samples(&self) -> usize {
        // After H frames every delay cell is <= .9 M under zero input. Seven
        // more frames contract the filter bound to q M, q=.9+.1*.9^7.
        // Input is <=1000, so every state is <=1000/(1-.9)=10000.
        // Repeat until the worst state is below 1e-6, regardless of modulation.
        let q = 0.9_f64 + 0.1 * 0.9_f64.powi(7);
        let repeats = ((1e-6_f64 / 10_000.0).ln() / q.ln()).ceil() as usize;
        (self.horizon() + 7) * repeats
    }
}
