//! An additive synthesizer that plays a table of partials.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::param::{ParamInfo, ParamScale, ParamSet, ParamUnit};

use super::engine::{AdditiveEngine, Common, MAX_PARTIALS, engine_instrument};
use super::labels::{RESYNTH_IDS, RESYNTH_NAMES};
use super::partials::{PartialTone, Spectrum};
use super::rows::{
    BLANK, approached, cleaned, cleaned_at, float_row, fraction_row, int_row, saw_gain,
};
use super::voicing::VoicingParams;

/// Rows in the table. Each one costs three controls, so the table is held
/// to half the engine's width.
pub const TABLE_PARTIALS: usize = 32;

/// Smallest and largest frequency ratio a row may hold.
const MIN_RATIO: f32 = 0.25;
const MAX_RATIO: f32 = 64.0;

/// Smallest and largest factor the whole table can be stretched by.
const MIN_STRETCH: f32 = 0.5;
const MAX_STRETCH: f32 = 2.0;

/// One row of the [`Resynth`] table: a partial of the sound it plays.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct TablePartial {
    /// The partial's frequency as a multiple of the note's pitch. 0.25 to
    /// 64. Nothing requires it to be a whole number.
    pub ratio: f32,
    /// The partial's level, 0 to 1.
    pub gain: f32,
    /// Where in its cycle the partial starts, 0 to 1. Every note starts its
    /// partials here, so the same note always sounds the same.
    pub phase: f32,
}

impl Default for TablePartial {
    fn default() -> Self {
        Self {
            ratio: 1.0,
            gain: 0.0,
            phase: 0.0,
        }
    }
}

/// Where the `index`th partial starts by default: an eighth of a cycle
/// further on each time, round and round.
///
/// A table whose partials all started together would be the same sound as
/// a [`HarmonicStack`](super::HarmonicStack) with the same levels, and the
/// column that makes this instrument what it is would be doing nothing.
/// Spreading them shows it: the spectrum is still a saw, and the waveform
/// is not.
const fn default_phase(index: usize) -> f32 {
    (index % 8) as f32 / 8.0
}

/// The default table: the harmonic series with the levels of a saw and
/// their phases spread out.
fn default_row(index: usize) -> TablePartial {
    TablePartial {
        ratio: (index + 1) as f32,
        gain: saw_gain(index),
        phase: default_phase(index),
    }
}

/// Settings of the [`Resynth`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ResynthParams {
    /// How many rows of the table are played, from the top. 1 to 32,
    /// default 16.
    pub partials: u8,
    /// Multiplies every ratio in the table, which moves the partials apart
    /// or together without touching their levels. 0.5 to 2, default 1.
    pub stretch: f32,
    /// The table: a frequency ratio, a level and a starting phase per
    /// partial. The default is the harmonic series with the levels of a
    /// saw.
    pub table: [TablePartial; TABLE_PARTIALS],
    pub voicing: VoicingParams,
}

impl Default for ResynthParams {
    fn default() -> Self {
        Self {
            partials: 16,
            stretch: 1.0,
            table: std::array::from_fn(default_row),
            voicing: VoicingParams::default(),
        }
    }
}

const SHAPE_ROWS: usize = 2;
const TABLE_BASE: usize = SHAPE_ROWS;
const TABLE_ROWS: usize = 3 * TABLE_PARTIALS;
const VOICING_BASE: usize = TABLE_BASE + TABLE_ROWS;
const ROWS: usize = VOICING_BASE + VoicingParams::ROWS;

static INFO: [ParamInfo; ROWS] = {
    let mut rows = [BLANK; ROWS];
    rows[0] = int_row("partials", "Partials", 1, TABLE_PARTIALS as i32, 16);
    rows[1] = float_row(
        "stretch",
        "Stretch",
        ParamUnit::Ratio,
        ParamScale::Logarithmic,
        MIN_STRETCH,
        MAX_STRETCH,
        1.0,
    );
    let mut partial = 0;
    while partial < TABLE_PARTIALS {
        let row = TABLE_BASE + 3 * partial;
        rows[row] = float_row(
            RESYNTH_IDS[3 * partial],
            RESYNTH_NAMES[3 * partial],
            ParamUnit::Ratio,
            ParamScale::Logarithmic,
            MIN_RATIO,
            MAX_RATIO,
            (partial + 1) as f32,
        );
        rows[row + 1] = fraction_row(
            RESYNTH_IDS[3 * partial + 1],
            RESYNTH_NAMES[3 * partial + 1],
            0.0,
            1.0,
            saw_gain(partial),
        );
        rows[row + 2] = fraction_row(
            RESYNTH_IDS[3 * partial + 2],
            RESYNTH_NAMES[3 * partial + 2],
            0.0,
            1.0,
            default_phase(partial),
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

impl ParamSet for ResynthParams {
    const NAME: &'static str = "Resynth";

    fn descriptors() -> &'static [ParamInfo] {
        &INFO
    }

    fn get(&self, index: usize) -> Option<f32> {
        match index {
            0 => Some(f32::from(self.partials)),
            1 => Some(self.stretch),
            _ if index < VOICING_BASE => {
                let row = &self.table[(index - TABLE_BASE) / 3];
                Some(match (index - TABLE_BASE) % 3 {
                    0 => row.ratio,
                    1 => row.gain,
                    _ => row.phase,
                })
            }
            _ => self.voicing.get(index - VOICING_BASE),
        }
    }

    fn set(&mut self, index: usize, value: f32) -> bool {
        let Some(value) = cleaned_at(&INFO, index, value) else {
            return false;
        };
        match index {
            0 => self.partials = value.round() as u8,
            1 => self.stretch = value,
            _ if index < VOICING_BASE => {
                let row = &mut self.table[(index - TABLE_BASE) / 3];
                match (index - TABLE_BASE) % 3 {
                    0 => row.ratio = value,
                    1 => row.gain = value,
                    _ => row.phase = value,
                }
            }
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

/// An additive synthesizer that plays a table of partials.
///
/// The table is the oscillator. Each of its 32 rows gives a frequency as a
/// ratio of the note's pitch, a level and a starting phase, and a note is
/// those partials sounded together. Because every row carries its own
/// phase, and every note starts its partials from the phases in the table,
/// the same note always comes out the same waveform rather than the same
/// spectrum, which is what lets a table hold a sound that was measured
/// rather than designed.
///
/// This is Windfall's answer to an additive synth that can resynthesize
/// (parity `inst-harmor`). The table is data in the project: Windfall does
/// not read an audio file or an image to fill it. **Image resynthesis is
/// not included**, and neither is analysing a recording into partials;
/// whatever writes the table -- a preset, an editor, an analysis pass added
/// later -- is outside this instrument.
///
/// What separates it from [`HarmonicStack`](super::HarmonicStack) is that
/// the frequencies and the phases are data too, not just the levels.
pub struct Resynth {
    params: ResynthParams,
    engine: AdditiveEngine<PartialTone>,
}

impl Resynth {
    /// Hands the engine the spectrum the current parameters ask for.
    fn apply(&mut self) {
        let params = &self.params;
        let count = usize::from(params.partials).clamp(1, TABLE_PARTIALS);
        let mut spectrum = Spectrum::SILENT;
        for row in &params.table[..count] {
            spectrum.push(row.ratio * params.stretch, row.gain, row.phase);
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

engine_instrument!(Resynth, ResynthParams);

/// The table has to fit in the engine's bank.
const _: () = assert!(TABLE_PARTIALS <= MAX_PARTIALS);
