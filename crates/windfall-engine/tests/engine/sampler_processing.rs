//! Independent estimates and shared realtime/export sampler policy.
use crate::realtime::allocator_calls;
use crate::support::{Rig, left, run, sine};
use windfall_core::AudioBuffer;
use windfall_engine::{
    Controller, RenderOptions, SamplePool, render_reporting, render_streaming_checked,
};
use windfall_project::{ClipStretchQuality, SamplerKeyRange, SamplerLoopMode, SamplerStretch};

const RATE: u32 = 48_000;
fn spectral(rig: &mut Rig, channel: windfall_project::ChannelId, ratio: f64, first: u8, last: u8) {
    rig.sampler_mut(channel).stretch = SamplerStretch::Spectral {
        ratio,
        quality: ClipStretchQuality::Standard,
        formants: false,
        range: SamplerKeyRange { first, last },
    };
}
fn prepared(rig: &mut Rig) {
    rig.pool = rig
        .pool
        .prepare_samplers(&rig.project, &mut || true, &mut |_, _, _| {})
        .unwrap();
}

/// Linear interpolation of positive zero crossings, independently of the DSP's
/// FFT/peak estimator. Only stationary interior audio enters this measurement.
fn estimate(audio: &[f32]) -> f64 {
    let mut crossings = Vec::new();
    for (index, pair) in audio.windows(2).enumerate() {
        if pair[0] <= 0.0 && pair[1] > 0.0 {
            crossings.push(index as f64 - f64::from(pair[0]) / f64::from(pair[1] - pair[0]));
        }
    }
    assert!(crossings.len() > 20);
    (crossings.len() - 1) as f64 * f64::from(RATE) / (crossings.last().unwrap() - crossings[0])
}

#[test]
fn sampler_spectral_duration_and_pitch_are_independent_with_tenth_cent_sine_bound() {
    let mut worst_cents = 0.0_f64;
    for ratio in [0.5, 1.0, 2.0] {
        let mut rig = Rig::new();
        let channel = rig.channel(sine(RATE, 440.0, 2.0));
        spectral(&mut rig, channel, ratio, 48, 72);
        prepared(&mut rig);
        let frames = (96_000.0 * ratio) as usize;
        for key in [48, 60, 72] {
            let (mut processor, controller) = rig.processor(RATE);
            controller.note_on(channel, key, 1.0);
            let audio = left(&run(&mut processor, frames + 300, 257));
            let frequency = estimate(&audio[frames / 4..frames * 3 / 4]);
            let wanted = 440.0 * 2_f64.powf((f64::from(key) - 60.0) / 12.0);
            let cents = 1200.0 * (frequency / wanted).log2();
            worst_cents = worst_cents.max(cents.abs());
            assert!(
                cents.abs() <= 0.1,
                "ratio {ratio}, key {key}: {frequency} Hz, {cents} cents"
            );
            assert!(
                audio[frames - 1000..frames - 500]
                    .iter()
                    .any(|sample| sample.abs() > 0.05)
            );
            assert!(audio[frames..].iter().all(|sample| *sample == 0.0));
        }
    }
    eprintln!(
        "Prepared sampler stationary 440 Hz fixture, ±12 semitones and ratios 0.5/1/2: worst frequency error {worst_cents:.8} cent. This is not musical listening evidence."
    );
}

#[test]
fn sampler_prepared_range_edges_unsupported_keys_and_export_are_bit_identical() {
    let mut rig = Rig::new();
    let channel = rig.channel(sine(RATE, 440.0, 0.25));
    spectral(&mut rig, channel, 1.5, 59, 61);
    prepared(&mut rig);
    for key in [58, 59, 60, 61, 62] {
        rig.project.patterns[0].lanes.clear();
        rig.note(channel, 0, 240).key = key;
        let options = RenderOptions {
            block_frames: 19,
            ..Default::default()
        };
        let exported = render_reporting(&rig.project, &rig.pool, &options, &mut |_| true);
        assert!(exported.sampler_error.is_none());
        let playback = rig.play(RATE, exported.audio.frames(), 257);
        assert_eq!(playback, exported.audio.samples(), "key {key}");
        assert_eq!(playback, rig.play(RATE, exported.audio.frames(), 1024));
        let mut streamed = Vec::new();
        render_streaming_checked(
            &rig.project,
            &rig.pool,
            &options,
            &mut |block| {
                streamed.extend_from_slice(block);
                true
            },
            &mut |_| true,
        )
        .unwrap();
        assert_eq!(playback, streamed);
        assert_eq!(
            playback.iter().any(|sample| *sample != 0.0),
            (59..=61).contains(&key)
        );
        let (_, controller) = rig.processor(RATE);
        assert_eq!(
            controller.sampler_key_supported(channel, key),
            (59..=61).contains(&key)
        );
    }
}

