//! A state-variable filter that stays stable and in tune while its cutoff
//! and resonance move every sample, and the one-pole filter built the same
//! way.
//!
//! Both use trapezoidal integration with the delay-free loop solved
//! (Vadim Zavalishin, "The Art of VA Filter Design"; the update equations
//! are the form Andrew Simper published in "Solving the continuous SVF
//! equations using trapezoidal integration and equivalent currents").

use std::f32::consts::PI;

use super::math::flush;

/// The integrator gain `g = tan(pi * cutoff / sample_rate)` both filters are
/// tuned with. The cutoff is held below half the sample rate.
#[inline]
pub fn cutoff_gain(cutoff_hz: f32, sample_rate: f32) -> f32 {
    let normalized = (cutoff_hz / sample_rate.max(1.0)).clamp(1.0e-5, 0.49);
    (PI * normalized).tan()
}

/// Coefficients of a [`Svf`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SvfCoeffs {
    /// Damping, the inverse of Q.
    pub k: f32,
    a1: f32,
    a2: f32,
    a3: f32,
}

impl SvfCoeffs {
    /// Coefficients for a cutoff in Hz and a Q. A Q of 0.707 gives no
    /// resonant peak.
    pub fn new(cutoff_hz: f32, q: f32, sample_rate: f32) -> Self {
        Self::from_gain(cutoff_gain(cutoff_hz, sample_rate), 1.0 / q.max(0.05))
    }

    /// Coefficients from an integrator gain (see [`cutoff_gain`]) and a
    /// damping `k = 1 / Q`. Cheap enough to call per sample.
    #[inline]
    pub fn from_gain(g: f32, k: f32) -> Self {
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        Self {
            k,
            a1,
            a2,
            a3: g * a2,
        }
    }
}

/// The three outputs a [`Svf`] produces at once.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SvfOutput {
    pub low: f32,
    /// Band-pass with a peak gain of Q. Multiply by `k` for unity gain.
    pub band: f32,
    pub high: f32,
}

/// A 12 dB per octave state-variable filter.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Svf {
    ic1: f32,
    ic2: f32,
}

impl Svf {
    /// Filters one sample and returns all three responses.
    #[inline]
    pub fn tick(&mut self, coeffs: &SvfCoeffs, input: f32) -> SvfOutput {
        let v3 = input - self.ic2;
        let v1 = coeffs.a1 * self.ic1 + coeffs.a2 * v3;
        let v2 = self.ic2 + coeffs.a2 * self.ic1 + coeffs.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        SvfOutput {
            low: v2,
            band: v1,
            high: input - coeffs.k * v1 - v2,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Zeroes state that has decayed below hearing.
    pub fn flush(&mut self) {
        self.ic1 = flush(self.ic1);
        self.ic2 = flush(self.ic2);
    }
}

/// A 6 dB per octave filter.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct OnePoleFilter {
    state: f32,
}

impl OnePoleFilter {
    /// The coefficient for [`OnePoleFilter::low_pass`] and
    /// [`OnePoleFilter::high_pass`] at a cutoff in Hz.
    #[inline]
    pub fn coefficient(cutoff_hz: f32, sample_rate: f32) -> f32 {
        let g = cutoff_gain(cutoff_hz, sample_rate);
        g / (1.0 + g)
    }

    #[inline]
    pub fn low_pass(&mut self, coefficient: f32, input: f32) -> f32 {
        let v = (input - self.state) * coefficient;
        let low = v + self.state;
        self.state = low + v;
        low
    }

    #[inline]
    pub fn high_pass(&mut self, coefficient: f32, input: f32) -> f32 {
        input - self.low_pass(coefficient, input)
    }

    pub fn reset(&mut self) {
        self.state = 0.0;
    }

    pub fn flush(&mut self) {
        self.state = flush(self.state);
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_1_SQRT_2, TAU};

    use super::*;

    /// Steady-state gains of the three outputs at one frequency.
    fn measured(coeffs: &SvfCoeffs, frequency: f32, rate: f32) -> [f32; 3] {
        let mut filter = Svf::default();
        let mut power = [0.0_f64; 4];
        let total = (rate as usize) * 2;
        for n in 0..total {
            let input = (TAU * frequency * n as f32 / rate).sin();
            let out = filter.tick(coeffs, input);
            if n >= total / 2 {
                power[0] += f64::from(input * input);
                power[1] += f64::from(out.low * out.low);
                power[2] += f64::from(out.band * out.band);
                power[3] += f64::from(out.high * out.high);
            }
        }
        [1, 2, 3].map(|index| (power[index] / power[0]).sqrt() as f32)
    }

    fn db(gain: f32) -> f32 {
        20.0 * gain.log10()
    }

