//! The short-time Fourier transform the stretcher works in: blocks of
//! input go to spectra, and spectra are overlapped and added back into
//! output.
//!
//! Ported from `DynamicSTFT` in Signalsmith Linear (MIT, see
//! `LICENSE-THIRD-PARTY`). It normalises itself: next to the overlapped
//! output it adds up, sample by sample, how much the windows of the blocks
//! laid there weigh, and divides by that on the way out. A spectrum that
//! goes through unchanged therefore comes back as the input, wherever the
//! blocks are put.

use std::f64::consts::PI;

use crate::fft::{Complex32, ShiftedRealFft, fast_size};

/// What a sample of output with no block over it yet is divided by: small
/// enough to add nothing, large enough to divide by.
const ALMOST_ZERO: f32 = 1e-30;

/// The size of a full-scale sine in a spectrum. The predictions multiply
/// three bins together and square the result, so this is placed to leave
/// as much room above full scale as below the quietest sound that matters.
const FULL_SCALE_BIN: f64 = 16.0;

/// Input is clipped to this, 80 dB over full scale, which keeps the
/// arithmetic finite whatever comes in.
const INPUT_LIMIT: f32 = 1e4;

/// A sample that arithmetic is safe with: not a number becomes silence,
/// and anything else is clipped to [`INPUT_LIMIT`].
#[inline]
fn clean(sample: f32) -> f32 {
    if sample.is_nan() {
        0.0
    } else {
        sample.clamp(-INPUT_LIMIT, INPUT_LIMIT)
    }
}

/// The zeroth-order modified Bessel function of the first kind, by its
/// power series.
fn bessel_i0(x: f64) -> f64 {
    let (mut sum, mut term) = (1.0, 1.0);
    for k in 1..64 {
        term *= (x / (2.0 * f64::from(k))).powi(2);
        sum += term;
        if term < 1e-12 * sum {
            break;
        }
    }
    sum
}

/// How wide the Kaiser window is made for blocks that overlap `overlap`
/// times: the distance between the two nulls around its main lobe, in
/// bins of a transform as long as the block.
///
/// This follows Signalsmith's numerical search for the Kaiser window that
/// leaks least between overlapping blocks.
fn bandwidth(overlap: f64) -> f64 {
    (overlap + 8.0 / (overlap + 3.0).powi(2) + 0.25 * (3.0 - overlap).max(0.0)).max(2.0)
}

/// A Kaiser window of `block` samples for blocks `interval` samples apart,
/// scaled so that the squares of the windows of overlapping blocks add up
/// to one everywhere.
fn window(block: usize, interval: usize) -> Vec<f64> {
    let bandwidth = bandwidth((block as f64 / interval as f64).min(8.0));
    let beta = (bandwidth * bandwidth * 0.25 - 1.0).sqrt() * PI;
    // An even block peaks on sample `block / 2`; an odd one is symmetric.
    let shift = block % 2;
    let mut window: Vec<f64> = (0..block)
        .map(|i| {
            let position = (2 * i + shift) as f64 / block as f64 - 1.0;
            bessel_i0(beta * (1.0 - position * position).max(0.0).sqrt()) / bessel_i0(beta)
        })
        .collect();
    for first in 0..interval.min(block) {
        let power: f64 = window[first..]
            .iter()
            .step_by(interval)
            .map(|value| value * value)
            .sum();
        let scale = 1.0 / power.sqrt();
        for value in window[first..].iter_mut().step_by(interval) {
            *value *= scale;
        }
    }
    window
}

