//! Fixed-cost history and delay transitions, private to the delay family.
use crate::blocks::smooth::LinearRamp;

pub(crate) fn audio(x: f32) -> f32 {
    if x.is_finite() {
        x.clamp(-1000.0, 1000.0)
    } else {
        0.0
    }
}
pub(crate) fn bounded(x: f32) -> f32 {
    if x.is_finite() {
        x.clamp(-64.0, 64.0)
    } else {
        0.0
    }
}
pub(crate) fn rate(x: f32) -> f32 {
    crate::blocks::math::clean(x, 1.0, 384_000.0, 48_000.0)
}
pub(crate) fn balance(x: [f32; 2], pan: f32) -> [f32; 2] {
    [x[0] * (1.0 - pan.max(0.0)), x[1] * (1.0 + pan.min(0.0))]
}
#[derive(Default)]
pub(crate) struct History {
    data: Vec<f32>,
    write: usize,
    valid: usize,
}
impl History {
    pub fn prepare(&mut self, maximum: usize) {
        // Replace the owner: a lower-rate reprepare retains no old capacity.
        self.data = vec![0.0; maximum + 1];
        self.reset();
    }
    pub fn reset(&mut self) {
        self.write = 0;
        self.valid = 0;
    }
    pub fn valid(&self) -> usize {
        self.valid
    }
    pub fn bytes(&self) -> usize {
        self.data.capacity() * std::mem::size_of::<f32>()
    }
    pub fn read(&self, delay: usize, current: f32) -> f32 {
        if delay == 0 {
            return current;
        }
        if delay > self.valid || self.data.is_empty() {
            return 0.0;
        }
        let index = (self.write + self.data.len() - delay) % self.data.len();
        self.data[index]
    }
    pub fn push(&mut self, value: f32) {
        if self.data.is_empty() {
            return;
        }
        self.data[self.write] = value;
        self.write += 1;
        if self.write == self.data.len() {
            self.write = 0;
        }
        self.valid = (self.valid + 1).min(self.data.len() - 1);
    }
}
#[derive(Clone, Copy)]
pub(crate) struct Tap {
    from: usize,
    to: usize,
    wanted: usize,
    remaining: u32,
    frames: u32,
}
impl Default for Tap {
    fn default() -> Self {
        Self {
            from: 0,
            to: 0,
            wanted: 0,
            remaining: 0,
            frames: 1,
        }
    }
}
impl Tap {
    pub fn set(&mut self, delay: usize, frames: u32, fresh: bool) {
        self.wanted = delay;
        self.frames = frames.max(1);
        if fresh {
            self.from = delay;
            self.to = delay;
            self.remaining = 0;
        }
    }
    pub fn read(&mut self, history: &History, current: f32) -> f32 {
        // Live changes retain the audible source until the new tap exists.
        if self.remaining == 0 && self.from != self.wanted && history.valid() >= self.wanted {
            self.to = self.wanted;
            self.remaining = self.frames;
        }
        let old = history.read(self.from, current);
        if self.remaining == 0 {
            return old;
        }
        let weight = 1.0 - self.remaining as f32 / self.frames as f32;
        let value = old + (history.read(self.to, current) - old) * weight;
        self.remaining -= 1;
        if self.remaining == 0 {
            self.from = self.to;
        }
        value
    }
    pub fn longest(&self) -> usize {
        self.from.max(self.to).max(self.wanted)
    }
    pub fn transition(&self, valid: usize) -> usize {
        if self.from == self.wanted && self.remaining == 0 {
            0
        } else {
            self.wanted.saturating_sub(valid) + self.remaining as usize + 2 * self.frames as usize
        }
    }
}
pub(crate) struct Ramps<const N: usize> {
    values: [LinearRamp; N],
    pub fresh: bool,
    frames: u32,
}
impl<const N: usize> Default for Ramps<N> {
    fn default() -> Self {
        Self {
            values: [LinearRamp::new(0.0); N],
            fresh: true,
            frames: 1,
        }
    }
}
impl<const N: usize> Ramps<N> {
    pub fn prepare(&mut self, rate: f32) {
        self.frames = (rate * 0.01).round().max(1.0) as u32;
        self.fresh = true;
    }
    pub fn reset(&mut self) {
        self.fresh = true;
    }
    pub fn restart_group(&mut self, range: std::ops::Range<usize>) {
        for ramp in &mut self.values[range] {
            ramp.snap(ramp.value());
        }
    }
    pub fn set(&mut self, values: [f32; N]) {
        for (r, v) in self.values.iter_mut().zip(values) {
            r.set_target(v, if self.fresh { 0 } else { self.frames });
        }
    }
    pub fn tick(&mut self) -> [f32; N] {
        self.fresh = false;
        std::array::from_fn(|i| self.values[i].tick())
    }
    pub fn max(&self, i: usize) -> f32 {
        self.values[i].value().max(self.values[i].target())
    }
    pub fn abs_max(&self, i: usize) -> f32 {
        self.values[i]
            .value()
            .abs()
            .max(self.values[i].target().abs())
    }
}