    #[test]
    fn corner_is_three_decibels_down_at_every_sample_rate() {
        for rate in [44_100.0, 48_000.0, 96_000.0] {
            for cutoff in [100.0, 1_000.0, 8_000.0] {
                let coeffs = SvfCoeffs::new(cutoff, FRAC_1_SQRT_2, rate);
                let [low, band, high] = measured(&coeffs, cutoff, rate);
                assert!(
                    (db(low) + 3.01).abs() < 0.05,
                    "low {low} at {cutoff}/{rate}"
                );
                assert!((db(high) + 3.01).abs() < 0.05, "high {high}");
                // The band output peaks at Q.
                assert!((band - FRAC_1_SQRT_2).abs() < 0.01, "band {band}");
            }
        }
    }

    #[test]
    fn slopes_are_twelve_decibels_per_octave() {
        let coeffs = SvfCoeffs::new(500.0, FRAC_1_SQRT_2, 48_000.0);
        let [low_far, _, _] = measured(&coeffs, 4_000.0, 48_000.0);
        let [low_farther, _, _] = measured(&coeffs, 8_000.0, 48_000.0);
        // Frequency warping steepens the top octaves slightly.
        let slope = db(low_farther) - db(low_far);
        assert!((-14.5..-11.5).contains(&slope), "{slope}");
        let [_, _, high_far] = measured(&coeffs, 62.5, 48_000.0);
        let [_, _, high_farther] = measured(&coeffs, 31.25, 48_000.0);
        assert!((db(high_farther) - db(high_far) + 12.0).abs() < 0.3);
    }

    #[test]
    fn resonance_peaks_at_q() {
        let coeffs = SvfCoeffs::new(1_000.0, 10.0, 48_000.0);
        let [low, band, _] = measured(&coeffs, 1_000.0, 48_000.0);
        assert!((low - 10.0).abs() < 0.2, "{low}");
        assert!((band - 10.0).abs() < 0.2, "{band}");
    }

    #[test]
    fn stays_bounded_when_cutoff_and_resonance_jump_every_sample() {
        let mut filter = Svf::default();
        let mut seed = 0x9E37_79B9_u32;
        let mut noise = move || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed as f32 / u32::MAX as f32
        };
        for rate in [22_050.0, 48_000.0, 192_000.0] {
            filter.reset();
            let mut loudest = 0.0_f32;
            for _ in 0..200_000 {
                let cutoff = 10.0 * (2_400.0_f32).powf(noise());
                let q = 0.5 + 40.0 * noise();
                let coeffs = SvfCoeffs::new(cutoff, q, rate);
                let out = filter.tick(&coeffs, noise() * 2.0 - 1.0);
                assert!(out.low.is_finite() && out.band.is_finite() && out.high.is_finite());
                loudest = loudest.max(out.low.abs()).max(out.band.abs());
            }
            assert!(loudest < 500.0, "{loudest} at {rate}");
        }
    }

    #[test]
    fn one_pole_corner_is_three_decibels_down() {
        for rate in [44_100.0, 96_000.0] {
            let coefficient = OnePoleFilter::coefficient(2_000.0, rate);
            let (mut low, mut high) = (OnePoleFilter::default(), OnePoleFilter::default());
            let mut power = [0.0_f64; 3];
            let total = rate as usize;
            for n in 0..total {
                let input = (TAU * 2_000.0 * n as f32 / rate).sin();
                let l = low.low_pass(coefficient, input);
                let h = high.high_pass(coefficient, input);
                if n >= total / 2 {
                    power[0] += f64::from(input * input);
                    power[1] += f64::from(l * l);
                    power[2] += f64::from(h * h);
                }
            }
            let low_db = 10.0 * (power[1] / power[0]).log10();
            let high_db = 10.0 * (power[2] / power[0]).log10();
            assert!((low_db + 3.01).abs() < 0.05, "{low_db}");
            assert!((high_db + 3.01).abs() < 0.05, "{high_db}");
        }
    }

    #[test]
    fn one_pole_passes_and_blocks_dc() {
        let coefficient = OnePoleFilter::coefficient(100.0, 48_000.0);
        let (mut low, mut high) = (OnePoleFilter::default(), OnePoleFilter::default());
        let (mut l, mut h) = (0.0, 0.0);
        for _ in 0..48_000 {
            l = low.low_pass(coefficient, 1.0);
            h = high.high_pass(coefficient, 1.0);
        }
        assert!((l - 1.0).abs() < 1e-4);
        assert!(h.abs() < 1e-4);
    }

    #[test]
    fn flush_zeroes_a_dead_tail() {
        let coeffs = SvfCoeffs::new(100.0, 10.0, 48_000.0);
        let mut filter = Svf::default();
        filter.tick(&coeffs, 1.0e-25);
        filter.flush();
        assert_eq!(filter, Svf::default());
    }
}
