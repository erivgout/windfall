//! Cymbals, in stereo. Both channels share most of their sound and differ
//! in noise and a slight detune, so they stay solid when summed to mono.

use crate::dsp::{Audio, Metal, Mode, Rng, Svf, decay, decay_to, ring, synth};

/// A crash: a burst of noise and clashing squares split into three bands,
/// with the treble dying away first as it does on real brass.
pub fn crash() -> Audio {
    let side = |seed: u64, detune: f64| {
        let mut shared = Metal::new(317.0);
        let mut own = Metal::new(411.0 * detune);
        let mut shared_noise = Rng::new(0x50);
        let mut own_noise = Rng::new(seed);
        let mut floor = Svf::new(2_400.0, 0.7);
        let mut low = Svf::new(3_400.0, 0.6);
        let mut mid = Svf::new(6_500.0, 0.6);
        let mut high = Svf::new(10_500.0, 0.8);
        let mut strike = Svf::new(4_000.0, 0.7);
        synth(4.0, |t| {
            let noise = 0.75 * shared_noise.white() + 0.65 * own_noise.white();
            let source = floor.highpass(shared.tick(0.6) + 0.6 * own.tick(0.8) + 1.6 * noise);
            let wash = 0.55 * low.bandpass(source) * decay_to(t, 0.7, 1.4)
                + mid.bandpass(source) * decay_to(t, 0.5, 1.4)
                + 1.3 * high.bandpass(source) * decay_to(t, 0.33, 1.4);
            wash + 1.6 * strike.highpass(noise) * decay(t, 0.012)
        })
    };
    Audio::Stereo(side(0x51, 0.996), side(0x52, 1.004))
}

/// A ride: the ping of the stick as a handful of high ringing tones, a
/// lower bell under it, and a quiet metallic wash.
pub fn ride() -> Audio {
    let side = |seed: u64, detune: f64| {
        let mut tones = [
            (1.0, Mode::new(2_870.0 * detune, 0.3)),
            (0.8, Mode::new(4_115.0, 0.24)),
            (0.7, Mode::new(5_390.0 * detune, 0.19)),
            (0.6, Mode::new(7_260.0, 0.13)),
            (0.25, Mode::new(437.0, 0.36)),
            (0.3, Mode::new(683.0 * detune, 0.34)),
            (0.35, Mode::new(1_012.0, 0.32)),
            (0.4, Mode::new(1_561.0 * detune, 0.3)),
        ];
        let mut metal = Metal::new(283.0 * detune);
        let mut shared = Metal::new(349.0);
        let mut rng = Rng::new(seed);
        let mut floor = Svf::new(3_000.0, 0.7);
        let mut wash = Svf::new(7_000.0, 0.8);
        let mut tick = Svf::new(6_000.0, 0.7);
        synth(3.0, |t| {
            let noise = rng.white();
            let cluster = shared.tick(0.2) + 0.6 * metal.tick(0.6) + 0.5 * noise;
            let shimmer = wash.bandpass(floor.highpass(cluster));
            0.2 * ring(&mut tones, t)
                + 0.6 * shimmer * decay_to(t, 0.4, 1.1)
                + 0.8 * tick.highpass(noise) * decay(t, 0.002)
        })
    };
    Audio::Stereo(side(0x53, 0.997), side(0x54, 1.003))
}
