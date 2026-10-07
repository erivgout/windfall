//! New, open, save, recent projects, settings and backups.

use std::fs;
use std::path::Path;

use windfall_codec::{WavSampleFormat, write_wav};
use windfall_core::AudioBuffer;
use windfall_ipc::{PlayMode, TransportPatch};
use windfall_project::file::{self, BACKUP_FOLDER};
use windfall_project::{Command, PatternId, ProjectSession, SampleId, SamplePath, SettingsPatch};

use super::{Rig, rms, still_running};
use crate::events::Event;
use crate::paths;
use crate::session::{EDITED_WHILE_MOVING, NO_FILE_YET};
use crate::settings::SETTINGS_FILE;

fn rename(rig: &Rig, name: &str) {
    rig.session
        .dispatch(
            Command::UpdateSettings {
                patch: SettingsPatch {
                    name: Some(name.to_owned()),
                    ..SettingsPatch::default()
                },
            },
            None,
        )
        .unwrap();
}

fn write_tone(path: &Path) {
    let buffer = AudioBuffer::from_interleaved(48_000, 2, vec![0.25; 9_600]);
    write_wav(path, &buffer, WavSampleFormat::Int24).unwrap();
}

#[test]
fn a_project_opens_in_the_mode_it_was_saved_in() {
    let rig = Rig::new();
    let session = &rig.session;
    let second = session
        .dispatch(Command::AddPattern { name: None }, None)
        .unwrap()
        .created[0];
    let plain = session.project_save(Some(&rig.file("plain"))).unwrap();
    let history = session.document_snapshot().history;

    // Song mode, looping, with the second pattern selected: none of it is
    // an edit, so the project stays saved and its history as it was.
    let song = session
        .transport_set(TransportPatch {
            mode: Some(PlayMode::Song),
            pattern: Some(PatternId(second)),
            loop_song: Some(true),
        })
        .unwrap();
    let snapshot = session.document_snapshot();
    assert!(!snapshot.dirty);
    assert_eq!(snapshot.history, history);
    let saved = session.project_save(Some(&rig.file("song"))).unwrap();
    let project = rig.project();
    // The file holds the project it always held, and the session beside it.
    let (stored, played) = file::load_with_session(&saved).unwrap();
    assert_eq!(stored, project);
    assert_eq!(
        played,
        Some(ProjectSession {
            mode: PlayMode::Song,
            loop_song: true,
            pattern: Some(PatternId(second)),
        })
    );

    // A new project starts in pattern mode, and the song opens as it was
    // left. The UI hears of it with the project.
    session.project_new().unwrap();
    assert_eq!(session.transport_state().mode, PlayMode::Pattern);
    session
        .transport_set(TransportPatch {
            loop_song: Some(false),
            ..TransportPatch::default()
        })
        .unwrap();
    rig.events.take();
    let opened = session.project_open(&saved).unwrap();
    assert_eq!(opened.project, project);
    assert!(!opened.dirty && opened.history.entries.is_empty());
    assert_eq!(session.transport_state(), song);
    assert_eq!(rig.take_transport_states().last(), Some(&song));

    // The file saved before, in pattern mode on the first pattern with
    // looping off, opens that way.
    let first = PatternId(project.patterns[0].id.0);
    session.project_open(&plain).unwrap();
    let state = session.transport_state();
    assert_eq!(
        (state.mode, state.pattern, state.loop_song),
        (PlayMode::Pattern, first, false)
    );

    // A file from before any of this says nothing: pattern mode, the
    // first pattern, and looping left as it is.
    session.project_open(&saved).unwrap();
    let old = rig.file("old.windfall");
    file::save(&project, &old).unwrap();
    assert_eq!(
        file::session_from_json(&fs::read_to_string(&old).unwrap()),
        None
    );
    session.project_open(&old).unwrap();
    let state = session.transport_state();
    assert_eq!(
        (state.mode, state.pattern, state.loop_song),
        (PlayMode::Pattern, first, true)
    );

    // A pattern the project does not have is passed over.
    let made_up = ProjectSession {
        mode: PlayMode::Song,
        loop_song: false,
        pattern: Some(PatternId(9_999)),
    };
    file::save_with(&project, Some(&made_up), &old).unwrap();
    session.project_open(&old).unwrap();
    let state = session.transport_state();
    assert_eq!(
        (state.mode, state.pattern, state.loop_song),
        (PlayMode::Song, first, false)
    );
}

#[test]
fn a_backup_keeps_how_the_project_was_played_too() {
    let rig = Rig::new();
    let session = &rig.session;
    let saved = session.project_save(Some(&rig.file("song"))).unwrap();
    session
        .transport_set(TransportPatch {
            mode: Some(PlayMode::Song),
            loop_song: Some(true),
            ..TransportPatch::default()
        })
        .unwrap();
    // Changing the mode alone is nothing to back up.
    assert_eq!(session.write_backup("2026-10-06 18-00-00").unwrap(), None);
    rename(&rig, "Edited");
    let backup = session
        .write_backup("2026-10-06 18-00-01")
        .unwrap()
        .unwrap();
    let (_, played) = file::load_with_session(&backup).unwrap();
    let played = played.unwrap();
    assert_eq!((played.mode, played.loop_song), (PlayMode::Song, true));
    // The project's own file is as it was saved, in pattern mode.
    let (_, played) = file::load_with_session(&saved).unwrap();
    assert_eq!(played.unwrap().mode, PlayMode::Pattern);
}

