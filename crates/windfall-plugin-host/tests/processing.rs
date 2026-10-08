//! Running audio and events through CLAP plugins.

mod common;

use common::{GAIN, MIDI_SINE, MONO, SIDECHAIN, SINE, SINE_LATENCY, SWAP, create, gain, signal};
use windfall_plugin_host::{HostEvent, PluginError, ProcessStatus, Transport};

const RATE: f64 = 48_000.0;

/// The sine a voice of the test instrument produces, before its level.
fn sine(key: u8, frame: usize) -> f32 {
    let frequency = 440.0 * 2.0_f64.powf((f64::from(key) - 69.0) / 12.0);
    (std::f64::consts::TAU * frequency / RATE * frame as f64).sin() as f32
}

#[test]
fn a_plugin_is_described_by_its_file_and_by_itself() {
    let (module, instance) = create(GAIN);
    let descriptors = module.descriptors();
    assert!(descriptors.len() >= 12);
    let descriptor = instance.descriptor();
    assert_eq!(descriptor.name, "Test Gain");
    assert_eq!(descriptor.vendor, "Windfall tests");
    assert_eq!(descriptor.version, "1.2.3");
    assert_eq!(descriptor.kind, windfall_plugin_host::PluginKind::Effect);

    let layout = instance.layout();
    assert_eq!(layout.audio_inputs.len(), 1);
    assert_eq!(layout.audio_inputs[0].channels, 2);
    assert!(layout.audio_inputs[0].main);
    assert_eq!(layout.audio_outputs.len(), 1);
    assert_eq!(layout.note_inputs, 1);
    assert_eq!(layout.parameter_count, 9);
    assert!(layout.has_state);

    let (_module, sine) = create(SINE);
    assert_eq!(
        sine.descriptor().kind,
        windfall_plugin_host::PluginKind::Instrument
    );
    assert!(sine.layout().audio_inputs.is_empty());
    assert_eq!(sine.layout().audio_outputs[0].channels, 2);
}

#[test]
fn an_unknown_id_is_an_error_and_not_a_crash() {
    let (_, module) = common::load();
    let Err(error) = module.create("org.windfall.test.nothing") else {
        panic!("a plugin that does not exist was created");
    };
    assert_eq!(
        error,
        PluginError::NotFound("org.windfall.test.nothing".to_owned())
    );
}

#[test]
fn an_effect_that_allows_it_is_processed_in_place() {
    let (_module, mut instance) = create(GAIN);
    let mut processor = instance.activate(RATE, 256).unwrap();
    assert!(processor.has_audio_input());

    let (mut left, mut right) = signal(256);
    let (dry_left, dry_right) = (left.clone(), right.clone());
    processor.set_param(0, gain::GAIN, 0.5);
    assert_eq!(
        processor.process(&mut left, &mut right),
        ProcessStatus::Continue
    );
    for frame in 0..256 {
        assert_eq!(left[frame], dry_left[frame] * 0.5);
        assert_eq!(right[frame], dry_right[frame] * 0.5);
    }
    assert_eq!(instance.param_value(gain::IN_PLACE), Some(1.0));
    instance.deactivate(processor).unwrap();
    assert!(!instance.is_active());
}

#[test]
fn a_parameter_change_lands_on_its_frame() {
    let (_module, mut instance) = create(GAIN);
    let mut processor = instance.activate(RATE, 512).unwrap();
    let mut left = vec![1.0_f32; 400];
    let mut right = vec![1.0_f32; 400];
    // Queued out of order on purpose.
    processor.set_param(300, gain::GAIN, 2.0);
    processor.set_param(100, gain::GAIN, 0.25);
    processor.process(&mut left, &mut right);
    assert!(left[..100].iter().all(|&sample| sample == 1.0));
    assert!(left[100..300].iter().all(|&sample| sample == 0.25));
    assert!(left[300..].iter().all(|&sample| sample == 2.0));
    assert_eq!(left, right);

    // The next block starts where the last one ended.
    left.fill(1.0);
    right.fill(1.0);
    processor.process(&mut left, &mut right);
    assert!(left.iter().all(|&sample| sample == 2.0));
    instance.deactivate(processor).unwrap();
}

