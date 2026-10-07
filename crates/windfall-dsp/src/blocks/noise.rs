//! Repeatable random numbers and the noise colours made from them.

/// A small, fast random number generator (George Marsaglia's 32-bit
/// xorshift). The same seed always gives the same sequence, which keeps
/// every render of a project identical.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rng(u32);

impl Rng {
    /// A generator for `seed`. Neighbouring seeds give unrelated sequences.
    pub fn new(seed: u32) -> Self {
        // Scramble the seed so that seeds 1, 2, 3 do not start alike, and
        // keep the state away from zero, where xorshift gets stuck.
        let mut state = seed.wrapping_add(0x9E37_79B9);
        state = (state ^ (state >> 16)).wrapping_mul(0x85EB_CA6B);
        state = (state ^ (state >> 13)).wrapping_mul(0xC2B2_AE35);
        state ^= state >> 16;
        Self(if state == 0 { 0x6D2B_79F5 } else { state })
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        let mut state = self.0;
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        self.0 = state;
        state
    }

    /// A value from 0 up to, but not including, 1.
    #[inline]
    pub fn unipolar(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 * (1.0 / 16_777_216.0)
    }

    /// A value from -1 up to, but not including, 1: one sample of white
    /// noise.
    #[inline]
    pub fn bipolar(&mut self) -> f32 {
        self.unipolar() * 2.0 - 1.0
    }
}

/// Rows of white noise a [`PinkNoise`] adds up.
const PINK_ROWS: usize = 12;

/// Brings the sum of the rows to about the loudness of white noise.
const PINK_SCALE: f32 = 0.2;

/// Pink noise: equal energy in every octave, 3 dB per octave darker than
/// white.
///
/// This is the Voss-McCartney method: a stack of white noise sources where
/// each one is refreshed half as often as the one before, added together.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PinkNoise {
    rows: [f32; PINK_ROWS],
    sum: f32,
    counter: u32,
}

impl Default for PinkNoise {
    fn default() -> Self {
        Self::new()
    }
}

impl PinkNoise {
    pub const fn new() -> Self {
        Self {
            rows: [0.0; PINK_ROWS],
            sum: 0.0,
            counter: 0,
        }
    }

    /// The next sample, drawing its randomness from `rng`.
    #[inline]
    pub fn tick(&mut self, rng: &mut Rng) -> f32 {
        self.counter = self.counter.wrapping_add(1);
        let row = self.counter.trailing_zeros() as usize;
        if row < PINK_ROWS {
            let fresh = rng.bipolar();
            self.sum += fresh - self.rows[row];
            self.rows[row] = fresh;
        } else {
            // Once per full cycle of the slowest row, add the rows up from
            // scratch so rounding in the running sum cannot build into an
            // offset.
            self.sum = self.rows.iter().sum();
        }
        (self.sum + rng.bipolar()) * PINK_SCALE
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;

    use super::*;

    #[test]
    fn the_same_seed_repeats_and_different_seeds_differ() {
        let (mut a, mut b, mut c) = (Rng::new(7), Rng::new(7), Rng::new(8));
        let mut differences = 0;
        for _ in 0..1_000 {
            let value = a.next_u32();
            assert_eq!(value, b.next_u32());
            if value != c.next_u32() {
                differences += 1;
            }
        }
        assert!(differences > 990);
    }

    #[test]
    fn white_noise_is_centred_and_fills_its_range() {
        let mut rng = Rng::new(1);
        let (mut sum, mut power) = (0.0_f64, 0.0_f64);
        let (mut low, mut high) = (0.0_f32, 0.0_f32);
        let count = 200_000;
        for _ in 0..count {
            let value = rng.bipolar();
            assert!((-1.0..1.0).contains(&value));
            sum += f64::from(value);
            power += f64::from(value * value);
            low = low.min(value);
            high = high.max(value);
        }
        assert!((sum / f64::from(count)).abs() < 0.01);
        // A uniform distribution over -1..1 has a variance of one third.
        assert!((power / f64::from(count) - 1.0 / 3.0).abs() < 0.01);
        assert!(low < -0.999 && high > 0.999);
        let mut unipolar = Rng::new(2);
        for _ in 0..10_000 {
            assert!((0.0..1.0).contains(&unipolar.unipolar()));
        }
    }

    /// Power of `signal` in a band around `frequency`, by correlating with
    /// a windowed sine and cosine at a handful of frequencies in the band.
    fn band_power(signal: &[f32], frequency: f32, rate: f32) -> f64 {
        let mut total = 0.0;
        for step in 0..24 {
            let probe = frequency * 2.0_f32.powf((step as f32 - 11.5) / 24.0);
            let (mut re, mut im) = (0.0_f64, 0.0_f64);
            for (n, sample) in signal.iter().enumerate() {
                let phase = f64::from(TAU) * f64::from(probe) * n as f64 / f64::from(rate);
                re += f64::from(*sample) * phase.cos();
                im += f64::from(*sample) * phase.sin();
            }
            total += re * re + im * im;
        }
        // Each probe measures a density. An octave band is as wide as its
        // centre frequency.
        total / 24.0 * f64::from(frequency)
    }

    #[test]
    fn white_noise_has_a_flat_spectrum() {
        let mut rng = Rng::new(3);
        let signal: Vec<f32> = (0..65_536).map(|_| rng.bipolar()).collect();
        let reference = band_power(&signal, 1_000.0, 48_000.0) / 1_000.0;
        for frequency in [125.0, 500.0, 4_000.0, 16_000.0] {
            let density = band_power(&signal, frequency, 48_000.0) / f64::from(frequency);
            let difference = 10.0 * (density / reference).log10();
            assert!(difference.abs() < 2.5, "{frequency} Hz: {difference} dB");
        }
    }

    #[test]
    fn pink_noise_has_equal_energy_per_octave() {
        let mut rng = Rng::new(4);
        let mut pink = PinkNoise::default();
        let signal: Vec<f32> = (0..131_072).map(|_| pink.tick(&mut rng)).collect();
        let reference = band_power(&signal, 1_000.0, 48_000.0);
        for frequency in [125.0, 250.0, 500.0, 2_000.0, 4_000.0, 8_000.0] {
            let difference = 10.0 * (band_power(&signal, frequency, 48_000.0) / reference).log10();
            assert!(difference.abs() < 3.0, "{frequency} Hz: {difference} dB");
        }
        // White noise measured the same way gains 3 dB per octave, so this
        // would read about +9 dB if the filter did nothing.
        let top = 10.0 * (band_power(&signal, 8_000.0, 48_000.0) / reference).log10();
        assert!(top < 3.0);
    }

    #[test]
    fn pink_noise_is_centred_and_about_as_loud_as_white() {
        let mut rng = Rng::new(5);
        let mut pink = PinkNoise::default();
        let (mut sum, mut power) = (0.0_f64, 0.0_f64);
        let count = 400_000;
        for _ in 0..count {
            let value = pink.tick(&mut rng);
            assert!(value.abs() < 2.7);
            sum += f64::from(value);
            power += f64::from(value * value);
        }
        assert!((sum / f64::from(count)).abs() < 0.05);
        let rms = (power / f64::from(count)).sqrt();
        assert!((0.3..0.6).contains(&rms), "{rms}");
    }
}
