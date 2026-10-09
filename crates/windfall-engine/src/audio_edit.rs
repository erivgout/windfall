//! Bounded, immutable clip editing on workers. Uses playback's region reader,
//! spectral preparation and equal-power fade law; excludes mixer processing
//! and transport's transient de-click ramps.
use windfall_core::{AudioBuffer, pan_gains, samples_per_tick};
use windfall_ipc::AudioEditOperation;
use windfall_project::{Clip, ClipContent, ClipStretch};

use crate::{SamplePool, clips::equal_power, voice::Region};

pub const MAX_EDIT_SECONDS: f64 = 120.0;
pub const MAX_EDIT_BYTES: usize = 64 * 1024 * 1024;

fn bounded(frames: f64, rate: u32, channels: u16) -> Result<usize, String> {
    if !frames.is_finite()
        || frames < 1.0
        || frames / f64::from(rate) > MAX_EDIT_SECONDS
        || frames * f64::from(channels) * 4.0 > MAX_EDIT_BYTES as f64
    {
        return Err("The audio editor supports up to two minutes and 64 MiB per buffer.".into());
    }
    Ok(frames as usize)
}

/// Renders the clip's visible window at the source rate, with silence after
/// the source ends. Tempo automation is rejected by the session before calling.
pub fn render_view(pool: &SamplePool, clip: &Clip, tempo: f64) -> Result<AudioBuffer, String> {
    let ClipContent::Audio {
        sample,
        gain,
        pan,
        fade_in,
        fade_out,
        reverse,
        pitch,
        stretch,
        ..
    } = clip.content
    else {
        return Err("Select an audio clip.".into());
    };
    if !tempo.is_finite() || tempo <= 0.0 {
        return Err("The project tempo is invalid.".into());
    }
    let source = pool
        .get(sample)
        .ok_or("The clip's audio is not loaded. Reload samples first.")?;
    if !matches!(source.channels(), 1 | 2) {
        return Err("The audio editor supports mono and stereo sources.".into());
    }
    bounded(
        source.frames() as f64,
        source.sample_rate(),
        source.channels(),
    )?;
    if source.samples().iter().any(|s| !s.is_finite()) {
        return Err("The source contains non-finite audio samples.".into());
    }
    if let ClipStretch::Spectral { ratio, .. } = stretch {
        bounded(
            (source.frames() as f64 * ratio).round(),
            source.sample_rate(),
            source.channels(),
        )?;
    }
    let per_tick = samples_per_tick(tempo, f64::from(source.sample_rate()));
    let frames = bounded(
        (f64::from(clip.length) * per_tick - 1e-6).ceil(),
        source.sample_rate(),
        2,
    )?;
    let audio = pool
        .clip_audio(sample, stretch, pitch)
        .ok_or("The source is unavailable.")?;
    let speed = if matches!(stretch, ClipStretch::Tape) {
        2.0_f64.powf(f64::from(pitch) / 12.0)
    } else {
        1.0
    };
    let skip = f64::from(clip.offset) * per_tick * speed;
    let region = Region {
        first: 0,
        frames: audio.frames(),
        reverse,
    };
    let (left, right) = pan_gains(pan);
    let mut data = Vec::with_capacity(frames * 2);
    for frame in 0..frames {
        let tick = frame as f64 / per_tick;
        let mut level = gain;
        if fade_in > 0 {
            level *= equal_power(tick / f64::from(fade_in.min(clip.length)));
        }
        if fade_out > 0 {
            level *=
                equal_power((f64::from(clip.length) - tick) / f64::from(fade_out.min(clip.length)));
        }
        let position = skip + frame as f64 * speed;
        let (a, b) = if position < audio.frames() as f64 {
            region.read(&audio, position)
        } else {
            (0.0, 0.0)
        };
        data.extend([a * level * left, b * level * right]);
    }
    if data.iter().any(|s| !s.is_finite()) {
        return Err("Rendering produced non-finite audio.".into());
    }
    Ok(AudioBuffer::from_interleaved(source.sample_rate(), 2, data))
}

