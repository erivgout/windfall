use super::*;
use windfall_project::piano_tools::{ArpDirection, ChopStep, FlamPosition, RhythmMode};

fn pattern() -> NoteTransform {
    NoteTransform::ChopPattern {
        origin: 100,
        period: 480,
        steps: vec![
            ChopStep {
                tick: 0,
                gate: 1.0,
                velocity: 1.0,
            },
            ChopStep {
                tick: 120,
                gate: 0.5,
                velocity: 1.5,
            },
            ChopStep {
                tick: 360,
                gate: 1.0,
                velocity: 0.5,
            },
        ],
    }
}

fn arp(direction: ArpDirection, repetitions: u32) -> NoteTransform {
    NoteTransform::Arpeggiate {
        rate: 120,
        gate: 0.5,
        octaves: 1,
        repetitions,
        direction,
    }
}

fn reshape(mode: RhythmMode, offset: i32) -> NoteTransform {
    NoteTransform::RhythmReshape {
        origin: 0,
        step: 240,
        period: 2,
        phase: 0,
        offset,
        mode,
    }
}

#[test]
fn custom_chop_repeats_before_origin_and_keeps_accented_partial_edges() {
    let source = note(9, 50, 600, 60);
    let result = transform_selected_notes(&[source], pattern()).unwrap();
    assert_eq!(
        result
            .iter()
            .map(|n| (n.id.0, n.start, n.length, n.velocity))
            .collect::<Vec<_>>(),
        vec![
            (9, 50, 50, 0.25),
            (0, 100, 120, 0.5),
            (0, 220, 120, 0.75),
            (0, 460, 120, 0.25),
            (0, 580, 70, 0.5)
        ]
    );
    assert!(
        result
            .iter()
            .all(|n| n.key == source.key && n.pan == source.pan)
    );
    // Old uniform Chop and equivalent custom boundaries agree exactly.
    assert_eq!(
        transform_selected_notes(&[source], NoteTransform::Chop { grid: 120 }).unwrap(),
        transform_selected_notes(
            &[source],
            NoteTransform::ChopPattern {
                origin: 0,
                period: 240,
                steps: vec![
                    ChopStep {
                        tick: 0,
                        gate: 1.0,
                        velocity: 1.0
                    },
                    ChopStep {
                        tick: 120,
                        gate: 1.0,
                        velocity: 1.0
                    },
                ],
            }
        )
        .unwrap()
    );
}

#[test]
fn arpeggio_repeats_real_voices_order_dynamics_and_complete_slots() {
    let mut high = note(2, 0, 720, 64);
    high.velocity = 0.8;
    high.pan = 0.3;
    let low = note(3, 0, 360, 60);
    let tie = note(1, 0, 480, 64);
    let input = [high, low, tie];
    for (direction, keys, ids) in [
        (
            ArpDirection::Ascending,
            vec![60, 64, 64, 60, 64, 64],
            vec![3, 1, 2, 0, 0, 0],
        ),
        (
            ArpDirection::Descending,
            vec![64, 64, 60, 64, 64, 60],
            vec![1, 2, 3, 0, 0, 0],
        ),
        (
            ArpDirection::Alternating,
            vec![60, 64, 64, 64, 60, 64],
            vec![3, 1, 2, 0, 0, 0],
        ),
    ] {
        let output = transform_selected_notes(&input, arp(direction, 0)).unwrap();
        assert_eq!(output.iter().map(|n| n.key).collect::<Vec<_>>(), keys);
        assert_eq!(output.iter().map(|n| n.id.0).collect::<Vec<_>>(), ids);
        assert_eq!(
            output.iter().map(|n| n.start).collect::<Vec<_>>(),
            vec![0, 120, 240, 360, 480, 600]
        );
        assert!(output.iter().all(|n| n.length == 60));
        for n in output.iter().filter(|n| n.velocity == high.velocity) {
            assert_eq!(n.pan, high.pan);
        }
    }
    let output = transform_selected_notes(
        &[low],
        NoteTransform::Arpeggiate {
            rate: 120,
            gate: 1.0,
            octaves: 2,
            repetitions: 2,
            direction: ArpDirection::Ascending,
        },
    )
    .unwrap();
    assert_eq!(
        output
            .iter()
            .map(|n| (n.start, n.key, n.id.0))
            .collect::<Vec<_>>(),
        vec![(0, 60, 3), (120, 72, 0), (240, 60, 0), (360, 72, 0)]
    );
    assert!(
        transform_selected_notes(&[note(1, 0, 121, 60)], arp(ArpDirection::Ascending, 0)).is_err()
    );
}

