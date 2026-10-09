use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::effect::Effect;
use crate::param::{ParamSet, param_set};

use super::{Controls, balance, rate};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct PanLfoParams {
    pub rate_hz: f32,
    pub depth: f32,
    pub pan: f32,
    pub volume_depth: f32,
    pub gain: f32,
}

impl Default for PanLfoParams {
    fn default() -> Self {
        Self {
            rate_hz: 1.0,
            depth: 1.0,
            pan: 0.0,
            volume_depth: 0.0,
            gain: 1.0,
        }
    }
}

param_set!(PanLfoParams, "Pan Motion", {
    float [rate_hz] "rateHz" "Rate" { Hertz, Logarithmic, 0.01, 20.0, 1.0 }
    float [depth] "depth" "Pan Depth" { Fraction, Linear, 0.0, 1.0, 1.0 }
    float [pan] "pan" "Pan Center" { Pan, Linear, -1.0, 1.0, 0.0 }
    float [volume_depth] "volumeDepth" "Volume Depth" { Fraction, Linear, 0.0, 1.0, 0.0 }
    float [gain] "gain" "Gain" { Gain, Linear, 0.0, 2.0, 1.0 }
});

pub struct PanLfo {
    controls: Controls<5>,
    sample_rate: f32,
    phase: f64,
}

impl Default for PanLfo {
    fn default() -> Self {
        Self {
            controls: Controls::new([1.0, 1.0, 0.0, 0.0, 1.0]),
            sample_rate: 48_000.0,
            phase: 0.0,
        }
    }
}

impl PanLfo {
    fn tick(&mut self) -> [f32; 2] {
        let [hz, depth, center, volume, gain] = self.controls.tick();
        let wave = (std::f64::consts::TAU * self.phase).sin() as f32;
        self.phase = (self.phase + f64::from(hz / self.sample_rate)).fract();
        [
            (center + depth * wave).clamp(-1.0, 1.0),
            gain * (1.0 - volume * (0.5 + 0.5 * wave)),
        ]
    }

    /// Outputs [bipolar pan, linear gain], one pair per sample.
    pub fn process_control<const N: usize>(&mut self, output: &mut [[f32; 2]; N]) {
        for frame in output {
            *frame = self.tick();
        }
    }

    pub fn process_with_control<const N: usize>(
        &mut self,
        left: &mut [f32; N],
        right: &mut [f32; N],
        output: &mut [[f32; 2]; N],
    ) {
        for ((l, r), frame) in left.iter_mut().zip(right).zip(output) {
            *frame = self.tick();
            [*l, *r] = balance(*l, *r, frame[0], frame[1]);
        }
    }
}

impl Effect for PanLfo {
    type Params = PanLfoParams;

    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.sample_rate = rate(sample_rate);
        self.controls.prepare(self.sample_rate);
        self.reset();
    }

    fn reset(&mut self) {
        self.phase = 0.0;
        self.controls.reset();
    }

    fn set_params(&mut self, params: &Self::Params) {
        let p = params.sanitized();
        self.controls
            .set([p.rate_hz, p.depth, p.pan, p.volume_depth, p.gain]);
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let [pan, gain] = self.tick();
            [*l, *r] = balance(*l, *r, pan, gain);
        }
    }
}
