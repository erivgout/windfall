//! A scanned row of values: the oscillator [`ScanSynth`](super::ScanSynth)
//! uses.
//!
//! This is not an additive bank. The row is the waveform itself, read once
//! per cycle of the note, so the sound is whatever shape the row happens to
//! have. Reading it as a staircase is the point rather than an oversight:
//! the steps are what gives the instrument its edge, and a smoother is
//! offered for taking that edge off.

use crate::blocks::CONTROL_PERIOD;
use crate::blocks::math::{flush, lerp};

use super::engine::Tone;

/// Values in the row that is scanned. One cycle of the note covers all of
/// them.
pub const SCAN_STEPS: usize = 32;

/// The row an instrument wants scanned.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct ScanShape {
    pub row: [f32; SCAN_STEPS],
    /// Whether the steps are joined by a straight line instead of being
    /// held.
    pub interpolate: bool,
    /// Share of the distance to the current value the smoother closes each
    /// sample. 1 is no smoothing.
    pub smoothing: f32,
}

/// The row as a voice reads it: the values the control period starts from,
/// together with the change each one makes per sample.
#[derive(Clone, Copy)]
pub(super) struct ScanSetup {
    row: [f32; SCAN_STEPS],
    steps: [f32; SCAN_STEPS],
    interpolate: bool,
    smoothing: f32,
}

impl ScanSetup {
    /// The value of step `index` after `elapsed` samples of the control
    /// period.
    #[inline]
    fn value(&self, index: usize, elapsed: f32) -> f32 {
        self.row[index] + self.steps[index] * elapsed
    }
}

/// A voice scanning the row: where it has got to, and what its smoother
/// holds.
#[derive(Clone, Copy)]
pub(super) struct ScanTone {
    phase: f32,
    smoothed: f32,
}

impl Tone for ScanTone {
    type Target = ScanShape;
    type Setup = ScanSetup;

    fn silent() -> Self {
        Self {
            phase: 0.0,
            smoothed: 0.0,
        }
    }

    fn settled(target: &ScanShape) -> ScanSetup {
        ScanSetup {
            row: target.row,
            steps: [0.0; SCAN_STEPS],
            interpolate: target.interpolate,
            smoothing: target.smoothing,
        }
    }

    fn restart(&mut self, _setup: &ScanSetup) {
        // Every note starts at the top of the row, so the same note always
        // sounds the same.
        self.phase = 0.0;
        self.smoothed = 0.0;
    }

    fn glide(setup: &mut ScanSetup, target: &ScanShape, remaining: u32) {
        let span = CONTROL_PERIOD as f32;
        for index in 0..SCAN_STEPS {
            let now = setup.row[index] + setup.steps[index] * span;
            setup.row[index] = now;
            setup.steps[index] = if remaining > 0 {
                (target.row[index] - now) / (remaining as f32 * span)
            } else {
                0.0
            };
        }
        // Reading the row and smoothing it are ways of reading, not levels,
        // so they change at the period boundary.
        setup.interpolate = target.interpolate;
        setup.smoothing = target.smoothing;
    }

    fn render(&mut self, setup: &ScanSetup, increment: f32, offset: usize, out: &mut [f32]) {
        for (index, sample) in out.iter_mut().enumerate() {
            let elapsed = (offset + index) as f32;
            let position = self.phase * SCAN_STEPS as f32;
            let step = (position as usize).min(SCAN_STEPS - 1);
            let value = if setup.interpolate {
                let next = if step + 1 == SCAN_STEPS { 0 } else { step + 1 };
                lerp(
                    setup.value(step, elapsed),
                    setup.value(next, elapsed),
                    position - step as f32,
                )
            } else {
                setup.value(step, elapsed)
            };
            self.smoothed = flush(self.smoothed + (value - self.smoothed) * setup.smoothing);
            *sample += self.smoothed;
            self.phase += increment;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
        }
    }
}
