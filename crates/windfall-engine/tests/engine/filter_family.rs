//! Actual routed filters, independently referenced signals and callback guards.

use crate::support::{Rig, left, peak, right, rms};
use windfall_core::AudioBuffer;
use windfall_dsp::{
    BassShelfParams, EffectKind, EffectParams, FastLowpassParams, SelectableFilterMode as Mode,
    SelectableFilterParams,
};
use windfall_engine::{RenderOptions, StemMode, StemOptions, render, render_stems, stems};
use windfall_ipc::PlayMode;
use windfall_project::{AutomationTarget, EffectId, TrackId};

const RATE: u32 = 48_000;

fn settings() -> Vec<EffectParams> {
    let mut result = vec![EffectParams::FastLowpass(FastLowpassParams {
        cutoff_hz: 1000.0,
        q: 1.3,
    })];
    result.extend(Mode::ALL.map(|mode| {
        EffectParams::SelectableFilter(SelectableFilterParams {
            mode,
            frequency_hz: 1000.0,
            q: 1.3,
            gain_db: 9.0,
        })
    }));
    result.push(EffectParams::BassShelf(BassShelfParams {
        frequency_hz: 150.0,
        gain_db: 12.0,
    }));
    result
}

/// Bilinear transfer expanded independently into f64 direct-form polynomials.
/// It does not call a product coefficient builder or a product processor.
fn reference(params: EffectParams) -> ([f64; 3], [f64; 3]) {
    let (frequency, q, mode, gain) = match params {
        EffectParams::FastLowpass(p) => {
            (f64::from(p.cutoff_hz), f64::from(p.q), Mode::Lowpass, 0.0)
        }
        EffectParams::SelectableFilter(p) => (
            f64::from(p.frequency_hz),
            f64::from(p.q),
            p.mode,
            f64::from(p.gain_db),
        ),
        EffectParams::BassShelf(p) => {
            let c = 10.0_f64.powf(f64::from(p.gain_db) / 20.0);
            let pole = (std::f64::consts::PI * f64::from(p.frequency_hz) / f64::from(RATE)).tan()
                / c.sqrt();
            return (
                [1.0 + c * pole, c * pole - 1.0, 0.0],
                [1.0 + pole, pole - 1.0, 0.0],
            );
        }
        _ => unreachable!(),
    };
    let mut g = (std::f64::consts::PI * frequency / f64::from(RATE)).tan();
    let mut k = 1.0 / q;
    let a = 10.0_f64.powf(gain / 40.0);
    let (alpha, beta, gamma) = match mode {
        Mode::Lowpass => (0.0, 0.0, g * g),
        Mode::Highpass => (1.0, 0.0, 0.0),
        Mode::Bandpass => (0.0, k * g, 0.0),
        Mode::Notch => (1.0, 0.0, g * g),
        Mode::LowShelf => {
            g /= a.sqrt();
            (1.0, k * g * a, g * g * a * a)
        }
        Mode::Peak => {
            k /= a;
            (1.0, k * g * a * a, g * g)
        }
        Mode::HighShelf => {
            g *= a.sqrt();
            (a * a, k * g * a, g * g)
        }
    };
    (
        [
            alpha + beta + gamma,
            2.0 * (gamma - alpha),
            alpha - beta + gamma,
        ],
        [
            1.0 + k * g + g * g,
            2.0 * (g * g - 1.0),
            1.0 - k * g + g * g,
        ],
    )
}

fn magnitude(params: EffectParams, hz: f64) -> f64 {
    let (b, a) = reference(params);
    let phase = std::f64::consts::TAU * hz / f64::from(RATE);
    let energy = |c: [f64; 3]| {
        let re = c[0] + c[1] * phase.cos() + c[2] * (2.0 * phase).cos();
        let im = c[1] * phase.sin() + c[2] * (2.0 * phase).sin();
        re * re + im * im
    };
    (energy(b) / energy(a)).sqrt()
}

