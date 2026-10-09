//! Original, bounded live-audio performance processors. See the integration seam
//! for clock, trigger, latency and intentionally partial parity contracts.

mod buffer;
mod params;
pub use params::*;

use crate::Effect;
use buffer::{Controls, Frame, JumpFade, Ring, blend, finite, loop_length, sample_rate, tempo};

struct History {
    ring: Ring,
    fade: JumpFade,
    rate: f32,
    bpm: f32,
    length: usize,
}
impl Default for History {
    fn default() -> Self {
        Self {
            ring: Ring::default(),
            fade: JumpFade::default(),
            rate: 48_000.0,
            bpm: 120.0,
            length: 1,
        }
    }
}
impl History {
    fn prepare(&mut self, rate: f32) {
        self.rate = sample_rate(rate);
        self.ring.prepare(self.rate);
        self.fade.prepare(self.rate);
    }
    fn reset(&mut self) {
        self.ring.clear();
        self.fade.reset();
    }
    fn sync(&mut self, beats: f32, split: bool) -> bool {
        let capacity = if split {
            self.ring.data.len() / 2
        } else {
            self.ring.data.len()
        };
        let next = loop_length(self.rate, self.bpm, beats, capacity);
        let changed = next != self.length;
        self.length = next;
        changed
    }
}

/// Sixteen-step volume curve and deterministic probabilistic history retrigger.
#[derive(Default)]
pub struct VolumeGate {
    history: History,
    controls: Controls<VolumeGateParams>,
    phase: usize,
    last_step: Option<usize>,
    delay: usize,
    random: u32,
    feedback: Frame,
}
impl VolumeGate {
    pub fn new() -> Self {
        Self::default()
    }
    fn sync(&mut self) {
        if self.history.sync(self.controls.target.loop_beats, false) {
            self.phase %= self.history.length;
            self.last_step = None;
        }
    }
}
impl Effect for VolumeGate {
    type Params = VolumeGateParams;
    fn prepare(&mut self, rate: f32, _max_block: usize) {
        self.history.prepare(rate);
        self.controls.prepare(self.history.rate);
        self.sync();
        self.reset();
    }
    fn reset(&mut self) {
        self.history.reset();
        self.controls.reset();
        self.phase = 0;
        self.last_step = None;
        self.delay = 0;
        self.random = 0x6d2b_79f5;
        self.feedback = [0.0; 2];
    }
    fn set_params(&mut self, params: &Self::Params) {
        self.controls.set(params);
        self.sync();
    }
    fn set_tempo(&mut self, bpm: f32) {
        self.history.bpm = tempo(bpm);
        self.sync();
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let p = self.controls.tick();
            let dry = [finite(*l), finite(*r)];
            let step = self.phase * 16 / self.history.length;
            if self.last_step != Some(step) {
                // Fixed seed and exactly one random draw per visited step: block invariant.
                if self.random == 0 {
                    self.random = 0x6d2b_79f5;
                }
                self.random ^= self.random << 13;
                self.random ^= self.random >> 17;
                self.random ^= self.random << 5;
                let retrigger = (self.random as f64 / 4_294_967_296.0) < p.retrigger_chance as f64;
                let next = if retrigger {
                    (self.history.length * p.retrigger_steps as usize / 16)
                        .max(1)
                        .min(self.history.ring.filled)
                } else {
                    0
                };
                if next != self.delay {
                    self.history.fade.jump();
                }
                self.delay = next;
                self.last_step = Some(step);
            }
            let replay = if self.delay > 0 {
                self.history
                    .ring
                    .read(self.history.ring.start(self.delay), self.delay, 0.0)
            } else {
                dry
            };
            // Apply the curve AFTER the head transition: a zero step is exactly silent.
            let wet = self
                .history
                .fade
                .tick(replay)
                .map(|v| finite(v * p.steps[step]));
            self.history.ring.push(std::array::from_fn(|c| {
                finite(dry[c] + self.feedback[c] * p.feedback)
            }));
            self.feedback = wet;
            let out = blend(dry, wet, p.mix);
            *l = out[0];
            *r = out[1];
            self.phase = (self.phase + 1) % self.history.length;
        }
    }
    fn latency_samples(&self) -> usize {
        0
    }
    fn tail_samples(&self) -> usize {
        if self.controls.target.feedback > 0.0 {
            self.history.ring.data.len().saturating_mul(280)
        } else {
            self.history.ring.data.len()
        }
    }
    fn gap_samples(&self) -> usize {
        self.history.ring.data.len()
    }
}