#[test]
fn flam_preserves_original_and_positions_short_grace_hit_explicitly() {
    let source = note(9, 100, 400, 60);
    for (position, start) in [(FlamPosition::Before, 70), (FlamPosition::After, 130)] {
        let output = transform_selected_notes(
            &[source],
            NoteTransform::Flam {
                interval: 30,
                velocity: 0.4,
                position,
            },
        )
        .unwrap();
        assert!(output.contains(&source));
        assert_eq!(
            *output.iter().find(|n| n.id.0 == 0).unwrap(),
            Note {
                id: NoteId(0),
                start,
                length: 30,
                velocity: 0.2,
                ..source
            }
        );
    }
}

#[test]
fn reshaper_matches_cells_with_euclidean_phase_and_handles_empty_or_duplicate_output() {
    let notes = [
        note(1, 0, 120, 60),
        note(2, 250, 120, 64),
        note(3, 490, 120, 67),
    ];
    assert_eq!(
        transform_selected_notes(&notes, reshape(RhythmMode::Remove, 0)).unwrap(),
        vec![notes[1]]
    );
    assert_eq!(
        transform_selected_notes(&notes, reshape(RhythmMode::Shift, 30))
            .unwrap()
            .iter()
            .map(|n| n.start)
            .collect::<Vec<_>>(),
        vec![30, 250, 520]
    );
    let added = transform_selected_notes(&notes, reshape(RhythmMode::Add, 240)).unwrap();
    assert_eq!(
        added.iter().map(|n| (n.id.0, n.start)).collect::<Vec<_>>(),
        vec![(1, 0), (0, 240), (2, 250), (3, 490), (0, 730)]
    );
    assert!(
        transform_selected_notes(
            &notes,
            NoteTransform::RhythmReshape {
                origin: 0,
                step: 1,
                period: 1,
                phase: 0,
                offset: 0,
                mode: RhythmMode::Remove,
            }
        )
        .unwrap()
        .is_empty()
    );
    let pair = [note(1, 0, 120, 60), note(2, 240, 120, 60)];
    assert_eq!(
        transform_selected_notes(&pair, reshape(RhythmMode::Add, 240)).unwrap(),
        pair
    );
    let before_origin = NoteTransform::RhythmReshape {
        origin: 480,
        step: 240,
        period: 2,
        phase: 1,
        offset: 1,
        mode: RhythmMode::Shift,
    };
    assert_eq!(
        transform_selected_notes(&pair, before_origin).unwrap()[1].start,
        241
    );
}

fn tools() -> Vec<NoteTransform> {
    vec![
        pattern(),
        arp(ArpDirection::Alternating, 6),
        NoteTransform::Flam {
            interval: 960,
            velocity: 0.5,
            position: FlamPosition::After,
        },
        reshape(RhythmMode::Shift, 480),
        reshape(RhythmMode::Add, 480),
        reshape(RhythmMode::Remove, 0),
    ]
}

