//! An additive synthesizer whose whole spectrum comes from one number.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::blocks::noise::Rng;
use crate::param::{ParamInfo, ParamSet};

use super::engine::{AdditiveEngine, Common, MAX_PARTIALS, engine_instrument};
use super::partials::{PartialTone, Spectrum};
use super::rows::{BLANK, approached, cleaned, cleaned_at, fraction_row, int_row};
use super::voicing::VoicingParams;

/// Largest seed a control can address.
///
/// The seed is a `u32`, but every control of every processor reads and
/// writes as an `f32`, which carries whole numbers exactly only up to
/// 2^24. The range stops there so that a seed cannot be changed by being
/// read and written back.
pub const MAX_SEED: u32 = (1 << 24) - 1;

/// How far a partial can be moved off its harmonic at a spread of 1, as a
/// share of its frequency. Under half keeps the lowest partial above zero
/// and the series roughly in order.
const MAX_WANDER: f32 = 0.35;

/// The quietest a drawn level can be, as a share of the loudest. A patch
/// where some partials were silent would waste them.
const QUIETEST: f32 = 0.2;

/// Octaves of fall per octave of frequency at a rolloff of 1.
const ROLLOFF_SLOPE: f32 = 4.0;

/// Settings of the [`SeedPatch`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct SeedPatchParams {
    /// The number the whole spectrum is derived from. Any two seeds give
    /// unrelated timbres, and the same seed always gives the same one. 0 to
    /// 16777215, default 12345.
    pub seed: u32,
    /// How many partials are sounded, counting the lowest. 1 to 64,
    /// default 32. Raising it adds partials above the ones already there
    /// rather than changing them.
    pub partials: u8,
    /// How far the seed is allowed to move the partials off the harmonic
    /// series. At 0 the spectrum is harmonic and only the levels are drawn;
    /// toward 1 the partials wander and the sound turns metallic. 0 to 1,
    /// default 0.3.
    pub spread: f32,
    /// How much quieter each partial is than the one below. 0 to 1,
    /// default 0.3.
    pub rolloff: f32,
    pub voicing: VoicingParams,
}

impl Default for SeedPatchParams {
    fn default() -> Self {
        Self {
            seed: 12_345,
            partials: 32,
            spread: 0.3,
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
    rows[0] = int_row("seed", "Seed", 0, MAX_SEED as i32, 12_345);
    rows[1] = int_row("partials", "Partials", 1, MAX_PARTIALS as i32, 32);
    rows[2] = fraction_row("spread", "Spread", 0.0, 1.0, 0.3);
    rows[3] = fraction_row("rolloff", "Rolloff", 0.0, 1.0, 0.3);
    let voicing = VoicingParams::rows();
    let mut row = 0;
    while row < VoicingParams::ROWS {
        rows[VOICING_BASE + row] = voicing[row];
        row += 1;
    }
    rows
};

impl ParamSet for SeedPatchParams {
    const NAME: &'static str = "Seed patch";

    fn descriptors() -> &'static [ParamInfo] {
        &INFO
    }

    fn get(&self, index: usize) -> Option<f32> {
        match index {
            0 => Some(self.seed as f32),
            1 => Some(f32::from(self.partials)),
            2 => Some(self.spread),
            3 => Some(self.rolloff),
            _ => self.voicing.get(index - VOICING_BASE),
        }
    }

    fn set(&mut self, index: usize, value: f32) -> bool {
        let Some(value) = cleaned_at(&INFO, index, value) else {
            return false;
        };
        match index {
            0 => self.seed = value.round() as u32,
            1 => self.partials = value.round() as u8,
            2 => self.spread = value,
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

/// An additive synthesizer whose whole spectrum comes from one number.
///
/// There are no per-partial controls. A seed is fed to the same xorshift
/// generator every other part of Windfall uses, and its numbers become the
/// frequency and the level of each partial in turn. The same seed always
/// gives the same timbre, on any machine and at any sample rate, so a patch
/// is a number that can be written down; two neighbouring seeds give
/// unrelated sounds, so the way to use it is to go looking. Three controls
/// say how far the seed is allowed to reach: how many partials it fills,
/// how far it may move them off the harmonic series, and how fast the
/// spectrum falls away.
///
/// Because the draws happen lowest partial first, raising the count leaves
/// the partials already there untouched and adds above them.
///
/// This is Windfall's answer to a synth whose patches are a number rather
/// than a page of controls (parity `inst-autogun`). What separates it from
/// the other five is that its spectrum is in none of its parameters: it is
/// generated, and the only way to change one partial is to change the seed
/// and get a different sound altogether.
pub struct SeedPatch {
    params: SeedPatchParams,
    engine: AdditiveEngine<PartialTone>,
}

impl SeedPatch {
    /// Hands the engine the spectrum the current parameters ask for.
    fn apply(&mut self) {
        let params = &self.params;
        let count = usize::from(params.partials).clamp(1, MAX_PARTIALS);
        let slope = params.rolloff * ROLLOFF_SLOPE;
        let mut rng = Rng::new(params.seed);
        let mut spectrum = Spectrum::SILENT;
        for index in 0..count {
            let number = (index + 1) as f32;
            // Two draws per partial, always in this order, so that the
            // spectrum depends on the seed and on nothing else.
            let wander = rng.bipolar();
            let drawn_level = rng.unipolar();
            let ratio = number * (1.0 + params.spread * MAX_WANDER * wander);
            let level = QUIETEST + (1.0 - QUIETEST) * drawn_level;
            let rolloff = if slope == 0.0 {
                1.0
            } else {
                number.powf(-slope)
            };
            spectrum.push(ratio, level * rolloff, 0.0);
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

engine_instrument!(SeedPatch, SeedPatchParams);
