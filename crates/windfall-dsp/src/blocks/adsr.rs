//! An attack, decay, sustain, release envelope with exponential segments.

use super::math::{flush, smoothing_coefficient};

/// How far past its goal the attack aims, as a share of full level. Aiming
/// high and stopping at the goal gives the slightly rounded rise of an
/// analogue envelope instead of a straight line.
const ATTACK_OVERSHOOT: f32 = 0.3;

/// How far past its goal a decay or release aims, as a share of the drop.
/// The curve covers 60 dB of the drop before it is cut off at the goal.
const FALL_OVERSHOOT: f32 = 0.001;

/// Time a held note takes to follow a change of the sustain level.
const SUSTAIN_GLIDE_MS: f32 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdsrStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// An envelope generator.
///
/// Each segment is an exponential approach to a point a little past its
/// goal, stopped when it reaches the goal. That gives segments the curved
/// shape ears expect and makes each one last exactly its set time: the
/// attack from silence to full level, the decay from full level to the
/// sustain level, the release from wherever the note was let go to silence.
///
/// A new note that arrives while the envelope is still sounding rises from
/// the current level, so retriggering never clicks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Adsr {
    stage: AdsrStage,
    value: f32,
    attack: f32,
    decay: f32,
    release: f32,
    sustain: f32,
    /// Share of the distance to a changed sustain level closed per step.
    sustain_glide: f32,
    /// The point below zero the current release aims at.
    release_goal: f32,
}

impl Default for Adsr {
    fn default() -> Self {
        Self {
            stage: AdsrStage::Idle,
            value: 0.0,
            attack: 0.0,
            decay: 0.0,
            release: 0.0,
            sustain: 1.0,
            sustain_glide: 1.0,
            release_goal: 0.0,
        }
    }
}

/// Multiplier per step for a segment that lasts `steps` and aims
/// `overshoot` past its goal.
fn segment_coefficient(steps: f32, overshoot: f32) -> f32 {
    if steps < 1.0 {
        return 0.0;
    }
    ((overshoot / (1.0 + overshoot)).ln() / steps).exp()
}

impl Adsr {
    /// Sets the segment times in milliseconds and the sustain level, 0 to 1.
    /// `rate` is how many times per second [`Adsr::tick`] is called. Safe to
    /// call while a note sounds.
    pub fn configure(
        &mut self,
        attack_ms: f32,
        decay_ms: f32,
        sustain: f32,
        release_ms: f32,
        rate: f32,
    ) {
        let steps = |time_ms: f32| time_ms * 0.001 * rate;
        self.attack = segment_coefficient(steps(attack_ms), ATTACK_OVERSHOOT);
        self.decay = segment_coefficient(steps(decay_ms), FALL_OVERSHOOT);
        self.release = segment_coefficient(steps(release_ms), FALL_OVERSHOOT);
        self.sustain = sustain.clamp(0.0, 1.0);
        self.sustain_glide = smoothing_coefficient(SUSTAIN_GLIDE_MS, rate);
    }

    /// Starts the attack from the current level.
    pub fn gate_on(&mut self) {
        self.stage = AdsrStage::Attack;
    }

    /// Starts the release from the current level.
    pub fn gate_off(&mut self) {
        if self.stage == AdsrStage::Idle {
            return;
        }
        self.stage = AdsrStage::Release;
        self.release_goal = -FALL_OVERSHOOT * self.value;
    }

    /// Silences the envelope at once.
    pub fn reset(&mut self) {
        self.stage = AdsrStage::Idle;
        self.value = 0.0;
    }

    /// Scales the current level, for handing a fading voice to a new note
    /// without a jump.
    pub fn scale_level(&mut self, factor: f32) {
        self.value *= factor;
    }

