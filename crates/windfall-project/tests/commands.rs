//! What each command does: its success path, each way it fails, and what it
//! cascades to. Every successful command is also undone and redone.

use windfall_project::*;

const PATTERN: PatternId = PatternId(1);

fn document() -> Document {
    Document::new(Project::new("Test"))
}

/// Equal apart from `next_id`, which undo leaves alone.
fn same_content(a: &Project, b: &Project) -> bool {
    let mut b = b.clone();
    b.next_id = a.next_id;
    *a == b
}

/// Dispatches a command that must succeed, then checks that the project is
/// valid and that undo and redo restore it exactly.
#[track_caller]
fn run(doc: &mut Document, command: Command) -> Applied {
    let before = doc.project().clone();
    let cursor = doc.history().cursor;
    let applied = match doc.dispatch(command.clone(), None) {
        Ok(applied) => applied,
        Err(error) => panic!("{command:?} failed: {error}"),
    };
    doc.project().check().expect("the project is valid");
    let after = doc.project().clone();
    if doc.history().cursor == cursor {
        assert!(applied.touched.is_empty(), "an edit left no history entry");
        assert!(same_content(&before, &after));
    } else {
        doc.undo().expect("there is an edit to undo");
        assert!(
            same_content(doc.project(), &before),
            "undo did not restore the project"
        );
        doc.project().check().expect("the undone project is valid");
        doc.redo().expect("there is an edit to redo");
        assert_eq!(doc.project(), &after, "redo did not restore the project");
    }
    applied
}

/// Dispatches a command that must fail, and checks that nothing changed.
#[track_caller]
fn fail(doc: &mut Document, command: Command) -> CommandError {
    let before = doc.project().clone();
    let history = doc.history();
    let dirty = doc.is_dirty();
    let error = match doc.dispatch(command.clone(), None) {
        Ok(_) => panic!("{command:?} should have failed"),
        Err(error) => error,
    };
    assert_eq!(
        doc.project(),
        &before,
        "a failed command changed the project"
    );
    assert_eq!(doc.history(), history);
    assert_eq!(doc.is_dirty(), dirty);
    error
}

#[track_caller]
fn assert_invalid(error: CommandError, words: &str) {
    match error {
        CommandError::Invalid(message) => {
            assert!(message.contains(words), "\"{message}\" lacks \"{words}\"");
        }
        other => panic!("expected an invalid-input error, got {other:?}"),
    }
}

fn not_found(kind: &'static str, id: u32) -> CommandError {
    CommandError::NotFound { kind, id }
}

fn add_channel(doc: &mut Document, name: &str) -> (ChannelId, TrackId) {
    let applied = run(
        doc,
        Command::AddChannel {
            name: Some(name.to_owned()),
            sample: None,
            index: None,
            mixer_track: None,
        },
    );
    (ChannelId(applied.created[0]), TrackId(applied.created[1]))
}

fn add_sample(doc: &mut Document, name: &str) -> SampleId {
    let applied = run(
        doc,
        Command::AddSample {
            name: name.to_owned(),
            path: SamplePath::Factory(format!("drums/{name}.wav")),
        },
    );
    SampleId(applied.created[0])
}

fn add_mixer_track(doc: &mut Document) -> TrackId {
    TrackId(run(doc, Command::AddMixerTrack { name: None }).created[0])
}

fn add_playlist_track(doc: &mut Document) -> PlaylistTrackId {
    PlaylistTrackId(run(doc, Command::AddPlaylistTrack { name: None }).created[0])
}

fn add_pattern(doc: &mut Document) -> PatternId {
    PatternId(run(doc, Command::AddPattern { name: None }).created[0])
}

fn note(start: u32, key: u8) -> NoteInit {
    NoteInit {
        start,
        length: 240,
        key,
        velocity: None,
        pan: None,
    }
}

fn add_notes(doc: &mut Document, channel: ChannelId, notes: Vec<NoteInit>) -> Vec<NoteId> {
    let applied = run(
        doc,
        Command::AddNotes {
            pattern: PATTERN,
            channel,
            notes,
        },
    );
    applied.created.into_iter().map(NoteId).collect()
}

fn add_clip(doc: &mut Document, track: PlaylistTrackId, pattern: PatternId, start: u32) -> ClipId {
    let applied = run(
        doc,
        Command::AddClips {
            clips: vec![ClipInit {
                track,
                start,
                length: None,
                content: ClipContent::Pattern { pattern },
            }],
        },
    );
    ClipId(applied.created[0])
}

fn lane_notes(doc: &Document, pattern: PatternId, channel: ChannelId) -> Vec<Note> {
    let pattern = doc.project().pattern(pattern).expect("the pattern exists");
    pattern
        .lane(channel)
        .map(|lane| lane.notes.clone())
        .unwrap_or_default()
}

fn sampler(doc: &Document, channel: ChannelId) -> SamplerSettings {
    let channel = doc.project().channel(channel).expect("the channel exists");
    let ChannelSource::Sampler(sampler) = &channel.source;
    sampler.clone()
}

fn track(doc: &Document, id: TrackId) -> MixerTrack {
    let track = doc.project().mixer.track(id).expect("the track exists");
    track.clone()
}

fn touched(build: impl FnOnce(&mut Touched)) -> Touched {
    let mut touched = Touched::default();
    build(&mut touched);
    touched
}

fn label(doc: &Document) -> String {
    let history = doc.history();
    history.entries[history.cursor as usize - 1].label.clone()
}

// Project::new

#[test]
fn a_new_project_is_empty_and_valid() {
    let project = Project::new("Song");
    project.check().unwrap();
    assert_eq!(project.format_version, FORMAT_VERSION);
    assert_eq!(project.settings.name, "Song");
    assert_eq!(project.settings.tempo_bpm, 120.0);
    assert_eq!(project.settings.time_signature.numerator, 4);
    assert_eq!(project.settings.time_signature.denominator, 4);
    assert_eq!(project.settings.swing, 0.0);
    assert!(project.samples.is_empty());
    assert!(project.channels.is_empty());
    assert!(project.playlist.tracks.is_empty());
    assert!(project.playlist.clips.is_empty());

    let [pattern] = project.patterns.as_slice() else {
        panic!("expected one pattern");
    };
    assert_eq!(pattern.name, "Pattern 1");
    assert_eq!(pattern.length_steps, 16);
    assert!(pattern.lanes.is_empty());

    let [master] = project.mixer.tracks.as_slice() else {
        panic!("expected only the master track");
    };
    assert_eq!(master.id, TrackId::MASTER);
    assert_eq!(master.name, "Master");
    assert_eq!(master.volume, 1.0);
    assert_eq!(master.output, None);
    assert!(project.next_id > pattern.id.0);
}

// UpdateSettings

#[test]
fn update_settings_changes_each_field() {
    let mut doc = document();
    let applied = run(
        &mut doc,
        Command::UpdateSettings {
            patch: SettingsPatch {
                name: Some("Banger".to_owned()),
                tempo_bpm: Some(140.5),
                time_signature: Some(TimeSignature {
                    numerator: 7,
                    denominator: 8,
                }),
                swing: Some(0.25),
            },
        },
    );
    assert_eq!(applied.touched, touched(|t| t.settings = true));
    assert_eq!(applied.label, "Change project settings");
    assert!(applied.created.is_empty());
    let settings = &doc.project().settings;
    assert_eq!(settings.name, "Banger");
    assert_eq!(settings.tempo_bpm, 140.5);
    assert_eq!(settings.time_signature.numerator, 7);
    assert_eq!(settings.time_signature.denominator, 8);
    assert_eq!(settings.swing, 0.25);
}

fn tempo(bpm: f64) -> Command {
    Command::UpdateSettings {
        patch: SettingsPatch {
            tempo_bpm: Some(bpm),
            ..SettingsPatch::default()
        },
    }
}

#[test]
fn update_settings_labels_a_single_field() {
    let mut doc = document();
    assert_eq!(run(&mut doc, tempo(90.0)).label, "Change tempo");
    let rename = Command::UpdateSettings {
        patch: SettingsPatch {
            name: Some("New".to_owned()),
            ..SettingsPatch::default()
        },
    };
    assert_eq!(run(&mut doc, rename).label, "Rename project");
}

#[test]
fn tempo_and_swing_are_clamped() {
    let mut doc = document();
    run(&mut doc, tempo(1.0));
    assert_eq!(doc.project().settings.tempo_bpm, MIN_TEMPO_BPM);
    run(&mut doc, tempo(f64::INFINITY));
    assert_eq!(doc.project().settings.tempo_bpm, MAX_TEMPO_BPM);

    let swing = |swing| Command::UpdateSettings {
        patch: SettingsPatch {
            swing: Some(swing),
            ..SettingsPatch::default()
        },
    };
    run(&mut doc, swing(3.0));
    assert_eq!(doc.project().settings.swing, 1.0);
    run(&mut doc, swing(-3.0));
    assert_eq!(doc.project().settings.swing, 0.0);
    assert_invalid(fail(&mut doc, swing(f32::NAN)), "not a number");
    assert_invalid(fail(&mut doc, tempo(f64::NAN)), "not a number");
}

#[test]
fn a_bad_time_signature_is_rejected() {
    let mut doc = document();
    let signature = |numerator, denominator| Command::UpdateSettings {
        patch: SettingsPatch {
            // The name must not change either when the signature is bad.
            name: Some("Changed".to_owned()),
            time_signature: Some(TimeSignature {
                numerator,
                denominator,
            }),
            ..SettingsPatch::default()
        },
    };
    assert_invalid(fail(&mut doc, signature(0, 4)), "beats per bar");
    assert_invalid(fail(&mut doc, signature(17, 4)), "beats per bar");
    assert_invalid(fail(&mut doc, signature(4, 3)), "beat unit");
    assert_invalid(fail(&mut doc, signature(4, 0)), "beat unit");
    run(&mut doc, signature(16, 16));
}

#[test]
fn a_command_that_changes_nothing_leaves_no_history() {
    let mut doc = document();
    let applied = run(&mut doc, tempo(120.0));
    assert!(applied.touched.is_empty());
    assert_eq!(applied.label, "Change tempo");
    assert!(doc.history().entries.is_empty());
    assert!(!doc.is_dirty());
}

// Samples

#[test]
fn add_sample_registers_a_file() {
    let mut doc = document();
    let applied = run(
        &mut doc,
        Command::AddSample {
            name: "Kick".to_owned(),
            path: SamplePath::Project("samples/kick.wav".to_owned()),
        },
    );
    assert_eq!(applied.touched, touched(|t| t.samples = true));
    assert_eq!(applied.label, "Add sample");
    let [sample] = doc.project().samples.as_slice() else {
        panic!("expected one sample");
    };
    assert_eq!(applied.created, [sample.id.0]);
    assert_eq!(sample.name, "Kick");
    assert_eq!(
        sample.path,
        SamplePath::Project("samples/kick.wav".to_owned())
    );
}

#[test]
fn add_sample_reuses_a_sample_with_the_same_path() {
    let mut doc = document();
    let first = add_sample(&mut doc, "kick");
    let entries = doc.history().entries.len();
    let applied = run(
        &mut doc,
        Command::AddSample {
            name: "Another name".to_owned(),
            path: SamplePath::Factory("drums/kick.wav".to_owned()),
        },
    );
    assert_eq!(applied.created, [first.0]);
    assert!(applied.touched.is_empty());
    assert_eq!(doc.project().samples.len(), 1);
    assert_eq!(doc.history().entries.len(), entries);

    // The same text under another kind is another file.
    let other = run(
        &mut doc,
        Command::AddSample {
            name: "kick".to_owned(),
            path: SamplePath::Project("drums/kick.wav".to_owned()),
        },
    );
    assert_ne!(other.created, [first.0]);
    assert_eq!(doc.project().samples.len(), 2);
}

#[test]
fn add_sample_rejects_a_malformed_path() {
    let mut doc = document();
    let add = |path| Command::AddSample {
        name: "x".to_owned(),
        path,
    };
    assert_invalid(
        fail(&mut doc, add(SamplePath::Project(String::new()))),
        "empty",
    );
    assert_invalid(
        fail(&mut doc, add(SamplePath::External(String::new()))),
        "empty",
    );
    assert_invalid(
        fail(&mut doc, add(SamplePath::Factory("a\\b.wav".to_owned()))),
        "forward slashes",
    );
    assert_invalid(
        fail(&mut doc, add(SamplePath::Project("../b.wav".to_owned()))),
        "inside its folder",
    );
    assert_invalid(
        fail(&mut doc, add(SamplePath::Project("/b.wav".to_owned()))),
        "inside its folder",
    );
}