#[test]
fn rhythm_commands_are_deterministic_atomic_undoable_and_persistable() {
    for transform in tools() {
        let (mut doc, channel, other) = fixture();
        let input = vec![selected(&doc, channel)[1]]; // Preserve an unselected note in the same lane.
        let pure = transform_selected_notes(&input, transform.clone()).unwrap();
        assert_eq!(
            pure,
            transform_selected_notes(&[input[0], input[0]], transform.clone()).unwrap()
        );
        let before = doc.project().clone();
        let history = doc.history();
        let next = before.next_id;
        let command = Command::TransformNotes {
            pattern: PatternId(1),
            channel,
            notes: input.clone(),
            transform: transform.clone(),
        };
        let applied = doc
            .dispatch(
                serde_json::from_str(&serde_json::to_string(&command).unwrap()).unwrap(),
                None,
            )
            .unwrap();
        let after = doc.project().clone();
        assert_eq!(doc.history().cursor, history.cursor + 1);
        if matches!(
            transform,
            NoteTransform::Arpeggiate { .. }
                | NoteTransform::Flam { .. }
                | NoteTransform::RhythmReshape {
                    mode: RhythmMode::Add | RhythmMode::Shift,
                    ..
                }
        ) {
            assert!(after.patterns[0].length_steps > before.patterns[0].length_steps);
        }
        let unrelated = selected(&Document::new(before.clone()), channel)[0];
        assert!(selected(&doc, channel).contains(&unrelated));
        assert_eq!(
            selected(&doc, other),
            selected(&Document::new(before.clone()), other)
        );
        assert_eq!(
            applied.created,
            (next..next + pure.iter().filter(|n| n.id.0 == 0).count() as u32).collect::<Vec<_>>()
        );
        let mut expected = pure;
        let mut allocated = applied.created.iter();
        for n in &mut expected {
            if n.id.0 == 0 {
                n.id = NoteId(*allocated.next().unwrap());
            }
        }
        let actual = selected(&doc, channel)
            .into_iter()
            .filter(|n| n.id != unrelated.id)
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
        after.check().unwrap();
        doc.undo().unwrap();
        let mut restored = before;
        restored.next_id = doc.project().next_id;
        assert_eq!(doc.project(), &restored);
        doc.redo().unwrap();
        assert_eq!(doc.project(), &after);
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("rhythm.windfall");
        file::save(&after, &path).unwrap();
        assert_eq!(file::load(&path).unwrap(), after);
    }
    // Input order and identical duplicate snapshots do not affect any tool.
    let input = [note(1, 480, 480, 60), note(2, 480, 480, 64)];
    for tool in tools() {
        assert_eq!(
            transform_selected_notes(&input, tool.clone()),
            transform_selected_notes(&[input[1], input[0], input[1]], tool)
        );
    }
}

#[test]
fn rhythm_noops_preserve_dirty_redo_and_ids() {
    for tool in [
        NoteTransform::ChopPattern {
            origin: 0,
            period: MAX_PATTERN_TICKS,
            steps: vec![ChopStep {
                tick: 0,
                gate: 1.0,
                velocity: 1.0,
            }],
        },
        reshape(RhythmMode::Shift, 0),
        NoteTransform::RhythmReshape {
            origin: 0,
            step: MAX_PATTERN_TICKS,
            period: 2,
            phase: 1,
            offset: 10,
            mode: RhythmMode::Remove,
        },
    ] {
        let (mut doc, channel, _) = fixture();
        doc.dispatch(
            Command::TransformNotes {
                pattern: PatternId(1),
                channel,
                notes: selected(&doc, channel),
                transform: NoteTransform::FlipPitch,
            },
            None,
        )
        .unwrap();
        doc.undo().unwrap();
        doc.mark_saved();
        let project = doc.project().clone();
        let history = doc.history();
        let result = doc
            .dispatch(
                Command::TransformNotes {
                    pattern: PatternId(1),
                    channel,
                    notes: selected(&doc, channel),
                    transform: tool,
                },
                None,
            )
            .unwrap();
        assert!(result.touched.is_empty());
        assert!(result.created.is_empty());
        assert_eq!(doc.project(), &project);
        assert_eq!(doc.history(), history);
        assert!(!doc.is_dirty());
    }
}

