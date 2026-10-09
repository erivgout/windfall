use super::{MappedNote, NoteTransform};
use crate::param::{ParamSet, param_set};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Velocity-only scaling: percent 0..200, floor/ceiling 0..1.
/// The effective ceiling is max(floor, ceiling) when bounds are reversed.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct LevelScaleParams {
    pub percent: f32,
    pub floor: f32,
    pub ceiling: f32,
}
impl Default for LevelScaleParams {
    fn default() -> Self {
        Self {
            percent: 100.0,
            floor: 0.0,
            ceiling: 1.0,
        }
    }
}
param_set!(LevelScaleParams, "Level scale", {
    float [percent] "percent" "Percent" { None, Linear, 0.0, 200.0, 100.0 }
    float [floor] "floor" "Velocity floor" { Fraction, Linear, 0.0, 1.0, 0.0 }
    float [ceiling] "ceiling" "Velocity ceiling" { Fraction, Linear, 0.0, 1.0, 1.0 }
});

#[derive(Debug, Clone, Copy, Default)]
pub struct LevelScale {
    params: LevelScaleParams,
}
impl NoteTransform for LevelScale {
    type Params = LevelScaleParams;
    fn set_params(&mut self, params: &Self::Params) {
        self.params = params.sanitized();
    }
    fn map(&self, note: MappedNote) -> Option<MappedNote> {
        let mut note = note.sanitized();
        note.velocity = (note.velocity * (self.params.percent / 100.0)).clamp(
            self.params.floor,
            self.params.ceiling.max(self.params.floor),
        );
        Some(note)
    }
}
