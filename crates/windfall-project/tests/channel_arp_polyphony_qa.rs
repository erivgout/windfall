//! Significant channel arp/polyphony controls retain exact state through history and files.
use windfall_project::*;

#[test]
fn qa_arp_modes_gate_range_rate_and_polyphony_glide_round_trip_with_history() {
    let mut doc = Document::new(Project::new("Voice acceptance"));
    let id = ChannelId(
        doc.dispatch(
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
    let pattern = doc.project().patterns[0].id;
    doc.dispatch(
        Command::AddNotes {
            pattern,
            channel: id,
            notes: vec![NoteInit {
                start: 0,
                length: 1920,
                key: 60,
                velocity: Some(0.75),
                pan: None,
                expression: None,
            }],
        },
        None,
    )
    .unwrap();
    let original_pattern = doc.project().patterns[0].clone();
    for (mode, rate, gate, octaves, cap, mono, glide) in [
        (
            ArpeggiatorMode::Up,
            NoteDivision::Quarter,
            0.0,
            1,
            1,
            false,
            0.0,
        ),
        (
            ArpeggiatorMode::Down,
            NoteDivision::Eighth,
            0.25,
            2,
            2,
            true,
            10.0,
        ),
        (
            ArpeggiatorMode::UpDown,
            NoteDivision::Sixteenth,
            0.75,
            3,
            16,
            false,
            1000.0,
        ),
        (
            ArpeggiatorMode::AsPlayed,
            NoteDivision::ThirtySecond,
            1.0,
            4,
            32,
            true,
            60000.0,
        ),
        (
            ArpeggiatorMode::Off,
            NoteDivision::Sixteenth,
            0.5,
            1,
            32,
            false,
            0.0,
        ),
    ] {
        let before = doc.project().channel(id).unwrap().voice;
        let mut settings = before;
        settings.arpeggiator = ArpeggiatorSettings {
            mode,
            rate,
            gate,
            range_octaves: octaves,
        };
        settings.polyphony = PolyphonySettings {
            max_voices: cap,
            mono_legato: mono,
            portamento_ms: glide,
        };
        let applied = doc
            .dispatch(Command::SetChannelVoiceSettings { id, settings }, None)
            .unwrap();
        assert!(applied.touched.channels);
        assert_eq!(doc.project().channel(id).unwrap().voice, settings);
        assert_eq!(doc.project().patterns[0], original_pattern);
        let loaded = file::from_json(&file::to_json(doc.project()).unwrap()).unwrap();
        assert_eq!(loaded.channel(id).unwrap().voice, settings);
        assert_eq!(loaded.patterns[0], original_pattern);
        doc.undo().unwrap();
        assert_eq!(doc.project().channel(id).unwrap().voice, before);
        doc.redo().unwrap();
        assert_eq!(doc.project().channel(id).unwrap().voice, settings);
        assert_eq!(doc.project().patterns[0], original_pattern);
    }
}
