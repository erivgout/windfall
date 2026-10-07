//! A low-frequency oscillator for modulation.

use std::f32::consts::TAU;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::noise::Rng;

/// The shape of one LFO cycle. Every shape swings between -1 and 1.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum LfoShape {
    #[default]
    Sine,
    /// Starts at zero and rises.
    Triangle,
    /// Rises from -1 to 1, then drops back.
    Saw,
    /// 1 for the first half of the cycle, -1 for the second.
    Square,
    /// A new random level every cycle, held until the next.
    Random,
}

/// A free-running modulation source.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lfo {
    phase: f32,
    held: f32,
    seed: u32,
    rng: Rng,
}

impl Lfo {
    /// An LFO at the start of its cycle. `seed` picks the sequence of the
    /// random shape.
    pub fn new(seed: u32) -> Self {
        let mut rng = Rng::new(seed);
        let held = rng.bipolar();
        Self {
            phase: 0.0,
            held,
            seed,
            rng,
        }
    }

    /// Returns to the start of the cycle and of the random sequence.
    pub fn reset(&mut self) {
        *self = Self::new(self.seed);
    }

    /// Returns the value at the current position, then moves on by
    /// `increment` cycles (rate in Hz divided by calls per second).
    #[inline]
    pub fn tick(&mut self, shape: LfoShape, increment: f32) -> f32 {
        let phase = self.phase;
        let value = match shape {
            LfoShape::Sine => (TAU * phase).sin(),
            LfoShape::Triangle => {
                let shifted = phase + 0.75;
                4.0 * (shifted - shifted.floor() - 0.5).abs() - 1.0
            }
            LfoShape::Saw => 2.0 * phase - 1.0,
            LfoShape::Square => {
                if phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            LfoShape::Random => self.held,
        };
        self.phase += increment.clamp(0.0, 1.0);
        if self.phase >= 1.0 {
            self.phase -= 1.0;
            self.held = self.rng.bipolar();
        }
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHAPES: [LfoShape; 5] = [
        LfoShape::Sine,
        LfoShape::Triangle,
        LfoShape::Saw,
        LfoShape::Square,
        LfoShape::Random,
    ];

    #[test]
    fn every_shape_stays_between_minus_one_and_one() {
        for shape in SHAPES {
            let mut lfo = Lfo::new(1);
            for _ in 0..10_000 {
                let value = lfo.tick(shape, 0.0137);
                assert!((-1.0..=1.0).contains(&value), "{shape:?} gave {value}");
            }
        }
    }

    #[test]
    fn one_cycle_takes_one_over_the_increment_steps() {
        let mut lfo = Lfo::new(1);
        let values: Vec<f32> = (0..200).map(|_| lfo.tick(LfoShape::Sine, 0.01)).collect();
        assert!(values[0].abs() < 1e-6);
        assert!((values[25] - 1.0).abs() < 1e-4);
        assert!((values[75] + 1.0).abs() < 1e-4);
        assert!((values[100] - values[0]).abs() < 1e-3);
    }

    #[test]
    fn shapes_hit_their_landmarks() {
        let sample = |shape, steps: usize| {
            let mut lfo = Lfo::new(1);
            let mut value = 0.0;
            for _ in 0..=steps {
                value = lfo.tick(shape, 0.01);
            }
            value
        };
        assert!(sample(LfoShape::Triangle, 0).abs() < 1e-5);
        assert!((sample(LfoShape::Triangle, 25) - 1.0).abs() < 1e-4);
        assert!((sample(LfoShape::Triangle, 75) + 1.0).abs() < 1e-4);
        assert!((sample(LfoShape::Saw, 0) + 1.0).abs() < 1e-5);
        assert!(sample(LfoShape::Saw, 50).abs() < 1e-4);
        assert_eq!(sample(LfoShape::Square, 10), 1.0);
        assert_eq!(sample(LfoShape::Square, 60), -1.0);
    }

    #[test]
    fn random_holds_for_a_cycle_then_changes() {
        let mut lfo = Lfo::new(9);
        let first = lfo.tick(LfoShape::Random, 0.1);
        for _ in 0..9 {
            assert_eq!(lfo.tick(LfoShape::Random, 0.1), first);
        }
        // Rounding may stretch a cycle by a step.
        let next = (0..2)
            .map(|_| lfo.tick(LfoShape::Random, 0.1))
            .next_back()
            .unwrap();
        assert_ne!(next, first);
    }

    #[test]
    fn reset_restarts_the_cycle_and_the_random_sequence() {
        let mut lfo = Lfo::new(3);
        let before: Vec<f32> = (0..50).map(|_| lfo.tick(LfoShape::Random, 0.3)).collect();
        lfo.reset();
        let after: Vec<f32> = (0..50).map(|_| lfo.tick(LfoShape::Random, 0.3)).collect();
        assert_eq!(before, after);
    }
}
