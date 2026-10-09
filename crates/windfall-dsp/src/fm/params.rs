use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::param::param_set;

/// Linear attack, decay, sustain and release. Times are milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct FmEnvelopeParams {
    pub attack_ms: f32,
    pub decay_ms: f32,
    pub sustain: f32,
    pub release_ms: f32,
}

impl Default for FmEnvelopeParams {
    fn default() -> Self {
        Self {
            attack_ms: 2.0,
            decay_ms: 150.0,
            sustain: 0.8,
            release_ms: 120.0,
        }
    }
}

/// Sine operator. Level scales both carrier output and modulation depth.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct FmOperatorParams {
    pub ratio: f32,
    pub level: f32,
    pub envelope: FmEnvelopeParams,
}

impl Default for FmOperatorParams {
    fn default() -> Self {
        Self {
            ratio: 1.0,
            level: 0.0,
            envelope: FmEnvelopeParams::default(),
        }
    }
}

/// Eight acyclic graphs; operator numbers in these names are one-based.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FourOpAlgorithm {
    /// 4 -> 3 -> 2 -> 1; output 1.
    #[default]
    Stack,
    /// 4 -> 3 and 2 -> 1; output 1 + 3.
    Pairs,
    /// 4, 3 and 2 -> 1; output 1.
    FanIn,
    /// 4 -> 1, 2 and 3; output 1 + 2 + 3.
    FanOut,
    /// 4 -> 3 -> 1 and 2 -> 1; output 1.
    Branch,
    /// 4 -> 2 and 3 -> 2 -> 1; output 1.
    Merge,
    /// 4 -> 3 -> 2; output 1 + 2.
    Triple,
    /// Four independent carriers.
    Parallel,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct FourOpParams {
    pub algorithm: FourOpAlgorithm,
    /// Operator 4 self-modulation, in radians at unit operator level.
    pub feedback: f32,
    pub operators: [FmOperatorParams; 4],
    pub gain: f32,
}

impl Default for FourOpParams {
    fn default() -> Self {
        let mut operators = [FmOperatorParams::default(); 4];
        operators[0].level = 1.0;
        Self {
            algorithm: FourOpAlgorithm::Stack,
            feedback: 0.0,
            operators,
            gain: 0.2,
        }
    }
}

/// Matrix operator, with independent carrier send and pitch-dependent level.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct MatrixOperatorParams {
    pub ratio: f32,
    pub level: f32,
    pub envelope: FmEnvelopeParams,
    pub feedback: f32,
    /// Gain octaves per keyboard octave, centered on A4; bounded to 4x.
    pub key_scaling: f32,
    pub output: f32,
}

