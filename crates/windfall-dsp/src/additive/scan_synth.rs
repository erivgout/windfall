//! A synthesizer that scans a row of values as its waveform.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::blocks::math::smoothing_coefficient;
use crate::param::{ParamInfo, ParamSet};

use super::engine::{AdditiveEngine, Common, engine_instrument};
use super::labels::{SCAN_ROW_IDS, SCAN_ROW_NAMES};
use super::rows::{BLANK, approached, cleaned, cleaned_at, fraction_row, toggle_row};
use super::scan::{SCAN_STEPS, ScanShape, ScanTone};
use super::voicing::VoicingParams;

/// Time the smoother takes to cover 63% of a step at a smoothing of 1.
const SMOOTHEST_MS: f32 = 0.5;

/// The default row: one cycle of a 32-step ramp, which has no offset and
/// plenty of overtones to hear the steps in.
const fn default_step(index: usize) -> f32 {
    2.0 * index as f32 / (SCAN_STEPS - 1) as f32 - 1.0
}

/// Settings of the [`ScanSynth`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ScanSynthParams {
    /// The row that is scanned: 32 values from -1 to 1, which are the
    /// waveform. One cycle of the note covers all of them, lowest index
    /// first. The default is a ramp.
    pub row: [f32; SCAN_STEPS],
    /// Whether the steps are joined by a straight line instead of being
    /// held. Default off, which is the staircase the instrument is for.
    pub interpolate: bool,
    /// Rounds off the corners of the staircase. 0 leaves them alone;
    /// toward 1 the steps are smeared into one another and the sound
    /// darkens. 0 to 1, default 0.
    pub smoothing: f32,
    pub voicing: VoicingParams,
}

impl Default for ScanSynthParams {
    fn default() -> Self {
        Self {
            row: std::array::from_fn(default_step),
            interpolate: false,
            smoothing: 0.0,
            voicing: VoicingParams::default(),
        }
    }
}

const ROW_BASE: usize = 0;
const SHAPE_BASE: usize = ROW_BASE + SCAN_STEPS;
const VOICING_BASE: usize = SHAPE_BASE + 2;
const ROWS: usize = VOICING_BASE + VoicingParams::ROWS;

static INFO: [ParamInfo; ROWS] = {
    let mut rows = [BLANK; ROWS];
    let mut step = 0;
    while step < SCAN_STEPS {
        rows[ROW_BASE + step] = fraction_row(
            SCAN_ROW_IDS[step],
            SCAN_ROW_NAMES[step],
            -1.0,
            1.0,
            default_step(step),
        );
        step += 1;
    }
    rows[SHAPE_BASE] = toggle_row("interpolate", "Interpolate", false);
    rows[SHAPE_BASE + 1] = fraction_row("smoothing", "Smoothing", 0.0, 1.0, 0.0);
    let voicing = VoicingParams::rows();
    let mut row = 0;
    while row < VoicingParams::ROWS {
        rows[VOICING_BASE + row] = voicing[row];
        row += 1;
    }
    rows
};

impl ParamSet for ScanSynthParams {
    const NAME: &'static str = "Scan synth";

    fn descriptors() -> &'static [ParamInfo] {
        &INFO
    }

    fn get(&self, index: usize) -> Option<f32> {
        match index {
            _ if index < SHAPE_BASE => Some(self.row[index - ROW_BASE]),
            SHAPE_BASE => Some(f32::from(u8::from(self.interpolate))),
            _ if index == SHAPE_BASE + 1 => Some(self.smoothing),
            _ => self.voicing.get(index - VOICING_BASE),
        }
    }

    fn set(&mut self, index: usize, value: f32) -> bool {
        let Some(value) = cleaned_at(&INFO, index, value) else {
            return false;
        };
        match index {
            _ if index < SHAPE_BASE => self.row[index - ROW_BASE] = value,
            SHAPE_BASE => self.interpolate = value >= 0.5,
            _ if index == SHAPE_BASE + 1 => self.smoothing = value,
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

/// A synthesizer that scans a row of values as its waveform.
///
/// The row is not a spectrum. Its 32 values are the waveform itself, read
/// once per cycle of the note, so a value is a height rather than the level
/// of a partial: moving one changes the shape at one point in the cycle and
/// affects every overtone at once. That is the opposite of how the other
/// five instruments are edited, and it is the point -- a row of numbers
/// read as a waveform is a bleep machine, not an additive synth.
///
/// Read as a staircase the row has corners sharper than the sample rate can
/// carry, and what folds back is part of the sound. Nothing here is
/// band-limited: a high note is meant to come out gritty. The smoothing
/// control rounds the corners off for when it should not.
///
/// This is Windfall's answer to turning a grid of values into sound (parity
/// `inst-beepmap`). The row is data in the project: Windfall does not read
/// an image to fill it, and nothing here maps colour to frequency.
pub struct ScanSynth {
    params: ScanSynthParams,
    engine: AdditiveEngine<ScanTone>,
}

impl ScanSynth {
    /// Hands the engine the row the current parameters ask for.
    fn apply(&mut self) {
        let params = &self.params;
        // The smoother is a one-pole, so smoothing is a time: none at 0,
        // and the shortest that still rounds a corner at 1.
        let smoothing = if params.smoothing <= 0.0 {
            1.0
        } else {
            smoothing_coefficient(SMOOTHEST_MS * params.smoothing, self.engine.sample_rate())
        };
        let shape = ScanShape {
            row: params.row,
            interpolate: params.interpolate,
            smoothing,
        };
        self.engine
            .set_target(&shape, &Common::new(&params.voicing));
    }
}

engine_instrument!(ScanSynth, ScanSynthParams);
