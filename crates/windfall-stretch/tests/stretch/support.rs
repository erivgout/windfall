//! Signals, measurements and drivers shared by the tests.

use std::f64::consts::TAU;

use windfall_core::AudioBuffer;
use windfall_stretch::{Quality, Stretcher};

pub const RATE: u32 = 48_000;

/// Marsaglia's 32-bit xorshift, so that every run tests the same signals.
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        let mut state = seed.wrapping_add(0x9E37_79B9);
        state = (state ^ (state >> 16)).wrapping_mul(0x85EB_CA6B);
        state = (state ^ (state >> 13)).wrapping_mul(0xC2B2_AE35);
        state ^= state >> 16;
        Self(if state == 0 { 0x6D2B_79F5 } else { state })
    }

    pub fn next_u32(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    /// A value from 0 up to, but not including, 1.
    pub fn unipolar(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 * (1.0 / 16_777_216.0)
    }

    /// A value from -1 up to, but not including, 1.
    pub fn bipolar(&mut self) -> f32 {
        self.unipolar() * 2.0 - 1.0
    }

    /// A whole number from `low` up to and including `high`.
    pub fn between(&mut self, low: usize, high: usize) -> usize {
        low + (self.next_u32() as usize) % (high - low + 1)
    }
}

pub fn sine(frequency: f64, level: f64, frames: usize, rate: u32) -> Vec<f32> {
    (0..frames)
        .map(|n| (level * (TAU * frequency * n as f64 / f64::from(rate)).sin()) as f32)
        .collect()
}

/// Sines of equal level at `frequencies`, adding up to a peak of at most
/// `level`.
pub fn chord(frequencies: &[f64], level: f64, frames: usize, rate: u32) -> Vec<f32> {
    let each = level / frequencies.len() as f64;
    (0..frames)
        .map(|n| {
            let time = n as f64 / f64::from(rate);
            let sum: f64 = frequencies
                .iter()
                .enumerate()
                .map(|(index, frequency)| (TAU * frequency * time + index as f64).sin())
                .sum();
            (sum * each) as f32
        })
        .collect()
}

pub fn noise(seed: u32, level: f32, frames: usize) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    (0..frames).map(|_| rng.bipolar() * level).collect()
}

pub fn square(period: usize, level: f32, frames: usize) -> Vec<f32> {
    (0..frames)
        .map(|n| {
            if (n / (period / 2)).is_multiple_of(2) {
                level
            } else {
                -level
            }
        })
        .collect()
}

/// Clicks every `period` frames, the first at frame `first`: each one a
/// burst of noise that dies away in about a millisecond, which is what the
/// start of a drum hit looks like.
pub fn clicks(first: usize, period: usize, frames: usize, rate: u32) -> Vec<f32> {
    let mut rng = Rng::new(5);
    let length = (rate / 500) as usize;
    let mut signal = vec![0.0; frames];
    for start in (first..frames).step_by(period) {
        for offset in 0..length.min(frames - start) {
            let decay = (-6.0 * offset as f32 / length as f32).exp();
            signal[start + offset] = rng.bipolar() * decay * 0.9;
        }
    }
    signal
}

/// A crude drum loop at `bpm`: a thump on every beat, a burst of noise on
/// every other one, and short ticks on the eighths.
pub fn drum_loop(bpm: f64, beats: usize, rate: u32) -> Vec<f32> {
    let beat = 60.0 / bpm * f64::from(rate);
    let frames = (beat * beats as f64).round() as usize;
    let mut rng = Rng::new(77);
    let hiss: Vec<f32> = (0..frames).map(|_| rng.bipolar()).collect();
    let mut signal = vec![0.0_f32; frames];
    let seconds = |frames: usize| frames as f32 / rate as f32;
    for eighth in 0..beats * 2 {
        let start = (eighth as f64 * beat / 2.0).round() as usize;
        for n in start..frames.min(start + (beat as usize)) {
            let since = seconds(n - start);
            if eighth % 2 == 0 {
                let sweep = 45.0 + 90.0 * (-since * 30.0).exp();
                signal[n] +=
                    0.8 * (std::f32::consts::TAU * sweep * since).sin() * (-since * 9.0).exp();
            }
            if eighth % 4 == 2 {
                signal[n] += 0.5 * hiss[n] * (-since * 25.0).exp();
            }
            signal[n] += 0.15 * hiss[n] * (-since * 90.0).exp();
        }
    }
    signal
}

