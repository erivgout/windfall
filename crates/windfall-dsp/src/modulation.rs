//! Shared frame-clock controls for the original modulation processors.

use crate::blocks::math::{clean, ms_to_samples};
use crate::blocks::smooth::LinearRamp;

mod chorus;
mod flanger;
mod phaser;

pub use chorus::{Chorus, ChorusParams};
pub use flanger::{Flanger, FlangerParams};
pub use phaser::{Phaser, PhaserParams};

const RAMP_MS: f32 = 10.0;

fn rate(value: f32) -> f32 {
    clean(value, 1.0, 384_000.0, 48_000.0)
}

fn signal(value: f32) -> f32 {
    crate::blocks::math::flush(clean(value, -1_000.0, 1_000.0, 0.0))
}

struct Controls<const N: usize> {
    ramps: [LinearRamp; N],
    frames: u32,
    fresh: bool,
}

impl<const N: usize> Controls<N> {
    fn new() -> Self {
        Self {
            ramps: [LinearRamp::new(0.0); N],
            frames: 480,
            fresh: true,
        }
    }

    fn prepare(&mut self, sample_rate: f32) {
        self.frames = ms_to_samples(RAMP_MS, sample_rate);
        self.fresh = true;
    }

    fn reset(&mut self) {
        self.fresh = true;
    }

    fn set(&mut self, values: [f32; N]) {
        for (ramp, value) in self.ramps.iter_mut().zip(values) {
            ramp.set_target(value, if self.fresh { 0 } else { self.frames });
        }
    }

    fn tick(&mut self) -> [f32; N] {
        self.fresh = false;
        std::array::from_fn(|i| self.ramps[i].tick())
    }
}

fn advance(phase: &mut f64, hz: f32, sample_rate: f32) {
    *phase = (*phase + f64::from(hz) / f64::from(sample_rate)).fract();
}

fn sine(phase: f64) -> f32 {
    (std::f64::consts::TAU * phase).sin() as f32
}
