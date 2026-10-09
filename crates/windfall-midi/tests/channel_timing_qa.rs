//! MIDI must encode played timing while preserving the stored note grid.
use windfall_midi::{ExportOptions, WriteOptions, export_pattern, read, write};
use windfall_project::*;

#[test]
fn midi_export_preserves_per_channel_swing_gate_and_signed_shift() {
    let mut doc = Document::new(Project::new("Timing MIDI QA"));
    let channel = ChannelId(
        doc.dispatch(
            Command::AddChannel {
                name: None,
                sample: None,
                instrument: Some(InstrumentKind::SubtractiveSynth),
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
        Command::UpdateSettings {
            patch: SettingsPatch {
                swing: Some(1.0),
                ..Default::default()
            },
        },
        None,
    )
    .unwrap();
    doc.dispatch(
        Command::AddNotes {
            pattern,
            channel,
            notes: vec![NoteInit {
                start: 240,
                length: 240,
                key: 60,
                velocity: Some(1.0),
                pan: None,
                expression: None,
            }],
        },
        None,
    )
    .unwrap();
    for (shift_ticks, expected) in [(-960, 0), (-19, 261), (19, 299)] {
        doc.dispatch(
            Command::UpdateChannel {
                id: channel,
                patch: ChannelPatch {
                    timing: Some(ChannelTiming {
                        swing_mix: 0.5,
                        gate_ticks: 75,
                        shift_ticks,
                    }),
                    ..Default::default()
                },
            },
            None,
        )
        .unwrap();
        let before = doc.project().clone();
        let song = export_pattern(doc.project(), pattern, &ExportOptions::default()).unwrap();
        let notes: Vec<_> = song.tracks.iter().flat_map(|track| &track.notes).collect();
        assert_eq!(notes.len(), 1);
        assert_eq!((notes[0].start, notes[0].length), (expected, 75));
        assert_eq!(doc.project(), &before);
        let bytes = write(&song, &WriteOptions::default());
        assert_eq!(read(&bytes).unwrap(), song.normalized());
    }
    // Turning off export swing removes the global swing warp; explicit
    // channel gate and shift are still playback settings.
    let song = export_pattern(
        doc.project(),
        pattern,
        &ExportOptions {
            swing: false,
            ..Default::default()
        },
    )
    .unwrap();
    let note = &song
        .tracks
        .iter()
        .find(|track| !track.notes.is_empty())
        .unwrap()
        .notes[0];
    assert_eq!((note.start, note.length), (259, 75));
}