pub fn rms(signal: &[f32]) -> f64 {
    let power: f64 = signal.iter().map(|s| f64::from(*s) * f64::from(*s)).sum();
    (power / signal.len().max(1) as f64).sqrt()
}

pub fn energy(signal: &[f32]) -> f64 {
    signal.iter().map(|s| f64::from(*s) * f64::from(*s)).sum()
}

pub fn peak(signal: &[f32]) -> f32 {
    signal.iter().fold(0.0, |peak, s| peak.max(s.abs()))
}

pub fn mean(signal: &[f32]) -> f64 {
    signal.iter().map(|s| f64::from(*s)).sum::<f64>() / signal.len().max(1) as f64
}

pub fn db(ratio: f64) -> f64 {
    20.0 * ratio.max(1.0e-30).log10()
}

pub fn cents(frequency: f64, reference: f64) -> f64 {
    1200.0 * (frequency / reference).log2()
}

pub fn assert_finite(signal: &[f32], what: &str) {
    if let Some(position) = signal.iter().position(|s| !s.is_finite()) {
        panic!("{what}: sample {position} is {}", signal[position]);
    }
}

/// The largest change between two neighbouring samples.
pub fn steepest(signal: &[f32]) -> f32 {
    signal.windows(2).fold(0.0, |steepest, pair| {
        steepest.max((pair[1] - pair[0]).abs())
    })
}

/// The complex amplitude of the component of `signal` at exactly
/// `frequency`, through a Hann window: its size and its phase at the
/// first sample.
pub fn tone(signal: &[f32], frequency: f64, rate: u32) -> (f64, f64) {
    let length = signal.len() as f64;
    let (mut re, mut im, mut weight) = (0.0, 0.0, 0.0);
    for (n, sample) in signal.iter().enumerate() {
        let window = 0.5 - 0.5 * (TAU * n as f64 / length).cos();
        let phase = TAU * frequency * n as f64 / f64::from(rate);
        re += f64::from(*sample) * window * phase.cos();
        im -= f64::from(*sample) * window * phase.sin();
        weight += window;
    }
    (2.0 * re.hypot(im) / weight, im.atan2(re))
}

pub fn tone_level(signal: &[f32], frequency: f64, rate: u32) -> f64 {
    tone(signal, frequency, rate).0
}

/// The frequency of the strongest component of `signal` within `span`
/// (a ratio, such as 0.03 for 3%) of `near`.
pub fn frequency_near(signal: &[f32], near: f64, span: f64, rate: u32) -> f64 {
    // A coarse scan finds the main lobe, and a golden-section search its
    // top.
    let steps = 64;
    let step = near * span * 2.0 / f64::from(steps);
    let coarse = (0..=steps)
        .map(|index| near * (1.0 - span) + step * f64::from(index))
        .max_by(|a, b| tone_level(signal, *a, rate).total_cmp(&tone_level(signal, *b, rate)))
        .unwrap_or(near);
    let (mut low, mut high) = (coarse - step, coarse + step);
    let golden = 0.618_033_988_749_895;
    for _ in 0..40 {
        let (a, b) = (high - (high - low) * golden, low + (high - low) * golden);
        if tone_level(signal, a, rate) > tone_level(signal, b, rate) {
            high = b;
        } else {
            low = a;
        }
    }
    (low + high) / 2.0
}

