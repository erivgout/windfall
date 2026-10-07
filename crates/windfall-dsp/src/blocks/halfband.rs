//! Halves a sample rate.

/// Non-zero taps on each side of the centre tap.
const SIDE_TAPS: usize = 12;

/// Distance from the newest input sample used to the centre of the filter.
const CENTRE: usize = 2 * SIDE_TAPS;

const RING: usize = 64;

/// Shape of the Kaiser window. 7 puts the stopband about 70 dB down.
const KAISER_BETA: f64 = 7.0;

/// A filter that turns a signal sampled at twice the target rate into one
/// at the target rate, removing everything that would not fit.
///
/// It is a linear-phase half-band filter: a windowed sinc whose cutoff sits
/// at a quarter of the input rate, which makes every second tap zero and
/// halves the work. Running an oscillator or a distortion at twice the rate
/// and coming down through this filter is what keeps their aliasing out of
/// the audible band.
///
/// The passband is flat to about 0.4 of the output rate (19 kHz at 48 kHz)
/// and the stopband starts at about 0.6.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HalfbandDecimator {
    taps: [f32; SIDE_TAPS],
    ring: [f32; RING],
    write: usize,
}

/// The zeroth-order modified Bessel function of the first kind, by its
/// power series.
fn bessel_i0(x: f64) -> f64 {
    let mut sum = 1.0;
    let mut term = 1.0;
    for k in 1..40 {
        term *= (x / (2.0 * f64::from(k))).powi(2);
        sum += term;
    }
    sum
}

impl Default for HalfbandDecimator {
    fn default() -> Self {
        let mut taps = [0.0_f64; SIDE_TAPS];
        for (index, tap) in taps.iter_mut().enumerate() {
            let offset = (2 * index + 1) as f64;
            let sinc =
                (std::f64::consts::FRAC_PI_2 * offset).sin() / (std::f64::consts::PI * offset);
            let position = offset / CENTRE as f64;
            let window = bessel_i0(KAISER_BETA * (1.0 - position * position).sqrt())
                / bessel_i0(KAISER_BETA);
            *tap = sinc * window;
        }
        // The centre tap is one half, so the side taps must add up to one
        // quarter on each side for a constant input to pass at unity.
        let scale = 0.25 / taps.iter().sum::<f64>();
        Self {
            taps: taps.map(|tap| (tap * scale) as f32),
            ring: [0.0; RING],
            write: 0,
        }
    }
}

impl HalfbandDecimator {
    /// Output samples by which the filter delays its signal.
    pub const LATENCY: usize = SIDE_TAPS;

    pub fn reset(&mut self) {
        self.ring = [0.0; RING];
        self.write = 0;
    }

    /// Takes the next two input samples and returns one output sample.
    #[inline]
    pub fn tick(&mut self, first: f32, second: f32) -> f32 {
        self.ring[self.write] = first;
        // Centring on an even input sample keeps the delay a whole number
        // of output samples.
        let centre = self.write.wrapping_sub(CENTRE);
        let mut output = 0.5 * self.ring[centre & (RING - 1)];
        for (index, tap) in self.taps.iter().enumerate() {
            let offset = 2 * index + 1;
            let before = self.ring[centre.wrapping_sub(offset) & (RING - 1)];
            let after = self.ring[centre.wrapping_add(offset) & (RING - 1)];
            output += tap * (before + after);
        }
        self.ring[(self.write + 1) & (RING - 1)] = second;
        self.write = (self.write + 2) & (RING - 1);
        output
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::TAU;

    use super::*;

    /// Level of the output, relative to the input, for a sine at
    /// `frequency` when the input rate is 96 kHz.
    fn gain_db(frequency: f64) -> f64 {
        let mut filter = HalfbandDecimator::default();
        let mut power = 0.0;
        let outputs = 8_000;
        for n in 0..outputs {
            let sample = |index: usize| (TAU * frequency * index as f64 / 96_000.0).sin() as f32;
            let output = filter.tick(sample(2 * n), sample(2 * n + 1));
            if n >= 200 {
                power += f64::from(output * output);
            }
        }
        10.0 * (power / (outputs - 200) as f64 / 0.5).log10()
    }

    #[test]
    fn passband_is_flat_and_stopband_is_deep() {
        for frequency in [50.0, 1_000.0, 10_000.0, 16_000.0, 19_000.0] {
            let gain = gain_db(frequency);
            assert!(gain.abs() < 0.1, "{frequency} Hz: {gain} dB");
        }
        // These would fold to 19, 14, 8 and 1 kHz.
        for frequency in [29_000.0, 34_000.0, 40_000.0, 47_000.0] {
            let gain = gain_db(frequency);
            assert!(gain < -65.0, "{frequency} Hz: {gain} dB");
        }
    }

    #[test]
    fn a_constant_passes_at_unity() {
        let mut filter = HalfbandDecimator::default();
        let mut output = 0.0;
        for _ in 0..100 {
            output = filter.tick(1.0, 1.0);
        }
        assert!((output - 1.0).abs() < 1e-6);
    }

    #[test]
    fn delay_is_a_whole_number_of_output_samples() {
        // A slow sine comes out the same, LATENCY output samples late.
        let mut filter = HalfbandDecimator::default();
        let sample = |index: usize| (TAU * 440.0 * index as f64 / 96_000.0).sin();
        for n in 0..2_000_usize {
            let output = filter.tick(sample(2 * n) as f32, sample(2 * n + 1) as f32);
            if n > 100 {
                let expected = sample(2 * (n - HalfbandDecimator::LATENCY));
                assert!((f64::from(output) - expected).abs() < 1e-4, "at {n}");
            }
        }
    }

    #[test]
    fn silence_in_gives_exact_silence_out_after_the_filter_empties() {
        let mut filter = HalfbandDecimator::default();
        filter.tick(1.0, -1.0);
        for _ in 0..RING {
            filter.tick(0.0, 0.0);
        }
        assert_eq!(filter.tick(0.0, 0.0), 0.0);
        filter.tick(0.5, 0.5);
        filter.reset();
        assert_eq!(filter.tick(0.0, 0.0), 0.0);
    }
}
