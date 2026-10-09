use crate::balance::{Controls, audio};
use crate::blocks::math::db_to_gain;
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const CURVE_POINTS: usize = 33;
/// Knot inputs are evenly spaced over -1..=1, with the origin at index 16.
pub const IDENTITY_CURVE: [f32; CURVE_POINTS] = [
    -1.0, -0.9375, -0.875, -0.8125, -0.75, -0.6875, -0.625, -0.5625, -0.5, -0.4375, -0.375,
    -0.3125, -0.25, -0.1875, -0.125, -0.0625, 0.0, 0.0625, 0.125, 0.1875, 0.25, 0.3125, 0.375,
    0.4375, 0.5, 0.5625, 0.625, 0.6875, 0.75, 0.8125, 0.875, 0.9375, 1.0,
];

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct WaveshaperParams {
    /// Input gain in dB, -24..36; default 0.
    pub input_db: f32,
    /// Output trim in dB, -24..12; default 0.
    pub output_db: f32,
    /// Output at each fixed input knot; values are bounded to -1..=1.
    #[serde(with = "curve_serde")]
    #[ts(as = "[f32; 33]")]
    pub curve: [f32; CURVE_POINTS],
}

// Serde's built-in array implementations stop at 32. Persist exactly 33
// values using fixed storage, and reject both short and oversized curves.
mod curve_serde {
    use super::CURVE_POINTS;
    use serde::de::{Error, SeqAccess, Visitor};
    use serde::{Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(
        curve: &[f32; CURVE_POINTS],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        curve.as_slice().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[f32; CURVE_POINTS], D::Error> {
        struct CurveVisitor;
        impl<'de> Visitor<'de> for CurveVisitor {
            type Value = [f32; CURVE_POINTS];
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("exactly 33 curve points")
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut curve = [0.0; CURVE_POINTS];
                for (index, point) in curve.iter_mut().enumerate() {
                    *point = sequence
                        .next_element()?
                        .ok_or_else(|| A::Error::invalid_length(index, &self))?;
                }
                if sequence.next_element::<serde::de::IgnoredAny>()?.is_some() {
                    return Err(A::Error::invalid_length(CURVE_POINTS + 1, &self));
                }
                Ok(curve)
            }
        }
        deserializer.deserialize_seq(CurveVisitor)
    }
}

impl Default for WaveshaperParams {
    fn default() -> Self {
        Self {
            input_db: 0.0,
            output_db: 0.0,
            curve: IDENTITY_CURVE,
        }
    }
}

param_set!(WaveshaperParams, "Waveshaper", {
    float [input_db] "inputDb" "Input" { Decibels, Linear, -24.0, 36.0, 0.0 }
    float [output_db] "outputDb" "Output" { Decibels, Linear, -24.0, 12.0, 0.0 }
    float [curve[0]] "curve.0" "Point 1" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[0] }
    float [curve[1]] "curve.1" "Point 2" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[1] }
    float [curve[2]] "curve.2" "Point 3" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[2] }
    float [curve[3]] "curve.3" "Point 4" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[3] }
    float [curve[4]] "curve.4" "Point 5" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[4] }
    float [curve[5]] "curve.5" "Point 6" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[5] }
    float [curve[6]] "curve.6" "Point 7" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[6] }
    float [curve[7]] "curve.7" "Point 8" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[7] }
    float [curve[8]] "curve.8" "Point 9" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[8] }
    float [curve[9]] "curve.9" "Point 10" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[9] }
    float [curve[10]] "curve.10" "Point 11" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[10] }
    float [curve[11]] "curve.11" "Point 12" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[11] }
    float [curve[12]] "curve.12" "Point 13" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[12] }
    float [curve[13]] "curve.13" "Point 14" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[13] }
    float [curve[14]] "curve.14" "Point 15" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[14] }
    float [curve[15]] "curve.15" "Point 16" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[15] }
    float [curve[16]] "curve.16" "Point 17" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[16] }
    float [curve[17]] "curve.17" "Point 18" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[17] }
    float [curve[18]] "curve.18" "Point 19" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[18] }
    float [curve[19]] "curve.19" "Point 20" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[19] }
    float [curve[20]] "curve.20" "Point 21" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[20] }
    float [curve[21]] "curve.21" "Point 22" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[21] }
    float [curve[22]] "curve.22" "Point 23" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[22] }
    float [curve[23]] "curve.23" "Point 24" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[23] }
    float [curve[24]] "curve.24" "Point 25" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[24] }
    float [curve[25]] "curve.25" "Point 26" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[25] }
    float [curve[26]] "curve.26" "Point 27" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[26] }
    float [curve[27]] "curve.27" "Point 28" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[27] }
    float [curve[28]] "curve.28" "Point 29" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[28] }
    float [curve[29]] "curve.29" "Point 30" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[29] }
    float [curve[30]] "curve.30" "Point 31" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[30] }
    float [curve[31]] "curve.31" "Point 32" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[31] }
    float [curve[32]] "curve.32" "Point 33" { None, Linear, -1.0, 1.0, IDENTITY_CURVE[32] }
});

#[inline]
fn lookup(input: f32, curve: &[f32; CURVE_POINTS]) -> f32 {
    let input = input.clamp(-1.0, 1.0);
    let position = (input + 1.0) * 16.0;
    let index = (position as usize).min(CURVE_POINTS - 2);
    // Interpolate the deviation from identity. This preserves exact unity
    // for the default curve even when a tiny input is rounded in position.
    let a = curve[index] - IDENTITY_CURVE[index];
    let b = curve[index + 1] - IDENTITY_CURVE[index + 1];
    input + a + (b - a) * (position - index as f32)
}

impl WaveshaperParams {
    pub fn transfer(&self, input: f32) -> f32 {
        let p = self.sanitized();
        audio(lookup(audio(input) * db_to_gain(p.input_db), &p.curve) * db_to_gain(p.output_db))
    }
}

pub struct Waveshaper {
    controls: Controls<35>,
}

impl Default for Waveshaper {
    fn default() -> Self {
        Self {
            controls: Controls::new(Self::values(&WaveshaperParams::default())),
        }
    }
}

impl Waveshaper {
    fn values(p: &WaveshaperParams) -> [f32; 35] {
        let mut values = [0.0; 35];
        values[0] = db_to_gain(p.input_db);
        values[1] = db_to_gain(p.output_db);
        values[2..].copy_from_slice(&p.curve);
        values
    }
}

impl Effect for Waveshaper {
    type Params = WaveshaperParams;
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
            let curve = std::array::from_fn(|i| values[i + 2]);
            *l = audio(lookup(audio(*l) * values[0], &curve) * values[1]);
            *r = audio(lookup(audio(*r) * values[0], &curve) * values[1]);
        }
    }
}
