//! Independently smoothed channel mutes inside an effect chain.
use crate::balance::{Controls, audio};
use crate::blocks::math::flush;
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ChannelMuteParams {
    /// Silence the left channel; default false.
    pub left: bool,
    /// Silence the right channel; default false.
    pub right: bool,
}
param_set!(ChannelMuteParams, "Channel mute", {
    toggle [left] "left" "Mute left" { false }
    toggle [right] "right" "Mute right" { false }
});
pub struct ChannelMute {
    controls: Controls<2>,
}
impl Default for ChannelMute {
    fn default() -> Self {
        Self {
            controls: Controls::new([1.0; 2]),
        }
    }
}
impl Effect for ChannelMute {
    type Params = ChannelMuteParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.controls.prepare(sample_rate);
    }
    fn reset(&mut self) {
        self.controls.reset();
    }
    fn set_params(&mut self, params: &Self::Params) {
        let p = params.sanitized();
        self.controls.set([
            if p.left { 0.0 } else { 1.0 },
            if p.right { 0.0 } else { 1.0 },
        ]);
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let [lg, rg] = self.controls.tick();
            *l = flush(audio(*l) * lg);
            *r = flush(audio(*r) * rg);
        }
    }
}
