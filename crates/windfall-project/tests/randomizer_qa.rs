//! Public document acceptance for the seeded randomizer and chord-map generator.
use windfall_project::*;

fn fixture() -> (Document, ChannelId) {
    let mut doc = Document::new(Project::new("Randomizer QA"));
    let channel = ChannelId(
        doc.dispatch(
            Command::AddChannel {
                name: Some("Lead".into()),
                sample: None,
                instrument: Some(InstrumentKind::SubtractiveSynth),
                mixer_track: None,
                index: None,
            },
            None,
        )
        .unwrap()
        .created[0],
    );
    let expression = NoteExpression {
        release: 0.37,
        fine_pitch_cents: 42.0,
        modulation_x: 0.23,
        modulation_y: 0.81,
        articulation: NoteArticulation::Portamento,
        glide_ticks: 333,
        color_group: Some(12),
    };
    doc.dispatch(
        Command::AddNotes {
            pattern: PatternId(1),
            channel,
            notes: vec![
                NoteInit {
                    start: 120,
                    length: 601,
                    key: 60,
                    velocity: Some(0.6),
                    pan: Some(-0.4),
                    expression: Some(expression),
                },
                NoteInit {
                    start: 2000,
                    length: 240,
                    key: 72,
                    velocity: Some(0.9),
                    pan: Some(0.2),
                    expression: None,
                },
            ],
        },
        None,
    )
    .unwrap();
    (doc, channel)
}

fn notes(doc: &Document, channel: ChannelId) -> Vec<Note> {
    doc.project()
        .pattern(PatternId(1))
        .unwrap()
        .lanes
        .iter()
        .find(|lane| lane.channel == channel)
        .unwrap()
        .notes
        .clone()
}

fn apply(doc: &mut Document, channel: ChannelId, selected: Vec<Note>, transform: NoteTransform) {
    // Exercise the public JSON command boundary as used by the UI/native bridge.
    let command = Command::TransformNotes {
        pattern: PatternId(1),
        channel,
        notes: selected,
        transform,
    };
    let command = serde_json::from_str(&serde_json::to_string(&command).unwrap()).unwrap();
    doc.dispatch(command, None).unwrap();
}

fn randomize(seed: u32) -> NoteTransform {
    NoteTransform::Randomize {
        seed,
        pitch: 7,
        velocity: 0.25,
        pan: 0.5,
        timing: 100,
        length: 0.4,
    }
}

fn generate(seed: u32, density: f64) -> NoteTransform {
    NoteTransform::GenerateRandom {
        seed,
        grid: 240,
        density,
        gate: 0.5,
        root: 0,
        pitch_classes: (1 << 0) | (1 << 4) | (1 << 7),
        low: 60,
        high: 72,
        velocity_low: 0.3,
        velocity_high: 0.8,
    }
}

#[test]
fn seeded_randomization_preserves_identity_expression_unselected_notes_and_atomic_history() {
    let (mut doc, channel) = fixture();
    let before = doc.project().clone();
    let selected = notes(&doc, channel)[0];
    let unrelated = notes(&doc, channel)[1];
    let cursor = doc.history().cursor;
    apply(&mut doc, channel, vec![selected], randomize(0x1234));
    let after = doc.project().clone();
    let changed = notes(&doc, channel)
        .into_iter()
        .find(|note| note.id == selected.id)
        .unwrap();
    assert_eq!(changed.expression, selected.expression);
    assert_eq!(
        notes(&doc, channel)
            .into_iter()
            .find(|note| note.id == unrelated.id),
        Some(unrelated)
    );
    assert!(changed.key.abs_diff(selected.key) <= 7);
    assert!(changed.start.abs_diff(selected.start) <= 100);
    assert!((changed.velocity - selected.velocity).abs() <= 0.251);
    assert!((changed.pan - selected.pan).abs() <= 0.501);
    assert!((361..=841).contains(&changed.length));
    assert_eq!(after.next_id, before.next_id);
    assert_eq!(doc.history().cursor, cursor + 1);
    assert_eq!(
        doc.history().entries.last().unwrap().label,
        "Randomize notes"
    );
    let mut repeat = Document::new(before.clone());
    apply(&mut repeat, channel, vec![selected], randomize(0x1234));
    assert_eq!(repeat.project(), &after);
    let mut another = Document::new(before.clone());
    apply(&mut another, channel, vec![selected], randomize(0x5678));
    assert_ne!(notes(&another, channel), notes(&doc, channel));
    doc.undo().unwrap();
    assert_eq!(doc.project(), &before);
    doc.redo().unwrap();
    assert_eq!(doc.project(), &after);
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("randomized.windfall");
    file::save(&after, &path).unwrap();
    assert_eq!(file::load(&path).unwrap(), after);
}