pub(crate) struct Stft {
    channels: usize,
    block: usize,
    interval: usize,
    /// Index of the peak of the window, which is the moment a block
    /// stands for.
    centre: usize,
    fft: ShiftedRealFft,
    /// The window a block of input is multiplied by.
    analysis: Vec<f32>,
    /// The window a block of output is multiplied by.
    synthesis: Vec<f32>,
    /// What one block adds to the weights of the samples under it.
    product: Vec<f32>,
    /// The weights the blocks before a block leave under it once blocks
    /// have been coming at every interval for a while.
    settled: Vec<f32>,
    /// The newest input of every channel, one ring after the other.
    input: Vec<f32>,
    input_len: usize,
    /// Where the next input sample goes.
    input_pos: usize,
    /// The overlapped output of every channel, one ring after the other,
    /// not yet divided by `weights`.
    output: Vec<f32>,
    weights: Vec<f32>,
    /// The next output sample to be read.
    output_pos: usize,
    time: Vec<f32>,
}

impl Stft {
    /// Blocks of `block` samples, a new one every `interval` samples of
    /// output. The input ring keeps one interval more than a block, so
    /// that the block one interval back can be looked at again.
    pub fn new(channels: usize, block: usize, interval: usize) -> Self {
        assert!(channels >= 1 && interval >= 1 && block >= interval);
        let fft = ShiftedRealFft::new(fast_size(block));
        let shape = window(block, interval);
        let scale = FULL_SCALE_BIN * 2.0 / shape.iter().sum::<f64>();
        let analysis: Vec<f32> = shape.iter().map(|value| (value * scale) as f32).collect();
        let synthesis: Vec<f32> = shape.iter().map(|value| *value as f32).collect();
        let product: Vec<f32> = shape
            .iter()
            .map(|value| (value * value * scale * fft.size() as f64) as f32)
            .collect();
        let mut settled = vec![0.0; block];
        for index in (0..block - interval).rev() {
            settled[index] = product[index + interval] + settled[index + interval];
        }
        let input_len = block + interval + 1;
        let mut stft = Self {
            channels,
            block,
            interval,
            centre: block / 2,
            analysis,
            synthesis,
            product,
            settled,
            input: vec![0.0; input_len * channels],
            input_len,
            input_pos: 0,
            output: vec![0.0; block * channels],
            weights: vec![0.0; block],
            output_pos: 0,
            time: vec![0.0; fft.size()],
            fft,
        };
        stft.reset();
        stft
    }

    pub fn block(&self) -> usize {
        self.block
    }

    pub fn interval(&self) -> usize {
        self.interval
    }

    pub fn bins(&self) -> usize {
        self.fft.bins()
    }

    pub fn fft_size(&self) -> usize {
        self.fft.size()
    }

    /// Bins from the centre of a steady partial to the edge of the main
    /// lobe the window gives it.
    pub fn lobe_bins(&self) -> f32 {
        let width = bandwidth((self.block as f64 / self.interval as f64).min(8.0)) * 0.5;
        (width * self.fft.size() as f64 / self.block as f64) as f32
    }

    /// Input samples the ring remembers.
    pub fn input_len(&self) -> usize {
        self.input_len
    }

    /// Samples from the moment a block stands for to the newest input
    /// sample it needs.
    pub fn analysis_latency(&self) -> usize {
        self.block - self.centre
    }

    /// Samples from where a block is laid into the output to the moment
    /// it stands for.
    pub fn synthesis_latency(&self) -> usize {
        self.centre
    }

    /// Forgets all input and output, as if silence had been coming in and
    /// blocks had been laid at every interval for ever.
    pub fn reset(&mut self) {
        self.input.fill(0.0);
        self.output.fill(0.0);
        self.input_pos = 0;
        self.output_pos = 0;
        for (weight, settled) in self.weights.iter_mut().zip(&self.settled) {
            *weight = settled + ALMOST_ZERO;
        }
    }

    /// Writes `samples` for `channel` as the newest input, `offset`
    /// samples after the write position. [`Stft::move_input`] then moves
    /// the position for all channels at once.
    ///
    /// What is written is always a finite number within [`INPUT_LIMIT`].
    pub fn write_input(&mut self, channel: usize, offset: usize, samples: &[f32]) {
        let ring = &mut self.input[channel * self.input_len..][..self.input_len];
        let start = (self.input_pos + offset) % self.input_len;
        let (ahead, wrapped) = samples.split_at(samples.len().min(self.input_len - start));
        for (slot, sample) in ring[start..].iter_mut().zip(ahead) {
            *slot = clean(*sample);
        }
        for (slot, sample) in ring.iter_mut().zip(wrapped) {
            *slot = clean(*sample);
        }
    }

