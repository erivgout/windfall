//! Hand claps and a finger snap: band-passed noise shaped by several
//! bursts in a row, the way a group of hands never lands at once.

use crate::dsp::{Audio, Rng, Svf, decay, decay_to, mix_into, synth};
use crate::math::tanh;

/// Loudness of a clap at time `t`. Every entry of `bursts` restarts a fast
/// decay, and the last one rings out with the slower `tail` decay.
fn clap_envelope(t: f64, bursts: &[f64], tail: f64) -> f64 {
    let last = bursts[bursts.len() - 1];
    if t >= last {
        return decay_to(t - last, tail, 5.0 * tail);
    }
    let start = bursts.iter().rev().find(|&&start| t >= start);
    start.map_or(0.0, |start| decay(t - start, 0.004_5))
}

/// One group of hands: noise through a band around `center` hertz.
fn hands(seed: u64, center: f64, bursts: &[f64], tail: f64, seconds: f64) -> Vec<f64> {
    let mut rng = Rng::new(seed);
    let mut band = Svf::new(center, 1.8);
    let mut bite = Svf::new(center * 2.7, 1.0);
    let mut floor = Svf::new(500.0, 0.7);
    let mut ceiling = Svf::new(8_000.0, 0.7);
    synth(seconds, |t| {
        let noise = ceiling.lowpass(floor.highpass(rng.white()));
        (band.bandpass(noise) + 0.35 * bite.bandpass(noise)) * clap_envelope(t, bursts, tail)
    })
}

/// Clips the loudest noise peaks of a clap. Noise has a few stray peaks far
/// above its average level, and without this they alone would set how loud
/// the finished file can be.
fn saturate(samples: &mut [f64], drive: f64) {
    for sample in samples {
        *sample = tanh(drive * *sample);
    }
}

/// A stereo clap: one group of hands in the middle and one on each side,
/// each with its own timing, and a tail long enough to sound like a room.
pub fn wide() -> Audio {
    let seconds = 1.0;
    let center = hands(0x31, 1_250.0, &[0.0, 0.010, 0.021, 0.031], 0.075, seconds);
    let mut left = hands(0x32, 1_120.0, &[0.0, 0.009, 0.020, 0.030], 0.09, seconds);
    let mut right = hands(0x33, 1_400.0, &[0.0, 0.011, 0.022, 0.033], 0.09, seconds);
    for side in [&mut left, &mut right] {
        for sample in side.iter_mut() {
            *sample *= 0.7;
        }
        mix_into(side, &center, 1.0);
        saturate(side, 1.5);
    }
    Audio::Stereo(left, right)
}

/// A dry mono clap with three quick bursts and almost no tail.
pub fn tight() -> Audio {
    let mut clap = hands(0x34, 1_500.0, &[0.0, 0.008, 0.017], 0.03, 0.5);
    saturate(&mut clap, 4.0);
    Audio::Mono(clap)
}

/// A finger snap: a tick, then a short ring from two narrow bands of noise.
pub fn snap() -> Audio {
    let mut rng = Rng::new(0x35);
    let mut tick = Svf::new(5_000.0, 0.7);
    let mut low = Svf::new(2_350.0, 3.5);
    let mut high = Svf::new(4_100.0, 3.0);
    Audio::Mono(synth(0.4, |t| {
        let noise = rng.white();
        let ring = low.bandpass(noise) + 0.6 * high.bandpass(noise);
        let amp = decay_to(t, 0.013, 0.06) + 0.12 * decay_to(t, 0.05, 0.15);
        1.5 * tick.highpass(noise) * decay(t, 0.000_6) + ring * amp
    }))
}