#[test]
fn a_long_block_is_split_and_its_events_stay_on_their_frames() {
    let (_module, mut instance) = create(GAIN);
    let mut processor = instance.activate(RATE, 64).unwrap();
    assert_eq!(processor.max_block(), 64);
    let mut left = vec![1.0_f32; 1000];
    let mut right = vec![1.0_f32; 1000];
    processor.set_param(63, gain::GAIN, 0.5);
    processor.set_param(64, gain::GAIN, 0.25);
    processor.set_param(777, gain::GAIN, 1.5);
    // Past the end of the block: it takes effect on the last frame.
    processor.set_param(5000, gain::GAIN, 0.125);
    processor.process(&mut left, &mut right);
    assert!(left[..63].iter().all(|&sample| sample == 1.0));
    assert_eq!(left[63], 0.5);
    assert!(left[64..777].iter().all(|&sample| sample == 0.25));
    assert!(left[777..999].iter().all(|&sample| sample == 1.5));
    assert_eq!(left[999], 0.125);
    instance.deactivate(processor).unwrap();
}

#[test]
fn notes_start_and_stop_on_their_frames_and_latency_is_reported() {
    let (_module, mut instance) = create(SINE);
    assert_eq!(
        instance.latency_samples(),
        0,
        "latency is known once active"
    );
    let mut processor = instance.activate(RATE, 512).unwrap();
    assert!(!processor.has_audio_input());
    assert_eq!(processor.latency_samples() as usize, SINE_LATENCY);
    assert_eq!(instance.latency_samples() as usize, SINE_LATENCY);

    // The buffers hold junk, which an instrument must replace.
    let mut left = vec![9.0_f32; 512];
    let mut right = vec![9.0_f32; 512];
    processor.note_on(100, 69, 1.0);
    processor.note_off(300, 69);
    assert_eq!(
        processor.process(&mut left, &mut right),
        ProcessStatus::Sleep
    );

    let start = 100 + SINE_LATENCY;
    let end = 300 + SINE_LATENCY;
    assert!(left[..start].iter().all(|&sample| sample == 0.0));
    assert!(left[end..].iter().all(|&sample| sample == 0.0));
    // The level parameter is 0.5 by default.
    for (frame, &sample) in left.iter().enumerate().take(end).skip(start) {
        let expected = sine(69, frame - start) * 0.5;
        assert!(
            (sample - expected).abs() < 1.0e-6,
            "frame {frame}: {} is not {expected}",
            sample
        );
    }
    assert_eq!(left, right);

    let notifications = common::idle(&mut instance);
    assert_eq!(
        notifications,
        [windfall_plugin_host::PluginNotification::NoteEnded {
            key: 69,
            channel: 0
        }]
    );
    instance.deactivate(processor).unwrap();
}

#[test]
fn a_note_that_spans_blocks_keeps_its_phase() {
    let (_module, mut instance) = create(SINE);
    let mut processor = instance.activate(RATE, 128).unwrap();
    let mut output = Vec::new();
    processor.note_on(0, 60, 0.5);
    for _ in 0..4 {
        let mut left = vec![0.0_f32; 100];
        let mut right = vec![0.0_f32; 100];
        assert_eq!(
            processor.process(&mut left, &mut right),
            ProcessStatus::Continue
        );
        output.extend(left);
    }
    for (frame, sample) in output.iter().enumerate().skip(SINE_LATENCY) {
        let expected = sine(60, frame - SINE_LATENCY) * 0.5 * 0.5;
        assert!((sample - expected).abs() < 1.0e-6, "frame {frame}");
    }
    instance.deactivate(processor).unwrap();
}

#[test]
fn a_plugin_that_only_speaks_midi_gets_midi() {
    let (_module, mut instance) = create(MIDI_SINE);
    let mut processor = instance.activate(RATE, 256).unwrap();
    let mut left = vec![0.0_f32; 256];
    let mut right = vec![0.0_f32; 256];
    processor.note_on(10, 69, 1.0);
    processor.process(&mut left, &mut right);
    let start = 10 + SINE_LATENCY;
    assert!(left[..start].iter().all(|&sample| sample == 0.0));
    for (frame, &sample) in left.iter().enumerate().take(256).skip(start) {
        let expected = sine(69, frame - start) * 0.5;
        assert!((sample - expected).abs() < 1.0e-6, "frame {frame}");
    }

    // Stopping everything reaches a MIDI plugin as controller messages.
    processor.all_notes_off(0);
    left.fill(0.0);
    right.fill(0.0);
    processor.process(&mut left, &mut right);
    assert!(left[SINE_LATENCY..].iter().all(|&sample| sample == 0.0));
    instance.deactivate(processor).unwrap();
}

