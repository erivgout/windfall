//! The controls every additive instrument shares.
//!
//! The six instruments differ only in how they work out a spectrum. What
//! happens to that spectrum once a key goes down -- the amplitude envelope,
//! how many notes sound at once, how hard they are played and where the
//! result sits in the stereo field -- is the same for all of them, so it
//! lives in one struct that each of them embeds under `voicing`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::blocks::math::clean;
use crate::param::{ParamInfo, ParamScale, ParamUnit};
use crate::synth::EnvelopeParams;

use super::engine::MAX_POLYPHONY;
use super::rows::{float_row, fraction_row, int_row};

/// The controls every additive instrument shares.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct VoicingParams {
    /// Shapes the volume of each note. Default: attack 5 ms, decay 400 ms,
    /// sustain 0.7, release 300 ms.
    pub envelope: EnvelopeParams,
    /// Most notes sounding at once. One more takes over the oldest. 1 to
    /// 16, default 8. An additive voice costs one sine per partial per
    /// sample, so this is the control that decides what the instrument
    /// costs.
    pub polyphony: u8,
    /// How much playing softly lowers the volume. At 0 every note is at
    /// full level; at 1 volume follows velocity directly. Default 0.7.
    pub velocity: f32,
    /// Output level as a linear gain. 0 to 2, default 0.2. Partial levels
    /// add up, so this is where the room for a chord is made.
    pub gain: f32,
    /// Output position from left (-1) to right (1). Default 0.
    pub pan: f32,
}

impl Default for VoicingParams {
    fn default() -> Self {
        Self {
            envelope: EnvelopeParams {
                attack_ms: 5.0,
                decay_ms: 400.0,
                sustain: 0.7,
                release_ms: 300.0,
            },
            polyphony: 8,
            velocity: 0.7,
            gain: 0.2,
            pan: 0.0,
        }
    }
}

impl VoicingParams {
    /// How many rows [`VoicingParams::rows`] adds to a table.
    pub(super) const ROWS: usize = 8;

    /// The table rows for these controls, to be copied into an
    /// instrument's own table.
    pub(super) const fn rows() -> [ParamInfo; Self::ROWS] {
        [
            float_row(
                "voicing.envelope.attackMs",
                "Attack",
                ParamUnit::Milliseconds,
                ParamScale::Linear,
                0.0,
                10_000.0,
                5.0,
            ),
            float_row(
                "voicing.envelope.decayMs",
                "Decay",
                ParamUnit::Milliseconds,
                ParamScale::Logarithmic,
                1.0,
                10_000.0,
                400.0,
            ),
            fraction_row("voicing.envelope.sustain", "Sustain", 0.0, 1.0, 0.7),
            float_row(
                "voicing.envelope.releaseMs",
                "Release",
                ParamUnit::Milliseconds,
                ParamScale::Logarithmic,
                1.0,
                10_000.0,
                300.0,
            ),
            int_row("voicing.polyphony", "Polyphony", 1, MAX_POLYPHONY as i32, 8),
            fraction_row("voicing.velocity", "Volume velocity", 0.0, 1.0, 0.7),
            float_row(
                "voicing.gain",
                "Volume",
                ParamUnit::Gain,
                ParamScale::Linear,
                0.0,
                2.0,
                0.2,
            ),
            float_row(
                "voicing.pan",
                "Pan",
                ParamUnit::Pan,
                ParamScale::Linear,
                -1.0,
                1.0,
                0.0,
            ),
        ]
    }

    /// The value of the row at `index` of [`VoicingParams::rows`].
    pub(super) fn get(&self, index: usize) -> Option<f32> {
        Some(match index {
            0 => self.envelope.attack_ms,
            1 => self.envelope.decay_ms,
            2 => self.envelope.sustain,
            3 => self.envelope.release_ms,
            4 => f32::from(self.polyphony),
            5 => self.velocity,
            6 => self.gain,
            7 => self.pan,
            _ => return None,
        })
    }

    /// Sets the row at `index`, forcing the value into its range.
    pub(super) fn set(&mut self, index: usize, value: f32) -> bool {
        let rows = Self::rows();
        let Some(row) = rows.get(index) else {
            return false;
        };
        let value = clean(value, row.min, row.max, row.default);
        match index {
            0 => self.envelope.attack_ms = value,
            1 => self.envelope.decay_ms = value,
            2 => self.envelope.sustain = value,
            3 => self.envelope.release_ms = value,
            4 => self.polyphony = value.round() as u8,
            5 => self.velocity = value,
            6 => self.gain = value,
            7 => self.pan = value,
            _ => return false,
        }
        true
    }
}
