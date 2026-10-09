//! Bounded note-event transforms. These processors do not process audio.
mod color_map;
mod key_map;
mod key_split;
mod level_scale;
mod step_grid;

pub use color_map::{ColorMap, ColorMapParams};
pub use key_map::{KeyMap, KeyMapParams};
pub use key_split::{KeySplit, KeySplitParams};
pub use level_scale::{LevelScale, LevelScaleParams};
pub use step_grid::{GridStep, StepGrid, StepGridParams};

use crate::param::ParamSet;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A note-on candidate: MIDI key 0..127, velocity 0..1, channel/color 0..15.
/// Identity, release, pan, and expression remain the host's responsibility.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct MappedNote {
    pub key: u8,
    pub velocity: f32,
    pub channel: u8,
    pub color: u8,
}

impl MappedNote {
    pub fn sanitized(self) -> Self {
        Self {
            key: self.key.min(127),
            velocity: velocity(self.velocity),
            channel: self.channel.min(15),
            color: self.color.min(15),
        }
    }
}

fn velocity(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// At most one output per input; None means drop. No heap or IO is used.
/// Implementations sanitize inputs and apply parameter changes immediately.
pub trait NoteTransform: Copy + Default {
    type Params: ParamSet;
    fn set_params(&mut self, params: &Self::Params);
    fn map(&self, note: MappedNote) -> Option<MappedNote>;

    /// Transform only the first min(len, N) notes, preserving accepted order.
    /// Returns the valid output prefix length. The remaining slots are unused.
    /// Every input in a batch sees the same clock position.
    fn transform<const N: usize>(&self, notes: &mut [MappedNote; N], len: usize) -> usize {
        let mut written = 0;
        for read in 0..len.min(N) {
            if let Some(note) = self.map(notes[read]) {
                notes[written] = note;
                written += 1;
            }
        }
        written
    }
}

#[cfg(test)]
mod tests;