#[test]
fn remove_sample_fails_while_a_channel_uses_it() {
    let mut doc = document();
    let sample = add_sample(&mut doc, "kick");
    let applied = run(
        &mut doc,
        Command::AddChannel {
            name: None,
            sample: Some(sample),
            index: None,
            mixer_track: None,
        },
    );
    let channel = ChannelId(applied.created[0]);

    assert_invalid(
        fail(&mut doc, Command::RemoveSample { id: sample }),
        "still used by the channel \"kick\"",
    );
    assert_eq!(
        fail(&mut doc, Command::RemoveSample { id: SampleId(99) }),
        not_found("sample", 99)
    );

    run(
        &mut doc,
        Command::SetChannelSample {
            id: channel,
            sample: None,
        },
    );
    let applied = run(&mut doc, Command::RemoveSample { id: sample });
    assert_eq!(applied.touched, touched(|t| t.samples = true));
    assert_eq!(applied.label, "Delete sample");
    assert!(doc.project().samples.is_empty());
}

// Channels

#[test]
fn add_channel_makes_a_mixer_track_and_routes_to_it() {
    let mut doc = document();
    let applied = run(
        &mut doc,
        Command::AddChannel {
            name: None,
            sample: None,
            index: None,
            mixer_track: None,
        },
    );
    assert_eq!(applied.label, "Add channel");
    assert_eq!(
        applied.touched,
        touched(|t| {
            t.channels = true;
            t.mixer = true;
        })
    );
    let [channel_id, track_id] = applied.created[..] else {
        panic!("expected a channel id and a track id");
    };

    let [channel] = doc.project().channels.as_slice() else {
        panic!("expected one channel");
    };
    assert_eq!(channel.id, ChannelId(channel_id));
    assert_eq!(channel.name, "Sampler");
    assert_eq!(channel.color, PALETTE[0]);
    assert_eq!(channel.volume, DEFAULT_CHANNEL_VOLUME);
    assert_eq!(channel.pan, 0.0);
    assert!(!channel.muted && !channel.solo);
    assert_eq!(channel.mixer_track, TrackId(track_id));
    assert_eq!(
        channel.source,
        ChannelSource::Sampler(SamplerSettings::default())
    );

    let [_, insert] = doc.project().mixer.tracks.as_slice() else {
        panic!("expected the master and one insert");
    };
    assert_eq!(insert.id, TrackId(track_id));
    assert_eq!(insert.name, "Sampler");
    assert_eq!(insert.color, channel.color);
    assert_eq!(insert.volume, 1.0);
    assert_eq!(insert.output, Some(TrackId::MASTER));
    assert!(insert.sends.is_empty());
}

#[test]
fn add_channel_takes_its_name_from_the_sample_unless_given_one() {
    let mut doc = document();
    let sample = add_sample(&mut doc, "snare");
    let add = |name: Option<&str>| Command::AddChannel {
        name: name.map(str::to_owned),
        sample: Some(sample),
        index: None,
        mixer_track: None,
    };
    run(&mut doc, add(None));
    run(&mut doc, add(Some("Clap")));
    let channels = &doc.project().channels;
    assert_eq!(channels[0].name, "snare");
    assert_eq!(channels[1].name, "Clap");
    assert_eq!(sampler(&doc, channels[0].id).sample, Some(sample));
    assert_eq!(doc.project().mixer.tracks[2].name, "Clap");
}

#[test]
fn add_channel_on_a_given_track_makes_no_track() {
    let mut doc = document();
    let insert = add_mixer_track(&mut doc);
    let applied = run(
        &mut doc,
        Command::AddChannel {
            name: None,
            sample: None,
            index: None,
            mixer_track: Some(insert),
        },
    );
    assert_eq!(applied.created.len(), 1);
    assert_eq!(applied.touched, touched(|t| t.channels = true));
    assert_eq!(doc.project().channels[0].mixer_track, insert);
    assert_eq!(doc.project().mixer.tracks.len(), 2);
}

#[test]
fn add_channel_rejects_unknown_references() {
    let mut doc = document();
    let error = fail(
        &mut doc,
        Command::AddChannel {
            name: None,
            sample: Some(SampleId(50)),
            index: None,
            mixer_track: None,
        },
    );
    assert_eq!(error, not_found("sample", 50));
    assert_eq!(error.to_string(), "sample 50 does not exist");
    let error = fail(
        &mut doc,
        Command::AddChannel {
            name: None,
            sample: None,
            index: None,
            mixer_track: Some(TrackId(51)),
        },
    );
    assert_eq!(error, not_found("mixer track", 51));
}

#[test]
fn add_channel_places_the_channel_at_the_index() {
    let mut doc = document();
    let (a, _) = add_channel(&mut doc, "A");
    let (b, _) = add_channel(&mut doc, "B");
    let at = |index| Command::AddChannel {
        name: None,
        sample: None,
        index: Some(index),
        mixer_track: Some(TrackId::MASTER),
    };
    let first = ChannelId(run(&mut doc, at(0)).created[0]);
    let middle = ChannelId(run(&mut doc, at(2)).created[0]);
    let past_end = ChannelId(run(&mut doc, at(99)).created[0]);
    let order: Vec<ChannelId> = doc.project().channels.iter().map(|c| c.id).collect();
    assert_eq!(order, [first, a, middle, b, past_end]);
}

#[test]
fn new_channels_cycle_through_the_palette() {
    let mut doc = document();
    for _ in 0..PALETTE.len() + 1 {
        run(
            &mut doc,
            Command::AddChannel {
                name: None,
                sample: None,
                index: None,
                mixer_track: Some(TrackId::MASTER),
            },
        );
    }
    let colors: Vec<u32> = doc.project().channels.iter().map(|c| c.color).collect();
    assert_eq!(colors[..PALETTE.len()], PALETTE);
    assert_eq!(colors[PALETTE.len()], PALETTE[0]);
}

#[test]
fn add_channel_plays_into_the_master_when_the_mixer_is_full() {
    let mut doc = document();
    for _ in 1..MAX_MIXER_TRACKS {
        doc.dispatch(Command::AddMixerTrack { name: None }, None)
            .unwrap();
    }
    let applied = run(
        &mut doc,
        Command::AddChannel {
            name: None,
            sample: None,
            index: None,
            mixer_track: None,
        },
    );
    assert_eq!(applied.created.len(), 1);
    assert_eq!(doc.project().channels[0].mixer_track, TrackId::MASTER);
    assert_eq!(doc.project().mixer.tracks.len(), MAX_MIXER_TRACKS);
}

#[test]
fn remove_channel_takes_its_notes_and_leaves_its_track() {
    let mut doc = document();
    let (kick, kick_track) = add_channel(&mut doc, "Kick");
    let (hat, _) = add_channel(&mut doc, "Hat");
    let second = add_pattern(&mut doc);
    let third = add_pattern(&mut doc);
    add_notes(&mut doc, kick, vec![note(0, 60), note(960, 60)]);
    add_notes(&mut doc, hat, vec![note(480, 60)]);
    run(
        &mut doc,
        Command::ToggleStep {
            pattern: second,
            channel: kick,
            step: 3,
        },
    );

    let applied = run(&mut doc, Command::RemoveChannel { id: kick });
    assert_eq!(applied.label, "Delete channel");
    assert_eq!(
        applied.touched,
        touched(|t| {
            t.channels = true;
            t.patterns = vec![PATTERN, second];
        })
    );
    assert!(doc.project().channel(kick).is_none());
    assert!(doc.project().mixer.track(kick_track).is_some());
    for pattern in [PATTERN, second, third] {
        assert!(lane_notes(&doc, pattern, kick).is_empty());
    }
    assert_eq!(lane_notes(&doc, PATTERN, hat).len(), 1);

    assert_eq!(
        fail(&mut doc, Command::RemoveChannel { id: kick }),
        not_found("channel", kick.0)
    );
}

#[test]
fn duplicate_channel_copies_the_channel_and_its_notes() {
    let mut doc = document();
    let (kick, kick_track) = add_channel(&mut doc, "Kick");
    let (hat, _) = add_channel(&mut doc, "Hat");
    let second = add_pattern(&mut doc);
    add_notes(
        &mut doc,
        kick,
        vec![note(0, 60), note(0, 60), note(960, 64)],
    );
    run(
        &mut doc,
        Command::ToggleStep {
            pattern: second,
            channel: kick,
            step: 2,
        },
    );
    run(
        &mut doc,
        Command::UpdateChannel {
            id: kick,
            patch: ChannelPatch {
                volume: Some(0.5),
                muted: Some(true),
                ..ChannelPatch::default()
            },
        },
    );
    let tracks_before = doc.project().mixer.tracks.len();

    let applied = run(&mut doc, Command::DuplicateChannel { id: kick });
    assert_eq!(applied.label, "Duplicate channel");
    assert_eq!(
        applied.touched,
        touched(|t| {
            t.channels = true;
            t.patterns = vec![PATTERN, second];
        })
    );
    let [copy_id] = applied.created[..] else {
        panic!("expected one created id");
    };
    let copy_id = ChannelId(copy_id);

    let order: Vec<ChannelId> = doc.project().channels.iter().map(|c| c.id).collect();
    assert_eq!(order, [kick, copy_id, hat]);
    let original = doc.project().channel(kick).unwrap();
    let copy = doc.project().channel(copy_id).unwrap();
    assert_eq!(copy.name, "Kick #2");
    assert_eq!(copy.mixer_track, kick_track);
    assert_eq!(copy.volume, 0.5);
    assert!(copy.muted);
    assert_eq!(copy.source, original.source);
    assert_eq!(doc.project().mixer.tracks.len(), tracks_before);

    for pattern in [PATTERN, second] {
        let originals = lane_notes(&doc, pattern, kick);
        let copies = lane_notes(&doc, pattern, copy_id);
        assert_eq!(originals.len(), copies.len());
        for (original, copy) in originals.iter().zip(&copies) {
            assert_ne!(original.id, copy.id);
            assert_eq!(
                Note {
                    id: original.id,
                    ..*copy
                },
                *original
            );
        }
    }

    let again = run(&mut doc, Command::DuplicateChannel { id: copy_id });
    let third = doc.project().channel(ChannelId(again.created[0])).unwrap();
    assert_eq!(third.name, "Kick #3");

    assert_eq!(
        fail(&mut doc, Command::DuplicateChannel { id: ChannelId(77) }),
        not_found("channel", 77)
    );
}

#[test]
fn move_channel_reorders_the_rack() {
    let mut doc = document();
    let (a, _) = add_channel(&mut doc, "A");
    let (b, _) = add_channel(&mut doc, "B");
    let (c, _) = add_channel(&mut doc, "C");
    let order = |doc: &Document| -> Vec<ChannelId> {
        doc.project().channels.iter().map(|c| c.id).collect()
    };

    let applied = run(&mut doc, Command::MoveChannel { id: a, index: 2 });
    assert_eq!(applied.touched, touched(|t| t.channels = true));
    assert_eq!(applied.label, "Move channel");
    assert_eq!(order(&doc), [b, c, a]);

    run(&mut doc, Command::MoveChannel { id: c, index: 0 });
    assert_eq!(order(&doc), [c, b, a]);

    // An index past the end means the end.
    run(&mut doc, Command::MoveChannel { id: c, index: 50 });
    assert_eq!(order(&doc), [b, a, c]);

    let entries = doc.history().entries.len();
    let applied = run(&mut doc, Command::MoveChannel { id: c, index: 2 });
    assert!(applied.touched.is_empty());
    assert_eq!(doc.history().entries.len(), entries);

    assert_eq!(
        fail(
            &mut doc,
            Command::MoveChannel {
                id: ChannelId(77),
                index: 0
            }
        ),
        not_found("channel", 77)
    );
}

fn update_channel(id: ChannelId, patch: ChannelPatch) -> Command {
    Command::UpdateChannel { id, patch }
}

#[test]
fn update_channel_changes_each_field() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    let insert = add_mixer_track(&mut doc);
    let applied = run(
        &mut doc,
        update_channel(
            kick,
            ChannelPatch {
                name: Some("Boom".to_owned()),
                color: Some(0x123456),
                volume: Some(1.5),
                pan: Some(-0.5),
                muted: Some(true),
                solo: Some(true),
                mixer_track: Some(insert),
            },
        ),
    );
    assert_eq!(applied.touched, touched(|t| t.channels = true));
    assert_eq!(applied.label, "Change channel");
    let channel = doc.project().channel(kick).unwrap();
    assert_eq!(channel.name, "Boom");
    assert_eq!(channel.color, 0x123456);
    assert_eq!(channel.volume, 1.5);
    assert_eq!(channel.pan, -0.5);
    assert!(channel.muted && channel.solo);
    assert_eq!(channel.mixer_track, insert);
}