#[test]
fn sampler_spectral_trim_reverse_fractional_one_frame_loops_and_release() {
    for mode in [SamplerLoopMode::Forward, SamplerLoopMode::PingPong] {
        for reverse in [false, true] {
            for frames in [1, 4, 13] {
                let mut rig = Rig::new();
                let channel = rig.channel(AudioBuffer::from_interleaved(
                    44_100,
                    2,
                    vec![0.25; frames * 2],
                ));
                spectral(&mut rig, channel, 1.0, 60, 60);
                let settings = rig.sampler_mut(channel);
                settings.reverse = reverse;
                settings.start = 0.2;
                settings.end = 0.8;
                settings.loop_mode = mode;
                settings.loop_start = 0.333;
                settings.loop_end = 0.334;
                prepared(&mut rig);
                let (mut processor, controller) = rig.processor(RATE);
                controller.note_on(channel, 60, 1.0);
                assert!(
                    run(&mut processor, 1000, 17)
                        .iter()
                        .all(|sample| *sample == 0.25)
                );
                controller.note_off(channel, 60);
                let released = run(&mut processor, 500, 13);
                assert!(released[..50].iter().any(|sample| *sample > 0.0));
                assert!(released[400..].iter().all(|sample| *sample == 0.0));
            }
        }
    }
    let mut rig = Rig::new();
    let channel = rig.channel(AudioBuffer::from_interleaved(
        RATE,
        1,
        vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5],
    ));
    spectral(&mut rig, channel, 1.0, 60, 60);
    let settings = rig.sampler_mut(channel);
    settings.start = 1.0 / 6.0;
    settings.end = 5.0 / 6.0;
    settings.reverse = true;
    settings.loop_mode = SamplerLoopMode::PingPong;
    settings.loop_start = 0.25;
    settings.loop_end = 0.75;
    prepared(&mut rig);
    rig.note(channel, 0, 240);
    assert_eq!(
        left(&rig.play(RATE, 8, 3)),
        vec![0.4, 0.3, 0.2, 0.3, 0.2, 0.3, 0.2, 0.3]
    );
}

#[test]
fn sampler_first_notes_edges_cuts_320_voices_reload_eviction_and_retirement_do_not_touch_allocator()
{
    let mut rig = Rig::new();
    let channel = rig.channel(sine(RATE, 440.0, 0.1));
    spectral(&mut rig, channel, 1.0, 59, 61);
    rig.sampler_mut(channel).loop_mode = SamplerLoopMode::PingPong;
    prepared(&mut rig);
    let (mut processor, controller) = rig.processor(RATE);
    let mut block = [0.0; 514];
    for key in [58, 59, 60, 61, 62] {
        controller.note_on(channel, key, 1.0);
        assert_eq!(allocator_calls(|| processor.process(&mut block)), 0);
    }
    // Republishing identical settings reuses the bank without warming note-on.
    let retained = rig.pool.sampler_retained_bytes();
    controller.set_project(&rig.project, &rig.pool);
    assert_eq!(rig.pool.sampler_retained_bytes(), retained);
    assert_eq!(allocator_calls(|| processor.process(&mut block)), 0);
    for index in 0..320 {
        controller.note_on(channel, 59 + (index % 3) as u8, 0.5);
    }
    assert_eq!(allocator_calls(|| processor.process(&mut block)), 0);
    for key in [59, 60, 61] {
        controller.note_off(channel, key);
    }
    rig.sampler_mut(channel).cut_self = true;
    spectral(&mut rig, channel, 0.5, 60, 60);
    let sample = rig.sampler_mut(channel).sample.unwrap();
    rig.pool.insert(sample, sine(RATE, 220.0, 0.1));
    prepared(&mut rig);
    controller.set_project(&rig.project, &rig.pool);
    controller.note_on(channel, 60, 0.5);
    for _ in 0..20 {
        assert_eq!(allocator_calls(|| processor.process(&mut block)), 0);
        controller.transport(); // drain retired plans and banks on control
    }
    controller.stop();
    for _ in 0..20 {
        assert_eq!(allocator_calls(|| processor.process(&mut block)), 0);
        controller.transport();
    }
}

