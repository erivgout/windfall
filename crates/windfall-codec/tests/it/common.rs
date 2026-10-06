use std::f64::consts::TAU;
use std::fs;
use std::path::{Path, PathBuf};

use windfall_core::AudioBuffer;

/// Peak level of every tone in `tests/fixtures`, as set in `generate.sh`.
pub const FIXTURE_LEVEL: f64 = 0.5;

pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Every audio fixture, for tests that damage them.
pub fn fixture_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(fixture(""))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(windfall_codec::is_audio_extension)
        })
        .collect();
    files.sort();
    files
}

/// Interleaved sines starting at phase zero, one frequency per channel: the
/// formula `generate.sh` gives ffmpeg.
pub fn tones(sample_rate: u32, frequencies: &[f64], frames: usize, level: f64) -> AudioBuffer {
    let mut samples = Vec::with_capacity(frames * frequencies.len());
    for frame in 0..frames {
        let seconds = frame as f64 / f64::from(sample_rate);
        for frequency in frequencies {
            samples.push((level * (TAU * frequency * seconds).sin()) as f32);
        }
    }
    AudioBuffer::from_interleaved(sample_rate, frequencies.len() as u16, samples)
}

pub fn channel(buffer: &AudioBuffer, index: usize) -> Vec<f32> {
    buffer
        .samples()
        .chunks_exact(usize::from(buffer.channels()))
        .map(|frame| frame[index])
        .collect()
}

/// The strongest frequency in a signal, searched in 20 Hz steps up to 2 kHz.
/// Every fixture tone sits on that grid, so a tone decoded at the wrong rate
/// or on the wrong channel lands on a different step.
pub fn dominant_frequency(signal: &[f32], sample_rate: u32) -> f64 {
    let mut best = (0.0, 0.0);
    for step in 1..=100 {
        let frequency = f64::from(step) * 20.0;
        let turn = TAU * frequency / f64::from(sample_rate);
        let (mut real, mut imaginary) = (0.0, 0.0);
        for (index, &sample) in signal.iter().enumerate() {
            let angle = turn * index as f64;
            real += f64::from(sample) * angle.cos();
            imaginary += f64::from(sample) * angle.sin();
        }
        let power = real * real + imaginary * imaginary;
        if power > best.1 {
            best = (frequency, power);
        }
    }
    best.0
}

/// Largest difference between two signals, sample by sample.
pub fn max_difference(left: &[f32], right: &[f32]) -> f32 {
    assert_eq!(left.len(), right.len(), "the signals differ in length");
    left.iter()
        .zip(right)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f32::max)
}

/// A fresh, empty directory under the system temp folder, removed again when
/// dropped.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("windfall-codec-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// Names of everything in the directory, sorted.
    pub fn entries(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(&self.0)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A small seeded generator (xorshift64) so the fuzz-style tests feed the
/// same bytes on every run.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// A value below `bound`.
    pub fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }

    /// A sample between -1 and 1.
    pub fn sample(&mut self) -> f32 {
        ((self.next() >> 40) as f32 / (1_u64 << 23) as f32) - 1.0
    }
}
