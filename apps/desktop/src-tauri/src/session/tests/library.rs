//! The browser panel and sample information.

use std::fs;

use windfall_ipc::{BrowserEntryKind, BrowserRootKind};
use windfall_project::{Command, SampleId, SamplePath};

use super::{Rig, factory_file};
use crate::paths;
use crate::samples::PEAK_BUCKETS;
use crate::session::PROJECT_REPLACED;

fn checked_import(
    rig: &Rig,
    path: &str,
    token: windfall_ipc::LibraryFileToken,
    destination: &str,
) -> Result<windfall_project::DispatchResult, String> {
    match destination {
        "rack" => rig.session.browser_add_channel(path, None, token),
        "playlist" => rig.session.browser_add_clip(
            path,
            crate::session::ClipPlace {
                track: None,
                start: 0,
                mixer_track: None,
            },
            token,
        ),
        "replacement" => {
            rig.session
                .browser_replace_sample(rig.project().channels[0].id, path, token)
        }
        _ => panic!("unknown destination"),
    }
}

#[test]
fn checked_import_refuses_reusing_a_loaded_older_file_version_at_every_destination() {
    use windfall_codec::{WavSampleFormat, write_wav};
    use windfall_core::AudioBuffer;

    for destination in ["rack", "playlist", "replacement"] {
        let rig = Rig::new();
        let samples = rig.folder.path().join("Samples");
        fs::create_dir(&samples).unwrap();
        let file = samples.join("tone.wav");
        let write = |frames, level| {
            write_wav(
                &file,
                &AudioBuffer::from_interleaved(48_000, 1, vec![level; frames]),
                WavSampleFormat::Float32,
            )
            .unwrap();
        };
        write(480, 0.25);
        let path = paths::display(&file);
        rig.session
            .browser_add_root(&paths::display(&samples))
            .unwrap();
        let token = rig.session.library_file(&path).unwrap();
        let first = rig
            .session
            .browser_add_channel(&path, None, token.clone())
            .unwrap();
        let sample = SampleId(first.created[0]);
        let old = rig.session.state().pool.get(sample).unwrap().clone();
        // Unchanged imports must deduplicate and undo as one normal edit.
        rig.session.browser_add_channel(&path, None, token).unwrap();
        assert_eq!(
            rig.project()
                .samples
                .iter()
                .filter(|s| s.id == sample)
                .count(),
            1
        );
        rig.session.undo().unwrap();
        write(960, 0.75);
        rig.session.library_refresh();
        let token = rig.session.library_file(&path).unwrap();
        assert_eq!(
            rig.session
                .browser_sample_info(&path, &token)
                .unwrap()
                .frames,
            960
        );
        rig.session.browser_preview(&path, &token).unwrap();
        let before = rig.session.document_snapshot();
        let result = checked_import(&rig, &path, token, destination);
        assert!(
            result.is_err(),
            "{destination} accepted new audio but retained {} old frames",
            rig.session.state().pool.get(sample).unwrap().frames()
        );
        assert_eq!(rig.session.document_snapshot(), before);
        assert_eq!(
            rig.session.state().pool.get(sample).unwrap().samples(),
            old.samples()
        );
        assert!(result.unwrap_err().contains("older or unverified version"));

        // Saving does not persist an unundoable source replacement. Reopening
        // deliberately reloads linked external audio from its current version.
        let saved = rig
            .session
            .project_save(Some(&rig.file("Saved/song")))
            .unwrap();
        rig.session.project_open(&saved).unwrap();
        assert_eq!(rig.session.state().pool.get(sample).unwrap().frames(), 960);
        assert_eq!(
            rig.session.state().pool.get(sample).unwrap().samples()[0],
            0.75
        );
        let before = rig.project();
        let token = rig.session.library_file(&path).unwrap();
        checked_import(&rig, &path, token, destination).unwrap();
        assert_eq!(rig.project().samples.len(), before.samples.len());
        if destination == "playlist" {
            let clip = rig.project().playlist.clips.last().unwrap().clone();
            let ticks = (0.02 * before.settings.tempo_bpm * f64::from(windfall_core::PPQ) / 60.0)
                .ceil() as u32;
            assert_eq!(clip.length, ticks);
        }
        rig.session.undo().unwrap();
        let mut current = rig.project();
        current.next_id = before.next_id;
        assert_eq!(current, before);
        assert_eq!(rig.session.state().pool.get(sample).unwrap().frames(), 960);
    }
}

