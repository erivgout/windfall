//! A real FFT whose bins sit half way between the usual ones.
//!
//! Bin `k` of a block of `size` samples is at `(k + 0.5) / size` cycles per
//! sample. There is no bin at 0 Hz and none at the Nyquist frequency, so
//! every one of the `size / 2` bins is an ordinary complex number with a
//! phase that can turn, which is what a phase vocoder wants of them.
//!
//! Ported from `RealFFT` with `halfBinShift` in Signalsmith Linear (MIT,
//! see `LICENSE-THIRD-PARTY`): the real block is folded into a complex one
//! of half the length, which `rustfft` transforms.

use std::f64::consts::{FRAC_PI_2, TAU};
use std::sync::Arc;

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};

pub(crate) type Complex32 = Complex<f32>;

pub(crate) const ZERO: Complex32 = Complex32::new(0.0, 0.0);

/// The smallest length at or above `size` that the complex FFT is fast at:
/// a power of two times 1, 2, 3, 4, 5, 6 or 8.
fn fast_complex_size(size: usize) -> usize {
    let mut power = 1;
    while power < 16 && power < size {
        power *= 2;
    }
    while power * 8 < size {
        power *= 2;
    }
    let multiple = size.div_ceil(power);
    power * if multiple == 7 { 8 } else { multiple }
}

/// The FFT length for blocks of `block` samples: at least as long, and a
/// multiple of four.
pub(crate) fn fast_size(block: usize) -> usize {
    fast_complex_size(block.div_ceil(4)) * 4
}

pub(crate) struct ShiftedRealFft {
    forward: Arc<dyn Fft<f32>>,
    inverse: Arc<dyn Fft<f32>>,
    /// The folded block, transformed in place.
    work: Vec<Complex32>,
    scratch: Vec<Complex32>,
    /// Turns the folded block by half a bin.
    twists: Vec<Complex32>,
    /// Untangles the two real blocks the folded transform holds.
    twiddles: Vec<Complex32>,
}

impl ShiftedRealFft {
    /// A transform of `size` real samples. `size` must be even.
    pub fn new(size: usize) -> Self {
        assert!(
            size >= 2 && size.is_multiple_of(2),
            "the FFT length must be even"
        );
        let half = size / 2;
        let mut planner = FftPlanner::<f32>::new();
        let forward = planner.plan_fft_forward(half);
        let inverse = planner.plan_fft_inverse(half);
        let scratch = forward
            .get_inplace_scratch_len()
            .max(inverse.get_inplace_scratch_len());
        let turn = |angle: f64| Complex32::new(angle.cos() as f32, angle.sin() as f32);
        Self {
            forward,
            inverse,
            work: vec![ZERO; half],
            scratch: vec![ZERO; scratch],
            twists: (0..half)
                .map(|i| turn(-TAU * i as f64 / size as f64))
                .collect(),
            // A quarter turn back is folded in, which is the `-i` that
            // separates the odd samples from the even ones.
            twiddles: (0..half.div_ceil(2))
                .map(|i| turn(-TAU * (i as f64 + 0.5) / size as f64 - FRAC_PI_2))
                .collect(),
        }
    }

    pub fn size(&self) -> usize {
        self.work.len() * 2
    }

    /// Number of bins: half the length.
    pub fn bins(&self) -> usize {
        self.work.len()
    }

    /// `freq[k]` becomes the sum of `time[n] * e^(-2 pi i (k + 0.5) n / size)`.
    pub fn forward(&mut self, time: &[f32], freq: &mut [Complex32]) {
        let half = self.work.len();
        assert!(time.len() == half * 2 && freq.len() == half);
        for ((slot, pair), twist) in self
            .work
            .iter_mut()
            .zip(time.as_chunks::<2>().0.iter())
            .zip(&self.twists)
        {
            *slot = Complex32::new(pair[0], pair[1]) * twist;
        }
        self.forward
            .process_with_scratch(&mut self.work, &mut self.scratch);
        for (i, twiddle) in self.twiddles.iter().enumerate() {
            let mirror = half - 1 - i;
            let (a, b) = (self.work[i], self.work[mirror]);
            let odd = Complex32::new((a.re + b.re) * 0.5, (a.im - b.im) * 0.5);
            let even = Complex32::new((a.re - b.re) * 0.5, (a.im + b.im) * 0.5) * twiddle;
            freq[i] = odd + even;
            freq[mirror] = Complex32::new(odd.re - even.re, even.im - odd.im);
        }
    }

