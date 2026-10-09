//! Pattern-local timelines compose with song metadata through real SMF bytes.
use windfall_midi::{TimeSignatureChange, export_pattern, export_song, read, write};
use windfall_project::*;

fn edit(doc: &mut Document, edit: PatternTimelineEdit) {
    let pattern = doc.project().pattern(PatternId(1)).unwrap();
    doc.dispatch(
        Command::EditPatternTimeline {
            pattern: pattern.id,
            expected: pattern.timeline.clone(),
            expected_signature: pattern.time_signature,
            edit,
        },
        None,
    )
    .unwrap();
}

#[test]
fn local_meter_edits_preserve_ticks_and_export_pattern_and_song_maps_independently() {
    let mut doc = Document::new(Project::new("Mixed meters"));
    doc.dispatch(
        Command::UpdatePattern {
            id: PatternId(1),
            patch: PatternPatch {
                length_steps: Some(32),
                ..Default::default()
            },
        },
        None,
    )
    .unwrap();
    let channel = ChannelId(
        doc.dispatch(
            Command::AddChannel {
                name: None,
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
    doc.dispatch(
        Command::AddNotes {
            pattern: PatternId(1),
            channel,
            notes: vec![
                NoteInit {
                    start: 120,
                    length: 240,
                    key: 60,
                    velocity: Some(0.7),
                    pan: None,
                    expression: None,
                },
                NoteInit {
                    start: 4100,
                    length: 360,
                    key: 67,
                    velocity: Some(0.8),
                    pan: None,
                    expression: None,
                },
            ],
        },
        None,
    )
    .unwrap();
    let track = PlaylistTrackId(
        doc.dispatch(
            Command::AddPlaylistTrack {
                name: None,
                index: None,
            },
            None,
        )
        .unwrap()
        .created[0],
    );
    doc.dispatch(
        Command::AddClips {
            clips: vec![ClipInit {
                track,
                start: 960,
                length: Some(7680),
                offset: Some(0),
                muted: None,
                content: ClipContent::Pattern {
                    pattern: PatternId(1),
                },
            }],
        },
        None,
    )
    .unwrap();
    let lanes = doc.project().patterns[0].lanes.clone();
    let clips = doc.project().playlist.clips.clone();
    let before_export = export_pattern(doc.project(), PatternId(1), &Default::default()).unwrap();
    let before_song = export_song(doc.project(), &Default::default()).unwrap();
    edit(
        &mut doc,
        PatternTimelineEdit::SetSignature {
            signature: Some(TimeSignature {
                numerator: 3,
                denominator: 4,
            }),
        },
    );
    let cursor = doc.history().cursor;
    edit(
        &mut doc,
        PatternTimelineEdit::AddMeter {
            tick: 4001,
            signature: TimeSignature {
                numerator: 7,
                denominator: 8,
            },
        },
    );
    assert_eq!(doc.history().cursor, cursor + 1);
    let after_meter = doc.project().clone();
    doc.undo().unwrap();
    assert!(doc.project().patterns[0].timeline.meters.is_empty());
    doc.redo().unwrap();
    assert_eq!(doc.project(), &after_meter);
    edit(
        &mut doc,
        PatternTimelineEdit::AddMarker {
            tick: 4100,
            name: "Local phrase".into(),
        },
    );
    doc.dispatch(
        Command::AddMeterChange {
            tick: 4001,
            signature: TimeSignature {
                numerator: 5,
                denominator: 4,
            },
        },
        None,
    )
    .unwrap();
    doc.dispatch(
        Command::AddTimelineMarker {
            tick: 5100,
            name: "Song section".into(),
            kind: MarkerKind::Named,
        },
        None,
    )
    .unwrap();
    assert_eq!(doc.project().patterns[0].lanes, lanes);
    assert_eq!(doc.project().playlist.clips, clips);
    let pattern_export = export_pattern(doc.project(), PatternId(1), &Default::default()).unwrap();
    assert_eq!(
        pattern_export.tracks, before_export.tracks,
        "meter metadata never retimes notes"
    );
    let pattern_bytes = read(&write(&pattern_export, &Default::default())).unwrap();
    assert_eq!(
        pattern_bytes.time_signatures,
        [
            TimeSignatureChange {
                tick: 0,
                numerator: 3,
                denominator: 4
            },
            TimeSignatureChange {
                tick: 4001,
                numerator: 7,
                denominator: 8
            },
        ]
    );
    assert_eq!(
        pattern_bytes
            .markers
            .iter()
            .map(|marker| (marker.tick, marker.text.as_str()))
            .collect::<Vec<_>>(),
        [(4100, "Local phrase")]
    );
    let song_export = export_song(doc.project(), &Default::default()).unwrap();
    assert_eq!(song_export.tracks, before_song.tracks);
    assert_eq!(song_export.length, before_song.length);
    let song_bytes = read(&write(&song_export, &Default::default())).unwrap();
    assert_eq!(
        song_bytes.time_signatures,
        [
            TimeSignatureChange {
                tick: 0,
                numerator: 4,
                denominator: 4
            },
            TimeSignatureChange {
                tick: 4001,
                numerator: 5,
                denominator: 4
            },
        ]
    );
    assert_eq!(
        song_bytes
            .markers
            .iter()
            .map(|marker| (marker.tick, marker.text.as_str()))
            .collect::<Vec<_>>(),
        [(5100, "Song section")]
    );
    let song_starts: Vec<_> = song_bytes
        .tracks
        .iter()
        .flat_map(|track| &track.notes)
        .map(|note| note.start)
        .collect();
    assert!(song_starts.contains(&(960 + 120)));
    assert!(song_starts.contains(&(960 + 4100)));
    let after = doc.project().clone();
    assert_eq!(
        file::from_json(&file::to_json(&after).unwrap()).unwrap(),
        after
    );
    let stale = Command::EditPatternTimeline {
        pattern: PatternId(1),
        expected: Timeline::default(),
        expected_signature: None,
        edit: PatternTimelineEdit::AddMarker {
            tick: 100,
            name: "Stale".into(),
        },
    };
    let cursor = doc.history().cursor;
    assert!(doc.dispatch(stale, None).is_err());
    assert_eq!(doc.project(), &after);
    assert_eq!(doc.history().cursor, cursor);
    let meter = doc.project().patterns[0].timeline.meters[0];
    let marker = doc.project().patterns[0].timeline.markers[0].id;
    edit(&mut doc, PatternTimelineEdit::RemoveMeter { id: meter.id });
    edit(&mut doc, PatternTimelineEdit::RemoveMarker { id: marker });
    assert!(doc.project().patterns[0].timeline.is_empty());
    assert_eq!(doc.project().playlist.timeline.meters.len(), 1);
    assert_eq!(doc.project().playlist.timeline.markers.len(), 1);
    assert_eq!(doc.project().patterns[0].lanes, lanes);
}
