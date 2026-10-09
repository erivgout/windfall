//! Independent acceptance checks for non-destructive channel timing.
use windfall_project::*;

fn fixture() -> (Document, ChannelId) {
    let mut doc = Document::new(Project::new("Timing QA"));
    let id = ChannelId(
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
    (doc, id)
}

fn edit(id: ChannelId, timing: ChannelTiming) -> Command {
    Command::UpdateChannel {
        id,
        patch: ChannelPatch {
            timing: Some(timing),
            ..Default::default()
        },
    }
}

#[test]
fn signed_timing_controls_persist_undo_redo_and_legacy_defaults() {
    let (mut doc, id) = fixture();
    for shift_ticks in [-960, 0, 960] {
        let before = doc.project().channel(id).unwrap().timing;
        let timing = ChannelTiming {
            swing_mix: 0.5,
            gate_ticks: 137,
            shift_ticks,
        };
        doc.dispatch(edit(id, timing), None).unwrap();
        assert_eq!(doc.project().channel(id).unwrap().timing, timing);
        doc.undo().unwrap();
        assert_eq!(doc.project().channel(id).unwrap().timing, before);
        doc.redo().unwrap();
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("timing.windfall");
        file::save(doc.project(), &path).unwrap();
        assert_eq!(
            file::load(&path).unwrap().channel(id).unwrap().timing,
            timing
        );
    }
    let mut legacy = serde_json::to_value(doc.project()).unwrap();
    legacy["channels"][0]
        .as_object_mut()
        .unwrap()
        .remove("timing");
    assert_eq!(
        file::from_json(&legacy.to_string())
            .unwrap()
            .channel(id)
            .unwrap()
            .timing,
        ChannelTiming::default()
    );
}

#[test]
fn invalid_timing_edits_are_atomic_and_do_not_enter_history() {
    let (mut doc, id) = fixture();
    for timing in [
        ChannelTiming {
            swing_mix: f32::NAN,
            ..Default::default()
        },
        ChannelTiming {
            swing_mix: f32::INFINITY,
            ..Default::default()
        },
        ChannelTiming {
            swing_mix: -0.01,
            ..Default::default()
        },
        ChannelTiming {
            swing_mix: 1.01,
            ..Default::default()
        },
        ChannelTiming {
            gate_ticks: MAX_PATTERN_TICKS + 1,
            ..Default::default()
        },
        ChannelTiming {
            shift_ticks: 961,
            ..Default::default()
        },
        ChannelTiming {
            shift_ticks: -961,
            ..Default::default()
        },
        ChannelTiming {
            shift_ticks: i32::MIN,
            ..Default::default()
        },
    ] {
        let before = doc.project().clone();
        let history = doc.history();
        assert!(doc.dispatch(edit(id, timing), None).is_err());
        assert_eq!(doc.project(), &before);
        assert_eq!(doc.history(), history);
    }
}

#[test]
fn timing_kernel_has_independent_swing_gate_shift_and_boundary_references() {
    // A sixteenth is 240 ticks. Full swing delays its odd boundary by 80;
    // half channel mix therefore delays it by 40 and shortens it to 200.
    let half = ChannelTiming {
        swing_mix: 0.5,
        ..Default::default()
    };
    assert_eq!(half.place(240, 240, 960, 1.0), Some((280.0, 200.0)));
    let capped = ChannelTiming {
        gate_ticks: 75,
        shift_ticks: 19,
        ..half
    };
    assert_eq!(capped.place(240, 240, 960, 1.0), Some((299.0, 75.0)));
    assert_eq!(
        ChannelTiming {
            swing_mix: 0.0,
            ..capped
        }
        .place(240, 240, 960, 1.0),
        Some((259.0, 75.0))
    );
    let early = ChannelTiming {
        shift_ticks: -960,
        ..Default::default()
    };
    assert_eq!(early.place(240, 240, 960, 0.0), Some((0.0, 240.0)));
    let late = ChannelTiming {
        shift_ticks: 240,
        ..Default::default()
    };
    assert_eq!(late.place(720, 240, 960, 0.0), None);
    assert_eq!(ChannelTiming::default().place(960, 1, 960, 0.0), None);
    // A trailing unpaired step in 3/8 (six steps) or an odd five-step
    // pattern is not pulled into a nonexistent swing pair.
    for length in [1200, 1440, 1680, 2880] {
        let paired_end = length / 480 * 480;
        if paired_end < length {
            assert_eq!(
                half.place(paired_end, 240, length, 1.0),
                Some((f64::from(paired_end), 240.0))
            );
        }
        assert_eq!(half.place(480, 240, length, 1.0), Some((480.0, 280.0)));
    }
}
