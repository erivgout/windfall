//! Hand percussion and small struck things.

use crate::dsp::{Audio, Metal, Mode, Osc, Rng, Svf, attack, burst, decay, decay_to, ring, synth};
use crate::math::tanh;

/// A stick laid across the snare and tapped on the rim: a dry, woody tick.
pub fn rim_click() -> Audio {
    let mut modes = [
        (1.0, Mode::new(512.0, 0.012)),
        (0.7, Mode::new(1_730.0, 0.007)),
        (0.4, Mode::new(3_050.0, 0.003)),
    ];
    let mut rng = Rng::new(0x71);
    let mut click = Svf::new(2_000.0, 0.7);
    Audio::Mono(synth(0.3, |t| {
        ring(&mut modes, t) + 0.6 * click.highpass(rng.white()) * decay(t, 0.000_8)
    }))
}

/// A cowbell: two square waves a rough fifth apart through a resonant
/// filter, with a hard front and a short ring.
pub fn cowbell() -> Audio {
    let mut low = Osc::new();
    let mut high = Osc::at(0.3);
    let mut body = Svf::new(1_750.0, 2.0);
    let mut floor = Svf::new(500.0, 0.7);
    Audio::Mono(synth(1.0, |t| {
        let raw = low.square(571.0) + high.square(859.0);
        let amp = 0.6 * decay(t, 0.014) + 0.4 * decay_to(t, 0.11, 0.35);
        (0.4 * floor.highpass(raw) + body.bandpass(raw)) * amp
    }))
}

/// A hollow wooden block struck with a stick.
pub fn wood_block() -> Audio {
    let mut modes = [
        (1.0, Mode::new(1_040.0, 0.016).bend(0.08, 0.003)),
        (0.5, Mode::new(1_530.0, 0.009)),
        (0.25, Mode::new(2_870.0, 0.005)),
    ];
    let mut rng = Rng::new(0x72);
    let mut click = Svf::new(3_000.0, 1.0);
    Audio::Mono(synth(0.3, |t| {
        ring(&mut modes, t) + 0.5 * click.bandpass(rng.white()) * decay(t, 0.001)
    }))
}

/// Two hardwood sticks struck together: one clear, high, short note.
pub fn clave() -> Audio {
    let mut modes = [
        (1.0, Mode::new(2_480.0, 0.022)),
        (0.15, Mode::new(6_720.0, 0.006)),
    ];
    let mut rng = Rng::new(0x73);
    let mut click = Svf::new(4_000.0, 0.7);
    Audio::Mono(synth(0.3, |t| {
        ring(&mut modes, t) + 0.3 * click.highpass(rng.white()) * decay(t, 0.000_5)
    }))
}

/// One forward shake: treble noise that swells for a moment before it
/// fades, as the beads catch up with the shell.
pub fn shaker() -> Audio {
    let mut rng = Rng::new(0x74);
    let mut floor = Svf::new(4_500.0, 0.7);
    let mut band = Svf::new(7_500.0, 0.9);
    Audio::Mono(synth(0.5, |t| {
        let swell = (0.35 + 0.65 * attack(t, 0.012)) * attack(t, 0.000_6);
        band.bandpass(floor.highpass(rng.white())) * swell * decay_to(t, 0.08, 0.07)
    }))
}

/// A conga played open: a singing head tone with the slap of the palm.
pub fn conga() -> Audio {
    let mut modes = [
        (1.0, Mode::new(233.0, 0.11).bend(0.15, 0.012)),
        (0.35, Mode::new(410.0, 0.055)),
        (0.2, Mode::new(655.0, 0.03)),
    ];
    let mut rng = Rng::new(0x75);
    let mut slap = Svf::new(1_800.0, 1.0);
    Audio::Mono(synth(1.0, |t| {
        let palm = 0.5 * slap.bandpass(rng.white()) * decay(t, 0.003);
        tanh(1.2 * (ring(&mut modes, t) + palm))
    }))
}

/// A tambourine hit: the jingles land three times in quick succession and
/// then ring out.
pub fn tambourine() -> Audio {
    let mut metal = Metal::new(587.0);
    let mut rng = Rng::new(0x76);
    let mut floor = Svf::new(6_500.0, 0.7);
    let mut peak = Svf::new(9_500.0, 1.5);
    Audio::Mono(synth(1.0, |t| {
        let bright = floor.highpass(metal.tick(0.6) + 0.5 * rng.white());
        let jingles = 0.7 * burst(t, 0.0, 0.01)
            + 0.5 * burst(t, 0.017, 0.012)
            + 0.6 * burst(t, 0.036, 0.015)
            + 0.4 * decay_to(t, 0.09, 0.3);
        (bright + 0.6 * peak.bandpass(bright)) * jingles
    }))
}