/// Rising trigger captures preceding audio; holding trigger preserves that slice.
#[derive(Default)]
pub struct TimeTransport {
    history: History,
    controls: Controls<TimeTransportParams>,
    active: bool,
    mode: TransportMode,
    start: usize,
    captured: usize,
    position: f64,
}
impl TimeTransport {
    pub fn new() -> Self {
        Self::default()
    }
}
impl Effect for TimeTransport {
    type Params = TimeTransportParams;
    fn prepare(&mut self, rate: f32, _max_block: usize) {
        self.history.prepare(rate);
        self.controls.prepare(self.history.rate);
        self.history.sync(self.controls.target.loop_beats, false);
        self.reset();
    }
    fn reset(&mut self) {
        self.history.reset();
        self.controls.reset();
        self.active = false;
        self.start = 0;
        self.captured = 1;
        self.position = 0.0;
        self.mode = self.controls.target.mode;
    }
    fn set_params(&mut self, params: &Self::Params) {
        self.controls.set(params);
        self.history.sync(self.controls.target.loop_beats, false);
    }
    fn set_tempo(&mut self, bpm: f32) {
        self.history.bpm = tempo(bpm);
        self.history.sync(self.controls.target.loop_beats, false);
        // An already captured slice stays immutable; the next trigger uses this tempo.
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let p = self.controls.tick();
            let dry = [finite(*l), finite(*r)];
            if p.trigger != self.active {
                self.history.fade.jump();
                if p.trigger {
                    self.captured = self.history.length.min(self.history.ring.filled).max(1);
                    self.start = self.history.ring.start(self.captured);
                    self.position = if p.mode == TransportMode::Repeat {
                        0.0
                    } else {
                        (self.captured - 1) as f64
                    };
                }
                self.active = p.trigger;
                self.mode = p.mode;
            } else if self.active && p.mode != self.mode {
                self.history.fade.jump();
                self.mode = p.mode;
                self.position = if p.mode == TransportMode::Repeat {
                    0.0
                } else {
                    (self.captured - 1) as f64
                };
            }
            let mut wrapped = false;
            let next = if self.active {
                let out = self
                    .history
                    .ring
                    .read(self.start, self.captured, self.position);
                let delta = match p.mode {
                    TransportMode::Hold => 0.0,
                    TransportMode::Reverse => -(p.rate as f64),
                    TransportMode::Repeat => p.rate as f64,
                };
                let next = self.position + delta;
                wrapped = next < 0.0 || next >= self.captured as f64;
                self.position = next.rem_euclid(self.captured as f64);
                out
            } else {
                self.history.ring.push(dry);
                dry
            };
            let wet = self.history.fade.tick(next);
            if wrapped {
                self.history.fade.jump();
            }
            let out = blend(dry, wet, p.mix);
            *l = out[0];
            *r = out[1];
        }
    }
    fn latency_samples(&self) -> usize {
        0
    }
    fn warm_up_samples(&self) -> usize {
        self.history.length
    }
    fn tail_samples(&self) -> usize {
        if self.controls.target.trigger {
            usize::MAX
        } else {
            self.history.ring.data.len()
        }
    }
    fn gap_samples(&self) -> usize {
        self.tail_samples()
    }
}