#[test]
fn checked_unchanged_imports_deduplicate_after_decoded_cache_eviction() {
    use windfall_codec::{WavSampleFormat, write_wav};
    use windfall_core::AudioBuffer;
    let rig = Rig::new();
    let folder = rig.folder.path().join("Samples");
    fs::create_dir(&folder).unwrap();
    let file = folder.join("tone.wav");
    let audio = AudioBuffer::from_interleaved(48_000, 1, vec![0.25; 480]);
    write_wav(&file, &audio, WavSampleFormat::Float32).unwrap();
    rig.session
        .browser_add_root(&paths::display(&folder))
        .unwrap();
    let path = paths::display(&file);
    let first = rig
        .session
        .browser_add_channel(&path, None, rig.session.library_file(&path).unwrap())
        .unwrap();
    let sample = SampleId(first.created[0]);
    let original = rig.session.state().pool.get(sample).unwrap().identity();
    for i in 0..65 {
        let other = folder.join(format!("{i}.wav"));
        write_wav(&other, &audio, WavSampleFormat::Float32).unwrap();
        rig.session.inner.cache.decode(&other).unwrap();
    }
    assert!(rig.session.inner.cache.peek(&file).is_none());
    for destination in ["rack", "playlist", "replacement"] {
        let before = rig.project();
        checked_import(
            &rig,
            &path,
            rig.session.library_file(&path).unwrap(),
            destination,
        )
        .unwrap();
        assert_eq!(rig.project().samples.len(), before.samples.len());
        assert_eq!(
            rig.session.state().pool.get(sample).unwrap().identity(),
            original
        );
        rig.session.undo().unwrap();
        let mut current = rig.project();
        current.next_id = before.next_id;
        assert_eq!(current, before);
    }
}

#[test]
fn checked_imports_keep_native_project_guards_across_new_and_open_at_every_destination() {
    for change in ["New", "Open"] {
        for destination in ["rack", "playlist", "replacement"] {
            let rig = Rig::new();
            let saved = rig
                .session
                .project_save(Some(&rig.file("Saved/song")))
                .unwrap();
            let path = factory_file("Drums/Kicks/Kick Punch.wav");
            let token = rig.session.library_file(&path).unwrap();
            let channel = rig.project().channels[0].id;
            let hold = rig.session.hold("import:decoded");
            let session = rig.session.clone();
            let job = std::thread::spawn(move || match destination {
                "rack" => session.browser_add_channel(&path, None, token),
                "playlist" => session.browser_add_clip(
                    &path,
                    crate::session::ClipPlace {
                        track: None,
                        start: 0,
                        mixer_track: None,
                    },
                    token,
                ),
                _ => session.browser_replace_sample(channel, &path, token),
            });
            hold.wait();
            if change == "New" {
                rig.session.project_new().unwrap();
            } else {
                rig.session.project_open(&saved).unwrap();
            }
            assert_eq!(rig.project().channels[0].id, channel);
            let before = rig.session.document_snapshot();
            drop(hold);
            assert!(job.join().unwrap().is_err());
            assert_eq!(rig.session.document_snapshot(), before);
        }
    }
}

