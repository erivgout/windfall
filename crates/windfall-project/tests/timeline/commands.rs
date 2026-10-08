use windfall_project::*;

fn meter(tick: u32, numerator: u8, denominator: u8) -> Command {
    Command::AddMeterChange {
        tick,
        signature: TimeSignature {
            numerator,
            denominator,
        },
    }
}
fn marker(tick: u32, kind: MarkerKind) -> Command {
    Command::AddTimelineMarker {
        tick,
        name: "Section".to_owned(),
        kind,
    }
}

#[test]
fn checked_batch_patch_undo_save_and_legacy_scalar_adapter() {
    let mut project = Project::new("Meter song");
    project.settings.time_signature = TimeSignature {
        numerator: 3,
        denominator: 4,
    };
    let legacy = file::to_json(&project).unwrap();
    assert!(!legacy.contains("timeline"));
    assert_eq!(file::from_json(&legacy).unwrap(), project);
    let mut doc = Document::new(project);
    let original = doc.project().clone();
    let applied = doc
        .dispatch(
            Command::Batch {
                label: Some("Timeline".into()),
                commands: vec![
                    meter(4_001, 7, 8),
                    meter(7_400, 3, 4),
                    marker(100, MarkerKind::Named),
                    marker(120, MarkerKind::Skip { end: 400 }),
                    marker(500, MarkerKind::Pause),
                    marker(600, MarkerKind::Loop { end: 1_200 }),
                ],
            },
            None,
        )
        .unwrap();
    assert_eq!(doc.history().cursor, 1);
    let patch = doc.patch(&applied.touched);
    assert_eq!(
        patch.playlist.unwrap().timeline,
        doc.project().playlist.timeline
    );
    let changed = doc.project().clone();
    assert_eq!(
        file::from_json(&file::to_json(&changed).unwrap()).unwrap(),
        changed
    );
    doc.undo().unwrap();
    assert_eq!(doc.project().playlist, original.playlist);
    assert!(doc.project().next_id > original.next_id);
    doc.redo().unwrap();
    assert_eq!(*doc.project(), changed);
    let id = changed.playlist.timeline.meters[0].id;
    doc.dispatch(
        Command::UpdateMeterChange {
            id,
            tick: 4_500,
            signature: TimeSignature {
                numerator: 5,
                denominator: 8,
            },
        },
        Some(98),
    )
    .unwrap();
    doc.dispatch(
        Command::UpdateMeterChange {
            id,
            tick: 4_800,
            signature: TimeSignature {
                numerator: 6,
                denominator: 8,
            },
        },
        Some(98),
    )
    .unwrap();
    assert_eq!(doc.history().cursor, 2);
    doc.undo().unwrap();
    assert_eq!(doc.project().playlist.timeline, changed.playlist.timeline);
    doc.dispatch(
        Command::RemoveTimelineMarker {
            id: TimelineMarkerId(applied.created[2]),
        },
        None,
    )
    .unwrap();
    assert!(
        !doc.project()
            .playlist
            .timeline
            .markers
            .iter()
            .any(|m| m.id.0 == applied.created[2])
    );
    doc.undo().unwrap();
    assert_eq!(doc.project().playlist.timeline, changed.playlist.timeline);
}

#[test]
fn rejected_commands_restore_ids_project_history_and_dirty_state() {
    let mut doc = Document::new(Project::new("Atomic"));
    doc.dispatch(meter(960, 7, 8), None).unwrap();
    doc.dispatch(marker(100, MarkerKind::Skip { end: 300 }), None)
        .unwrap();
    for command in [
        meter(960, 3, 4),
        meter(MAX_SONG_TICKS, 4, 4),
        meter(10, 4, 0),
        marker(200, MarkerKind::Pause),
        marker(300, MarkerKind::Pause),
        marker(400, MarkerKind::Loop { end: u32::MAX }),
        Command::RemoveTimelineMarker {
            id: TimelineMarkerId(999),
        },
        Command::Batch {
            label: Some("Invalid".into()),
            commands: vec![meter(1_920, 3, 4), meter(960, 5, 4)],
        },
    ] {
        let before = doc.snapshot(None);
        assert!(doc.dispatch(command, None).is_err());
        assert_eq!(doc.snapshot(None), before);
    }
    for mutation in 0..3 {
        let mut invalid = doc.project().clone();
        match mutation {
            0 => invalid.playlist.timeline.meters[0].id = MeterChangeId(invalid.patterns[0].id.0),
            1 => {
                invalid.playlist.timeline.markers[0].id =
                    TimelineMarkerId(invalid.playlist.timeline.meters[0].id.0)
            }
            _ => invalid.playlist.timeline.meters[0].id = MeterChangeId(invalid.next_id),
        }
        assert!(invalid.check().is_err());
        assert!(file::from_json(&serde_json::to_string(&invalid).unwrap()).is_err());
    }
    assert!(TickRange { start: 1, end: 1 }.check().is_err());
    assert!(
        TickRange {
            start: 1,
            end: MAX_SONG_TICKS + 1
        }
        .check()
        .is_err()
    );
}