    /// Advances one step and returns the level, 0 to 1.
    #[inline]
    pub fn tick(&mut self) -> f32 {
        match self.stage {
            AdsrStage::Idle => {}
            AdsrStage::Attack => {
                let goal = 1.0 + ATTACK_OVERSHOOT;
                self.value = goal + (self.value - goal) * self.attack;
                if self.value >= 1.0 {
                    self.value = 1.0;
                    self.stage = AdsrStage::Decay;
                }
            }
            AdsrStage::Decay => {
                let goal = self.sustain - FALL_OVERSHOOT * (1.0 - self.sustain);
                self.value = goal + (self.value - goal) * self.decay;
                if self.value <= self.sustain {
                    self.value = self.sustain;
                    self.stage = AdsrStage::Sustain;
                }
            }
            AdsrStage::Sustain => {
                if self.value != self.sustain {
                    let gap = self.sustain - self.value;
                    self.value = if gap.abs() < 1.0e-5 {
                        self.sustain
                    } else {
                        self.value + gap * self.sustain_glide
                    };
                }
            }
            AdsrStage::Release => {
                self.value = self.release_goal + (self.value - self.release_goal) * self.release;
                if self.value <= 0.0 {
                    self.value = 0.0;
                    self.stage = AdsrStage::Idle;
                }
                self.value = flush(self.value);
            }
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> f32 {
        self.value
    }

    #[inline]
    pub fn stage(&self) -> AdsrStage {
        self.stage
    }

    #[inline]
    pub fn is_idle(&self) -> bool {
        self.stage == AdsrStage::Idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    fn envelope(attack: f32, decay: f32, sustain: f32, release: f32) -> Adsr {
        let mut adsr = Adsr::default();
        adsr.configure(attack, decay, sustain, release, RATE);
        adsr
    }

    /// Steps until the envelope leaves `stage`.
    fn steps_in(adsr: &mut Adsr, stage: AdsrStage) -> usize {
        let mut steps = 0;
        while adsr.stage() == stage {
            adsr.tick();
            steps += 1;
            assert!(steps < 10_000_000, "stuck in {stage:?}");
        }
        steps
    }

    #[test]
    fn each_segment_lasts_its_set_time() {
        let mut adsr = envelope(10.0, 100.0, 0.5, 200.0);
        adsr.gate_on();
        let attack = steps_in(&mut adsr, AdsrStage::Attack);
        assert!((attack as i64 - 480).abs() <= 2, "attack {attack}");
        assert_eq!(adsr.value(), 1.0);
        let decay = steps_in(&mut adsr, AdsrStage::Decay);
        assert!((decay as i64 - 4_800).abs() <= 5, "decay {decay}");
        assert_eq!(adsr.value(), 0.5);
        for _ in 0..1_000 {
            assert_eq!(adsr.tick(), 0.5);
        }
        adsr.gate_off();
        let release = steps_in(&mut adsr, AdsrStage::Release);
        assert!((release as i64 - 9_600).abs() <= 10, "release {release}");
        assert_eq!(adsr.value(), 0.0);
        assert!(adsr.is_idle());
    }

    #[test]
    fn segments_are_curved_and_monotonic() {
        let mut adsr = envelope(20.0, 50.0, 0.25, 50.0);
        adsr.gate_on();
        let mut previous = 0.0;
        let mut halfway = 0.0;
        for step in 0..960 {
            let value = adsr.tick();
            assert!(value >= previous);
            previous = value;
            if step == 479 {
                halfway = value;
            }
        }
        // An exponential rise is ahead of a straight line at the midpoint.
        assert!(halfway > 0.55 && halfway < 0.8, "{halfway}");
        steps_in(&mut adsr, AdsrStage::Attack);
        previous = adsr.value();
        while adsr.stage() == AdsrStage::Decay {
            let value = adsr.tick();
            assert!(value <= previous);
            previous = value;
        }
        adsr.gate_off();
        // Most of the release's drop happens early.
        for _ in 0..600 {
            adsr.tick();
        }
        assert!(adsr.value() < 0.25 * 0.2, "{}", adsr.value());
    }

    #[test]
    fn retriggering_rises_from_the_current_level() {
        let mut adsr = envelope(10.0, 10.0, 0.6, 100.0);
        adsr.gate_on();
        for _ in 0..2_000 {
            adsr.tick();
        }
        adsr.gate_off();
        for _ in 0..500 {
            adsr.tick();
        }
        let before = adsr.value();
        assert!(before > 0.0 && before < 0.6);
        adsr.gate_on();
        let after = adsr.tick();
        assert!(
            after > before && after - before < 0.01,
            "{before} to {after}"
        );
    }

    #[test]
    fn zero_times_jump() {
        let mut adsr = envelope(0.0, 0.0, 0.3, 0.0);
        adsr.gate_on();
        assert_eq!(adsr.tick(), 1.0);
        assert_eq!(adsr.tick(), 0.3);
        adsr.gate_off();
        assert_eq!(adsr.tick(), 0.0);
        assert!(adsr.is_idle());
    }

    #[test]
    fn a_new_sustain_level_is_reached_without_a_jump() {
        let mut adsr = envelope(1.0, 1.0, 0.8, 10.0);
        adsr.gate_on();
        for _ in 0..1_000 {
            adsr.tick();
        }
        adsr.configure(1.0, 1.0, 0.2, 10.0, RATE);
        let mut previous = adsr.value();
        for _ in 0..20_000 {
            let value = adsr.tick();
            assert!((value - previous).abs() < 0.002);
            previous = value;
        }
        assert_eq!(previous, 0.2);
    }

    #[test]
    fn full_sustain_skips_the_decay() {
        let mut adsr = envelope(1.0, 500.0, 1.0, 10.0);
        adsr.gate_on();
        for _ in 0..200 {
            adsr.tick();
        }
        assert_eq!(adsr.stage(), AdsrStage::Sustain);
        assert_eq!(adsr.value(), 1.0);
    }

    #[test]
    fn releasing_an_idle_envelope_keeps_it_idle() {
        let mut adsr = envelope(1.0, 1.0, 0.5, 10.0);
        adsr.gate_off();
        assert!(adsr.is_idle());
        assert_eq!(adsr.tick(), 0.0);
    }
}
