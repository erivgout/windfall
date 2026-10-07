use std::{
    fs,
    io::{Cursor, Write},
    path::Path,
};
use windfall_archive::{self as archive, Error, Limits, Stage};
use windfall_project::{Command, Document, Project, SamplePath, file};
use zip::{ZipWriter, write::SimpleFileOptions};

fn project(root: &Path) -> Project {
    fs::write(root.join("one.wav"), b"same bytes").unwrap();
    fs::write(root.join("two.wav"), b"same bytes").unwrap();
    let mut document = Document::new(Project::new("Archive"));
    for name in ["one.wav", "two.wav"] {
        document
            .dispatch(
                Command::AddSample {
                    name: name.into(),
                    path: SamplePath::Project(name.into()),
                },
                None,
            )
            .unwrap();
    }
    document.project().clone()
}
fn keep_going(_: Stage) -> Result<(), Error> {
    Ok(())
}
fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        zip.start_file(*name, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}
#[test]
fn exact_dedup_snapshot_and_original_metadata_round_trip() {
    let root = tempfile::tempdir().unwrap();
    let project = project(root.path());
    let target = root.path().join("portable.zip");
    archive::create(
        &project,
        None,
        Some(root.path()),
        root.path(),
        &target,
        Limits::default(),
        &mut keep_going,
    )
    .unwrap();
    fs::remove_file(root.path().join("one.wav")).unwrap();
    fs::remove_file(root.path().join("two.wav")).unwrap();
    let extracted = archive::extract(
        &target,
        &root.path().join("managed"),
        Limits::default(),
        &mut keep_going,
    )
    .unwrap();
    assert_ne!(
        extracted.project.samples[0].path,
        extracted.project.samples[1].path
    );
    let path = file::resolve_sample_path(
        &extracted.project.samples[0].path,
        Some(extracted.root()),
        root.path(),
    )
    .unwrap();
    assert_eq!(fs::read(path).unwrap(), b"same bytes");
    let manifest: archive::Manifest =
        serde_json::from_slice(&fs::read(extracted.root().join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest.audio[0].original_path, project.samples[0].path);
    assert_eq!(manifest.audio[1].original_name, "two.wav");
    assert_eq!(manifest.audio[0].member, manifest.audio[1].member);
    assert_eq!(
        fs::read_dir(extracted.root().join("audio"))
            .unwrap()
            .count(),
        1
    );
    let owned = extracted.root().to_path_buf();
    drop(extracted);
    assert!(!owned.exists());
}
#[test]
fn missing_sources_are_all_reported_without_publishing() {
    let root = tempfile::tempdir().unwrap();
    let project = project(root.path());
    fs::remove_file(root.path().join("one.wav")).unwrap();
    fs::remove_file(root.path().join("two.wav")).unwrap();
    let target = root.path().join("missing.zip");
    let error = archive::create(
        &project,
        None,
        Some(root.path()),
        root.path(),
        &target,
        Limits::default(),
        &mut keep_going,
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("one.wav") && error.contains("two.wav"));
    assert!(!target.exists());
}
#[test]
fn cancellation_and_injected_failures_clean_only_request_staging() {
    for stage in [Stage::Source, Stage::Write, Stage::Publish] {
        for cancellation in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let project = project(root.path());
            let target = root.path().join("portable.zip");
            fs::write(root.path().join("unrelated"), b"keep").unwrap();
            let mut check = |at| {
                if at != stage {
                    Ok(())
                } else if cancellation {
                    Err(Error::Cancelled)
                } else {
                    Err(Error::Io(std::io::Error::other("injected IO failure")))
                }
            };
            assert!(
                archive::create(
                    &project,
                    None,
                    Some(root.path()),
                    root.path(),
                    &target,
                    Limits::default(),
                    &mut check
                )
                .is_err()
            );
            assert!(!target.exists());
            assert_eq!(fs::read_dir(root.path()).unwrap().count(), 3);
        }
    }
    for stage in [Stage::Read, Stage::Extract, Stage::Ready] {
        let root = tempfile::tempdir().unwrap();
        let project = project(root.path());
        let target = root.path().join("portable.zip");
        archive::create(
            &project,
            None,
            Some(root.path()),
            root.path(),
            &target,
            Limits::default(),
            &mut keep_going,
        )
        .unwrap();
        let managed = root.path().join("managed");
        fs::create_dir(&managed).unwrap();
        fs::write(managed.join("unrelated"), b"keep").unwrap();
        for cancellation in [false, true] {
            let mut check = |at| {
                if at != stage {
                    Ok(())
                } else if cancellation {
                    Err(Error::Cancelled)
                } else {
                    Err(Error::Io(std::io::Error::other(
                        "injected read/write failure",
                    )))
                }
            };
            assert!(archive::extract(&target, &managed, Limits::default(), &mut check).is_err());
            assert_eq!(fs::read_dir(&managed).unwrap().count(), 1);
        }
    }
}
#[test]
fn competing_target_is_never_replaced() {
    let root = tempfile::tempdir().unwrap();
    let project = project(root.path());
    let target = root.path().join("portable.zip");
    let mut check = |stage| {
        if stage == Stage::Publish {
            fs::write(&target, b"competitor")?;
        }
        Ok(())
    };
    assert!(
        archive::create(
            &project,
            None,
            Some(root.path()),
            root.path(),
            &target,
            Limits::default(),
            &mut check
        )
        .is_err()
    );
    assert_eq!(fs::read(target).unwrap(), b"competitor");
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 3);
}
#[test]
fn source_change_during_staging_refuses_publication() {
    let root = tempfile::tempdir().unwrap();
    let project = project(root.path());
    let target = root.path().join("portable.zip");
    let mut changed = false;
    let mut check = |stage| {
        if stage == Stage::Source && !changed {
            changed = true;
            // The source is already open and its initial size captured.
            // A writer adds bytes before the first bounded read.
            fs::OpenOptions::new()
                .append(true)
                .open(root.path().join("one.wav"))?
                .write_all(b" changed")?;
        }
        Ok(())
    };
    let error = archive::create(
        &project,
        None,
        Some(root.path()),
        root.path(),
        &target,
        Limits::default(),
        &mut check,
    )
    .unwrap_err();
    assert!(error.to_string().contains("changed while packaging"));
    assert!(!target.exists());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
}
#[test]
fn unsafe_zip_names_case_collisions_types_and_limits_are_rejected_before_destination_creation() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("bad.zip");
    let managed = root.path().join("managed");
    for name in [
        "../escape",
        "/absolute",
        "C:/drive",
        "//host/share",
        "a\\b",
        "NUL.wav",
        "audio/trailing.",
        "audio/a:b",
        "audio/../escape",
        "Audio/0001.wav",
    ] {
        fs::write(&source, zip(&[(name, b"x")])).unwrap();
        assert!(
            archive::extract(&source, &managed, Limits::default(), &mut keep_going).is_err(),
            "{name}"
        );
        assert!(!managed.exists());
    }
    fs::write(&source, zip(&[("audio/a", b"x"), ("audio/A", b"y")])).unwrap();
    assert!(archive::extract(&source, &managed, Limits::default(), &mut keep_going).is_err());
    // Central-directory mode marks a symbolic link, independent of its content.
    let mut bytes = zip(&[("audio/link", b"../escape")]);
    let at = bytes.windows(4).position(|s| s == b"PK\x01\x02").unwrap();
    bytes[at + 38..at + 42].copy_from_slice(&(0o120777u32 << 16).to_le_bytes());
    fs::write(&source, bytes).unwrap();
    assert!(archive::extract(&source, &managed, Limits::default(), &mut keep_going).is_err());
    fs::write(&source, zip(&[("large", &[0; 256])])).unwrap();
    for limits in [
        Limits {
            entries: 0,
            ..Limits::default()
        },
        Limits {
            entry_bytes: 128,
            ..Limits::default()
        },
        Limits {
            expanded_bytes: 128,
            ..Limits::default()
        },
        Limits {
            archive_bytes: 1,
            ..Limits::default()
        },
    ] {
        assert!(archive::extract(&source, &managed, limits, &mut keep_going).is_err());
        assert!(!managed.exists());
    }
}
#[test]
fn highly_compressed_audio_is_bounded_before_destination_creation() {
    let root = tempfile::tempdir().unwrap();
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file(
            "audio/0001.wav",
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated),
        )
        .unwrap();
    writer.write_all(&vec![0; 1024 * 1024]).unwrap();
    let zip = writer.finish().unwrap().into_inner();
    assert!(zip.len() < 2048);
    let source = root.path().join("bomb.zip");
    fs::write(&source, zip).unwrap();
    let managed = root.path().join("managed");
    let limits = Limits {
        entry_bytes: 512 * 1024,
        ..Limits::default()
    };
    let error = archive::extract(&source, &managed, limits, &mut keep_going)
        .err()
        .unwrap();
    assert!(error.to_string().contains("size limit"));
    assert!(!managed.exists());
}
#[test]
fn schema_missing_members_external_paths_crc_and_truncation_fail_closed() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("bad.zip");
    let managed = root.path().join("managed");
    let project = project(root.path());
    let json = file::to_json(&project).unwrap();
    for manifest in [
        br#"{"schema":99,"project":"project.windfall","audio":[]}"#.as_slice(),
        br#"{"schema":1,"project":"project.windfall","audio":[]}"#.as_slice(),
    ] {
        fs::write(
            &source,
            zip(&[
                ("manifest.json", manifest),
                ("project.windfall", json.as_bytes()),
            ]),
        )
        .unwrap();
        assert!(archive::extract(&source, &managed, Limits::default(), &mut keep_going).is_err());
        assert!(!managed.exists());
    }
    archive::create(
        &project,
        None,
        Some(root.path()),
        root.path(),
        &source.with_extension("good.zip"),
        Limits::default(),
        &mut keep_going,
    )
    .unwrap();
    let good = fs::read(source.with_extension("good.zip")).unwrap();
    fs::write(&source, &good[..good.len() - 10]).unwrap();
    assert!(archive::extract(&source, &managed, Limits::default(), &mut keep_going).is_err());
    let mut damaged = good;
    let at = damaged.windows(4).position(|s| s == b"PK\x01\x02").unwrap();
    damaged[at + 16] ^= 1;
    fs::write(&source, damaged).unwrap();
    assert!(archive::extract(&source, &managed, Limits::default(), &mut keep_going).is_err());
}
