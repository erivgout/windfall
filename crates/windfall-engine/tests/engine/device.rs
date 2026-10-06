//! The device layer, as far as it can be tested without opening a device.

use windfall_engine::{Controller, Engine, Processor, SamplePool};
use windfall_ipc::{AudioSettings, PlayMode, TransportPatch};
use windfall_project::{ChannelId, Project};

use crate::support::{Rig, impulse, level};

fn assert_send_and_sync<T: Send + Sync>() {}
fn assert_send<T: Send>() {}

#[test]
fn the_handles_can_cross_threads() {
    assert_send_and_sync::<Engine>();
    assert_send_and_sync::<Controller>();
    assert_send_and_sync::<SamplePool>();
    assert_send::<Processor>();
}

#[test]
fn an_impossible_device_gives_a_stopped_engine_with_a_reason() {
    let settings = AudioSettings {
        device: Some("no such device, not on any machine".to_owned()),
        sample_rate: Some(48_000),
        buffer_frames: Some(128),
        ..AudioSettings::default()
    };
    let engine = Engine::start(&settings);
    let status = engine.status();
    assert!(!status.running);
    assert_eq!(status.device, settings.device);
    let error = status.error.expect("a stopped engine says why");
    assert!(!error.is_empty());

    // Everything on the controller is still safe to call.
    let controller = engine.controller();
    let mut rig = Rig::new();
    let channel = rig.channel(impulse(48_000));
    rig.steps(channel, &[0]);
    controller.set_project(&rig.project, &rig.pool);
    controller.play();
    controller.note_on(channel, 60, 1.0);
    controller.note_off(channel, 60);
    controller.preview(level(48_000, 0.1, 0.1));
    controller.stop_preview();
    controller.set_output_gain(0.5);
    controller.seek(960.0);
    controller.set_transport(TransportPatch {
        loop_song: Some(true),
        ..TransportPatch::default()
    });

    // With no stream nothing can play, but settings and playhead are kept.
    let transport = controller.transport();
    assert!(!transport.playing);
    assert!(transport.loop_song);
    assert_eq!(transport.pattern, rig.first_pattern());
    let frame = controller.frame();
    assert!(!frame.playing);
    assert_eq!(frame.tick, 960.0);
    assert_eq!(frame.meters, vec![0.0, 0.0]);
    assert_eq!(controller.stream_stats().xruns, 0);
    controller.stop();
}

#[test]
fn reconfiguring_to_an_impossible_host_reports_it() {
    let engine = Engine::start(&AudioSettings {
        device: Some("no such device, not on any machine".to_owned()),
        ..AudioSettings::default()
    });
    engine.reconfigure(&AudioSettings {
        host: Some("no such host".to_owned()),
        ..AudioSettings::default()
    });
    let status = engine.status();
    assert!(!status.running);
    assert!(status.error.expect("an error").contains("no such host"));
}

#[test]
fn listing_devices_does_not_panic() {
    for host in Engine::devices() {
        assert!(!host.name.is_empty());
        for device in host.devices {
            assert!(device.sample_rates.is_sorted());
            if let (Some(min), Some(max)) = (device.min_buffer_frames, device.max_buffer_frames) {
                assert!(min <= max);
            }
        }
    }
}

#[test]
fn a_controller_reports_its_requests_before_the_audio_thread_catches_up() {
    let (mut processor, controller) = Processor::new(48_000);
    let project = Project::new("empty");
    controller.set_project(&project, &SamplePool::new());

    controller.play();
    assert!(controller.transport().playing);
    controller.stop();
    assert!(!controller.transport().playing);
    controller.play();
    controller.set_transport(TransportPatch {
        mode: Some(PlayMode::Song),
        ..TransportPatch::default()
    });
    assert_eq!(controller.transport().mode, PlayMode::Song);
    assert!(controller.transport().playing);

    // Once it has, the audio thread's word counts: an empty song cannot
    // play.
    let mut out = [0.0; 256];
    processor.process(&mut out);
    assert!(!controller.transport().playing);
    assert!(!controller.frame().playing);
}

#[test]
fn a_flood_of_requests_is_queued_and_none_is_lost() {
    let mut rig = Rig::new();
    let channel = rig.channel(level(48_000, 0.001, 1.0));
    let (mut processor, controller) = rig.processor(48_000);
    // Far more than the queue to the audio thread holds.
    for _ in 0..3_000 {
        controller.note_on(ChannelId(9_999), 60, 1.0);
    }
    controller.note_on(channel, 60, 1.0);

    let mut out = [0.0; 256];
    for _ in 0..8 {
        processor.process(&mut out);
        controller.frame();
    }
    assert_eq!(controller.frame().voices, 1);
}
