//! Reviewed step fills use the real document transaction and ordinary notes.
use windfall_project::*;

fn setup() -> (Document, PatternId, ChannelId) {
    let mut doc = Document::new(Project::new("Fill test"));
    let pattern = doc.project().patterns[0].id;
    let added = doc
        .dispatch(
            Command::AddChannel {
                name: Some("Drums".into()),
                sample: None,
                instrument: None,
                index: None,
                mixer_track: None,
            },
            None,
        )
        .unwrap();
    (doc, pattern, ChannelId(added.created[0]))
}

fn hit(step: u32) -> NoteInit {
    NoteInit {
        start: step * TICKS_PER_STEP,
        length: TICKS_PER_STEP,
        key: 60,
        velocity: Some(0.8),
        pan: Some(0.0),
        expression: None,
    }
}

fn lane(doc: &Document, pattern: PatternId, channel: ChannelId) -> Vec<Note> {
    doc.project()
        .patterns
        .iter()
        .find(|item| item.id == pattern)
        .unwrap()
        .lanes
        .iter()
        .find(|item| item.channel == channel)
        .map(|item| item.notes.clone())
        .unwrap_or_default()
}

fn command(
    doc: &Document,
    pattern: PatternId,
    channel: ChannelId,
    replace: bool,
    notes: Vec<NoteInit>,
) -> Command {
    Command::FillStepRange {
        pattern,
        channel,
        length_steps: 16,
        expected: lane(doc, pattern, channel),
        start_step: 2,
        end_step: 8,
        replace,
        notes,
    }
}

fn unchanged(doc: &mut Document, command: Command) {
    let before = doc.snapshot(None);
    assert!(doc.dispatch(command, None).is_err());
    assert_eq!(doc.snapshot(None), before);
}

#[test]
fn reviewed_fill_preserves_outside_notes_undo_redo_and_save() {
    let (mut doc, pattern, channel) = setup();
    let mut chord = hit(3);
    chord.key = 67;
    chord.length = 480;
    let mut between = hit(4);
    between.start += 17;
    doc.dispatch(
        Command::AddNotes {
            pattern,
            channel,
            notes: vec![hit(0), hit(3), chord, between, hit(16)],
        },
        None,
    )
    .unwrap();
    let before = lane(&doc, pattern, channel);
    let cursor = doc.history().cursor;
    let fill = command(&doc, pattern, channel, true, vec![hit(3), hit(6)]);
    let result = doc.dispatch(fill, None).unwrap();
    assert_eq!(result.label, "Advanced step fill");
    assert_eq!(result.created.len(), 1, "exact matching note keeps its id");
    assert_eq!(doc.history().cursor, cursor + 1);
    let after = lane(&doc, pattern, channel);
    assert_eq!(
        after.iter().map(|n| n.start).collect::<Vec<_>>(),
        vec![0, 720, 1440, 3840]
    );
    for old in before
        .iter()
        .filter(|n| [0, 720, 3840].contains(&n.start) && n.key == 60)
    {
        assert!(after.contains(old));
    }
    doc.project().check().unwrap();
    let saved = file::to_json(doc.project()).unwrap();
    assert_eq!(file::from_json(&saved).unwrap(), *doc.project());
    doc.undo().unwrap();
    assert_eq!(lane(&doc, pattern, channel), before);
    doc.redo().unwrap();
    assert_eq!(lane(&doc, pattern, channel), after);
}

#[test]
fn overlay_retains_chords_and_skips_only_exact_occupied_onsets() {
    let (mut doc, pattern, channel) = setup();
    let mut chord = hit(3);
    chord.key = 65;
    let mut between = hit(4);
    between.start += 17;
    doc.dispatch(
        Command::AddNotes {
            pattern,
            channel,
            notes: vec![hit(3), chord, between],
        },
        None,
    )
    .unwrap();
    let before = lane(&doc, pattern, channel);
    let fill = command(&doc, pattern, channel, false, vec![hit(3), hit(4), hit(6)]);
    let result = doc.dispatch(fill, None).unwrap();
    assert_eq!(result.created.len(), 2);
    let after = lane(&doc, pattern, channel);
    assert_eq!(after.len(), before.len() + 2);
    assert!(before.iter().all(|note| after.contains(note)));
}

