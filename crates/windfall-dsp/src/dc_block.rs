//! A normalized first-order high-pass with a controllable DC cutoff.
use crate::balance::{Controls, audio, rate};
use crate::blocks::math::{flush, flush64};
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct DcBlockParams {
    /// DC cutoff in Hz, 1 to 40; default 5. This is always a filter.
    pub cutoff_hz: f32,
}
impl Default for DcBlockParams {
    fn default() -> Self {
        Self { cutoff_hz: 5.0 }
    }
}
param_set!(DcBlockParams, "DC blocker", {
    float [cutoff_hz] "cutoffHz" "Cutoff" { Hertz, Logarithmic, 1.0, 40.0, 5.0 }
});
pub struct DcBlock {
    params: DcBlockParams,
    sample_rate: f32,
    pole: Controls<1>,
    previous_input: [f64; 2],
    previous_output: [f64; 2],
}
impl Default for DcBlock {
    fn default() -> Self {
        Self {
            params: DcBlockParams::default(),
            sample_rate: 48_000.0,
            pole: Controls::new([0.0]),
            previous_input: [0.0; 2],
            previous_output: [0.0; 2],
        }
    }
}
impl DcBlock {
    fn update(&mut self) {
        let cutoff = self.params.cutoff_hz.min(self.sample_rate * 0.45);
        self.pole
            .set([(-std::f32::consts::TAU * cutoff / self.sample_rate).exp()]);
    }
}
impl Effect for DcBlock {
    type Params = DcBlockParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.sample_rate = rate(sample_rate);
        self.pole.prepare(self.sample_rate);
        self.reset();
    }
    fn reset(&mut self) {
        self.previous_input = [0.0; 2];
        self.previous_output = [0.0; 2];
        self.pole.reset();
        self.update();
    }
    fn set_params(&mut self, params: &Self::Params) {
        self.params = params.sanitized();
        self.update();
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let pole = f64::from(self.pole.tick()[0]);
            for (side, sample) in [l, r].into_iter().enumerate() {
                let input = f64::from(audio(*sample));
                let out = (1.0 + pole) * 0.5 * (input - self.previous_input[side])
                    + pole * self.previous_output[side];
                self.previous_input[side] = input;
                self.previous_output[side] = flush64(out);
                *sample = flush(out as f32);
            }
        }
    }
    fn tail_samples(&self) -> usize {
        // Conservative at the slowest allowed cutoff, including automation.
        (self.sample_rate * 48.0 / (std::f32::consts::TAU * (self.sample_rate * 0.45).min(1.0)))
            .ceil() as usize
    }
}
