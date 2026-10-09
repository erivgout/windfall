//! Clip normalization through the public renderer, without writing sample files.
use windfall_core::{AudioBuffer, PPQ};
use windfall_engine::{RenderOptions, SamplePool, render};
use windfall_ipc::PlayMode;
use windfall_project::{
    Clip, ClipContent, ClipId, ClipStretch, ClipStretchQuality, MAX_GAIN, PlaylistTrackId, Project,
    SampleId, TrackId,
};

fn with_invalid_sample(value: f32) -> Vec<f32> {
    let mut samples = vec![0.5; 24_000];
    samples[0] = value;
    samples
}

#[test]
fn playlist_normalize_uses_prepared_peak_and_preserves_source_and_knob() {
    for (samples, normalize, knob, stretch) in [
        (vec![0.5; 24_000], true, 1.0, ClipStretch::Tape),
        (vec![0.0; 24_000], true, 0.75, ClipStretch::Tape),
        (vec![0.5; 24_000], false, 0.75, ClipStretch::Tape),
        (vec![1.0e-9; 24_000], true, 0.75, ClipStretch::Tape),
        (with_invalid_sample(f32::NAN), true, 0.75, ClipStretch::Tape),
        (
            with_invalid_sample(f32::INFINITY),
            true,
            0.75,
            ClipStretch::Tape,
        ),
        (vec![-0.25; 24_000], true, 1.0, ClipStretch::Tape),
        (
            vec![0.5; 24_000],
            true,
            0.5,
            ClipStretch::Spectral {
                ratio: 1.5,
                quality: ClipStretchQuality::Standard,
                formants: false,
            },
        ),
    ] {
        let mut project = Project::new("Normalize playback");
        project.settings.tempo_bpm = 120.0;
        project.playlist.clips.push(Clip {
            id: ClipId(1),
            track: PlaylistTrackId(50),
            start: 0,
            length: PPQ,
            offset: 0,
            muted: false,
            content: ClipContent::Audio {
                sample: SampleId(1),
                mixer_track: TrackId::MASTER,
                output: Default::default(),
                normalize,
                gain: knob,
                pan: 0.0,
                fade_in: 0,
                fade_out: 0,
                reverse: false,
                pitch: 0.0,
                stretch,
            },
        });
        let stored = project.clone();
        let mut pool = SamplePool::new();
        let source = AudioBuffer::from_interleaved(48_000, 1, samples.clone());
        let identity = source.identity();
        pool.insert(SampleId(1), source);
        let prepared = pool.clip_audio(SampleId(1), stretch, 0.0).unwrap();
        let peak = prepared
            .samples()
            .iter()
            .map(|sample| sample.abs())
            .fold(0.0_f32, f32::max);
        let expected_gain = if normalize
            && prepared.samples().iter().all(|sample| sample.is_finite())
            && peak >= 1.0e-8
        {
            (knob / peak).min(MAX_GAIN)
        } else {
            knob
        };
        let options = RenderOptions {
            mode: PlayMode::Song,
            sample_rate: 48_000,
            block_frames: 480,
            tail_secs: 0.0,
            ..Default::default()
        };
        let audio = render(&project, &pool, &options, &mut |_| true);
        for frame in [1_000, 5_000, 10_000] {
            let expected = prepared.samples()[frame] * expected_gain;
            for channel in 0..2 {
                let actual = audio.samples()[frame * 2 + channel];
                assert!(
                    (actual - expected).abs() < 1.0e-12 + expected.abs() * 1.0e-6,
                    "{actual} != {expected}"
                );
            }
        }
        assert_eq!(project, stored);
        assert_eq!(pool.get(SampleId(1)).unwrap().identity(), identity);
        assert_eq!(
            pool.get(SampleId(1))
                .unwrap()
                .samples()
                .iter()
                .map(|sample| sample.to_bits())
                .collect::<Vec<_>>(),
            samples
                .iter()
                .map(|sample| sample.to_bits())
                .collect::<Vec<_>>()
        );
    }
}