#[test]
fn identical_fill_is_a_noop_preserving_redo_and_ids() {
    let (mut doc, pattern, channel) = setup();
    let fill = command(&doc, pattern, channel, true, vec![hit(2), hit(7)]);
    doc.dispatch(fill, None).unwrap();
    doc.dispatch(
        Command::UpdateSettings {
            patch: SettingsPatch {
                name: Some("Later".into()),
                ..Default::default()
            },
        },
        None,
    )
    .unwrap();
    doc.undo().unwrap();
    let before = doc.snapshot(None);
    let fill = command(&doc, pattern, channel, true, vec![hit(2), hit(7)]);
    let applied = doc.dispatch(fill, None).unwrap();
    assert!(applied.touched.is_empty());
    assert!(applied.created.is_empty());
    assert_eq!(doc.snapshot(None), before);
    assert!(doc.redo().is_some());
}

#[test]
fn a_zero_hit_replacement_clears_only_the_reviewed_range() {
    let (mut doc, pattern, channel) = setup();
    doc.dispatch(
        Command::AddNotes {
            pattern,
            channel,
            notes: vec![hit(0), hit(3), hit(16)],
        },
        None,
    )
    .unwrap();
    let before = lane(&doc, pattern, channel);
    let fill = command(&doc, pattern, channel, true, vec![]);
    doc.dispatch(fill, None).unwrap();
    assert_eq!(
        lane(&doc, pattern, channel)
            .iter()
            .map(|n| n.start)
            .collect::<Vec<_>>(),
        vec![0, 3840]
    );
    doc.undo().unwrap();
    assert_eq!(lane(&doc, pattern, channel), before);
}

#[test]
fn changed_note_added_note_and_pattern_length_refuse_without_mutation() {
    let (mut doc, pattern, channel) = setup();
    doc.dispatch(
        Command::AddNotes {
            pattern,
            channel,
            notes: vec![hit(3)],
        },
        None,
    )
    .unwrap();
    let fill = command(&doc, pattern, channel, true, vec![hit(6)]);
    let id = lane(&doc, pattern, channel)[0].id;
    doc.dispatch(
        Command::UpdateNotes {
            pattern,
            channel,
            updates: vec![NoteUpdate {
                id,
                patch: NotePatch {
                    velocity: Some(0.2),
                    ..Default::default()
                },
            }],
        },
        None,
    )
    .unwrap();
    unchanged(&mut doc, fill);
    let fill = command(&doc, pattern, channel, true, vec![hit(6)]);
    doc.dispatch(
        Command::AddNotes {
            pattern,
            channel,
            notes: vec![hit(5)],
        },
        None,
    )
    .unwrap();
    unchanged(&mut doc, fill);
    let fill = command(&doc, pattern, channel, true, vec![hit(6)]);
    doc.dispatch(
        Command::UpdatePattern {
            id: pattern,
            patch: PatternPatch {
                length_steps: Some(8),
                ..Default::default()
            },
        },
        None,
    )
    .unwrap();
    unchanged(&mut doc, fill);
}

#[test]
fn invalid_outputs_and_ranges_roll_back_notes_ids_and_history() {
    let (mut doc, pattern, channel) = setup();
    for notes in [
        vec![hit(1)],
        vec![hit(8)],
        vec![hit(3), hit(3)],
        vec![NoteInit {
            start: 721,
            ..hit(3)
        }],
        vec![NoteInit {
            length: 0,
            ..hit(3)
        }],
        vec![NoteInit {
            length: 241,
            ..hit(3)
        }],
        vec![NoteInit { key: 128, ..hit(3) }],
        vec![
            hit(3),
            NoteInit {
                velocity: Some(f32::NAN),
                ..hit(5)
            },
        ],
    ] {
        let fill = command(&doc, pattern, channel, true, notes);
        unchanged(&mut doc, fill);
    }
    for (start_step, end_step) in [(8, 8), (9, 8), (0, 17), (u32::MAX, u32::MAX)] {
        let fill = Command::FillStepRange {
            pattern,
            channel,
            length_steps: 16,
            expected: vec![],
            start_step,
            end_step,
            replace: true,
            notes: vec![],
        };
        unchanged(&mut doc, fill);
    }
}

#[test]
fn id_exhaustion_rolls_back_an_entire_fill() {
    let (doc, pattern, channel) = setup();
    let mut project = doc.project().clone();
    project.next_id = u32::MAX - 1;
    let mut doc = Document::new(project);
    let fill = command(&doc, pattern, channel, true, vec![hit(3), hit(6)]);
    unchanged(&mut doc, fill);
}