/// Absolute position playback of recent live audio, optionally frozen as a record.
#[derive(Default)]
pub struct Scratch {
    history: History,
    controls: Controls<ScratchParams>,
    frozen: bool,
    start: usize,
    captured: usize,
}
impl Scratch {
    pub fn new() -> Self {
        Self::default()
    }
}
impl Effect for Scratch {
    type Params = ScratchParams;
    fn prepare(&mut self, rate: f32, _max_block: usize) {
        self.history.prepare(rate);
        self.controls.prepare(self.history.rate);
        self.history.sync(self.controls.target.loop_beats, false);
        self.reset();
    }
    fn reset(&mut self) {
        self.history.reset();
        self.controls.reset();
        self.frozen = false;
        self.start = 0;
        self.captured = 1;
    }
    fn set_params(&mut self, params: &Self::Params) {
        self.controls.set(params);
        if self.history.sync(self.controls.target.loop_beats, false) {
            self.history.fade.jump();
        }
    }
    fn set_tempo(&mut self, bpm: f32) {
        self.history.bpm = tempo(bpm);
        if self.history.sync(self.controls.target.loop_beats, false) {
            self.history.fade.jump();
        }
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let p = self.controls.tick();
            let dry = [finite(*l), finite(*r)];
            if p.freeze != self.frozen {
                self.history.fade.jump();
                self.frozen = p.freeze;
                if self.frozen {
                    self.captured = self.history.length.min(self.history.ring.filled).max(1);
                    self.start = self.history.ring.start(self.captured);
                }
            }
            if !self.frozen {
                self.history.ring.push(dry);
                self.captured = self.history.length.min(self.history.ring.filled).max(1);
                self.start = self.history.ring.start(self.captured);
            }
            let wet = self.history.ring.read(
                self.start,
                self.captured,
                p.position as f64 * (self.captured - 1) as f64,
            );
            let wet = self.history.fade.tick(wet);
            let out = blend(dry, wet, p.mix);
            *l = out[0];
            *r = out[1];
        }
    }
    fn latency_samples(&self) -> usize {
        0
    }
    fn warm_up_samples(&self) -> usize {
        self.history.length
    }
    fn tail_samples(&self) -> usize {
        if self.controls.target.freeze {
            usize::MAX
        } else {
            self.history.ring.data.len()
        }
    }
    fn gap_samples(&self) -> usize {
        self.tail_samples()
    }
}

