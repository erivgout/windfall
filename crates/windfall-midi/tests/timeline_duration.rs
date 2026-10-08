//! Canonical SMF meter metadata and duration through checked Document imports.
use windfall_midi::{
    Adjustment, ImportPlan, MidiNote, MidiSong, MidiTrack, TempoChange, TimeSignatureChange,
    export_song, import, read, write,
};
use windfall_project::{Document, Project};

fn trip(bytes: &[u8]) -> (Document, ImportPlan, MidiSong, Vec<u8>) {
    let song = read(bytes).unwrap();
    let plan = import(&song, &Default::default());
    let mut document = Document::new(Project::new("Test"));
    let before = document.project().clone();
    document
        .dispatch(plan.command(document.project()), None)
        .unwrap();
    assert_eq!(document.project().check(), Ok(()));
    assert_eq!(document.history().entries.len(), 1);
    let after = document.project().clone();
    document.undo().unwrap();
    let mut restored = document.project().clone();
    restored.next_id = before.next_id;
    assert_eq!(restored, before);
    document.redo().unwrap();
    assert_eq!(document.project(), &after);
    let exported = export_song(document.project(), &Default::default()).unwrap();
    let bytes = write(&exported, &Default::default());
    (document, plan, exported, bytes)
}

#[test]
fn late_meter_default_insertion_does_not_change_the_canonical_end_on_reimport() {
    // The root seed bed5b8c5... has tempo 405860, which the existing byte
    // property maps to its exact metronome value 480000 before the first write.
    let song = MidiSong {
        tempos: vec![TempoChange {
            tick: 1,
            micros_per_quarter: 480_000,
        }],
        time_signatures: vec![TimeSignatureChange {
            tick: 1,
            numerator: 1,
            denominator: 1,
        }],
        length: 26_881,
        ..Default::default()
    };
    let (first, plan, exported, bytes) = trip(&write(&song, &Default::default()));
    let (second, next_plan, next_exported, again) = trip(&bytes);
    assert_eq!(again, bytes);
    // Canonical sizing uses 4/4 at tick zero, then retains the late 1/2 map.
    assert_eq!((plan.length, next_plan.length), (30_720, 30_720));
    assert_eq!((exported.length, next_exported.length), (30_720, 30_720));
    assert_eq!(read(&bytes).unwrap().length, 30_720);
    assert!(bytes.ends_with(&[0x81, 0xef, 0x7f, 0xff, 0x2f, 0]));
    let expected = [
        TimeSignatureChange {
            tick: 0,
            numerator: 4,
            denominator: 4,
        },
        TimeSignatureChange {
            tick: 1,
            numerator: 1,
            denominator: 2,
        },
    ];
    assert_eq!(exported.time_signatures, expected);
    assert_eq!(next_exported.time_signatures, expected);
    assert_eq!(exported.tempos, next_exported.tempos);
    assert_eq!(exported.tempos[0], TempoChange::from_bpm(0, 120.0));
    assert_eq!(exported.tempos[1], TempoChange::from_bpm(1, 125.0));
    assert!(exported.tracks.is_empty() && next_exported.tracks.is_empty());
    for document in [first, second] {
        assert_eq!(document.project().settings.time_signature.numerator, 4);
        assert_eq!(document.project().settings.time_signature.denominator, 4);
        assert_eq!(document.project().playlist.clips[0].length, 30_720);
    }
}

#[test]
fn last_tick_zero_meter_controls_sizing_and_note_clip_duration_without_tempo_automation() {
    let notes = vec![
        MidiNote {
            start: 959,
            length: 344,
            key: 60,
            velocity: 100,
            release: 64,
            channel: 0,
        },
        MidiNote {
            start: 5000,
            length: 77,
            key: 64,
            velocity: 90,
            release: 64,
            channel: 0,
        },
    ];
    let changes = vec![
        TimeSignatureChange {
            tick: 0,
            numerator: 3,
            denominator: 4,
        },
        TimeSignatureChange {
            tick: 0,
            numerator: 7,
            denominator: 8,
        },
        TimeSignatureChange {
            tick: 4001,
            numerator: 3,
            denominator: 4,
        },
    ];
    let song = MidiSong {
        tracks: vec![MidiTrack {
            name: Some("Lead".into()),
            notes: notes.clone(),
            ..Default::default()
        }],
        time_signatures: changes.clone(),
        length: 26_881,
        ..Default::default()
    };
    let (first, plan, exported, bytes) = trip(&write(&song, &Default::default()));
    let (second, next_plan, next_exported, again) = trip(&bytes);
    assert_eq!(again, bytes);
    assert_eq!((plan.length, next_plan.length), (30_240, 30_240));
    assert_eq!((exported.length, next_exported.length), (30_240, 30_240));
    assert!(plan.tempo_points.is_empty() && next_plan.tempo_points.is_empty());
    assert!(
        plan.adjustments
            .contains(&Adjustment::TimeSignatureChanges { count: 1 })
    );
    assert_eq!(exported.time_signatures, changes[1..]);
    assert_eq!(next_exported.time_signatures, changes[1..]);
    assert_eq!(exported.tracks[0].notes, notes);
    assert_eq!(next_exported.tracks[0].notes, notes);
    assert_eq!(exported.tempos, next_exported.tempos);
    for document in [first, second] {
        assert_eq!(document.project().settings.time_signature.numerator, 7);
        assert_eq!(document.project().settings.time_signature.denominator, 8);
        assert_eq!(document.project().playlist.clips.len(), 1);
        assert_eq!(document.project().playlist.clips[0].length, 30_240);
    }
}
