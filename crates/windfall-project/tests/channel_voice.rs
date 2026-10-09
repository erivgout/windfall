use windfall_project::*;

#[test]
fn channel_and_synth_lfo_bindings_have_distinct_export_names() {
    use ts_rs::TS;
    let config = ts_rs::Config::default();

    assert_ne!(
        <windfall_project::channel_voice::LfoShape as TS>::name(&config),
        <windfall_dsp::LfoShape as TS>::name(&config),
        "different LFO shape enums must not overwrite the same generated file"
    );
}

#[test]
fn channel_voice_command_undo_save_reload_and_legacy_defaults() {
    let mut document = Document::new(Project::new("Voice tools"));
    let id = ChannelId(
        document
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
            .unwrap()
            .created[0],
    );
    let mut settings = ChannelVoiceSettings::default();
    settings.arpeggiator.mode = ArpeggiatorMode::Up;
    settings.echo.enabled = true;
    settings.echo.repeats = 255;
    settings.polyphony.max_voices = 0;
    settings.envelopes.pitch.enabled = true;
    settings.envelopes.pitch.depth = 1200.0;
    let changed = document
        .dispatch(Command::SetChannelVoiceSettings { id, settings }, None)
        .unwrap();
    assert!(changed.touched.channels);
    assert_eq!(
        document.project().channel(id).unwrap().voice,
        settings.sanitized()
    );
    document.undo().unwrap();
    assert_eq!(
        document.project().channel(id).unwrap().voice,
        ChannelVoiceSettings::default()
    );
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("voice.windfall");
    file::save(document.project(), &path).unwrap();
    assert_eq!(
        file::load(&path).unwrap().channel(id).unwrap().voice,
        ChannelVoiceSettings::default()
    );
    document.redo().unwrap();
    file::save(document.project(), &path).unwrap();
    assert_eq!(
        file::load(&path).unwrap().channel(id).unwrap().voice,
        settings.sanitized()
    );
    let mut legacy = serde_json::to_value(document.project()).unwrap();
    legacy["channels"][0]
        .as_object_mut()
        .unwrap()
        .remove("voice");
    assert_eq!(
        file::from_json(&legacy.to_string())
            .unwrap()
            .channel(id)
            .unwrap()
            .voice,
        ChannelVoiceSettings::default()
    );
    legacy["channels"][0]["voice"] =
        serde_json::json!({"polyphony":{"maxVoices":0}, "arpeggiator":{"gate":5.0}});
    let repaired = file::from_json(&legacy.to_string()).unwrap();
    repaired.check().unwrap();
    assert_eq!(repaired.channel(id).unwrap().voice.polyphony.max_voices, 1);
    assert_eq!(repaired.channel(id).unwrap().voice.arpeggiator.gate, 1.0);
    let history = document.history();
    assert!(
        document
            .dispatch(
                Command::SetChannelVoiceSettings {
                    id: ChannelId(u32::MAX),
                    settings
                },
                None
            )
            .is_err()
    );
    assert_eq!(document.history(), history);
}