#[test]
fn library_results_audition_import_undo_and_metadata_restart_use_the_existing_loader() {
    let rig = Rig::new();
    let file = factory_file("Drums/Kicks/Kick Punch.wav");
    let token = rig.session.library_file(&file).unwrap();
    let before = rig.project();
    rig.session.browser_sample_info(&file, &token).unwrap();
    rig.session.browser_preview(&file, &token).unwrap();
    let result = rig
        .session
        .browser_add_channel(&file, None, token.clone())
        .unwrap();
    assert_eq!(result.created.len(), 3);
    assert_eq!(rig.project().channels.len(), before.channels.len() + 1);
    rig.session.undo().unwrap();
    let mut current = rig.project();
    current.next_id = before.next_id;
    assert_eq!(current, before);
    let metadata = windfall_ipc::LibraryMetadata {
        favorite: true,
        tags: vec!["punchy".into()],
    };
    rig.session
        .library_set_metadata(&file, metadata.clone())
        .unwrap();
    // Favorites and tags never dirty the project or add history.
    let mut current = rig.project();
    current.next_id = before.next_id;
    assert_eq!(current, before);
    let rig = rig.restart();
    assert_eq!(rig.session.library_metadata(&file).unwrap(), metadata);
    // Cached decoded audio cannot rescue a deleted library file.
    let samples = rig.folder.path().join("Samples");
    fs::create_dir(&samples).unwrap();
    let copied = samples.join("kick.wav");
    fs::copy(&file, &copied).unwrap();
    rig.session
        .browser_add_root(&paths::display(&samples))
        .unwrap();
    let path = paths::display(&copied);
    let token = rig.session.library_file(&path).unwrap();
    rig.session.browser_preview(&path, &token).unwrap();
    fs::remove_file(&copied).unwrap();
    assert!(rig.session.browser_add_channel(&path, None, token).is_err());
    let mut current = rig.project();
    current.next_id = before.next_id;
    assert_eq!(current, before);
}

#[test]
fn root_removal_during_decode_prevents_library_import_even_when_audio_is_cached() {
    let rig = Rig::new();
    let samples = rig.folder.path().join("Samples");
    fs::create_dir(&samples).unwrap();
    let file = samples.join("kick.wav");
    fs::copy(factory_file("Drums/Kicks/Kick Punch.wav"), &file).unwrap();
    let root = paths::display(&samples);
    let path = paths::display(&file);
    rig.session.browser_add_root(&root).unwrap();
    let token = rig.session.library_file(&path).unwrap();
    let before = rig.project();
    let hold = rig.session.hold("import:decoded");
    let session = rig.session.clone();
    let job = std::thread::spawn(move || session.browser_add_channel(&path, None, token));
    hold.wait();
    rig.session.browser_remove_root(&root).unwrap();
    drop(hold);
    assert!(job.join().unwrap().is_err());
    let mut current = rig.project();
    current.next_id = before.next_id;
    assert_eq!(current, before);
}

#[test]
fn guarded_library_clip_and_replace_imports_preserve_document_guards() {
    let rig = Rig::new();
    let file = factory_file("Drums/Kicks/Kick Punch.wav");
    let token = rig.session.library_file(&file).unwrap();
    let channel = rig.project().channels[0].id;
    rig.session
        .browser_replace_sample(channel, &file, token.clone())
        .unwrap();
    let before = rig.project();
    rig.session
        .browser_add_clip(
            &file,
            crate::session::ClipPlace {
                track: None,
                start: 0,
                mixer_track: None,
            },
            token,
        )
        .unwrap();
    assert_eq!(
        rig.project().playlist.clips.len(),
        before.playlist.clips.len() + 1
    );
    rig.session.undo().unwrap();
    let mut current = rig.project();
    current.next_id = before.next_id;
    assert_eq!(current, before);
    let token = rig.session.library_file(&file).unwrap();
    let hold = rig.session.hold("import:decoded");
    let session = rig.session.clone();
    let job = std::thread::spawn(move || session.browser_add_channel(&file, None, token));
    hold.wait();
    rig.session.project_new().unwrap();
    drop(hold);
    assert!(job.join().unwrap().is_err());
}

#[test]
fn checked_library_imports_refuse_an_active_synthetic_take_without_changing_it() {
    struct Capture;
    impl crate::session::recording::CaptureHandle for Capture {
        fn frames(&self) -> u64 {
            0
        }
        fn failed(&self) -> bool {
            false
        }
        fn finish(self: Box<Self>) -> Result<u64, String> {
            Ok(0)
        }
    }
    let rig = Rig::new();
    let file = factory_file("Drums/Kicks/Kick Punch.wav");
    let token = rig.session.library_file(&file).unwrap();
    let before = rig.project();
    let capture = rig
        .session
        .recording_start_with(
            windfall_ipc::RecordingSource {
                host: "Synthetic".into(),
                device: "Synthetic".into(),
                left: 0,
                right: None,
            },
            0,
            None,
            |_, _, _| Ok(Box::new(Capture)),
        )
        .unwrap();
    assert!(
        rig.session
            .browser_add_channel(&file, None, token.clone())
            .is_err()
    );
    assert!(
        rig.session
            .browser_replace_sample(before.channels[0].id, &file, token.clone())
            .is_err()
    );
    assert!(
        rig.session
            .browser_add_clip(
                &file,
                crate::session::ClipPlace {
                    track: None,
                    start: 0,
                    mixer_track: None
                },
                token
            )
            .is_err()
    );
    assert_eq!(rig.project(), before);
    assert_eq!(rig.session.recording_state(), capture);
    rig.session.recording_cancel();
}

