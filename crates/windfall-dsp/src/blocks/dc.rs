//! Removes a constant offset from a signal.

use super::math::flush;

/// A first-order high-pass with its corner a few Hz above zero: the
/// difference of the input, leaked back in (`y[n] = x[n] - x[n-1] +
/// r * y[n-1]`). It removes DC and leaves everything audible alone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DcBlocker {
    previous_input: f32,
    previous_output: f32,
    pole: f32,
}

impl DcBlocker {
    /// A blocker with its corner at `cutoff_hz`. 5 Hz is a good default.
    pub fn new(cutoff_hz: f32, sample_rate: f32) -> Self {
        let pole = 1.0 - std::f32::consts::TAU * cutoff_hz / sample_rate.max(1.0);
        Self {
            previous_input: 0.0,
            previous_output: 0.0,
            pole: pole.clamp(0.0, 0.999_99),
        }
    }

    #[inline]
    pub fn tick(&mut self, input: f32) -> f32 {
        let output = input - self.previous_input + self.pole * self.previous_output;
        self.previous_input = input;
        self.previous_output = flush(output);
        output
    }

    pub fn reset(&mut self) {
        self.previous_input = 0.0;
        self.previous_output = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_an_offset_and_keeps_the_signal() {
        let mut blocker = DcBlocker::new(5.0, 48_000.0);
        let (mut sum, mut power, mut input_power) = (0.0_f64, 0.0_f64, 0.0_f64);
        for n in 0..96_000 {
            let tone = (std::f32::consts::TAU * 100.0 * n as f32 / 48_000.0).sin() * 0.5;
            let output = blocker.tick(tone + 0.4);
            if n >= 48_000 {
                sum += f64::from(output);
                power += f64::from(output * output);
                input_power += f64::from(tone * tone);
            }
        }
        assert!((sum / 48_000.0).abs() < 1e-4);
        let change_db = 10.0 * (power / input_power).log10();
        assert!(change_db.abs() < 0.05, "{change_db}");
    }

    #[test]
    fn a_step_decays_to_exact_zero() {
        let mut blocker = DcBlocker::new(5.0, 48_000.0);
        let mut output = 0.0;
        for _ in 0..(48_000 * 20) {
            output = blocker.tick(1.0);
        }
        assert_eq!(output, 0.0);
    }
}
