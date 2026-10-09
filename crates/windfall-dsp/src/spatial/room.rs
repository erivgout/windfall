//! Compact Schroeder room: four early taps, four parallel damped combs per
//! channel and two short output allpasses. No sixteen-line FDN or modulation.
use super::common::{Controls, Lowpass, Tail, audio, bounded, frames, pole, rate, tap};
use crate::blocks::delay_line::DelayLine;
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct RoomParams {
    pub size: f32,
    /// Nominal low-frequency RT60, 0.1..1.5 seconds; default 0.45.
    pub decay_s: f32,
    pub pre_delay_ms: f32,
    pub damping: f32,
    pub early_level: f32,
    pub width: f32,
    pub mix: f32,
}
impl Default for RoomParams {
    fn default() -> Self {
        Self {
            size: 0.35,
            decay_s: 0.45,
            pre_delay_ms: 2.0,
            damping: 0.6,
            early_level: 0.65,
            width: 0.8,
            mix: 0.3,
        }
    }
}
param_set!(RoomParams, "Room", {
    float [size] "size" "Size" { Fraction, Linear, 0.0, 1.0, 0.35 }
    float [decay_s] "decayS" "Decay" { Seconds, Logarithmic, 0.1, 1.5, 0.45 }
    float [pre_delay_ms] "preDelayMs" "Pre-delay" { Milliseconds, Linear, 0.0, 30.0, 2.0 }
    float [damping] "damping" "Damping" { Fraction, Linear, 0.0, 1.0, 0.6 }
    float [early_level] "earlyLevel" "Early reflections" { Fraction, Linear, 0.0, 1.0, 0.65 }
    float [width] "width" "Width" { Fraction, Linear, 0.0, 1.0, 0.8 }
    float [mix] "mix" "Mix" { Fraction, Linear, 0.0, 1.0, 0.3 }
});
pub struct Room {
    params: RoomParams,
    controls: Controls<7>,
    pre: [DelayLine; 2],
    early: [DelayLine; 2],
    combs: [[DelayLine; 4]; 2],
    damping: [[Lowpass; 4]; 2],
    diffusers: [[DelayLine; 2]; 2],
    diffuser_lengths: [[usize; 2]; 2],
    tail: Tail,
    rate: f32,
    prepared: bool,
}
impl Default for Room {
    fn default() -> Self {
        Self {
            params: RoomParams::default(),
            controls: Controls::default(),
            pre: std::array::from_fn(|_| DelayLine::default()),
            early: std::array::from_fn(|_| DelayLine::default()),
            combs: std::array::from_fn(|_| std::array::from_fn(|_| DelayLine::default())),
            damping: [[Lowpass::default(); 4]; 2],
            diffusers: std::array::from_fn(|_| std::array::from_fn(|_| DelayLine::default())),
            diffuser_lengths: [[1; 2]; 2],
            tail: Tail::default(),
            rate: 48_000.0,
            prepared: false,
        }
    }
}
impl Room {
    fn apply(&mut self) {
        let p = self.params;
        self.controls.set([
            p.size,
            p.decay_s,
            p.pre_delay_ms,
            p.damping,
            p.early_level,
            p.width,
            p.mix,
        ]);
    }
    fn clear(&mut self) {
        for line in self
            .pre
            .iter_mut()
            .chain(&mut self.early)
            .chain(self.combs.iter_mut().flatten())
            .chain(self.diffusers.iter_mut().flatten())
        {
            line.clear();
        }
        self.damping = [[Lowpass::default(); 4]; 2];
    }
}
impl Effect for Room {
    type Params = RoomParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.rate = rate(sample_rate);
        self.pre = std::array::from_fn(|_| DelayLine::new(frames(30.0, self.rate) + 2));
        self.early = std::array::from_fn(|_| DelayLine::new(frames(32.0, self.rate) + 2));
        self.combs = std::array::from_fn(|_| {
            std::array::from_fn(|_| DelayLine::new(frames(64.0, self.rate) + 2))
        });
        self.diffuser_lengths =
            [[1.71, 2.93], [1.93, 3.17]].map(|ch| ch.map(|ms| frames(ms, self.rate)));
        self.diffusers = std::array::from_fn(|ch| {
            std::array::from_fn(|i| DelayLine::new(self.diffuser_lengths[ch][i]))
        });
        self.controls.prepare(self.rate);
        self.tail.prepare(5.0, self.rate);
        self.prepared = true;
        self.reset();
    }
    fn reset(&mut self) {
        self.clear();
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
            let dry = [audio(*l), audio(*r)];
            if !self.prepared {
                [*l, *r] = dry;
                continue;
            }
            let [size, decay, pre, damping, early_level, width, mix] = self.controls.tick();
            let scale = 0.6 + size;
            let (gain, expired) = self.tail.tick(dry);
            let mut wet = [0.0; 2];
            let p = pole(1200.0 + 14_000.0 * (1.0 - damping), self.rate);
            for ch in 0..2 {
                let d = super::common::samples(pre, self.rate);
                let input = if d < 1.0 {
                    dry[ch] + d * (self.pre[ch].tap(1) - dry[ch])
                } else {
                    tap(&self.pre[ch], d)
                };
                self.pre[ch].push(dry[ch]);
                let mut reflections = 0.0;
                for (i, ms) in [3.7, 7.9, 11.3, 17.1].iter().enumerate() {
                    let time = super::common::samples((*ms + ch as f32 * 0.67) * scale, self.rate);
                    let level = [0.45, -0.31, 0.23, 0.17][i];
                    reflections += level * tap(&self.early[ch], time);
                }
                self.early[ch].push(input);
                let mut late = 0.0;
                for (i, ms) in [19.3, 23.9, 31.1, 37.7].iter().enumerate() {
                    let delay = super::common::samples((*ms + ch as f32 * 0.83) * scale, self.rate)
                        .max(1.0);
                    let echo = tap(&self.combs[ch][i], delay);
                    let filtered = self.damping[ch][i].tick(echo, p);
                    let feedback = (-6.907_755 * delay / (decay * self.rate)).exp().min(0.94);
                    self.combs[ch][i].push(bounded(input + feedback * filtered));
                    late += echo * 0.25;
                }
                for i in 0..2 {
                    let delayed = self.diffusers[ch][i].tap(self.diffuser_lengths[ch][i]);
                    let into = bounded(late - 0.55 * delayed);
                    self.diffusers[ch][i].push(into);
                    late = bounded(delayed + 0.55 * into);
                }
                wet[ch] = gain * (early_level * reflections + 0.65 * late);
            }
            if expired {
                self.clear();
            }
            let mid = 0.5 * (wet[0] + wet[1]);
            let side = 0.5 * (wet[0] - wet[1]) * width;
            *l = bounded(dry[0] + mix * (mid + side - dry[0]));
            *r = bounded(dry[1] + mix * (mid - side - dry[1]));
        }
    }
    fn tail_samples(&self) -> usize {
        self.tail.samples()
    }
    fn gap_samples(&self) -> usize {
        self.tail_samples()
    }
    fn warm_up_samples(&self) -> usize {
        frames(100.0, self.rate)
    }
    fn delay_readiness_samples(&self) -> usize {
        self.warm_up_samples()
    }
}
