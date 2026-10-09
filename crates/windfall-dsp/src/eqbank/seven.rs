use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::stage::{Stage, input, output, peak, rate, transition_samples};
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};

/// Fixed bell centers. High centers are clamped below Nyquist at low rates.
pub const SEVEN_BAND_FREQUENCIES_HZ: [f32; 7] =
    [63.0, 160.0, 400.0, 1_000.0, 2_500.0, 6_300.0, 16_000.0];

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct SevenBandParams {
    /// Gains in dB, -18 to 18, ordered by ascending band frequency; default zero.
    pub gains_db: [f32; 7],
}
impl Default for SevenBandParams {
    fn default() -> Self {
        Self { gains_db: [0.0; 7] }
    }
}
param_set!(SevenBandParams, "Seven Band", {
    float [gains_db[0]] "gainsDb.0" "63 Hz Gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [gains_db[1]] "gainsDb.1" "160 Hz Gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [gains_db[2]] "gainsDb.2" "400 Hz Gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [gains_db[3]] "gainsDb.3" "1000 Hz Gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [gains_db[4]] "gainsDb.4" "2500 Hz Gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [gains_db[5]] "gainsDb.5" "6300 Hz Gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [gains_db[6]] "gainsDb.6" "16000 Hz Gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
});

/// Seven fixed, Q=1.4 bell filters in series; flat gains pass audio exactly.
pub struct SevenBand {
    sample_rate: f32,
    params: SevenBandParams,
    stages: [Stage; 7],
    fresh: bool,
}
impl Default for SevenBand {
    fn default() -> Self {
        Self {
            sample_rate: 48_000.0,
            params: SevenBandParams::default(),
            stages: [Stage::default(); 7],
            fresh: true,
        }
    }
}
impl SevenBand {
    fn apply(&mut self) {
        let samples = transition_samples(self.sample_rate, self.fresh);
        for (i, stage) in self.stages.iter_mut().enumerate() {
            stage.set(
                peak(
                    SEVEN_BAND_FREQUENCIES_HZ[i],
                    self.params.gains_db[i],
                    1.4,
                    self.sample_rate,
                ),
                samples,
            );
        }
    }
}
impl Effect for SevenBand {
    type Params = SevenBandParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.sample_rate = rate(sample_rate);
        self.reset();
    }
    fn reset(&mut self) {
        self.fresh = true;
        self.apply();
        for stage in &mut self.stages {
            stage.reset();
        }
    }
    fn set_params(&mut self, params: &Self::Params) {
        self.params = params.sanitized();
        self.apply();
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            self.fresh = false;
            let mut frame = [input(*l), input(*r)];
            for stage in &mut self.stages {
                frame = stage.tick(frame);
            }
            *l = output(frame[0]);
            *r = output(frame[1]);
        }
    }
    fn tail_samples(&self) -> usize {
        self.stages.iter().map(Stage::tail_samples).sum()
    }
}
