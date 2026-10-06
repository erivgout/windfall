//! Hi-hats: a cluster of clashing square waves and a little noise, with
//! everything below the treble filtered away.

use crate::dsp::{Audio, Metal, Rng, Svf, attack, decay, decay_to, synth};

/// The settings that tell one hat from another.
struct Hat {
    /// Pitch of the lowest square wave in the cluster, in hertz.
    base: f64,
    /// Blend of the cluster, from 0 (summed) to 1 (ring-modulated).
    ring: f64,
    /// Level of the white noise mixed in with the cluster.
    noise: f64,
    /// Everything below this frequency is removed.
    highpass: f64,
    /// Center of an extra peak that colors the tone, in hertz.
    peak: f64,
    /// Level of that peak.
    peak_gain: f64,
    seed: u64,
}

impl Hat {
    /// Renders `seconds` of this hat with the loudness curve `envelope`.
    fn play(&self, seconds: f64, envelope: impl Fn(f64) -> f64) -> Audio {
        let mut metal = Metal::new(self.base);
        let mut rng = Rng::new(self.seed);
        let mut high = Svf::new(self.highpass, 0.8);
        let mut steep = Svf::new(self.highpass, 0.8);
        let mut peak = Svf::new(self.peak, 1.5);
        Audio::Mono(synth(seconds, |t| {
            let source = metal.tick(self.ring) + self.noise * rng.white();
            let bright = steep.highpass(high.highpass(source));
            (bright + self.peak_gain * peak.bandpass(bright)) * envelope(t)
        }))
    }
}

/// The everyday closed hat: crisp, about a twentieth of a second long.
pub fn closed_1() -> Audio {
    Hat {
        base: 252.0,
        ring: 0.0,
        noise: 0.25,
        highpass: 7_200.0,
        peak: 9_800.0,
        peak_gain: 0.6,
        seed: 0x41,
    }
    .play(0.3, |t| decay_to(t, 0.016, 0.05))
}

/// A thinner, shorter tick for fast rolls.
pub fn closed_2() -> Audio {
    Hat {
        base: 318.0,
        ring: 0.8,
        noise: 0.2,
        highpass: 8_600.0,
        peak: 12_000.0,
        peak_gain: 0.5,
        seed: 0x42,
    }
    .play(0.2, |t| decay_to(t, 0.01, 0.035))
}

/// A darker, looser closed hat with more noise in it.
pub fn closed_3() -> Audio {
    Hat {
        base: 214.0,
        ring: 0.3,
        noise: 0.4,
        highpass: 5_200.0,
        peak: 6_800.0,
        peak_gain: 0.8,
        seed: 0x43,
    }
    .play(0.4, |t| decay_to(t, 0.03, 0.09))
}

/// An open hat of medium length that pairs with `closed_1`.
pub fn open_1() -> Audio {
    Hat {
        base: 252.0,
        ring: 0.0,
        noise: 0.22,
        highpass: 6_600.0,
        peak: 9_000.0,
        peak_gain: 0.5,
        seed: 0x44,
    }
    .play(1.5, |t| {
        0.45 * decay(t, 0.04) + 0.55 * decay_to(t, 0.2, 0.5)
    })
}

/// A long, bright, sizzling open hat.
pub fn open_2() -> Audio {
    Hat {
        base: 296.0,
        ring: 0.5,
        noise: 0.3,
        highpass: 7_600.0,
        peak: 11_000.0,
        peak_gain: 0.7,
        seed: 0x45,
    }
    .play(2.5, |t| {
        0.35 * decay(t, 0.05) + 0.65 * decay_to(t, 0.38, 0.9)
    })
}

/// The foot closing the hats: a dull, soft-edged "chick".
pub fn pedal() -> Audio {
    Hat {
        base: 231.0,
        ring: 0.2,
        noise: 0.2,
        highpass: 4_800.0,
        peak: 6_200.0,
        peak_gain: 1.2,
        seed: 0x46,
    }
    .play(0.3, |t| attack(t, 0.000_8) * decay_to(t, 0.02, 0.06))
}
