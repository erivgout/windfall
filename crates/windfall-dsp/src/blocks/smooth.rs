//! Parameter smoothers: a value that moves to its target instead of jumping
//! there, so a control change never clicks or zippers.

use super::math::smoothing_coefficient;

/// A value that reaches its target in a straight line over a fixed number
/// of samples, and lands on it exactly.
///
/// Use it for gains and mixes, where the arrival time should not depend on
/// the size of the change.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearRamp {
    value: f32,
    target: f32,
    step: f32,
    remaining: u32,
}

impl LinearRamp {
    /// A ramp resting on `value`.
    pub fn new(value: f32) -> Self {
        Self {
            value,
            target: value,
            step: 0.0,
            remaining: 0,
        }
    }

    /// Jumps to `value` and rests there.
    pub fn snap(&mut self, value: f32) {
        *self = Self::new(value);
    }

    /// Starts moving toward `target` from the current value, arriving after
    /// `samples` calls to [`LinearRamp::tick`]. Zero samples jumps.
    pub fn set_target(&mut self, target: f32, samples: u32) {
        if samples == 0 {
            self.snap(target);
            return;
        }
        if target == self.target {
            return;
        }

        self.target = target;
        self.step = (target - self.value) / samples as f32;
        self.remaining = samples;
    }

    /// Advances one sample and returns the new value.
    #[inline]
    pub fn tick(&mut self) -> f32 {
        if self.remaining > 0 {
            self.remaining -= 1;
            self.value = if self.remaining == 0 {
                self.target
            } else {
                self.value + self.step
            };
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> f32 {
        self.value
    }

    #[inline]
    pub fn target(&self) -> f32 {
        self.target
    }

    /// True when the value rests on the target.
    #[inline]
    pub fn is_settled(&self) -> bool {
        self.remaining == 0
    }
}

/// A value that closes a fixed share of the distance to its target each
/// sample, and snaps onto the target once the rest is inaudible.
///
/// Use it where the natural shape of a change is exponential, such as a
/// frequency or a time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OnePole {
    value: f32,
    target: f32,
    coefficient: f32,
}

impl OnePole {
    /// A smoother resting on `value` that does not smooth until
    /// [`OnePole::set_time`] is called.
    pub fn new(value: f32) -> Self {
        Self {
            value,
            target: value,
            coefficient: 1.0,
        }
    }

    /// Sets the time to cover 63% of a change. `rate` is how many times per
    /// second [`OnePole::tick`] is called.
    pub fn set_time(&mut self, time_ms: f32, rate: f32) {
        self.coefficient = smoothing_coefficient(time_ms, rate);
    }

    pub fn snap(&mut self, value: f32) {
        self.value = value;
        self.target = value;
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Advances one step and returns the new value.
    #[inline]
    pub fn tick(&mut self) -> f32 {
        if self.value != self.target {
            let gap = self.target - self.value;
            self.value = if gap.abs() <= 1.0e-5 * (1.0 + self.target.abs()) {
                self.target
            } else {
                self.value + gap * self.coefficient
            };
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> f32 {
        self.value
    }

    #[inline]
    pub fn target(&self) -> f32 {
        self.target
    }

    #[inline]
    pub fn is_settled(&self) -> bool {
        self.value == self.target
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_ramp_moves_evenly_and_lands_exactly() {
        let mut ramp = LinearRamp::new(1.0);
        ramp.set_target(0.0, 4);
        assert!(!ramp.is_settled());
        assert!((ramp.tick() - 0.75).abs() < 1e-6);
        assert!((ramp.tick() - 0.5).abs() < 1e-6);
        assert!((ramp.tick() - 0.25).abs() < 1e-6);
        assert_eq!(ramp.tick(), 0.0);
        assert!(ramp.is_settled());
        assert_eq!(ramp.tick(), 0.0);
    }

    #[test]
    fn linear_ramp_retargets_from_where_it_is() {
        let mut ramp = LinearRamp::new(0.0);
        ramp.set_target(1.0, 10);
        for _ in 0..5 {
            ramp.tick();
        }
        ramp.set_target(0.0, 5);
        let first = ramp.tick();
        assert!((first - 0.4).abs() < 1e-6, "{first}");
        for _ in 0..4 {
            ramp.tick();
        }
        assert_eq!(ramp.value(), 0.0);
    }

    #[test]
    fn linear_ramp_never_steps_more_than_its_slope() {
        let mut ramp = LinearRamp::new(0.2);
        ramp.set_target(0.9, 960);
        let mut previous = ramp.value();
        for _ in 0..960 {
            let value = ramp.tick();
            assert!((value - previous).abs() <= 1.1 * 0.7 / 960.0);
            previous = value;
        }
        assert_eq!(previous, 0.9);
    }

    #[test]
    fn zero_length_ramp_jumps() {
        let mut ramp = LinearRamp::new(0.0);
        ramp.set_target(1.0, 0);
        assert!(ramp.is_settled());
        assert_eq!(ramp.value(), 1.0);
    }

    #[test]
    fn one_pole_covers_63_percent_in_its_time_and_settles() {
        let mut smoother = OnePole::new(0.0);
        smoother.set_time(10.0, 48_000.0);
        smoother.set_target(1.0);
        for _ in 0..480 {
            smoother.tick();
        }
        assert!((smoother.value() - 0.632).abs() < 0.002);
        for _ in 0..48_000 {
            smoother.tick();
        }
        assert!(smoother.is_settled());
        assert_eq!(smoother.value(), 1.0);
    }

    #[test]
    fn one_pole_without_a_time_does_not_smooth() {
        let mut smoother = OnePole::new(0.0);
        smoother.set_target(0.5);
        assert_eq!(smoother.tick(), 0.5);
    }
}
