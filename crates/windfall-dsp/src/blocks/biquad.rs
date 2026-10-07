//! Second-order filter sections.
//!
//! The coefficient formulas are the ones in Robert Bristow-Johnson's "Audio
//! EQ Cookbook". Coefficients and state are double precision: at 20 Hz and
//! a 96 kHz sample rate a single precision direct form misplaces its poles
//! by several percent.

use std::f64::consts::TAU;

use super::math::flush64;

/// Lowest and highest Q a section is built with.
const Q_RANGE: (f64, f64) = (0.025, 40.0);

/// The five coefficients of one section, already divided by `a0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiquadCoeffs {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
}

/// The terms every cookbook formula starts from.
struct Prototype {
    cos: f64,
    alpha: f64,
}

impl Prototype {
    fn new(frequency_hz: f32, q: f32, sample_rate: f32) -> Self {
        let sample_rate = f64::from(sample_rate).max(1.0);
        // The formulas break down at 0 Hz and at half the sample rate.
        let frequency = f64::from(frequency_hz).clamp(1.0, 0.49 * sample_rate);
        let q = f64::from(q).clamp(Q_RANGE.0, Q_RANGE.1);
        let omega = TAU * frequency / sample_rate;
        Self {
            cos: omega.cos(),
            alpha: omega.sin() / (2.0 * q),
        }
    }
}

impl BiquadCoeffs {
    /// A section that passes its input through unchanged.
    pub const IDENTITY: Self = Self {
        b0: 1.0,
        b1: 0.0,
        b2: 0.0,
        a1: 0.0,
        a2: 0.0,
    };

    fn normalized(b: [f64; 3], a: [f64; 3]) -> Self {
        let scale = 1.0 / a[0];
        Self {
            b0: b[0] * scale,
            b1: b[1] * scale,
            b2: b[2] * scale,
            a1: a[1] * scale,
            a2: a[2] * scale,
        }
    }

    /// Passes everything below `frequency_hz` and falls 12 dB per octave
    /// above it. A `q` of 0.707 gives the flattest passband; higher values
    /// add a peak at the corner.
    pub fn low_pass(frequency_hz: f32, q: f32, sample_rate: f32) -> Self {
        let p = Prototype::new(frequency_hz, q, sample_rate);
        let edge = (1.0 - p.cos) * 0.5;
        Self::normalized(
            [edge, 1.0 - p.cos, edge],
            [1.0 + p.alpha, -2.0 * p.cos, 1.0 - p.alpha],
        )
    }

    /// Passes everything above `frequency_hz` and falls 12 dB per octave
    /// below it.
    pub fn high_pass(frequency_hz: f32, q: f32, sample_rate: f32) -> Self {
        let p = Prototype::new(frequency_hz, q, sample_rate);
        let edge = (1.0 + p.cos) * 0.5;
        Self::normalized(
            [edge, -(1.0 + p.cos), edge],
            [1.0 + p.alpha, -2.0 * p.cos, 1.0 - p.alpha],
        )
    }

    /// Passes a band around `frequency_hz` at unity gain and attenuates
    /// everything else. Higher `q` gives a narrower band.
    pub fn band_pass(frequency_hz: f32, q: f32, sample_rate: f32) -> Self {
        let p = Prototype::new(frequency_hz, q, sample_rate);
        Self::normalized(
            [p.alpha, 0.0, -p.alpha],
            [1.0 + p.alpha, -2.0 * p.cos, 1.0 - p.alpha],
        )
    }

    /// Removes `frequency_hz` and leaves the rest. Higher `q` gives a
    /// narrower notch.
    pub fn notch(frequency_hz: f32, q: f32, sample_rate: f32) -> Self {
        let p = Prototype::new(frequency_hz, q, sample_rate);
        Self::normalized(
            [1.0, -2.0 * p.cos, 1.0],
            [1.0 + p.alpha, -2.0 * p.cos, 1.0 - p.alpha],
        )
    }