fn reference_audio(params: EffectParams, dry: &[f32]) -> Vec<f32> {
    let (b, a) = reference(params);
    let mut x = [[0.0; 2]; 2];
    let mut y = [[0.0; 2]; 2];
    let mut result = Vec::with_capacity(dry.len());
    for frame in dry.as_chunks::<2>().0 {
        for channel in 0..2 {
            let input = f64::from(frame[channel]);
            let output = (b[0] * input + b[1] * x[channel][0] + b[2] * x[channel][1]
                - a[1] * y[channel][0]
                - a[2] * y[channel][1])
                / a[0];
            x[channel] = [input, x[channel][0]];
            y[channel] = [output, y[channel][0]];
            result.push(output as f32);
        }
    }
    result
}

fn fixture(audio: AudioBuffer, params: Option<EffectParams>) -> (Rig, TrackId, Option<EffectId>) {
    let mut rig = Rig::new();
    let track = rig.track();
    let channel = rig.channel_on(audio, track);
    rig.steps(channel, &[0]);
    let effect = params.map(|params| rig.effect(track, params));
    (rig, track, effect)
}

fn tone(hz: f64) -> AudioBuffer {
    let data = (0..RATE)
        .flat_map(|frame| {
            let phase = std::f64::consts::TAU * f64::from(frame) * hz / f64::from(RATE);
            [
                (phase.sin() * 0.005) as f32,
                ((phase * 1.2).sin() * 0.003) as f32,
            ]
        })
        .collect();
    AudioBuffer::from_interleaved(RATE, 2, data)
}

fn guarded_run(
    processor: &mut windfall_engine::Processor,
    frames: usize,
    block: usize,
) -> Vec<f32> {
    let mut audio = vec![0.0; frames * 2];
    for out in audio.chunks_mut(block * 2) {
        assert_eq!(
            crate::realtime::allocator_calls(|| processor.process(out)),
            0
        );
    }
    audio
}

fn difference(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f32::max)
}

#[test]
fn routed_tones_have_the_analytic_gain_for_both_channels_and_all_modes() {
    let mut max_error = 0.0_f64;
    for params in settings() {
        for hz in [50.0, 1000.0, 10_000.0] {
            let (dry, _, _) = fixture(tone(hz), None);
            let (wet, _, _) = fixture(tone(hz), Some(params));
            let dry = dry.play(RATE, RATE as usize, 113);
            let wet = wet.play(RATE, RATE as usize, 137);
            for (channel, channel_hz) in [(0, hz), (1, hz * 1.2)] {
                let extract = |audio: &[f32]| {
                    if channel == 0 {
                        left(audio)
                    } else {
                        right(audio)
                    }
                };
                let (dry, wet) = (extract(&dry), extract(&wet));
                let measured = f64::from(rms(&wet[24_000..]) / rms(&dry[24_000..]));
                let expected = magnitude(params, channel_hz);
                max_error = max_error.max((measured - expected).abs() / expected.max(1.0));
                assert!(
                    (measured - expected).abs() < 3e-4 * expected.max(1.0),
                    "{params:?}, {channel_hz} Hz: measured {measured}, reference {expected}"
                );
            }
            assert!(wet.iter().all(|x| x.is_finite()));
            assert!(
                peak(&wet) < 0.1,
                "the headroom fixture must remain unclipped"
            );
        }
    }
    eprintln!("54 routed tone measurements: max normalized gain error {max_error:e}");
}

#[test]
fn routed_stereo_impulses_match_independent_recurrences_and_block_partitions() {
    let mut max_error = 0.0_f32;
    let mut samples = vec![0.0; 8192 * 2];
    samples[512 * 2] = 0.05;
    samples[901 * 2 + 1] = -0.025;
    let audio = AudioBuffer::from_interleaved(RATE, 2, samples);
    let (dry, _, _) = fixture(audio.clone(), None);
    let dry = dry.play(RATE, 8192, 127);
    for params in settings() {
        let (rig, _, _) = fixture(audio.clone(), Some(params));
        let expected = reference_audio(params, &dry);
        let (mut processor, controller) = rig.processor(RATE);
        controller.play();
        assert_eq!(controller.latency_frames(), 0);
        let wet = guarded_run(&mut processor, 8192, 137);
        max_error = max_error.max(difference(&wet, &expected));
        assert_eq!(wet, rig.play(RATE, 8192, 1));
        assert!(
            difference(&wet, &expected) < 2e-6,
            "{params:?}: error {}",
            difference(&wet, &expected)
        );
        assert!(wet[..512 * 2].iter().all(|x| *x == 0.0));
        assert!(right(&wet)[..901].iter().all(|x| *x == 0.0));
    }
    eprintln!("9 routed stereo impulse paths: max absolute reference error {max_error:e}");
}

