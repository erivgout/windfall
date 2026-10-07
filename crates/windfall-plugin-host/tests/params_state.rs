//! Parameters and saved state of CLAP plugins.

mod common;

use common::{GAIN, MIDI_SINE, SINE, SINE_LEVEL, create, gain, idle};
use windfall_dsp::ParamKind;
use windfall_plugin_host::{PluginError, PluginNotification, PluginState};

const RATE: f64 = 48_000.0;

#[test]
fn retirement_flushes_more_parameter_edits_than_one_clap_list_can_hold() {
    for transfer_without_audio in [false, true] {
        retirement_flush(transfer_without_audio);
    }
}

fn retirement_flush(transfer_without_audio: bool) {
    let (_module, mut instance) = create(GAIN);
    let mut processor = instance.activate(RATE, 64).unwrap();
    // Include the audio-side pending list as well as the full main-thread
    // queue. Retirement must preserve their order without truncating either.
    for _ in 0..windfall_plugin_host::EVENT_CAPACITY {
        assert!(processor.set_param(0, gain::GAIN, 0.125));
    }
    for _ in 0..4095 {
        assert!(instance.set_param(gain::GAIN, 0.25));
    }
    assert!(instance.set_param(gain::GAIN, 0.75));
    assert!(
        !instance.set_param(gain::GAIN, 1.5),
        "the queue stays bounded"
    );
    if transfer_without_audio {
        processor.process(&mut [], &mut []);
    }
    instance.deactivate(processor);
    assert_eq!(instance.param_value(gain::GAIN), Some(0.75));
    let saved = instance.save_state().unwrap();

    assert!(instance.set_param(gain::GAIN, 1.0));
    instance.load_state(&saved).unwrap();
    let mut processor = instance.activate(RATE, 64).unwrap();
    let mut left = [1.0; 64];
    let mut right = left;
    processor.process(&mut left, &mut right);
    assert_eq!(left, [0.75; 64]);
    assert_eq!(right, left);
    instance.deactivate(processor);
    assert_eq!(instance.save_state().unwrap(), saved);
}

#[test]
fn parameters_are_listed_with_their_ranges_and_flags() {
    let (_module, instance) = create(GAIN);
    let params = instance.params();
    assert_eq!(params.len(), 9);

    let level = &params[0];
    assert_eq!(level.id, gain::GAIN);
    assert_eq!(level.name, "Gain");
    assert_eq!(level.module, "Main");
    assert_eq!((level.min, level.max, level.default), (0.0, 2.0, 1.0));
    assert!(level.automatable && !level.stepped && !level.read_only);
    assert_eq!(level.kind(), ParamKind::Float);

    let mode = instance.param(gain::MODE).unwrap();
    assert!(mode.stepped && mode.choice);
    assert_eq!(mode.kind(), ParamKind::Choice);

    let reading = instance.param(gain::TIMER_TICKS).unwrap();
    assert!(reading.read_only && !reading.automatable);
    assert_eq!(reading.module, "Readings");
    assert!(instance.param(12_345).is_none());
}

#[test]
fn the_plugin_turns_values_into_text_and_back() {
    let (_module, mut instance) = create(GAIN);
    assert_eq!(
        instance.param_text(gain::GAIN, 1.0).as_deref(),
        Some("0.0 dB")
    );
    assert_eq!(
        instance.param_text(gain::GAIN, 0.5).as_deref(),
        Some("-6.0 dB")
    );
    assert_eq!(
        instance.param_text(gain::GAIN, 0.0).as_deref(),
        Some("-inf dB")
    );
    assert_eq!(
        instance.param_text(gain::MODE, 1.0).as_deref(),
        Some("Invert")
    );
    assert_eq!(instance.param_text(999, 1.0), None);

    let value = instance.param_from_text(gain::GAIN, "-6 dB").unwrap();
    assert!((value - 0.501_187).abs() < 1.0e-5, "{value}");
    assert_eq!(instance.param_from_text(gain::MODE, "Invert"), Some(1.0));
    assert_eq!(instance.param_from_text(gain::MODE, "Sideways"), None);
    assert_eq!(instance.param_from_text(gain::GAIN, "no\0good"), None);
}

#[test]
fn a_parameter_takes_the_shape_of_the_apps_own_descriptors() {
    let (_module, mut instance) = create(GAIN);
    let mode = instance.param_info(gain::MODE).unwrap();
    assert_eq!(mode.id, "9");
    assert_eq!(mode.kind, ParamKind::Choice);
    let labels: Vec<&str> = mode
        .choices
        .iter()
        .map(|choice| choice.label.as_str())
        .collect();
    assert_eq!(labels, ["Normal", "Invert"]);

    let level = instance.param_info(gain::GAIN).unwrap();
    assert_eq!(level.kind, ParamKind::Float);
    assert_eq!((level.min, level.max, level.default), (0.0, 2.0, 1.0));
    assert!(level.choices.is_empty());
}

#[test]
fn a_parameter_set_while_inactive_reaches_the_plugin_at_once() {
    let (_module, mut instance) = create(GAIN);
    assert_eq!(instance.param_value(gain::GAIN), Some(1.0));
    assert!(instance.set_param(gain::GAIN, 0.75));
    assert_eq!(instance.param_value(gain::GAIN), Some(0.75));

    // Out of range values are forced in, stepped ones onto a step.
    assert!(instance.set_param(gain::GAIN, 50.0));
    assert_eq!(instance.param_value(gain::GAIN), Some(2.0));
    assert!(instance.set_param(gain::MODE, 0.7));
    assert_eq!(instance.param_value(gain::MODE), Some(1.0));
    assert!(instance.set_param(gain::GAIN, f64::NAN));
    assert_eq!(instance.param_value(gain::GAIN), Some(1.0));

    assert!(!instance.set_param(gain::TIMER_TICKS, 3.0), "a reading");
    assert!(!instance.set_param(999, 1.0), "no such parameter");
}