#[test]
fn all_notes_off_silences_a_clap_instrument() {
    let (_module, mut instance) = create(SINE);
    let mut processor = instance.activate(RATE, 256).unwrap();
    let mut left = vec![0.0_f32; 256];
    let mut right = vec![0.0_f32; 256];
    for key in [60, 64, 67] {
        processor.note_on(0, key, 0.7);
    }
    processor.process(&mut left, &mut right);
    assert!(left[SINE_LATENCY + 1..].iter().any(|&sample| sample != 0.0));

    processor.push_event(HostEvent::AllNotesOff { time: 0 });
    processor.process(&mut left, &mut right);
    assert!(left[SINE_LATENCY..].iter().all(|&sample| sample == 0.0));
    instance.deactivate(processor).unwrap();
}

#[test]
fn a_plugin_that_forbids_it_is_not_processed_in_place() {
    // The swap plugin writes the left output before it reads the left
    // input. Sharing the memory would give it its own output as input.
    let (_module, mut instance) = create(SWAP);
    let mut processor = instance.activate(RATE, 256).unwrap();
    let (mut left, mut right) = signal(256);
    let (dry_left, dry_right) = (left.clone(), right.clone());
    processor.process(&mut left, &mut right);
    assert_eq!(left, dry_right);
    assert_eq!(right, dry_left);
    instance.deactivate(processor).unwrap();
}

#[test]
fn every_port_a_plugin_declares_gets_a_buffer() {
    // The sidechain plugin reports an error if any of its four ports is
    // missing. Its sidechain reads silence, so the main signal passes.
    let (_module, mut instance) = create(SIDECHAIN);
    assert_eq!(instance.layout().audio_inputs.len(), 2);
    assert!(!instance.layout().audio_inputs[1].main);
    let mut processor = instance.activate(RATE, 256).unwrap();
    let (mut left, mut right) = signal(256);
    let (dry_left, dry_right) = (left.clone(), right.clone());
    assert_eq!(
        processor.process(&mut left, &mut right),
        ProcessStatus::Continue
    );
    assert_eq!(left, dry_left);
    assert_eq!(right, dry_right);
    assert!(!processor.health().failed);
    // The plugin's tail extension is read on the audio thread.
    assert_eq!(processor.tail_samples(), Some(4_800));
    instance.deactivate(processor).unwrap();
}

#[test]
fn a_mono_plugin_gets_the_sum_and_feeds_both_sides() {
    let (_module, mut instance) = create(MONO);
    let mut processor = instance.activate(RATE, 256).unwrap();
    let (mut left, mut right) = signal(256);
    let (dry_left, dry_right) = (left.clone(), right.clone());
    processor.process(&mut left, &mut right);
    for frame in 0..256 {
        // The plugin halves what it is given.
        let expected = (dry_left[frame] + dry_right[frame]) * 0.5 * 0.5;
        assert_eq!(left[frame], expected);
        assert_eq!(right[frame], expected);
    }
    instance.deactivate(processor).unwrap();
}

#[test]
fn the_plugin_is_told_where_the_song_is() {
    let (_module, mut instance) = create(GAIN);
    let mut processor = instance.activate(RATE, 512).unwrap();
    let mut left = vec![0.0_f32; 480];
    let mut right = vec![0.0_f32; 480];

    processor.process(&mut left, &mut right);
    assert_eq!(instance.param_value(gain::PLAYING), Some(0.0));
    assert_eq!(instance.param_value(gain::TEMPO), Some(120.0));

    processor.set_transport(Transport {
        playing: true,
        tempo_bpm: 150.0,
        position_beats: 8.0,
        position_seconds: 3.2,
        numerator: 3,
        denominator: 4,
    });
    processor.process(&mut left, &mut right);
    assert_eq!(instance.param_value(gain::PLAYING), Some(1.0));
    assert_eq!(instance.param_value(gain::TEMPO), Some(150.0));
    assert_eq!(instance.param_value(gain::BEATS), Some(8.0));

    // The position moves on by itself while the song plays: 480 frames at
    // 48 kHz and 150 bpm are 0.025 beats.
    processor.process(&mut left, &mut right);
    let beats = instance.param_value(gain::BEATS).unwrap();
    assert!((beats - 8.025).abs() < 1.0e-6, "{beats}");

    processor.set_tempo(90.0);
    processor.process(&mut left, &mut right);
    assert_eq!(instance.param_value(gain::TEMPO), Some(90.0));
    instance.deactivate(processor).unwrap();
}

