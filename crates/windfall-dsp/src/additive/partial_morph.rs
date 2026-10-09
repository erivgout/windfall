//! An additive synthesizer that interpolates between two spectra.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::blocks::math::lerp;
use crate::param::{ParamInfo, ParamSet};

use super::engine::{AdditiveEngine, Common, MAX_PARTIALS, engine_instrument};
use super::labels::{MORPH_A_IDS, MORPH_A_NAMES, MORPH_B_IDS, MORPH_B_NAMES};
use super::partials::{PartialTone, Spectrum};
use super::rows::{BLANK, approached, cleaned, cleaned_at, fraction_row, hollow_gain, int_row};
use super::voicing::VoicingParams;

/// Harmonics in each snapshot. Half the engine's width is plenty for a
/// control that is meant to be swept, and it keeps a project file that
/// holds two of them a readable size.
pub const MORPH_PARTIALS: usize = 32;

/// Settings of the [`PartialMorph`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct PartialMorphParams {
    /// How many harmonics are sounded, counting the fundamental. 1 to 32,
    /// default 24.
    pub partials: u8,
    /// Where between the two snapshots the sound sits. 0 is snapshot A, 1
    /// is snapshot B, and every value between mixes their levels in that
    /// proportion. Default 0.
    pub morph: f32,
    /// The levels of snapshot A, 0 to 1 each. The default is the
    /// fundamental alone: a plain sine.
    pub snapshot_a: [f32; MORPH_PARTIALS],
    /// The levels of snapshot B, 0 to 1 each. The default is the odd
    /// harmonics of a saw, with the even ones silent: the hollow spectrum
    /// of a square.
    pub snapshot_b: [f32; MORPH_PARTIALS],
    pub voicing: VoicingParams,
}

/// The default snapshot A: the fundamental alone.
const fn default_a(index: usize) -> f32 {
    if index == 0 { 1.0 } else { 0.0 }
}

impl Default for PartialMorphParams {
    fn default() -> Self {
        Self {
            partials: 24,
            morph: 0.0,
            snapshot_a: std::array::from_fn(default_a),
            snapshot_b: std::array::from_fn(hollow_gain),
            voicing: VoicingParams::default(),
        }
    }
}

const SHAPE_ROWS: usize = 2;
const A_BASE: usize = SHAPE_ROWS;
const B_BASE: usize = A_BASE + MORPH_PARTIALS;
const VOICING_BASE: usize = B_BASE + MORPH_PARTIALS;
const ROWS: usize = VOICING_BASE + VoicingParams::ROWS;

static INFO: [ParamInfo; ROWS] = {
    let mut rows = [BLANK; ROWS];
    rows[0] = int_row("partials", "Partials", 1, MORPH_PARTIALS as i32, 24);
    rows[1] = fraction_row("morph", "Morph", 0.0, 1.0, 0.0);
    let mut partial = 0;
    while partial < MORPH_PARTIALS {
        rows[A_BASE + partial] = fraction_row(
            MORPH_A_IDS[partial],
            MORPH_A_NAMES[partial],
            0.0,
            1.0,
            default_a(partial),
        );
        rows[B_BASE + partial] = fraction_row(
            MORPH_B_IDS[partial],
            MORPH_B_NAMES[partial],
            0.0,
            1.0,
            hollow_gain(partial),
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

impl ParamSet for PartialMorphParams {
    const NAME: &'static str = "Partial morph";

    fn descriptors() -> &'static [ParamInfo] {
        &INFO
    }

    fn get(&self, index: usize) -> Option<f32> {
        match index {
            0 => Some(f32::from(self.partials)),
            1 => Some(self.morph),
            _ if index < B_BASE => Some(self.snapshot_a[index - A_BASE]),
            _ if index < VOICING_BASE => Some(self.snapshot_b[index - B_BASE]),
            _ => self.voicing.get(index - VOICING_BASE),
        }
    }

    fn set(&mut self, index: usize, value: f32) -> bool {
        let Some(value) = cleaned_at(&INFO, index, value) else {
            return false;
        };
        match index {
            0 => self.partials = value.round() as u8,
            1 => self.morph = value,
            _ if index < B_BASE => self.snapshot_a[index - A_BASE] = value,
            _ if index < VOICING_BASE => self.snapshot_b[index - B_BASE] = value,
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

/// An additive synthesizer that interpolates between two spectra.
///
/// The instrument holds two snapshots of harmonic levels and one control
/// that says where between them the sound sits. At 0 it plays snapshot A,
/// at 1 snapshot B, and anywhere between it plays the levels mixed in that
/// proportion, partial by partial. Sweeping the control is the instrument:
/// the two snapshots are usually far apart -- a sine and a hollow square by
/// default -- so moving between them is a change of timbre rather than of
/// level, and it glides like any other control, so it can be automated
/// across a note.
///
/// This is Windfall's answer to an additive synth that morphs between
/// harmonic snapshots (parity `inst-morphine`). What separates it from
/// [`HarmonicStack`](super::HarmonicStack) is that no single set of levels
/// describes it: the spectrum it plays is derived from two of them, and the
/// one control that matters is not in either.
pub struct PartialMorph {
    params: PartialMorphParams,
    engine: AdditiveEngine<PartialTone>,
}

impl PartialMorph {
    /// Hands the engine the spectrum the current parameters ask for.
    fn apply(&mut self) {
        let params = &self.params;
        let count = usize::from(params.partials).clamp(1, MORPH_PARTIALS);
        let mut spectrum = Spectrum::SILENT;
        for index in 0..count {
            let gain = lerp(
                params.snapshot_a[index],
                params.snapshot_b[index],
                params.morph,
            );
            spectrum.push((index + 1) as f32, gain, 0.0);
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

engine_instrument!(PartialMorph, PartialMorphParams);

/// Both snapshots have to fit in the engine's bank.
const _: () = assert!(MORPH_PARTIALS <= MAX_PARTIALS);
