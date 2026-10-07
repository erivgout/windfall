//! Types shared by every Windfall crate: decoded audio, musical time, and
//! gain units. Nothing here allocates on a hot path or depends on the project
//! model.

use std::hash::{Hash, Hasher};
use std::sync::{Arc, Weak};

/// Ticks per quarter note. It is a multiple of 96 (the FL Studio default) and
/// 480 (the usual MIDI file resolution), so both import without rounding.
pub const PPQ: u32 = 960;

/// One step of the step sequencer is a sixteenth note.
pub const TICKS_PER_STEP: u32 = PPQ / 4;

/// Decoded audio held in memory as interleaved 32-bit float frames.
///
/// Cloning is cheap: the sample data is shared.
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    sample_rate: u32,
    channels: u16,
    // `Arc<Vec>` and not `Arc<[f32]>`: converting a `Vec` into the slice form
    // copies it, which doubles peak memory while a long file loads.
    data: Arc<Vec<f32>>,
}

/// Stable identity of a decoded allocation, without retaining its audio.
/// Clones of a buffer share it; even two empty buffers have distinct identities.
#[derive(Debug, Clone)]
pub struct AudioIdentity(Weak<Vec<f32>>);

impl AudioIdentity {
    pub fn is_live(&self) -> bool {
        self.0.strong_count() > 0
    }
}

impl PartialEq for AudioIdentity {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for AudioIdentity {}
impl Hash for AudioIdentity {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_ptr().hash(state);
    }
}

impl AudioBuffer {
    pub fn identity(&self) -> AudioIdentity {
        AudioIdentity(Arc::downgrade(&self.data))
    }

    /// Wraps interleaved samples. `data.len()` must be a multiple of
    /// `channels`, and `channels` must be at least 1.
    pub fn from_interleaved(sample_rate: u32, channels: u16, data: Vec<f32>) -> Self {
        assert!(channels >= 1, "an audio buffer needs at least one channel");
        assert!(sample_rate > 0, "sample rate must be positive");
        assert_eq!(
            data.len() % channels as usize,
            0,
            "interleaved data length must be a multiple of the channel count"
        );
        Self {
            sample_rate,
            channels,
            data: Arc::new(data),
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Number of frames. One frame holds one sample per channel.
    pub fn frames(&self) -> usize {
        self.data.len() / self.channels as usize
    }

    pub fn duration_secs(&self) -> f64 {
        self.frames() as f64 / self.sample_rate as f64
    }

    /// The interleaved samples.
    pub fn samples(&self) -> &[f32] {
        &self.data
    }

    /// Left and right samples of one frame. Mono buffers return the same
    /// value twice, and channels past the second are ignored.
    #[inline]
    pub fn stereo_frame(&self, frame: usize) -> (f32, f32) {
        let base = frame * self.channels as usize;
        let left = self.data[base];
        let right = if self.channels > 1 {
            self.data[base + 1]
        } else {
            left
        };
        (left, right)
    }
}

/// Converts decibels to a linear gain factor.
#[inline]
pub fn db_to_gain(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

/// Converts a linear gain factor to decibels. Zero and negative gains return
/// negative infinity.
#[inline]
pub fn gain_to_db(gain: f32) -> f32 {
    if gain <= 0.0 {
        f32::NEG_INFINITY
    } else {
        20.0 * gain.log10()
    }
}

/// Length of one tick in samples at the given tempo and sample rate.
#[inline]
pub fn samples_per_tick(tempo_bpm: f64, sample_rate: f64) -> f64 {
    sample_rate * 60.0 / (tempo_bpm * PPQ as f64)
}

/// Balance-law pan gains for a pan position from -1 (left) to 1 (right).
/// Center is unity on both sides, and panning only ever attenuates the far
/// side, so it can never push a signal into clipping.
#[inline]
pub fn pan_gains(pan: f32) -> (f32, f32) {
    let pan = pan.clamp(-1.0, 1.0);
    let left = if pan > 0.0 { 1.0 - pan } else { 1.0 };
    let right = if pan < 0.0 { 1.0 + pan } else { 1.0 };
    (left, right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_identity_is_shared_by_clones_and_does_not_retain_audio() {
        let first = AudioBuffer::from_interleaved(48_000, 1, Vec::new());
        let clone = first.clone();
        let other = AudioBuffer::from_interleaved(48_000, 1, Vec::new());
        let identity = first.identity();
        assert_eq!(identity, clone.identity());
        assert_ne!(identity, other.identity());
        drop(first);
        assert!(identity.is_live());
        drop(clone);
        assert!(!identity.is_live());
    }

    #[test]
    fn mono_frames_duplicate_to_stereo() {
        let buffer = AudioBuffer::from_interleaved(48_000, 1, vec![0.25, -0.5]);
        assert_eq!(buffer.frames(), 2);
        assert_eq!(buffer.stereo_frame(1), (-0.5, -0.5));
    }

    #[test]
    fn stereo_frames_read_both_channels() {
        let buffer = AudioBuffer::from_interleaved(44_100, 2, vec![0.1, 0.2, 0.3, 0.4]);
        assert_eq!(buffer.frames(), 2);
        assert_eq!(buffer.stereo_frame(1), (0.3, 0.4));
    }

    #[test]
    fn decibel_conversions_round_trip() {
        assert!((db_to_gain(0.0) - 1.0).abs() < 1e-6);
        assert!((db_to_gain(-6.0206) - 0.5).abs() < 1e-4);
        assert!((gain_to_db(db_to_gain(-12.0)) + 12.0).abs() < 1e-4);
        assert_eq!(gain_to_db(0.0), f32::NEG_INFINITY);
    }

    #[test]
    fn pan_attenuates_only_the_far_side() {
        assert_eq!(pan_gains(0.0), (1.0, 1.0));
        assert_eq!(pan_gains(-1.0), (1.0, 0.0));
        assert_eq!(pan_gains(0.5), (0.5, 1.0));
        assert_eq!(pan_gains(3.0), (0.0, 1.0));
    }

    #[test]
    fn tick_length_matches_tempo() {
        // At 120 bpm a quarter note lasts half a second.
        let per_tick = samples_per_tick(120.0, 48_000.0);
        assert!((per_tick * PPQ as f64 - 24_000.0).abs() < 1e-6);
    }
}
