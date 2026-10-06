use windfall_core::AudioBuffer;

/// Builds a waveform overview of the whole buffer for drawing.
///
/// The buffer is split into `buckets` equal spans. For each span the result
/// holds the lowest and then the highest sample value, with all channels
/// averaged into one signal, so the result is `2 * buckets` long.
///
/// A buffer with fewer frames than buckets is stretched: neighbouring buckets
/// repeat the same frame. An empty buffer gives all zeros.
pub fn peaks(buffer: &AudioBuffer, buckets: usize) -> Vec<f32> {
    let mut overview = vec![0.0; buckets * 2];
    let frames = buffer.frames();
    if frames == 0 {
        return overview;
    }

    let channels = usize::from(buffer.channels());
    let samples = buffer.samples();
    // The first frame of a bucket. The product can pass 64 bits for a long
    // buffer and a very fine overview, so it is done in 128.
    let first_frame = |bucket: usize| (bucket as u128 * frames as u128 / buckets as u128) as usize;

    let (pairs, _) = overview.as_chunks_mut::<2>();
    for (bucket, [low, high]) in pairs.iter_mut().enumerate() {
        let start = first_frame(bucket);
        // Every bucket covers at least one frame, even when there are more
        // buckets than frames.
        let end = first_frame(bucket + 1).max(start + 1);

        *low = f32::INFINITY;
        *high = f32::NEG_INFINITY;
        for frame in samples[start * channels..end * channels].chunks_exact(channels) {
            let mixed = frame.iter().sum::<f32>() / channels as f32;
            *low = low.min(mixed);
            *high = high.max(mixed);
        }
    }
    overview
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mono(samples: &[f32]) -> AudioBuffer {
        AudioBuffer::from_interleaved(48_000, 1, samples.to_vec())
    }

    #[test]
    fn each_bucket_holds_its_min_then_max() {
        let buffer = mono(&[0.1, -0.2, 0.3, 0.9, -1.0, 0.0, 0.5, 0.25]);
        assert_eq!(
            peaks(&buffer, 4),
            [-0.2, 0.1, 0.3, 0.9, -1.0, 0.0, 0.25, 0.5]
        );
    }

    #[test]
    fn one_bucket_spans_the_whole_buffer() {
        let buffer = mono(&[0.1, -0.2, 0.3, 0.9, -1.0, 0.0]);
        assert_eq!(peaks(&buffer, 1), [-1.0, 0.9]);
    }

    #[test]
    fn uneven_buckets_cover_every_frame_once() {
        // Seven frames in three buckets: frames 0-1, 2-3 and 4-6.
        let buffer = mono(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]);
        assert_eq!(peaks(&buffer, 3), [1.0, 2.0, 3.0, 4.0, 5.0, 7.0]);
    }

    #[test]
    fn channels_are_averaged() {
        let buffer = AudioBuffer::from_interleaved(48_000, 2, vec![1.0, 0.0, -1.0, -0.5]);
        assert_eq!(peaks(&buffer, 2), [0.5, 0.5, -0.75, -0.75]);
    }

    #[test]
    fn opposite_channels_cancel() {
        let buffer = AudioBuffer::from_interleaved(48_000, 2, vec![0.8, -0.8, -0.3, 0.3]);
        assert_eq!(peaks(&buffer, 1), [0.0, 0.0]);
    }

    #[test]
    fn a_sine_fills_every_bucket_from_minus_one_to_one() {
        let samples: Vec<f32> = (0..48_000)
            .map(|n| (n as f32 * std::f32::consts::TAU * 1_000.0 / 48_000.0).sin())
            .collect();
        let overview = peaks(&mono(&samples), 100);
        assert_eq!(overview.len(), 200);
        for [low, high] in overview.as_chunks::<2>().0 {
            assert!(*low < -0.99 && *high > 0.99, "{low} to {high}");
        }
    }

    #[test]
    fn short_buffers_are_stretched() {
        let buffer = mono(&[0.5, -0.5]);
        assert_eq!(
            peaks(&buffer, 4),
            [0.5, 0.5, 0.5, 0.5, -0.5, -0.5, -0.5, -0.5]
        );
    }

    #[test]
    fn a_single_frame_fills_every_bucket() {
        let overview = peaks(&mono(&[0.25]), 3);
        assert_eq!(overview, [0.25; 6]);
    }

    #[test]
    fn an_empty_buffer_gives_silence() {
        let buffer = AudioBuffer::from_interleaved(48_000, 2, Vec::new());
        assert_eq!(peaks(&buffer, 3), [0.0; 6]);
    }

    #[test]
    fn zero_buckets_give_nothing() {
        assert!(peaks(&mono(&[0.5, -0.5]), 0).is_empty());
    }
}