#[test]
fn the_factory_content_is_the_first_root_and_lists_as_folders_of_sounds() {
    let rig = Rig::new();
    let roots = rig.session.browser_roots();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].name, "Factory");
    assert_eq!(roots[0].kind, BrowserRootKind::Factory);
    assert_eq!(roots[0].path, paths::display(rig.session.factory_dir()));
    assert_eq!(roots[0].path, factory_file(""));

    let top = rig.session.browser_list(&roots[0].path).unwrap();
    let summary: Vec<(&str, BrowserEntryKind)> = top
        .iter()
        .map(|entry| (entry.name.as_str(), entry.kind))
        .collect();
    assert_eq!(
        summary,
        [
            ("Bass", BrowserEntryKind::Folder),
            ("Drums", BrowserEntryKind::Folder),
            ("LICENSE.md", BrowserEntryKind::Other),
            ("README.md", BrowserEntryKind::Other),
        ]
    );

    let hats = rig
        .session
        .browser_list(&factory_file("Drums/Hats"))
        .unwrap();
    let names: Vec<&str> = hats.iter().map(|entry| entry.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Hat Closed 1.wav",
            "Hat Closed 2.wav",
            "Hat Closed 3.wav",
            "Hat Open 1.wav",
            "Hat Open 2.wav",
            "Hat Pedal.wav",
        ]
    );
    assert!(
        hats.iter()
            .all(|entry| entry.kind == BrowserEntryKind::Audio)
    );
    // What the browser lists is what the other calls take.
    assert_eq!(hats[0].path, factory_file("Drums/Hats/Hat Closed 1.wav"));
    assert!(rig.session.sample_info(&hats[0].path).is_ok());

    let error = rig.session.browser_list(&rig.file("gone")).unwrap_err();
    assert!(error.contains("does not exist"), "{error}");
    assert!(rig.session.browser_list("Drums").is_err());
}

#[test]
fn user_folders_can_be_added_and_removed_and_are_remembered() {
    let rig = Rig::new();
    let session = &rig.session;
    let kits = rig.folder.path().join("My Kits");
    let loops = rig.folder.path().join("Loops");
    fs::create_dir_all(&kits).unwrap();
    fs::create_dir_all(&loops).unwrap();
    let (kits, loops) = (paths::display(&kits), paths::display(&loops));

    // A trailing separator is not part of the folder's name.
    let roots = session
        .browser_add_root(&format!("{kits}{}", std::path::MAIN_SEPARATOR))
        .unwrap();
    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0].kind, BrowserRootKind::Factory);
    assert_eq!(
        (
            roots[1].name.as_str(),
            roots[1].path.as_str(),
            roots[1].kind
        ),
        ("My Kits", kits.as_str(), BrowserRootKind::User)
    );
    let roots = session.browser_add_root(&loops).unwrap();
    assert_eq!(roots[2].name, "Loops");

    let error = session.browser_add_root(&kits).unwrap_err();
    assert!(error.contains("already in the browser"), "{error}");
    let error = session.browser_add_root(&roots[0].path).unwrap_err();
    assert!(error.contains("already in the browser"), "{error}");
    let error = session.browser_add_root(&rig.file("nowhere")).unwrap_err();
    assert!(error.contains("is not a folder"), "{error}");
    assert_eq!(
        session.browser_add_root(" ").unwrap_err(),
        "Choose a folder to add."
    );

    let error = session.browser_remove_root(&roots[0].path).unwrap_err();
    assert_eq!(error, "The factory library cannot be removed.");
    let error = session
        .browser_remove_root(&rig.file("nowhere"))
        .unwrap_err();
    assert!(error.contains("is not in the browser"), "{error}");
    assert_eq!(session.browser_roots(), roots);

    let rig = rig.restart();
    assert_eq!(rig.session.browser_roots(), roots);
    let left = rig.session.browser_remove_root(&kits).unwrap();
    assert_eq!(left.len(), 2);
    assert_eq!(left[1].path, loops);
    // The folder itself is not touched.
    assert!(std::path::Path::new(&kits).is_dir());
    let rig = rig.restart();
    assert_eq!(rig.session.browser_roots(), left);
}

