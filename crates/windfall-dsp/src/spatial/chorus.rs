//! Four mellow voices and eight wide, detuned voices, with wet-path tone.
use super::common::{
    Controls, Lowpass, Tail, advance, audio, bounded, frames, pole, rate, samples, sine,
};
use crate::blocks::delay_line::DelayLine;
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct VintageChorusParams {
    pub delay_ms: f32,
    pub depth_ms: f32,
    pub rate_hz: f32,
    pub spread: f32,
    pub tone_hz: f32,
    pub mix: f32,
}
impl Default for VintageChorusParams {
    fn default() -> Self {
        Self {
            delay_ms: 8.0,
            depth_ms: 3.0,
            rate_hz: 0.55,
            spread: 0.6,
            tone_hz: 4500.0,
            mix: 0.5,
        }
    }
}
param_set!(VintageChorusParams, "Vintage Chorus", {
    float [delay_ms] "delayMs" "Delay" { Milliseconds, Linear, 1.0, 25.0, 8.0 }
    float [depth_ms] "depthMs" "Depth" { Milliseconds, Linear, 0.0, 12.0, 3.0 }
    float [rate_hz] "rateHz" "Rate" { Hertz, Linear, 0.0, 5.0, 0.55 }
    float [spread] "spread" "Spread" { Fraction, Linear, 0.0, 1.0, 0.6 }
    float [tone_hz] "toneHz" "Tone" { Hertz, Logarithmic, 500.0, 20_000.0, 4500.0 }
    float [mix] "mix" "Mix" { Fraction, Linear, 0.0, 1.0, 0.5 }
});

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct HyperChorusParams {
    pub delay_ms: f32,
    pub depth_ms: f32,
    pub rate_hz: f32,
    pub spread: f32,
    pub tone_hz: f32,
    pub mix: f32,
}
impl Default for HyperChorusParams {
    fn default() -> Self {
        Self {
            delay_ms: 16.0,
            depth_ms: 7.0,
            rate_hz: 0.8,
            spread: 1.0,
            tone_hz: 16_000.0,
            mix: 0.65,
        }
    }
}
param_set!(HyperChorusParams, "Hyper Chorus", {
    float [delay_ms] "delayMs" "Delay" { Milliseconds, Linear, 1.0, 25.0, 16.0 }
    float [depth_ms] "depthMs" "Depth" { Milliseconds, Linear, 0.0, 12.0, 7.0 }
    float [rate_hz] "rateHz" "Rate" { Hertz, Linear, 0.0, 5.0, 0.8 }
    float [spread] "spread" "Spread" { Fraction, Linear, 0.0, 1.0, 1.0 }
    float [tone_hz] "toneHz" "Tone" { Hertz, Logarithmic, 500.0, 20_000.0, 16_000.0 }
    float [mix] "mix" "Mix" { Fraction, Linear, 0.0, 1.0, 0.65 }
});

struct Voices<const N: usize> {
    lines: [DelayLine; 2],
    phases: [f64; N],
    tone: [Lowpass; 2],
    controls: Controls<6>,
    tail: Tail,
    rate: f32,
    prepared: bool,
}
impl<const N: usize> Default for Voices<N> {
    fn default() -> Self {
        Self {
            lines: std::array::from_fn(|_| DelayLine::default()),
            phases: std::array::from_fn(|i| i as f64 / N as f64),
            tone: [Lowpass::default(); 2],
            controls: Controls::default(),
            tail: Tail::default(),
            rate: 48_000.0,
            prepared: false,
        }
    }
}
impl<const N: usize> Voices<N> {
    fn prepare(&mut self, sample_rate: f32) {
        self.rate = rate(sample_rate);
        self.lines = std::array::from_fn(|_| DelayLine::new(frames(64.0, self.rate) + 4));
        self.controls.prepare(self.rate);
        self.tail.prepare(0.3, self.rate);
        self.prepared = true;
    }
    fn reset(&mut self) {
        self.clear();
        self.phases = std::array::from_fn(|i| i as f64 / N as f64);
        self.controls.reset();
        self.tail.reset();
    }
    fn clear(&mut self) {
        for line in &mut self.lines {
            line.clear();
        }
        for tone in &mut self.tone {
            tone.clear();
        }
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let input = [audio(*l), audio(*r)];
            if !self.prepared {
                [*l, *r] = input;
                continue;
            }
            let [delay, depth, hz, spread, tone, mix] = self.controls.tick();
            let (gain, expired) = self.tail.tick(input);
            let mut wet = [0.0; 2];
            for (i, phase) in self.phases.iter_mut().enumerate() {
                let ratio = 0.67 + 0.173 * i as f32;
                for (ch, out) in wet.iter_mut().enumerate() {
                    let offset = spread as f64 * (0.17 + 0.037 * i as f64) * ch as f64;
                    let time = delay * (1.0 + 0.04 * i as f32)
                        + depth * (0.5 + 0.5 * sine(*phase + offset));
                    let samples = samples(time, self.rate)
                        .clamp(2.0, self.lines[ch].max_delay() as f32 - 2.0);
                    let own = self.lines[ch].tap_cubic(samples);
                    let other = self.lines[1 - ch].tap_cubic(samples);
                    // Eight-voice circuit distributes alternating voices across
                    // both source channels; four-voice circuit retains separation.
                    let cross = if N >= 6 {
                        spread * (0.15 + 0.2 * (i % 2) as f32)
                    } else {
                        0.0
                    };
                    *out += (own + cross * (other - own)) / N as f32;
                }
                advance(phase, hz * ratio, self.rate);
            }
            let p = pole(tone, self.rate);
            for ch in 0..2 {
                self.lines[ch].push(input[ch]);
                wet[ch] = self.tone[ch].tick(wet[ch], p) * gain;
            }
            if expired {
                self.clear();
            }
            *l = bounded(input[0] + mix * (wet[0] - input[0]));
            *r = bounded(input[1] + mix * (wet[1] - input[1]));
        }
    }
}

macro_rules! chorus {
    ($effect:ident, $params:ident, $voices:literal) => {
        pub struct $effect {
            params: $params,
            voices: Voices<$voices>,
        }
        impl Default for $effect {
            fn default() -> Self {
                Self {
                    params: $params::default(),
                    voices: Voices::default(),
                }
            }
        }
        impl $effect {
            fn apply(&mut self) {
                let p = self.params;
                self.voices.controls.set([
                    p.delay_ms, p.depth_ms, p.rate_hz, p.spread, p.tone_hz, p.mix,
                ]);
            }
        }
        impl Effect for $effect {
            type Params = $params;
            fn prepare(&mut self, rate: f32, _max_block: usize) {
                self.voices.prepare(rate);
                self.reset();
            }
            fn reset(&mut self) {
                self.voices.reset();
                self.apply();
            }
            fn set_params(&mut self, p: &$params) {
                self.params = p.sanitized();
                self.apply();
            }
            fn process(&mut self, l: &mut [f32], r: &mut [f32]) {
                self.voices.process(l, r);
            }
            fn tail_samples(&self) -> usize {
                self.voices.tail.samples()
            }
            fn gap_samples(&self) -> usize {
                frames(64.0, self.voices.rate) + 2
            }
            fn warm_up_samples(&self) -> usize {
                self.gap_samples()
            }
            fn delay_readiness_samples(&self) -> usize {
                self.gap_samples()
            }
        }
    };
}
chorus!(VintageChorus, VintageChorusParams, 4);
chorus!(HyperChorus, HyperChorusParams, 8);