    /// Boosts or cuts a bell-shaped band around `frequency_hz` by
    /// `gain_db`. At 0 dB it is exactly [`BiquadCoeffs::IDENTITY`] in
    /// effect.
    pub fn peak(frequency_hz: f32, q: f32, gain_db: f32, sample_rate: f32) -> Self {
        let p = Prototype::new(frequency_hz, q, sample_rate);
        let a = amplitude(gain_db);
        Self::normalized(
            [1.0 + p.alpha * a, -2.0 * p.cos, 1.0 - p.alpha * a],
            [1.0 + p.alpha / a, -2.0 * p.cos, 1.0 - p.alpha / a],
        )
    }

    /// Boosts or cuts everything below `frequency_hz` by `gain_db`. A `q`
    /// of 0.707 gives a shelf with no overshoot.
    pub fn low_shelf(frequency_hz: f32, q: f32, gain_db: f32, sample_rate: f32) -> Self {
        let p = Prototype::new(frequency_hz, q, sample_rate);
        let a = amplitude(gain_db);
        let root = 2.0 * a.sqrt() * p.alpha;
        Self::normalized(
            [
                a * ((a + 1.0) - (a - 1.0) * p.cos + root),
                2.0 * a * ((a - 1.0) - (a + 1.0) * p.cos),
                a * ((a + 1.0) - (a - 1.0) * p.cos - root),
            ],
            [
                (a + 1.0) + (a - 1.0) * p.cos + root,
                -2.0 * ((a - 1.0) + (a + 1.0) * p.cos),
                (a + 1.0) + (a - 1.0) * p.cos - root,
            ],
        )
    }

    /// Boosts or cuts everything above `frequency_hz` by `gain_db`.
    pub fn high_shelf(frequency_hz: f32, q: f32, gain_db: f32, sample_rate: f32) -> Self {
        let p = Prototype::new(frequency_hz, q, sample_rate);
        let a = amplitude(gain_db);
        let root = 2.0 * a.sqrt() * p.alpha;
        Self::normalized(
            [
                a * ((a + 1.0) + (a - 1.0) * p.cos + root),
                -2.0 * a * ((a - 1.0) + (a + 1.0) * p.cos),
                a * ((a + 1.0) + (a - 1.0) * p.cos - root),
            ],
            [
                (a + 1.0) - (a - 1.0) * p.cos + root,
                2.0 * ((a - 1.0) - (a + 1.0) * p.cos),
                (a + 1.0) - (a - 1.0) * p.cos - root,
            ],
        )
    }

    /// Gain of the section at `frequency_hz` as a linear factor.
    pub fn magnitude(&self, frequency_hz: f32, sample_rate: f32) -> f64 {
        let omega = TAU * f64::from(frequency_hz) / f64::from(sample_rate).max(1.0);
        // Written in terms of sin^2(omega / 2). The textbook form in
        // cos(omega) subtracts nearly equal numbers at low frequencies and
        // loses most of its digits there.
        let phi = (omega * 0.5).sin().powi(2);
        let power = |c0: f64, c1: f64, c2: f64| {
            let sum = c0 + c1 + c2;
            sum * sum - 4.0 * (c0 * c1 + 4.0 * c0 * c2 + c1 * c2) * phi + 16.0 * c0 * c2 * phi * phi
        };
        let numerator = power(self.b0, self.b1, self.b2);
        let denominator = power(1.0, self.a1, self.a2);
        (numerator.max(0.0) / denominator.max(1.0e-300)).sqrt()
    }

    /// True when both poles lie inside the unit circle.
    pub fn is_stable(&self) -> bool {
        self.a2.abs() < 1.0 && self.a1.abs() < 1.0 + self.a2
    }

    fn scaled_difference(&self, from: &Self, scale: f64) -> Self {
        Self {
            b0: (self.b0 - from.b0) * scale,
            b1: (self.b1 - from.b1) * scale,
            b2: (self.b2 - from.b2) * scale,
            a1: (self.a1 - from.a1) * scale,
            a2: (self.a2 - from.a2) * scale,
        }
    }