impl Default for MatrixOperatorParams {
    fn default() -> Self {
        Self {
            ratio: 1.0,
            level: 0.0,
            envelope: FmEnvelopeParams::default(),
            feedback: 0.0,
            key_scaling: 0.0,
            output: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct MatrixFmParams {
    pub operators: [MatrixOperatorParams; 6],
    /// `matrix[destination][source]`: signed radians per unit source output.
    /// All routes, including the diagonal, use the previous sample.
    pub matrix: [[f32; 6]; 6],
    pub gain: f32,
}

impl Default for MatrixFmParams {
    fn default() -> Self {
        let mut operators = [MatrixOperatorParams::default(); 6];
        operators[0].level = 1.0;
        operators[0].output = 1.0;
        Self {
            operators,
            matrix: [[0.0; 6]; 6],
            gain: 0.2,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum HybridWaveform {
    #[default]
    Sine,
    Triangle,
    Saw,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct HybridOscillatorParams {
    pub waveform: HybridWaveform,
    pub ratio: f32,
    pub level: f32,
}

impl Default for HybridOscillatorParams {
    fn default() -> Self {
        Self {
            waveform: HybridWaveform::Sine,
            ratio: 1.0,
            level: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct RingHybridParams {
    pub oscillators: [HybridOscillatorParams; 3],
    /// Oscillator 2 -> 1 phase modulation, in radians.
    pub fm_21: f32,
    /// Oscillator 3 -> 1 phase modulation, in radians.
    pub fm_31: f32,
    /// Oscillator 3 -> 2 phase modulation, in radians.
    pub fm_32: f32,
    /// Crossfade oscillator 1 into its product with oscillator 2.
    pub ring_12: f32,
    /// Crossfade that result into its product with oscillator 3.
    pub ring_13: f32,
    pub cutoff_hz: f32,
    pub resonance: f32,
    pub amp_envelope: FmEnvelopeParams,
    pub gain: f32,
}

impl Default for RingHybridParams {
    fn default() -> Self {
        let mut oscillators = [HybridOscillatorParams::default(); 3];
        oscillators[0].level = 1.0;
        Self {
            oscillators,
            fm_21: 0.0,
            fm_31: 0.0,
            fm_32: 0.0,
            ring_12: 0.0,
            ring_13: 0.0,
            cutoff_hz: 12_000.0,
            resonance: 0.0,
            amp_envelope: FmEnvelopeParams::default(),
            gain: 0.2,
        }
    }
}

param_set!(FourOpParams, "Four Operator", {
    choice [algorithm] "algorithm" "Algorithm" { FourOpAlgorithm, Stack, [Stack "stack" "Stack", Pairs "pairs" "Pairs", FanIn "fanIn" "Fan In", FanOut "fanOut" "Fan Out", Branch "branch" "Branch", Merge "merge" "Merge", Triple "triple" "Triple", Parallel "parallel" "Parallel"] }
    float [feedback] "feedback" "Operator 4 Feedback" { None, Linear, 0.0, 8.0, 0.0 }
    float [operators[0].ratio] "operators.0.ratio" "Operator 1 Ratio" { Ratio, Logarithmic, 0.125, 32.0, 1.0 }
    float [operators[0].level] "operators.0.level" "Operator 1 Level" { Gain, Linear, 0.0, 4.0, 1.0 }
    float [operators[0].envelope.attack_ms] "operators.0.envelope.attackMs" "Operator 1 Attack" { Milliseconds, Linear, 0.5, 10000.0, 2.0 }
    float [operators[0].envelope.decay_ms] "operators.0.envelope.decayMs" "Operator 1 Decay" { Milliseconds, Linear, 1.0, 10000.0, 150.0 }
    float [operators[0].envelope.sustain] "operators.0.envelope.sustain" "Operator 1 Sustain" { Fraction, Linear, 0.0, 1.0, 0.8 }
    float [operators[0].envelope.release_ms] "operators.0.envelope.releaseMs" "Operator 1 Release" { Milliseconds, Linear, 0.5, 10000.0, 120.0 }
    float [operators[1].ratio] "operators.1.ratio" "Operator 2 Ratio" { Ratio, Logarithmic, 0.125, 32.0, 1.0 }
    float [operators[1].level] "operators.1.level" "Operator 2 Level" { Gain, Linear, 0.0, 4.0, 0.0 }
    float [operators[1].envelope.attack_ms] "operators.1.envelope.attackMs" "Operator 2 Attack" { Milliseconds, Linear, 0.5, 10000.0, 2.0 }
    float [operators[1].envelope.decay_ms] "operators.1.envelope.decayMs" "Operator 2 Decay" { Milliseconds, Linear, 1.0, 10000.0, 150.0 }
    float [operators[1].envelope.sustain] "operators.1.envelope.sustain" "Operator 2 Sustain" { Fraction, Linear, 0.0, 1.0, 0.8 }
    float [operators[1].envelope.release_ms] "operators.1.envelope.releaseMs" "Operator 2 Release" { Milliseconds, Linear, 0.5, 10000.0, 120.0 }
    float [operators[2].ratio] "operators.2.ratio" "Operator 3 Ratio" { Ratio, Logarithmic, 0.125, 32.0, 1.0 }
    float [operators[2].level] "operators.2.level" "Operator 3 Level" { Gain, Linear, 0.0, 4.0, 0.0 }
    float [operators[2].envelope.attack_ms] "operators.2.envelope.attackMs" "Operator 3 Attack" { Milliseconds, Linear, 0.5, 10000.0, 2.0 }
    float [operators[2].envelope.decay_ms] "operators.2.envelope.decayMs" "Operator 3 Decay" { Milliseconds, Linear, 1.0, 10000.0, 150.0 }
    float [operators[2].envelope.sustain] "operators.2.envelope.sustain" "Operator 3 Sustain" { Fraction, Linear, 0.0, 1.0, 0.8 }
    float [operators[2].envelope.release_ms] "operators.2.envelope.releaseMs" "Operator 3 Release" { Milliseconds, Linear, 0.5, 10000.0, 120.0 }
    float [operators[3].ratio] "operators.3.ratio" "Operator 4 Ratio" { Ratio, Logarithmic, 0.125, 32.0, 1.0 }
    float [operators[3].level] "operators.3.level" "Operator 4 Level" { Gain, Linear, 0.0, 4.0, 0.0 }
    float [operators[3].envelope.attack_ms] "operators.3.envelope.attackMs" "Operator 4 Attack" { Milliseconds, Linear, 0.5, 10000.0, 2.0 }
    float [operators[3].envelope.decay_ms] "operators.3.envelope.decayMs" "Operator 4 Decay" { Milliseconds, Linear, 1.0, 10000.0, 150.0 }
    float [operators[3].envelope.sustain] "operators.3.envelope.sustain" "Operator 4 Sustain" { Fraction, Linear, 0.0, 1.0, 0.8 }
    float [operators[3].envelope.release_ms] "operators.3.envelope.releaseMs" "Operator 4 Release" { Milliseconds, Linear, 0.5, 10000.0, 120.0 }
    float [gain] "gain" "Gain" { Gain, Linear, 0.0, 1.0, 0.2 }
});

param_set!(MatrixFmParams, "Matrix FM", {
    float [operators[0].ratio] "operators.0.ratio" "Operator 1 Ratio" { Ratio, Logarithmic, 0.125, 32.0, 1.0 }
    float [operators[0].level] "operators.0.level" "Operator 1 Level" { Gain, Linear, 0.0, 4.0, 1.0 }
    float [operators[0].envelope.attack_ms] "operators.0.envelope.attackMs" "Operator 1 Attack" { Milliseconds, Linear, 0.5, 10000.0, 2.0 }
    float [operators[0].envelope.decay_ms] "operators.0.envelope.decayMs" "Operator 1 Decay" { Milliseconds, Linear, 1.0, 10000.0, 150.0 }
    float [operators[0].envelope.sustain] "operators.0.envelope.sustain" "Operator 1 Sustain" { Fraction, Linear, 0.0, 1.0, 0.8 }
    float [operators[0].envelope.release_ms] "operators.0.envelope.releaseMs" "Operator 1 Release" { Milliseconds, Linear, 0.5, 10000.0, 120.0 }
    float [operators[0].feedback] "operators.0.feedback" "Operator 1 Feedback" { None, Linear, 0.0, 8.0, 0.0 }
    float [operators[0].key_scaling] "operators.0.keyScaling" "Operator 1 Key Scaling" { Octaves, Linear, -1.0, 1.0, 0.0 }
    float [operators[0].output] "operators.0.output" "Operator 1 Output" { Gain, Linear, 0.0, 1.0, 1.0 }
    float [operators[1].ratio] "operators.1.ratio" "Operator 2 Ratio" { Ratio, Logarithmic, 0.125, 32.0, 1.0 }
    float [operators[1].level] "operators.1.level" "Operator 2 Level" { Gain, Linear, 0.0, 4.0, 0.0 }
    float [operators[1].envelope.attack_ms] "operators.1.envelope.attackMs" "Operator 2 Attack" { Milliseconds, Linear, 0.5, 10000.0, 2.0 }
    float [operators[1].envelope.decay_ms] "operators.1.envelope.decayMs" "Operator 2 Decay" { Milliseconds, Linear, 1.0, 10000.0, 150.0 }
    float [operators[1].envelope.sustain] "operators.1.envelope.sustain" "Operator 2 Sustain" { Fraction, Linear, 0.0, 1.0, 0.8 }
    float [operators[1].envelope.release_ms] "operators.1.envelope.releaseMs" "Operator 2 Release" { Milliseconds, Linear, 0.5, 10000.0, 120.0 }
    float [operators[1].feedback] "operators.1.feedback" "Operator 2 Feedback" { None, Linear, 0.0, 8.0, 0.0 }
    float [operators[1].key_scaling] "operators.1.keyScaling" "Operator 2 Key Scaling" { Octaves, Linear, -1.0, 1.0, 0.0 }
    float [operators[1].output] "operators.1.output" "Operator 2 Output" { Gain, Linear, 0.0, 1.0, 0.0 }
    float [operators[2].ratio] "operators.2.ratio" "Operator 3 Ratio" { Ratio, Logarithmic, 0.125, 32.0, 1.0 }
    float [operators[2].level] "operators.2.level" "Operator 3 Level" { Gain, Linear, 0.0, 4.0, 0.0 }
    float [operators[2].envelope.attack_ms] "operators.2.envelope.attackMs" "Operator 3 Attack" { Milliseconds, Linear, 0.5, 10000.0, 2.0 }
    float [operators[2].envelope.decay_ms] "operators.2.envelope.decayMs" "Operator 3 Decay" { Milliseconds, Linear, 1.0, 10000.0, 150.0 }
    float [operators[2].envelope.sustain] "operators.2.envelope.sustain" "Operator 3 Sustain" { Fraction, Linear, 0.0, 1.0, 0.8 }
    float [operators[2].envelope.release_ms] "operators.2.envelope.releaseMs" "Operator 3 Release" { Milliseconds, Linear, 0.5, 10000.0, 120.0 }
    float [operators[2].feedback] "operators.2.feedback" "Operator 3 Feedback" { None, Linear, 0.0, 8.0, 0.0 }
    float [operators[2].key_scaling] "operators.2.keyScaling" "Operator 3 Key Scaling" { Octaves, Linear, -1.0, 1.0, 0.0 }
    float [operators[2].output] "operators.2.output" "Operator 3 Output" { Gain, Linear, 0.0, 1.0, 0.0 }
    float [operators[3].ratio] "operators.3.ratio" "Operator 4 Ratio" { Ratio, Logarithmic, 0.125, 32.0, 1.0 }
    float [operators[3].level] "operators.3.level" "Operator 4 Level" { Gain, Linear, 0.0, 4.0, 0.0 }
    float [operators[3].envelope.attack_ms] "operators.3.envelope.attackMs" "Operator 4 Attack" { Milliseconds, Linear, 0.5, 10000.0, 2.0 }
    float [operators[3].envelope.decay_ms] "operators.3.envelope.decayMs" "Operator 4 Decay" { Milliseconds, Linear, 1.0, 10000.0, 150.0 }
    float [operators[3].envelope.sustain] "operators.3.envelope.sustain" "Operator 4 Sustain" { Fraction, Linear, 0.0, 1.0, 0.8 }
    float [operators[3].envelope.release_ms] "operators.3.envelope.releaseMs" "Operator 4 Release" { Milliseconds, Linear, 0.5, 10000.0, 120.0 }
    float [operators[3].feedback] "operators.3.feedback" "Operator 4 Feedback" { None, Linear, 0.0, 8.0, 0.0 }
    float [operators[3].key_scaling] "operators.3.keyScaling" "Operator 4 Key Scaling" { Octaves, Linear, -1.0, 1.0, 0.0 }
    float [operators[3].output] "operators.3.output" "Operator 4 Output" { Gain, Linear, 0.0, 1.0, 0.0 }
    float [operators[4].ratio] "operators.4.ratio" "Operator 5 Ratio" { Ratio, Logarithmic, 0.125, 32.0, 1.0 }
    float [operators[4].level] "operators.4.level" "Operator 5 Level" { Gain, Linear, 0.0, 4.0, 0.0 }
    float [operators[4].envelope.attack_ms] "operators.4.envelope.attackMs" "Operator 5 Attack" { Milliseconds, Linear, 0.5, 10000.0, 2.0 }
    float [operators[4].envelope.decay_ms] "operators.4.envelope.decayMs" "Operator 5 Decay" { Milliseconds, Linear, 1.0, 10000.0, 150.0 }
    float [operators[4].envelope.sustain] "operators.4.envelope.sustain" "Operator 5 Sustain" { Fraction, Linear, 0.0, 1.0, 0.8 }
    float [operators[4].envelope.release_ms] "operators.4.envelope.releaseMs" "Operator 5 Release" { Milliseconds, Linear, 0.5, 10000.0, 120.0 }
    float [operators[4].feedback] "operators.4.feedback" "Operator 5 Feedback" { None, Linear, 0.0, 8.0, 0.0 }
    float [operators[4].key_scaling] "operators.4.keyScaling" "Operator 5 Key Scaling" { Octaves, Linear, -1.0, 1.0, 0.0 }
    float [operators[4].output] "operators.4.output" "Operator 5 Output" { Gain, Linear, 0.0, 1.0, 0.0 }
    float [operators[5].ratio] "operators.5.ratio" "Operator 6 Ratio" { Ratio, Logarithmic, 0.125, 32.0, 1.0 }
    float [operators[5].level] "operators.5.level" "Operator 6 Level" { Gain, Linear, 0.0, 4.0, 0.0 }
    float [operators[5].envelope.attack_ms] "operators.5.envelope.attackMs" "Operator 6 Attack" { Milliseconds, Linear, 0.5, 10000.0, 2.0 }
    float [operators[5].envelope.decay_ms] "operators.5.envelope.decayMs" "Operator 6 Decay" { Milliseconds, Linear, 1.0, 10000.0, 150.0 }
    float [operators[5].envelope.sustain] "operators.5.envelope.sustain" "Operator 6 Sustain" { Fraction, Linear, 0.0, 1.0, 0.8 }
    float [operators[5].envelope.release_ms] "operators.5.envelope.releaseMs" "Operator 6 Release" { Milliseconds, Linear, 0.5, 10000.0, 120.0 }
    float [operators[5].feedback] "operators.5.feedback" "Operator 6 Feedback" { None, Linear, 0.0, 8.0, 0.0 }
    float [operators[5].key_scaling] "operators.5.keyScaling" "Operator 6 Key Scaling" { Octaves, Linear, -1.0, 1.0, 0.0 }
    float [operators[5].output] "operators.5.output" "Operator 6 Output" { Gain, Linear, 0.0, 1.0, 0.0 }
    float [matrix[0][0]] "matrix.0.0" "Operator 1 to 1" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[0][1]] "matrix.0.1" "Operator 2 to 1" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[0][2]] "matrix.0.2" "Operator 3 to 1" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[0][3]] "matrix.0.3" "Operator 4 to 1" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[0][4]] "matrix.0.4" "Operator 5 to 1" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[0][5]] "matrix.0.5" "Operator 6 to 1" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[1][0]] "matrix.1.0" "Operator 1 to 2" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[1][1]] "matrix.1.1" "Operator 2 to 2" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[1][2]] "matrix.1.2" "Operator 3 to 2" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[1][3]] "matrix.1.3" "Operator 4 to 2" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[1][4]] "matrix.1.4" "Operator 5 to 2" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[1][5]] "matrix.1.5" "Operator 6 to 2" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[2][0]] "matrix.2.0" "Operator 1 to 3" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[2][1]] "matrix.2.1" "Operator 2 to 3" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[2][2]] "matrix.2.2" "Operator 3 to 3" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[2][3]] "matrix.2.3" "Operator 4 to 3" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[2][4]] "matrix.2.4" "Operator 5 to 3" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[2][5]] "matrix.2.5" "Operator 6 to 3" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[3][0]] "matrix.3.0" "Operator 1 to 4" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[3][1]] "matrix.3.1" "Operator 2 to 4" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[3][2]] "matrix.3.2" "Operator 3 to 4" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[3][3]] "matrix.3.3" "Operator 4 to 4" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[3][4]] "matrix.3.4" "Operator 5 to 4" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[3][5]] "matrix.3.5" "Operator 6 to 4" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[4][0]] "matrix.4.0" "Operator 1 to 5" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[4][1]] "matrix.4.1" "Operator 2 to 5" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[4][2]] "matrix.4.2" "Operator 3 to 5" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[4][3]] "matrix.4.3" "Operator 4 to 5" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[4][4]] "matrix.4.4" "Operator 5 to 5" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[4][5]] "matrix.4.5" "Operator 6 to 5" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[5][0]] "matrix.5.0" "Operator 1 to 6" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[5][1]] "matrix.5.1" "Operator 2 to 6" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[5][2]] "matrix.5.2" "Operator 3 to 6" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[5][3]] "matrix.5.3" "Operator 4 to 6" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[5][4]] "matrix.5.4" "Operator 5 to 6" { None, Linear, -8.0, 8.0, 0.0 }
    float [matrix[5][5]] "matrix.5.5" "Operator 6 to 6" { None, Linear, -8.0, 8.0, 0.0 }
    float [gain] "gain" "Gain" { Gain, Linear, 0.0, 1.0, 0.2 }
});