/// In-place radix-2 FFT. The length must be a power of two.
pub fn fft(re: &mut [f64], im: &mut [f64]) {
    let length = re.len();
    assert!(length.is_power_of_two() && im.len() == length);
    let mut j = 0;
    for i in 1..length {
        let mut bit = length >> 1;
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
    while span <= length {
        let angle = -TAU / span as f64;
        let (step_re, step_im) = (angle.cos(), angle.sin());
        for start in (0..length).step_by(span) {
            let (mut w_re, mut w_im) = (1.0, 0.0);
            for offset in 0..span / 2 {
                let (a, b) = (start + offset, start + offset + span / 2);
                let t_re = re[b] * w_re - im[b] * w_im;
                let t_im = re[b] * w_im + im[b] * w_re;
                re[b] = re[a] - t_re;
                im[b] = im[a] - t_im;
                re[a] += t_re;
                im[a] += t_im;
                (w_re, w_im) = (
                    w_re * step_re - w_im * step_im,
                    w_re * step_im + w_im * step_re,
                );
            }
        }
        span *= 2;
    }
}

/// The power in each bin of the first half of the spectrum of `signal`
/// through a Hann window. The length must be a power of two.
pub fn power_spectrum(signal: &[f32]) -> Vec<f64> {
    let size = signal.len();
    let mut re: Vec<f64> = signal
        .iter()
        .enumerate()
        .map(|(n, sample)| {
            let window = 0.5 - 0.5 * (TAU * n as f64 / size as f64).cos();
            f64::from(*sample) * window
        })
        .collect();
    let mut im = vec![0.0; size];
    fft(&mut re, &mut im);
    (0..size / 2)
        .map(|k| re[k] * re[k] + im[k] * im[k])
        .collect()
}

/// The power spectrum of `signal` averaged over half-overlapping blocks of
/// `size` frames.
pub fn average_spectrum(signal: &[f32], size: usize) -> Vec<f64> {
    let mut sum = vec![0.0; size / 2];
    let mut blocks = 0;
    for block in signal.windows(size).step_by(size / 2) {
        for (sum, power) in sum.iter_mut().zip(power_spectrum(block)) {
            *sum += power;
        }
        blocks += 1;
    }
    sum.iter()
        .map(|power| power / f64::from(blocks.max(1)))
        .collect()
}

/// How far the short-time spectrum of noise strays from flat: the
/// standard deviation, in dB, of the power in bands a third of an octave
/// wide between 200 Hz and 12 kHz. White noise reads well under 1.
pub fn band_ripple_db(signal: &[f32], rate: u32) -> f64 {
    let size = 4_096;
    let spectrum = average_spectrum(signal, size);
    let hz_per_bin = f64::from(rate) / size as f64;
    let mut levels = Vec::new();
    let mut low = 200.0_f64;
    while low < 12_000.0 {
        let high = low * 2.0_f64.powf(1.0 / 3.0);
        let bins = (low / hz_per_bin) as usize..(high / hz_per_bin) as usize;
        let power: f64 = spectrum[bins.clone()].iter().sum::<f64>() / bins.len() as f64;
        levels.push(10.0 * power.max(1e-30).log10());
        low = high;
    }
    let mean = levels.iter().sum::<f64>() / levels.len() as f64;
    let variance = levels
        .iter()
        .map(|level| (level - mean).powi(2))
        .sum::<f64>();
    (variance / levels.len() as f64).sqrt()
}

/// Spectral flatness of `signal` between 200 Hz and 12 kHz: the geometric
/// mean of its power spectrum over the arithmetic mean, from 0 for a tone
/// to 1 for white noise. It is taken over blocks of `size` frames and
/// averaged, so a long `size` catches ringing that a phase vocoder leaves
/// in noise.
pub fn spectral_flatness(signal: &[f32], size: usize, rate: u32) -> f64 {
    let hz_per_bin = f64::from(rate) / size as f64;
    let bins = (200.0 / hz_per_bin) as usize..(12_000.0 / hz_per_bin) as usize;
    let mut total = 0.0;
    let mut blocks = 0;
    for block in signal.windows(size).step_by(size / 2) {
        let spectrum = power_spectrum(block);
        let band = &spectrum[bins.clone()];
        let log_mean =
            band.iter().map(|power| power.max(1e-30).ln()).sum::<f64>() / band.len() as f64;
        let mean = band.iter().sum::<f64>() / band.len() as f64;
        total += log_mean.exp() / mean.max(1e-30);
        blocks += 1;
    }
    total / f64::from(blocks.max(1))
}

/// A stretcher at `ratio` and `pitch`, in effect from its first frame.
pub fn stretcher(channels: usize, ratio: f64, pitch: f64, quality: Quality) -> Stretcher {
    let mut stretcher = Stretcher::with_quality(channels, RATE, quality);
    stretcher.set_time_ratio(ratio);
    stretcher.set_pitch_semitones(pitch);
    stretcher.reset();
    stretcher
}

/// Feeds `input`, one vector per channel, to `stretcher` and collects
/// `frames` frames of output, asking for `sizes` frames in turn. Returns
/// the output and the number of input frames taken.
pub fn run(
    stretcher: &mut Stretcher,
    input: &[Vec<f32>],
    position: &mut usize,
    frames: usize,
    sizes: &[usize],
) -> Vec<Vec<f32>> {
    let mut output = vec![vec![0.0_f32; frames]; input.len()];
    let (mut done, mut turn) = (0, 0);
    while done < frames {
        let size = sizes[turn % sizes.len()].min(frames - done);
        turn += 1;
        let needed = stretcher.input_frames_needed(size);
        let feed: Vec<&[f32]> = input
            .iter()
            .map(|channel| &channel[(*position).min(channel.len())..])
            .collect();
        let mut into: Vec<&mut [f32]> = output
            .iter_mut()
            .map(|channel| &mut channel[done..done + size])
            .collect();
        let taken = stretcher.process(&feed, &mut into);
        assert_eq!(taken, needed, "process took what it said it would");
        *position += taken;
        done += size;
    }
    output
}

/// Stretches a mono signal through the streaming interface and returns
/// the output lined up with the input: frame `n` of the result is frame
/// `n / ratio` of `input`. The result is `input.len() * ratio` frames
/// long.
pub fn stream_mono(input: &[f32], ratio: f64, pitch: f64, quality: Quality) -> Vec<f32> {
    let mut stretcher = stretcher(1, ratio, pitch, quality);
    let lead = stretcher.latency().output_frames(ratio).round() as usize;
    let frames = (input.len() as f64 * ratio).round() as usize;
    let mut position = 0;
    let mut output = run(
        &mut stretcher,
        &[input.to_vec()],
        &mut position,
        lead + frames,
        &[512],
    );
    output.remove(0).split_off(lead)
}

/// The offline stretch of a mono signal.
pub fn offline_mono(input: &[f32], ratio: f64, pitch: f64, quality: Quality) -> Vec<f32> {
    let buffer = AudioBuffer::from_interleaved(RATE, 1, input.to_vec());
    windfall_stretch::stretch(&buffer, ratio, pitch, quality)
        .samples()
        .to_vec()
}

/// Writes a 32-bit float WAV file with one channel per vector.
pub fn write_wav(path: &std::path::Path, channels: &[&[f32]], rate: u32) {
    let frames = channels
        .iter()
        .map(|channel| channel.len())
        .min()
        .unwrap_or(0);
    let count = channels.len() as u16;
    let data_bytes = (frames * channels.len() * 4) as u32;
    let mut bytes = Vec::with_capacity(44 + data_bytes as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    // Format 3 is IEEE float.
    bytes.extend_from_slice(&3_u16.to_le_bytes());
    bytes.extend_from_slice(&count.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * u32::from(count) * 4).to_le_bytes());
    bytes.extend_from_slice(&(count * 4).to_le_bytes());
    bytes.extend_from_slice(&32_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_bytes.to_le_bytes());
    for frame in 0..frames {
        for channel in channels {
            bytes.extend_from_slice(&channel[frame].to_le_bytes());
        }
    }
    std::fs::write(path, bytes).expect("could not write the WAV file");
}
