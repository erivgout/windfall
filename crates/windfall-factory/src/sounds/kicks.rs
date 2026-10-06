//! Kick drums: a sine whose pitch dives from a knock down to a low note,
//! with a short noise click for the beater.

use crate::dsp::{Audio, Osc, Rng, Svf, attack, decay, decay_to, synth};
use crate::math::tanh;

/// The beater hitting the head: high-passed noise that is gone within a few
/// milliseconds.
fn beater(seed: u64, cutoff: f64, tau: f64) -> impl FnMut(f64) -> f64 {
    let mut rng = Rng::new(seed);
    let mut filter = Svf::new(cutoff, 0.7);
    move |t| filter.highpass(rng.white()) * decay(t, tau)
}

/// The all-round kick: a firm knock, a body around 49 Hz and light
/// saturation that holds the level up for the first tenth of a second.
pub fn punch() -> Audio {
    let mut body = Osc::new();
    let mut click = beater(0x11, 2_500.0, 0.001_8);
    Audio::Mono(synth(0.8, |t| {
        let freq = 49.0 + 130.0 * decay(t, 0.028) + 230.0 * decay(t, 0.005);
        let amp = decay_to(t, 0.14, 0.26);
        tanh(1.7 * amp * body.sine(freq)) + 0.25 * click(t)
    }))
}

/// A long, low kick that settles on 43.65 Hz and rings for over a second.
pub fn deep() -> Audio {
    let mut body = Osc::new();
    let mut click = beater(0x12, 900.0, 0.003);
    Audio::Mono(synth(2.0, |t| {
        let freq = 43.65 + 70.0 * decay(t, 0.05) + 110.0 * decay(t, 0.009);
        let amp = decay_to(t, 0.4, 0.75);
        tanh(1.3 * amp * body.sine(freq)) + 0.08 * click(t)
    }))
}

/// A short, clicky kick that gets out of the way of a bass line.
pub fn tight() -> Audio {
    let mut body = Osc::new();
    let mut click = beater(0x13, 4_000.0, 0.001_2);
    Audio::Mono(synth(0.5, |t| {
        let freq = 61.74 + 170.0 * decay(t, 0.014) + 320.0 * decay(t, 0.003);
        let amp = decay_to(t, 0.055, 0.11);
        tanh(1.5 * amp * body.sine(freq)) + 0.4 * click(t)
    }))
}

/// A kick driven hard into the clipper, so the body turns square and buzzes.
pub fn hard() -> Audio {
    let mut body = Osc::new();
    let mut click = beater(0x14, 3_000.0, 0.002);
    let mut rng = Rng::new(0x15);
    let mut crunch = Svf::new(2_200.0, 1.0);
    let mut tone = Svf::new(7_000.0, 0.7);
    Audio::Mono(synth(1.2, |t| {
        let freq = 51.91 + 210.0 * decay(t, 0.03) + 420.0 * decay(t, 0.005);
        let driven = tanh(7.0 * decay(t, 0.22) * body.sine(freq));
        let grit = 0.25 * crunch.bandpass(rng.white()) * decay(t, 0.02);
        tone.lowpass(driven + grit + 0.3 * click(t)) * decay_to(t, 0.3, 0.42)
    }))
}

/// A round kick with no click and a millisecond of attack, for quiet parts.
pub fn soft() -> Audio {
    let mut body = Osc::new();
    let mut second = Osc::new();
    Audio::Mono(synth(0.9, |t| {
        let freq = 55.0 + 45.0 * decay(t, 0.045);
        let amp = attack(t, 0.001_2) * decay_to(t, 0.16, 0.3);
        amp * (body.sine(freq) + 0.06 * second.sine(2.0 * freq))
    }))
}