#[test]
fn update_channel_labels_say_what_changed() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    let cases = [
        (
            ChannelPatch {
                name: Some("K".to_owned()),
                ..ChannelPatch::default()
            },
            "Rename channel",
        ),
        (
            ChannelPatch {
                color: Some(1),
                ..ChannelPatch::default()
            },
            "Change channel color",
        ),
        (
            ChannelPatch {
                volume: Some(0.1),
                ..ChannelPatch::default()
            },
            "Change channel volume",
        ),
        (
            ChannelPatch {
                pan: Some(0.1),
                ..ChannelPatch::default()
            },
            "Change channel pan",
        ),
        (
            ChannelPatch {
                muted: Some(true),
                ..ChannelPatch::default()
            },
            "Mute channel",
        ),
        (
            ChannelPatch {
                muted: Some(false),
                ..ChannelPatch::default()
            },
            "Unmute channel",
        ),
        (
            ChannelPatch {
                solo: Some(true),
                ..ChannelPatch::default()
            },
            "Solo channel",
        ),
        (
            ChannelPatch {
                solo: Some(false),
                ..ChannelPatch::default()
            },
            "Unsolo channel",
        ),
        (
            ChannelPatch {
                mixer_track: Some(TrackId::MASTER),
                ..ChannelPatch::default()
            },
            "Route channel",
        ),
    ];
    for (patch, expected) in cases {
        assert_eq!(run(&mut doc, update_channel(kick, patch)).label, expected);
        assert_eq!(label(&doc), expected);
    }
}

#[test]
fn update_channel_validates_its_values() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    let volume = |volume| {
        update_channel(
            kick,
            ChannelPatch {
                volume: Some(volume),
                ..ChannelPatch::default()
            },
        )
    };
    let pan = |pan| {
        update_channel(
            kick,
            ChannelPatch {
                pan: Some(pan),
                ..ChannelPatch::default()
            },
        )
    };
    run(&mut doc, volume(9.0));
    assert_eq!(doc.project().channel(kick).unwrap().volume, MAX_GAIN);
    run(&mut doc, volume(-1.0));
    assert_eq!(doc.project().channel(kick).unwrap().volume, 0.0);
    run(&mut doc, pan(-7.0));
    assert_eq!(doc.project().channel(kick).unwrap().pan, -1.0);
    run(&mut doc, pan(f32::INFINITY));
    assert_eq!(doc.project().channel(kick).unwrap().pan, 1.0);
    assert_invalid(fail(&mut doc, volume(f32::NAN)), "not a number");
    assert_invalid(fail(&mut doc, pan(f32::NAN)), "not a number");

    let color = update_channel(
        kick,
        ChannelPatch {
            color: Some(0x1_000_000),
            ..ChannelPatch::default()
        },
    );
    assert_invalid(fail(&mut doc, color), "0xRRGGBB");

    let route = update_channel(
        kick,
        ChannelPatch {
            name: Some("Renamed".to_owned()),
            mixer_track: Some(TrackId(88)),
            ..ChannelPatch::default()
        },
    );
    assert_eq!(fail(&mut doc, route), not_found("mixer track", 88));
    assert_eq!(
        fail(
            &mut doc,
            update_channel(ChannelId(77), ChannelPatch::default())
        ),
        not_found("channel", 77)
    );
}

#[test]
fn negative_zero_is_stored_as_zero() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    run(
        &mut doc,
        update_channel(
            kick,
            ChannelPatch {
                pan: Some(0.5),
                ..ChannelPatch::default()
            },
        ),
    );
    run(
        &mut doc,
        update_channel(
            kick,
            ChannelPatch {
                pan: Some(-0.0),
                ..ChannelPatch::default()
            },
        ),
    );
    assert!(doc.project().channel(kick).unwrap().pan.is_sign_positive());
}

#[test]
fn set_channel_sample_sets_and_clears() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    let sample = add_sample(&mut doc, "kick");
    let set = |id, sample| Command::SetChannelSample { id, sample };

    let applied = run(&mut doc, set(kick, Some(sample)));
    assert_eq!(applied.touched, touched(|t| t.channels = true));
    assert_eq!(applied.label, "Change channel sample");
    assert_eq!(sampler(&doc, kick).sample, Some(sample));
    // The channel keeps the name it was given.
    assert_eq!(doc.project().channel(kick).unwrap().name, "Kick");

    run(&mut doc, set(kick, None));
    assert_eq!(sampler(&doc, kick).sample, None);

    assert_eq!(
        fail(&mut doc, set(kick, Some(SampleId(66)))),
        not_found("sample", 66)
    );
    assert_eq!(
        fail(&mut doc, set(ChannelId(77), Some(sample))),
        not_found("channel", 77)
    );
}

fn update_sampler(id: ChannelId, patch: SamplerPatch) -> Command {
    Command::UpdateSampler { id, patch }
}

#[test]
fn update_sampler_changes_each_field() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    let applied = run(
        &mut doc,
        update_sampler(
            kick,
            SamplerPatch {
                root_key: Some(48),
                tune: Some(-12.5),
                gain: Some(1.5),
                start: Some(0.25),
                end: Some(0.75),
                reverse: Some(true),
                cut_self: Some(true),
                cut_group: Some(3),
            },
        ),
    );
    assert_eq!(applied.touched, touched(|t| t.channels = true));
    assert_eq!(applied.label, "Change sampler");
    assert_eq!(
        sampler(&doc, kick),
        SamplerSettings {
            sample: None,
            root_key: 48,
            tune: -12.5,
            gain: 1.5,
            start: 0.25,
            end: 0.75,
            reverse: true,
            envelope: None,
            cut_self: true,
            cut_group: 3,
        }
    );
}

#[test]
fn update_sampler_validates_its_values() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");

    let applied = run(
        &mut doc,
        update_sampler(
            kick,
            SamplerPatch {
                tune: Some(100.0),
                ..SamplerPatch::default()
            },
        ),
    );
    assert_eq!(applied.label, "Change tuning");
    assert_eq!(sampler(&doc, kick).tune, MAX_TUNE_SEMITONES);
    run(
        &mut doc,
        update_sampler(
            kick,
            SamplerPatch {
                tune: Some(-100.0),
                gain: Some(50.0),
                ..SamplerPatch::default()
            },
        ),
    );
    assert_eq!(sampler(&doc, kick).tune, -MAX_TUNE_SEMITONES);
    assert_eq!(sampler(&doc, kick).gain, MAX_GAIN);

    let root = update_sampler(
        kick,
        SamplerPatch {
            root_key: Some(128),
            ..SamplerPatch::default()
        },
    );
    assert_invalid(fail(&mut doc, root), "key 128");
    let nan = update_sampler(
        kick,
        SamplerPatch {
            gain: Some(f32::NAN),
            ..SamplerPatch::default()
        },
    );
    assert_invalid(fail(&mut doc, nan), "not a number");
    assert_eq!(
        fail(
            &mut doc,
            update_sampler(ChannelId(77), SamplerPatch::default())
        ),
        not_found("channel", 77)
    );
}

#[test]
fn the_sample_start_must_come_before_its_end() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    let range = |start, end| {
        update_sampler(
            kick,
            SamplerPatch {
                start,
                end,
                ..SamplerPatch::default()
            },
        )
    };
    assert_invalid(
        fail(&mut doc, range(Some(0.5), Some(0.5))),
        "before its end",
    );
    assert_invalid(
        fail(&mut doc, range(Some(0.8), Some(0.2))),
        "before its end",
    );
    // Out-of-range values are clamped first: a start of 2 becomes 1, the end.
    assert_invalid(fail(&mut doc, range(Some(2.0), None)), "before its end");
    assert_invalid(fail(&mut doc, range(None, Some(-1.0))), "before its end");

    let applied = run(&mut doc, range(Some(-4.0), Some(4.0)));
    assert!(applied.touched.is_empty(), "0 to 1 is the default range");
    run(&mut doc, range(Some(0.4), None));
    assert_eq!(
        run(&mut doc, range(None, Some(0.6))).label,
        "Change sample range"
    );
    assert_eq!(sampler(&doc, kick).start, 0.4);
    assert_eq!(sampler(&doc, kick).end, 0.6);
    assert_invalid(fail(&mut doc, range(None, Some(0.3))), "before its end");
}

#[test]
fn set_sampler_envelope_turns_it_on_and_off() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    let set = |id, envelope| Command::SetSamplerEnvelope { id, envelope };

    let applied = run(&mut doc, set(kick, Some(Envelope::default())));
    assert_eq!(applied.touched, touched(|t| t.channels = true));
    assert_eq!(applied.label, "Change envelope");
    assert_eq!(sampler(&doc, kick).envelope, Some(Envelope::default()));

    run(
        &mut doc,
        set(
            kick,
            Some(Envelope {
                attack_ms: -5.0,
                decay_ms: f32::INFINITY,
                sustain: 4.0,
                release_ms: 30.0,
            }),
        ),
    );
    assert_eq!(
        sampler(&doc, kick).envelope,
        Some(Envelope {
            attack_ms: 0.0,
            decay_ms: MAX_ENVELOPE_MS,
            sustain: 1.0,
            release_ms: 30.0,
        })
    );

    let nan = Envelope {
        release_ms: f32::NAN,
        ..Envelope::default()
    };
    assert_invalid(fail(&mut doc, set(kick, Some(nan))), "not a number");

    run(&mut doc, set(kick, None));
    assert_eq!(sampler(&doc, kick).envelope, None);
    assert_eq!(
        fail(&mut doc, set(ChannelId(77), None)),
        not_found("channel", 77)
    );
}

// Patterns

#[test]
fn add_pattern_appends_an_empty_pattern() {
    let mut doc = document();
    let applied = run(&mut doc, Command::AddPattern { name: None });
    let id = PatternId(applied.created[0]);
    assert_eq!(applied.label, "Add pattern");
    assert_eq!(
        applied.touched,
        touched(|t| {
            t.pattern_list = true;
            t.patterns = vec![id];
        })
    );
    let pattern = doc.project().patterns.last().unwrap();
    assert_eq!(pattern.id, id);
    assert_eq!(pattern.name, "Pattern 2");
    assert_eq!(pattern.color, PALETTE[1]);
    assert_eq!(pattern.length_steps, DEFAULT_PATTERN_STEPS);
    assert!(pattern.lanes.is_empty());

    run(
        &mut doc,
        Command::AddPattern {
            name: Some("Pattern 4".to_owned()),
        },
    );
    // "Pattern 4" is taken, so the fourth pattern is numbered past it.
    let applied = run(&mut doc, Command::AddPattern { name: None });
    let pattern = doc
        .project()
        .pattern(PatternId(applied.created[0]))
        .unwrap();
    assert_eq!(pattern.name, "Pattern 5");
}

#[test]
fn remove_pattern_takes_its_clips() {
    let mut doc = document();
    let second = add_pattern(&mut doc);
    let lane = add_playlist_track(&mut doc);
    let kept = add_clip(&mut doc, lane, PATTERN, 0);
    add_clip(&mut doc, lane, second, 3840);
    add_clip(&mut doc, lane, second, 7680);

    let applied = run(&mut doc, Command::RemovePattern { id: second });
    assert_eq!(applied.label, "Delete pattern");
    assert_eq!(
        applied.touched,
        touched(|t| {
            t.pattern_list = true;
            t.playlist = true;
        })
    );
    assert!(doc.project().pattern(second).is_none());
    let clips: Vec<ClipId> = doc.project().playlist.clips.iter().map(|c| c.id).collect();
    assert_eq!(clips, [kept]);
}

#[test]
fn remove_pattern_fails_on_the_last_pattern() {
    let mut doc = document();
    assert_invalid(
        fail(&mut doc, Command::RemovePattern { id: PATTERN }),
        "at least one pattern",
    );
    assert_eq!(
        fail(&mut doc, Command::RemovePattern { id: PatternId(90) }),
        not_found("pattern", 90)
    );
    let second = add_pattern(&mut doc);
    let applied = run(&mut doc, Command::RemovePattern { id: PATTERN });
    assert_eq!(applied.touched, touched(|t| t.pattern_list = true));
    assert_eq!(doc.project().patterns[0].id, second);
}