#[test]
fn a_parameter_set_while_active_arrives_with_the_next_block() {
    let (_module, mut instance) = create(GAIN);
    let mut processor = instance.activate(RATE, 128).unwrap();
    let mut left = vec![1.0_f32; 128];
    let mut right = vec![1.0_f32; 128];

    assert!(instance.set_param(gain::GAIN, 0.25));
    assert_eq!(
        instance.param_value(gain::GAIN),
        Some(1.0),
        "nothing changes until the audio thread hands the event over"
    );
    processor.process(&mut left, &mut right);
    assert!(left.iter().all(|&sample| sample == 0.25));
    assert_eq!(instance.param_value(gain::GAIN), Some(0.25));
    instance.deactivate(processor);
}

#[test]
fn what_the_user_does_in_the_plugin_comes_back_as_a_gesture() {
    let (_module, mut instance) = create(GAIN);
    let mut processor = instance.activate(RATE, 128).unwrap();
    let mut left = vec![1.0_f32; 128];
    let mut right = vec![1.0_f32; 128];

    // Key 127 makes the test plugin act as if its own knob was turned to
    // twice the velocity.
    processor.note_on(40, 127, 0.3);
    processor.process(&mut left, &mut right);
    assert!(left[..40].iter().all(|&sample| sample == 1.0));
    assert!(
        left[40..]
            .iter()
            .all(|&sample| (sample - 0.6).abs() < 1.0e-6)
    );

    let notifications = idle(&mut instance);
    assert_eq!(notifications.len(), 3);
    assert_eq!(
        notifications[0],
        PluginNotification::ParamGestureBegin { id: gain::GAIN }
    );
    let PluginNotification::ParamChanged { id, value } = notifications[1] else {
        panic!("{:?} is not a parameter change", notifications[1]);
    };
    assert_eq!(id, gain::GAIN);
    assert!((value - 0.6).abs() < 1.0e-6);
    assert_eq!(
        notifications[2],
        PluginNotification::ParamGestureEnd { id: gain::GAIN }
    );
    assert!(
        idle(&mut instance).is_empty(),
        "each change is reported once"
    );

    // A gesture made in the last block before deactivation is not lost.
    processor.note_on(0, 127, 0.5);
    processor.process(&mut left, &mut right);
    instance.deactivate(processor);
    assert_eq!(idle(&mut instance).len(), 3);
}

#[test]
fn state_moves_from_one_instance_to_another() {
    let (_module, mut first) = create(GAIN);
    first.set_param(gain::GAIN, 0.3);
    first.set_param(gain::MODE, 1.0);
    let saved = first.save_state().unwrap();

    let (_other_module, mut second) = create(GAIN);
    assert_eq!(second.param_value(gain::GAIN), Some(1.0));
    second.load_state(&saved).unwrap();
    assert_eq!(second.param_value(gain::GAIN), Some(0.3));
    assert_eq!(second.param_value(gain::MODE), Some(1.0));
    assert_eq!(
        second.save_state().unwrap(),
        saved,
        "saving again gives the same bytes"
    );

    // The restored settings are the ones the audio uses.
    let mut processor = second.activate(RATE, 64).unwrap();
    let mut left = vec![1.0_f32; 64];
    let mut right = vec![1.0_f32; 64];
    processor.process(&mut left, &mut right);
    assert!(left.iter().all(|&sample| (sample + 0.3).abs() < 1.0e-6));

    // State can be saved and loaded while the plugin runs.
    let while_active = second.save_state().unwrap();
    assert_eq!(while_active, saved);
    second.deactivate(processor);
}

#[test]
fn state_survives_a_project_file() {
    let (_module, mut instance) = create(SINE);
    instance.set_param(SINE_LEVEL, 0.125);
    let saved = instance.save_state().unwrap();

    let json = serde_json::to_string(&saved).unwrap();
    assert!(json.starts_with('"') && json.ends_with('"'), "{json}");
    let loaded: PluginState = serde_json::from_str(&json).unwrap();
    let copy = PluginState::from_bytes(saved.as_bytes().to_vec());
    assert_eq!(loaded, copy);

    let (_other_module, mut fresh) = create(SINE);
    fresh.load_state(&loaded).unwrap();
    assert_eq!(fresh.param_value(SINE_LEVEL), Some(0.125));
}

#[test]
fn a_plugin_without_a_state_extension_is_saved_as_its_parameters() {
    let (_module, mut first) = create(MIDI_SINE);
    assert!(!first.layout().has_state);
    first.set_param(SINE_LEVEL, 0.9);
    let saved = first.save_state().unwrap();

    let (_other_module, mut second) = create(MIDI_SINE);
    second.load_state(&saved).unwrap();
    assert_eq!(second.param_value(SINE_LEVEL), Some(0.9));
    assert_eq!(second.save_state().unwrap(), saved);
}

#[test]
fn a_state_the_plugin_or_the_host_cannot_read_is_an_error() {
    let (_module, mut gain_plugin) = create(GAIN);
    let (_sine_module, mut sine_plugin) = create(SINE);
    let of_the_sine = sine_plugin.save_state().unwrap();
    assert_eq!(
        gain_plugin.load_state(&of_the_sine),
        Err(PluginError::State("loaded")),
        "the plugin refuses another plugin's state"
    );
    assert_eq!(gain_plugin.param_value(gain::GAIN), Some(1.0));

    let junk = PluginState::from_bytes(b"not a state at all".to_vec());
    assert!(matches!(
        gain_plugin.load_state(&junk),
        Err(PluginError::InvalidState(_))
    ));
}
