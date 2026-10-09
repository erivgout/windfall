//! Windfall's fixed eight-knob control surface with unchanged stereo audio.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::effect::Effect;
use crate::param::{ParamSet, param_set};

/// Eight independent normalized controls, in a stable order.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct ControlSurfaceParams {
    /// Knob 1, 0 to 1; default 0.5.
    pub knob1: f32,
    /// Knob 2, 0 to 1; default 0.5.
    pub knob2: f32,
    /// Knob 3, 0 to 1; default 0.5.
    pub knob3: f32,
    /// Knob 4, 0 to 1; default 0.5.
    pub knob4: f32,
    /// Knob 5, 0 to 1; default 0.5.
    pub knob5: f32,
    /// Knob 6, 0 to 1; default 0.5.
    pub knob6: f32,
    /// Knob 7, 0 to 1; default 0.5.
    pub knob7: f32,
    /// Knob 8, 0 to 1; default 0.5.
    pub knob8: f32,
}

impl Default for ControlSurfaceParams {
    fn default() -> Self {
        Self {
            knob1: 0.5,
            knob2: 0.5,
            knob3: 0.5,
            knob4: 0.5,
            knob5: 0.5,
            knob6: 0.5,
            knob7: 0.5,
            knob8: 0.5,
        }
    }
}

param_set!(ControlSurfaceParams, "Control Surface", {
    float [knob1] "knob1" "Knob 1" { Fraction, Linear, 0.0, 1.0, 0.5 }
    float [knob2] "knob2" "Knob 2" { Fraction, Linear, 0.0, 1.0, 0.5 }
    float [knob3] "knob3" "Knob 3" { Fraction, Linear, 0.0, 1.0, 0.5 }
    float [knob4] "knob4" "Knob 4" { Fraction, Linear, 0.0, 1.0, 0.5 }
    float [knob5] "knob5" "Knob 5" { Fraction, Linear, 0.0, 1.0, 0.5 }
    float [knob6] "knob6" "Knob 6" { Fraction, Linear, 0.0, 1.0, 0.5 }
    float [knob7] "knob7" "Knob 7" { Fraction, Linear, 0.0, 1.0, 0.5 }
    float [knob8] "knob8" "Knob 8" { Fraction, Linear, 0.0, 1.0, 0.5 }
});

/// Stores controls without modifying audio, adding latency or producing a tail.
/// Knob edits take effect immediately; they do not modulate the audio here.
#[derive(Debug, Default)]
pub struct ControlSurface {
    params: ControlSurfaceParams,
}

impl ControlSurface {
    /// Copies sanitized values in knob1 through knob8 order, without allocating.
    /// The host owns any publication or routing of this snapshot.
    pub fn readout(&self) -> [f32; 8] {
        let p = self.params;
        [
            p.knob1, p.knob2, p.knob3, p.knob4, p.knob5, p.knob6, p.knob7, p.knob8,
        ]
    }
}

impl Effect for ControlSurface {
    type Params = ControlSurfaceParams;

    fn prepare(&mut self, _sample_rate: f32, _max_block: usize) {
        self.reset();
    }

    fn reset(&mut self) {
        // There is no audio history; preserve the current controls.
    }

    fn set_params(&mut self, params: &Self::Params) {
        self.params = params.sanitized();
    }

    fn process(&mut self, _left: &mut [f32], _right: &mut [f32]) {}
}

#[cfg(test)]
mod tests;
