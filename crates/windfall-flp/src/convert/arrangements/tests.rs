//! Synthetic import projects are original test data, CC0-1.0.
use crate::convert::{ConvertOptions, convert};
use crate::model::{
    Arrangement, Channel, ChannelKind, FlpProject, LegacyPlaylistItem, Pattern, PlaylistItem,
    PlaylistSource, Plugin, TimeMarker, Track,
};
use windfall_project::{ArrangementBook, ChannelSource, Command, Document};

fn item(position: u32, track: u16) -> PlaylistItem {
    PlaylistItem {
        position,
        length: 384,
        track,
        group: 0,
        flags: 0,
        source: PlaylistSource::Pattern {
            pattern: 1,
            start: Some(0),
            end: None,
        },
        extra: None,
    }
}

fn source() -> FlpProject {
    FlpProject {
        patterns: vec![Pattern {
            iid: 1,
            length: Some(384),
            ..Default::default()
        }],
        arrangements: vec![
            Arrangement {
                index: 2,
                name: Some("Short version".into()),
                items: vec![item(384, 1), item(1152, 9)],
                markers: vec![TimeMarker {
                    position: 0,
                    name: Some("Alternate opening".into()),
                    ..Default::default()
                }],
                ..Default::default()
            },
            Arrangement {
                index: 7,
                name: Some("Full version".into()),
                items: vec![item(384, 1), item(768, 1)],
                tracks: vec![Track {
                    iid: 2,
                    name: Some("Lead".into()),
                    enabled: true,
                    height: 1.0,
                    ..Default::default()
                }],
                markers: vec![
                    TimeMarker {
                        position: 0,
                        kind: 8,
                        numerator: Some(3),
                        denominator: Some(4),
                        ..Default::default()
                    },
                    TimeMarker {
                        position: 768,
                        name: Some("Chorus".into()),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            },
        ],
        current_arrangement: Some(7),
        ..Default::default()
    }
}

#[test]
fn named_arrangements_keep_stable_ids_and_only_existing_references() {
    let source = source();
    let imported = convert(&source, &ConvertOptions::default());
    imported.project.check().unwrap();
    let playlist = &imported.project.playlist;
    let book = &playlist.arrangement_book;
    assert_eq!(
        book.arrangements
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        ["Short version", "Full version"]
    );
    assert_ne!(book.arrangements[0].id, book.arrangements[1].id);
    assert_eq!(book.active, Some(book.arrangements[1].id));
    assert_eq!(book.arrangements[0].clips, [playlist.clips[0].id]);
    assert_eq!(book.arrangements[0].tracks, [playlist.tracks[0].id]);
    assert_eq!(
        book.arrangements[1].clips,
        playlist.clips.iter().map(|c| c.id).collect::<Vec<_>>()
    );
    assert_eq!(book.arrangements[1].tracks, [playlist.tracks[0].id]);
    assert_eq!(playlist.clips.len(), 2);
    assert_eq!(
        book,
        &convert(&source, &ConvertOptions::default())
            .project
            .playlist
            .arrangement_book
    );
    let report = format!("{:?}", imported.report);
    assert!(report.contains("1 clip placements without matching playlist references"));
    assert!(report.contains("1 timeline markers that were left out"));
    assert!(report.contains("playback still uses one playlist"));
    assert!(!report.contains("other 1 left out"));
}

#[test]
fn selected_playlist_and_timeline_equal_a_single_arrangement_import_and_switching_keeps_them() {
    let source = source();
    let imported = convert(&source, &ConvertOptions::default());
    let mut single = source.clone();
    single.arrangements.remove(0);
    let baseline = convert(&single, &ConvertOptions::default());
    let mut playlist = imported.project.playlist.clone();
    playlist.arrangement_book = ArrangementBook::default();
    assert_eq!(playlist, baseline.project.playlist);
    assert_eq!(
        (playlist.clips[0].start, playlist.clips[1].start),
        (3840, 7680)
    );
    assert_eq!(playlist.timeline.markers[0].name, "Chorus");
    assert_eq!(playlist.timeline.markers[0].tick, 7680);
    assert_eq!(playlist.timeline.meters[0].signature.numerator, 3);
    assert_eq!(playlist.timeline.meters[0].signature.denominator, 4);

    let other_id = imported.project.playlist.arrangement_book.arrangements[0].id;
    let mut document = Document::new(imported.project);
    document
        .dispatch(Command::SwitchArrangement { id: other_id }, None)
        .unwrap();
    let switched = &document.project().playlist;
    assert_eq!(switched.clips, playlist.clips);
    assert_eq!(switched.tracks, playlist.tracks);
    assert_eq!(switched.timeline, playlist.timeline);
    assert_eq!(switched.arrangement_book.active, Some(other_id));
    document.project().check().unwrap();
}

#[test]
fn one_arrangement_keeps_legacy_book_and_allocator() {
    let mut source = source();
    source.arrangements.remove(0);
    let named = convert(&source, &ConvertOptions::default());
    source.arrangements[0].name = None;
    let unnamed = convert(&source, &ConvertOptions::default());
    assert!(named.project.playlist.arrangement_book.is_empty());
    assert_eq!(named, unnamed);
    assert_eq!(named.project.next_id, unnamed.project.next_id);
}

#[test]
fn distinct_layout_still_keeps_its_name_and_identity_without_dangling_references() {
    let mut source = source();
    source.arrangements[0].items = vec![item(1536, 9)];
    source.arrangements[0].tracks.clear();
    source.current_arrangement = Some(999); // Use the same first-arrangement fallback as playback.
    let imported = convert(&source, &ConvertOptions::default());
    let book = &imported.project.playlist.arrangement_book;
    assert_eq!(book.active, Some(book.arrangements[0].id));
    assert_eq!(book.arrangements[1].name, "Full version");
    assert!(book.arrangements[1].clips.is_empty());
    assert!(book.arrangements[1].tracks.is_empty());
    assert_eq!(imported.project.playlist.clips[0].start, 15360);
    imported.project.check().unwrap();
}

#[test]
fn repeated_and_legacy_items_use_each_available_reference_once_and_skip_failed_clips() {
    let mut source = source();
    source.arrangements[1].items = vec![item(384, 1), item(384, 1), item(0, 1)];
    source.arrangements[1].items[2].length = 0;
    source.arrangements[0].items = vec![item(384, 1); 3];
    let rejected = source.arrangements[1].items[2];
    source.arrangements[0].items.push(rejected);
    let legacy = LegacyPlaylistItem { bar: 2, pattern: 1 };
    source.arrangements[1].legacy_items = vec![legacy];
    source.arrangements[0].legacy_items = vec![legacy];
    let imported = convert(&source, &ConvertOptions::default());
    let playlist = &imported.project.playlist;
    assert_eq!(playlist.clips.len(), 3);
    assert_eq!(playlist.arrangement_book.arrangements[0].clips.len(), 3);
    imported.project.check().unwrap();
}

#[test]
fn name_limits_do_not_lose_entries_and_unsupported_native_state_stays_silent() {
    let mut source = source();
    source.arrangements[0].name = Some("\0  ".into());
    source.arrangements[1].name = Some("é".repeat(129));
    source.channels.push(Channel {
        iid: 0,
        kind: ChannelKind::Generator,
        plugin: Some(Plugin {
            internal_name: "Original unsupported generator".into(),
            state: vec![1, 2, 3],
            ..Default::default()
        }),
        ..Default::default()
    });
    let imported = convert(&source, &ConvertOptions::default());
    let book = &imported.project.playlist.arrangement_book;
    assert_eq!(book.arrangements.len(), 2);
    assert_eq!(book.arrangements[0].name, "Arrangement 3");
    assert_eq!(book.arrangements[1].name, "é".repeat(128));
    assert!(
        matches!(&imported.project.channels[0].source, ChannelSource::Sampler(sampler) if sampler.sample.is_none())
    );
    assert_eq!(imported.plugins[0].state, [1, 2, 3]);
    assert_eq!(imported.project.retained_plugins[0].state, [1, 2, 3]);
    assert!(format!("{:?}", imported.report).contains("Original unsupported generator"));
    imported.project.check().unwrap();
}
