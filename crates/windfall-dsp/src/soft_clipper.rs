//! A memoryless, odd, continuously differentiable polynomial soft knee.
use crate::balance::{Controls, audio};
use crate::blocks::math::{db_to_gain, flush};
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct SoftClipperParams {
    /// Peak ceiling in dBFS, -24 to 0; default 0.
    pub ceiling_db: f32,
    /// Fraction of the ceiling occupied by the rounded knee, 0.05 to 1; default 0.5.
    pub knee: f32,
}
impl Default for SoftClipperParams {
    fn default() -> Self {
        Self {
            ceiling_db: 0.0,
            knee: 0.5,
        }
    }
}
param_set!(SoftClipperParams, "Soft clipper", {
    float [ceiling_db] "ceilingDb" "Ceiling" { Decibels, Linear, -24.0, 0.0, 0.0 }
    float [knee] "knee" "Knee" { Fraction, Linear, 0.05, 1.0, 0.5 }
});
impl SoftClipperParams {
    /// Settled transfer, also useful for independent curve measurements.
    pub fn transfer(&self, input: f32) -> f32 {
        let p = self.sanitized();
        clip(audio(input), db_to_gain(p.ceiling_db), p.knee)
    }
}
fn clip(input: f32, ceiling: f32, knee: f32) -> f32 {
    let start = ceiling * (1.0 - knee);
    let width = ceiling * knee;
    let magnitude = input.abs();
    let out = if magnitude <= start {
        magnitude
    } else if magnitude >= start + 2.0 * width {
        ceiling
    } else {
        let t = (magnitude - start) / (2.0 * width);
        start + width * (2.0 * t - t * t)
    };
    flush(out.min(ceiling).copysign(input))
}
pub struct SoftClipper {
    controls: Controls<2>,
}
impl Default for SoftClipper {
    fn default() -> Self {
        Self {
            controls: Controls::new([1.0, 0.5]),
        }
    }
}
impl Effect for SoftClipper {
    type Params = SoftClipperParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.controls.prepare(sample_rate);
    }
    fn reset(&mut self) {
        self.controls.reset();
    }
    fn set_params(&mut self, params: &Self::Params) {
        let p = params.sanitized();
        self.controls.set([db_to_gain(p.ceiling_db), p.knee]);
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let [ceiling, knee] = self.controls.tick();
            *l = clip(audio(*l), ceiling, knee);
            *r = clip(audio(*r), ceiling, knee);
        }
    }
}
