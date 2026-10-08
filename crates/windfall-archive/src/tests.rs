use super::*;
use std::io::Cursor;

/// A small version of the reviewed allocation bypass. The real directory has
/// two entries; the EOCD in its comment points at a ZIP64 count of 32. Tests
/// stop at preflight, never handing the untrusted count to ZIP's allocator.
fn commented_zip64_directory() -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for name in [MANIFEST, PROJECT] {
        writer
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"{}").unwrap();
    }
    let mut bytes = writer.finish().unwrap().into_inner();
    let end = bytes.len() - 22;
    let count = 32u64;
    let mut comment = vec![0; count as usize * 46];
    let zip64_at = bytes.len() + comment.len();
    comment.extend_from_slice(b"PK\x06\x06");
    comment.extend_from_slice(&44u64.to_le_bytes());
    comment.extend_from_slice(&45u16.to_le_bytes());
    comment.extend_from_slice(&45u16.to_le_bytes());
    comment.extend_from_slice(&0u32.to_le_bytes());
    comment.extend_from_slice(&0u32.to_le_bytes());
    comment.extend_from_slice(&count.to_le_bytes());
    comment.extend_from_slice(&count.to_le_bytes());
    comment.extend_from_slice(&0u64.to_le_bytes());
    comment.extend_from_slice(&0u64.to_le_bytes());
    comment.extend_from_slice(b"PK\x06\x07");
    comment.extend_from_slice(&0u32.to_le_bytes());
    comment.extend_from_slice(&(zip64_at as u64).to_le_bytes());
    comment.extend_from_slice(&1u32.to_le_bytes());
    let mut fake = [0; 22];
    fake[..4].copy_from_slice(b"PK\x05\x06");
    fake[8..12].copy_from_slice(&[255; 4]);
    comment.extend_from_slice(&fake);
    // The fake EOCD ends before EOF, so only the real footer is selected by
    // preflight's exact comment-boundary predicate.
    comment.push(0);
    bytes[end + 20..end + 22].copy_from_slice(&(comment.len() as u16).to_le_bytes());
    bytes.extend_from_slice(&comment);
    assert!(bytes.len() < 4096);
    bytes
}

#[test]
fn eocd_in_comment_zip64_is_rejected_before_index_initialization() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("comment.zip");
    let bytes = commented_zip64_directory();
    let mut input = Cursor::new(&bytes);
    let limits = Limits {
        entries: 2,
        ..Limits::default()
    };
    let mut check = |_| Ok(());
    assert!(
        preflight(&mut input, limits, &mut check).is_err(),
        "ambiguous comment must be refused before calling ZIP's index parser"
    );
    fs::write(&source, bytes).unwrap();
    let managed = root.path().join("managed");
    let mut extracting = false;
    let mut check = |stage| {
        extracting |= stage == Stage::Extract;
        Ok(())
    };
    assert!(extract(&source, &managed, limits, &mut check).is_err());
    assert!(!extracting);
    assert!(!managed.exists());
}