#[test]
fn duplicate_pattern_copies_notes_under_new_ids() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    let (hat, _) = add_channel(&mut doc, "Hat");
    let last = add_pattern(&mut doc);
    add_notes(
        &mut doc,
        kick,
        vec![note(0, 60), note(0, 60), note(480, 62)],
    );
    add_notes(&mut doc, hat, vec![note(240, 60)]);
    run(
        &mut doc,
        Command::UpdatePattern {
            id: PATTERN,
            patch: PatternPatch {
                length_steps: Some(32),
                color: Some(0xABCDEF),
                ..PatternPatch::default()
            },
        },
    );

    let applied = run(&mut doc, Command::DuplicatePattern { id: PATTERN });
    let [copy_id] = applied.created[..] else {
        panic!("expected one created id");
    };
    let copy_id = PatternId(copy_id);
    assert_eq!(applied.label, "Duplicate pattern");
    assert_eq!(
        applied.touched,
        touched(|t| {
            t.pattern_list = true;
            t.patterns = vec![copy_id];
        })
    );
    let order: Vec<PatternId> = doc.project().patterns.iter().map(|p| p.id).collect();
    assert_eq!(order, [PATTERN, copy_id, last]);

    let original = doc.project().pattern(PATTERN).unwrap();
    let copy = doc.project().pattern(copy_id).unwrap();
    assert_eq!(copy.name, "Pattern 1 #2");
    assert_eq!(copy.length_steps, 32);
    assert_eq!(copy.color, 0xABCDEF);
    assert_eq!(copy.lanes.len(), original.lanes.len());
    for (original, copy) in original.lanes.iter().zip(&copy.lanes) {
        assert_eq!(original.channel, copy.channel);
        assert_eq!(original.notes.len(), copy.notes.len());
        for (original, copy) in original.notes.iter().zip(&copy.notes) {
            assert_ne!(original.id, copy.id);
            assert_eq!((original.start, original.key), (copy.start, copy.key));
        }
    }

    assert_eq!(
        fail(&mut doc, Command::DuplicatePattern { id: PatternId(90) }),
        not_found("pattern", 90)
    );
}

#[test]
fn move_pattern_reorders_the_list() {
    let mut doc = document();
    let second = add_pattern(&mut doc);
    let third = add_pattern(&mut doc);
    let order = |doc: &Document| -> Vec<PatternId> {
        doc.project().patterns.iter().map(|p| p.id).collect()
    };

    let applied = run(
        &mut doc,
        Command::MovePattern {
            id: third,
            index: 0,
        },
    );
    assert_eq!(applied.touched, touched(|t| t.pattern_list = true));
    assert_eq!(applied.label, "Move pattern");
    assert_eq!(order(&doc), [third, PATTERN, second]);

    run(
        &mut doc,
        Command::MovePattern {
            id: third,
            index: 99,
        },
    );
    assert_eq!(order(&doc), [PATTERN, second, third]);

    let same = run(
        &mut doc,
        Command::MovePattern {
            id: second,
            index: 1,
        },
    );
    assert!(same.touched.is_empty());
    assert_eq!(
        fail(
            &mut doc,
            Command::MovePattern {
                id: PatternId(90),
                index: 0
            }
        ),
        not_found("pattern", 90)
    );
}

#[test]
fn update_pattern_changes_each_field() {
    let mut doc = document();
    let update = |id, patch| Command::UpdatePattern { id, patch };
    let applied = run(
        &mut doc,
        update(
            PATTERN,
            PatternPatch {
                name: Some("Intro".to_owned()),
                color: Some(0x00FF00),
                length_steps: Some(64),
            },
        ),
    );
    assert_eq!(applied.touched, touched(|t| t.patterns = vec![PATTERN]));
    assert_eq!(applied.label, "Change pattern");
    let pattern = doc.project().pattern(PATTERN).unwrap();
    assert_eq!(pattern.name, "Intro");
    assert_eq!(pattern.color, 0x00FF00);
    assert_eq!(pattern.length_steps, 64);
    assert_eq!(pattern.length_ticks(), 64 * 240);

    let length = |length_steps| {
        update(
            PATTERN,
            PatternPatch {
                length_steps: Some(length_steps),
                ..PatternPatch::default()
            },
        )
    };
    assert_eq!(run(&mut doc, length(0)).label, "Change pattern length");
    assert_eq!(doc.project().pattern(PATTERN).unwrap().length_steps, 1);
    run(&mut doc, length(50_000));
    assert_eq!(
        doc.project().pattern(PATTERN).unwrap().length_steps,
        MAX_PATTERN_STEPS
    );

    let color = update(
        PATTERN,
        PatternPatch {
            color: Some(u32::MAX),
            ..PatternPatch::default()
        },
    );
    assert_invalid(fail(&mut doc, color), "0xRRGGBB");
    assert_eq!(
        fail(&mut doc, update(PatternId(90), PatternPatch::default())),
        not_found("pattern", 90)
    );
}

// Notes

fn toggle(channel: ChannelId, step: u32) -> Command {
    Command::ToggleStep {
        pattern: PATTERN,
        channel,
        step,
    }
}

#[test]
fn toggle_step_adds_then_removes_a_note() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");

    let applied = run(&mut doc, toggle(kick, 4));
    assert_eq!(applied.label, "Toggle step");
    assert_eq!(applied.touched, touched(|t| t.patterns = vec![PATTERN]));
    let [id] = applied.created[..] else {
        panic!("expected one created id");
    };
    assert_eq!(
        lane_notes(&doc, PATTERN, kick),
        [Note {
            id: NoteId(id),
            start: 4 * TICKS_PER_STEP,
            length: TICKS_PER_STEP,
            key: DEFAULT_KEY,
            velocity: DEFAULT_VELOCITY,
            pan: 0.0,
        }]
    );

    let applied = run(&mut doc, toggle(kick, 4));
    assert!(applied.created.is_empty());
    assert_eq!(applied.touched, touched(|t| t.patterns = vec![PATTERN]));
    assert!(doc.project().pattern(PATTERN).unwrap().lanes.is_empty());
}

#[test]
fn toggle_step_removes_every_note_that_starts_on_the_step() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    let ids = add_notes(
        &mut doc,
        kick,
        vec![note(480, 60), note(480, 72), note(481, 60), note(0, 60)],
    );
    run(&mut doc, toggle(kick, 2));
    let left: Vec<NoteId> = lane_notes(&doc, PATTERN, kick)
        .iter()
        .map(|n| n.id)
        .collect();
    assert_eq!(left, [ids[3], ids[2]]);
}

#[test]
fn toggle_step_rejects_unknown_ids_and_impossible_steps() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    assert_eq!(
        fail(&mut doc, toggle(ChannelId(77), 0)),
        not_found("channel", 77)
    );
    let unknown_pattern = Command::ToggleStep {
        pattern: PatternId(90),
        channel: kick,
        step: 0,
    };
    assert_eq!(fail(&mut doc, unknown_pattern), not_found("pattern", 90));
    assert_invalid(fail(&mut doc, toggle(kick, u32::MAX)), "past the last tick");
    assert_invalid(
        fail(&mut doc, toggle(kick, u32::MAX / TICKS_PER_STEP)),
        "past the last tick",
    );
}

#[test]
fn add_notes_creates_ids_in_order_and_keeps_the_lane_sorted() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    let applied = run(
        &mut doc,
        Command::AddNotes {
            pattern: PATTERN,
            channel: kick,
            notes: vec![
                note(960, 60),
                note(0, 64),
                NoteInit {
                    start: 0,
                    length: 100,
                    key: 60,
                    velocity: Some(0.5),
                    pan: Some(-0.25),
                },
                note(0, 60),
            ],
        },
    );
    assert_eq!(applied.label, "Add notes");
    assert_eq!(applied.touched, touched(|t| t.patterns = vec![PATTERN]));
    let ids = &applied.created;
    assert_eq!(ids.len(), 4);
    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));

    let notes = lane_notes(&doc, PATTERN, kick);
    let order: Vec<u32> = notes.iter().map(|n| n.id.0).collect();
    // By start, then key, then id.
    assert_eq!(order, [ids[2], ids[3], ids[1], ids[0]]);
    assert_eq!(notes[0].length, 100);
    assert_eq!(notes[0].velocity, 0.5);
    assert_eq!(notes[0].pan, -0.25);
    assert_eq!(notes[1].velocity, DEFAULT_VELOCITY);
    assert_eq!(notes[1].pan, 0.0);

    let one = run(
        &mut doc,
        Command::AddNotes {
            pattern: PATTERN,
            channel: kick,
            notes: vec![note(5, 5)],
        },
    );
    assert_eq!(one.label, "Add note");
}

#[test]
fn add_notes_validates_every_note_before_adding_any() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    let add = |notes| Command::AddNotes {
        pattern: PATTERN,
        channel: kick,
        notes,
    };
    let with = |change: fn(&mut NoteInit)| {
        let mut bad = note(0, 60);
        change(&mut bad);
        vec![note(0, 60), bad]
    };
    assert_invalid(
        fail(&mut doc, add(with(|n| n.length = 0))),
        "at least one tick long",
    );
    assert_invalid(fail(&mut doc, add(with(|n| n.key = 128))), "key 128");
    assert_invalid(
        fail(&mut doc, add(with(|n| n.start = u32::MAX))),
        "past the last tick",
    );
    assert_invalid(
        fail(&mut doc, add(with(|n| n.velocity = Some(f32::NAN)))),
        "not a number",
    );
    assert_invalid(
        fail(&mut doc, add(with(|n| n.pan = Some(f32::NAN)))),
        "not a number",
    );

    run(
        &mut doc,
        add(with(|n| {
            n.velocity = Some(7.0);
            n.pan = Some(-7.0);
            n.key = 127;
        })),
    );
    let notes = lane_notes(&doc, PATTERN, kick);
    assert_eq!(notes[1].velocity, 1.0);
    assert_eq!(notes[1].pan, -1.0);

    let empty = run(&mut doc, add(Vec::new()));
    assert!(empty.touched.is_empty() && empty.created.is_empty());

    let unknown_channel = Command::AddNotes {
        pattern: PATTERN,
        channel: ChannelId(77),
        notes: vec![note(0, 60)],
    };
    assert_eq!(fail(&mut doc, unknown_channel), not_found("channel", 77));
    let unknown_pattern = Command::AddNotes {
        pattern: PatternId(90),
        channel: kick,
        notes: vec![note(0, 60)],
    };
    assert_eq!(fail(&mut doc, unknown_pattern), not_found("pattern", 90));
}

#[test]
fn lanes_are_kept_in_channel_order() {
    let mut doc = document();
    let (a, _) = add_channel(&mut doc, "A");
    let (b, _) = add_channel(&mut doc, "B");
    let (c, _) = add_channel(&mut doc, "C");
    for channel in [c, a, b] {
        add_notes(&mut doc, channel, vec![note(0, 60)]);
    }
    let lanes = &doc.project().pattern(PATTERN).unwrap().lanes;
    let order: Vec<ChannelId> = lanes.iter().map(|lane| lane.channel).collect();
    assert_eq!(order, [a, b, c]);
}

#[test]
fn remove_notes_removes_them_and_drops_an_empty_lane() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    let ids = add_notes(
        &mut doc,
        kick,
        vec![note(0, 60), note(240, 60), note(480, 60)],
    );
    let remove = |notes| Command::RemoveNotes {
        pattern: PATTERN,
        channel: kick,
        notes,
    };

    let applied = run(&mut doc, remove(vec![ids[1]]));
    assert_eq!(applied.label, "Delete note");
    assert_eq!(applied.touched, touched(|t| t.patterns = vec![PATTERN]));
    let left: Vec<NoteId> = lane_notes(&doc, PATTERN, kick)
        .iter()
        .map(|n| n.id)
        .collect();
    assert_eq!(left, [ids[0], ids[2]]);

    // One unknown id fails the whole command.
    assert_eq!(
        fail(&mut doc, remove(vec![ids[0], ids[1]])),
        not_found("note", ids[1].0)
    );

    // Naming a note twice removes it once.
    assert_eq!(
        run(&mut doc, remove(vec![ids[0], ids[2], ids[0]])).label,
        "Delete notes"
    );
    assert!(doc.project().pattern(PATTERN).unwrap().lanes.is_empty());

    assert_eq!(
        fail(&mut doc, remove(vec![ids[0]])),
        not_found("note", ids[0].0)
    );
    assert!(run(&mut doc, remove(Vec::new())).touched.is_empty());
}

#[test]
fn notes_of_another_lane_are_not_found() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    let (hat, _) = add_channel(&mut doc, "Hat");
    let ids = add_notes(&mut doc, kick, vec![note(0, 60)]);
    add_notes(&mut doc, hat, vec![note(0, 60)]);
    let error = fail(
        &mut doc,
        Command::RemoveNotes {
            pattern: PATTERN,
            channel: hat,
            notes: ids.clone(),
        },
    );
    assert_eq!(error, not_found("note", ids[0].0));
}

