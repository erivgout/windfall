use super::stage::{Stage, input, output, peak, rate, transition_samples};
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct MorphBandParams {
    /// Center in Hz, 20 to 20,000; clamped below Nyquist during design.
    pub frequency_hz: f32,
    /// Bell gain in dB, -18 to 18; default zero.
    pub gain_db: f32,
    /// Band Q, 0.1 to 10; default 1.
    pub q: f32,
}
impl Default for MorphBandParams {
    fn default() -> Self {
        Self {
            frequency_hz: 1_000.0,
            gain_db: 0.0,
            q: 1.0,
        }
    }
}
param_set!(MorphBandParams, "Morph Band", {
    float [frequency_hz] "frequencyHz" "Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1_000.0 }
    float [gain_db] "gainDb" "Gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [q] "q" "Q" { Ratio, Logarithmic, 0.1, 10.0, 1.0 }
});

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct MorphSnapshotParams {
    /// Four stored bell bands, with default centers 150, 600, 2500, 10000 Hz.
    pub bands: [MorphBandParams; 4],
}
impl Default for MorphSnapshotParams {
    fn default() -> Self {
        Self {
            bands: [150.0, 600.0, 2_500.0, 10_000.0].map(|frequency_hz| MorphBandParams {
                frequency_hz,
                ..MorphBandParams::default()
            }),
        }
    }
}
param_set!(MorphSnapshotParams, "Morph Snapshot", {
    float [bands[0].frequency_hz] "bands.0.frequencyHz" "Band 1 Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 150.0 }
    float [bands[0].gain_db] "bands.0.gainDb" "Band 1 Gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [bands[0].q] "bands.0.q" "Band 1 Q" { Ratio, Logarithmic, 0.1, 10.0, 1.0 }
    float [bands[1].frequency_hz] "bands.1.frequencyHz" "Band 2 Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 600.0 }
    float [bands[1].gain_db] "bands.1.gainDb" "Band 2 Gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [bands[1].q] "bands.1.q" "Band 2 Q" { Ratio, Logarithmic, 0.1, 10.0, 1.0 }
    float [bands[2].frequency_hz] "bands.2.frequencyHz" "Band 3 Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 2_500.0 }
    float [bands[2].gain_db] "bands.2.gainDb" "Band 3 Gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [bands[2].q] "bands.2.q" "Band 3 Q" { Ratio, Logarithmic, 0.1, 10.0, 1.0 }
    float [bands[3].frequency_hz] "bands.3.frequencyHz" "Band 4 Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 10_000.0 }
    float [bands[3].gain_db] "bands.3.gainDb" "Band 4 Gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [bands[3].q] "bands.3.q" "Band 4 Q" { Ratio, Logarithmic, 0.1, 10.0, 1.0 }
});

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct MorphEqParams {
    pub snapshot_a: MorphSnapshotParams,
    pub snapshot_b: MorphSnapshotParams,
    /// 0 selects A, 1 selects B. Hz and Q interpolate logarithmically, dB linearly.
    pub morph: f32,
}
impl Default for MorphEqParams {
    fn default() -> Self {
        Self {
            snapshot_a: MorphSnapshotParams::default(),
            snapshot_b: MorphSnapshotParams::default(),
            morph: 0.0,
        }
    }
}
param_set!(MorphEqParams, "Morph EQ", {
    float [snapshot_a.bands[0].frequency_hz] "snapshotA.bands.0.frequencyHz" "A Band 1 Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 150.0 }
    float [snapshot_a.bands[0].gain_db] "snapshotA.bands.0.gainDb" "A Band 1 Gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [snapshot_a.bands[0].q] "snapshotA.bands.0.q" "A Band 1 Q" { Ratio, Logarithmic, 0.1, 10.0, 1.0 }
    float [snapshot_a.bands[1].frequency_hz] "snapshotA.bands.1.frequencyHz" "A Band 2 Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 600.0 }
    float [snapshot_a.bands[1].gain_db] "snapshotA.bands.1.gainDb" "A Band 2 Gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [snapshot_a.bands[1].q] "snapshotA.bands.1.q" "A Band 2 Q" { Ratio, Logarithmic, 0.1, 10.0, 1.0 }
    float [snapshot_a.bands[2].frequency_hz] "snapshotA.bands.2.frequencyHz" "A Band 3 Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 2_500.0 }
    float [snapshot_a.bands[2].gain_db] "snapshotA.bands.2.gainDb" "A Band 3 Gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [snapshot_a.bands[2].q] "snapshotA.bands.2.q" "A Band 3 Q" { Ratio, Logarithmic, 0.1, 10.0, 1.0 }
    float [snapshot_a.bands[3].frequency_hz] "snapshotA.bands.3.frequencyHz" "A Band 4 Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 10_000.0 }
    float [snapshot_a.bands[3].gain_db] "snapshotA.bands.3.gainDb" "A Band 4 Gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [snapshot_a.bands[3].q] "snapshotA.bands.3.q" "A Band 4 Q" { Ratio, Logarithmic, 0.1, 10.0, 1.0 }
    float [snapshot_b.bands[0].frequency_hz] "snapshotB.bands.0.frequencyHz" "B Band 1 Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 150.0 }
    float [snapshot_b.bands[0].gain_db] "snapshotB.bands.0.gainDb" "B Band 1 Gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [snapshot_b.bands[0].q] "snapshotB.bands.0.q" "B Band 1 Q" { Ratio, Logarithmic, 0.1, 10.0, 1.0 }
    float [snapshot_b.bands[1].frequency_hz] "snapshotB.bands.1.frequencyHz" "B Band 2 Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 600.0 }
    float [snapshot_b.bands[1].gain_db] "snapshotB.bands.1.gainDb" "B Band 2 Gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [snapshot_b.bands[1].q] "snapshotB.bands.1.q" "B Band 2 Q" { Ratio, Logarithmic, 0.1, 10.0, 1.0 }
    float [snapshot_b.bands[2].frequency_hz] "snapshotB.bands.2.frequencyHz" "B Band 3 Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 2_500.0 }
    float [snapshot_b.bands[2].gain_db] "snapshotB.bands.2.gainDb" "B Band 3 Gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [snapshot_b.bands[2].q] "snapshotB.bands.2.q" "B Band 3 Q" { Ratio, Logarithmic, 0.1, 10.0, 1.0 }
    float [snapshot_b.bands[3].frequency_hz] "snapshotB.bands.3.frequencyHz" "B Band 4 Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 10_000.0 }
    float [snapshot_b.bands[3].gain_db] "snapshotB.bands.3.gainDb" "B Band 4 Gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [snapshot_b.bands[3].q] "snapshotB.bands.3.q" "B Band 4 Q" { Ratio, Logarithmic, 0.1, 10.0, 1.0 }
    float [morph] "morph" "Morph" { Fraction, Linear, 0.0, 1.0, 0.0 }
});

