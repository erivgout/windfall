//! Checks the files the generator writes, and that the factory folder
//! checked into the repository is exactly what it writes.

use std::fs;
use std::path::{Path, PathBuf};

use windfall_factory::{BITS_PER_SAMPLE, Mismatch, SAMPLE_RATE, generate, manifest, verify};

/// A fresh, empty folder under the build's temporary directory.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn checked_in_folder_matches_the_generator() {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/factory");
    let mismatches = verify(&folder).unwrap();
    assert!(
        mismatches.is_empty(),
        "content/factory is out of date; run \
         `cargo run -p windfall-factory --release -- generate content/factory`: {mismatches:?}"
    );
}

#[test]
fn generated_folder_verifies_and_holds_every_sound() {
    let dir = scratch("factory-generate");
    generate(&dir).unwrap();
    assert_eq!(verify(&dir).unwrap(), []);
    for sound in manifest() {
        assert!(dir.join(sound.path).is_file(), "{}", sound.path);
    }
    let license = fs::read_to_string(dir.join("LICENSE.md")).unwrap();
    assert!(license.contains("https://creativecommons.org/publicdomain/zero/1.0/"));
    assert!(license.contains("crates/windfall-factory"));
    assert!(dir.join("README.md").is_file());
}

#[test]
fn verify_reports_missing_changed_and_stray_files() {
    let dir = scratch("factory-tamper");
    generate(&dir).unwrap();

    let [removed, changed] = [manifest()[0].path, manifest()[1].path];
    fs::remove_file(dir.join(removed)).unwrap();
    let mut bytes = fs::read(dir.join(changed)).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    fs::write(dir.join(changed), bytes).unwrap();
    fs::write(dir.join("Drums/Kicks/Extra.wav"), b"stray").unwrap();

    let mut mismatches = verify(&dir).unwrap();
    mismatches.sort_by_key(|mismatch| mismatch.to_string());
    assert_eq!(
        mismatches,
        [
            Mismatch::Different(changed.to_owned()),
            Mismatch::Missing(removed.to_owned()),
            Mismatch::Unexpected("Drums/Kicks/Extra.wav".to_owned()),
        ]
    );
}

#[test]
fn verify_ignores_line_endings_in_the_text_files() {
    let dir = scratch("factory-crlf");
    generate(&dir).unwrap();
    let readme = fs::read_to_string(dir.join("README.md")).unwrap();
    assert!(!readme.contains('\r'));
    fs::write(dir.join("README.md"), readme.replace('\n', "\r\n")).unwrap();
    assert_eq!(verify(&dir).unwrap(), []);

    fs::write(dir.join("README.md"), readme.replace("Windfall", "Other")).unwrap();
    assert_eq!(
        verify(&dir).unwrap(),
        [Mismatch::Different("README.md".to_owned())]
    );
}

#[test]
fn verify_fails_on_a_folder_that_does_not_exist() {
    let dir = scratch("factory-missing").join("nowhere");
    assert!(verify(&dir).is_err());
}

#[test]
fn wav_files_decode_with_an_independent_reader() {
    for sound in manifest() {
        let rendered = sound.render();
        let wav = rendered.to_wav();
        let mut reader = hound::WavReader::new(wav.as_slice()).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.sample_rate, SAMPLE_RATE, "{}", sound.name);
        assert_eq!(spec.bits_per_sample, BITS_PER_SAMPLE, "{}", sound.name);
        assert_eq!(spec.sample_format, hound::SampleFormat::Int);
        assert_eq!(spec.channels, rendered.channels(), "{}", sound.name);
        let samples: Vec<i32> = reader.samples::<i32>().map(Result::unwrap).collect();
        assert_eq!(samples, rendered.samples(), "{}", sound.name);
    }
}