    /// The inverse of [`ShiftedRealFft::forward`], `size` times too loud.
    pub fn inverse(&mut self, freq: &[Complex32], time: &mut [f32]) {
        let half = self.work.len();
        assert!(time.len() == half * 2 && freq.len() == half);
        for (i, twiddle) in self.twiddles.iter().enumerate() {
            let mirror = half - 1 - i;
            let (a, b) = (freq[i], freq[mirror]);
            let odd = Complex32::new(a.re + b.re, a.im - b.im);
            let even = Complex32::new(a.re - b.re, a.im + b.im) * twiddle.conj();
            self.work[i] = odd + even;
            self.work[mirror] = Complex32::new(odd.re - even.re, even.im - odd.im);
        }
        self.inverse
            .process_with_scratch(&mut self.work, &mut self.scratch);
        for ((slot, pair), twist) in self
            .work
            .iter()
            .zip(time.as_chunks_mut::<2>().0.iter_mut())
            .zip(&self.twists)
        {
            let sample = slot * twist.conj();
            pair[0] = sample.re;
            pair[1] = sample.im;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_signal(size: usize) -> Vec<f32> {
        let mut state = 0x1234_5678_u32;
        (0..size)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (state >> 8) as f32 / 8_388_608.0 - 1.0
            })
            .collect()
    }

    #[test]
    fn fast_sizes_cover_the_block_and_split_into_small_factors() {
        assert_eq!(fast_size(5_760), 6_144);
        assert_eq!(fast_size(5_292), 6_144);
        assert_eq!(fast_size(11_520), 12_288);
        for block in [1, 2, 3, 16, 17, 100, 1_323, 4_410, 4_800, 23_040] {
            let size = fast_size(block);
            assert!(size >= block && size.is_multiple_of(4), "{block} -> {size}");
            let mut rest = size;
            for factor in [2, 3, 5] {
                while rest.is_multiple_of(factor) {
                    rest /= factor;
                }
            }
            assert_eq!(rest, 1, "{block} -> {size}");
        }
    }

    #[test]
    fn the_forward_transform_matches_its_definition() {
        for size in [4, 16, 20, 24, 30, 96, 160] {
            let time = test_signal(size);
            let mut fft = ShiftedRealFft::new(size);
            assert_eq!((fft.size(), fft.bins()), (size, size / 2));
            let mut freq = vec![ZERO; size / 2];
            fft.forward(&time, &mut freq);
            for (k, bin) in freq.iter().enumerate() {
                let (mut re, mut im) = (0.0_f64, 0.0_f64);
                for (n, sample) in time.iter().enumerate() {
                    let angle = -TAU * (k as f64 + 0.5) * n as f64 / size as f64;
                    re += f64::from(*sample) * angle.cos();
                    im += f64::from(*sample) * angle.sin();
                }
                let error = (f64::from(bin.re) - re).hypot(f64::from(bin.im) - im);
                assert!(error < 1e-4 * size as f64, "size {size} bin {k}: {error}");
            }
        }
    }

    #[test]
    fn a_round_trip_returns_the_block_scaled_by_its_length() {
        for size in [4, 24, 30, 6_144, 12_288] {
            let time = test_signal(size);
            let mut fft = ShiftedRealFft::new(size);
            let mut freq = vec![ZERO; size / 2];
            let mut back = vec![0.0; size];
            fft.forward(&time, &mut freq);
            fft.inverse(&freq, &mut back);
            for (sample, back) in time.iter().zip(&back) {
                assert!((back / size as f32 - sample).abs() < 1e-5, "size {size}");
            }
        }
    }
}