    fn add(&mut self, step: &Self) {
        self.b0 += step.b0;
        self.b1 += step.b1;
        self.b2 += step.b2;
        self.a1 += step.a1;
        self.a2 += step.a2;
    }
}

/// The cookbook's `A`: the square root of the linear gain.
fn amplitude(gain_db: f32) -> f64 {
    10.0_f64.powf(f64::from(gain_db) / 40.0)
}

/// The memory of one section, in transposed direct form II.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Biquad {
    s1: f64,
    s2: f64,
}

impl Biquad {
    /// Filters one sample.
    #[inline]
    pub fn tick(&mut self, coeffs: &BiquadCoeffs, input: f64) -> f64 {
        let output = coeffs.b0 * input + self.s1;
        self.s1 = coeffs.b1 * input - coeffs.a1 * output + self.s2;
        self.s2 = coeffs.b2 * input - coeffs.a2 * output;
        output
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Zeroes state that has decayed below hearing. Call it now and then,
    /// not per sample.
    pub fn flush(&mut self) {
        self.s1 = flush64(self.s1);
        self.s2 = flush64(self.s2);
    }
}

/// Coefficients that move to a new set in a straight line.
///
/// The stable coefficients of a second-order section form a convex region
/// (the triangle `|a2| < 1`, `|a1| < 1 + a2`), so every set on the line
/// between two stable sets is stable too. That is what keeps a fast sweep
/// from blowing up, however far apart its two ends are.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoeffRamp {
    current: BiquadCoeffs,
    target: BiquadCoeffs,
    step: BiquadCoeffs,
    remaining: u32,
}

impl CoeffRamp {
    pub fn new(coeffs: BiquadCoeffs) -> Self {
        Self {
            current: coeffs,
            target: coeffs,
            step: BiquadCoeffs::IDENTITY,
            remaining: 0,
        }
    }

    pub fn snap(&mut self, coeffs: BiquadCoeffs) {
        *self = Self::new(coeffs);
    }

    /// Starts moving to `target`, arriving after `samples` ticks.
    pub fn set_target(&mut self, target: BiquadCoeffs, samples: u32) {
        if samples == 0 {
            self.snap(target);
            return;
        }
        self.target = target;
        self.step = target.scaled_difference(&self.current, 1.0 / f64::from(samples));
        self.remaining = samples;
    }

    /// Advances one sample and returns the coefficients to use for it.
    #[inline]
    pub fn tick(&mut self) -> &BiquadCoeffs {
        if self.remaining > 0 {
            self.remaining -= 1;
            if self.remaining == 0 {
                self.current = self.target;
            } else {
                self.current.add(&self.step);
            }
        }
        &self.current
    }

    #[inline]
    pub fn current(&self) -> &BiquadCoeffs {
        &self.current
    }

