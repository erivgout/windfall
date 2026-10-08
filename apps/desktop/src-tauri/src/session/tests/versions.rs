use super::{Rig, rms, still_running};
use std::fs;
use windfall_project::{Command, SettingsPatch, file};
fn rename(rig: &Rig, name: &str) {
    rig.session
        .dispatch(
            Command::UpdateSettings {
                patch: SettingsPatch {
                    name: Some(name.into()),
                    ..Default::default()
                },
            },
            None,
        )
        .unwrap();
}
#[test]
fn padding_numbered_stems_collisions_and_older_files_are_preserved() {
    let rig = Rig::new();
    let base = rig
        .session
        .project_save(Some(&rig.file("夜 Song")))
        .unwrap();
    let bytes = fs::read(&base).unwrap();
    fs::write(rig.file("夜 Song (001).windfall"), b"occupied").unwrap();
    let second = rig.session.project_save_new_version(None).unwrap();
    assert!(second.ends_with("夜 Song (002).windfall"));
    assert_eq!(
        rig.session.document_snapshot().path.as_deref(),
        Some(second.as_str())
    );
    let third = rig.session.project_save_new_version(None).unwrap();
    assert!(third.ends_with("夜 Song (003).windfall"));
    assert_eq!(fs::read(base).unwrap(), bytes);
    assert_eq!(
        fs::read(rig.file("夜 Song (001).windfall")).unwrap(),
        b"occupied"
    );
    assert_eq!(file::load(second).unwrap(), file::load(third).unwrap());
}
#[test]
fn edits_and_replacement_while_saving_do_not_mark_other_work_clean() {
    for replacement in [false, true] {
        let rig = Rig::new();
        rig.session.project_save(Some(&rig.file("Song"))).unwrap();
        rename(&rig, "Captured");
        let held = rig.session.hold("save:write");
        let work = rig.session.background(|s| s.project_save_new_version(None));
        held.wait();
        if replacement {
            rig.session.project_new().unwrap();
        }
        rename(&rig, "Concurrent");
        let before = rig.session.document_snapshot();
        held.release();
        let saved = work.join().unwrap().unwrap();
        assert_eq!(file::load(&saved).unwrap().settings.name, "Captured");
        let after = rig.session.document_snapshot();
        assert_eq!(after.project.settings.name, "Concurrent");
        assert!(after.dirty);
        if replacement {
            assert_eq!(after, before);
        } else {
            assert_eq!(after.path.as_deref(), Some(saved.as_str()));
        }
    }
}
#[test]
fn same_session_saves_are_serialized_and_process_competitor_is_not_overwritten() {
    let rig = Rig::new();
    rig.session.project_save(Some(&rig.file("Song"))).unwrap();
    let held = rig.session.hold("save:write");
    let first = rig.session.background(|s| s.project_save_new_version(None));
    held.wait();
    let second = rig.session.background(|s| s.project_save_new_version(None));
    assert!(still_running(&second));
    fs::write(rig.file("Song (001).windfall"), b"competitor").unwrap();
    held.release();
    assert!(
        first
            .join()
            .unwrap()
            .unwrap()
            .ends_with("Song (002).windfall")
    );
    assert!(
        second
            .join()
            .unwrap()
            .unwrap()
            .ends_with("Song (003).windfall")
    );
    assert_eq!(
        fs::read(rig.file("Song (001).windfall")).unwrap(),
        b"competitor"
    );
}
#[test]
fn independent_saving_owners_race_atomic_publication_without_overwrite() {
    let rig = Rig::new();
    let project = rig.project();
    let played = rig.session.played();
    let base = std::path::PathBuf::from(rig.file("Same.windfall"));
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let threads: Vec<_> = (0..2)
        .map(|_| {
            let (project, base, barrier) = (project.clone(), base.clone(), barrier.clone());
            std::thread::spawn(move || {
                let (path, written, prepared_path) =
                    crate::session::versions::write(&project, &played, &base, |_, target| {
                        if target.file_name().unwrap() == "Same (001).windfall" {
                            barrier.wait();
                        }
                        Ok(target.to_path_buf())
                    })
                    .unwrap();
                assert_eq!(path, prepared_path);
                assert_eq!(written, project);
                path
            })
        })
        .collect();
    let paths: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_ne!(paths[0], paths[1]);
    for path in paths {
        assert_eq!(file::load(path).unwrap(), project);
    }
}
#[test]
fn recording_waits_for_numbered_save_and_keeps_its_take() {
    let rig = Rig::new();
    let held = rig.session.hold("save:write");
    let base = rig.file("Recording.windfall");
    let work = rig
        .session
        .background(move |session| session.project_save_new_version(Some(&base)));
    held.wait();
    let recording = rig
        .session
        .background(super::archive::start_recording_session);
    assert!(still_running(&recording));
    held.release();
    let saved = work.join().unwrap().unwrap();
    recording.join().unwrap();
    assert_eq!(rig.session.document_snapshot().path, Some(saved.clone()));
    assert!(rig.session.recording_state().active);
    assert_eq!(file::load(saved).unwrap(), rig.project());
    assert!(
        rig.session
            .project_save_new_version(None)
            .unwrap_err()
            .contains("recording")
    );
    rig.session.recording_cancel();
}
#[test]
fn unsaved_base_failed_write_and_carry_samples_use_ordinary_workflow() {
    let rig = Rig::new();
    assert!(rig.session.project_save_new_version(None).is_err());
    let before = rig.session.document_snapshot();
    fs::write(rig.file("blocked"), b"file not folder").unwrap();
    assert!(
        rig.session
            .project_save_new_version(Some(&rig.file("blocked/Song")))
            .is_err()
    );
    assert_eq!(rig.session.document_snapshot(), before);
    let source = rig.file("nested/tone.wav");
    fs::create_dir_all(std::path::Path::new(&source).parent().unwrap()).unwrap();
    windfall_codec::write_wav(
        &source,
        &windfall_core::AudioBuffer::from_interleaved(48000, 2, vec![0.2; 960]),
        windfall_codec::WavSampleFormat::Float32,
    )
    .unwrap();
    rig.session.project_save(Some(&rig.file("Song"))).unwrap();
    rig.session.add_channel_from_file(&source, None).unwrap();
    let saved = rig
        .session
        .project_save_new_version(Some(&rig.file("new/Song")))
        .unwrap();
    assert!(
        saved.ends_with("new/Song (001).windfall") || saved.ends_with("new\\Song (001).windfall")
    );
    assert_eq!(
        fs::read(rig.file("new/nested/tone.wav")).unwrap(),
        fs::read(source).unwrap()
    );
    rig.session.project_open(&saved).unwrap();
    assert!(rig.has_audio(rig.project().samples.last().unwrap().id));
}