#[test]
fn a_project_with_no_file_cannot_be_saved_without_a_path() {
    let rig = Rig::new();
    rename(&rig, "Nameless");
    rig.events.take();

    assert_eq!(rig.session.project_save(None).unwrap_err(), NO_FILE_YET);
    assert_eq!(
        rig.session.project_save(Some("beat.windfall")).unwrap_err(),
        "\"beat.windfall\" is not a full path"
    );
    assert!(rig.events.take().is_empty());
    assert!(rig.session.document_snapshot().dirty);
}

#[test]
fn saving_writes_the_file_and_remembers_where() {
    let rig = Rig::new();
    let session = &rig.session;
    rename(&rig, "Night Drive");

    let saved = session
        .project_save(Some(&rig.file("songs/night")))
        .unwrap();
    // The folder is made, and the extension added.
    let expected = rig.folder.path().join("songs").join("night.windfall");
    assert_eq!(saved, paths::display(&expected));
    assert_eq!(file::load(&saved).unwrap(), rig.project());
    let snapshot = session.document_snapshot();
    assert_eq!(snapshot.path.as_deref(), Some(saved.as_str()));
    assert!(!snapshot.dirty);
    assert_eq!(session.recent_projects(), std::slice::from_ref(&saved));

    // With a file, saving needs no path.
    rename(&rig, "Night Drive 2");
    assert_eq!(session.project_save(None).unwrap(), saved);
    assert_eq!(file::load(&saved).unwrap().settings.name, "Night Drive 2");

    // Save as: the project moves to the new file.
    let copy = session
        .project_save(Some(&rig.file("copy.WINDFALL")))
        .unwrap();
    assert!(copy.ends_with("copy.WINDFALL"), "{copy}");
    assert_eq!(session.document_snapshot().path, Some(copy.clone()));
    assert_eq!(session.recent_projects(), [copy, saved]);
}

#[test]
fn opening_replaces_the_project_and_resets_the_transport() {
    let mut rig = Rig::new();
    let session = rig.session.clone();
    rename(&rig, "Kept");
    let pattern = rig.pattern();
    session
        .dispatch(
            Command::ToggleStep {
                pattern,
                channel: rig.channel(0),
                step: 0,
            },
            None,
        )
        .unwrap();
    let path = session.project_save(Some(&rig.file("kept"))).unwrap();
    let kept = rig.project();

    // Leave the transport somewhere else, playing.
    session.transport_seek(1_234.0);
    session.transport_play().unwrap();
    rig.run(4_800);
    session
        .transport_set(TransportPatch {
            mode: Some(PlayMode::Song),
            ..TransportPatch::default()
        })
        .unwrap();
    rename(&rig, "Thrown away");
    rig.events.take();

    let opened = session.project_open(&path).unwrap();
    assert_eq!(opened.project, kept);
    assert_eq!(opened.revision, 0);
    assert!(!opened.dirty);
    assert!(opened.history.entries.is_empty());
    assert_eq!(session.document_snapshot(), opened);
    assert_eq!(session.undo(), None);

    let events = rig.events.take();
    let [
        Event::ProjectLoaded(loaded),
        Event::TransportState(transport),
    ] = &events[..]
    else {
        panic!("{events:#?}");
    };
    assert_eq!(loaded, &opened);
    assert!(!transport.playing);
    assert_eq!(transport.mode, PlayMode::Pattern);
    assert_eq!(transport.pattern, pattern);

    // Playback stopped, and the playhead is back at the start.
    let ringing = rig.run(48_000);
    assert!(
        rms(&ringing[90_000..]) < 1e-4,
        "the old project is still playing"
    );
    let frame = session.realtime_tick();
    assert!(!frame.playing);
    assert_eq!(frame.tick, 0.0);
    assert_eq!(session.recent_projects(), [path]);
}

#[test]
fn a_new_project_is_the_default_one_with_no_file() {
    let rig = Rig::new();
    let session = &rig.session;
    let template = rig.project();
    rename(&rig, "Scratch");
    session.project_save(Some(&rig.file("scratch"))).unwrap();
    session
        .dispatch(Command::AddPattern { name: None }, None)
        .unwrap();
    rig.events.take();

    let fresh = session.project_new().unwrap();
    assert_eq!(fresh.project, template);
    assert_eq!(fresh.path, None);
    assert_eq!(fresh.revision, 0);
    assert!(!fresh.dirty);
    let events = rig.events.take();
    assert!(
        matches!(
            &events[..],
            [Event::ProjectLoaded(loaded), Event::TransportState(_)] if loaded == &fresh
        ),
        "{events:#?}"
    );
    for sample in &fresh.project.samples {
        assert!(rig.has_audio(sample.id));
    }
    assert_eq!(session.project_save(None).unwrap_err(), NO_FILE_YET);
}