#[test]
fn invalid_rhythm_parameters_and_whole_output_fail_atomically() {
    let (mut doc, channel, _) = fixture();
    let step = ChopStep {
        tick: 0,
        gate: 1.0,
        velocity: 1.0,
    };
    let mut invalid = vec![
        NoteTransform::ChopPattern {
            origin: 0,
            period: 0,
            steps: vec![step],
        },
        NoteTransform::ChopPattern {
            origin: 0,
            period: 480,
            steps: vec![],
        },
        NoteTransform::ChopPattern {
            origin: 0,
            period: 480,
            steps: vec![step; 65],
        },
        NoteTransform::ChopPattern {
            origin: 0,
            period: 480,
            steps: vec![step, step],
        },
        NoteTransform::ChopPattern {
            origin: 0,
            period: 480,
            steps: vec![ChopStep { tick: 480, ..step }],
        },
        NoteTransform::ChopPattern {
            origin: u32::MAX,
            period: 480,
            steps: vec![step],
        },
        NoteTransform::Arpeggiate {
            rate: 1,
            gate: 1.0,
            octaves: 1,
            repetitions: 65,
            direction: ArpDirection::Ascending,
        },
        NoteTransform::Arpeggiate {
            rate: 120,
            gate: 1.0,
            octaves: 0,
            repetitions: 1,
            direction: ArpDirection::Ascending,
        },
        NoteTransform::Arpeggiate {
            rate: 245760,
            gate: 1.0,
            octaves: 1,
            repetitions: 1,
            direction: ArpDirection::Ascending,
        },
        NoteTransform::Flam {
            interval: 0,
            velocity: 0.5,
            position: FlamPosition::Before,
        },
        NoteTransform::Flam {
            interval: 961,
            velocity: 0.5,
            position: FlamPosition::After,
        },
        reshape(RhythmMode::Shift, -5000),
        reshape(RhythmMode::Add, 245760),
        reshape(RhythmMode::Add, 0),
        NoteTransform::RhythmReshape {
            origin: 0,
            step: 0,
            period: 2,
            phase: 0,
            offset: 1,
            mode: RhythmMode::Shift,
        },
        NoteTransform::RhythmReshape {
            origin: 0,
            step: 240,
            period: 2,
            phase: 2,
            offset: 1,
            mode: RhythmMode::Shift,
        },
    ];
    for bad in [f64::NAN, f64::INFINITY, -1.0, 0.0, 1.1] {
        invalid.push(NoteTransform::ChopPattern {
            origin: 0,
            period: 480,
            steps: vec![ChopStep { gate: bad, ..step }],
        });
        invalid.push(NoteTransform::Arpeggiate {
            rate: 120,
            gate: bad,
            octaves: 1,
            repetitions: 1,
            direction: ArpDirection::Ascending,
        });
    }
    for bad in [f64::NAN, f64::INFINITY, -0.1, 4.1] {
        invalid.push(NoteTransform::ChopPattern {
            origin: 0,
            period: 480,
            steps: vec![ChopStep {
                velocity: bad,
                ..step
            }],
        });
        invalid.push(NoteTransform::Flam {
            interval: 30,
            velocity: bad,
            position: FlamPosition::Before,
        });
    }
    for transform in invalid {
        let before = doc.project().clone();
        let history = doc.history();
        let dirty = doc.is_dirty();
        assert!(
            doc.dispatch(
                Command::TransformNotes {
                    pattern: PatternId(1),
                    channel,
                    notes: selected(&doc, channel),
                    transform
                },
                None
            )
            .is_err()
        );
        assert_eq!(doc.project(), &before);
        assert_eq!(doc.history(), history);
        assert_eq!(doc.is_dirty(), dirty);
    }
    for (input, tool) in [
        (
            vec![note(1, 0, 1, 60)],
            NoteTransform::Flam {
                interval: 1,
                velocity: 0.5,
                position: FlamPosition::Before,
            },
        ),
        (
            vec![note(1, 245759, 1, 60)],
            NoteTransform::Flam {
                interval: 1,
                velocity: 0.5,
                position: FlamPosition::After,
            },
        ),
        (
            vec![note(1, 0, 120, 127)],
            NoteTransform::Arpeggiate {
                rate: 120,
                gate: 1.0,
                octaves: 2,
                repetitions: 1,
                direction: ArpDirection::Ascending,
            },
        ),
        (
            vec![note(1, 0, 16385, 60)],
            NoteTransform::ChopPattern {
                origin: 0,
                period: 1,
                steps: vec![step],
            },
        ),
        (
            vec![note(1, 0, 16385, 60)],
            NoteTransform::Arpeggiate {
                rate: 1,
                gate: 1.0,
                octaves: 1,
                repetitions: 0,
                direction: ArpDirection::Ascending,
            },
        ),
        (
            (1..=8193).map(|id| note(id, 30, 1, 60)).collect(),
            NoteTransform::Flam {
                interval: 1,
                velocity: 0.5,
                position: FlamPosition::Before,
            },
        ),
        (
            (1..=16384).map(|id| note(id, 0, id, 60)).collect(),
            reshape(RhythmMode::Add, 240),
        ),
    ] {
        assert!(transform_selected_notes(&input, tool).is_err());
    }
}