fn update_notes(channel: ChannelId, updates: Vec<(NoteId, NotePatch)>) -> Command {
    Command::UpdateNotes {
        pattern: PATTERN,
        channel,
        updates: updates
            .into_iter()
            .map(|(id, patch)| NoteUpdate { id, patch })
            .collect(),
    }
}

#[test]
fn update_notes_changes_notes_and_keeps_the_lane_sorted() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    let ids = add_notes(
        &mut doc,
        kick,
        vec![note(0, 60), note(240, 60), note(480, 60)],
    );

    let applied = run(
        &mut doc,
        update_notes(
            kick,
            vec![
                (
                    ids[0],
                    NotePatch {
                        start: Some(960),
                        key: Some(72),
                        ..NotePatch::default()
                    },
                ),
                (
                    ids[2],
                    NotePatch {
                        length: Some(10),
                        velocity: Some(2.0),
                        pan: Some(0.5),
                        ..NotePatch::default()
                    },
                ),
            ],
        ),
    );
    assert_eq!(applied.label, "Move notes");
    assert_eq!(applied.touched, touched(|t| t.patterns = vec![PATTERN]));
    assert!(applied.created.is_empty());
    let notes = lane_notes(&doc, PATTERN, kick);
    let order: Vec<NoteId> = notes.iter().map(|n| n.id).collect();
    assert_eq!(order, [ids[1], ids[2], ids[0]]);
    assert_eq!((notes[2].start, notes[2].key), (960, 72));
    assert_eq!(notes[1].length, 10);
    assert_eq!(notes[1].velocity, 1.0);
    assert_eq!(notes[1].pan, 0.5);
}

#[test]
fn update_notes_labels_say_what_changed() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    let ids = add_notes(&mut doc, kick, vec![note(0, 60), note(240, 60)]);
    let cases = [
        (
            NotePatch {
                start: Some(7),
                ..NotePatch::default()
            },
            "Move note",
        ),
        (
            NotePatch {
                key: Some(7),
                ..NotePatch::default()
            },
            "Move note",
        ),
        (
            NotePatch {
                length: Some(7),
                ..NotePatch::default()
            },
            "Resize note",
        ),
        (
            NotePatch {
                velocity: Some(0.7),
                ..NotePatch::default()
            },
            "Change note velocity",
        ),
        (
            NotePatch {
                pan: Some(0.7),
                ..NotePatch::default()
            },
            "Change note pan",
        ),
    ];
    for (patch, expected) in cases {
        let applied = run(&mut doc, update_notes(kick, vec![(ids[0], patch)]));
        assert_eq!(applied.label, expected);
    }
    let resize = NotePatch {
        length: Some(99),
        ..NotePatch::default()
    };
    let applied = run(
        &mut doc,
        update_notes(kick, vec![(ids[0], resize), (ids[1], resize)]),
    );
    assert_eq!(applied.label, "Resize notes");
}

#[test]
fn update_notes_validates_before_changing_anything() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    let ids = add_notes(&mut doc, kick, vec![note(0, 60), note(240, 60)]);
    let good = NotePatch {
        start: Some(100),
        ..NotePatch::default()
    };
    let with = |bad: NotePatch| update_notes(kick, vec![(ids[0], good), (ids[1], bad)]);

    let zero = NotePatch {
        length: Some(0),
        ..NotePatch::default()
    };
    assert_invalid(fail(&mut doc, with(zero)), "at least one tick long");
    let high = NotePatch {
        key: Some(200),
        ..NotePatch::default()
    };
    assert_invalid(fail(&mut doc, with(high)), "key 200");
    let far = NotePatch {
        start: Some(u32::MAX),
        ..NotePatch::default()
    };
    assert_invalid(fail(&mut doc, with(far)), "past the last tick");

    let missing = update_notes(kick, vec![(ids[0], good), (NoteId(4000), good)]);
    assert_eq!(fail(&mut doc, missing), not_found("note", 4000));

    // A patch that sets what is already there changes nothing.
    let same = NotePatch {
        start: Some(0),
        key: Some(60),
        ..NotePatch::default()
    };
    assert!(
        run(&mut doc, update_notes(kick, vec![(ids[0], same)]))
            .touched
            .is_empty()
    );

    // Two updates of one note build on each other.
    let longer = NotePatch {
        length: Some(500),
        ..NotePatch::default()
    };
    run(
        &mut doc,
        update_notes(kick, vec![(ids[0], good), (ids[0], longer)]),
    );
    let moved = lane_notes(&doc, PATTERN, kick)
        .into_iter()
        .find(|n| n.id == ids[0])
        .unwrap();
    assert_eq!((moved.start, moved.length), (100, 500));
}

#[test]
fn clear_lane_removes_every_note_of_the_channel() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    let (hat, _) = add_channel(&mut doc, "Hat");
    add_notes(&mut doc, kick, vec![note(0, 60), note(240, 60)]);
    add_notes(&mut doc, hat, vec![note(0, 60)]);
    let clear = |pattern, channel| Command::ClearLane { pattern, channel };

    let applied = run(&mut doc, clear(PATTERN, kick));
    assert_eq!(applied.label, "Clear notes");
    assert_eq!(applied.touched, touched(|t| t.patterns = vec![PATTERN]));
    assert!(lane_notes(&doc, PATTERN, kick).is_empty());
    assert_eq!(lane_notes(&doc, PATTERN, hat).len(), 1);

    assert!(run(&mut doc, clear(PATTERN, kick)).touched.is_empty());
    assert_eq!(
        fail(&mut doc, clear(PATTERN, ChannelId(77))),
        not_found("channel", 77)
    );
    assert_eq!(
        fail(&mut doc, clear(PatternId(90), kick)),
        not_found("pattern", 90)
    );
}

// Mixer

#[test]
fn add_mixer_track_adds_a_track_routed_to_the_master() {
    let mut doc = document();
    let applied = run(&mut doc, Command::AddMixerTrack { name: None });
    assert_eq!(applied.label, "Add mixer track");
    assert_eq!(applied.touched, touched(|t| t.mixer = true));
    let added = track(&doc, TrackId(applied.created[0]));
    assert_eq!(added.name, "Insert 1");
    assert_eq!(added.color, PALETTE[0]);
    assert_eq!(added.volume, 1.0);
    assert_eq!(added.pan, 0.0);
    assert_eq!(added.output, Some(TrackId::MASTER));

    let named = run(
        &mut doc,
        Command::AddMixerTrack {
            name: Some("Insert 3".to_owned()),
        },
    );
    assert_eq!(track(&doc, TrackId(named.created[0])).name, "Insert 3");
    let next = add_mixer_track(&mut doc);
    assert_eq!(track(&doc, next).name, "Insert 4");
}

#[test]
fn the_mixer_holds_a_limited_number_of_tracks() {
    let mut doc = document();
    for _ in 1..MAX_MIXER_TRACKS {
        doc.dispatch(Command::AddMixerTrack { name: None }, None)
            .unwrap();
    }
    doc.project().check().unwrap();
    assert_eq!(doc.project().mixer.tracks.len(), MAX_MIXER_TRACKS);
    assert_invalid(
        fail(&mut doc, Command::AddMixerTrack { name: None }),
        "the mixer is full",
    );
}

#[test]
fn remove_mixer_track_reroutes_to_the_master_and_drops_sends() {
    let mut doc = document();
    let (kick, bus) = add_channel(&mut doc, "Kick");
    let (hat, hat_track) = add_channel(&mut doc, "Hat");
    let feeder = add_mixer_track(&mut doc);
    let sender = add_mixer_track(&mut doc);
    run(
        &mut doc,
        Command::SetTrackOutput {
            id: feeder,
            output: Some(bus),
        },
    );
    for (to, gain) in [(bus, 0.5), (hat_track, 0.25)] {
        run(
            &mut doc,
            Command::SetSend {
                from: sender,
                to,
                gain: Some(gain),
            },
        );
    }

    let applied = run(&mut doc, Command::RemoveMixerTrack { id: bus });
    assert_eq!(applied.label, "Delete mixer track");
    assert_eq!(
        applied.touched,
        touched(|t| {
            t.channels = true;
            t.mixer = true;
        })
    );
    assert!(doc.project().mixer.track(bus).is_none());
    assert_eq!(
        doc.project().channel(kick).unwrap().mixer_track,
        TrackId::MASTER
    );
    assert_eq!(doc.project().channel(hat).unwrap().mixer_track, hat_track);
    assert_eq!(track(&doc, feeder).output, Some(TrackId::MASTER));
    assert_eq!(
        track(&doc, sender).sends,
        [Send {
            target: hat_track,
            gain: 0.25
        }]
    );

    // A track nothing is routed to touches the mixer only.
    let applied = run(&mut doc, Command::RemoveMixerTrack { id: feeder });
    assert_eq!(applied.touched, touched(|t| t.mixer = true));
}

#[test]
fn the_master_track_cannot_be_removed() {
    let mut doc = document();
    assert_invalid(
        fail(
            &mut doc,
            Command::RemoveMixerTrack {
                id: TrackId::MASTER,
            },
        ),
        "master track cannot be deleted",
    );
    assert_eq!(
        fail(&mut doc, Command::RemoveMixerTrack { id: TrackId(88) }),
        not_found("mixer track", 88)
    );
}

#[test]
fn update_mixer_track_changes_each_field() {
    let mut doc = document();
    let insert = add_mixer_track(&mut doc);
    let update = |id, patch| Command::UpdateMixerTrack { id, patch };
    let applied = run(
        &mut doc,
        update(
            insert,
            MixerTrackPatch {
                name: Some("Drums".to_owned()),
                color: Some(0x0000FF),
                volume: Some(0.5),
                pan: Some(0.25),
                muted: Some(true),
                solo: Some(true),
            },
        ),
    );
    assert_eq!(applied.touched, touched(|t| t.mixer = true));
    assert_eq!(applied.label, "Change mixer track");
    let changed = track(&doc, insert);
    assert_eq!(changed.name, "Drums");
    assert_eq!(changed.color, 0x0000FF);
    assert_eq!(changed.volume, 0.5);
    assert_eq!(changed.pan, 0.25);
    assert!(changed.muted && changed.solo);

    let volume = |id, volume| {
        update(
            id,
            MixerTrackPatch {
                volume: Some(volume),
                ..MixerTrackPatch::default()
            },
        )
    };
    let applied = run(&mut doc, volume(TrackId::MASTER, 5.0));
    assert_eq!(applied.label, "Change mixer track volume");
    assert_eq!(track(&doc, TrackId::MASTER).volume, MAX_GAIN);
    assert_invalid(fail(&mut doc, volume(insert, f32::NAN)), "not a number");

    let color = update(
        insert,
        MixerTrackPatch {
            color: Some(0xFF00_0000),
            ..MixerTrackPatch::default()
        },
    );
    assert_invalid(fail(&mut doc, color), "0xRRGGBB");
    assert_eq!(
        fail(&mut doc, update(TrackId(88), MixerTrackPatch::default())),
        not_found("mixer track", 88)
    );
}

fn output(id: TrackId, output: Option<TrackId>) -> Command {
    Command::SetTrackOutput { id, output }
}

fn send(from: TrackId, to: TrackId, gain: Option<f32>) -> Command {
    Command::SetSend { from, to, gain }
}

#[test]
fn set_track_output_routes_a_track() {
    let mut doc = document();
    let a = add_mixer_track(&mut doc);
    let b = add_mixer_track(&mut doc);

    let applied = run(&mut doc, output(a, Some(b)));
    assert_eq!(applied.label, "Route mixer track");
    assert_eq!(applied.touched, touched(|t| t.mixer = true));
    assert_eq!(track(&doc, a).output, Some(b));

    run(&mut doc, output(a, None));
    assert_eq!(track(&doc, a).output, None);

    assert_invalid(
        fail(&mut doc, output(TrackId::MASTER, Some(a))),
        "master track's output",
    );
    assert_invalid(
        fail(&mut doc, output(TrackId::MASTER, None)),
        "master track's output",
    );
    assert_eq!(
        fail(&mut doc, output(a, Some(TrackId(88)))),
        not_found("mixer track", 88)
    );
    assert_eq!(
        fail(&mut doc, output(TrackId(88), None)),
        not_found("mixer track", 88)
    );
}

