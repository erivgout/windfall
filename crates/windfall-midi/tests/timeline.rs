//! Meter events survive an actual SMF write/read and one checked import.
use windfall_midi::{
    ImportOptions, MidiSong, TimeSignatureChange, export_song, import, read, write,
};
use windfall_project::{Document, Project};

#[test]
fn timeline_smf_nonzero_unaligned_map_survives_bytes_import_export_undo_and_save() {
    let changes = vec![
        TimeSignatureChange {
            tick: 0,
            numerator: 4,
            denominator: 4,
        },
        TimeSignatureChange {
            tick: 4001,
            numerator: 7,
            denominator: 8,
        },
        TimeSignatureChange {
            tick: 7400,
            numerator: 3,
            denominator: 4,
        },
    ];
    let song = MidiSong {
        time_signatures: changes.clone(),
        ..Default::default()
    };
    let bytes = write(&song, &Default::default());
    let parsed = read(&bytes).unwrap();
    let plan = import(&parsed, &ImportOptions::default());
    assert!(!plan.is_empty());
    assert!(plan.adjustments.is_empty());
    let mut doc = Document::new(Project::new("Meter"));
    doc.dispatch(plan.command(doc.project()), None).unwrap();
    assert_eq!(doc.history().entries.len(), 1);
    let exported = export_song(doc.project(), &Default::default()).unwrap();
    assert_eq!(
        read(&write(&exported, &Default::default()))
            .unwrap()
            .time_signatures,
        changes
    );
    doc.undo().unwrap();
    assert!(doc.project().playlist.timeline.is_empty());
    doc.redo().unwrap();
    assert_eq!(doc.project().playlist.timeline.meters.len(), 3);
}

#[test]
fn timeline_smf_late_first_meter_keeps_legacy_adapter_and_reports_collisions() {
    let song = MidiSong {
        time_signatures: vec![
            TimeSignatureChange {
                tick: 13,
                numerator: 7,
                denominator: 8,
            },
            TimeSignatureChange {
                tick: 13,
                numerator: 3,
                denominator: 4,
            },
        ],
        ..Default::default()
    };
    let plan = import(&song, &Default::default());
    assert_eq!(plan.time_signature, None);
    assert_eq!(plan.meters.len(), 1);
    assert!(plan.adjustments.iter().any(|a| matches!(
        a,
        windfall_midi::Adjustment::TimeSignatureChanges { count: 1 }
    )));
    let mut doc = Document::new(Project::new("Late"));
    doc.dispatch(plan.command(doc.project()), None).unwrap();
    assert_eq!(doc.project().settings.time_signature.numerator, 4);
    assert_eq!(
        doc.project().playlist.timeline.meters[0]
            .signature
            .numerator,
        3
    );
}