param_set!(RingHybridParams, "Ring Hybrid", {
    choice [oscillators[0].waveform] "oscillators.0.waveform" "Oscillator 1 Waveform" { HybridWaveform, Sine, [Sine "sine" "Sine", Triangle "triangle" "Triangle", Saw "saw" "Saw", Square "square" "Square"] }
    float [oscillators[0].ratio] "oscillators.0.ratio" "Oscillator 1 Ratio" { Ratio, Logarithmic, 0.125, 32.0, 1.0 }
    float [oscillators[0].level] "oscillators.0.level" "Oscillator 1 Level" { Gain, Linear, 0.0, 1.0, 1.0 }
    choice [oscillators[1].waveform] "oscillators.1.waveform" "Oscillator 2 Waveform" { HybridWaveform, Sine, [Sine "sine" "Sine", Triangle "triangle" "Triangle", Saw "saw" "Saw", Square "square" "Square"] }
    float [oscillators[1].ratio] "oscillators.1.ratio" "Oscillator 2 Ratio" { Ratio, Logarithmic, 0.125, 32.0, 1.0 }
    float [oscillators[1].level] "oscillators.1.level" "Oscillator 2 Level" { Gain, Linear, 0.0, 1.0, 0.0 }
    choice [oscillators[2].waveform] "oscillators.2.waveform" "Oscillator 3 Waveform" { HybridWaveform, Sine, [Sine "sine" "Sine", Triangle "triangle" "Triangle", Saw "saw" "Saw", Square "square" "Square"] }
    float [oscillators[2].ratio] "oscillators.2.ratio" "Oscillator 3 Ratio" { Ratio, Logarithmic, 0.125, 32.0, 1.0 }
    float [oscillators[2].level] "oscillators.2.level" "Oscillator 3 Level" { Gain, Linear, 0.0, 1.0, 0.0 }
    float [fm_21] "fm21" "FM 2 to 1" { None, Linear, -8.0, 8.0, 0.0 }
    float [fm_31] "fm31" "FM 3 to 1" { None, Linear, -8.0, 8.0, 0.0 }
    float [fm_32] "fm32" "FM 3 to 2" { None, Linear, -8.0, 8.0, 0.0 }
    float [ring_12] "ring12" "Ring 1 x 2" { Fraction, Linear, 0.0, 1.0, 0.0 }
    float [ring_13] "ring13" "Ring 1 x 3" { Fraction, Linear, 0.0, 1.0, 0.0 }
    float [cutoff_hz] "cutoffHz" "Cutoff" { Hertz, Logarithmic, 20.0, 20000.0, 12000.0 }
    float [resonance] "resonance" "Resonance" { Fraction, Linear, 0.0, 1.0, 0.0 }
    float [amp_envelope.attack_ms] "ampEnvelope.attackMs" "Amplitude Attack" { Milliseconds, Linear, 0.5, 10000.0, 2.0 }
    float [amp_envelope.decay_ms] "ampEnvelope.decayMs" "Amplitude Decay" { Milliseconds, Linear, 1.0, 10000.0, 150.0 }
    float [amp_envelope.sustain] "ampEnvelope.sustain" "Amplitude Sustain" { Fraction, Linear, 0.0, 1.0, 0.8 }
    float [amp_envelope.release_ms] "ampEnvelope.releaseMs" "Amplitude Release" { Milliseconds, Linear, 0.5, 10000.0, 120.0 }
    float [gain] "gain" "Gain" { Gain, Linear, 0.0, 1.0, 0.2 }
});