#[test]
fn routing_cannot_loop() {
    let mut doc = document();
    let a = add_mixer_track(&mut doc);
    let b = add_mixer_track(&mut doc);
    let c = add_mixer_track(&mut doc);
    let looped = "loop back on itself";

    assert_invalid(fail(&mut doc, output(a, Some(a))), looped);
    assert_invalid(fail(&mut doc, send(a, a, Some(1.0))), looped);

    // a -> b -> c -> master.
    run(&mut doc, output(a, Some(b)));
    run(&mut doc, output(b, Some(c)));
    assert_invalid(fail(&mut doc, output(c, Some(a))), looped);
    assert_invalid(fail(&mut doc, output(b, Some(a))), looped);
    assert_invalid(fail(&mut doc, send(c, a, Some(1.0))), looped);
    assert_invalid(fail(&mut doc, send(c, b, Some(1.0))), looped);

    // A loop through a send counts too: c sends to nothing yet, so a send
    // from a to c is fine, and then c may not output to a.
    run(&mut doc, output(a, None));
    run(&mut doc, send(a, c, Some(1.0)));
    assert_invalid(fail(&mut doc, output(c, Some(a))), looped);

    // Routing that merely meets again is not a loop.
    run(&mut doc, send(a, b, Some(1.0)));
    run(&mut doc, send(a, TrackId::MASTER, Some(1.0)));
    run(&mut doc, output(a, Some(c)));
}

#[test]
fn set_send_adds_changes_and_removes_a_send() {
    let mut doc = document();
    let a = add_mixer_track(&mut doc);
    let b = add_mixer_track(&mut doc);

    let applied = run(&mut doc, send(a, b, Some(0.5)));
    assert_eq!(applied.label, "Add send");
    assert_eq!(applied.touched, touched(|t| t.mixer = true));
    assert_eq!(
        track(&doc, a).sends,
        [Send {
            target: b,
            gain: 0.5
        }]
    );

    let applied = run(&mut doc, send(a, b, Some(9.0)));
    assert_eq!(applied.label, "Change send level");
    assert_eq!(
        track(&doc, a).sends,
        [Send {
            target: b,
            gain: MAX_GAIN
        }]
    );

    let applied = run(&mut doc, send(a, b, None));
    assert_eq!(applied.label, "Remove send");
    assert!(track(&doc, a).sends.is_empty());
    assert!(run(&mut doc, send(a, b, None)).touched.is_empty());

    assert_invalid(fail(&mut doc, send(a, b, Some(f32::NAN))), "not a number");
    assert_invalid(
        fail(&mut doc, send(TrackId::MASTER, a, Some(1.0))),
        "master track cannot send",
    );
    assert_eq!(
        fail(&mut doc, send(a, TrackId(88), Some(1.0))),
        not_found("mixer track", 88)
    );
    assert_eq!(
        fail(&mut doc, send(TrackId(88), a, None)),
        not_found("mixer track", 88)
    );
}

// Playlist

#[test]
fn playlist_tracks_are_added_renamed_and_muted() {
    let mut doc = document();
    let applied = run(&mut doc, Command::AddPlaylistTrack { name: None });
    assert_eq!(applied.label, "Add playlist track");
    assert_eq!(applied.touched, touched(|t| t.playlist = true));
    let id = PlaylistTrackId(applied.created[0]);
    assert_eq!(
        doc.project().playlist.tracks,
        [PlaylistTrack {
            id,
            name: "Track 1".to_owned(),
            muted: false,
        }]
    );
    let second = add_playlist_track(&mut doc);
    assert_eq!(doc.project().playlist.tracks[1].name, "Track 2");

    let update = |id, patch| Command::UpdatePlaylistTrack { id, patch };
    let applied = run(
        &mut doc,
        update(
            second,
            PlaylistTrackPatch {
                name: Some("Drums".to_owned()),
                muted: None,
            },
        ),
    );
    assert_eq!(applied.label, "Rename playlist track");
    assert_eq!(applied.touched, touched(|t| t.playlist = true));
    let applied = run(
        &mut doc,
        update(
            second,
            PlaylistTrackPatch {
                name: None,
                muted: Some(true),
            },
        ),
    );
    assert_eq!(applied.label, "Mute playlist track");
    assert_eq!(
        doc.project().playlist.tracks[1],
        PlaylistTrack {
            id: second,
            name: "Drums".to_owned(),
            muted: true,
        }
    );
    assert_eq!(
        fail(
            &mut doc,
            update(PlaylistTrackId(55), PlaylistTrackPatch::default())
        ),
        not_found("playlist track", 55)
    );
}

#[test]
fn remove_playlist_track_takes_its_clips() {
    let mut doc = document();
    let first = add_playlist_track(&mut doc);
    let second = add_playlist_track(&mut doc);
    add_clip(&mut doc, first, PATTERN, 0);
    let kept = add_clip(&mut doc, second, PATTERN, 0);
    add_clip(&mut doc, first, PATTERN, 3840);

    let applied = run(&mut doc, Command::RemovePlaylistTrack { id: first });
    assert_eq!(applied.label, "Delete playlist track");
    assert_eq!(applied.touched, touched(|t| t.playlist = true));
    let clips: Vec<ClipId> = doc.project().playlist.clips.iter().map(|c| c.id).collect();
    assert_eq!(clips, [kept]);
    assert_eq!(doc.project().playlist.tracks.len(), 1);
    assert_eq!(
        fail(&mut doc, Command::RemovePlaylistTrack { id: first }),
        not_found("playlist track", first.0)
    );
}

fn clip(track: PlaylistTrackId, start: u32, length: Option<u32>) -> ClipInit {
    ClipInit {
        track,
        start,
        length,
        content: ClipContent::Pattern { pattern: PATTERN },
    }
}

#[test]
fn add_clips_creates_ids_in_order_and_keeps_clips_sorted() {
    let mut doc = document();
    let lane = add_playlist_track(&mut doc);
    let applied = run(
        &mut doc,
        Command::AddClips {
            clips: vec![
                clip(lane, 7680, None),
                clip(lane, 0, Some(100)),
                clip(lane, 0, None),
            ],
        },
    );
    assert_eq!(applied.label, "Add clips");
    assert_eq!(applied.touched, touched(|t| t.playlist = true));
    let ids = &applied.created;
    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));

    let clips = &doc.project().playlist.clips;
    let order: Vec<u32> = clips.iter().map(|c| c.id.0).collect();
    assert_eq!(order, [ids[1], ids[2], ids[0]]);
    assert_eq!(clips[0].length, 100);
    // One pass of a 16-step pattern.
    assert_eq!(clips[1].length, 16 * TICKS_PER_STEP);
    assert_eq!(clips[1].offset, 0);
    assert!(!clips[1].muted);
    assert_eq!(clips[1].track, lane);
    assert_eq!(clips[1].content, ClipContent::Pattern { pattern: PATTERN });
}

#[test]
fn add_clips_validates_every_clip_before_adding_any() {
    let mut doc = document();
    let lane = add_playlist_track(&mut doc);
    let add = |bad: ClipInit| Command::AddClips {
        clips: vec![clip(lane, 0, None), bad],
    };
    assert_invalid(
        fail(&mut doc, add(clip(lane, 0, Some(0)))),
        "at least one tick long",
    );
    assert_invalid(
        fail(&mut doc, add(clip(lane, u32::MAX, None))),
        "past the last tick",
    );
    assert_eq!(
        fail(&mut doc, add(clip(PlaylistTrackId(55), 0, None))),
        not_found("playlist track", 55)
    );
    let unknown_pattern = ClipInit {
        content: ClipContent::Pattern {
            pattern: PatternId(90),
        },
        ..clip(lane, 0, None)
    };
    assert_eq!(
        fail(&mut doc, add(unknown_pattern)),
        not_found("pattern", 90)
    );
    assert!(
        run(&mut doc, Command::AddClips { clips: Vec::new() })
            .touched
            .is_empty()
    );
}

#[test]
fn remove_clips_removes_them() {
    let mut doc = document();
    let lane = add_playlist_track(&mut doc);
    let a = add_clip(&mut doc, lane, PATTERN, 0);
    let b = add_clip(&mut doc, lane, PATTERN, 3840);
    let remove = |clips| Command::RemoveClips { clips };

    assert_eq!(
        fail(&mut doc, remove(vec![a, ClipId(600)])),
        not_found("clip", 600)
    );
    let applied = run(&mut doc, remove(vec![a]));
    assert_eq!(applied.label, "Delete clip");
    assert_eq!(applied.touched, touched(|t| t.playlist = true));
    let clips: Vec<ClipId> = doc.project().playlist.clips.iter().map(|c| c.id).collect();
    assert_eq!(clips, [b]);
    assert_eq!(fail(&mut doc, remove(vec![a])), not_found("clip", a.0));
}

fn update_clips(updates: Vec<(ClipId, ClipPatch)>) -> Command {
    Command::UpdateClips {
        updates: updates
            .into_iter()
            .map(|(id, patch)| ClipUpdate { id, patch })
            .collect(),
    }
}

#[test]
fn update_clips_changes_clips_and_keeps_them_sorted() {
    let mut doc = document();
    let first = add_playlist_track(&mut doc);
    let second = add_playlist_track(&mut doc);
    let a = add_clip(&mut doc, first, PATTERN, 0);
    let b = add_clip(&mut doc, first, PATTERN, 3840);

    let applied = run(
        &mut doc,
        update_clips(vec![(
            a,
            ClipPatch {
                track: Some(second),
                start: Some(9000),
                length: Some(50),
                offset: Some(7),
                muted: Some(true),
            },
        )]),
    );
    assert_eq!(applied.label, "Move clip");
    assert_eq!(applied.touched, touched(|t| t.playlist = true));
    let clips = &doc.project().playlist.clips;
    let order: Vec<ClipId> = clips.iter().map(|c| c.id).collect();
    assert_eq!(order, [b, a]);
    assert_eq!(
        clips[1],
        Clip {
            id: a,
            track: second,
            start: 9000,
            length: 50,
            offset: 7,
            muted: true,
            content: ClipContent::Pattern { pattern: PATTERN },
        }
    );

    let resize = ClipPatch {
        length: Some(10),
        ..ClipPatch::default()
    };
    let applied = run(
        &mut doc,
        update_clips(vec![(a, resize.clone()), (b, resize)]),
    );
    assert_eq!(applied.label, "Resize clips");
    let unmute = ClipPatch {
        muted: Some(false),
        ..ClipPatch::default()
    };
    assert_eq!(
        run(&mut doc, update_clips(vec![(a, unmute)])).label,
        "Unmute clip"
    );
}

#[test]
fn update_clips_validates_before_changing_anything() {
    let mut doc = document();
    let lane = add_playlist_track(&mut doc);
    let a = add_clip(&mut doc, lane, PATTERN, 0);
    let b = add_clip(&mut doc, lane, PATTERN, 3840);
    let good = ClipPatch {
        start: Some(5),
        ..ClipPatch::default()
    };
    let with = |bad: ClipPatch| update_clips(vec![(a, good.clone()), (b, bad)]);

    let zero = ClipPatch {
        length: Some(0),
        ..ClipPatch::default()
    };
    assert_invalid(fail(&mut doc, with(zero)), "at least one tick long");
    let far = ClipPatch {
        start: Some(u32::MAX),
        ..ClipPatch::default()
    };
    assert_invalid(fail(&mut doc, with(far)), "past the last tick");
    let lost = ClipPatch {
        track: Some(PlaylistTrackId(55)),
        ..ClipPatch::default()
    };
    assert_eq!(fail(&mut doc, with(lost)), not_found("playlist track", 55));
    assert_eq!(
        fail(
            &mut doc,
            update_clips(vec![(a, good.clone()), (ClipId(600), good.clone())])
        ),
        not_found("clip", 600)
    );

    let same = ClipPatch {
        start: Some(0),
        ..ClipPatch::default()
    };
    assert!(
        run(&mut doc, update_clips(vec![(a, same)]))
            .touched
            .is_empty()
    );
}

// Batch

#[test]
fn a_batch_is_one_undo_step() {
    let mut doc = document();
    let before = doc.project().clone();
    let applied = doc
        .dispatch(
            Command::Batch {
                label: None,
                commands: vec![
                    Command::AddSample {
                        name: "Kick".to_owned(),
                        path: SamplePath::Factory("kick.wav".to_owned()),
                    },
                    Command::AddPattern { name: None },
                    tempo(99.0),
                    Command::AddPlaylistTrack { name: None },
                ],
            },
            None,
        )
        .unwrap();
    doc.project().check().unwrap();
    assert_eq!(applied.label, "Add sample");
    assert_eq!(applied.created.len(), 3);
    let pattern = PatternId(applied.created[1]);
    assert_eq!(
        applied.touched,
        touched(|t| {
            t.settings = true;
            t.samples = true;
            t.playlist = true;
            t.pattern_list = true;
            t.patterns = vec![pattern];
        })
    );
    assert_eq!(doc.history().entries.len(), 1);
    assert_eq!(label(&doc), "Add sample");

    doc.undo().unwrap();
    assert!(same_content(doc.project(), &before));
}

