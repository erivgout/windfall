//! Twelve staggered lossless lattice allpasses, with a colored feedback path.
use super::common::{Controls, Lowpass, Tail, advance, audio, bounded, pole, rate, sine};
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct VintagePhaserParams {
    pub min_hz: f32,
    pub max_hz: f32,
    pub rate_hz: f32,
    pub feedback: f32,
    pub color_hz: f32,
    pub stereo_phase: f32,
    pub mix: f32,
}
impl Default for VintagePhaserParams {
    fn default() -> Self {
        Self {
            min_hz: 150.0,
            max_hz: 2400.0,
            rate_hz: 0.3,
            feedback: 0.45,
            color_hz: 2500.0,
            stereo_phase: 0.15,
            mix: 0.5,
        }
    }
}
param_set!(VintagePhaserParams, "Vintage Phaser", {
    float [min_hz] "minHz" "Sweep minimum" { Hertz, Logarithmic, 20.0, 10_000.0, 150.0 }
    float [max_hz] "maxHz" "Sweep maximum" { Hertz, Logarithmic, 20.0, 10_000.0, 2400.0 }
    float [rate_hz] "rateHz" "Rate" { Hertz, Linear, 0.0, 5.0, 0.3 }
    float [feedback] "feedback" "Feedback" { Fraction, Linear, -0.8, 0.8, 0.45 }
    float [color_hz] "colorHz" "Feedback color" { Hertz, Logarithmic, 200.0, 12_000.0, 2500.0 }
    float [stereo_phase] "stereoPhase" "Stereo phase (cycles)" { Fraction, Linear, 0.0, 1.0, 0.15 }
    float [mix] "mix" "Mix" { Fraction, Linear, 0.0, 1.0, 0.5 }
});
pub struct VintagePhaser {
    params: VintagePhaserParams,
    controls: Controls<7>,
    states: [[f32; 12]; 2],
    back: [f32; 2],
    color: [Lowpass; 2],
    phase: f64,
    rate: f32,
    tail: Tail,
    prepared: bool,
}
impl Default for VintagePhaser {
    fn default() -> Self {
        Self {
            params: VintagePhaserParams::default(),
            controls: Controls::default(),
            states: [[0.0; 12]; 2],
            back: [0.0; 2],
            color: [Lowpass::default(); 2],
            phase: 0.0,
            rate: 48_000.0,
            tail: Tail::default(),
            prepared: false,
        }
    }
}
impl VintagePhaser {
    fn apply(&mut self) {
        let p = self.params;
        self.controls.set([
            p.min_hz.ln(),
            p.max_hz.ln(),
            p.rate_hz,
            p.feedback,
            p.color_hz,
            p.stereo_phase,
            p.mix,
        ]);
    }
    fn clear(&mut self) {
        self.states = [[0.0; 12]; 2];
        self.back = [0.0; 2];
        self.color = [Lowpass::default(); 2];
    }
}
impl Effect for VintagePhaser {
    type Params = VintagePhaserParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.rate = rate(sample_rate);
        self.controls.prepare(self.rate);
        self.tail.prepare(3.0, self.rate);
        self.prepared = true;
        self.reset();
    }
    fn reset(&mut self) {
        self.clear();
        self.phase = 0.0;
        self.controls.reset();
        self.tail.reset();
        self.apply();
    }
    fn set_params(&mut self, p: &Self::Params) {
        self.params = p.sanitized();
        self.apply();
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let input = [audio(*l), audio(*r)];
            if !self.prepared {
                [*l, *r] = input;
                continue;
            }
            let [lo, hi, hz, feedback, color, stereo, mix] = self.controls.tick();
            let (gain, expired) = self.tail.tick(input);
            let mut out = [0.0; 2];
            for ch in 0..2 {
                let motion = 0.5 + 0.5 * sine(self.phase + stereo as f64 * ch as f64);
                let center = (lo.min(hi) + (hi - lo).abs() * motion).exp();
                let mut wet = bounded(input[ch] + feedback * self.back[ch]);
                for (i, state) in self.states[ch].iter_mut().enumerate() {
                    let frequency = (center * (0.72 + 0.055 * i as f32)).min(0.45 * self.rate);
                    let g = (std::f32::consts::PI * frequency / self.rate).tan();
                    let a = ((g - 1.0) / (g + 1.0)).clamp(-0.9995, 0.9995);
                    let b = (1.0 - a * a).sqrt();
                    // Orthogonal scattering remains passive while sweeping.
                    let next = a * wet + b * *state;
                    *state = bounded(b * wet - a * *state);
                    wet = bounded(next);
                }
                self.back[ch] = self.color[ch].tick(wet, pole(color, self.rate));
                out[ch] = bounded(input[ch] + mix * (wet * gain - input[ch]));
            }
            if expired {
                self.clear();
            }
            advance(&mut self.phase, hz, self.rate);
            [*l, *r] = out;
        }
    }
    fn tail_samples(&self) -> usize {
        self.tail.samples()
    }
    fn gap_samples(&self) -> usize {
        self.tail_samples()
    }
}