impl MorphEqParams {
    /// The four designed bands at the requested morph position, without smoothing.
    pub fn interpolated_bands(&self) -> [MorphBandParams; 4] {
        let p = self.sanitized();
        if p.morph == 0.0 {
            return p.snapshot_a.bands;
        }
        if p.morph == 1.0 {
            return p.snapshot_b.bands;
        }
        let logarithmic = |a: f32, b: f32| (a.ln() + p.morph * (b.ln() - a.ln())).exp();
        std::array::from_fn(|i| {
            let a = p.snapshot_a.bands[i];
            let b = p.snapshot_b.bands[i];
            MorphBandParams {
                frequency_hz: logarithmic(a.frequency_hz, b.frequency_hz),
                gain_db: a.gain_db + p.morph * (b.gain_db - a.gain_db),
                q: logarithmic(a.q, b.q),
            }
        })
    }
}

/// Four bells whose shapes interpolate between two independent stored snapshots.
/// This is a partial morph EQ: no spectral analysis or automatic curve capture.
pub struct MorphEq {
    sample_rate: f32,
    params: MorphEqParams,
    stages: [Stage; 4],
    fresh: bool,
}
impl Default for MorphEq {
    fn default() -> Self {
        Self {
            sample_rate: 48_000.0,
            params: MorphEqParams::default(),
            stages: [Stage::default(); 4],
            fresh: true,
        }
    }
}
impl MorphEq {
    fn apply(&mut self) {
        let samples = transition_samples(self.sample_rate, self.fresh);
        for (stage, band) in self.stages.iter_mut().zip(self.params.interpolated_bands()) {
            stage.set(
                peak(band.frequency_hz, band.gain_db, band.q, self.sample_rate),
                samples,
            );
        }
    }
}
impl Effect for MorphEq {
    type Params = MorphEqParams;
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