fn song(params: EffectParams) -> (Rig, TrackId, EffectId) {
    let (mut rig, track, effect) = fixture(tone(700.0), Some(params));
    let lane = rig.playlist_track();
    let pattern = rig.first_pattern();
    rig.clip(lane, pattern, 0, 960);
    (rig, track, effect.unwrap())
}

fn automate_all(rig: &mut Rig, track: TrackId, effect: EffectId, kind: EffectKind) {
    for index in 0..kind.descriptors().len() {
        let automation = rig.automation(
            AutomationTarget::EffectParam {
                track,
                effect,
                param: index as u32,
            },
            &[(0, 0.1), (240, 0.9), (480, 0.25), (720, 0.75), (960, 0.5)],
        );
        let lane = rig.playlist_track();
        rig.automation_clip(lane, automation, 0, 960);
    }
}

#[test]
fn descriptor_automation_is_audible_and_live_offline_and_both_stem_modes_agree() {
    for params in settings() {
        let (mut rig, track, effect) = song(params);
        automate_all(&mut rig, track, effect, params.kind());
        let options = RenderOptions {
            mode: PlayMode::Song,
            block_frames: 101,
            ..Default::default()
        };
        let exported = render(&rig.project, &rig.pool, &options, &mut |_| true);
        let (mut processor, controller) = rig.song_processor(RATE);
        controller.play();
        let played = guarded_run(&mut processor, exported.frames(), 137);
        assert_eq!(controller.latency_frames(), 0);
        assert_eq!(exported.samples(), played);
        assert_eq!(
            exported.samples(),
            rig.play_song(RATE, exported.frames(), 1)
        );
        let (static_rig, _, _) = song(params);
        let static_audio = render(&static_rig.project, &static_rig.pool, &options, &mut |_| {
            true
        });
        assert!(
            difference(exported.samples(), static_audio.samples()) > 1e-3,
            "{params:?}: automation must change the signal"
        );
        for mode in [StemMode::TrackOutputs, StemMode::ToMaster] {
            let stems_options = StemOptions {
                mode,
                tracks: Some(vec![track]),
                include_mix: true,
                numbered: false,
            };
            let list = stems(&rig.project, &stems_options).unwrap();
            let mut audio = vec![Vec::new(); list.len()];
            render_stems(
                &rig.project,
                &rig.pool,
                &options,
                &stems_options,
                &mut |index, block| {
                    audio[index].extend_from_slice(block);
                    true
                },
                &mut |_| true,
            )
            .unwrap();
            for samples in audio {
                assert_eq!(samples, exported.samples(), "{params:?}, stem {mode:?}");
            }
        }
    }
}

#[test]
fn silent_tail_and_automatic_export_keep_the_ringout_instead_of_truncating_it() {
    for params in settings() {
        let mut data = vec![0.0; 24_000 * 2];
        // The last sample belongs to the song. Its response belongs to the tail.
        data[23_999 * 2] = 0.05;
        let (mut rig, _, _) = fixture(AudioBuffer::from_interleaved(RATE, 2, data), Some(params));
        let lane = rig.playlist_track();
        let pattern = rig.first_pattern();
        rig.clip(lane, pattern, 0, 960);
        let options = RenderOptions {
            mode: PlayMode::Song,
            tail_secs: 1.0,
            auto_tail: true,
            block_frames: 101,
            ..Default::default()
        };
        let exported = render(&rig.project, &rig.pool, &options, &mut |_| true);
        assert!(
            exported.frames() > 24_000,
            "{params:?}: tail was cut at the song boundary"
        );
        assert!(
            peak(&exported.samples()[48_000..]) > 1e-5,
            "{params:?}: expected a real ringing tail"
        );
        assert!(exported.frames() <= 72_000);
        assert!(exported.samples().iter().all(|x| x.is_finite()));
        let played = rig.play_song(RATE, exported.frames(), 137);
        assert_eq!(exported.samples(), played);
    }
}

