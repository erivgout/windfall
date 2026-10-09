//! An additive synthesizer whose partials are not harmonics.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::param::{ParamInfo, ParamScale, ParamSet, ParamUnit};

use super::engine::{AdditiveEngine, Common, MAX_PARTIALS, engine_instrument};
use super::partials::{PartialTone, Spectrum};
use super::rows::{BLANK, approached, cleaned, cleaned_at, float_row, fraction_row, int_row};
use super::voicing::VoicingParams;

/// Smallest and largest stiffness the control reaches, as the `B` of the
/// stiff-string series. Together with the floor under the stretch, the low
/// end keeps every partial above the lowest at least 2% away from the
/// harmonic it would otherwise be, at every setting.
const MIN_STIFFNESS: f32 = 0.01;
const MAX_STIFFNESS: f32 = 0.3;

/// Smallest stretch the control reaches. It does not go to zero: a series
/// that could become harmonic is not what this instrument is for.
const MIN_STRETCH: f32 = 0.02;
const MAX_STRETCH: f32 = 0.3;

/// Octaves of fall per octave of frequency at a rolloff of 1.
const ROLLOFF_SLOPE: f32 = 4.0;

/// Settings of the [`Inharmonic`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct InharmonicParams {
    /// How many partials are sounded, counting the lowest. 1 to 64,
    /// default 24.
    pub partials: u8,
    /// How much the partials spread apart as they climb. At 0 the series is
    /// close to a harmonic one and sounds like a struck string; toward 1 it
    /// opens out into a bell. 0 to 1, default 0.35.
    pub stiffness: f32,
    /// Bends the whole series by a power of the partial number, which no
    /// amount of stiffness can do: it moves the low partials as well as the
    /// high ones. 0.02 to 0.3, default 0.08.
    pub stretch: f32,
    /// How much quieter each partial is than the one below. 0 is a flat
    /// spectrum; 1 takes 24 dB off every octave of frequency. 0 to 1,
    /// default 0.3.
    pub rolloff: f32,
    pub voicing: VoicingParams,
}

impl Default for InharmonicParams {
    fn default() -> Self {
        Self {
            partials: 24,
            stiffness: 0.35,
            stretch: 0.08,
            rolloff: 0.3,
            voicing: VoicingParams::default(),
        }
    }
}

const SHAPE_ROWS: usize = 4;
const VOICING_BASE: usize = SHAPE_ROWS;
const ROWS: usize = VOICING_BASE + VoicingParams::ROWS;

static INFO: [ParamInfo; ROWS] = {
    let mut rows = [BLANK; ROWS];
    rows[0] = int_row("partials", "Partials", 1, MAX_PARTIALS as i32, 24);
    rows[1] = fraction_row("stiffness", "Stiffness", 0.0, 1.0, 0.35);
    rows[2] = float_row(
        "stretch",
        "Stretch",
        ParamUnit::Fraction,
        ParamScale::Linear,
        MIN_STRETCH,
        MAX_STRETCH,
        0.08,
    );
    rows[3] = fraction_row("rolloff", "Rolloff", 0.0, 1.0, 0.3);
    let voicing = VoicingParams::rows();
    let mut row = 0;
    while row < VoicingParams::ROWS {
        rows[VOICING_BASE + row] = voicing[row];
        row += 1;
    }
    rows
};

impl ParamSet for InharmonicParams {
    const NAME: &'static str = "Inharmonic";

    fn descriptors() -> &'static [ParamInfo] {
        &INFO
    }

    fn get(&self, index: usize) -> Option<f32> {
        match index {
            0 => Some(f32::from(self.partials)),
            1 => Some(self.stiffness),
            2 => Some(self.stretch),
            3 => Some(self.rolloff),
            _ => self.voicing.get(index - VOICING_BASE),
        }
    }

    fn set(&mut self, index: usize, value: f32) -> bool {
        let Some(value) = cleaned_at(&INFO, index, value) else {
            return false;
        };
        match index {
            0 => self.partials = value.round() as u8,
            1 => self.stiffness = value,
            2 => self.stretch = value,
            3 => self.rolloff = value,
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

/// The frequency of partial `number` as a ratio of the note's pitch.
///
/// A real string is stiff as well as stretched, and stiffness raises its
/// overtones: the `n`th is at `n * sqrt(1 + B n^2)` rather than at `n`
/// (Fletcher and Rossing, *The Physics of Musical Instruments*, on the
/// inharmonicity of piano strings). A bar or a bell goes further still, so
/// the series is also bent by a power of the partial number, which spreads
/// the low partials rather than only the high ones.
fn partial_ratio(number: f32, stiffness: f32, stretch: f32) -> f32 {
    number.powf(1.0 + stretch) * (1.0 + stiffness * number * number).sqrt()
}

/// An additive synthesizer whose partials are not harmonics.
///
/// Every note is a bank of sines whose frequencies come from a stiff
/// string rather than from the harmonic series, bent further by a power
/// law. Nothing in the bank lands on a whole-number multiple of the pitch,
/// so the sound has no clear octave: it is struck metal, a bell or a tuned
/// bar, and it stays that way at every setting. The two controls that
/// matter are stiffness, which spreads the partials apart as they climb,
/// and stretch, which moves the low ones too.
///
/// This is Windfall's answer to an additive synth aimed at metallic and
/// shimmering timbres (parity `inst-ogun`). Where
/// [`HarmonicStack`](super::HarmonicStack) puts a level on every harmonic,
/// this one has no per-partial controls at all: the whole spectrum,
/// frequencies and levels alike, follows from the physical model.
pub struct Inharmonic {
    params: InharmonicParams,
    engine: AdditiveEngine<PartialTone>,
}

impl Inharmonic {
    /// Hands the engine the spectrum the current parameters ask for.
    fn apply(&mut self) {
        let params = &self.params;
        let count = usize::from(params.partials).clamp(1, MAX_PARTIALS);
        let stiffness = MIN_STIFFNESS * (MAX_STIFFNESS / MIN_STIFFNESS).powf(params.stiffness);
        let slope = params.rolloff * ROLLOFF_SLOPE;
        // The lowest partial is the pitch that is played, so the series is
        // divided by its own first term.
        let lowest = partial_ratio(1.0, stiffness, params.stretch);
        let mut spectrum = Spectrum::SILENT;
        for index in 0..count {
            let number = (index + 1) as f32;
            let ratio = partial_ratio(number, stiffness, params.stretch) / lowest;
            let gain = if slope == 0.0 {
                1.0
            } else {
                number.powf(-slope)
            };
            spectrum.push(ratio, gain, 0.0);
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

engine_instrument!(Inharmonic, InharmonicParams);