#[test]
fn sample_info_describes_a_file_with_a_waveform_overview() {
    let rig = Rig::new();
    let file = factory_file("Drums/Kicks/Kick Punch.wav");

    let info = rig.session.sample_info(&file).unwrap();
    assert_eq!(info.path, file);
    assert_eq!(info.name, "Kick Punch");
    assert_eq!(info.sample_rate, windfall_factory::SAMPLE_RATE);
    assert!(info.channels >= 1);
    assert!(info.frames > 1_000);
    let seconds = info.frames as f64 / f64::from(info.sample_rate);
    assert!((info.duration_secs - seconds).abs() < 1e-9);

    assert_eq!(info.peaks.len(), PEAK_BUCKETS * 2);
    let (pairs, _) = info.peaks.as_chunks::<2>();
    assert!(pairs.iter().all(|[low, high]| low <= high));
    // A kick is loud at the start and has died away by the end.
    let height = |pair: &[f32; 2]| pair[1] - pair[0];
    assert!(pairs[..32].iter().map(height).fold(0.0, f32::max) > 0.5);
    assert!(height(&pairs[PEAK_BUCKETS - 1]) < 0.05);

    assert_eq!(rig.session.sample_info(&file).unwrap(), info);

    let error = rig.session.sample_info(&factory_file("Drums")).unwrap_err();
    assert!(error.contains("is a folder"), "{error}");
    let error = rig
        .session
        .sample_info(&factory_file("README.md"))
        .unwrap_err();
    assert!(error.contains("README.md"), "{error}");
    assert!(rig.session.sample_info("Kick Punch.wav").is_err());
}

#[test]
fn sample_info_by_id_resolves_the_samples_of_the_project() {
    let rig = Rig::new();
    let session = &rig.session;
    let project = rig.project();

    // A factory sample is stored relative to the factory folder; the
    // result names the file in full.
    let kick = &project.samples[0];
    assert_eq!(
        kick.path,
        SamplePath::Factory("Drums/Kicks/Kick Punch.wav".to_owned())
    );
    let info = session.sample_info_by_id(kick.id).unwrap();
    let file = factory_file("Drums/Kicks/Kick Punch.wav");
    assert_eq!(info.path, file);
    assert_eq!(info, session.sample_info(&file).unwrap());

    assert_eq!(
        session.sample_info_by_id(SampleId(999)).unwrap_err(),
        "sample 999 does not exist"
    );

    // In the pool, but its file is not there.
    let missing = rig.file("missing.wav");
    let added = session
        .dispatch(
            Command::AddSample {
                name: "Missing".to_owned(),
                path: SamplePath::External(missing),
            },
            None,
        )
        .unwrap();
    let error = session
        .sample_info_by_id(SampleId(added.created[0]))
        .unwrap_err();
    assert!(error.starts_with("Could not load"), "{error}");
    assert!(error.contains("missing.wav"), "{error}");

    // A project sample of a project with no folder cannot be found at all.
    let added = session
        .dispatch(
            Command::AddSample {
                name: "Homeless".to_owned(),
                path: SamplePath::Project("sounds/own.wav".to_owned()),
            },
            None,
        )
        .unwrap();
    assert_eq!(
        session
            .sample_info_by_id(SampleId(added.created[0]))
            .unwrap_err(),
        "Missing sample: sounds/own.wav"
    );
}

#[test]
fn sample_info_for_a_project_that_is_gone_is_not_handed_out() {
    let rig = Rig::new();
    let session = &rig.session;
    let kick = rig.project().samples[0].id;

    let hold = session.hold("sample-info:read");
    let asking = session.background(move |session| session.sample_info_by_id(kick));
    hold.wait();
    // The id means a sample of the new project now.
    session.project_new().unwrap();
    hold.release();
    assert_eq!(asking.join().unwrap().unwrap_err(), PROJECT_REPLACED);

    assert_eq!(session.sample_info_by_id(kick).unwrap().name, "Kick Punch");
}