#[test]
fn edits_bypass_noops_removal_and_settled_restore_are_callback_allocation_free() {
    for params in settings() {
        let (mut rig, track, effect) = fixture(tone(500.0), Some(params));
        let effect = effect.unwrap();
        let (mut processor, controller) = rig.processor(RATE);
        controller.play();
        guarded_run(&mut processor, 4096, 137);
        controller.set_project(&rig.project, &rig.pool);
        let unchanged = guarded_run(&mut processor, 1024, 29);
        let reference = rig.play(RATE, 5120, 31);
        assert_eq!(unchanged, reference[4096 * 2..]);
        let mut edited = params;
        for (index, info) in params.kind().descriptors().iter().enumerate() {
            edited.set(index, info.min + 0.35 * (info.max - info.min));
        }
        rig.effect_mut(effect).params = edited;
        controller.set_project(&rig.project, &rig.pool);
        let changed = guarded_run(&mut processor, 2048, 7);
        assert!(changed.iter().all(|x| x.is_finite()));
        rig.effect_mut(effect).enabled = false;
        controller.set_project(&rig.project, &rig.pool);
        guarded_run(&mut processor, 2048, 113);
        rig.effect_mut(effect).enabled = true;
        controller.set_project(&rig.project, &rig.pool);
        guarded_run(&mut processor, 2048, 1);
        let slot = rig.remove_effect(effect);
        controller.set_project(&rig.project, &rig.pool);
        let removed = guarded_run(&mut processor, 4096, 137);
        let (dry_rig, _, _) = fixture(tone(500.0), None);
        let dry = dry_rig.play(RATE, 16_000, 101);
        assert_eq!(&removed[2048 * 2..], &dry[13_312 * 2..15_360 * 2]);
        rig.track_mut(track).effects.push(slot);
        controller.set_project(&rig.project, &rig.pool);
        let restored = guarded_run(&mut processor, 4096, 31);
        let restored_reference = rig.play(RATE, 19_456, 101);
        assert!(
            difference(
                &restored[2048 * 2..],
                &restored_reference[17_408 * 2..19_456 * 2]
            ) < 2e-5,
            "{params:?}: restored filter did not settle to the configured transfer"
        );
        assert_eq!(controller.latency_frames(), 0);
    }
}

#[test]
fn restore_during_removal_preserves_the_current_wet_history_and_splice_share() {
    // A restore after only 80 of the 240 departure frames must not open a
    // dry gap or restart a resonant integrator. This intentionally exercises
    // the utility owner's engine policy through the new filter kinds.
    for params in settings() {
        let (mut rig, track, effect) = fixture(tone(500.0), Some(params));
        let effect = effect.unwrap();
        let reference = rig.play(RATE, 4608, 127);
        let (dry, _, _) = fixture(tone(500.0), None);
        let dry = dry.play(RATE, 4608, 113);
        let (mut processor, controller) = rig.processor(RATE);
        controller.play();
        guarded_run(&mut processor, 4096, 137);
        let slot = rig.remove_effect(effect);
        controller.set_project(&rig.project, &rig.pool);
        let departing = guarded_run(&mut processor, 80, 7);
        let share = 161.0 / 240.0; // The last frame supplied before restore.
        for channel in 0..2 {
            let index = (4096 + 79) * 2 + channel;
            let expected = dry[index] + (reference[index] - dry[index]) * share;
            assert!((departing[79 * 2 + channel] - expected).abs() < 2e-6);
        }
        rig.track_mut(track).effects.push(slot);
        controller.set_project(&rig.project, &rig.pool);
        let restored = guarded_run(&mut processor, 432, 29);
        for (channel, &sample) in restored.iter().take(2).enumerate() {
            let index = (4096 + 80) * 2 + channel;
            let expected = dry[index] + (reference[index] - dry[index]) * share;
            // One frame of fade movement is allowed; a fresh dry splice or
            // fresh history is much larger than this bound.
            let tolerance = (reference[index] - dry[index]).abs() / 240.0 + 2e-6;
            assert!(
                (sample - expected).abs() <= tolerance,
                "{params:?}, channel {channel}: restore {}, expected {expected}, tolerance {tolerance}",
                sample
            );
        }
        assert!(difference(&restored[256 * 2..], &reference[4432 * 2..4608 * 2]) < 2e-6);
    }
}
