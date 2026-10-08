//! A 2x2 matrix in stereo or mid/side coordinates, followed by channel delays.
use crate::balance::{Controls, audio, rate};
use crate::blocks::delay_line::DelayLine;
use crate::blocks::math::{flush, ms_to_samples};
use crate::blocks::tap_crossfade::TapCrossfade;
use crate::effect::Effect;
use crate::limiter::LOOKAHEAD_FADE_MS;
use crate::param::{ParamSet, param_set};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const MATRIX_MAX_DELAY_MS: f32 = 50.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum MatrixMode {
    #[default]
    Stereo,
    /// Encode M=(L+R)/2, S=(L-R)/2, apply the matrix, then decode L=M+S, R=M-S.
    MidSide,
    /// Encode to M/S then apply the matrix; output channels contain M and S.
    EncodeMidSide,
    /// Apply the matrix to input M/S channels then decode to L/R.
    DecodeMidSide,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct StereoMatrixParams {
    /// Coordinate/routing view; default stereo. Every view uses the same matrix.
    pub mode: MatrixMode,
    /// First input to first output, -2 to 2; default 1.
    pub ll: f32,
    /// Second input to first output, -2 to 2; default 0.
    pub lr: f32,
    /// First input to second output, -2 to 2; default 0.
    pub rl: f32,
    /// Second input to second output, -2 to 2; default 1.
    pub rr: f32,
    /// Left output delay, 0 to 50 ms; rounded to samples, default 0.
    pub left_delay_ms: f32,
    /// Right output delay, 0 to 50 ms; rounded to samples, default 0.
    pub right_delay_ms: f32,
}
impl Default for StereoMatrixParams {
    fn default() -> Self {
        Self {
            mode: MatrixMode::Stereo,
            ll: 1.0,
            lr: 0.0,
            rl: 0.0,
            rr: 1.0,
            left_delay_ms: 0.0,
            right_delay_ms: 0.0,
        }
    }
}
param_set!(StereoMatrixParams, "Stereo matrix", {
    choice [mode] "mode" "Routing" { MatrixMode, Stereo, [
        Stereo "stereo" "Stereo", MidSide "midSide" "Mid/side",
        EncodeMidSide "encodeMidSide" "Encode mid/side", DecodeMidSide "decodeMidSide" "Decode mid/side"
    ] }
    // Signed dimensionless coefficients: the Gain UI unit is a dB readout
    // and cannot display or type a negative multiplier.
    float [ll] "ll" "Input 1 to output 1" { None, Linear, -2.0, 2.0, 1.0 }
    float [lr] "lr" "Input 2 to output 1" { None, Linear, -2.0, 2.0, 0.0 }
    float [rl] "rl" "Input 1 to output 2" { None, Linear, -2.0, 2.0, 0.0 }
    float [rr] "rr" "Input 2 to output 2" { None, Linear, -2.0, 2.0, 1.0 }
    float [left_delay_ms] "leftDelayMs" "Left delay" { Milliseconds, Linear, 0.0, MATRIX_MAX_DELAY_MS, 0.0 }
    float [right_delay_ms] "rightDelayMs" "Right delay" { Milliseconds, Linear, 0.0, MATRIX_MAX_DELAY_MS, 0.0 }
});
impl StereoMatrixParams {
    fn delays(&self, sample_rate: f32) -> [usize; 2] {
        let p = self.sanitized();
        let rate = rate(sample_rate);
        [p.left_delay_ms, p.right_delay_ms].map(|ms| (ms * 0.001 * rate).round() as usize)
    }
    /// Shared delay only. The difference between the channels is intentional
    /// stereo processing, retained by PDC and the slot's aligned dry signal.
    pub fn latency_samples(&self, sample_rate: f32) -> usize {
        let [left, right] = self.delays(sample_rate);
        left.min(right)
    }
    /// Input history required before a live edit can use both new taps.
    pub fn delay_readiness_samples(&self, sample_rate: f32) -> usize {
        let [left, right] = self.delays(sample_rate);
        left.max(right)
    }
    fn coefficients(&self) -> [f32; 4] {
        let Self {
            ll: a,
            lr: b,
            rl: c,
            rr: d,
            ..
        } = *self;
        match self.mode {
            MatrixMode::Stereo => [a, b, c, d],
            MatrixMode::MidSide => [
                (a + b + c + d) * 0.5,
                (a - b + c - d) * 0.5,
                (a + b - c - d) * 0.5,
                (a - b - c + d) * 0.5,
            ],
            MatrixMode::EncodeMidSide => {
                [(a + b) * 0.5, (a - b) * 0.5, (c + d) * 0.5, (c - d) * 0.5]
            }
            MatrixMode::DecodeMidSide => [a + c, b + d, a - c, b - d],
        }
    }
}
pub struct StereoMatrix {
    params: StereoMatrixParams,
    sample_rate: f32,
    controls: Controls<4>,
    lines: [DelayLine; 2],
    delays: [TapCrossfade; 2],
    fade_len: u32,
    fresh: bool,
    history_samples: usize,
    delay_waiting: bool,
}
impl Default for StereoMatrix {
    fn default() -> Self {
        Self {
            params: StereoMatrixParams::default(),
            sample_rate: 48_000.0,
            controls: Controls::new([1.0, 0.0, 0.0, 1.0]),
            lines: std::array::from_fn(|_| DelayLine::default()),
            delays: std::array::from_fn(|_| TapCrossfade::new(0, 0)),
            fade_len: 240,
            fresh: true,
            history_samples: 0,
            delay_waiting: false,
        }
    }
}
impl StereoMatrix {
    pub fn max_latency_samples(sample_rate: f32) -> usize {
        (MATRIX_MAX_DELAY_MS * 0.001 * rate(sample_rate)).round() as usize
    }
    fn update(&mut self) {
        self.controls.set(self.params.coefficients());
        self.update_delays();
    }
    fn update_delays(&mut self) {
        self.delay_waiting = !self.fresh
            && self.history_samples < self.params.delay_readiness_samples(self.sample_rate);
        if self.delay_waiting {
            return;
        }
        for (fade, delay) in self
            .delays
            .iter_mut()
            .zip(self.params.delays(self.sample_rate))
        {
            fade.retarget(delay, if self.fresh { 0 } else { self.fade_len });
        }
    }
}
impl Effect for StereoMatrix {
    type Params = StereoMatrixParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.sample_rate = rate(sample_rate);
        self.lines =
            std::array::from_fn(|_| DelayLine::new(Self::max_latency_samples(self.sample_rate)));
        self.delays = std::array::from_fn(|_| {
            TapCrossfade::new(Self::max_latency_samples(self.sample_rate), 0)
        });
        self.fade_len = ms_to_samples(LOOKAHEAD_FADE_MS, self.sample_rate);
        self.controls.prepare(self.sample_rate);
        self.reset();
    }
    fn reset(&mut self) {
        for line in &mut self.lines {
            line.clear();
        }
        self.controls.reset();
        self.fresh = true;
        self.history_samples = 0;
        self.update();
    }
    fn set_params(&mut self, params: &Self::Params) {
        self.params = params.sanitized();
        self.update();
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            if self.delay_waiting {
                self.update_delays();
            }
            self.fresh = false;
            let [a, b, c, d] = self.controls.tick();
            let (il, ir) = (audio(*l), audio(*r));
            let inputs = [flush(a * il + b * ir), flush(c * il + d * ir)];
            for (side, sample) in [l, r].into_iter().enumerate() {
                let line = &mut self.lines[side];
                let tap = |delay: usize| {
                    if delay == 0 {
                        inputs[side]
                    } else {
                        line.tap(delay.min(line.max_delay()))
                    }
                };
                let out = self.delays[side].read(tap);
                *sample = flush(out);
                line.push(inputs[side]);
                self.delays[side].advance();
            }
            self.history_samples =
                (self.history_samples + 1).min(Self::max_latency_samples(self.sample_rate));
        }
    }
    fn latency_samples(&self) -> usize {
        self.params.latency_samples(self.sample_rate)
    }
    fn warm_up_samples(&self) -> usize {
        self.tail_samples()
    }
    fn delay_readiness_samples(&self) -> usize {
        self.params.delay_readiness_samples(self.sample_rate)
    }
    fn latency_transition_samples_remaining(&self) -> usize {
        if self.delay_waiting {
            self.params
                .delay_readiness_samples(self.sample_rate)
                .saturating_sub(self.history_samples)
                + self.fade_len as usize
        } else {
            self.delays
                .iter()
                .map(|fade| fade.remaining() as usize)
                .max()
                .unwrap_or(0)
        }
    }
    fn tail_samples(&self) -> usize {
        self.params.delay_readiness_samples(self.sample_rate).max(
            self.delays[0]
                .longest_delay()
                .max(self.delays[1].longest_delay()),
        )
    }
    fn gap_samples(&self) -> usize {
        self.tail_samples()
    }
}
