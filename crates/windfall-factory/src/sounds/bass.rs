//! Sustained bass notes, meant to be played up and down the keyboard by the
//! sampler. All three are recorded at the same C.

use crate::dsp::{Audio, Osc, Svf, decay, decay_to, synth};
use crate::math::{cents, tanh};

/// The pitch every bass sound is recorded at: C2, MIDI note 36, in equal
/// temperament with A4 at 440 Hz. A sampler whose root key is a C plays
/// these in tune.
pub const ROOT_HZ: f64 = 65.406_391_325_149_66;

/// A clean sub: a sine with just enough saturation to be heard on small
/// speakers.
pub fn sub() -> Audio {
    let mut osc = Osc::new();
    Audio::Mono(synth(3.5, |t| {
        tanh(1.2 * decay_to(t, 0.9, 1.0) * osc.sine(ROOT_HZ))
    }))
}

/// A sub with its octave, clipped hard. It stays loud for a second while
/// the clipper holds it, then cleans up as it fades.
pub fn drive() -> Audio {
    let mut fundamental = Osc::new();
    let mut octave = Osc::new();
    let mut tone = Svf::new(3_000.0, 0.7);
    Audio::Mono(synth(3.5, |t| {
        let raw = fundamental.sine(ROOT_HZ) + 0.45 * octave.sine(2.0 * ROOT_HZ);
        tone.lowpass(tanh(9.0 * decay(t, 0.8) * raw)) * decay_to(t, 1.6, 1.0)
    }))
}

/// A plucked synth bass: two detuned saws and a sub through a low-pass
/// filter that snaps shut.
pub fn pluck() -> Audio {
    let mut flat = Osc::new();
    let mut sharp = Osc::at(0.37);
    let mut sub = Osc::new();
    let mut filter = Svf::new(3_000.0, 1.6);
    let detune = cents(5.0);
    Audio::Mono(synth(2.0, |t| {
        filter.set_cutoff(170.0 + 2_800.0 * decay(t, 0.07));
        let raw =
            flat.saw(ROOT_HZ / detune) + sharp.saw(ROOT_HZ * detune) + 0.8 * sub.sine(ROOT_HZ);
        tanh(0.8 * filter.lowpass(raw)) * decay_to(t, 0.3, 0.55)
    }))
}
