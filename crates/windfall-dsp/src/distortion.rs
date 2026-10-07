//! Four-times oversampled drive, morphing from rational saturation to clipping.
//!
//! Two 129-tap Blackman-windowed sinc filters suppress images before the
//! nonlinearity and aliases before decimation. Fixed arrays and polyphase
//! interpolation keep every audio-side call allocation-free.
use crate::balance::{Controls, audio};
use crate::blocks::math::{db_to_gain, flush};
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

const FACTOR: usize = 4;
const TAPS: usize = 129;
const INPUT_TAPS: usize = TAPS.div_ceil(FACTOR);
pub const DISTORTION_LATENCY_SAMPLES: usize = (TAPS - 1) / FACTOR;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct DistortionParams {
    /// Input drive in dB, 0 to 36; default 12.
    pub drive_db: f32,
    /// Rational saturation (0) through hard clipping (1); default 0.5.
    pub shape: f32,
    /// Output trim in dB, -24 to 0; default -6.
    pub output_db: f32,
}
impl Default for DistortionParams {
    fn default() -> Self {
        Self {
            drive_db: 12.0,
            shape: 0.5,
            output_db: -6.0,
        }
    }
}
param_set!(DistortionParams, "Drive distortion", {
    float [drive_db] "driveDb" "Drive" { Decibels, Linear, 0.0, 36.0, 12.0 }
    float [shape] "shape" "Shape" { Fraction, Linear, 0.0, 1.0, 0.5 }
    float [output_db] "outputDb" "Output" { Decibels, Linear, -24.0, 0.0, -6.0 }
});
impl DistortionParams {
    /// The settled nonlinearity, before oversampling filters.
    pub fn transfer(&self, input: f32) -> f32 {
        let p = self.sanitized();
        (shape(
            f64::from(audio(input) * db_to_gain(p.drive_db)),
            f64::from(p.shape),
        ) * f64::from(db_to_gain(p.output_db))) as f32
    }
}
fn shape(input: f64, amount: f64) -> f64 {
    let soft = input / (1.0 + input.abs());
    soft + (input.clamp(-1.0, 1.0) - soft) * amount
}
struct Channel {
    input: [f64; INPUT_TAPS],
    high: [f64; TAPS],
    input_at: usize,
    high_at: usize,
}
impl Default for Channel {
    fn default() -> Self {
        Self {
            input: [0.0; INPUT_TAPS],
            high: [0.0; TAPS],
            input_at: 0,
            high_at: 0,
        }
    }
}
impl Channel {
    fn reset(&mut self) {
        self.input.fill(0.0);
        self.high.fill(0.0);
        self.input_at = 0;
        self.high_at = 0;
    }
    fn tick(
        &mut self,
        input: f32,
        drive: f32,
        amount: f32,
        up: &[[f64; INPUT_TAPS]; FACTOR],
        down: &[f64; TAPS],
    ) -> f32 {
        self.input[self.input_at] = f64::from(input);
        let mut output = 0.0;
        for (phase, coefficients) in up.iter().enumerate() {
            let mut interpolated = 0.0;
            for (k, coefficient) in coefficients.iter().enumerate() {
                interpolated +=
                    coefficient * self.input[(self.input_at + INPUT_TAPS - k) % INPUT_TAPS];
            }
            self.high[self.high_at] = shape(interpolated * f64::from(drive), f64::from(amount));
            if phase == 0 {
                for (k, coefficient) in down.iter().enumerate() {
                    output += coefficient * self.high[(self.high_at + TAPS - k) % TAPS];
                }
            }
            self.high_at = (self.high_at + 1) % TAPS;
        }
        self.input_at = (self.input_at + 1) % INPUT_TAPS;
        flush(output as f32)
    }
}
pub struct Distortion {
    controls: Controls<3>,
    channels: [Channel; 2],
    up: [[f64; INPUT_TAPS]; FACTOR],
    down: [f64; TAPS],
}
impl Default for Distortion {
    fn default() -> Self {
        Self {
            controls: Controls::new([db_to_gain(12.0), 0.5, db_to_gain(-6.0)]),
            channels: std::array::from_fn(|_| Channel::default()),
            up: [[0.0; INPUT_TAPS]; FACTOR],
            down: [0.0; TAPS],
        }
    }
}
impl Effect for Distortion {
    type Params = DistortionParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        // 90% of base-rate Nyquist. Coefficients are rate-independent.
        let cutoff = 0.45 / FACTOR as f64;
        for (n, coefficient) in self.down.iter_mut().enumerate() {
            let x = n as f64 - (TAPS - 1) as f64 * 0.5;
            let sinc = if x == 0.0 {
                2.0 * cutoff
            } else {
                (std::f64::consts::TAU * cutoff * x).sin() / (std::f64::consts::PI * x)
            };
            let phase = std::f64::consts::TAU * n as f64 / (TAPS - 1) as f64;
            *coefficient = sinc * (0.42 - 0.5 * phase.cos() + 0.08 * (2.0 * phase).cos());
        }
        let sum: f64 = self.down.iter().sum();
        for coefficient in &mut self.down {
            *coefficient /= sum;
        }
        self.up.fill([0.0; INPUT_TAPS]);
        for (n, coefficient) in self.down.iter().enumerate() {
            self.up[n % FACTOR][n / FACTOR] = *coefficient;
        }
        for phase in &mut self.up {
            let sum: f64 = phase.iter().sum();
            for coefficient in phase {
                *coefficient /= sum;
            }
        }
        self.controls.prepare(sample_rate);
        self.reset();
    }
    fn reset(&mut self) {
        self.controls.reset();
        for channel in &mut self.channels {
            channel.reset();
        }
    }
    fn set_params(&mut self, params: &Self::Params) {
        let p = params.sanitized();
        self.controls
            .set([db_to_gain(p.drive_db), p.shape, db_to_gain(p.output_db)]);
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let [drive, amount, gain] = self.controls.tick();
            for (side, sample) in [l, r].into_iter().enumerate() {
                *sample = flush(
                    self.channels[side].tick(audio(*sample), drive, amount, &self.up, &self.down)
                        * gain,
                );
            }
        }
    }
    fn latency_samples(&self) -> usize {
        DISTORTION_LATENCY_SAMPLES
    }
    fn tail_samples(&self) -> usize {
        2 * DISTORTION_LATENCY_SAMPLES
    }
    fn gap_samples(&self) -> usize {
        self.tail_samples()
    }
}
