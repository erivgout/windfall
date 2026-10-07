use super::{Rig, still_running};
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
                barrier.wait();
                crate::session::versions::write(&project, &played, &base).unwrap()
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