#[test]
fn a_file_that_is_not_a_project_is_refused_and_the_open_project_stays() {
    let rig = Rig::new();
    let session = &rig.session;
    rename(&rig, "Still here");
    let before = session.document_snapshot();
    rig.events.take();

    let missing = rig.file("missing.windfall");
    let error = session.project_open(&missing).unwrap_err();
    assert!(error.starts_with("Could not read"), "{error}");
    assert!(error.contains("missing.windfall"), "{error}");

    let garbage = rig.file("garbage.windfall");
    fs::write(&garbage, "RIFF....WAVE").unwrap();
    let error = session.project_open(&garbage).unwrap_err();
    assert!(
        error.starts_with("This is not a Windfall project"),
        "{error}"
    );

    let newer = rig.file("newer.windfall");
    fs::write(&newer, r#"{ "formatVersion": 999 }"#).unwrap();
    let error = session.project_open(&newer).unwrap_err();
    assert!(error.contains("newer version of Windfall"), "{error}");

    assert_eq!(session.document_snapshot(), before);
    assert!(rig.events.take().is_empty());
    assert!(session.recent_projects().is_empty());
}

#[test]
fn opening_a_project_whose_samples_are_gone_warns_and_still_opens() {
    let rig = Rig::new();
    let session = &rig.session;
    let lost = rig.file("lost.wav");
    let broken = rig.file("broken.wav");
    write_tone(Path::new(&lost));
    write_tone(Path::new(&broken));
    let lost_sample = SampleId(session.add_channel_from_file(&lost, None).unwrap().created[0]);
    let broken_sample = SampleId(
        session
            .add_channel_from_file(&broken, None)
            .unwrap()
            .created[0],
    );
    session
        .dispatch(
            Command::AddSample {
                name: "Stray".to_owned(),
                path: SamplePath::Project("never/there.wav".to_owned()),
            },
            None,
        )
        .unwrap();
    let path = session
        .project_save(Some(&rig.file("elsewhere/gaps")))
        .unwrap();
    let saved = rig.project();
    fs::remove_file(&lost).unwrap();
    fs::write(&broken, "no longer audio").unwrap();

    let rig = rig.restart();
    let session = &rig.session;
    let opened = session.project_open(&path).unwrap();
    assert_eq!(opened.project, saved);

    let events = rig.events.take();
    let [
        Event::ProjectLoaded(_),
        Event::TransportState(_),
        Event::ProjectWarnings(warnings),
    ] = &events[..]
    else {
        panic!("{events:#?}");
    };
    assert_eq!(warnings.len(), 3, "{warnings:#?}");
    assert_eq!(warnings[0], format!("Missing sample: {lost}"));
    assert!(
        warnings[1].starts_with(&format!("Could not load \"{broken}\"")),
        "{}",
        warnings[1]
    );
    let stray = Path::new(&path).with_file_name("never").join("there.wav");
    assert_eq!(
        warnings[2],
        format!("Missing sample: {}", paths::display(&stray))
    );

    // The channels are there and silent; the kit still plays.
    assert_eq!(opened.project.channels.len(), 6);
    assert!(!rig.has_audio(lost_sample));
    assert!(!rig.has_audio(broken_sample));
    assert!(rig.has_audio(opened.project.samples[0].id));
}

#[test]
fn recent_projects_are_newest_first_and_survive_a_restart() {
    let rig = Rig::new();
    let session = &rig.session;
    let first = session.project_save(Some(&rig.file("first"))).unwrap();
    let second = session.project_save(Some(&rig.file("second"))).unwrap();
    let third = session.project_save(Some(&rig.file("third"))).unwrap();
    assert_eq!(
        session.recent_projects(),
        [third.clone(), second.clone(), first.clone()]
    );

    // Opening one moves it to the front.
    session.project_open(&first).unwrap();
    assert_eq!(
        session.recent_projects(),
        [first.clone(), third.clone(), second.clone()]
    );

    let rig = rig.restart();
    assert_eq!(
        rig.session.recent_projects(),
        [first.clone(), third.clone(), second.clone()]
    );
    // A new run starts on the default project, not on the last one.
    assert_eq!(rig.session.document_snapshot().path, None);

    fs::remove_file(&third).unwrap();
    assert_eq!(rig.session.recent_projects(), [first, second]);
}

#[test]
fn a_damaged_settings_file_does_not_stop_the_app_from_starting() {
    let folder = tempfile::tempdir().unwrap();
    let settings_file = folder.path().join(SETTINGS_FILE);
    fs::write(&settings_file, "{ \"audio\": { \"bufferFrames\": \"lots\"").unwrap();

    let rig = Rig::in_folder(folder);
    assert_eq!(rig.session.document_snapshot().project.channels.len(), 4);
    assert!(rig.session.recent_projects().is_empty());
    assert_eq!(rig.session.browser_roots().len(), 1);
    assert!(rig.session.engine_status().running);

    // The next change replaces the damaged file with a good one.
    let saved = rig.session.project_save(Some(&rig.file("after"))).unwrap();
    let rig = rig.restart();
    assert_eq!(rig.session.recent_projects(), [saved]);
}

#[test]
fn a_backup_is_written_only_for_a_saved_project_with_unsaved_changes() {
    let rig = Rig::new();
    let session = &rig.session;
    let stamp = "2026-10-06 18-13-05";

    // Never saved: there is nowhere to put a backup.
    rename(&rig, "Unsaved");
    assert_eq!(session.write_backup(stamp), Ok(None));

    // Saved and unchanged: nothing to back up.
    let path = session.project_save(Some(&rig.file("song"))).unwrap();
    assert_eq!(session.write_backup(stamp), Ok(None));

    rename(&rig, "Changed");
    let backup = session.write_backup(stamp).unwrap().unwrap();
    let expected = Path::new(&path)
        .with_file_name(BACKUP_FOLDER)
        .join(format!("song {stamp}.windfall"));
    assert_eq!(backup, expected);
    assert_eq!(file::load(&backup).unwrap(), rig.project());
    // A backup is not a save.
    assert!(session.document_snapshot().dirty);
    assert_eq!(file::load(&path).unwrap().settings.name, "Unsaved");

    assert!(
        session
            .write_backup("2026/10/06")
            .unwrap_err()
            .contains("timestamp")
    );
}

#[test]
fn save_as_into_another_folder_takes_the_projects_own_samples_along() {
    let rig = Rig::new();
    let session = &rig.session;
    let home = session.project_save(Some(&rig.file("home/song"))).unwrap();
    let own = Path::new(&home).with_file_name("sounds").join("own.wav");
    fs::create_dir_all(own.parent().unwrap()).unwrap();
    write_tone(&own);
    let added = session
        .add_channel_from_file(&paths::display(&own), None)
        .unwrap();
    let sample = SampleId(added.created[0]);
    assert_eq!(
        rig.project().sample(sample).unwrap().path,
        SamplePath::Project("sounds/own.wav".to_owned())
    );

    let moved = session.project_save(Some(&rig.file("away/song"))).unwrap();
    assert!(
        Path::new(&moved)
            .with_file_name("sounds")
            .join("own.wav")
            .is_file()
    );

    let rig = rig.restart();
    rig.session.project_open(&moved).unwrap();
    assert!(rig.has_audio(sample));
    assert!(
        !rig.events
            .take()
            .iter()
            .any(|event| matches!(event, Event::ProjectWarnings(_)))
    );
}

/// Saves the rig's project as `home/song` and gives it a channel that plays
/// a sample of its own, `sounds/own.wav`. The project is left with that
/// edit unsaved. Returns the project's path and the sample.
fn project_with_its_own_sample(rig: &Rig) -> (String, SampleId) {
    let home = rig
        .session
        .project_save(Some(&rig.file("home/song")))
        .unwrap();
    let own = Path::new(&home).with_file_name("sounds").join("own.wav");
    fs::create_dir_all(own.parent().unwrap()).unwrap();
    write_tone(&own);
    let added = rig
        .session
        .add_channel_from_file(&paths::display(&own), None)
        .unwrap();
    (home, SampleId(added.created[0]))
}

/// Writes a sound that is not the one [`write_tone`] writes.
fn write_other_tone(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let buffer = AudioBuffer::from_interleaved(48_000, 2, vec![-0.75; 4_800]);
    write_wav(path, &buffer, WavSampleFormat::Int24).unwrap();
}

fn names_in(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort_unstable();
    names
}

fn own_sample_path(name: &str) -> SamplePath {
    SamplePath::Project(format!("sounds/{name}"))
}

#[test]
fn an_older_save_cannot_land_on_top_of_a_newer_one() {
    let rig = Rig::new();
    let session = &rig.session;
    rename(&rig, "First");
    let path = session.project_save(Some(&rig.file("song"))).unwrap();

    // One save has copied the project and is about to write it.
    rename(&rig, "Second");
    let hold = session.hold("save:write");
    let older = session.background(|session| session.project_save(None));
    hold.wait();

    // The project moves on, and a second save is asked for.
    rename(&rig, "Third");
    let newer = session.background(|session| session.project_save(None));
    assert!(
        still_running(&newer),
        "the newer save did not wait for the older one"
    );

    hold.release();
    assert_eq!(older.join().unwrap().unwrap(), path);
    assert_eq!(newer.join().unwrap().unwrap(), path);
    // What is on disk is what is open, and only therefore is it clean.
    assert_eq!(file::load(&path).unwrap(), rig.project());
    assert_eq!(rig.project().settings.name, "Third");
    assert!(!session.document_snapshot().dirty);
    let dirty: Vec<bool> = rig.take_patches().iter().map(|patch| patch.dirty).collect();
    // The older save found the project changed and left it marked.
    assert_eq!(dirty[dirty.len() - 2..], [true, false]);
}

#[test]
fn backups_reach_the_disk_in_the_order_they_were_taken() {
    let rig = Rig::new();
    let session = &rig.session;
    let path = session.project_save(Some(&rig.file("song"))).unwrap();
    let stamp = "2026-10-06 18-13-05";

    rename(&rig, "Second");
    let hold = session.hold("backup:write");
    let older = session.background(move |session| session.write_backup(stamp));
    hold.wait();

    rename(&rig, "Third");
    let newer = session.background(move |session| session.write_backup(stamp));
    assert!(
        still_running(&newer),
        "the newer backup did not wait for the older one"
    );

    hold.release();
    let backup = older.join().unwrap().unwrap().unwrap();
    assert_eq!(newer.join().unwrap().unwrap(), Some(backup.clone()));
    assert_eq!(file::load(&backup).unwrap().settings.name, "Third");
    // Neither was a save.
    assert_eq!(file::load(&path).unwrap().settings.name, "Untitled");
    assert!(session.document_snapshot().dirty);
}

#[test]
fn an_open_that_a_new_project_overtook_is_cancelled() {
    let rig = Rig::new();
    let session = &rig.session;
    rename(&rig, "On disk");
    let path = session.project_save(Some(&rig.file("on disk"))).unwrap();

    // The open has read its file and is about to swap the project in.
    let hold = session.hold("open:install");
    let opening = {
        let path = path.clone();
        session.background(move |session| session.project_open(&path))
    };
    hold.wait();

    // Meanwhile the user starts a new project and works in it.
    session.project_new().unwrap();
    rename(&rig, "Newer work");
    session
        .dispatch(Command::AddPattern { name: None }, None)
        .unwrap();
    let newer = session.document_snapshot();
    rig.events.take();

    hold.release();
    assert_eq!(
        opening.join().unwrap().unwrap_err(),
        "\"on disk.windfall\" was not opened because another project was opened or started after it."
    );
    assert_eq!(session.document_snapshot(), newer);
    assert_eq!(newer.history.entries.len(), 2);
    assert!(newer.dirty);
    assert!(rig.events.take().is_empty());
    assert!(session.undo().is_some());
}

#[test]
fn an_open_is_cancelled_when_the_project_was_edited_while_it_loaded() {
    let rig = Rig::new();
    let session = &rig.session;
    rename(&rig, "On disk");
    let path = session.project_save(Some(&rig.file("on disk"))).unwrap();

    // The project is clean when the open is asked for, so the UI had no
    // reason to ask about unsaved changes.
    let hold = session.hold("open:install");
    let opening = {
        let path = path.clone();
        session.background(move |session| session.project_open(&path))
    };
    hold.wait();
    rename(&rig, "Edited while it loaded");
    let edited = session.document_snapshot();
    rig.events.take();

    hold.release();
    assert_eq!(
        opening.join().unwrap().unwrap_err(),
        "\"on disk.windfall\" was not opened because the project was edited in the meantime. Save or discard those changes, then try again."
    );
    assert_eq!(session.document_snapshot(), edited);
    assert!(edited.dirty);
    assert!(rig.events.take().is_empty());

    // The same goes for a new project.
    let hold = session.hold("new:install");
    let starting = session.background(|session| session.project_new());
    hold.wait();
    rename(&rig, "Edited once more");
    hold.release();
    assert_eq!(
        starting.join().unwrap().unwrap_err(),
        "The new project was not started because the project was edited in the meantime. Save or discard those changes, then try again."
    );
    assert_eq!(rig.project().settings.name, "Edited once more");

    // With nothing in between, it opens.
    assert_eq!(
        session.project_open(&path).unwrap().project.settings.name,
        "On disk"
    );
}

#[test]
fn an_open_goes_ahead_when_nothing_in_between_changed_the_project() {
    let rig = Rig::new();
    let session = &rig.session;
    rename(&rig, "First");
    let first = session.project_save(Some(&rig.file("first"))).unwrap();
    rename(&rig, "Second");
    let second = session.project_save(Some(&rig.file("second"))).unwrap();

    let hold = session.hold("open:install");
    let opening = session.background(move |session| session.project_open(&first));
    hold.wait();
    // A save, a backup and a command that changes nothing leave the
    // project as the user last saw it.
    assert_eq!(session.project_save(None).unwrap(), second);
    assert_eq!(session.write_backup("2026-10-06 18-13-05"), Ok(None));
    rename(&rig, "Second");
    assert!(!session.document_snapshot().dirty);
    hold.release();

    let opened = opening.join().unwrap().unwrap();
    assert_eq!(opened.project.settings.name, "First");
    assert_eq!(session.document_snapshot(), opened);
}

#[test]
fn of_two_requests_to_replace_the_project_the_later_one_wins() {
    let rig = Rig::new();
    let session = &rig.session;
    rename(&rig, "On disk");
    let path = session.project_save(Some(&rig.file("on disk"))).unwrap();
    rig.events.take();

    // The open is asked for first, and both are ready to swap in.
    let open_hold = session.hold("open:install");
    let opening = session.background(move |session| session.project_open(&path));
    open_hold.wait();
    let new_hold = session.hold("new:install");
    let starting = session.background(|session| session.project_new());
    new_hold.wait();

    // The earlier request gets there first and still loses.
    open_hold.release();
    let error = opening.join().unwrap().unwrap_err();
    assert!(
        error.contains("another project was opened or started"),
        "{error}"
    );
    assert_eq!(rig.project().settings.name, "On disk");
    assert!(rig.events.take().is_empty());

    new_hold.release();
    let fresh = starting.join().unwrap().unwrap();
    assert_eq!(fresh.project.settings.name, "Untitled");
    assert_eq!(session.document_snapshot(), fresh);
}

#[test]
fn save_as_uses_a_sample_that_is_already_in_the_new_folder() {
    let rig = Rig::new();
    let session = &rig.session;
    let (home, sample) = project_with_its_own_sample(&rig);
    let away = rig.folder.path().join("away");
    // The same sound under the same name, as after the folder was copied.
    let there = away.join("sounds").join("own.wav");
    fs::create_dir_all(there.parent().unwrap()).unwrap();
    fs::copy(
        Path::new(&home).with_file_name("sounds").join("own.wav"),
        &there,
    )
    .unwrap();
    rig.events.take();

    let moved = session.project_save(Some(&rig.file("away/song"))).unwrap();
    assert_eq!(names_in(&away.join("sounds")), ["own.wav"]);
    assert_eq!(
        rig.project().sample(sample).unwrap().path,
        own_sample_path("own.wav")
    );
    assert_eq!(file::load(&moved).unwrap(), rig.project());
    // An ordinary save: the history is kept and nothing was reloaded.
    let snapshot = session.document_snapshot();
    assert_eq!(snapshot.history.entries.len(), 1);
    assert!(!snapshot.dirty);
    let events = rig.events.take();
    assert!(
        matches!(&events[..], [Event::ProjectPatch(_)]),
        "{events:#?}"
    );
}

#[test]
fn save_as_does_not_take_another_sound_that_has_the_samples_name() {
    let rig = Rig::new();
    let session = &rig.session;
    let (home, sample) = project_with_its_own_sample(&rig);
    let ours = fs::read(Path::new(&home).with_file_name("sounds").join("own.wav")).unwrap();
    let audio = session.state().pool.get(sample).unwrap().clone();

    // The new folder has a file of that name with another sound in it.
    let away = rig.folder.path().join("away");
    let theirs_file = away.join("sounds").join("own.wav");
    write_other_tone(&theirs_file);
    let theirs = fs::read(&theirs_file).unwrap();
    assert_ne!(ours, theirs);
    rig.events.take();

    let moved = session.project_save(Some(&rig.file("away/song"))).unwrap();

    // Their file is as it was, and ours is beside it under a new name.
    assert_eq!(fs::read(&theirs_file).unwrap(), theirs);
    assert_eq!(names_in(&away.join("sounds")), ["own (2).wav", "own.wav"]);
    assert_eq!(
        fs::read(away.join("sounds").join("own (2).wav")).unwrap(),
        ours
    );

    // The file points at the copy, and so does the open project.
    let written = file::load(&moved).unwrap();
    assert_eq!(
        written.sample(sample).unwrap().path,
        own_sample_path("own (2).wav")
    );
    let snapshot = session.document_snapshot();
    assert_eq!(snapshot.project, written);
    assert_eq!(snapshot.path.as_deref(), Some(moved.as_str()));
    assert!(!snapshot.dirty);
    // The sample was pointed at the copy in place: the UI hears of it as a
    // patch that carries the samples, and the undo history is still there.
    assert_eq!(snapshot.history.entries.len(), 1);
    match &rig.events.take()[..] {
        [Event::ProjectPatch(patch)] => {
            assert_eq!(patch.samples.as_ref(), Some(&written.samples));
            assert!(patch.channels.is_none() && patch.mixer.is_none());
            assert_eq!(patch.history, snapshot.history);
            assert!(!patch.dirty);
        }
        other => panic!("expected one patch, got {other:?}"),
    }
    assert!(rig.has_audio(sample));
    assert_eq!(
        session.state().pool.get(sample).unwrap().samples(),
        audio.samples()
    );

    // Undoing the edit that added the sample takes it away, and redoing
    // brings it back under the name of the copy, not the name it had when
    // the edit was made, which belongs to the other sound here.
    assert!(session.undo().is_some());
    assert!(rig.project().sample(sample).is_none());
    assert!(session.redo().is_some());
    assert_eq!(rig.project(), written);
    assert!(!session.document_snapshot().dirty);
    rig.wait_until_loaded_or_failed(sample);
    assert_eq!(
        session.state().pool.get(sample).unwrap().samples(),
        audio.samples()
    );

    // Opened again, it still plays our sound and not theirs.
    let rig = rig.restart();
    rig.session.project_open(&moved).unwrap();
    assert_eq!(
        rig.session.state().pool.get(sample).unwrap().samples(),
        audio.samples()
    );
    assert!(
        !rig.events
            .take()
            .iter()
            .any(|event| matches!(event, Event::ProjectWarnings(_)))
    );
}

#[test]
fn a_sample_that_cannot_be_copied_fails_the_save_and_the_project_stays_unsaved() {
    let rig = Rig::new();
    let session = &rig.session;
    let (home, _) = project_with_its_own_sample(&rig);
    // A file sits where the folder of the sample would have to go.
    let away = rig.folder.path().join("away");
    fs::create_dir_all(&away).unwrap();
    fs::write(away.join("sounds"), "in the way").unwrap();
    let before = session.document_snapshot();
    assert!(before.dirty);
    rig.events.take();

    let error = session
        .project_save(Some(&rig.file("away/song")))
        .unwrap_err();
    assert!(
        error.starts_with("Could not copy the sample \"own\" to "),
        "{error}"
    );
    assert!(error.ends_with(". The project was not saved."), "{error}");
    assert!(!away.join("song.windfall").exists());
    assert_eq!(session.document_snapshot(), before);
    assert!(rig.events.take().is_empty());
    assert_eq!(session.recent_projects(), std::slice::from_ref(&home));

    // It can still be saved where it was.
    assert_eq!(session.project_save(None).unwrap(), home);
    assert!(!session.document_snapshot().dirty);
}

#[test]
fn a_save_as_that_renamed_a_sample_and_was_overtaken_by_an_edit_keeps_both() {
    let rig = Rig::new();
    let session = &rig.session;
    let (home, sample) = project_with_its_own_sample(&rig);
    let audio = session.state().pool.get(sample).unwrap().clone();
    let away = rig.folder.path().join("away");
    write_other_tone(&away.join("sounds").join("own.wav"));
    let target = rig.file("away/song");

    let hold = session.hold("save:write");
    let saving = {
        let target = target.clone();
        session.background(move |session| session.project_save(Some(&target)))
    };
    hold.wait();
    rename(&rig, "Edited meanwhile");
    // A sample from the old project folder comes in too, after the samples
    // to take along were listed.
    let late = Path::new(&home).with_file_name("sounds").join("late.wav");
    write_other_tone(&late);
    let added = session
        .add_channel_from_file(&paths::display(&late), None)
        .unwrap();
    let late_sample = SampleId(added.created[0]);
    assert_eq!(
        rig.project().sample(late_sample).unwrap().path,
        own_sample_path("late.wav")
    );
    let edited = session.document_snapshot();
    rig.events.take();
    hold.release();

    // The save is finished: the project lives in the new folder, with its
    // sample pointed at the copy that was made beside the other sound.
    let moved = saving.join().unwrap().unwrap();
    assert_eq!(names_in(&away.join("sounds")), ["own (2).wav", "own.wav"]);
    let snapshot = session.document_snapshot();
    assert_eq!(snapshot.path.as_deref(), Some(moved.as_str()));
    assert_ne!(moved, home);
    assert_eq!(
        snapshot.project.sample(sample).unwrap().path,
        own_sample_path("own (2).wav")
    );
    // The sample that came too late to be taken along is found where it
    // is, not looked for in the new folder.
    assert_eq!(
        snapshot.project.sample(late_sample).unwrap().path,
        SamplePath::External(paths::display(&late))
    );

    // The edits made meanwhile are all still there, with their history,
    // and they are not in the file: the project has unsaved changes.
    assert_eq!(snapshot.project.settings.name, "Edited meanwhile");
    assert_eq!(snapshot.project.channels, edited.project.channels);
    assert_eq!(snapshot.history, edited.history);
    assert_eq!(snapshot.history.entries.len(), 3);
    assert!(snapshot.dirty);
    let written = file::load(&moved).unwrap();
    assert_ne!(written.settings.name, "Edited meanwhile");
    assert!(written.sample(late_sample).is_none());
    assert_eq!(
        written.sample(sample).unwrap().path,
        own_sample_path("own (2).wav")
    );

    // The UI hears of the new paths as one ordinary patch.
    match &rig.events.take()[..] {
        [Event::ProjectPatch(patch)] => {
            assert_eq!(patch.samples.as_ref(), Some(&snapshot.project.samples));
            assert!(patch.settings.is_none() && patch.channels.is_none());
            assert_eq!(patch.history, snapshot.history);
            assert!(patch.dirty);
        }
        other => panic!("expected one patch, got {other:?}"),
    }
    assert_eq!(
        session.state().pool.get(sample).unwrap().samples(),
        audio.samples()
    );

    // Undoing everything and redoing it brings the samples back under the
    // paths they have now.
    while session.undo().is_some() {}
    assert!(rig.project().sample(sample).is_none());
    while session.redo().is_some() {}
    assert_eq!(rig.project(), snapshot.project);

    // Saved again, where it now lives, the file holds all of it, and it
    // opens with both sounds.
    assert_eq!(session.project_save(None).unwrap(), moved);
    assert!(!session.document_snapshot().dirty);
    assert_eq!(file::load(&moved).unwrap(), rig.project());
    let rig = rig.restart();
    rig.session.project_open(&moved).unwrap();
    assert_eq!(
        rig.session.state().pool.get(sample).unwrap().samples(),
        audio.samples()
    );
    assert!(rig.has_audio(late_sample));
    assert!(
        !rig.events
            .take()
            .iter()
            .any(|event| matches!(event, Event::ProjectWarnings(_)))
    );
}

#[test]
fn a_save_as_overtaken_by_an_edit_is_refused_only_when_a_new_name_is_taken_in_the_history() {
    let rig = Rig::new();
    let session = &rig.session;
    let (home, sample) = project_with_its_own_sample(&rig);
    // A second sample of the project's own, added and then undone, so that
    // it lives on in the history alone: `own (2).wav`.
    let second = Path::new(&home)
        .with_file_name("sounds")
        .join("own (2).wav");
    write_other_tone(&second);
    session
        .add_channel_from_file(&paths::display(&second), None)
        .unwrap();
    assert!(session.undo().is_some());
    let away = rig.folder.path().join("away");
    write_other_tone(&away.join("sounds").join("own.wav"));
    let target = rig.file("away/song");

    let hold = session.hold("save:write");
    let saving = {
        let target = target.clone();
        session.background(move |session| session.project_save(Some(&target)))
    };
    hold.wait();
    // Edits that leave the project as it was and keep what can be redone:
    // the last step is undone and done again.
    assert!(session.undo().is_some());
    assert!(session.redo().is_some());
    let edited = session.document_snapshot();
    assert_eq!(edited.history.entries.len(), 2);
    rig.events.take();
    hold.release();

    // The copy in the new folder is `own (2).wav`, which the undone sample
    // has. Renaming would need a fresh history, and the edits forbid that.
    assert_eq!(saving.join().unwrap().unwrap_err(), EDITED_WHILE_MOVING);
    assert_eq!(session.document_snapshot(), edited);
    assert_eq!(edited.path.as_deref(), Some(home.as_str()));
    assert_eq!(
        rig.project().sample(sample).unwrap().path,
        own_sample_path("own.wav")
    );
    assert!(rig.events.take().is_empty());
}

#[test]
fn save_as_tries_missing_samples_again_in_the_new_folder() {
    let rig = Rig::new();
    let (home, sample) = project_with_its_own_sample(&rig);
    rig.session.project_save(None).unwrap();
    let own = Path::new(&home).with_file_name("sounds").join("own.wav");
    let sound = fs::read(&own).unwrap();
    fs::remove_file(&own).unwrap();

    let rig = rig.restart();
    let session = &rig.session;
    session.project_open(&home).unwrap();
    assert!(!rig.has_audio(sample));
    rig.events.take();

    // Nothing to take along, and nothing there either: it is still
    // missing, now from the new place.
    let nowhere = session
        .project_save(Some(&rig.file("nowhere/song")))
        .unwrap();
    let expected = Path::new(&nowhere).with_file_name("sounds").join("own.wav");
    assert!(
        rig.events
            .take()
            .contains(&Event::ProjectWarnings(vec![format!(
                "Missing sample: {}",
                paths::display(&expected)
            )]))
    );
    assert!(!rig.has_audio(sample));

    // Saved beside where the sample went, the project plays it again.
    let found = rig
        .folder
        .path()
        .join("found")
        .join("sounds")
        .join("own.wav");
    fs::create_dir_all(found.parent().unwrap()).unwrap();
    fs::write(&found, sound).unwrap();
    session.project_save(Some(&rig.file("found/song"))).unwrap();
    assert!(rig.has_audio(sample));
    assert_eq!(
        rig.project().sample(sample).unwrap().path,
        own_sample_path("own.wav")
    );
    assert!(
        !rig.events
            .take()
            .iter()
            .any(|event| matches!(event, Event::ProjectWarnings(_)))
    );
}

#[test]
fn a_backup_opens_with_its_samples_and_has_to_be_saved_as() {
    let rig = Rig::new();
    let (home, sample) = project_with_its_own_sample(&rig);
    rig.session.project_save(None).unwrap();
    rename(&rig, "Backed up");
    let backed_up = rig.project();
    let backup = rig
        .session
        .write_backup("2026-10-06 18-13-05")
        .unwrap()
        .unwrap();
    let backups = backup.parent().unwrap().to_path_buf();

    let rig = rig.restart();
    let session = &rig.session;
    let opened = session.project_open(&paths::display(&backup)).unwrap();
    assert_eq!(opened.project, backed_up);
    // Its own sample is found in the project folder, one level up.
    assert!(rig.has_audio(sample));
    // It has no file: saving must not put it in the Backup folder, and
    // must not replace the project it came from without being told to.
    assert_eq!(opened.path, None);
    assert!(!opened.dirty);
    // A backup is not offered again under Open recent.
    assert!(!session.recent_projects().contains(&paths::display(&backup)));
    let events = rig.events.take();
    let [
        Event::ProjectLoaded(_),
        Event::TransportState(_),
        Event::ProjectWarnings(warnings),
    ] = &events[..]
    else {
        panic!("{events:#?}");
    };
    assert_eq!(
        warnings[..],
        [
            "\"song 2026-10-06 18-13-05.windfall\" is a backup of \"song.windfall\". It was opened as a copy: use Save as to keep it."
        ]
    );

    assert_eq!(session.project_save(None).unwrap_err(), NO_FILE_YET);
    assert_eq!(names_in(&backups), ["song 2026-10-06 18-13-05.windfall"]);
    assert_eq!(file::load(&home).unwrap().settings.name, "Untitled");
    // Nothing to back up for a project with no file.
    rename(&rig, "Restored");
    assert_eq!(session.write_backup("2026-10-06 18-20-00"), Ok(None));

    // A sample added now is stored relative to the project folder too.
    let second = Path::new(&home).with_file_name("sounds").join("second.wav");
    write_tone(&second);
    let added = session
        .add_channel_from_file(&paths::display(&second), None)
        .unwrap();
    assert_eq!(
        rig.project()
            .sample(SampleId(added.created[0]))
            .unwrap()
            .path,
        own_sample_path("second.wav")
    );

    // Saved over the project it came from, the samples are where they are.
    assert_eq!(session.project_save(Some(&home)).unwrap(), home);
    assert_eq!(
        session.document_snapshot().path.as_deref(),
        Some(home.as_str())
    );
    assert_eq!(file::load(&home).unwrap().settings.name, "Restored");
    assert_eq!(
        names_in(Path::new(&home).parent().unwrap()),
        ["Backup", "song.windfall", "sounds"]
    );

    // Saved somewhere else straight from the backup, they are taken along.
    session.project_open(&paths::display(&backup)).unwrap();
    let copy = session
        .project_save(Some(&rig.file("restored/song")))
        .unwrap();
    assert!(
        Path::new(&copy)
            .with_file_name("sounds")
            .join("own.wav")
            .is_file()
    );
    let rig = rig.restart();
    rig.session.project_open(&copy).unwrap();
    assert!(rig.has_audio(sample));
}
