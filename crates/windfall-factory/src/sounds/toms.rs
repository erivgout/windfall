//! Toms: three sizes of the same drum.

use crate::dsp::{Audio, Mode, Rng, Svf, decay, synth};
use crate::math::tanh;

/// A tom tuned to `freq` hertz that rings with time constant `tau` seconds.
/// The two quieter modes sit at the overtone ratios of a round drum head,
/// which is what makes it sound like a drum and not a sine wave.
fn tom(freq: f64, tau: f64, seed: u64) -> Audio {
    let mut head = Mode::new(freq, tau).bend(0.5, 0.025);
    let mut second = Mode::new(freq * 1.59, tau * 0.45).bend(0.3, 0.02);
    let mut third = Mode::new(freq * 2.14, tau * 0.3);
    let mut rng = Rng::new(seed);
    let mut stick = Svf::new(1_500.0, 0.7);
    Audio::Mono(synth(tau * 7.0, |t| {
        let hit = stick.bandpass(rng.white()) * decay(t, 0.004);
        let ring = head.tick(t) + 0.3 * second.tick(t) + 0.15 * third.tick(t);
        tanh(1.3 * (ring + 0.35 * hit))
    }))
}

/// Floor tom, around 82 Hz.
pub fn low() -> Audio {
    tom(82.4, 0.2, 0x61)
}

/// Middle tom, around 123 Hz.
pub fn mid() -> Audio {
    tom(123.5, 0.16, 0x62)
}

/// High tom, around 175 Hz.
pub fn high() -> Audio {
    tom(174.6, 0.13, 0x63)
}