#[test]
fn a_batch_that_fails_changes_nothing() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    let next_id = doc.project().next_id;
    let error = fail(
        &mut doc,
        Command::Batch {
            label: Some("Doomed".to_owned()),
            commands: vec![
                Command::AddPattern { name: None },
                toggle(kick, 0),
                Command::AddMixerTrack { name: None },
                Command::RemoveChannel { id: kick },
                // The channel is gone by now, so this fails.
                toggle(kick, 1),
            ],
        },
    );
    assert_eq!(error, not_found("channel", kick.0));
    assert_eq!(doc.project().next_id, next_id);
}

#[test]
fn a_batch_takes_its_label_or_its_first_commands() {
    let mut doc = document();
    let batch = |label: Option<&str>, commands| Command::Batch {
        label: label.map(str::to_owned),
        commands,
    };
    let applied = run(
        &mut doc,
        batch(Some("Set up drums"), vec![tempo(100.0), tempo(101.0)]),
    );
    assert_eq!(applied.label, "Set up drums");
    assert_eq!(label(&doc), "Set up drums");

    let nested = batch(
        None,
        vec![
            batch(None, vec![Command::AddPattern { name: None }]),
            tempo(102.0),
        ],
    );
    assert_eq!(run(&mut doc, nested).label, "Add pattern");

    // A blank label is no label.
    let blank = batch(Some("  "), vec![tempo(103.0)]);
    assert_eq!(run(&mut doc, blank).label, "Change tempo");
    assert_eq!(run(&mut doc, batch(Some(""), Vec::new())).label, "Edit");

    let entries = doc.history().entries.len();
    let empty = run(&mut doc, batch(None, Vec::new()));
    assert!(empty.touched.is_empty());
    assert_eq!(doc.history().entries.len(), entries);
}

#[test]
fn later_commands_of_a_batch_see_what_earlier_ones_did() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    let applied = run(
        &mut doc,
        Command::Batch {
            label: None,
            commands: vec![toggle(kick, 0), toggle(kick, 1), toggle(kick, 0)],
        },
    );
    // Step 0 was turned on and off again inside the batch.
    assert_eq!(applied.created.len(), 2);
    let notes = lane_notes(&doc, PATTERN, kick);
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].start, TICKS_PER_STEP);
    assert_eq!(notes[0].id, NoteId(applied.created[1]));
}

#[test]
fn a_pattern_added_and_removed_in_one_batch_is_not_reported_as_changed() {
    let mut doc = document();
    let next = doc.project().next_id;
    let applied = run(
        &mut doc,
        Command::Batch {
            label: None,
            commands: vec![
                Command::AddPattern { name: None },
                Command::RemovePattern {
                    id: PatternId(next),
                },
            ],
        },
    );
    assert_eq!(applied.created, [next]);
    assert!(applied.touched.patterns.is_empty());
    assert_eq!(doc.project().patterns.len(), 1);
}

// History

#[test]
fn undo_and_redo_walk_the_history() {
    let mut doc = document();
    assert_eq!(doc.undo(), None);
    assert_eq!(doc.redo(), None);

    let (kick, _) = add_channel(&mut doc, "Kick");
    run(&mut doc, toggle(kick, 0));
    run(&mut doc, tempo(90.0));
    let labels: Vec<String> = doc
        .history()
        .entries
        .into_iter()
        .map(|entry| entry.label)
        .collect();
    assert_eq!(labels, ["Add channel", "Toggle step", "Change tempo"]);
    assert_eq!(doc.history().cursor, 3);

    assert_eq!(doc.undo(), Some(touched(|t| t.settings = true)));
    assert_eq!(doc.undo(), Some(touched(|t| t.patterns = vec![PATTERN])));
    assert_eq!(doc.history().cursor, 1);
    assert_eq!(doc.history().entries.len(), 3);
    assert!(lane_notes(&doc, PATTERN, kick).is_empty());

    assert_eq!(doc.redo(), Some(touched(|t| t.patterns = vec![PATTERN])));
    assert_eq!(lane_notes(&doc, PATTERN, kick).len(), 1);
    assert_eq!(doc.redo(), Some(touched(|t| t.settings = true)));
    assert_eq!(doc.redo(), None);
}

#[test]
fn a_new_edit_discards_what_was_undone() {
    let mut doc = document();
    run(&mut doc, tempo(90.0));
    run(&mut doc, tempo(91.0));
    run(&mut doc, tempo(92.0));
    doc.undo().unwrap();
    doc.undo().unwrap();

    // An edit that changes nothing leaves the redo steps alone.
    doc.dispatch(tempo(90.0), None).unwrap();
    assert_eq!(doc.history().entries.len(), 3);

    doc.dispatch(Command::AddPattern { name: None }, None)
        .unwrap();
    let labels: Vec<String> = doc
        .history()
        .entries
        .into_iter()
        .map(|entry| entry.label)
        .collect();
    assert_eq!(labels, ["Change tempo", "Add pattern"]);
    assert_eq!(doc.redo(), None);
    assert_eq!(doc.project().settings.tempo_bpm, 90.0);
}

#[test]
fn jump_moves_to_any_point_of_the_history() {
    let mut doc = document();
    let start = doc.project().clone();
    let (kick, _) = add_channel(&mut doc, "Kick");
    run(&mut doc, toggle(kick, 0));
    let second = add_pattern(&mut doc);
    run(&mut doc, tempo(90.0));
    let end = doc.project().clone();

    let back = doc.jump(0);
    assert!(same_content(doc.project(), &start));
    assert_eq!(doc.history().cursor, 0);
    // The second pattern no longer exists, so only the first is listed.
    assert_eq!(
        back,
        touched(|t| {
            t.settings = true;
            t.channels = true;
            t.mixer = true;
            t.pattern_list = true;
            t.patterns = vec![PATTERN];
        })
    );

    let forward = doc.jump(99);
    assert_eq!(doc.project(), &end);
    assert_eq!(doc.history().cursor, 4);
    assert!(forward.patterns.contains(&second));

    assert!(doc.jump(4).is_empty());
    let partial = doc.jump(2);
    assert_eq!(doc.history().cursor, 2);
    assert_eq!(
        partial,
        touched(|t| {
            t.settings = true;
            t.pattern_list = true;
        })
    );
    assert_eq!(doc.project().settings.tempo_bpm, 120.0);
    assert_eq!(lane_notes(&doc, PATTERN, kick).len(), 1);
}

#[test]
fn undo_keeps_ids_retired() {
    let mut doc = document();
    let (kick, kick_track) = add_channel(&mut doc, "Kick");
    let after_add = doc.project().clone();
    doc.undo().unwrap();
    assert_eq!(doc.project().next_id, after_add.next_id);

    // Redo brings back the very same ids.
    doc.redo().unwrap();
    assert_eq!(doc.project(), &after_add);

    // An edit made after an undo never reuses an undone id.
    doc.undo().unwrap();
    let (snare, snare_track) = add_channel(&mut doc, "Snare");
    for id in [snare.0, snare_track.0] {
        assert!(id != kick.0 && id != kick_track.0);
        assert!(id >= after_add.next_id);
    }
}

// Gestures

fn volume(id: ChannelId, volume: f32) -> Command {
    update_channel(
        id,
        ChannelPatch {
            volume: Some(volume),
            ..ChannelPatch::default()
        },
    )
}

#[test]
fn dispatches_of_one_gesture_collapse_into_one_undo_step() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    let before = doc.project().clone();
    for step in 1..=50 {
        doc.dispatch(volume(kick, step as f32 / 100.0), Some(7))
            .unwrap();
    }
    assert_eq!(doc.history().entries.len(), 2);
    assert_eq!(label(&doc), "Change channel volume");
    let after = doc.project().clone();
    assert_eq!(after.channel(kick).unwrap().volume, 0.5);

    assert_eq!(doc.undo(), Some(touched(|t| t.channels = true)));
    assert_eq!(doc.project(), &before);
    doc.redo().unwrap();
    assert_eq!(doc.project(), &after);
}

#[test]
fn a_gesture_ends_when_anything_else_happens() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    let entries = |doc: &Document| doc.history().entries.len();
    let level = |doc: &Document| doc.project().channel(kick).unwrap().volume;
    let drag = |doc: &mut Document, gesture| {
        let next = level(doc) - 0.01;
        doc.dispatch(volume(kick, next), gesture).unwrap();
    };

    drag(&mut doc, Some(1));
    drag(&mut doc, Some(1));
    assert_eq!(entries(&doc), 2);

    // Another gesture id.
    drag(&mut doc, Some(2));
    assert_eq!(entries(&doc), 3);
    // Going back to the first id is a new gesture too.
    drag(&mut doc, Some(1));
    assert_eq!(entries(&doc), 4);

    // No gesture id never merges, before or after.
    drag(&mut doc, None);
    drag(&mut doc, None);
    assert_eq!(entries(&doc), 6);
    drag(&mut doc, Some(1));
    assert_eq!(entries(&doc), 7);

    // A dispatch without a gesture in between, even one that changes nothing.
    doc.dispatch(tempo(120.0), None).unwrap();
    drag(&mut doc, Some(1));
    assert_eq!(entries(&doc), 8);

    // A save in between.
    doc.mark_saved();
    drag(&mut doc, Some(1));
    assert_eq!(entries(&doc), 9);
    assert!(doc.is_dirty());

    // An undo and redo in between.
    doc.undo().unwrap();
    doc.redo().unwrap();
    drag(&mut doc, Some(1));
    assert_eq!(entries(&doc), 10);

    // A jump in between, even one that goes nowhere.
    doc.jump(10);
    drag(&mut doc, Some(1));
    assert_eq!(entries(&doc), 11);

    // A dispatch of the same gesture that changes nothing does not end it.
    let same = level(&doc);
    doc.dispatch(volume(kick, same), Some(1)).unwrap();
    drag(&mut doc, Some(1));
    assert_eq!(entries(&doc), 11);

    // Nor does a dispatch that fails.
    doc.dispatch(volume(ChannelId(77), 0.5), Some(1))
        .unwrap_err();
    drag(&mut doc, Some(1));
    assert_eq!(entries(&doc), 11);
}

#[test]
fn a_gesture_can_span_different_commands() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    let ids = add_notes(&mut doc, kick, vec![note(0, 60), note(240, 62)]);
    let before = doc.project().clone();
    let entries = doc.history().entries.len();

    for offset in 1..=20 {
        let patch = |start: u32, key: u8| NotePatch {
            start: Some(start),
            key: Some(key),
            ..NotePatch::default()
        };
        let moved = update_notes(
            kick,
            vec![
                (ids[0], patch(offset * 10, 60 + offset as u8)),
                (ids[1], patch(240 + offset * 10, 62 + offset as u8)),
            ],
        );
        doc.dispatch(moved, Some(3)).unwrap();
        doc.project().check().unwrap();
    }
    doc.dispatch(volume(kick, 0.3), Some(3)).unwrap();
    doc.dispatch(toggle(kick, 8), Some(3)).unwrap();
    assert_eq!(doc.history().entries.len(), entries + 1);
    // The step keeps the label of the gesture's first command.
    assert_eq!(label(&doc), "Move notes");
    let after = doc.project().clone();

    doc.undo().unwrap();
    assert!(same_content(doc.project(), &before));
    doc.redo().unwrap();
    assert_eq!(doc.project(), &after);
}

#[test]
fn a_gesture_that_returns_to_its_start_leaves_no_step() {
    let mut doc = document();
    let (kick, _) = add_channel(&mut doc, "Kick");
    doc.mark_saved();
    let before = doc.project().clone();

    doc.dispatch(volume(kick, 0.2), Some(1)).unwrap();
    assert!(doc.is_dirty());
    assert_eq!(doc.history().entries.len(), 2);
    let applied = doc
        .dispatch(volume(kick, DEFAULT_CHANNEL_VOLUME), Some(1))
        .unwrap();
    // The dispatch itself did change the channel back.
    assert_eq!(applied.touched, touched(|t| t.channels = true));
    assert_eq!(doc.project(), &before);
    assert_eq!(doc.history().entries.len(), 1);
    assert!(!doc.is_dirty());

    // The gesture can carry on and makes a fresh step.
    doc.dispatch(volume(kick, 0.4), Some(1)).unwrap();
    assert_eq!(doc.history().entries.len(), 2);
    doc.undo().unwrap();
    assert_eq!(doc.project(), &before);
}

