use super::params::{TransientShaperParams, TransientSplitParams};
use crate::balance::{Controls, audio, rate};
use crate::blocks::envelope::Ballistics;
use crate::blocks::math::db_to_gain_exp;
use crate::effect::Effect;
use crate::param::ParamSet;

struct TransientDetector {
    fast: Ballistics,
    slow: Ballistics,
}
impl Default for TransientDetector {
    fn default() -> Self {
        let mut detector = Self {
            fast: Ballistics::new(0.0),
            slow: Ballistics::new(0.0),
        };
        detector.prepare(48_000.0);
        detector
    }
}
impl TransientDetector {
    fn prepare(&mut self, sample_rate: f32) {
        let rate = rate(sample_rate).max(8.0);
        self.fast.set_times(0.15, 5.0, rate);
        self.slow.set_times(15.0, 120.0, rate);
        self.reset();
    }
    fn reset(&mut self) {
        self.fast.snap(0.0);
        self.slow.snap(0.0);
    }
    fn tick(&mut self, input: [f32; 2], sensitivity: f32) -> f32 {
        let peak = input[0].abs().max(input[1].abs());
        let fast = self.fast.tick(peak);
        // Follow the fast envelope rather than the rectified carrier. This
        // makes a steady tone converge to zero contrast instead of mistaking
        // differing peak-follower duty cycles for repeated attacks.
        let slow = self.slow.tick(fast);
        (((fast - slow).max(0.0) / fast.max(1.0e-6)) * sensitivity).clamp(0.0, 1.0)
    }
}

/// Fast/slow stereo-linked envelope contrast controls attack and body gain.
pub struct TransientShaper {
    detector: TransientDetector,
    controls: Controls<3>,
}
impl Default for TransientShaper {
    fn default() -> Self {
        Self {
            detector: TransientDetector::default(),
            controls: Controls::new([0.0, 0.0, 1.0]),
        }
    }
}
impl Effect for TransientShaper {
    type Params = TransientShaperParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.detector.prepare(sample_rate);
        self.controls.prepare(sample_rate);
    }
    fn reset(&mut self) {
        self.detector.reset();
        self.controls.reset();
    }
    fn set_params(&mut self, params: &Self::Params) {
        let p = params.sanitized();
        self.controls
            .set([18.0 * p.attack, 18.0 * p.sustain, p.sensitivity]);
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let input = [audio(*l), audio(*r)];
            let [attack, sustain, sensitivity] = self.controls.tick();
            let transient = self.detector.tick(input, sensitivity);
            let gain = db_to_gain_exp(attack * transient + sustain * (1.0 - transient));
            *l = audio(input[0] * gain);
            *r = audio(input[1] * gain);
        }
    }
}

/// Complementary time-varying separation, with independent output gains.
/// This is the `fx-transmitter` behavior, not a frequency exciter.
pub struct TransientSplit {
    detector: TransientDetector,
    controls: Controls<3>,
}
impl Default for TransientSplit {
    fn default() -> Self {
        Self {
            detector: TransientDetector::default(),
            controls: Controls::new([1.0; 3]),
        }
    }
}
impl TransientSplit {
    /// Separate-output seam: [transient, sustain], each a stereo frame.
    /// At unity gains these sum to the input within floating-point tolerance.
    pub fn split_frame(&mut self, input: [f32; 2]) -> [[f32; 2]; 2] {
        let input = input.map(audio);
        let [tg, sg, sensitivity] = self.controls.tick();
        let mask = self.detector.tick(input, sensitivity);
        let transient = input.map(|x| x * mask);
        [
            transient.map(|x| audio(x * tg)),
            std::array::from_fn(|c| audio((input[c] - transient[c]) * sg)),
        ]
    }
}
impl Effect for TransientSplit {
    type Params = TransientSplitParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.detector.prepare(sample_rate);
        self.controls.prepare(sample_rate);
    }
    fn reset(&mut self) {
        self.detector.reset();
        self.controls.reset();
    }
    fn set_params(&mut self, params: &Self::Params) {
        let p = params.sanitized();
        self.controls.set([
            db_to_gain_exp(p.transient_gain_db),
            db_to_gain_exp(p.sustain_gain_db),
            p.sensitivity,
        ]);
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let [attack, sustain] = self.split_frame([*l, *r]);
            *l = audio(attack[0] + sustain[0]);
            *r = audio(attack[1] + sustain[1]);
        }
    }
}