#[test]
fn seeded_generator_covers_partial_cell_chord_bounds_new_ids_expression_and_exact_redo() {
    let (mut doc, channel) = fixture();
    let before = doc.project().clone();
    let source = notes(&doc, channel)[0];
    let unrelated = notes(&doc, channel)[1];
    let cursor = doc.history().cursor;
    apply(&mut doc, channel, vec![source], generate(17, 1.0));
    let after = doc.project().clone();
    let generated: Vec<_> = notes(&doc, channel)
        .into_iter()
        .filter(|note| note.id != unrelated.id)
        .collect();
    assert_eq!(
        generated
            .iter()
            .map(|note| (note.start, note.length))
            .collect::<Vec<_>>(),
        [(120, 120), (360, 120), (600, 61)]
    );
    for note in &generated {
        assert!(note.id.0 >= before.next_id);
        assert_ne!(note.id, source.id);
        assert!(matches!(note.key, 60 | 64 | 67 | 72));
        assert!((0.3..=0.8).contains(&note.velocity));
        assert_eq!(note.pan, source.pan);
        assert_eq!(note.expression, source.expression);
        assert!(note.start + note.length <= source.start + source.length);
    }
    assert_eq!(
        generated
            .iter()
            .map(|note| note.id)
            .collect::<std::collections::HashSet<_>>()
            .len(),
        3
    );
    assert_eq!(
        notes(&doc, channel)
            .into_iter()
            .find(|note| note.id == unrelated.id),
        Some(unrelated)
    );
    assert_eq!(doc.history().cursor, cursor + 1);
    let mut repeat = Document::new(before.clone());
    apply(&mut repeat, channel, vec![source], generate(17, 1.0));
    assert_eq!(repeat.project(), &after);
    doc.undo().unwrap();
    let mut restored = before.clone();
    restored.next_id = doc.project().next_id;
    assert_eq!(doc.project(), &restored);
    doc.redo().unwrap();
    assert_eq!(doc.project(), &after);
    assert_eq!(
        file::from_json(&file::to_json(&after).unwrap()).unwrap(),
        after
    );
    let mut rests = Document::new(before);
    apply(&mut rests, channel, vec![source], generate(17, 0.0));
    assert_eq!(notes(&rests, channel), [unrelated]);
}

#[test]
fn stale_invalid_and_oversized_random_requests_refuse_without_history_or_id_mutation() {
    let (mut doc, channel) = fixture();
    let source = notes(&doc, channel)[0];
    let mut stale = source;
    stale.length += 1;
    let mut too_many_cells = source;
    too_many_cells.length = windfall_project::piano_tools::MAX_TOOL_NOTES as u32 + 1;
    // Install a valid long note first so the size rejection exercises generation,
    // rather than the captured-source guard.
    doc.dispatch(
        Command::UpdateNotes {
            pattern: PatternId(1),
            channel,
            updates: vec![NoteUpdate {
                id: source.id,
                patch: NotePatch {
                    length: Some(too_many_cells.length),
                    ..Default::default()
                },
            }],
        },
        None,
    )
    .unwrap();
    let actual = notes(&doc, channel)
        .into_iter()
        .find(|note| note.id == source.id)
        .unwrap();
    let before = doc.project().clone();
    let cursor = doc.history().cursor;
    let requests = [
        (vec![stale], randomize(1)),
        (
            vec![actual],
            NoteTransform::GenerateRandom {
                seed: 1,
                grid: 1,
                density: 1.0,
                gate: 1.0,
                root: 0,
                pitch_classes: 1,
                low: 60,
                high: 60,
                velocity_low: 0.5,
                velocity_high: 0.5,
            },
        ),
        (
            vec![actual],
            NoteTransform::GenerateRandom {
                seed: 1,
                grid: 240,
                density: 1.0,
                gate: 1.0,
                root: 0,
                pitch_classes: 1,
                low: 61,
                high: 61,
                velocity_low: 0.5,
                velocity_high: 0.5,
            },
        ),
        (
            vec![actual],
            NoteTransform::Randomize {
                seed: 1,
                pitch: 0,
                velocity: f64::NAN,
                pan: 0.0,
                timing: 0,
                length: 0.0,
            },
        ),
    ];
    for (selected, transform) in requests {
        assert!(
            doc.dispatch(
                Command::TransformNotes {
                    pattern: PatternId(1),
                    channel,
                    notes: selected,
                    transform
                },
                None
            )
            .is_err()
        );
        assert_eq!(doc.project(), &before);
        assert_eq!(doc.history().cursor, cursor);
    }
}
