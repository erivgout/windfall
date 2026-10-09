use crate::balance::{Controls, audio};
use crate::blocks::math::db_to_gain;
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DriveStage {
    #[default]
    Bypass,
    Overdrive,
    Waveshape,
    Fuzz,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct DriveStageParams {
    /// Stage algorithm; default bypass (which ignores stage gain).
    pub model: DriveStage,
    /// Gain ahead of the nonlinearity in dB, -24..36; default 0.
    pub gain_db: f32,
}
impl Default for DriveStageParams {
    fn default() -> Self {
        Self {
            model: DriveStage::Bypass,
            gain_db: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct DriveChainParams {
    /// Signal flows through indices 0, 1, 2, then through output trim.
    pub stages: [DriveStageParams; 3],
    /// Output trim in dB, -24..12; default 0.
    pub output_db: f32,
}
impl Default for DriveChainParams {
    fn default() -> Self {
        Self {
            stages: [DriveStageParams::default(); 3],
            output_db: 0.0,
        }
    }
}
param_set!(DriveChainParams, "Drive Chain", {
    choice [stages[0].model] "stages.0.model" "Stage 1" { DriveStage, Bypass, [Bypass "bypass" "Bypass", Overdrive "overdrive" "Overdrive", Waveshape "waveshape" "Waveshape", Fuzz "fuzz" "Fuzz"] }
    float [stages[0].gain_db] "stages.0.gainDb" "Stage 1 gain" { Decibels, Linear, -24.0, 36.0, 0.0 }
    choice [stages[1].model] "stages.1.model" "Stage 2" { DriveStage, Bypass, [Bypass "bypass" "Bypass", Overdrive "overdrive" "Overdrive", Waveshape "waveshape" "Waveshape", Fuzz "fuzz" "Fuzz"] }
    float [stages[1].gain_db] "stages.1.gainDb" "Stage 2 gain" { Decibels, Linear, -24.0, 36.0, 0.0 }
    choice [stages[2].model] "stages.2.model" "Stage 3" { DriveStage, Bypass, [Bypass "bypass" "Bypass", Overdrive "overdrive" "Overdrive", Waveshape "waveshape" "Waveshape", Fuzz "fuzz" "Fuzz"] }
    float [stages[2].gain_db] "stages.2.gainDb" "Stage 3 gain" { Decibels, Linear, -24.0, 36.0, 0.0 }
    float [output_db] "outputDb" "Output" { Decibels, Linear, -24.0, 12.0, 0.0 }
});

#[inline]
fn transfers(input: f32, gain: f32) -> [f32; 4] {
    let x = input * gain;
    let bounded = x.clamp(-1.0, 1.0);
    [
        input,
        super::asymmetric(x, 0.8),
        1.5 * bounded - 0.5 * bounded.powi(3),
        (4.0 * x).clamp(-0.7, 0.7),
    ]
}
impl DriveChainParams {
    pub fn transfer(&self, input: f32) -> f32 {
        let p = self.sanitized();
        let mut x = audio(input);
        for stage in p.stages {
            x = transfers(x, db_to_gain(stage.gain_db))[stage.model as usize];
        }
        audio(x * db_to_gain(p.output_db))
    }
}

pub struct DriveChain {
    controls: Controls<16>,
}
impl Default for DriveChain {
    fn default() -> Self {
        Self {
            controls: Controls::new(Self::values(&DriveChainParams::default())),
        }
    }
}
impl DriveChain {
    fn values(p: &DriveChainParams) -> [f32; 16] {
        let mut values = [0.0; 16];
        for (i, stage) in p.stages.iter().enumerate() {
            values[5 * i] = db_to_gain(stage.gain_db);
            values[5 * i + 1 + stage.model as usize] = 1.0;
        }
        values[15] = db_to_gain(p.output_db);
        values
    }
    fn tick(input: f32, values: &[f32; 16]) -> f32 {
        let mut x = audio(input);
        for stage in values[..15].as_chunks::<5>().0 {
            x = transfers(x, stage[0])
                .iter()
                .zip(&stage[1..])
                .map(|(y, w)| y * w)
                .sum();
        }
        audio(x * values[15])
    }
}
impl Effect for DriveChain {
    type Params = DriveChainParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.controls.prepare(sample_rate);
    }
    fn reset(&mut self) {
        self.controls.reset();
    }
    fn set_params(&mut self, params: &Self::Params) {
        self.controls.set(Self::values(&params.sanitized()));
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let values = self.controls.tick();
            *l = Self::tick(*l, &values);
            *r = Self::tick(*r, &values);
        }
    }
}