#[test]
fn malformed_directory_cannot_fall_back_to_an_embedded_eocd() {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for name in [MANIFEST, PROJECT] {
        writer
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"{}").unwrap();
    }
    let mut bytes = writer.finish().unwrap().into_inner();
    let end = bytes.len() - 22;
    let at = u32le(&bytes, end + 16) as usize;
    let directory_size = u32le(&bytes, end + 12);
    let after_name = at + 46 + u16le(&bytes, at + 28) as usize;
    let mut extra_and_comment = vec![0x01, 0x99, 0, 0]; // Invalid AES length.
    let mut alternate = [0; 22]; // ZIP accepts this empty alternate directory.
    alternate[..4].copy_from_slice(b"PK\x05\x06");
    extra_and_comment.extend_from_slice(&alternate);
    bytes[at + 30..at + 32].copy_from_slice(&4u16.to_le_bytes());
    bytes[at + 32..at + 34].copy_from_slice(&22u16.to_le_bytes());
    bytes.splice(after_name..after_name, extra_and_comment);
    let end = bytes.len() - 22;
    bytes[end + 12..end + 16].copy_from_slice(&(directory_size + 26).to_le_bytes());
    // A bounded, zero-entry demonstration of the locked parser's fallback.
    assert!(
        ZipArchive::new(Cursor::new(bytes.clone()))
            .unwrap()
            .is_empty()
    );
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("fallback.zip");
    fs::write(&source, &bytes).unwrap();
    let preflight = preflight(
        &mut File::open(&source).unwrap(),
        Limits::default(),
        &mut |_| Ok(()),
    )
    .unwrap();
    let error = ZipArchive::with_config(
        zip::read::Config {
            archive_offset: zip::read::ArchiveOffset::Known(0),
        },
        preflight.index,
    )
    .err()
    .unwrap();
    match error {
        zip::result::ZipError::Io(error) => {
            assert!(
                error.to_string().contains("fallback is forbidden"),
                "{error}"
            );
        }
        error => panic!("expected the fallback guard's IO error, got {error:?}"),
    }
    let managed = root.path().join("managed");
    assert!(extract(&source, &managed, Limits::default(), &mut |_| Ok(())).is_err());
    assert!(!managed.exists());
}

#[test]
fn zip64_member_overrides_are_refused_by_preflight() {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file(MANIFEST, SimpleFileOptions::default())
        .unwrap();
    writer.write_all(b"{}").unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    let end = bytes.len() - 22;
    let at = u32le(&bytes, end + 16) as usize;
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("zip64.zip");
    for field in [20, 24, 42] {
        let mut modified = bytes.clone();
        modified[at + field..at + field + 4].fill(255);
        fs::write(&source, modified).unwrap();
        assert!(
            preflight(
                &mut File::open(&source).unwrap(),
                Limits::default(),
                &mut |_| Ok(())
            )
            .is_err()
        );
    }
    let mut modified = bytes.clone();
    let directory_size = u32le(&bytes, end + 12);
    let after_name = at + 46 + u16le(&bytes, at + 28) as usize;
    let mut extra = vec![1, 0, 24, 0];
    extra.resize(28, 0);
    modified[at + 30..at + 32].copy_from_slice(&28u16.to_le_bytes());
    modified.splice(after_name..after_name, extra);
    let end = modified.len() - 22;
    modified[end + 12..end + 16].copy_from_slice(&(directory_size + 28).to_le_bytes());
    fs::write(&source, modified).unwrap();
    assert!(
        preflight(
            &mut File::open(&source).unwrap(),
            Limits::default(),
            &mut |_| Ok(())
        )
        .is_err()
    );
}

#[test]
fn index_uses_only_the_validated_snapshot_across_search_windows() {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for number in 0..60 {
        writer
            .start_file(
                format!("audio/{number:04}.wav"),
                SimpleFileOptions::default(),
            )
            .unwrap();
        writer.write_all(b"data").unwrap();
    }
    let bytes = writer.finish().unwrap().into_inner();
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("snapshot.zip");
    fs::write(&source, bytes).unwrap();
    let mut input = File::open(&source).unwrap();
    let Preflight { members, index } =
        preflight(&mut input, Limits::default(), &mut |_| Ok(())).unwrap();
    assert!(index.directory_end > 2048);
    // Any accidental original-file metadata read would now fail. Metadata
    // remains immutable even if an external writer changes the source.
    fs::write(&source, []).unwrap();
    let initialized = Cell::new(false);
    let zip = ZipArchive::with_config(
        zip::read::Config {
            archive_offset: zip::read::ArchiveOffset::Known(0),
        },
        ArchiveReader {
            input,
            index,
            initialized: &initialized,
        },
    )
    .unwrap();
    assert_eq!(zip.len(), members.len());
    assert_eq!(zip.len(), 60);
    assert!(!initialized.get());
}
