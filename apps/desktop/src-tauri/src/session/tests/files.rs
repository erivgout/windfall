//! New, open, save, recent projects, settings and backups.

use std::fs;
use std::path::Path;

use windfall_codec::{WavSampleFormat, write_wav};
use windfall_core::AudioBuffer;
use windfall_ipc::{PlayMode, TransportPatch};
use windfall_project::file::{self, BACKUP_FOLDER};
use windfall_project::{Command, SampleId, SamplePath, SettingsPatch};

use super::{Rig, rms};
use crate::events::Event;
use crate::paths;
use crate::session::NO_FILE_YET;
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
    session
        .transport_set(TransportPatch {
            mode: Some(PlayMode::Song),
            ..TransportPatch::default()
        })
        .unwrap();
    session.transport_seek(1_234.0);
    session.transport_play();
    rig.run(4_800);
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

    let fresh = session.project_new();
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