/// Selection is [start, end). All operations preserve channel interleaving.
/// Normalization uses one linked gain for all channels, targeting -1 dBFS.
pub fn edit(
    view: &AudioBuffer,
    operation: AudioEditOperation,
    start: u32,
    end: u32,
) -> Result<AudioBuffer, String> {
    let (start, end) = (start as usize, end as usize);
    if start >= end || end > view.frames() {
        return Err("Select at least one frame within the clip.".into());
    }
    bounded(view.frames() as f64, view.sample_rate(), view.channels())?;
    if view.samples().iter().any(|s| !s.is_finite()) {
        return Err("The audio contains non-finite samples.".into());
    }
    let channels = usize::from(view.channels());
    let first = start * channels;
    let last = end * channels;
    let data = match operation {
        AudioEditOperation::Trim | AudioEditOperation::Extract => {
            view.samples()[first..last].to_vec()
        }
        AudioEditOperation::Cut => {
            if start == 0 && end == view.frames() {
                return Err("Cut would leave an empty clip. Select a smaller range.".into());
            }
            let mut out = Vec::with_capacity(view.samples().len() - (last - first));
            out.extend_from_slice(&view.samples()[..first]);
            out.extend_from_slice(&view.samples()[last..]);
            out
        }
        _ => {
            let mut out = view.samples().to_vec();
            let selected = &mut out[first..last];
            match operation {
                AudioEditOperation::Normalize => {
                    let peak = selected.iter().fold(0.0_f32, |peak, s| peak.max(s.abs()));
                    if peak == 0.0 {
                        return Err("The selection is silent and cannot be normalized.".into());
                    }
                    let target = windfall_core::db_to_gain(-1.0);
                    for s in selected {
                        *s = ((*s as f64 / f64::from(peak)) * f64::from(target)) as f32;
                    }
                }
                AudioEditOperation::Reverse => {
                    for frame in 0..(end - start) / 2 {
                        for channel in 0..channels {
                            selected.swap(
                                frame * channels + channel,
                                (end - start - 1 - frame) * channels + channel,
                            );
                        }
                    }
                }
                AudioEditOperation::Silence => selected.fill(0.0),
                AudioEditOperation::FadeIn | AudioEditOperation::FadeOut => {
                    let denominator = (end - start - 1).max(1) as f64;
                    for (frame, samples) in selected.chunks_exact_mut(channels).enumerate() {
                        let part = if operation == AudioEditOperation::FadeIn {
                            frame as f64 / denominator
                        } else {
                            (end - start - 1 - frame) as f64 / denominator
                        };
                        for s in samples {
                            *s *= equal_power(part);
                        }
                    }
                }
                _ => unreachable!(),
            }
            out
        }
    };
    Ok(AudioBuffer::from_interleaved(
        view.sample_rate(),
        view.channels(),
        data,
    ))
}

/// Envelope across channels, rather than their average: anti-phase stereo
/// must remain visible. Bucket count is caller bounded.
pub fn overview(audio: &AudioBuffer, buckets: usize) -> Vec<f32> {
    let buckets = buckets.min(1024);
    let mut peaks = Vec::with_capacity(buckets * 2);
    let channels = usize::from(audio.channels());
    for bucket in 0..buckets {
        let first = bucket * audio.frames() / buckets;
        let end = ((bucket + 1) * audio.frames() / buckets)
            .max(first + 1)
            .min(audio.frames());
        let span = &audio.samples()[first * channels..end * channels];
        peaks.extend([
            span.iter().copied().fold(0.0_f32, f32::min),
            span.iter().copied().fold(0.0_f32, f32::max),
        ]);
    }
    peaks
}

