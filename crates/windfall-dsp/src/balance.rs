//! Chain-position gain and stereo balance, with exact unity at the center.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::blocks::math::{clean, flush, ms_to_samples};
use crate::blocks::smooth::LinearRamp;
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct BalanceParams {
    /// Linear gain, 0 to 4; default 1. Zero is silence.
    pub gain: f32,
    /// Stereo balance, -1 (left only) to 1 (right only); default 0.
    /// The opposite channel is attenuated; the favored channel is unchanged.
    pub pan: f32,
}

impl Default for BalanceParams {
    fn default() -> Self {
        Self {
            gain: 1.0,
            pan: 0.0,
        }
    }
}

param_set!(BalanceParams, "Balance", {
    float [gain] "gain" "Gain" { Gain, Linear, 0.0, 4.0, 1.0 }
    float [pan] "pan" "Pan" { Pan, Linear, -1.0, 1.0, 0.0 }
});

// Shared by the seven utilities. Guard even direct Effect callers, rather
// than relying on EffectSlot to contain damaged upstream audio.
pub(crate) fn audio(value: f32) -> f32 {
    flush(clean(value, -1.0e3, 1.0e3, 0.0))
}
pub(crate) fn rate(value: f32) -> f32 {
    clean(value, 1.0, 384_000.0, 48_000.0)
}

/// Linear control ramps in signal space, including toggles mapped to gains.
/// Initial settings and reset snap; subsequent changes take exactly 5 ms.
pub(crate) struct Controls<const N: usize> {
    ramps: [LinearRamp; N],
    samples: u32,
    fresh: bool,
}

impl<const N: usize> Controls<N> {
    pub(crate) fn new(values: [f32; N]) -> Self {
        Self {
            ramps: values.map(LinearRamp::new),
            samples: 240,
            fresh: true,
        }
    }
    pub(crate) fn prepare(&mut self, sample_rate: f32) {
        self.samples = ms_to_samples(5.0, rate(sample_rate));
        self.reset();
    }
    pub(crate) fn reset(&mut self) {
        for ramp in &mut self.ramps {
            ramp.snap(ramp.target());
        }
        self.fresh = true;
    }
    pub(crate) fn set(&mut self, values: [f32; N]) {
        for (ramp, value) in self.ramps.iter_mut().zip(values) {
            ramp.set_target(value, if self.fresh { 0 } else { self.samples });
        }
    }
    pub(crate) fn tick(&mut self) -> [f32; N] {
        self.fresh = false;
        std::array::from_fn(|i| self.ramps[i].tick())
    }
}

pub struct Balance {
    controls: Controls<2>,
}
impl Default for Balance {
    fn default() -> Self {
        Self {
            controls: Controls::new([1.0; 2]),
        }
    }
}
impl Effect for Balance {
    type Params = BalanceParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.controls.prepare(sample_rate);
    }
    fn reset(&mut self) {
        self.controls.reset();
    }
    fn set_params(&mut self, params: &Self::Params) {
        let p = params.sanitized();
        self.controls.set([
            p.gain * (1.0 - p.pan.max(0.0)),
            p.gain * (1.0 + p.pan.min(0.0)),
        ]);
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let [lg, rg] = self.controls.tick();
            *l = flush(audio(*l) * lg);
            *r = flush(audio(*r) * rg);
        }
    }
}
