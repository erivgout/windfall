//! Envelope followers: how loud a signal is, and a smoother that rises and
//! falls at different speeds.
//!
//! Dynamics processors measure level in dB and smooth in dB, as recommended
//! in Giannoulis, Massberg and Reiss, "Digital Dynamic Range Compressor
//! Design: A Tutorial and Analysis" (JAES, 2012). Working in the log domain
//! makes attack and release times mean the same thing at every level.

use super::math::{flush, level_to_db, power_to_db, smoothing_coefficient};

/// Peak level of a stereo sample in dB: the louder of the two channels.
#[inline]
pub fn peak_db(left: f32, right: f32) -> f32 {
    level_to_db(left.abs().max(right.abs()))
}

/// A running mean of the squared signal, for RMS level detection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeanSquare {
    value: f32,
    coefficient: f32,
}

impl MeanSquare {
    /// A detector that averages over roughly `window_ms`.
    pub fn new(window_ms: f32, sample_rate: f32) -> Self {
        Self {
            value: 0.0,
            coefficient: smoothing_coefficient(window_ms, sample_rate),
        }
    }

    /// Takes one stereo sample and returns the RMS level in dB, averaged
    /// over both channels. A full-scale sine reads -3 dB.
    #[inline]
    pub fn tick_db(&mut self, left: f32, right: f32) -> f32 {
        let power = 0.5 * (left * left + right * right);
        self.value = flush(self.value + (power - self.value) * self.coefficient);
        power_to_db(self.value)
    }

    pub fn reset(&mut self) {
        self.value = 0.0;
    }
}

/// A smoother with one speed for rising and another for falling.
///
/// Fed a level, it is a classic envelope follower. Fed a compressor's gain
/// reduction, the rise time is the attack and the fall time the release.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ballistics {
    value: f32,
    /// Share of the distance to the input that is left after one sample of
    /// rising, and of falling. Zero follows the input exactly.
    rise_keep: f32,
    fall_keep: f32,
}

impl Ballistics {
    /// A follower resting on `value` that follows its input instantly until
    /// [`Ballistics::set_times`] is called.
    pub fn new(value: f32) -> Self {
        Self {
            value,
            rise_keep: 0.0,
            fall_keep: 0.0,
        }
    }

    /// Sets the times to cover 63% of a rise and of a fall.
    pub fn set_times(&mut self, rise_ms: f32, fall_ms: f32, sample_rate: f32) {
        self.rise_keep = 1.0 - smoothing_coefficient(rise_ms, sample_rate);
        self.fall_keep = 1.0 - smoothing_coefficient(fall_ms, sample_rate);
    }

    #[inline]
    pub fn tick(&mut self, input: f32) -> f32 {
        let keep = if input > self.value {
            self.rise_keep
        } else {
            self.fall_keep
        };
        self.value = flush(input + (self.value - input) * keep);
        self.value
    }

    #[inline]
    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn snap(&mut self, value: f32) {
        self.value = value;
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;

    use super::*;
    use crate::blocks::math::SILENCE_DB;

    #[test]
    fn peak_level_is_the_louder_channel() {
        assert!((peak_db(0.5, -1.0)).abs() < 1e-5);
        assert!((peak_db(-0.25, 0.1) + 12.04).abs() < 0.01);
        assert_eq!(peak_db(0.0, 0.0), SILENCE_DB);
    }

    #[test]
    fn rms_of_a_sine_is_three_decibels_under_its_peak() {
        let mut detector = MeanSquare::new(20.0, 48_000.0);
        let mut level = 0.0;
        for n in 0..48_000 {
            let sample = (TAU * 1_000.0 * n as f32 / 48_000.0).sin();
            level = detector.tick_db(sample, sample);
        }
        assert!((level + 3.01).abs() < 0.1, "{level}");
    }

    #[test]
    fn rms_of_silence_reaches_the_floor_and_exact_zero() {
        let mut detector = MeanSquare::new(10.0, 48_000.0);
        detector.tick_db(1.0, 1.0);
        let mut level = 0.0;
        for _ in 0..48_000 {
            level = detector.tick_db(0.0, 0.0);
        }
        assert_eq!(level, SILENCE_DB);
        assert_eq!(detector.value, 0.0);
    }

    #[test]
    fn rise_and_fall_take_their_own_times() {
        let mut follower = Ballistics::new(0.0);
        follower.set_times(10.0, 100.0, 48_000.0);
        for _ in 0..480 {
            follower.tick(1.0);
        }
        assert!(
            (follower.value() - 0.632).abs() < 0.003,
            "{}",
            follower.value()
        );
        follower.snap(1.0);
        for _ in 0..4_800 {
            follower.tick(0.0);
        }
        assert!(
            (follower.value() - 0.368).abs() < 0.003,
            "{}",
            follower.value()
        );
    }

    #[test]
    fn zero_times_follow_instantly() {
        let mut follower = Ballistics::new(0.0);
        follower.set_times(0.0, 0.0, 48_000.0);
        assert_eq!(follower.tick(0.7), 0.7);
        assert_eq!(follower.tick(0.2), 0.2);
    }
}
