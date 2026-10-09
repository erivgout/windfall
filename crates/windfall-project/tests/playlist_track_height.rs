use windfall_project::{Command, Document, PlaylistTrackId, PlaylistTrackPatch, Project, file};

fn document() -> (Document, PlaylistTrackId) {
    let mut doc = Document::new(Project::new("Track height"));
    let added = doc
        .dispatch(
            Command::AddPlaylistTrack {
                name: None,
                index: None,
            },
            None,
        )
        .unwrap();
    let id = PlaylistTrackId(added.created[0]);
    (doc, id)
}

fn set_height(doc: &mut Document, id: PlaylistTrackId, height: Option<u32>) {
    doc.dispatch(
        Command::UpdatePlaylistTrack {
            id,
            patch: PlaylistTrackPatch {
                height,
                name: Some("Resized".into()),
                ..PlaylistTrackPatch::default()
            },
        },
        None,
    )
    .unwrap();
}

#[test]
fn new_and_legacy_tracks_default_to_global_height() {
    let (doc, _) = document();
    assert_eq!(doc.project().playlist.tracks[0].height, 0);
    let saved = file::to_json(doc.project()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&saved).unwrap();
    assert!(value["playlist"]["tracks"][0].get("height").is_none());
    assert_eq!(
        file::from_json(&saved).unwrap().playlist.tracks[0].height,
        0
    );
}

#[test]
fn patch_clamps_height_without_rejecting_other_fields() {
    let (mut doc, id) = document();
    for (input, expected) in [
        (1, 18),
        (17, 18),
        (18, 18),
        (80, 80),
        (160, 160),
        (161, 160),
        (u32::MAX, 160),
    ] {
        set_height(&mut doc, id, Some(input));
        assert_eq!(doc.project().playlist.tracks[0].height, expected);
        assert_eq!(doc.project().playlist.tracks[0].name, "Resized");
        doc.project().check().unwrap();
    }
}

#[test]
fn absent_patch_preserves_height_and_zero_clears_with_undo_redo() {
    let (mut doc, id) = document();
    set_height(&mut doc, id, Some(80));
    set_height(&mut doc, id, None);
    assert_eq!(doc.project().playlist.tracks[0].height, 80);
    set_height(&mut doc, id, Some(0));
    assert_eq!(doc.project().playlist.tracks[0].height, 0);
    doc.undo().unwrap();
    assert_eq!(doc.project().playlist.tracks[0].height, 80);
    doc.redo().unwrap();
    assert_eq!(doc.project().playlist.tracks[0].height, 0);
}

#[test]
fn saved_height_round_trips_and_zero_is_omitted() {
    let (mut doc, id) = document();
    for height in [80, 0] {
        set_height(&mut doc, id, Some(height));
        let saved = file::to_json(doc.project()).unwrap();
        assert_eq!(file::from_json(&saved).unwrap(), *doc.project());
        let value: serde_json::Value = serde_json::from_str(&saved).unwrap();
        let field = value["playlist"]["tracks"][0].get("height");
        if height == 0 {
            assert!(field.is_none());
        } else {
            assert_eq!(field.unwrap(), height);
        }
    }
}
