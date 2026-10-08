//! Six lossless lattice allpasses, mixed with dry to create three notches.
//! The lattice remains energy preserving as its coefficient moves. A bounded
//! signed, one-frame feedback loop adds resonance. See the explicit idle-tail
//! policy in MODULATION-EFFECTS.md.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{Controls, advance, rate, signal, sine};
use crate::blocks::math::{clean, flush, ms_to_samples};
use crate::blocks::smooth::LinearRamp;
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};

const STAGES: usize = 6;
const IDLE_SECONDS: f32 = 4.0;
const TAIL_FADE_MS: f32 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct PhaserParams {
    /// Sweep endpoints, each 20..=10000 Hz; defaults 200 and 2000. If
    /// crossed, the smaller endpoint is used as the lower frequency.
    pub min_hz: f32,
    pub max_hz: f32,
    /// Log-frequency sweep rate, 0..=5 Hz; default 0.3. Zero freezes phase.
    pub rate_hz: f32,
    /// Right LFO offset in cycles, 0..=1; default 0.25.
    pub stereo_phase: f32,
    /// Signed one-frame feedback, -0.85..=0.85; default 0.35.
    pub feedback: f32,
    /// Wet share, 0..=1; default 0.5 (deep notches without feedback).
    pub mix: f32,
}

impl Default for PhaserParams {
    fn default() -> Self {
        Self {
            min_hz: 200.0,
            max_hz: 2000.0,
            rate_hz: 0.3,
            stereo_phase: 0.25,
            feedback: 0.35,
            mix: 0.5,
        }
    }
}

param_set!(PhaserParams, "Phaser", {
    float [min_hz] "minHz" "Sweep minimum" { Hertz, Logarithmic, 20.0, 10_000.0, 200.0 }
    float [max_hz] "maxHz" "Sweep maximum" { Hertz, Logarithmic, 20.0, 10_000.0, 2000.0 }
    float [rate_hz] "rateHz" "Rate" { Hertz, Linear, 0.0, 5.0, 0.3 }
    float [stereo_phase] "stereoPhase" "Stereo phase (cycles)" { Fraction, Linear, 0.0, 1.0, 0.25 }
    float [feedback] "feedback" "Feedback" { Fraction, Linear, -0.85, 0.85, 0.35 }
    float [mix] "mix" "Mix" { Fraction, Linear, 0.0, 1.0, 0.5 }
});

pub struct Phaser {
    params: PhaserParams,
    sample_rate: f32,
    controls: Controls<6>,
    state: [[f32; STAGES]; 2],
    feedback: [f32; 2],
    phase: f64,
    idle: [usize; 2],
    tail_gain: [LinearRamp; 2],
    idle_limit: usize,
    fade_length: usize,
    prepared: bool,
}

impl Default for Phaser {
    fn default() -> Self {
        Self {
            params: PhaserParams::default(),
            sample_rate: 48_000.0,
            controls: Controls::new(),
            state: [[0.0; STAGES]; 2],
            feedback: [0.0; 2],
            phase: 0.0,
            idle: [0; 2],
            tail_gain: [LinearRamp::new(1.0); 2],
            idle_limit: 192_000,
            fade_length: 480,
            prepared: false,
        }
    }
}

impl Phaser {
    fn update(&mut self) {
        let p = self.params;
        self.controls.set([
            p.min_hz.ln(),
            p.max_hz.ln(),
            p.rate_hz,
            p.stereo_phase,
            p.feedback,
            p.mix,
        ]);
    }
    fn idle_frames(&self) -> usize {
        self.idle_limit
    }
    fn fade_frames(&self) -> usize {
        self.fade_length
    }
}

impl Effect for Phaser {
    type Params = PhaserParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.sample_rate = rate(sample_rate);
        self.idle_limit = (self.sample_rate * IDLE_SECONDS).ceil() as usize;
        self.fade_length = ms_to_samples(TAIL_FADE_MS, self.sample_rate) as usize;
        self.controls.prepare(self.sample_rate);
        self.prepared = true;
        self.reset();
    }
    fn reset(&mut self) {
        self.state = [[0.0; STAGES]; 2];
        self.feedback = [0.0; 2];
        self.phase = 0.0;
        self.idle = [0; 2];
        self.tail_gain = [LinearRamp::new(1.0); 2];
        self.controls.reset();
        self.update();
    }
    fn set_params(&mut self, params: &PhaserParams) {
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
            let [min, max, hz, stereo, feed, mix] = self.controls.tick();
            let (low, high) = (min.min(max), min.max(max));
            let idle_frames = self.idle_frames();
            let fade_frames = self.fade_frames();
            let mut output = [0.0; 2];
            for channel in 0..2 {
                if input[channel].abs() >= 1e-20 {
                    self.idle[channel] = 0;
                    // Reentry during the idle fade retains live filter memory.
                    // Recover from its current audible gain; repeated input
                    // or parameter writes must not restart this frame clock.
                    self.tail_gain[channel].set_target(1.0, fade_frames as u32);
                } else {
                    self.idle[channel] = (self.idle[channel] + 1).min(idle_frames + fade_frames);
                    if self.idle[channel] > idle_frames {
                        self.tail_gain[channel].snap(
                            1.0 - (self.idle[channel] - idle_frames) as f32 / fade_frames as f32,
                        );
                    }
                }
                let tail_gain = self.tail_gain[channel].tick();
                let amount = 0.5 + 0.5 * sine(self.phase + f64::from(stereo) * channel as f64);
                let frequency = (low + (high - low) * amount)
                    .exp()
                    .min(0.45 * self.sample_rate);
                let g = (std::f32::consts::PI * frequency / self.sample_rate).tan();
                let a = ((g - 1.0) / (g + 1.0)).clamp(-0.9995, 0.9995);
                let b = (1.0 - a * a).sqrt();
                let mut wet = input[channel] + feed * self.feedback[channel];
                for state in &mut self.state[channel] {
                    let next = a * wet + b * (*state);
                    // The finite guard handles direct hostile input/roundoff,
                    // far above ordinary audio. It never allocates or retries.
                    *state = flush(clean(b * wet - a * (*state), -1e6, 1e6, 0.0));
                    wet = clean(next, -1e6, 1e6, 0.0);
                }
                self.feedback[channel] = flush(wet);
                output[channel] = input[channel] + mix * (wet * tail_gain - input[channel]);
                if self.idle[channel] == idle_frames + fade_frames {
                    self.state[channel] = [0.0; STAGES];
                    self.feedback[channel] = 0.0;
                }
            }
            advance(&mut self.phase, hz, self.sample_rate);
            [*left, *right] = output;
        }
    }
    fn tail_samples(&self) -> usize {
        self.idle_frames() + self.fade_frames()
    }
    fn gap_samples(&self) -> usize {
        self.tail_samples()
    }
}
