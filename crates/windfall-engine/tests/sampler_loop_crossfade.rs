use windfall_engine::loop_crossfade_mix;
use windfall_project::{
    ChannelId, ChannelSource, Command, Document, Project, SamplerLoopMode, SamplerPatch,
};

fn frame(index: usize) -> (f32, f32) {
    (index as f32 + 1.0, -(index as f32) - 1.0)
}

#[test]
fn zero_returns_the_dry_frame_bit_for_bit() {
    for mode in [SamplerLoopMode::Forward, SamplerLoopMode::PingPong] {
        for offset in -20isize..20 {
            let period = if mode == SamplerLoopMode::PingPong {
                14
            } else {
                8
            };
            let phase = offset.rem_euclid(period) as usize;
            let index = if mode == SamplerLoopMode::PingPong {
                phase.min(14 - phase)
            } else {
                phase
            };
            let dry = (f32::from_bits(0x80000000), frame(index).1);
            let got = loop_crossfade_mix(8, mode, 0.0, offset, |mapped| {
                (f32::from_bits(0x80000000), frame(mapped).1)
            });
            assert_eq!(
                (got.0.to_bits(), got.1.to_bits()),
                (dry.0.to_bits(), dry.1.to_bits())
            );
        }
    }
}

#[test]
fn forward_tail_mixes_end_and_start_with_equal_power() {
    // N = 4: phase 5 blends dry frame 5 with start frame 1 at 50%.
    let got = loop_crossfade_mix(8, SamplerLoopMode::Forward, 0.5, 5, frame);
    let gain = std::f32::consts::FRAC_1_SQRT_2;
    assert!((got.0 - (frame(5).0 + frame(1).0) * gain).abs() < 1e-6);
    assert!((got.1 - (frame(5).1 + frame(1).1) * gain).abs() < 1e-6);
}

#[test]
fn middle_of_loop_stays_dry() {
    for offset in 0..6 {
        assert_eq!(
            loop_crossfade_mix(8, SamplerLoopMode::Forward, 0.25, offset, frame),
            frame(offset as usize)
        );
    }
}

#[test]
fn crossfade_never_exceeds_half_the_loop() {
    for frames in 2..20 {
        let tail = frames - frames / 2;
        for amount in [0.5, 1.0, 10.0] {
            for offset in 0..tail {
                assert_eq!(
                    loop_crossfade_mix(
                        frames,
                        SamplerLoopMode::Forward,
                        amount,
                        offset as isize,
                        frame
                    ),
                    frame(offset)
                );
            }
        }
        assert_eq!(
            loop_crossfade_mix(frames, SamplerLoopMode::Forward, 1.0, tail as isize, frame),
            loop_crossfade_mix(frames, SamplerLoopMode::Forward, 10.0, tail as isize, frame)
        );
    }
}

#[test]
fn small_amount_rounds_to_zero_frames() {
    assert_eq!(
        loop_crossfade_mix(8, SamplerLoopMode::Forward, 0.01, 7, frame),
        frame(7)
    );
    assert_eq!(
        loop_crossfade_mix(8, SamplerLoopMode::Forward, 0.1, 7, frame),
        frame(0)
    );
}

#[test]
fn ping_pong_tail_blends_into_the_reflected_start() {
    // Eight source frames have a 14-frame reflected period; its four-frame
    // tail at phase 11 reads source frame 3 and blends with start frame 1.
    let got = loop_crossfade_mix(8, SamplerLoopMode::PingPong, 0.5, 11, frame);
    let gain = std::f32::consts::FRAC_1_SQRT_2;
    assert!((got.0 - (frame(3).0 + frame(1).0) * gain).abs() < 1e-6);
    assert_eq!(
        loop_crossfade_mix(8, SamplerLoopMode::PingPong, 0.5, 9, frame),
        frame(5)
    );
}

#[test]
fn off_and_non_finite_amounts_are_dry() {
    assert_eq!(
        loop_crossfade_mix(8, SamplerLoopMode::Off, 1.0, 7, frame),
        frame(7)
    );
    for amount in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0] {
        assert_eq!(
            loop_crossfade_mix(8, SamplerLoopMode::Forward, amount, 7, frame),
            frame(7)
        );
    }
}

#[test]
fn one_frame_loops_hold_the_original_frame() {
    for mode in [SamplerLoopMode::Forward, SamplerLoopMode::PingPong] {
        for amount in [0.0, 0.5, 1.0] {
            for offset in -4..5 {
                assert_eq!(loop_crossfade_mix(1, mode, amount, offset, frame), frame(0));
            }
        }
    }
}

#[test]
fn saved_crossfade_defaults_round_trips_and_patches_are_sanitized() {
    let mut doc = Document::new(Project::new("Loop crossfade"));
    let added = doc
        .dispatch(
            Command::AddChannel {
                name: None,
                sample: None,
                instrument: None,
                index: None,
                mixer_track: None,
            },
            None,
        )
        .unwrap();
    let id = ChannelId(added.created[0]);
    let original = windfall_project::file::to_json(doc.project()).unwrap();
    assert!(!original.contains("loopCrossfade"));
    let legacy = windfall_project::file::from_json(&original).unwrap();
    let ChannelSource::Sampler(settings) = &legacy.channels[0].source else {
        panic!()
    };
    assert_eq!(settings.loop_crossfade, 0.0);
    for (amount, expected) in [
        (0.25, 0.25),
        (2.0, 1.0),
        (-1.0, 0.0),
        (f32::NAN, 0.0),
        (f32::INFINITY, 0.0),
        (f32::NEG_INFINITY, 0.0),
    ] {
        doc.dispatch(
            Command::UpdateSampler {
                id,
                patch: SamplerPatch {
                    loop_crossfade: Some(amount),
                    ..Default::default()
                },
            },
            None,
        )
        .unwrap();
        let ChannelSource::Sampler(settings) = &doc.project().channels[0].source else {
            panic!()
        };
        assert_eq!(settings.loop_crossfade, expected);
        assert_eq!(
            settings.loop_mode,
            SamplerLoopMode::Off,
            "values can be stored while looping is off"
        );
        let json = windfall_project::file::to_json(doc.project()).unwrap();
        assert_eq!(json.contains("loopCrossfade"), expected != 0.0);
        assert_eq!(
            windfall_project::file::from_json(&json).unwrap(),
            *doc.project()
        );
    }
}
