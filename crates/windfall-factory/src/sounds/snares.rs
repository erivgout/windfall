//! Snare drums: two tuned head modes for the body and filtered noise for
//! the wires.

use crate::dsp::{Audio, Mode, Rng, Svf, decay, decay_to, ring, synth};
use crate::math::tanh;

/// The wires under a snare: noise kept between two frequencies. Without the
/// upper limit it turns into hiss.
struct Wires {
    rng: Rng,
    floor: Svf,
    ceiling: Svf,
}

impl Wires {
    fn new(seed: u64, low: f64, high: f64) -> Self {
        Self {
            rng: Rng::new(seed),
            floor: Svf::new(low, 0.8),
            ceiling: Svf::new(high, 0.7),
        }
    }

    fn tick(&mut self) -> f64 {
        self.ceiling.lowpass(self.floor.highpass(self.rng.white()))
    }
}

/// A dry, short snare that cuts through: high head pitch, quick wires.
pub fn tight() -> Audio {
    let mut head = Mode::new(205.0, 0.045).bend(0.5, 0.008);
    let mut overtone = Mode::new(338.0, 0.03).bend(0.4, 0.008);
    let mut wires = Wires::new(0x21, 1_600.0, 9_000.0);
    Audio::Mono(synth(0.6, |t| {
        let body = head.tick(t) + 0.5 * overtone.tick(t);
        // The second term is the stick: a burst of the same noise that is
        // over in a few milliseconds.
        let rattle = wires.tick() * (decay_to(t, 0.05, 0.14) + 0.6 * decay(t, 0.001_5));
        tanh(1.4 * (0.7 * body + 1.6 * rattle))
    }))
}

/// A low, thick snare pushed into saturation, with a long wire tail.
pub fn fat() -> Audio {
    let mut head = Mode::new(168.0, 0.11).bend(0.45, 0.02);
    let mut overtone = Mode::new(255.0, 0.07).bend(0.4, 0.02);
    let mut wires = Wires::new(0x22, 900.0, 6_500.0);
    Audio::Mono(synth(1.0, |t| {
        let body = head.tick(t) + 0.6 * overtone.tick(t);
        let rattle = wires.tick() * decay_to(t, 0.12, 0.3);
        tanh(1.8 * (0.8 * body + 2.0 * rattle))
    }))
}

/// Mostly wires: a wash of bright noise with only a hint of drum under it.
pub fn noisy() -> Audio {
    let mut head = Mode::new(190.0, 0.05).bend(0.4, 0.01);
    let mut wires = Wires::new(0x23, 1_100.0, 11_000.0);
    let mut presence = Svf::new(5_200.0, 1.0);
    Audio::Mono(synth(1.2, |t| {
        let noise = wires.tick();
        let rattle = noise + 0.7 * presence.bandpass(noise);
        let amp = 0.55 * decay(t, 0.035) + 0.45 * decay_to(t, 0.17, 0.4);
        tanh(0.45 * head.tick(t) + 0.8 * rattle * amp)
    }))
}

/// Stick on rim and head together: a hard click and a ring of shell tones.
pub fn rimshot() -> Audio {
    let mut modes = [
        (0.6, Mode::new(228.0, 0.055).bend(0.3, 0.006)),
        (0.8, Mode::new(487.0, 0.04)),
        (0.8, Mode::new(812.0, 0.03)),
        (0.7, Mode::new(1_345.0, 0.022)),
        (0.6, Mode::new(2_110.0, 0.016)),
    ];
    let mut wires = Wires::new(0x24, 2_500.0, 10_000.0);
    Audio::Mono(synth(0.6, |t| {
        let rattle = wires.tick() * (0.9 * decay(t, 0.04) + 1.2 * decay(t, 0.001_5));
        tanh(1.5 * (0.6 * ring(&mut modes, t) + rattle))
    }))
}
