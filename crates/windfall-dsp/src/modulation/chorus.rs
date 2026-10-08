//! Three independent detuned delay taps per channel; no feedback or crossfeed.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{Controls, advance, rate, signal, sine};
use crate::blocks::delay_line::DelayLine;
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};

const MAX_DELAY_MS: f32 = 40.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct ChorusParams {
    /// Minimum tap delay, 1..=30 ms; default 12.
    pub delay_ms: f32,
    /// Sweep above the minimum, 0..=10 ms; default 4. Zero freezes delay.
    pub depth_ms: f32,
    /// Independent tap rates, 0..=5 Hz; defaults 0.6, 0.8, 1.1.
    pub rate1_hz: f32,
    pub rate2_hz: f32,
    pub rate3_hz: f32,
    /// Right LFO offset in cycles, 0..=1; default 0.25 (90 degrees).
    pub stereo_phase: f32,
    /// Wet share, 0..=1; default 0.5. Zero is exact dry, one is wet only.
    pub mix: f32,
}

impl Default for ChorusParams {
    fn default() -> Self {
        Self {
            delay_ms: 12.0,
            depth_ms: 4.0,
            rate1_hz: 0.6,
            rate2_hz: 0.8,
            rate3_hz: 1.1,
            stereo_phase: 0.25,
            mix: 0.5,
        }
    }
}

param_set!(ChorusParams, "Chorus", {
    float [delay_ms] "delayMs" "Delay" { Milliseconds, Linear, 1.0, 30.0, 12.0 }
    float [depth_ms] "depthMs" "Depth" { Milliseconds, Linear, 0.0, 10.0, 4.0 }
    float [rate1_hz] "rate1Hz" "Voice 1 rate" { Hertz, Linear, 0.0, 5.0, 0.6 }
    float [rate2_hz] "rate2Hz" "Voice 2 rate" { Hertz, Linear, 0.0, 5.0, 0.8 }
    float [rate3_hz] "rate3Hz" "Voice 3 rate" { Hertz, Linear, 0.0, 5.0, 1.1 }
    float [stereo_phase] "stereoPhase" "Stereo phase (cycles)" { Fraction, Linear, 0.0, 1.0, 0.25 }
    float [mix] "mix" "Mix" { Fraction, Linear, 0.0, 1.0, 0.5 }
});

pub struct Chorus {
    params: ChorusParams,
    sample_rate: f32,
    controls: Controls<7>,
    lines: [DelayLine; 2],
    phases: [f64; 3],
    prepared: bool,
}

impl Default for Chorus {
    fn default() -> Self {
        Self {
            params: ChorusParams::default(),
            sample_rate: 48_000.0,
            controls: Controls::new(),
            lines: std::array::from_fn(|_| DelayLine::default()),
            phases: [0.0, 1.0 / 3.0, 2.0 / 3.0],
            prepared: false,
        }
    }
}

impl Chorus {
    fn update(&mut self) {
        let p = self.params;
        self.controls.set([
            p.delay_ms,
            p.depth_ms,
            p.rate1_hz,
            p.rate2_hz,
            p.rate3_hz,
            p.stereo_phase,
            p.mix,
        ]);
    }

    fn horizon(&self) -> usize {
        // Cubic interpolation also sees two older guard frames. Reserve the
        // maximum across every live setting/ramp, not only today's target.
        (f64::from(MAX_DELAY_MS) * 0.001 * f64::from(self.sample_rate))
            .ceil()
            .max(2.0) as usize
            + 2
    }
}

impl Effect for Chorus {
    type Params = ChorusParams;

    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.sample_rate = rate(sample_rate);
        let horizon = self.horizon();
        self.lines = std::array::from_fn(|_| DelayLine::new(horizon));
        self.controls.prepare(self.sample_rate);
        self.prepared = true;
        self.reset();
    }

    fn reset(&mut self) {
        for line in &mut self.lines {
            line.clear();
        }
        self.phases = [0.0, 1.0 / 3.0, 2.0 / 3.0];
        self.controls.reset();
        self.update();
    }

    fn set_params(&mut self, params: &ChorusParams) {
        self.params = params.sanitized();
        self.update();
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (left, right) in left.iter_mut().zip(right) {
            let input = [signal(*left), signal(*right)];
            if !self.prepared {
                [*left, *right] = input;
                continue;
            }
            let [delay, depth, r1, r2, r3, stereo, mix] = self.controls.tick();
            let mut output = [0.0; 2];
            for channel in 0..2 {
                let mut wet = 0.0;
                for phase in self.phases {
                    let lfo = sine(phase + f64::from(stereo) * channel as f64);
                    let tap = ((f64::from(delay) + f64::from(depth) * (0.5 + 0.5 * f64::from(lfo)))
                        * 0.001
                        * f64::from(self.sample_rate)) as f32;
                    let tap = tap.max(2.0);
                    wet += self.lines[channel].tap_cubic(tap) / 3.0;
                }
                self.lines[channel].push(input[channel]);
                output[channel] = input[channel] + mix * (wet - input[channel]);
            }
            for (phase, hz) in self.phases.iter_mut().zip([r1, r2, r3]) {
                advance(phase, hz, self.sample_rate);
            }
            [*left, *right] = output;
        }
    }

    fn warm_up_samples(&self) -> usize {
        self.horizon()
    }
    fn delay_readiness_samples(&self) -> usize {
        self.horizon()
    }
    fn tail_samples(&self) -> usize {
        self.horizon()
    }
    fn gap_samples(&self) -> usize {
        self.horizon()
    }
}
