//! Spectrum measurements for the tests. Nothing here affects the sounds, so
//! it uses the standard math functions.

use std::f64::consts::TAU;

/// How the energy of a sound is spread over frequency.
pub struct Spectrum {
    /// Power in each bin from 0 Hz to half the sample rate inclusive.
    power: Vec<f64>,
    /// Width of one bin in hertz.
    bin_hz: f64,
}

impl Spectrum {
    /// Measures `samples`, padded with silence to a power of two.
    pub fn of(samples: &[f64], sample_rate: f64) -> Self {
        let size = samples.len().next_power_of_two().max(2);
        let mut re = samples.to_vec();
        re.resize(size, 0.0);
        let mut im = vec![0.0; size];
        fft(&mut re, &mut im);
        let power = (0..=size / 2)
            .map(|bin| re[bin] * re[bin] + im[bin] * im[bin])
            .collect();
        Self {
            power,
            bin_hz: sample_rate / size as f64,
        }
    }

    /// Share of the energy that lies between `low` and `high` hertz, from 0
    /// to 1.
    pub fn share(&self, low: f64, high: f64) -> f64 {
        let total: f64 = self.power.iter().sum();
        let band: f64 = self
            .power
            .iter()
            .enumerate()
            .filter(|&(bin, _)| (low..high).contains(&(bin as f64 * self.bin_hz)))
            .map(|(_, power)| power)
            .sum();
        band / total
    }
}

/// In-place radix-2 fast Fourier transform. The length must be a power of
/// two.
fn fft(re: &mut [f64], im: &mut [f64]) {
    let size = re.len();
    assert!(size.is_power_of_two());
    let mut j = 0;
    for i in 1..size {
        let mut bit = size >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut span = 2;
    while span <= size {
        let angle = -TAU / span as f64;
        for start in (0..size).step_by(span) {
            for k in 0..span / 2 {
                let (sin, cos) = (angle * k as f64).sin_cos();
                let (a, b) = (start + k, start + k + span / 2);
                let (tr, ti) = (re[b] * cos - im[b] * sin, re[b] * sin + im[b] * cos);
                (re[b], im[b]) = (re[a] - tr, im[a] - ti);
                (re[a], im[a]) = (re[a] + tr, im[a] + ti);
            }
        }
        span *= 2;
    }
}

/// Power of `samples` at exactly `freq` hertz (the Goertzel algorithm).
pub fn power_at(samples: &[f64], sample_rate: f64, freq: f64) -> f64 {
    let coeff = 2.0 * (TAU * freq / sample_rate).cos();
    let (mut s1, mut s2) = (0.0, 0.0);
    for sample in samples {
        (s1, s2) = (sample + coeff * s1 - s2, s1);
    }
    s1 * s1 + s2 * s2 - coeff * s1 * s2
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(freq: f64, frames: usize) -> Vec<f64> {
        (0..frames)
            .map(|frame| (TAU * freq * frame as f64 / 48_000.0).sin())
            .collect()
    }

    #[test]
    fn share_finds_a_tone() {
        let spectrum = Spectrum::of(&tone(1_000.0, 10_000), 48_000.0);
        assert!(spectrum.share(900.0, 1_100.0) > 0.95);
        assert!(spectrum.share(2_000.0, 24_000.0) < 0.01);
        assert!((spectrum.share(0.0, 24_001.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn share_splits_two_tones_by_energy() {
        let mut wave = tone(200.0, 16_384);
        for (out, high) in wave.iter_mut().zip(tone(8_000.0, 16_384)) {
            *out += 2.0 * high;
        }
        // Twice the amplitude is four times the energy.
        let high = Spectrum::of(&wave, 48_000.0).share(4_000.0, 24_000.0);
        assert!((high - 0.8).abs() < 0.01, "{high}");
    }

    #[test]
    fn power_at_picks_out_one_frequency() {
        let wave = tone(440.0, 48_000);
        let on = power_at(&wave, 48_000.0, 440.0);
        assert!(on > 1_000.0 * power_at(&wave, 48_000.0, 466.16));
        assert!(on > 1_000.0 * power_at(&wave, 48_000.0, 415.3));
    }
}
