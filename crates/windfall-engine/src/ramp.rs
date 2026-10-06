//! A gain or pan value that moves to a new target in a straight line.

/// A value that glides to its target instead of stepping, so a fader move
/// never clicks.
///
/// The value at any frame is a pure function of that frame. Nothing is
/// advanced per block, which keeps the output identical for every buffer
/// size and lets any number of voices read the same ramp.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Ramp {
    target: f32,
    /// Change per frame while the ramp is still moving.
    step: f32,
    /// First frame at which the value sits on the target.
    end: u64,
}

impl Ramp {
    /// A ramp already resting on `value`.
    pub fn at_rest(value: f32) -> Self {
        Self {
            target: value,
            step: 0.0,
            end: 0,
        }
    }

    #[inline]
    pub fn at(&self, frame: u64) -> f32 {
        if frame >= self.end {
            self.target
        } else {
            self.target - self.step * (self.end - frame) as f32
        }
    }

    /// True when the value no longer changes from `frame` on.
    #[inline]
    pub fn settled(&self, frame: u64) -> bool {
        frame >= self.end
    }

    /// Starts moving toward `target` from wherever the value is at `now`,
    /// arriving `frames` later.
    pub fn retarget(&mut self, target: f32, now: u64, frames: u64) {
        if target == self.target {
            return;
        }
        let frames = frames.max(1);
        let current = self.at(now);
        self.step = (target - current) / frames as f32;
        self.target = target;
        self.end = now + frames;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moves_in_a_straight_line_and_lands_exactly() {
        let mut ramp = Ramp::at_rest(1.0);
        ramp.retarget(0.0, 100, 4);
        assert!((ramp.at(100) - 1.0).abs() < 1e-6);
        assert!((ramp.at(102) - 0.5).abs() < 1e-6);
        assert_eq!(ramp.at(104), 0.0);
        assert!(!ramp.settled(103));
        assert!(ramp.settled(104));
    }

    #[test]
    fn retargeting_mid_ramp_continues_from_the_current_value() {
        let mut ramp = Ramp::at_rest(0.0);
        ramp.retarget(1.0, 0, 10);
        ramp.retarget(0.0, 5, 10);
        assert!((ramp.at(5) - 0.5).abs() < 1e-6);
        assert_eq!(ramp.at(15), 0.0);
    }
}
