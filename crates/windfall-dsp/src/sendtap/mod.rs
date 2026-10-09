//! A levelled stereo copy at this point in Windfall's effect chain.
//! The host supplies and routes the tap buffers; dry audio is never changed.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::effect::Effect;
use crate::param::{ParamSet, param_set};

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct SendTapParams {
    /// Tap gain, 0 to 1; default 0 (silent).
    pub level: f32,
}

param_set!(SendTapParams, "Send Tap", {
    float [level] "level" "Level" { Gain, Linear, 0.0, 1.0, 0.0 }
});

/// Stateless unity passthrough with a separate, host-owned send output.
#[derive(Debug, Default)]
pub struct SendTap {
    params: SendTapParams,
}

impl SendTap {
    /// Copies the block just passed to `process` into the destination pair,
    /// scaled by the current level. Call before downstream processing changes
    /// the source block. No audio is retained internally.
    ///
    /// All four slices must have the same length. Destinations are overwritten,
    /// not accumulated; the host handles mixing and destination routing.
    /// Level changes apply immediately, without a gain ramp.
    pub fn send(&self, left: &[f32], right: &[f32], send_left: &mut [f32], send_right: &mut [f32]) {
        let level = self.params.level;
        if level == 0.0 {
            // Exact silence even when the source contains nonfinite samples.
            send_left.fill(0.0);
            send_right.fill(0.0);
        } else if level == 1.0 {
            send_left.copy_from_slice(left);
            send_right.copy_from_slice(right);
        } else {
            for (output, input) in send_left.iter_mut().zip(left) {
                *output = *input * level;
            }
            for (output, input) in send_right.iter_mut().zip(right) {
                *output = *input * level;
            }
        }
    }
}

impl Effect for SendTap {
    type Params = SendTapParams;

    fn prepare(&mut self, _sample_rate: f32, _max_block: usize) {
        self.reset();
    }

    fn reset(&mut self) {}

    fn set_params(&mut self, params: &Self::Params) {
        self.params = params.sanitized();
    }

    fn process(&mut self, _left: &mut [f32], _right: &mut [f32]) {}
}

#[cfg(test)]
mod tests;