    /// Writes `count` samples of silence for `channel`, like
    /// [`Stft::write_input`].
    pub fn write_silence(&mut self, channel: usize, offset: usize, count: usize) {
        let ring = &mut self.input[channel * self.input_len..][..self.input_len];
        let start = (self.input_pos + offset) % self.input_len;
        let first = count.min(self.input_len - start);
        ring[start..start + first].fill(0.0);
        ring[..count - first].fill(0.0);
    }

    pub fn move_input(&mut self, samples: usize) {
        self.input_pos = (self.input_pos + samples) % self.input_len;
    }

    /// The spectrum of the block of `channel` that ends `samples_in_past`
    /// samples before the newest input.
    pub fn analyse(&mut self, channel: usize, samples_in_past: usize, spectrum: &mut [Complex32]) {
        let ring = &self.input[channel * self.input_len..][..self.input_len];
        let start =
            (2 * self.input_len + self.input_pos - self.block - samples_in_past) % self.input_len;
        let size = self.time.len();
        // The block is turned so that its centre is sample 0 of the FFT,
        // which makes a bin's phase the phase at the moment the block
        // stands for. With bins half way between the usual ones a block
        // repeats upside down, so the half that wraps is negated.
        let mut at = start;
        for (index, weight) in self.analysis.iter().enumerate() {
            let sample = ring[at] * weight;
            at = if at + 1 == self.input_len { 0 } else { at + 1 };
            if index < self.centre {
                self.time[size - self.centre + index] = -sample;
            } else {
                self.time[index - self.centre] = sample;
            }
        }
        self.time[self.block - self.centre..size - self.centre].fill(0.0);
        self.fft.forward(&self.time, spectrum);
    }

    /// Makes room in the weights for a block laid at the read position.
    /// Call it once before the [`Stft::synthesise`] of each channel.
    pub fn add_block_weight(&mut self) {
        let (ahead, wrapped) = self.product.split_at(self.block - self.output_pos);
        for (weight, product) in self.weights[self.output_pos..].iter_mut().zip(ahead) {
            *weight += product;
        }
        for (weight, product) in self.weights.iter_mut().zip(wrapped) {
            *weight += product;
        }
    }

    /// Adds the block that `spectrum` holds to the output of `channel`,
    /// starting at the read position.
    pub fn synthesise(&mut self, channel: usize, spectrum: &[Complex32]) {
        self.fft.inverse(spectrum, &mut self.time);
        let ring = &mut self.output[channel * self.block..][..self.block];
        let size = self.time.len();
        let mut at = self.output_pos;
        for (index, weight) in self.synthesis.iter().enumerate() {
            let sample = if index < self.centre {
                -self.time[size - self.centre + index]
            } else {
                self.time[index - self.centre]
            };
            ring[at] += sample * weight;
            at = if at + 1 == self.block { 0 } else { at + 1 };
        }
    }

    /// Copies the next `into.len()` samples of output of `channel`, at
    /// most one interval. [`Stft::move_output`] then steps past them for
    /// all channels at once.
    pub fn read_output(&self, channel: usize, into: &mut [f32]) {
        let ring = &self.output[channel * self.block..][..self.block];
        let mut at = self.output_pos;
        for sample in into {
            *sample = ring[at] / self.weights[at];
            at = if at + 1 == self.block { 0 } else { at + 1 };
        }
    }

