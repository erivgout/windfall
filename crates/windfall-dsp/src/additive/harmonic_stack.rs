//! An additive synthesizer whose harmonics are set one at a time.

use std::fmt;
use std::ops::{Index, IndexMut};

use serde::de::{SeqAccess, Visitor};
use serde::ser::SerializeTuple;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::TS;

use crate::param::{ParamInfo, ParamScale, ParamSet, ParamUnit};

use super::engine::{AdditiveEngine, Common, MAX_PARTIALS, engine_instrument};
use super::labels::{PARTIAL_NAMES, STACK_GAIN_IDS};
use super::partials::{PartialTone, Spectrum};
use super::rows::{
    BLANK, approached, cleaned, cleaned_at, float_row, fraction_row, int_row, saw_gain,
};
use super::voicing::VoicingParams;

/// Octaves of fall per octave of frequency at a rolloff of 1.
const ROLLOFF_SLOPE: f32 = 4.0;

/// The level of each of the [`MAX_PARTIALS`] partials.
///
/// `serde` describes arrays of up to 32 entries, so a bank twice that wide
/// carries its own pair of impls. In JSON it is a plain array of numbers.
/// A shorter one leaves the partials past its end silent, so a project
/// written by a version with fewer partials, or a damaged one, still loads.
#[derive(Debug, Clone, Copy, PartialEq, TS)]
#[ts(export)]
pub struct PartialGains([f32; MAX_PARTIALS]);

impl Default for PartialGains {
    /// The spectrum of a saw: each partial `1 / n` as loud as the
    /// fundamental. A bank of 64 partials all at full level would add up to
    /// a level no one asked for, so the default falls away.
    fn default() -> Self {
        Self(std::array::from_fn(saw_gain))
    }
}

impl PartialGains {
    /// Every level, lowest partial first.
    pub fn as_slice(&self) -> &[f32] {
        &self.0
    }

    /// A bank where partial `index` is `levels(index)` loud.
    pub fn from_fn(levels: impl FnMut(usize) -> f32) -> Self {
        Self(std::array::from_fn(levels))
    }
}

impl Index<usize> for PartialGains {
    type Output = f32;
    fn index(&self, index: usize) -> &f32 {
        &self.0[index]
    }
}

impl IndexMut<usize> for PartialGains {
    fn index_mut(&mut self, index: usize) -> &mut f32 {
        &mut self.0[index]
    }
}

impl Serialize for PartialGains {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut array = serializer.serialize_tuple(MAX_PARTIALS)?;
        for gain in &self.0 {
            array.serialize_element(gain)?;
        }
        array.end()
    }
}

impl<'de> Deserialize<'de> for PartialGains {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Gains;

        impl<'de> Visitor<'de> for Gains {
            type Value = PartialGains;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "up to {MAX_PARTIALS} partial levels")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut gains = PartialGains([0.0; MAX_PARTIALS]);
                let mut at = 0;
                while let Some(gain) = seq.next_element::<f32>()? {
                    if at < MAX_PARTIALS {
                        gains.0[at] = gain;
                    }
                    at += 1;
                }
                Ok(gains)
            }
        }

        deserializer.deserialize_seq(Gains)
    }
}

/// Settings of the [`HarmonicStack`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct HarmonicStackParams {
    /// How many harmonics are sounded, counting the fundamental. 1 to 64,
    /// default 16. Each one costs a sine per sample per note.
    pub partials: u8,
    /// Which half of the harmonics is favoured. At -1 only the odd ones
    /// sound, which is hollow and clarinet-like; at 1 only the even ones,
    /// which is the same series an octave up. -1 to 1, default 0.
    pub odd_even_tilt: f32,
    /// How much quieter each harmonic is than the one below, on top of the
    /// levels set for them. 0 leaves them alone; 1 takes 24 dB off every
    /// octave of frequency. 0 to 1, default 0.
    pub rolloff: f32,
    /// The level of each harmonic, 0 to 1 each. The default is the
    /// spectrum of a saw.
    pub gains: PartialGains,
    pub voicing: VoicingParams,
}

impl Default for HarmonicStackParams {
    fn default() -> Self {
        Self {
            partials: 16,
            odd_even_tilt: 0.0,
            rolloff: 0.0,
            gains: PartialGains::default(),
            voicing: VoicingParams::default(),
        }
    }
}

const SHAPE_ROWS: usize = 3;
const GAIN_BASE: usize = SHAPE_ROWS;
const VOICING_BASE: usize = GAIN_BASE + MAX_PARTIALS;
const ROWS: usize = VOICING_BASE + VoicingParams::ROWS;