    #[inline]
    pub fn is_settled(&self) -> bool {
        self.remaining == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: f32 = 48_000.0;

    /// Gain of a filter at one frequency, measured by running a sine
    /// through it and comparing levels once it has settled.
    fn measured_gain(coeffs: &BiquadCoeffs, frequency: f32) -> f64 {
        let mut filter = Biquad::default();
        let (mut input_power, mut output_power) = (0.0, 0.0);
        for n in 0..96_000 {
            let input = (TAU * f64::from(frequency) * f64::from(n) / f64::from(RATE)).sin();
            let output = filter.tick(coeffs, input);
            if n >= 48_000 {
                input_power += input * input;
                output_power += output * output;
            }
        }
        (output_power / input_power).sqrt()
    }

    fn db(gain: f64) -> f64 {
        20.0 * gain.log10()
    }

    #[test]
    fn measured_response_matches_the_formula() {
        let filters = [
            BiquadCoeffs::low_pass(1_000.0, 0.707, RATE),
            BiquadCoeffs::high_pass(300.0, 1.5, RATE),
            BiquadCoeffs::band_pass(2_000.0, 4.0, RATE),
            BiquadCoeffs::notch(5_000.0, 2.0, RATE),
            BiquadCoeffs::peak(800.0, 2.0, 9.0, RATE),
            BiquadCoeffs::low_shelf(200.0, 0.707, -6.0, RATE),
            BiquadCoeffs::high_shelf(6_000.0, 0.707, 12.0, RATE),
        ];
        for coeffs in &filters {
            for frequency in [50.0, 300.0, 1_000.0, 2_500.0, 9_000.0, 18_000.0] {
                let expected = coeffs.magnitude(frequency, RATE);
                let measured = measured_gain(coeffs, frequency);
                if expected > 1.0e-3 {
                    assert!(
                        (db(measured) - db(expected)).abs() < 0.02,
                        "{coeffs:?} at {frequency} Hz: {measured} against {expected}"
                    );
                }
            }
        }
    }

    #[test]
    fn corner_and_centre_gains_are_the_textbook_ones() {
        let low = BiquadCoeffs::low_pass(1_000.0, std::f32::consts::FRAC_1_SQRT_2, RATE);
        assert!((db(low.magnitude(1_000.0, RATE)) + 3.0103).abs() < 0.01);
        assert!(db(low.magnitude(10.0, RATE)).abs() < 0.01);
        // Two octaves up a second-order low-pass is close to 24 dB down.
        assert!((db(low.magnitude(4_000.0, RATE)) + 24.4).abs() < 0.6);

        let high = BiquadCoeffs::high_pass(1_000.0, std::f32::consts::FRAC_1_SQRT_2, RATE);
        assert!((db(high.magnitude(1_000.0, RATE)) + 3.0103).abs() < 0.01);
        assert!(db(high.magnitude(20_000.0, RATE)).abs() < 0.05);

        let peak = BiquadCoeffs::peak(1_000.0, 1.0, 6.0, RATE);
        assert!((db(peak.magnitude(1_000.0, RATE)) - 6.0).abs() < 0.001);
        assert!(db(peak.magnitude(20.0, RATE)).abs() < 0.05);

        let low_shelf = BiquadCoeffs::low_shelf(500.0, 0.707, 8.0, RATE);
        assert!((db(low_shelf.magnitude(5.0, RATE)) - 8.0).abs() < 0.01);
        assert!((db(low_shelf.magnitude(500.0, RATE)) - 4.0).abs() < 0.05);
        assert!(db(low_shelf.magnitude(20_000.0, RATE)).abs() < 0.01);

        let high_shelf = BiquadCoeffs::high_shelf(2_000.0, 0.707, -10.0, RATE);
        assert!((db(high_shelf.magnitude(23_900.0, RATE)) + 10.0).abs() < 0.01);
        assert!(db(high_shelf.magnitude(5.0, RATE)).abs() < 0.01);

        let band = BiquadCoeffs::band_pass(1_000.0, 5.0, RATE);
        assert!(db(band.magnitude(1_000.0, RATE)).abs() < 0.001);
        let notch = BiquadCoeffs::notch(1_000.0, 5.0, RATE);
        assert!(notch.magnitude(1_000.0, RATE) < 1.0e-6);
    }

    #[test]
    fn a_flat_peak_or_shelf_changes_nothing() {
        for coeffs in [
            BiquadCoeffs::peak(1_000.0, 3.0, 0.0, RATE),
            BiquadCoeffs::low_shelf(100.0, 0.707, 0.0, RATE),
            BiquadCoeffs::high_shelf(8_000.0, 0.707, 0.0, RATE),
        ] {
            let mut filter = Biquad::default();
            for n in 0..1_000 {
                let input = f64::from(n % 17) / 17.0 - 0.5;
                assert!((filter.tick(&coeffs, input) - input).abs() < 1.0e-12);
            }
        }
    }

    #[test]
    fn every_corner_of_the_parameter_range_is_stable() {
        for rate in [22_050.0, 44_100.0, 48_000.0, 96_000.0, 192_000.0] {
            for frequency in [0.0, 1.0, 20.0, 1_000.0, 20_000.0, 1.0e6] {
                for q in [0.0, 0.025, 0.707, 40.0, 1.0e6] {
                    for gain in [-48.0, 0.0, 48.0] {
                        for coeffs in [
                            BiquadCoeffs::low_pass(frequency, q, rate),
                            BiquadCoeffs::high_pass(frequency, q, rate),
                            BiquadCoeffs::band_pass(frequency, q, rate),
                            BiquadCoeffs::notch(frequency, q, rate),
                            BiquadCoeffs::peak(frequency, q, gain, rate),
                            BiquadCoeffs::low_shelf(frequency, q, gain, rate),
                            BiquadCoeffs::high_shelf(frequency, q, gain, rate),
                        ] {
                            assert!(coeffs.is_stable(), "{coeffs:?}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn low_frequencies_stay_accurate_at_high_sample_rates() {
        let coeffs = BiquadCoeffs::peak(20.0, 4.0, 12.0, 192_000.0);
        assert!((db(coeffs.magnitude(20.0, 192_000.0)) - 12.0).abs() < 0.001);
        let mut filter = Biquad::default();
        let (mut input_power, mut output_power) = (0.0, 0.0);
        for n in 0..(192_000 * 4) {
            let input = (TAU * 20.0 * f64::from(n) / 192_000.0).sin();
            let output = filter.tick(&coeffs, input);
            if n >= 192_000 * 2 {
                input_power += input * input;
                output_power += output * output;
            }
        }
        assert!((db((output_power / input_power).sqrt()) - 12.0).abs() < 0.02);
    }

    #[test]
    fn sweeping_between_extremes_stays_bounded() {
        // A narrow, loud peak thrown back and forth across the spectrum
        // every few samples, with noise going through it.
        let mut ramp = CoeffRamp::new(BiquadCoeffs::peak(30.0, 30.0, 24.0, RATE));
        let mut filter = Biquad::default();
        let mut seed = 0x1234_5678_u32;
        let mut loudest = 0.0_f64;
        for n in 0..200_000_u32 {
            if n % 16 == 0 {
                let low = (n / 16) % 2 == 0;
                let target = if low {
                    BiquadCoeffs::peak(25.0 + (n % 97) as f32, 35.0, 24.0, RATE)
                } else {
                    BiquadCoeffs::high_shelf(19_000.0 - (n % 89) as f32, 2.0, -24.0, RATE)
                };
                ramp.set_target(target, 16);
            }
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let input = f64::from(seed) / f64::from(u32::MAX) * 2.0 - 1.0;
            let coeffs = *ramp.tick();
            assert!(coeffs.is_stable());
            let output = filter.tick(&coeffs, input);
            assert!(output.is_finite());
            loudest = loudest.max(output.abs());
        }
        // The loudest steady gain either end can reach is 24 dB, about 16.
        assert!(loudest < 200.0, "{loudest}");
    }

    #[test]
    fn coefficient_ramp_lands_exactly_on_its_target() {
        let from = BiquadCoeffs::low_pass(200.0, 0.7, RATE);
        let to = BiquadCoeffs::low_pass(8_000.0, 2.0, RATE);
        let mut ramp = CoeffRamp::new(from);
        ramp.set_target(to, 16);
        for _ in 0..15 {
            ramp.tick();
            assert!(!ramp.is_settled());
        }
        assert_eq!(*ramp.tick(), to);
        assert!(ramp.is_settled());
    }

    #[test]
    fn flush_zeroes_a_dead_tail() {
        let coeffs = BiquadCoeffs::low_pass(100.0, 10.0, RATE);
        let mut filter = Biquad::default();
        filter.tick(&coeffs, 1.0e-24);
        filter.flush();
        assert_eq!(filter, Biquad::default());
    }
}