// Dirty tracking

#[test]
fn the_document_is_dirty_when_it_differs_from_the_saved_state() {
    let mut doc = document();
    assert!(!doc.is_dirty());

    run(&mut doc, tempo(90.0));
    assert!(doc.is_dirty());
    doc.mark_saved();
    assert!(!doc.is_dirty());

    run(&mut doc, tempo(91.0));
    assert!(doc.is_dirty());
    doc.undo().unwrap();
    assert!(!doc.is_dirty(), "undoing back to the saved state is clean");
    doc.undo().unwrap();
    assert!(doc.is_dirty(), "undoing past the saved state is dirty");
    doc.redo().unwrap();
    assert!(!doc.is_dirty());
    doc.redo().unwrap();
    assert!(doc.is_dirty());
}

#[test]
fn the_document_stays_dirty_once_the_saved_state_is_discarded() {
    let mut doc = document();
    run(&mut doc, tempo(90.0));
    run(&mut doc, tempo(91.0));
    doc.mark_saved();
    doc.undo().unwrap();
    doc.undo().unwrap();

    // This drops the two undone steps, and the saved state with them.
    doc.dispatch(tempo(95.0), None).unwrap();
    assert!(doc.is_dirty());
    doc.undo().unwrap();
    assert!(doc.is_dirty());
    doc.redo().unwrap();
    assert!(doc.is_dirty());
    doc.dispatch(tempo(96.0), None).unwrap();
    assert!(doc.is_dirty(), "two steps in is no longer the saved state");

    doc.mark_saved();
    assert!(!doc.is_dirty());
}

// Patches and snapshots

#[test]
fn a_patch_carries_only_the_touched_sections() {
    let mut doc = document();
    assert_eq!(doc.revision(), 0);
    let (kick, _) = add_channel(&mut doc, "Kick");

    let applied = doc.dispatch(toggle(kick, 0), None).unwrap();
    let patch = doc.patch(&applied.touched);
    assert_eq!(patch.revision, 1);
    assert_eq!(doc.revision(), 1);
    assert_eq!(patch.settings, None);
    assert_eq!(patch.samples, None);
    assert_eq!(patch.channels, None);
    assert_eq!(patch.mixer, None);
    assert_eq!(patch.playlist, None);
    assert_eq!(patch.pattern_order, None);
    assert_eq!(patch.patterns, doc.project().patterns);
    assert_eq!(patch.history, doc.history());
    assert!(patch.dirty);

    let json = serde_json::to_value(&patch).unwrap();
    let mut keys: Vec<&str> = json
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["dirty", "history", "patterns", "revision"]);

    let second = add_pattern(&mut doc);
    let applied = doc
        .dispatch(Command::RemovePattern { id: PATTERN }, None)
        .unwrap();
    let patch = doc.patch(&applied.touched);
    assert_eq!(patch.revision, 2);
    assert_eq!(patch.pattern_order, Some(vec![second]));
    assert!(patch.patterns.is_empty());

    let everything = Touched::all(doc.project());
    let patch = doc.patch(&everything);
    assert_eq!(patch.revision, 3);
    assert_eq!(patch.settings.as_ref(), Some(&doc.project().settings));
    assert_eq!(patch.samples.as_ref(), Some(&doc.project().samples));
    assert_eq!(patch.channels.as_ref(), Some(&doc.project().channels));
    assert_eq!(patch.mixer.as_ref(), Some(&doc.project().mixer));
    assert_eq!(patch.playlist.as_ref(), Some(&doc.project().playlist));
    assert_eq!(patch.patterns, doc.project().patterns);

    doc.mark_saved();
    let patch = doc.patch(&Touched::default());
    assert_eq!(patch.revision, 4);
    assert!(!patch.dirty);
}

#[test]
fn a_snapshot_holds_the_whole_document() {
    let mut doc = document();
    run(&mut doc, tempo(90.0));
    doc.patch(&Touched::default());
    let snapshot = doc.snapshot(Some("C:/Music/Song.windfall".to_owned()));
    assert_eq!(snapshot.revision, 1);
    assert_eq!(&snapshot.project, doc.project());
    assert_eq!(snapshot.history, doc.history());
    assert!(snapshot.dirty);
    assert_eq!(snapshot.path.as_deref(), Some("C:/Music/Song.windfall"));
    assert_eq!(doc.snapshot(None).path, None);
}

// Project::check

/// A project that uses every part of the model.
fn full_project() -> Project {
    let mut doc = document();
    let sample = add_sample(&mut doc, "kick");
    let applied = run(
        &mut doc,
        Command::AddChannel {
            name: None,
            sample: Some(sample),
            index: None,
            mixer_track: None,
        },
    );
    let kick = ChannelId(applied.created[0]);
    let (hat, hat_track) = add_channel(&mut doc, "Hat");
    add_notes(&mut doc, kick, vec![note(0, 60), note(240, 60)]);
    add_notes(&mut doc, hat, vec![note(0, 60)]);
    run(
        &mut doc,
        Command::SetSamplerEnvelope {
            id: kick,
            envelope: Some(Envelope::default()),
        },
    );
    let bus = add_mixer_track(&mut doc);
    run(&mut doc, send(hat_track, bus, Some(0.5)));
    let lane = add_playlist_track(&mut doc);
    add_clip(&mut doc, lane, PATTERN, 0);
    add_clip(&mut doc, lane, PATTERN, 3840);
    doc.project().clone()
}

#[test]
fn check_accepts_a_project_built_by_commands() {
    full_project().check().unwrap();
}

/// Breaks one rule of a valid project.
type Damage = fn(&mut Project);

#[test]
fn check_names_each_broken_rule() {
    fn sampler(project: &mut Project) -> &mut SamplerSettings {
        let ChannelSource::Sampler(sampler) = &mut project.channels[0].source;
        sampler
    }
    fn first_note(project: &mut Project) -> &mut Note {
        &mut project.patterns[0].lanes[0].notes[0]
    }
    let cases: Vec<(&str, Damage)> = vec![
        ("format version", |p| p.format_version = 2),
        ("tempo", |p| p.settings.tempo_bpm = 5.0),
        ("tempo", |p| p.settings.tempo_bpm = f64::NAN),
        ("beats per bar", |p| p.settings.time_signature.numerator = 0),
        ("beat unit", |p| p.settings.time_signature.denominator = 5),
        ("swing", |p| p.settings.swing = 1.5),
        ("more than one sample", |p| {
            let copy = p.samples[0].clone();
            p.samples.push(copy);
        }),
        ("repeats the path", |p| {
            let mut copy = p.samples[0].clone();
            copy.id = SampleId(p.next_id - 1);
            p.samples.push(copy);
        }),
        ("forward slashes", |p| {
            p.samples[0].path = SamplePath::Project("a\\b.wav".to_owned());
        }),
        ("reserved for the master", |p| p.samples[0].id = SampleId(0)),
        ("not below the next id", |p| p.next_id = 3),
        ("no master", |p| p.mixer.tracks.clear()),
        ("not the master", |p| p.mixer.tracks.swap(0, 1)),
        ("master track has an output", |p| {
            p.mixer.tracks[0].output = Some(p.mixer.tracks[1].id);
        }),
        ("master track has sends", |p| {
            let target = p.mixer.tracks[1].id;
            p.mixer.tracks[0].sends.push(Send { target, gain: 1.0 });
        }),
        ("more than the limit", |p| {
            let extra = p.mixer.tracks[1].clone();
            p.mixer.tracks.resize(MAX_MIXER_TRACKS + 1, extra);
        }),
        ("more than one mixer track", |p| {
            let copy = p.mixer.tracks[1].clone();
            p.mixer.tracks.push(copy);
        }),
        ("volume", |p| p.mixer.tracks[1].volume = 2.5),
        ("pan", |p| p.mixer.tracks[1].pan = f32::NAN),
        ("not 0xRRGGBB", |p| p.mixer.tracks[1].color = 0x1_000_000),
        ("outputs to mixer track 999", |p| {
            p.mixer.tracks[1].output = Some(TrackId(999));
        }),
        ("sends to mixer track 999", |p| {
            p.mixer.tracks[2].sends[0].target = TrackId(999);
        }),
        ("more than one send", |p| {
            let copy = p.mixer.tracks[2].sends[0];
            p.mixer.tracks[2].sends.push(copy);
        }),
        ("send with gain", |p| p.mixer.tracks[2].sends[0].gain = -1.0),
        ("loops back", |p| {
            p.mixer.tracks[1].output = Some(p.mixer.tracks[1].id);
        }),
        ("loops back", |p| {
            p.mixer.tracks[3].output = Some(p.mixer.tracks[2].id);
        }),
        ("more than one channel", |p| {
            let copy = p.channels[0].clone();
            p.channels.push(copy);
        }),
        ("volume", |p| p.channels[0].volume = -0.5),
        ("pan", |p| p.channels[0].pan = 2.0),
        ("not 0xRRGGBB", |p| p.channels[0].color = u32::MAX),
        ("plays into mixer track 999", |p| {
            p.channels[0].mixer_track = TrackId(999);
        }),
        ("plays sample 999", |p| {
            sampler(p).sample = Some(SampleId(999));
        }),
        ("root key", |p| sampler(p).root_key = 128),
        ("tuned", |p| sampler(p).tune = 49.0),
        ("sample gain", |p| sampler(p).gain = 3.0),
        ("outside 0 to 1", |p| sampler(p).end = 1.5),
        ("not before its end", |p| sampler(p).start = 1.0),
        ("envelope attack", |p| {
            sampler(p).envelope = Some(Envelope {
                attack_ms: -1.0,
                ..Envelope::default()
            });
        }),
        ("envelope sustain", |p| {
            sampler(p).envelope = Some(Envelope {
                sustain: 2.0,
                ..Envelope::default()
            });
        }),
        ("no patterns", |p| {
            p.playlist.clips.clear();
            p.patterns.clear();
        }),
        ("more than one pattern", |p| {
            let mut copy = p.patterns[0].clone();
            copy.lanes.clear();
            p.patterns.push(copy);
        }),
        ("steps long", |p| p.patterns[0].length_steps = 0),
        ("steps long", |p| p.patterns[0].length_steps = 1025),
        ("not 0xRRGGBB", |p| p.patterns[0].color = 0x1_000_000),
        ("lanes sorted", |p| p.patterns[0].lanes.swap(0, 1)),
        ("lanes sorted", |p| {
            let copy = p.patterns[0].lanes[0].clone();
            p.patterns[0].lanes.insert(0, copy);
        }),
        ("the channel does not exist", |p| {
            p.channels.remove(1);
        }),
        ("the lane is empty", |p| {
            p.patterns[0].lanes[0].notes.clear()
        }),
        ("notes are not sorted", |p| {
            p.patterns[0].lanes[0].notes.swap(0, 1);
        }),
        ("more than one note", |p| {
            let id = p.patterns[0].lanes[0].notes[0].id;
            p.patterns[0].lanes[1].notes[0].id = id;
        }),
        ("has no length", |p| first_note(p).length = 0),
        ("ends past the last tick", |p| {
            first_note(p).start = 1;
            first_note(p).length = u32::MAX;
        }),
        ("has key", |p| first_note(p).key = 128),
        ("velocity", |p| first_note(p).velocity = 1.5),
        ("pan", |p| first_note(p).pan = -1.5),
        ("more than one playlist track", |p| {
            let copy = p.playlist.tracks[0].clone();
            p.playlist.tracks.push(copy);
        }),
        ("clips are not sorted", |p| p.playlist.clips.swap(0, 1)),
        ("more than one clip", |p| {
            p.playlist.clips[1].id = p.playlist.clips[0].id;
        }),
        ("on playlist track 999", |p| {
            p.playlist.clips[0].track = PlaylistTrackId(999);
        }),
        ("has no length", |p| p.playlist.clips[0].length = 0),
        ("ends past the last tick", |p| {
            p.playlist.clips[1].length = u32::MAX;
        }),
        ("plays pattern 999", |p| {
            p.playlist.clips[0].content = ClipContent::Pattern {
                pattern: PatternId(999),
            };
        }),
    ];
    for (index, (words, damage)) in cases.into_iter().enumerate() {
        let mut project = full_project();
        damage(&mut project);
        match project.check() {
            Ok(()) => panic!("case {index} (\"{words}\") passed the check"),
            Err(problem) => assert!(
                problem.contains(words),
                "case {index}: \"{problem}\" lacks \"{words}\""
            ),
        }
    }
}
