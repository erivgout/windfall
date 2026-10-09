//! Three telescoping crossover bands with independent feedback delays/pans.
//! This is the compact three-band circuit, not the sixteen-band parity target.
use super::common::{Controls, Lowpass, Tail, audio, bounded, frames, pole, rate, tap};
use crate::blocks::delay_line::DelayLine;
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct BandDelayParams {
    pub low_hz: f32,
    pub high_hz: f32,
    pub time_ms: [f32; 3],
    pub level: [f32; 3],
    pub pan: [f32; 3],
    pub feedback: [f32; 3],
    pub mix: f32,
}
impl Default for BandDelayParams {
    fn default() -> Self {
        Self {
            low_hz: 300.0,
            high_hz: 3000.0,
            time_ms: [150.0, 250.0, 375.0],
            level: [1.0; 3],
            pan: [-0.3, 0.0, 0.3],
            feedback: [0.2, 0.3, 0.4],
            mix: 0.3,
        }
    }
}
param_set!(BandDelayParams, "Band Delay", {
    float [low_hz] "lowHz" "Low crossover" { Hertz, Logarithmic, 80.0, 1000.0, 300.0 }
    float [high_hz] "highHz" "High crossover" { Hertz, Logarithmic, 1000.0, 10_000.0, 3000.0 }
    float [time_ms[0]] "timeMs.0" "Low delay" { Milliseconds, Logarithmic, 1.0, 1000.0, 150.0 }
    float [time_ms[1]] "timeMs.1" "Mid delay" { Milliseconds, Logarithmic, 1.0, 1000.0, 250.0 }
    float [time_ms[2]] "timeMs.2" "High delay" { Milliseconds, Logarithmic, 1.0, 1000.0, 375.0 }
    float [level[0]] "level.0" "Low level" { Gain, Linear, 0.0, 1.0, 1.0 }
    float [level[1]] "level.1" "Mid level" { Gain, Linear, 0.0, 1.0, 1.0 }
    float [level[2]] "level.2" "High level" { Gain, Linear, 0.0, 1.0, 1.0 }
    float [pan[0]] "pan.0" "Low pan" { Pan, Linear, -1.0, 1.0, -0.3 }
    float [pan[1]] "pan.1" "Mid pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    float [pan[2]] "pan.2" "High pan" { Pan, Linear, -1.0, 1.0, 0.3 }
    float [feedback[0]] "feedback.0" "Low feedback" { Fraction, Linear, -0.85, 0.85, 0.2 }
    float [feedback[1]] "feedback.1" "Mid feedback" { Fraction, Linear, -0.85, 0.85, 0.3 }
    float [feedback[2]] "feedback.2" "High feedback" { Fraction, Linear, -0.85, 0.85, 0.4 }
    float [mix] "mix" "Mix" { Fraction, Linear, 0.0, 1.0, 0.3 }
});
pub struct BandDelay {
    params: BandDelayParams,
    controls: Controls<15>,
    crossovers: [[[Lowpass; 2]; 2]; 2],
    lines: [[DelayLine; 2]; 3],
    tone: [[Lowpass; 2]; 3],
    rate: f32,
    tail: Tail,
    prepared: bool,
}
impl Default for BandDelay {
    fn default() -> Self {
        Self {
            params: BandDelayParams::default(),
            controls: Controls::default(),
            crossovers: [[[Lowpass::default(); 2]; 2]; 2],
            lines: std::array::from_fn(|_| std::array::from_fn(|_| DelayLine::default())),
            tone: [[Lowpass::default(); 2]; 3],
            rate: 48_000.0,
            tail: Tail::default(),
            prepared: false,
        }
    }
}
impl BandDelay {
    fn apply(&mut self) {
        let p = self.params;
        self.controls.set([
            p.low_hz.ln(),
            p.high_hz.ln(),
            p.time_ms[0],
            p.time_ms[1],
            p.time_ms[2],
            p.level[0],
            p.level[1],
            p.level[2],
            p.pan[0],
            p.pan[1],
            p.pan[2],
            p.feedback[0],
            p.feedback[1],
            p.feedback[2],
            p.mix,
        ]);
    }
    fn clear(&mut self) {
        for line in self.lines.iter_mut().flatten() {
            line.clear();
        }
        self.crossovers = [[[Lowpass::default(); 2]; 2]; 2];
        self.tone = [[Lowpass::default(); 2]; 3];
    }
}
impl Effect for BandDelay {
    type Params = BandDelayParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.rate = rate(sample_rate);
        self.lines = std::array::from_fn(|_| {
            std::array::from_fn(|_| DelayLine::new(frames(1000.0, self.rate) + 2))
        });
        self.controls.prepare(self.rate);
        self.tail.prepare(90.0, self.rate);
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
            let c = self.controls.tick();
            let (gain, expired) = self.tail.tick(dry);
            let poles = [pole(c[0].exp(), self.rate), pole(c[1].exp(), self.rate)];
            let tone = pole(7000.0, self.rate);
            let mut wet = [0.0; 2];
            for ch in 0..2 {
                let mut cumulative = [0.0; 2];
                for crossover in 0..2 {
                    let mut x = dry[ch];
                    for section in &mut self.crossovers[ch][crossover] {
                        x = section.tick(x, poles[crossover]);
                    }
                    cumulative[crossover] = x;
                }
                // These sum exactly to the input before independent delays.
                let bands = [
                    cumulative[0],
                    cumulative[1] - cumulative[0],
                    dry[ch] - cumulative[1],
                ];
                for band in 0..3 {
                    let echo = tap(
                        &self.lines[band][ch],
                        super::common::samples(c[2 + band], self.rate),
                    );
                    let back = self.tone[band][ch].tick(echo, tone);
                    self.lines[band][ch].push(bounded(bands[band] + c[11 + band] * back));
                    let pan = c[8 + band];
                    let balance = if ch == 0 {
                        (1.0 - pan).min(1.0)
                    } else {
                        (1.0 + pan).min(1.0)
                    };
                    wet[ch] += echo * c[5 + band] * balance;
                }
            }
            if expired {
                self.clear();
            }
            *l = bounded(dry[0] + c[14] * (gain * wet[0] - dry[0]));
            *r = bounded(dry[1] + c[14] * (gain * wet[1] - dry[1]));
        }
    }
    fn tail_samples(&self) -> usize {
        self.tail.samples()
    }
    fn gap_samples(&self) -> usize {
        self.tail_samples()
    }
    fn warm_up_samples(&self) -> usize {
        frames(1000.0, self.rate)
    }
    fn delay_readiness_samples(&self) -> usize {
        self.warm_up_samples()
    }
}
