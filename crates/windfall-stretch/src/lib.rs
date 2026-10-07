//! Time-stretching and pitch-shifting.

mod fft;
mod offline;
mod stft;
mod stretcher;
mod tempo;

pub use offline::{stretch, stretched_frames};
pub use stretcher::{
    Latency, MAX_PITCH_SEMITONES, MAX_TIME_RATIO, MIN_TIME_RATIO, Quality, Stretcher,
};

pub use tempo::{TempoCandidate, beat_length_ratio, estimate_tempo, tempo_ratio};
