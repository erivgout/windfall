//! Stretching a whole buffer at once.

use windfall_core::AudioBuffer;

use crate::stretcher::{MAX_PITCH_SEMITONES, MAX_TIME_RATIO, MIN_TIME_RATIO, Quality, Stretcher};

/// Frames worked on at a time.
const CHUNK: usize = 4_096;

/// Number of frames [`stretch`] returns for a buffer of `frames` frames:
/// `frames * time_ratio`, rounded, with the ratio forced into
/// [`MIN_TIME_RATIO`] to [`MAX_TIME_RATIO`].
pub fn stretched_frames(frames: usize, time_ratio: f64) -> usize {
    let ratio = if time_ratio.is_finite() {
        time_ratio
    } else {
        1.0
    };
    (frames as f64 * ratio.clamp(MIN_TIME_RATIO, MAX_TIME_RATIO)).round() as usize
}

/// Returns `buffer` made `time_ratio` times as long and transposed by
/// `pitch_semitones`, each without changing the other.
///
/// The result has exactly [`stretched_frames`] frames, at the sample rate
/// and with the channels of `buffer`. It starts where the audio starts and
/// ends where it ends: the stretcher's latency is taken off the front, and
/// the input is spread over the output's length exactly, so frame `n` of
/// the output is frame `n / time_ratio` of the input.
///
/// A time ratio of 1 with a pitch of 0 returns the audio as it is. The
/// call allocates and takes a while, so it belongs on a worker thread.
pub fn stretch(
    buffer: &AudioBuffer,
    time_ratio: f64,
    pitch_semitones: f64,
    quality: Quality,
) -> AudioBuffer {
    stretch_with_formants(buffer, time_ratio, pitch_semitones, quality, false)
}

/// Offline rendering with optional approximate voiced-spectrum formant preservation.
pub fn stretch_with_formants(
    buffer: &AudioBuffer,
    time_ratio: f64,
    pitch_semitones: f64,
    quality: Quality,
    preserve_formants: bool,
) -> AudioBuffer {
    let channels = usize::from(buffer.channels());
    let frames = buffer.frames();
    let wanted = stretched_frames(frames, time_ratio);
    let pitch = if pitch_semitones.is_finite() {
        pitch_semitones.clamp(-MAX_PITCH_SEMITONES, MAX_PITCH_SEMITONES)
    } else {
        0.0
    };
    if wanted == 0 || (wanted == frames && pitch == 0.0) {
        let samples = buffer.samples()[..wanted * channels].to_vec();
        return AudioBuffer::from_interleaved(buffer.sample_rate(), buffer.channels(), samples);
    }

    let planar: Vec<Vec<f32>> = (0..channels)
        .map(|channel| {
            let samples = buffer.samples().iter().skip(channel);
            samples.step_by(channels).copied().collect()
        })
        .collect();
    let mut stretcher = Stretcher::with_quality(channels, buffer.sample_rate(), quality);
    stretcher.set_pitch_semitones(pitch);
    stretcher.set_formant_preservation(preserve_formants);
    let lead = stretcher.begin_exact(frames, wanted);

    let mut scratch = vec![vec![0.0_f32; CHUNK]; channels];
    let mut data = Vec::with_capacity(wanted * channels);
    let (mut position, mut produced) = (0, 0);
    while produced < lead + wanted {
        let chunk = CHUNK.min(lead + wanted - produced);
        let input: Vec<&[f32]> = planar
            .iter()
            .map(|channel| &channel[position.min(frames)..])
            .collect();
        let mut output: Vec<&mut [f32]> = scratch
            .iter_mut()
            .map(|channel| &mut channel[..chunk])
            .collect();
        position += stretcher.process(&input, &mut output);
        let skip = lead.saturating_sub(produced).min(chunk);
        for frame in skip..chunk {
            data.extend(scratch.iter().map(|channel| channel[frame]));
        }
        produced += chunk;
    }
    AudioBuffer::from_interleaved(buffer.sample_rate(), buffer.channels(), data)
}