static INFO: [ParamInfo; ROWS] = {
    let mut rows = [BLANK; ROWS];
    rows[0] = int_row("partials", "Partials", 1, MAX_PARTIALS as i32, 16);
    rows[1] = fraction_row("oddEvenTilt", "Odd/even tilt", -1.0, 1.0, 0.0);
    rows[2] = float_row(
        "rolloff",
        "Rolloff",
        ParamUnit::Fraction,
        ParamScale::Linear,
        0.0,
        1.0,
        0.0,
    );
    let mut partial = 0;
    while partial < MAX_PARTIALS {
        rows[GAIN_BASE + partial] = fraction_row(
            STACK_GAIN_IDS[partial],
            PARTIAL_NAMES[partial],
            0.0,
            1.0,
            saw_gain(partial),
        );
        partial += 1;
    }
    let voicing = VoicingParams::rows();
    let mut row = 0;
    while row < VoicingParams::ROWS {
        rows[VOICING_BASE + row] = voicing[row];
        row += 1;
    }
    rows
};

impl ParamSet for HarmonicStackParams {
    const NAME: &'static str = "Harmonic stack";

    fn descriptors() -> &'static [ParamInfo] {
        &INFO
    }

    fn get(&self, index: usize) -> Option<f32> {
        match index {
            0 => Some(f32::from(self.partials)),
            1 => Some(self.odd_even_tilt),
            2 => Some(self.rolloff),
            _ if index < VOICING_BASE => Some(self.gains[index - GAIN_BASE]),
            _ => self.voicing.get(index - VOICING_BASE),
        }
    }

    fn set(&mut self, index: usize, value: f32) -> bool {
        let Some(value) = cleaned_at(&INFO, index, value) else {
            return false;
        };
        match index {
            0 => self.partials = value.round() as u8,
            1 => self.odd_even_tilt = value,
            2 => self.rolloff = value,
            _ if index < VOICING_BASE => self.gains[index - GAIN_BASE] = value,
            _ => return self.voicing.set(index - VOICING_BASE, value),
        }
        true
    }

    fn sanitized(&self) -> Self {
        cleaned(self)
    }

    fn approach(&mut self, target: &Self, amount: f32) -> bool {
        approached(self, target, amount)
    }
}

/// An additive synthesizer whose harmonics are set one at a time.
///
/// Every note is a stack of up to 64 sines at whole-number multiples of its
/// pitch. The level of each is set directly, and two controls shape the
/// whole stack at once: a tilt that fades out the odd or the even
/// harmonics, and a rolloff that takes the top off. There is no filter and
/// no modulation; what the levels say is what is heard.
///
/// This is Windfall's answer to an additive engine presented with ordinary
/// controls (parity `inst-harmless`). It is the plainest of the six: the
/// partials are harmonic, their levels are the parameters, and nothing
/// derives them. [`PartialMorph`](super::PartialMorph) interpolates between
/// two such spectra, [`Inharmonic`](super::Inharmonic) moves the partials
/// off the harmonic series, and [`SeedPatch`](super::SeedPatch) fills them
/// from a number.
pub struct HarmonicStack {
    params: HarmonicStackParams,
    engine: AdditiveEngine<PartialTone>,
}

impl HarmonicStack {
    /// Hands the engine the spectrum the current parameters ask for.
    fn apply(&mut self) {
        let params = &self.params;
        let count = usize::from(params.partials).clamp(1, MAX_PARTIALS);
        let odd = 1.0 - params.odd_even_tilt.max(0.0);
        let even = 1.0 + params.odd_even_tilt.min(0.0);
        let slope = params.rolloff * ROLLOFF_SLOPE;
        let mut spectrum = Spectrum::SILENT;
        for index in 0..count {
            let number = (index + 1) as f32;
            let side = if (index + 1).is_multiple_of(2) {
                even
            } else {
                odd
            };
            let rolloff = if slope == 0.0 {
                1.0
            } else {
                number.powf(-slope)
            };
            spectrum.push(number, params.gains[index] * side * rolloff, 0.0);
        }
        self.engine
            .set_target(&spectrum, &Common::new(&params.voicing));
    }

    /// The spectrum the engine was last given, for a test to measure
    /// against.
    #[cfg(test)]
    pub(super) fn spectrum(&self) -> Spectrum {
        self.engine.target()
    }
}

engine_instrument!(HarmonicStack, HarmonicStackParams);
