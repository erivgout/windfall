use crate::balance::{audio, rate};
use crate::blocks::biquad::{Biquad, BiquadCoeffs, CoeffRamp};
use crate::blocks::math::ms_to_samples;

#[derive(Default)]
struct Pair {
    low: [Biquad; 2],
    high: [Biquad; 2],
}
impl Pair {
    fn tick(&mut self, input: f64, low: &BiquadCoeffs, high: &BiquadCoeffs) -> [f64; 2] {
        let l = self.low[0].tick(low, input);
        let l = self.low[1].tick(low, l);
        let h = self.high[0].tick(high, input);
        let h = self.high[1].tick(high, h);
        // Per-frame flushing preserves partition invariance.
        for filter in self.low.iter_mut().chain(&mut self.high) {
            filter.flush();
        }
        [l, h]
    }
}

#[derive(Default)]
struct Channel {
    lower: Pair,
    upper: Pair,
    compensation: Pair,
}
impl Channel {
    fn tick(&mut self, input: f32, c: &[BiquadCoeffs; 4]) -> [f32; 3] {
        let [l, h] = self.lower.tick(f64::from(audio(input)), &c[0], &c[1]);
        let [ll, lh] = self.compensation.tick(l, &c[2], &c[3]);
        let [mid, high] = self.upper.tick(h, &c[2], &c[3]);
        [(ll + lh) as f32, mid as f32, high as f32]
    }
}

/// LR4: low=L1*(L2+H2), mid=H1*L2, high=H1*H2.
/// Sum=(L1+H1)*(L2+H2), a cascade of unit-magnitude all-passes.
pub(super) struct Crossover {
    channels: [Channel; 2],
    coefficients: [CoeffRamp; 4],
    frequencies: [f32; 2],
    sample_rate: f32,
    fresh: bool,
}
impl Default for Crossover {
    fn default() -> Self {
        let mut split = Self {
            channels: std::array::from_fn(|_| Channel::default()),
            coefficients: [CoeffRamp::new(BiquadCoeffs::IDENTITY); 4],
            frequencies: [200.0, 2_500.0],
            sample_rate: 48_000.0,
            fresh: true,
        };
        split.set(200.0, 2_500.0);
        split
    }
}
impl Crossover {
    pub(super) fn prepare(&mut self, sample_rate: f32) {
        // Cookbook coefficients require a nonempty sub-Nyquist interval.
        self.sample_rate = rate(sample_rate).max(8.0);
        self.reset();
    }
    pub(super) fn reset(&mut self) {
        self.channels = std::array::from_fn(|_| Channel::default());
        self.fresh = true;
        self.set(self.frequencies[0], self.frequencies[1]);
    }
    pub(super) fn set(&mut self, low: f32, high: f32) {
        if !self.fresh && self.frequencies == [low, high] {
            return;
        }
        self.frequencies = [low, high];
        let low = low.clamp(1.0, self.sample_rate * 0.35);
        let high = high.max(low * 1.25).clamp(1.25, self.sample_rate * 0.45);
        let q = std::f32::consts::FRAC_1_SQRT_2;
        let targets = [
            BiquadCoeffs::low_pass(low, q, self.sample_rate),
            BiquadCoeffs::high_pass(low, q, self.sample_rate),
            BiquadCoeffs::low_pass(high, q, self.sample_rate),
            BiquadCoeffs::high_pass(high, q, self.sample_rate),
        ];
        let samples = if self.fresh {
            0
        } else {
            ms_to_samples(20.0, self.sample_rate)
        };
        for (ramp, target) in self.coefficients.iter_mut().zip(targets) {
            ramp.set_target(target, samples);
        }
    }
    pub(super) fn tick(&mut self, input: [f32; 2]) -> [[f32; 2]; 3] {
        self.fresh = false;
        let c = std::array::from_fn(|i| *self.coefficients[i].tick());
        let left = self.channels[0].tick(input[0], &c);
        let right = self.channels[1].tick(input[1], &c);
        std::array::from_fn(|i| [left[i], right[i]])
    }
    pub(super) fn tail_samples(&self) -> usize {
        let low = self.frequencies[0].clamp(1.0, self.sample_rate * 0.35);
        (64.0 * self.sample_rate / low).ceil() as usize
    }
}