    /// Steps past `samples` samples of output and clears them for the
    /// blocks to come.
    pub fn move_output(&mut self, samples: usize) {
        let mut at = self.output_pos;
        for _ in 0..samples.min(self.block) {
            for channel in 0..self.channels {
                self.output[channel * self.block + at] = 0.0;
            }
            self.weights[at] = ALMOST_ZERO;
            at = if at + 1 == self.block { 0 } else { at + 1 };
        }
        self.output_pos = (self.output_pos + samples) % self.block;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fft::ZERO;

    #[test]
    fn overlapping_windows_add_up_to_one_in_power() {
        for (block, interval) in [(5_760, 1_440), (4_800, 1_920), (5_292, 1_323), (1_323, 331)] {
            let window = window(block, interval);
            for first in 0..interval {
                let power: f64 = window[first..]
                    .iter()
                    .step_by(interval)
                    .map(|w| w * w)
                    .sum();
                assert!((power - 1.0).abs() < 1e-9, "{block}/{interval} at {first}");
            }
            let peak = window
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(index, _)| index);
            assert_eq!(peak, Some(block / 2), "{block}/{interval}");
        }
    }

    /// Runs `signal` through analysis and synthesis with blocks every
    /// `hop` samples of input and returns the output.
    fn round_trip(stft: &mut Stft, signal: &[f32], hop: usize) -> Vec<f32> {
        let interval = stft.interval();
        let mut spectrum = vec![ZERO; stft.bins()];
        let mut output = vec![0.0; signal.len() / hop * interval];
        for (index, chunk) in signal.chunks_exact(hop).enumerate() {
            stft.write_input(0, 0, chunk);
            stft.move_input(hop);
            stft.analyse(0, 0, &mut spectrum);
            stft.add_block_weight();
            stft.synthesise(0, &spectrum);
            stft.read_output(0, &mut output[index * interval..][..interval]);
            stft.move_output(interval);
        }
        output
    }

    #[test]
    fn an_untouched_spectrum_comes_back_as_the_input() {
        let (block, interval) = (480, 120);
        let mut stft = Stft::new(1, block, interval);
        let signal: Vec<f32> = (0..interval * 40)
            .map(|n| (n as f32 * 0.05).sin() * 0.7 + (n as f32 * 0.71).sin() * 0.2)
            .collect();
        let output = round_trip(&mut stft, &signal, interval);
        // What comes out is what went in one block earlier: the newest
        // sample a block needs is half a block after its centre, and the
        // block is laid half a block before it.
        let delay = stft.analysis_latency() + stft.synthesis_latency() - interval;
        // A reset stands for silence before, so this holds from the first
        // sample that has any input to show.
        for n in delay..output.len() {
            let expected = signal[n - delay];
            assert!((output[n] - expected).abs() < 2e-5, "sample {n}");
        }
        assert!(output[..delay].iter().all(|sample| sample.abs() < 2e-5));
    }

    #[test]
    fn a_full_scale_sine_is_a_bin_of_the_size_the_predictions_expect() {
        let mut stft = Stft::new(1, 5_760, 1_440);
        let bin = 100.5 / stft.fft_size() as f32;
        let signal: Vec<f32> = (0..5_760)
            .map(|n| (std::f32::consts::TAU * bin * n as f32).sin())
            .collect();
        stft.write_input(0, 0, &signal);
        stft.move_input(signal.len());
        let mut spectrum = vec![ZERO; stft.bins()];
        stft.analyse(0, 0, &mut spectrum);
        assert!(
            (spectrum[100].norm() - 16.0).abs() < 0.1,
            "{}",
            spectrum[100].norm()
        );
    }

    #[test]
    fn reset_clears_what_was_heard() {
        let mut stft = Stft::new(2, 480, 120);
        let signal = vec![0.5; 480 * 4];
        let before = round_trip(&mut stft, &signal, 120);
        assert!(before.iter().any(|sample| sample.abs() > 0.1));
        stft.reset();
        let after = round_trip(&mut stft, &vec![0.0; 480 * 2], 120);
        assert!(after.iter().all(|sample| *sample == 0.0));
    }
}
