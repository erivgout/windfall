//! How `WavWriter` treats the destination file and bad requests.

use std::fs;

use windfall_codec::{CodecError, WavSampleFormat, WavWriter, write_wav};
use windfall_core::AudioBuffer;

use crate::common::*;

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

#[test]
fn nothing_appears_at_the_destination_before_finalize() {
    let dir = TempDir::new("atomic");
    let path = dir.path("out.wav");
    let mut writer = WavWriter::create(&path, 48_000, 2, WavSampleFormat::Int16).unwrap();
    writer.write(&[0.1, 0.2, 0.3, 0.4]).unwrap();
    assert!(!path.exists());
    assert_eq!(dir.entries().len(), 1);

    writer.finalize().unwrap();
    assert_eq!(dir.entries(), ["out.wav"]);
}

#[test]
fn dropping_the_writer_leaves_the_old_file_alone() {
    let dir = TempDir::new("abandon");
    let path = dir.path("out.wav");
    fs::write(&path, b"the previous export").unwrap();

    let mut writer = WavWriter::create(&path, 48_000, 1, WavSampleFormat::Float32).unwrap();
    writer.write(&[0.1, 0.2, 0.3]).unwrap();
    drop(writer);

    assert_eq!(dir.entries(), ["out.wav"]);
    assert_eq!(fs::read(&path).unwrap(), b"the previous export");
}

#[test]
fn finalize_replaces_the_old_file() {
    let dir = TempDir::new("replace");
    let path = dir.path("out.wav");
    fs::write(&path, b"the previous export").unwrap();

    let buffer = AudioBuffer::from_interleaved(48_000, 1, vec![0.0; 8]);
    write_wav(&path, &buffer, WavSampleFormat::Int16).unwrap();

    assert_eq!(dir.entries(), ["out.wav"]);
    assert_eq!(fs::read(&path).unwrap().len(), 44 + 16);
}

#[test]
fn two_writers_for_the_same_file_do_not_collide() {
    let dir = TempDir::new("two-writers");
    let path = dir.path("out.wav");
    let mut first = WavWriter::create(&path, 48_000, 1, WavSampleFormat::Float32).unwrap();
    let mut second = WavWriter::create(&path, 48_000, 1, WavSampleFormat::Float32).unwrap();
    assert_eq!(dir.entries().len(), 2);

    first.write(&[0.25]).unwrap();
    second.write(&[0.5, 0.75]).unwrap();
    first.finalize().unwrap();
    second.finalize().unwrap();

    // The writer that finished last wins, whole.
    assert_eq!(dir.entries(), ["out.wav"]);
    assert_eq!(
        windfall_codec::decode_file(&path).unwrap().samples(),
        [0.5, 0.75]
    );
}

#[test]
fn a_missing_folder_is_an_io_error() {
    let dir = TempDir::new("no-folder");
    let path = dir.path("no-such-folder").join("out.wav");
    let result = WavWriter::create(&path, 48_000, 2, WavSampleFormat::Int16);
    assert!(matches!(result, Err(CodecError::Io(_))));
}

#[test]
fn impossible_layouts_are_refused() {
    let dir = TempDir::new("layout");
    let path = dir.path("out.wav");
    for (rate, channels) in [(0, 2), (48_000, 0), (48_000, 40_000), (u32::MAX, 2)] {
        let result = WavWriter::create(&path, rate, channels, WavSampleFormat::Int24);
        assert!(matches!(result, Err(CodecError::InvalidInput(_))));
    }
    assert!(dir.entries().is_empty());
}

#[test]
fn a_block_that_splits_a_frame_is_refused() {
    let dir = TempDir::new("ragged");
    let path = dir.path("out.wav");
    let mut writer = WavWriter::create(&path, 48_000, 2, WavSampleFormat::Int16).unwrap();
    assert!(matches!(
        writer.write(&[0.1, 0.2, 0.3]),
        Err(CodecError::InvalidInput(_))
    ));

    // The refused block left the writer usable.
    writer.write(&[0.1, 0.2]).unwrap();
    writer.finalize().unwrap();
    assert_eq!(fs::read(&path).unwrap().len(), 44 + 4);
}

#[test]
fn odd_length_data_is_padded_to_an_even_size() {
    let dir = TempDir::new("pad");
    let path = dir.path("odd.wav");
    // One mono 24-bit frame is three bytes.
    let buffer = AudioBuffer::from_interleaved(48_000, 1, vec![0.5]);
    write_wav(&path, &buffer, WavSampleFormat::Int24).unwrap();

    let bytes = fs::read(&path).unwrap();
    assert_eq!(bytes.len() % 2, 0);
    assert_eq!(u32_at(&bytes, 4) as usize, bytes.len() - 8);
    // The data chunk's own size leaves the pad byte out.
    assert_eq!(u32_at(&bytes, bytes.len() - 8), 3);
    assert_eq!(bytes[bytes.len() - 1], 0);
}
