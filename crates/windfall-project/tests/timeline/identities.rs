//! Numeric wire identities stay compatible while Rust distinguishes entities.
use serde_json::{Value, json};
use windfall_project::*;

fn command(value: Value) -> Command {
    serde_json::from_value(value).unwrap()
}

#[test]
fn numeric_ids_preserve_v1_wire_commands_allocator_and_history() {
    let mut doc = Document::new(Project::new("Typed timeline"));
    let first = doc.project().next_id;
    let applied = doc
        .dispatch(
            command(json!({"type":"batch","commands":[
                {"type":"addMeterChange","tick":4001,"signature":{"numerator":7,"denominator":8}},
                {"type":"addTimelineMarker","tick":17,"name":"Verse","kind":{"type":"named"}}
            ]})),
            None,
        )
        .unwrap();
    assert_eq!(applied.created, [first, first + 1]);
    let meter = MeterChangeId(first);
    let marker = TimelineMarkerId(first + 1);
    assert_eq!(doc.project().playlist.timeline.meters[0].id, meter);
    assert_eq!(doc.project().playlist.timeline.markers[0].id, marker);
    assert_eq!(serde_json::to_value(meter).unwrap(), json!(first));
    assert_eq!(serde_json::to_value(marker).unwrap(), json!(first + 1));
    let saved = file::to_json(doc.project()).unwrap();
    let wire: Value = serde_json::from_str(&saved).unwrap();
    assert_eq!(wire["formatVersion"], FORMAT_VERSION);
    assert_eq!(wire["playlist"]["timeline"]["meters"][0]["id"], first);
    assert_eq!(wire["playlist"]["timeline"]["markers"][0]["id"], first + 1);
    assert_eq!(file::from_json(&saved).unwrap(), *doc.project());
    let original = doc.project().playlist.timeline.clone();
    for update in [
        json!({"type":"updateMeterChange","id":first,"tick":4201,"signature":{"numerator":3,"denominator":4}}),
        json!({"type":"updateTimelineMarker","marker":{"id":first+1,"tick":29,"name":"Chorus","kind":{"type":"named"}}}),
    ] {
        doc.dispatch(command(update), None).unwrap();
    }
    let updated = doc.project().playlist.timeline.clone();
    for remove in [
        json!({"type":"removeMeterChange","id":first}),
        json!({"type":"removeTimelineMarker","id":first+1}),
    ] {
        doc.dispatch(command(remove), None).unwrap();
    }
    assert!(doc.project().playlist.timeline.is_empty());
    doc.undo().unwrap();
    doc.undo().unwrap();
    assert_eq!(doc.project().playlist.timeline, updated);
    doc.undo().unwrap();
    doc.undo().unwrap();
    assert_eq!(doc.project().playlist.timeline, original);
    doc.redo().unwrap();
    doc.redo().unwrap();
    assert_eq!(doc.project().playlist.timeline, updated);
    doc.undo().unwrap();
    doc.undo().unwrap();
    doc.undo().unwrap();
    let next = doc.project().next_id;
    assert!(next > first + 1);
    let fresh = doc.dispatch(command(json!({"type":"addMeterChange","tick":0,"signature":{"numerator":5,"denominator":8}})), None).unwrap();
    assert_eq!(fresh.created, [next]);
    assert_eq!(
        doc.project().playlist.timeline.meters[0].id,
        MeterChangeId(next)
    );
}

#[test]
fn invalid_numeric_and_wrong_entity_ids_refuse_atomically() {
    let mut doc = Document::new(Project::new("Identity refusal"));
    let first = doc.project().next_id;
    doc.dispatch(
        command(json!({"type":"batch","commands":[
            {"type":"addMeterChange","tick":0,"signature":{"numerator":7,"denominator":8}},
            {"type":"addTimelineMarker","tick":17,"name":"Verse","kind":{"type":"named"}}
        ]})),
        None,
    )
    .unwrap();
    for id in [
        json!(-1),
        json!(1.5),
        json!(u64::from(u32::MAX) + 1),
        json!("1"),
        json!({"id":1}),
        Value::Null,
    ] {
        assert!(
            serde_json::from_value::<Command>(json!({"type":"removeMeterChange","id":id})).is_err()
        );
        assert!(
            serde_json::from_value::<Command>(json!({"type":"removeTimelineMarker","id":id}))
                .is_err()
        );
    }
    for invalid in [
        json!({"type":"removeMeterChange","id":0}),
        json!({"type":"removeMeterChange","id":first+1}),
        json!({"type":"removeTimelineMarker","id":first}),
        json!({"type":"updateMeterChange","id":first+1,"tick":99,"signature":{"numerator":3,"denominator":4}}),
        json!({"type":"updateTimelineMarker","marker":{"id":first,"tick":29,"name":"Wrong family","kind":{"type":"named"}}}),
    ] {
        let before = doc.snapshot(None);
        assert!(doc.dispatch(command(invalid), None).is_err());
        assert_eq!(doc.snapshot(None), before);
    }
    let wire = serde_json::to_value(doc.project()).unwrap();
    for id in [0, first, doc.project().next_id, u32::MAX] {
        let mut invalid = wire.clone();
        invalid["playlist"]["timeline"]["markers"][0]["id"] = json!(id);
        assert!(file::from_json(&invalid.to_string()).is_err());
    }
}
