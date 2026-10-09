use super::{audio, output, rate};
use crate::balance::Controls;
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

const TAPS: usize = 255;
pub const FREQUENCY_SHIFTER_LATENCY: usize = (TAPS - 1) / 2;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct FrequencyShifterParams {
    /// Additive frequency translation, -5000..5000 Hz. Positive shifts up.
    pub shift_hz: f32,
    pub mix: f32,
}
impl Default for FrequencyShifterParams {
    fn default() -> Self {
        Self {
            shift_hz: 0.0,
            mix: 1.0,
        }
    }
}
param_set!(FrequencyShifterParams, "Frequency Shifter", {
    float [shift_hz] "shiftHz" "Shift" { Hertz, Linear, -5000.0, 5000.0, 0.0 }
    float [mix] "mix" "Mix" { Fraction, Linear, 0.0, 1.0, 1.0 }
});

/// Windowed FIR analytic signal followed by complex sinusoidal modulation.
/// The in-phase signal and dry path share the Hilbert FIR group delay.
pub struct FrequencyShifter {
    coefficients: [f32; TAPS],
    history: [[f32; 256]; 2],
    cursor: usize,
    phase: f64,
    sample_rate: f32,
    controls: Controls<2>,
}
impl Default for FrequencyShifter {
    fn default() -> Self {
        Self {
            coefficients: std::array::from_fn(|i| {
                let n = i as i32 - FREQUENCY_SHIFTER_LATENCY as i32;
                if n == 0 || n % 2 == 0 {
                    0.0
                } else {
                    let window = 0.42
                        - 0.5 * (std::f32::consts::TAU * i as f32 / (TAPS - 1) as f32).cos()
                        + 0.08 * (2.0 * std::f32::consts::TAU * i as f32 / (TAPS - 1) as f32).cos();
                    2.0 * window / (std::f32::consts::PI * n as f32)
                }
            }),
            history: [[0.0; 256]; 2],
            cursor: 0,
            phase: 0.0,
            sample_rate: 48_000.0,
            controls: Controls::new([0.0, 1.0]),
        }
    }
}
impl Effect for FrequencyShifter {
    type Params = FrequencyShifterParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.sample_rate = rate(sample_rate);
        self.controls.prepare(self.sample_rate);
        self.reset();
    }
    fn reset(&mut self) {
        self.history.fill([0.0; 256]);
        self.cursor = 0;
        self.phase = 0.0;
        self.controls.reset();
    }
    fn set_params(&mut self, params: &Self::Params) {
        let p = params.sanitized();
        self.controls.set([p.shift_hz, p.mix]);
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let [shift, mix] = self.controls.tick();
            let (sin, cos) = self.phase.sin_cos();
            for (ch, sample) in [l, r].into_iter().enumerate() {
                self.history[ch][self.cursor] = audio(*sample);
                let real =
                    self.history[ch][self.cursor.wrapping_sub(FREQUENCY_SHIFTER_LATENCY) & 255];
                let mut imaginary = 0.0;
                for (i, coefficient) in self.coefficients.iter().enumerate() {
                    imaginary += coefficient * self.history[ch][self.cursor.wrapping_sub(i) & 255];
                }
                let wet = real * cos as f32 - imaginary * sin as f32;
                *sample = output(real + (wet - real) * mix);
            }
            self.cursor = (self.cursor + 1) & 255;
            let step = f64::from(shift.clamp(-0.45 * self.sample_rate, 0.45 * self.sample_rate))
                / f64::from(self.sample_rate);
            self.phase =
                (self.phase + std::f64::consts::TAU * step).rem_euclid(std::f64::consts::TAU);
        }
    }
    fn latency_samples(&self) -> usize {
        FREQUENCY_SHIFTER_LATENCY
    }
    fn tail_samples(&self) -> usize {
        TAPS - 1
    }
    fn gap_samples(&self) -> usize {
        TAPS - 1
    }
    fn warm_up_samples(&self) -> usize {
        TAPS - 1
    }
    fn delay_readiness_samples(&self) -> usize {
        TAPS - 1
    }
}