#[test]
fn denormals_are_flushed_while_the_plugin_runs_and_not_after() {
    let (_module, mut instance) = create(GAIN);
    let mut processor = instance.activate(RATE, 64).unwrap();
    let mut left = vec![0.0_f32; 64];
    let mut right = vec![0.0_f32; 64];
    processor.process(&mut left, &mut right);
    if cfg!(any(target_arch = "x86_64", target_arch = "aarch64")) {
        assert_eq!(instance.param_value(gain::DENORMALS_FLUSHED), Some(1.0));
    }
    let tiny = std::hint::black_box(f32::MIN_POSITIVE) * std::hint::black_box(0.5_f32);
    assert!(tiny > 0.0, "the host's own arithmetic is left as it was");
    instance.deactivate(processor).unwrap();
}

#[test]
fn a_plugin_can_be_activated_again_at_another_rate() {
    let (_module, mut instance) = create(SINE);
    for rate in [44_100.0, 96_000.0] {
        let mut processor = instance.activate(rate, 256).unwrap();
        assert_eq!(processor.sample_rate(), rate);
        assert!(matches!(
            instance.activate(rate, 256),
            Err(PluginError::AlreadyActive)
        ));
        let mut left = vec![0.0_f32; 256];
        let mut right = vec![0.0_f32; 256];
        processor.note_on(0, 69, 1.0);
        processor.process(&mut left, &mut right);
        // One period of 440 Hz is rate / 440 frames: a quarter of it in,
        // the sine is at its peak of half the level.
        let quarter = SINE_LATENCY + (rate / 440.0 / 4.0).round() as usize;
        assert!((left[quarter] - 0.5).abs() < 1.0e-3, "{}", left[quarter]);
        processor.stop();
        instance.deactivate(processor).unwrap();
    }
}

#[test]
fn reset_ends_notes_and_forgets_queued_events() {
    let (_module, mut instance) = create(SINE);
    let mut processor = instance.activate(RATE, 256).unwrap();
    let mut left = vec![0.0_f32; 256];
    let mut right = vec![0.0_f32; 256];
    processor.note_on(0, 60, 1.0);
    processor.process(&mut left, &mut right);
    processor.note_on(0, 72, 1.0);
    processor.reset();
    processor.process(&mut left, &mut right);
    assert!(left.iter().all(|&sample| sample == 0.0));
    instance.deactivate(processor).unwrap();
}

#[test]
fn a_processor_from_another_plugin_is_refused() {
    let (_first_module, mut first) = create(GAIN);
    let (_second_module, mut second) = create(GAIN);
    let processor = first.activate(RATE, 64).unwrap();
    let other = second.activate(RATE, 64).unwrap();
    let other = first.deactivate(other).unwrap_err().returned;
    second.deactivate(other).unwrap();
    assert!(first.is_active(), "the wrong processor changed nothing");
    first.deactivate(processor).unwrap();
    assert!(!first.is_active());
}

#[test]
fn invalid_events_do_not_cross_the_plugin_abi() {
    let (_module, mut instance) = create(GAIN);
    let mut processor = instance.activate(RATE, 64).unwrap();
    assert!(!processor.note_on(0, 60, f32::NAN));
    assert!(!processor.push_event(HostEvent::NoteOn {
        time: 0,
        key: 255,
        channel: 0,
        velocity: 0.5
    }));
    assert!(!processor.push_event(HostEvent::NoteOff {
        time: 0,
        key: 60,
        channel: 16,
        velocity: 0.0
    }));
    assert!(!processor.set_param(0, gain::GAIN, f64::INFINITY));
    assert!(!processor.set_param(0, u32::MAX, 1.0));
    let mut left = [1.0; 64];
    let mut right = [1.0; 64];
    processor.process(&mut left, &mut right);
    assert_eq!(left, [1.0; 64]);
    assert_eq!(processor.health().dropped_events, 5);
    instance.deactivate(processor).unwrap();
}