#[test]
fn capped_output_stale_selection_and_id_exhaustion_refuse_without_mutation() {
    let (base, channel, other) = fixture();
    let unit = ChopStep {
        tick: 0,
        gate: 1.0,
        velocity: 1.0,
    };
    for tool in [
        NoteTransform::ChopPattern {
            origin: 0,
            period: 1,
            steps: vec![unit],
        },
        NoteTransform::Arpeggiate {
            rate: 1,
            gate: 1.0,
            octaves: 1,
            repetitions: 0,
            direction: ArpDirection::Ascending,
        },
    ] {
        let mut project = base.project().clone();
        let lane = project.patterns[0]
            .lanes
            .iter_mut()
            .find(|l| l.channel == channel)
            .unwrap();
        lane.notes[0].length = 16385;
        let mut doc = Document::new(project.clone());
        let history = doc.history();
        assert!(
            doc.dispatch(
                Command::TransformNotes {
                    pattern: PatternId(1),
                    channel,
                    notes: vec![selected(&doc, channel)[0]],
                    transform: tool
                },
                None
            )
            .is_err()
        );
        assert_eq!(doc.project(), &project);
        assert_eq!(doc.history(), history);
        assert!(!doc.is_dirty());
    }
    for tool in tools().into_iter().filter(|t| {
        !matches!(
            t,
            NoteTransform::RhythmReshape {
                mode: RhythmMode::Remove | RhythmMode::Shift,
                ..
            }
        )
    }) {
        let mut project = base.project().clone();
        project.next_id = u32::MAX;
        let mut doc = Document::new(project.clone());
        let history = doc.history();
        let input = vec![selected(&doc, channel)[1]];
        for (target, notes) in [
            (other, input.clone()),
            (
                channel,
                vec![Note {
                    length: 239,
                    ..input[0]
                }],
            ),
            (channel, input),
        ] {
            assert!(
                doc.dispatch(
                    Command::TransformNotes {
                        pattern: PatternId(1),
                        channel: target,
                        notes,
                        transform: tool.clone()
                    },
                    None
                )
                .is_err()
            );
            assert_eq!(doc.project(), &project);
            assert_eq!(doc.history(), history);
            assert!(!doc.is_dirty());
        }
    }
    // The cap itself is allowed, the first result retains its ID.
    let allowed = transform_selected_notes(
        &[note(9, 0, 16384, 60)],
        NoteTransform::ChopPattern {
            origin: 0,
            period: 1,
            steps: vec![unit],
        },
    )
    .unwrap();
    assert_eq!(allowed.len(), MAX_TOOL_NOTES);
    assert_eq!(allowed[0].id, NoteId(9));
}

#[test]
fn original_span_single_hit_arpeggio_is_a_real_noop_and_chords_are_independent() {
    let source = note(1, 480, 120, 60);
    let tool = NoteTransform::Arpeggiate {
        rate: 120,
        gate: 1.0,
        octaves: 1,
        repetitions: 0,
        direction: ArpDirection::Ascending,
    };
    assert_eq!(
        transform_selected_notes(&[source], tool.clone()).unwrap(),
        vec![source]
    );
    let input = [
        note(1, 0, 240, 60),
        note(2, 0, 240, 64),
        note(3, 360, 360, 67),
    ];
    let output = transform_selected_notes(&input, tool).unwrap();
    assert_eq!(
        output
            .iter()
            .map(|n| (n.start, n.key, n.id.0))
            .collect::<Vec<_>>(),
        vec![
            (0, 60, 1),
            (120, 64, 2),
            (360, 67, 3),
            (480, 67, 0),
            (600, 67, 0)
        ]
    );
}
