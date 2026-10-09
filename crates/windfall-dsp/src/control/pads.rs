use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::blocks::svf::{Svf, SvfCoeffs};
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};

use super::{Controls, audio, balance, rate};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct XyPadParams {
    pub x: f32,
    pub y: f32,
}

impl Default for XyPadParams {
    fn default() -> Self {
        Self { x: 0.5, y: 0.5 }
    }
}

param_set!(XyPadParams, "XY Pad", {
    float [x] "x" "X / Pan" { Fraction, Linear, 0.0, 1.0, 0.5 }
    float [y] "y" "Y / Gain" { Fraction, Linear, 0.0, 1.0, 0.5 }
});

pub struct XyPad {
    controls: Controls<2>,
}

impl Default for XyPad {
    fn default() -> Self {
        Self {
            controls: Controls::new([0.5; 2]),
        }
    }
}

impl XyPad {
    /// Normalized X/Y outputs, one pair per sample.
    pub fn process_control<const N: usize>(&mut self, output: &mut [[f32; 2]; N]) {
        for frame in output {
            *frame = self.controls.tick();
        }
    }

    /// Apply X as stereo balance (-1..1) and Y as gain (0..2).
    pub fn process_with_control<const N: usize>(
        &mut self,
        left: &mut [f32; N],
        right: &mut [f32; N],
        output: &mut [[f32; 2]; N],
    ) {
        for ((l, r), frame) in left.iter_mut().zip(right).zip(output) {
            *frame = self.controls.tick();
            [*l, *r] = balance(*l, *r, 2.0 * frame[0] - 1.0, 2.0 * frame[1]);
        }
    }
}

impl Effect for XyPad {
    type Params = XyPadParams;

    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.controls.prepare(sample_rate);
    }

    fn reset(&mut self) {
        self.controls.reset();
    }

    fn set_params(&mut self, params: &Self::Params) {
        let p = params.sanitized();
        self.controls.set([p.x, p.y]);
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let [x, y] = self.controls.tick();
            [*l, *r] = balance(*l, *r, 2.0 * x - 1.0, 2.0 * y);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct XyzPadParams {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Default for XyzPadParams {
    fn default() -> Self {
        Self {
            x: 0.5,
            y: 1.0,
            z: 0.0,
        }
    }
}

param_set!(XyzPadParams, "XYZ Pad", {
    float [x] "x" "X / Pan" { Fraction, Linear, 0.0, 1.0, 0.5 }
    float [y] "y" "Y / Cutoff" { Fraction, Linear, 0.0, 1.0, 1.0 }
    float [z] "z" "Z / Resonance" { Fraction, Linear, 0.0, 1.0, 0.0 }
});

pub struct XyzPad {
    controls: Controls<3>,
    sample_rate: f32,
    filters: [Svf; 2],
}

impl Default for XyzPad {
    fn default() -> Self {
        Self {
            controls: Controls::new([0.5, 1.0, 0.0]),
            sample_rate: 48_000.0,
            filters: [Svf::default(); 2],
        }
    }
}

impl XyzPad {
    pub fn process_control<const N: usize>(&mut self, output: &mut [[f32; 3]; N]) {
        for frame in output {
            *frame = self.controls.tick();
        }
    }

    fn tick_audio(&mut self, left: f32, right: f32, axes: [f32; 3]) -> [f32; 2] {
        // Logarithmic 20..20,000 Hz cutoff, capped by the shared SVF below Nyquist.
        let cutoff = 20.0 * 1000.0_f32.powf(axes[1]);
        let minimum_q = std::f32::consts::FRAC_1_SQRT_2;
        let q = minimum_q + axes[2] * (8.0 - minimum_q);
        let coeffs = SvfCoeffs::new(cutoff, q, self.sample_rate);
        let l = self.filters[0].tick(&coeffs, audio(left)).low;
        let r = self.filters[1].tick(&coeffs, audio(right)).low;
        for filter in &mut self.filters {
            filter.flush();
        }
        balance(l, r, 2.0 * axes[0] - 1.0, 1.0)
    }

    pub fn process_with_control<const N: usize>(
        &mut self,
        left: &mut [f32; N],
        right: &mut [f32; N],
        output: &mut [[f32; 3]; N],
    ) {
        for ((l, r), frame) in left.iter_mut().zip(right).zip(output) {
            *frame = self.controls.tick();
            [*l, *r] = self.tick_audio(*l, *r, *frame);
        }
    }
}

impl Effect for XyzPad {
    type Params = XyzPadParams;

    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.sample_rate = rate(sample_rate);
        self.controls.prepare(self.sample_rate);
        self.reset();
    }

    fn reset(&mut self) {
        self.controls.reset();
        for filter in &mut self.filters {
            filter.reset();
        }
    }

    fn set_params(&mut self, params: &Self::Params) {
        let p = params.sanitized();
        self.controls.set([p.x, p.y, p.z]);
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let axes = self.controls.tick();
            [*l, *r] = self.tick_audio(*l, *r, axes);
        }
    }

    fn tail_samples(&self) -> usize {
        // Conservative decay at the minimum cutoff and maximum Q, including
        // cutoff clamping at unusually low sample rates.
        ((self.sample_rate * 10.0).ceil() as usize).max(20_000)
    }
}