#[cfg(test)]
mod tests {
    use super::*;
    use windfall_project::{ClipId, ClipStretchQuality, PlaylistTrackId, SampleId, TrackId};
    fn source() -> AudioBuffer {
        AudioBuffer::from_interleaved(960, 2, vec![0.1, -0.1, 0.2, -0.2, 0.3, -0.3, 0.4, -0.4])
    }
    fn clip() -> Clip {
        Clip {
            id: ClipId(1),
            track: PlaylistTrackId(2),
            start: 0,
            length: 4,
            offset: 0,
            muted: false,
            content: ClipContent::Audio {
                sample: SampleId(3),
                mixer_track: TrackId::MASTER,
                output: Default::default(),
                normalize: false,
                gain: 1.0,
                pan: 0.0,
                fade_in: 0,
                fade_out: 0,
                reverse: false,
                pitch: 0.0,
                stretch: ClipStretch::Tape,
            },
        }
    }
    #[test]
    fn exact_stereo_edits_leave_the_source_unchanged() {
        let view = source();
        let original = view.samples().to_vec();
        for operation in [AudioEditOperation::Trim, AudioEditOperation::Extract] {
            assert_eq!(
                edit(&view, operation, 1, 3).unwrap().samples(),
                &[0.2, -0.2, 0.3, -0.3]
            );
        }
        assert_eq!(
            edit(&view, AudioEditOperation::Reverse, 1, 3)
                .unwrap()
                .samples(),
            &[0.1, -0.1, 0.3, -0.3, 0.2, -0.2, 0.4, -0.4]
        );
        assert_eq!(
            edit(&view, AudioEditOperation::Cut, 1, 3)
                .unwrap()
                .samples(),
            &[0.1, -0.1, 0.4, -0.4]
        );
        assert_eq!(
            edit(&view, AudioEditOperation::Silence, 1, 3)
                .unwrap()
                .samples(),
            &[0.1, -0.1, 0.0, 0.0, 0.0, 0.0, 0.4, -0.4]
        );
        assert_eq!(
            edit(&view, AudioEditOperation::FadeIn, 1, 3)
                .unwrap()
                .samples(),
            &[0.1, -0.1, 0.0, 0.0, 0.3, -0.3, 0.4, -0.4]
        );
        assert_eq!(
            edit(&view, AudioEditOperation::FadeOut, 1, 3)
                .unwrap()
                .samples(),
            &[0.1, -0.1, 0.2, -0.2, 0.0, 0.0, 0.4, -0.4]
        );
        let normalized = edit(&view, AudioEditOperation::Normalize, 1, 3).unwrap();
        assert_eq!(&normalized.samples()[..2], &original[..2]);
        assert!((normalized.samples()[4] - windfall_core::db_to_gain(-1.0)).abs() < 1e-6);
        assert_eq!(normalized.samples()[4], -normalized.samples()[5]);
        assert_eq!(view.samples(), original);
        assert_eq!(
            overview(&view, 4),
            vec![-0.1, 0.1, -0.2, 0.2, -0.3, 0.3, -0.4, 0.4]
        );
    }
    #[test]
    fn view_uses_playback_offset_reverse_pitch_gain_pan_and_fades() {
        let mut pool = SamplePool::new();
        pool.insert(SampleId(3), source());
        let mut clip = clip(); // at 60 BPM / 960Hz: one frame per tick
        assert_eq!(
            render_view(&pool, &clip, 60.0).unwrap().samples(),
            source().samples()
        );
        clip.offset = 1;
        clip.length = 2;
        let ClipContent::Audio {
            reverse,
            gain,
            pan,
            fade_in,
            ..
        } = &mut clip.content
        else {
            unreachable!()
        };
        *reverse = true;
        *gain = 0.5;
        *pan = 1.0;
        *fade_in = 1;
        assert_eq!(
            render_view(&pool, &clip, 60.0).unwrap().samples(),
            &[0.0, -0.0, 0.0, -0.1]
        );
        clip = super::tests::clip();
        let ClipContent::Audio { pitch, .. } = &mut clip.content else {
            unreachable!()
        };
        *pitch = 12.0;
        assert_eq!(
            render_view(&pool, &clip, 60.0).unwrap().samples(),
            &[0.1, -0.1, 0.3, -0.3, 0.0, 0.0, 0.0, 0.0]
        );
    }
    #[test]
    fn spectral_view_has_independent_duration_and_preserves_source() {
        let mut pool = SamplePool::new();
        let audio = AudioBuffer::from_interleaved(
            48000,
            1,
            (0..4800).map(|i| (i as f32 * 0.05).sin() * 0.2).collect(),
        );
        pool.insert(SampleId(3), audio.clone());
        let mut clip = clip();
        clip.length = 288;
        let ClipContent::Audio { stretch, pitch, .. } = &mut clip.content else {
            unreachable!()
        };
        *stretch = ClipStretch::Spectral {
            ratio: 1.5,
            quality: ClipStretchQuality::Fast,
            formants: false,
        };
        *pitch = 7.0;
        let view = render_view(&pool, &clip, 120.0).unwrap();
        assert_eq!(view.frames(), 7200);
        assert!(view.samples().iter().any(|s| s.abs() > 0.01));
        assert_eq!(pool.get(SampleId(3)).unwrap().samples(), audio.samples());
    }
    #[test]
    fn invalid_empty_nonfinite_and_oversized_edits_are_refused() {
        let view = source();
        for (start, end) in [(0, 0), (2, 1), (0, 5)] {
            assert!(edit(&view, AudioEditOperation::Trim, start, end).is_err());
        }
        assert!(edit(&view, AudioEditOperation::Cut, 0, 4).is_err());
        let silent = AudioBuffer::from_interleaved(960, 1, vec![0.0; 4]);
        assert!(edit(&silent, AudioEditOperation::Normalize, 0, 4).is_err());
        let bad = AudioBuffer::from_interleaved(960, 1, vec![f32::NAN]);
        assert!(edit(&bad, AudioEditOperation::Silence, 0, 1).is_err());
        assert!(bounded(120.0 * 48000.0 + 1.0, 48000, 2).is_err());
        assert!(bounded(120.0 * 192000.0, 192000, 2).is_err());
        let mut pool = SamplePool::new();
        pool.insert(SampleId(3), view);
        let mut clip = clip();
        clip.length = 960 * 121;
        assert!(render_view(&pool, &clip, 60.0).is_err());
    }
}
