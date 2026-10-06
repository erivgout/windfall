//! The browser panel and sample information.

use std::fs;

use windfall_ipc::{BrowserEntryKind, BrowserRootKind};
use windfall_project::{Command, SampleId, SamplePath};

use super::{Rig, factory_file};
use crate::paths;
use crate::samples::PEAK_BUCKETS;

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