#[test]
fn backup_shaped_numbered_destination_carries_relinks_and_reopens_audio() {
    for concurrent_edit in [false, true] {
        let rig = Rig::new();
        let home = rig
            .session
            .project_save(Some(&rig.file("home/Song")))
            .unwrap();
        let source = rig.file("home/sounds/tone.wav");
        fs::create_dir_all(std::path::Path::new(&source).parent().unwrap()).unwrap();
        windfall_codec::write_wav(
            &source,
            &windfall_core::AudioBuffer::from_interleaved(48000, 2, vec![0.2; 960]),
            windfall_codec::WavSampleFormat::Float32,
        )
        .unwrap();
        let added = rig.session.add_channel_from_file(&source, None).unwrap();
        let sample = windfall_project::SampleId(added.created[0]);
        rig.session
            .dispatch(
                Command::ToggleStep {
                    pattern: rig.pattern(),
                    channel: windfall_project::ChannelId(added.created[1]),
                    step: 0,
                },
                None,
            )
            .unwrap();
        let audio = rig.session.state().pool.get(sample).unwrap().clone();
        rig.session.project_save(None).unwrap();
        let old = fs::read(&home).unwrap();
        rename(&rig, "Backup source");
        let backup = rig
            .session
            .write_backup("2026-10-06 18-12-00")
            .unwrap()
            .unwrap();
        let old_backup = fs::read(&backup).unwrap();
        rig.session
            .project_open(&crate::paths::display(&backup))
            .unwrap();
        rename(&rig, "Captured");
        let base = rig.file("away/Backup/Song 2026-10-06 18-13-05.windfall");
        fs::create_dir_all(std::path::Path::new(&base).parent().unwrap()).unwrap();
        fs::write(&base, b"old backup").unwrap();
        let occupied = rig.file("away/Backup/Song 2026-10-06 18-13-05 (001).windfall");
        fs::write(&occupied, b"older version").unwrap();
        // Exercise relinking too: the final destination holds another sound.
        let collision = rig.file("away/Backup/sounds/tone.wav");
        fs::create_dir_all(std::path::Path::new(&collision).parent().unwrap()).unwrap();
        fs::write(&collision, b"another sound").unwrap();
        let held = rig.session.hold("save:write");
        let work = rig
            .session
            .background(move |s| s.project_save_new_version(Some(&base)));
        held.wait();
        if concurrent_edit {
            rename(&rig, "Concurrent");
        }
        let history = rig.session.document_snapshot().history;
        held.release();
        let saved = work.join().unwrap().unwrap();
        assert!(saved.ends_with("Song 2026-10-06 18-13-05 (002).windfall"));
        let final_root = file::sample_dir(std::path::Path::new(&saved)).unwrap();
        assert_eq!(rig.session.state().sample_dir.as_ref(), Some(&final_root));
        assert_eq!(final_root, rig.folder.path().join("away/Backup"));
        let written = file::load(&saved).unwrap();
        let expected_path = windfall_project::SamplePath::Project("sounds/tone (2).wav".into());
        assert_eq!(written.sample(sample).unwrap().path, expected_path);
        assert_eq!(rig.project().sample(sample).unwrap().path, expected_path);
        assert_eq!(
            fs::read(final_root.join("sounds/tone (2).wav")).unwrap(),
            fs::read(&source).unwrap()
        );
        assert_eq!(fs::read(&home).unwrap(), old);
        assert_eq!(fs::read(&backup).unwrap(), old_backup);
        assert_eq!(fs::read(&occupied).unwrap(), b"older version");
        assert_eq!(
            fs::read(rig.file("away/Backup/Song 2026-10-06 18-13-05.windfall")).unwrap(),
            b"old backup"
        );
        assert_eq!(fs::read(&collision).unwrap(), b"another sound");
        let snapshot = rig.session.document_snapshot();
        assert_eq!(snapshot.history, history);
        assert_eq!(snapshot.dirty, concurrent_edit);
        assert_eq!(written.settings.name, "Captured");
        assert_eq!(
            snapshot.project.settings.name,
            if concurrent_edit {
                "Concurrent"
            } else {
                "Captured"
            }
        );
        assert!(rig.has_audio(sample));
        assert!(rig.session.undo().is_some());
        assert!(rig.session.redo().is_some());
        assert_eq!(rig.project().sample(sample).unwrap().path, expected_path);
        fs::remove_file(&source).unwrap();
        let mut rig = rig.restart();
        rig.session.project_open(&saved).unwrap();
        assert!(rig.has_audio(sample));
        assert_eq!(
            rig.session.state().pool.get(sample).unwrap().samples(),
            audio.samples()
        );
        assert_eq!(rig.session.state().sample_dir.as_ref(), Some(&final_root));
        rig.session.transport_play().unwrap();
        assert!(rms(&rig.run(4800)) > 0.0, "the reopened sampler must play");
    }
}