/// Twelve different treatments of the preceding slice. Both internal mix paths
/// are delayed by one loop; captured and recording regions never overlap.
#[derive(Default)]
pub struct PerformanceRack {
    history: History,
    controls: Controls<PerformanceRackParams>,
    phase: usize,
    start: usize,
    model: PerformanceModel,
    feedback: Frame,
    highpass_memory: [f64; 2],
    lowpass_memory: [f64; 2],
    last_read: Option<f64>,
}
impl PerformanceRack {
    pub fn new() -> Self {
        Self::default()
    }
    fn sync(&mut self) {
        if self.history.sync(self.controls.target.loop_beats, true) {
            self.phase = 0;
            self.history.fade.jump();
        }
    }
    fn treatment(&mut self, p: PerformanceRackParams, dry: Frame) -> Frame {
        let n = self.history.length;
        let t = self.phase as f64;
        let progress = t / n as f64;
        let a = p.amount as f64;
        let sub = (n / (2 + (a * 14.0).round() as usize)).max(1);
        let position = match p.model {
            PerformanceModel::Stutter => t.rem_euclid(sub as f64),
            PerformanceModel::Reverse => (n - 1) as f64 - t,
            PerformanceModel::TapeStop => t - a * t * t / (2.0 * n as f64),
            PerformanceModel::HalfSpeed => t * 0.5,
            PerformanceModel::DoubleSpeed => t * 2.0,
            // Repeat the LAST quarter beat, rather than stutter's first short slice.
            PerformanceModel::BeatRepeat => {
                let beat = loop_length(self.history.rate, self.history.bpm, 0.25, n);
                (n - beat) as f64 + t.rem_euclid(beat as f64)
            }
            PerformanceModel::PitchJump => t * (p.pitch_semitones as f64 * a / 12.0).exp2(),
            _ => t,
        };
        let wrapped = position.rem_euclid(n as f64);
        if self.phase > 0
            && self.last_read.is_some_and(|last| wrapped < last)
            && matches!(
                p.model,
                PerformanceModel::Stutter
                    | PerformanceModel::DoubleSpeed
                    | PerformanceModel::BeatRepeat
                    | PerformanceModel::PitchJump
            )
        {
            self.history.fade.jump();
        }
        self.last_read = Some(wrapped);
        let mut wet = self.history.ring.read(self.start, n, position);
        match p.model {
            PerformanceModel::TelephoneFilter => {
                let hp = 1.0
                    - (-std::f64::consts::TAU * 300.0_f64.min(self.history.rate as f64 * 0.1)
                        / self.history.rate as f64)
                        .exp();
                let lp = 1.0
                    - (-std::f64::consts::TAU * 3000.0_f64.min(self.history.rate as f64 * 0.45)
                        / self.history.rate as f64)
                        .exp();
                for (c, sample) in wet.iter_mut().enumerate() {
                    self.highpass_memory[c] += hp * (*sample as f64 - self.highpass_memory[c]);
                    let high = *sample as f64 - self.highpass_memory[c];
                    self.lowpass_memory[c] += lp * (high - self.lowpass_memory[c]);
                    self.highpass_memory[c] = crate::blocks::math::flush64(self.highpass_memory[c]);
                    self.lowpass_memory[c] = crate::blocks::math::flush64(self.lowpass_memory[c]);
                    *sample = finite(self.lowpass_memory[c] as f32);
                }
                wet = blend(dry, wet, p.amount);
            }
            PerformanceModel::Distortion => {
                let drive = 1.0 + 19.0 * p.amount;
                wet = wet.map(|v| (v * drive).tanh() / drive.tanh());
                wet = blend(dry, wet, p.amount);
            }
            PerformanceModel::PanSpin => {
                let pan = (std::f64::consts::TAU * progress).sin() as f32 * p.amount;
                wet[0] *= (1.0 - pan).sqrt();
                wet[1] *= (1.0 + pan).sqrt();
            }
            PerformanceModel::Fade => wet = wet.map(|v| v * (1.0 - p.amount * progress as f32)),
            PerformanceModel::SilenceGate if self.phase >= n.div_ceil(2) => {
                wet = wet.map(|v| v * (1.0 - p.amount));
            }
            _ => {}
        }
        wet.map(finite)
    }
}
impl Effect for PerformanceRack {
    type Params = PerformanceRackParams;
    fn prepare(&mut self, rate: f32, _max_block: usize) {
        self.history.prepare(rate);
        self.controls.prepare(self.history.rate);
        self.sync();
        self.reset();
    }
    fn reset(&mut self) {
        self.history.reset();
        self.controls.reset();
        self.phase = 0;
        self.start = 0;
        self.model = self.controls.target.model;
        self.feedback = [0.0; 2];
        self.highpass_memory = [0.0; 2];
        self.lowpass_memory = [0.0; 2];
        self.last_read = None;
    }
    fn set_params(&mut self, params: &Self::Params) {
        self.controls.set(params);
        self.sync();
    }
    fn set_tempo(&mut self, bpm: f32) {
        self.history.bpm = tempo(bpm);
        self.sync();
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let p = self.controls.tick();
            let input = [finite(*l), finite(*r)];
            if self.phase == 0 {
                self.start = self.history.ring.start(self.history.length);
                self.last_read = None;
                // Only remapping modes need a transition at the slice seam.
                if matches!(
                    p.model,
                    PerformanceModel::Stutter
                        | PerformanceModel::Reverse
                        | PerformanceModel::TapeStop
                        | PerformanceModel::HalfSpeed
                        | PerformanceModel::DoubleSpeed
                        | PerformanceModel::BeatRepeat
                        | PerformanceModel::PitchJump
                ) {
                    self.history.fade.jump();
                }
            }
            if self.model != p.model {
                self.history.fade.jump();
                self.model = p.model;
                self.highpass_memory = [0.0; 2];
                self.lowpass_memory = [0.0; 2];
                self.last_read = None;
            }
            let dry = self
                .history
                .ring
                .read(self.start, self.history.length, self.phase as f64);
            let wet = self.treatment(p, dry);
            let wet = self.history.fade.tick(wet);
            self.history.ring.push(std::array::from_fn(|c| {
                finite(input[c] + self.feedback[c] * p.feedback)
            }));
            self.feedback = wet;
            let out = blend(dry, wet, p.mix);
            *l = out[0];
            *r = out[1];
            self.phase = (self.phase + 1) % self.history.length;
        }
    }
    fn latency_samples(&self) -> usize {
        self.history.length
    }
    fn tail_samples(&self) -> usize {
        if self.controls.target.feedback > 0.0 {
            usize::MAX
        } else {
            self.history
                .length
                .saturating_mul(2)
                .saturating_add((self.history.rate * 0.1) as usize)
        }
    }
    fn gap_samples(&self) -> usize {
        if self.controls.target.feedback > 0.0 {
            usize::MAX
        } else {
            self.history.length.saturating_mul(2)
        }
    }
}

#[cfg(test)]
mod tests;