#[test]
fn sampler_missing_empty_and_full_midi_range_are_bounded_and_have_no_tape_fallback() {
    let mut rig = Rig::new();
    let channel = rig.channel(AudioBuffer::from_interleaved(RATE, 1, vec![0.25; 8]));
    spectral(&mut rig, channel, 1.0, 0, 127);
    prepared(&mut rig);
    let (_, controller) = rig.processor(RATE);
    for key in [0, 127] {
        assert!(controller.sampler_key_supported(channel, key));
    }
    let sample = rig.sampler_mut(channel).sample.unwrap();
    rig.pool.remove(sample);
    rig.note(channel, 0, 240);
    assert!(rig.play(RATE, 100, 13).iter().all(|sample| *sample == 0.0));
    rig.pool
        .insert(sample, AudioBuffer::from_interleaved(RATE, 1, Vec::new()));
    assert!(rig.play(RATE, 100, 13).iter().all(|sample| *sample == 0.0));
}

#[test]
fn sampler_cache_reservations_include_retained_banks_and_fail_without_mutating_the_pool() {
    let mut rig = Rig::new();
    let channel = rig.channel(AudioBuffer::from_interleaved(RATE, 1, vec![0.25; 512]));
    let mut pool = SamplePool::with_sampler_budget(25_000);
    for (id, audio) in rig.pool.iter() {
        pool.insert(id, audio.clone());
    }
    rig.pool = pool;
    spectral(&mut rig, channel, 1.0, 60, 60);
    prepared(&mut rig);
    let original = rig.pool.sampler_retained_bytes();
    let held = Controller::prepare_project(&rig.project, &rig.pool).unwrap();
    spectral(&mut rig, channel, 4.0, 48, 72);
    let error = match Controller::prepare_project(&rig.project, &rig.pool) {
        Ok(_) => panic!("budget must refuse"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("budget"));
    let render = render_reporting(
        &rig.project,
        &rig.pool,
        &RenderOptions::default(),
        &mut |_| true,
    );
    assert!(render.sampler_error.is_some());
    assert_eq!(render.audio.frames(), 0);
    assert!(
        render_streaming_checked(
            &rig.project,
            &rig.pool,
            &RenderOptions::default(),
            &mut |_| true,
            &mut |_| true
        )
        .is_err()
    );
    assert_eq!(rig.pool.sampler_retained_bytes(), original);
    assert!(rig.pool.needs_sampler_preparation(&rig.project));
    drop(held);
    let mut calls = 0;
    let error = rig
        .pool
        .prepare_samplers(
            &rig.project,
            &mut || {
                calls += 1;
                false
            },
            &mut |_, _, _| {},
        )
        .unwrap_err();
    assert_eq!(
        error,
        windfall_engine::sampler_processing::SamplerPreparationError::Cancelled
    );
    assert_eq!(calls, 1);
    assert_eq!(rig.pool.sampler_retained_bytes(), original);
    spectral(&mut rig, channel, 1.0, 60, 60);
    let reused = rig
        .pool
        .prepare_samplers(
            &rig.project,
            &mut || panic!("cache hit must skip DSP"),
            &mut |_, _, _| {},
        )
        .unwrap();
    assert_eq!(reused.sampler_retained_bytes(), original);
}

#[test]
fn sampler_reload_evicts_cache_but_old_voice_bank_stays_charged_until_control_retirement() {
    let mut rig = Rig::new();
    let channel = rig.channel(AudioBuffer::from_interleaved(RATE, 1, vec![0.25; 512]));
    spectral(&mut rig, channel, 1.0, 60, 60);
    rig.sampler_mut(channel).loop_mode = SamplerLoopMode::Forward;
    prepared(&mut rig);
    let one = rig.pool.sampler_retained_bytes();
    let (mut processor, controller) = rig.processor(RATE);
    controller.note_on(channel, 60, 1.0);
    assert!(
        run(&mut processor, 100, 17)
            .iter()
            .all(|sample| *sample == 0.25)
    );
    let sample = rig.sampler_mut(channel).sample.unwrap();
    rig.pool.insert(
        sample,
        AudioBuffer::from_interleaved(RATE, 1, vec![-0.5; 512]),
    );
    assert!(rig.pool.needs_sampler_preparation(&rig.project));
    prepared(&mut rig);
    let both = rig.pool.sampler_retained_bytes();
    assert!(both >= one * 2);
    controller.set_project(&rig.project, &rig.pool);
    assert!(
        run(&mut processor, 500, 13)
            .iter()
            .all(|sample| *sample == 0.25)
    );
    controller.transport();
    assert_eq!(rig.pool.sampler_retained_bytes(), both);
    controller.note_off(channel, 60);
    run(&mut processor, 1000, 31);
    controller.transport();
    let mut retirement = windfall_engine::ProjectRetirement::default();
    controller.take_retired(&mut retirement);
    drop(retirement);
    assert_eq!(rig.pool.sampler_retained_bytes(), one);
    controller.note_on(channel, 60, 1.0);
    assert!(
        run(&mut processor, 100, 17)
            .iter()
            .all(|sample| *sample == -0.5)
    );
}

#[test]
fn sampler_cancellation_inside_a_render_chunk_releases_private_reservations() {
    let mut rig = Rig::new();
    let channel = rig.channel(sine(RATE, 440.0, 1.0));
    spectral(&mut rig, channel, 2.0, 48, 72);
    let mut checks = 0;
    let result = rig.pool.prepare_samplers(
        &rig.project,
        &mut || {
            checks += 1;
            checks < 5
        },
        &mut |_, _, _| panic!("the first key must not finish"),
    );
    assert!(matches!(
        result,
        Err(windfall_engine::sampler_processing::SamplerPreparationError::Cancelled)
    ));
    assert_eq!(checks, 5);
    assert_eq!(rig.pool.sampler_retained_bytes(), 0);
    assert!(rig.pool.needs_sampler_preparation(&rig.project));
}

#[test]
fn sampler_format_limits_are_reported_before_any_bank_reservation() {
    for (rate, channels) in [(7_999, 1), (192_001, 2), (48_000, 3)] {
        let mut rig = Rig::new();
        let channel = rig.channel(AudioBuffer::from_interleaved(
            rate,
            channels,
            vec![0.25; usize::from(channels) * 32],
        ));
        spectral(&mut rig, channel, 1.0, 60, 60);
        let error = match Controller::prepare_project(&rig.project, &rig.pool) {
            Ok(_) => panic!("unsupported source must refuse"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("mono/stereo"));
        assert_eq!(rig.pool.sampler_retained_bytes(), 0);
    }
}

#[test]
fn sampler_unsupported_keys_do_not_cut_a_supported_looping_voice() {
    let mut rig = Rig::new();
    let channel = rig.channel(AudioBuffer::from_interleaved(RATE, 1, vec![0.25; 512]));
    spectral(&mut rig, channel, 1.0, 60, 60);
    let settings = rig.sampler_mut(channel);
    settings.cut_self = true;
    settings.loop_mode = SamplerLoopMode::Forward;
    prepared(&mut rig);
    let (mut processor, controller) = rig.processor(RATE);
    controller.note_on(channel, 60, 1.0);
    for key in [59, 61, 0, 127] {
        controller.note_on(channel, key, 1.0);
        assert!(
            run(&mut processor, 700, 31)
                .iter()
                .all(|sample| *sample == 0.25)
        );
    }
    controller.note_off(channel, 60);
    assert!(
        run(&mut processor, 600, 17)[400..]
            .iter()
            .all(|sample| *sample == 0.0)
    );
}

#[test]
fn sampler_transformed_fractional_loops_keep_block_and_export_parity_through_release() {
    for ratio in [0.5, 2.0] {
        for mode in [SamplerLoopMode::Forward, SamplerLoopMode::PingPong] {
            for reverse in [false, true] {
                let mut rig = Rig::new();
                let channel = rig.channel(sine(44_100, 440.0, 0.1));
                let key = if reverse { 48 } else { 72 };
                spectral(&mut rig, channel, ratio, key, key);
                let settings = rig.sampler_mut(channel);
                settings.reverse = reverse;
                settings.start = 0.125;
                settings.end = 0.875;
                settings.loop_mode = mode;
                settings.loop_start = 0.1;
                settings.loop_end = 0.9;
                prepared(&mut rig);
                rig.note(channel, 0, 240).key = key;
                let options = RenderOptions {
                    block_frames: 17,
                    ..Default::default()
                };
                let exported = render_reporting(&rig.project, &rig.pool, &options, &mut |_| true);
                assert!(exported.sampler_error.is_none());
                let playback = rig.play(RATE, exported.audio.frames(), 257);
                assert_eq!(playback, exported.audio.samples());
                assert_eq!(playback, rig.play(RATE, exported.audio.frames(), 1024));
                assert!(
                    playback[1000..10_000]
                        .iter()
                        .any(|sample| sample.abs() > 0.01)
                );
                // A sixteenth note at 120 BPM ends at frame 6000;
                // a looping sampler without ADSR then closes in four ms.
                assert!(playback[13_000..].iter().all(|sample| *sample == 0.0));
            }
        }
    }
}
