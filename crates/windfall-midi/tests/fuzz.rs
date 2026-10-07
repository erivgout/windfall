//! Hostile input: random bytes, valid files cut short at every length, and
//! valid files with bytes changed, dropped and added. The reader must
//! answer every one of them with a song or an error, and never panic.

mod common;

use common::*;
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::Index;
use windfall_midi::*;

/// Reads `bytes` and checks what every outcome promises: an error has
/// something to say, and a song is in its normal form and survives being
/// written and read again.
fn check(bytes: &[u8]) -> Result<(), TestCaseError> {
    match read(bytes) {
        Ok(song) => {
            prop_assert_eq!(&song.clone().normalized(), &song);
            prop_assert!(song.length >= song.tracks.iter().map(MidiTrack::end).max().unwrap_or(0));
            let again = read(&write(&song, &WriteOptions::default()));
            prop_assert_eq!(again, Ok(song));
        }
        Err(error) => prop_assert!(!error.to_string().is_empty()),
    }
    Ok(())
}

/// Bytes that look like the events of a track often enough to get deep
/// into the reader: real status bytes, short waits, and now and then
/// anything at all.
fn event_soup() -> impl Strategy<Value = Vec<u8>> {
    let piece = prop_oneof![
        4 => (0_u8..0x80, 0x80_u8..0xF0, any::<u8>(), any::<u8>())
            .prop_map(|(wait, status, a, b)| vec![wait, status, a & 0x7F, b & 0x7F]),
        2 => (0_u8..0x80, any::<u8>(), any::<u8>()).prop_map(|(wait, a, b)| vec![wait, a, b]),
        2 => (any::<u8>(), any::<u8>(), vec(any::<u8>(), 0..6)).prop_map(|(kind, length, data)| {
            [vec![0, 0xFF, kind, length], data].concat()
        }),
        1 => (any::<u8>(), vec(any::<u8>(), 0..6))
            .prop_map(|(length, data)| [vec![0, 0xF0, length], data].concat()),
        1 => Just(vec![0, 0xFF, 0x2F, 0]),
        1 => Just(vec![0xFF, 0xFF, 0xFF, 0x7F]),
        1 => vec(any::<u8>(), 0..8),
    ];
    vec(piece, 0..40).prop_map(|pieces| pieces.concat())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(600))]

    #[test]
    fn random_bytes_are_answered(bytes in vec(any::<u8>(), 0..300)) {
        check(&bytes)?;
    }

    #[test]
    fn random_bytes_behind_a_header_are_answered(
        format in 0_u16..4,
        tracks in any::<u16>(),
        division in any::<u16>(),
        rest in vec(any::<u8>(), 0..300),
    ) {
        check(&[header(format, tracks, division), rest].concat())?;
    }

    #[test]
    fn random_events_are_answered(
        format in 0_u16..3,
        division in prop_oneof![1_u16..2_000, any::<u16>()],
        tracks in vec(event_soup(), 1..4),
    ) {
        let chunks: Vec<Vec<u8>> = tracks.iter().map(|body| chunk(b"MTrk", body)).collect();
        check(&file(format, division, &chunks))?;
    }

    #[test]
    fn a_valid_file_with_changed_bytes_is_answered(
        song in midi_song(Wildness::TAME),
        single in any::<bool>(),
        changes in vec((any::<Index>(), any::<u8>()), 1..6),
    ) {
        let format = if single { SmfFormat::Single } else { SmfFormat::Multi };
        let options = WriteOptions { format, ..WriteOptions::default() };
        let mut bytes = write(&song, &options);
        for (at, byte) in changes {
            let at = at.index(bytes.len());
            bytes[at] = byte;
        }
        check(&bytes)?;
    }

    #[test]
    fn a_valid_file_with_bytes_dropped_and_added_is_answered(
        song in midi_song(Wildness::TAME),
        dropped in vec(any::<Index>(), 0..4),
        added in vec((any::<Index>(), any::<u8>()), 0..4),
    ) {
        let mut bytes = write(&song, &WriteOptions::default());
        for at in dropped {
            bytes.remove(at.index(bytes.len()));
        }
        for (at, byte) in added {
            bytes.insert(at.index(bytes.len()), byte);
        }
        check(&bytes)?;
    }
}

proptest! {
    // Each case reads the file once for every byte it has.
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn a_valid_file_cut_short_anywhere_is_answered(
        song in midi_song(Wildness::TAME),
        single in any::<bool>(),
        running_status in any::<bool>(),
    ) {
        let format = if single { SmfFormat::Single } else { SmfFormat::Multi };
        let options = WriteOptions { format, running_status, ..WriteOptions::default() };
        let bytes = write(&song, &options);
        for length in 0..bytes.len() {
            check(&bytes[..length])?;
        }
    }
}

#[test]
fn lengths_that_promise_gigabytes_cost_nothing() {
    // Each of these claims far more than the few bytes that are there. If
    // the reader set memory aside on their word, this test would not
    // finish.
    let huge = [0xFF, 0xFF, 0xFF, 0x7F];
    let claims: Vec<Vec<u8>> = vec![
        [b"MThd".as_slice(), &[0xFF; 4], &[0; 6]].concat(),
        [header(1, 0xFFFF, 960), b"MTrk".to_vec(), vec![0xFF; 4]].concat(),
        file(
            0,
            960,
            &[track(&[[&[0, 0xFF, 0x03], huge.as_slice()].concat()])],
        ),
        file(0, 960, &[track(&[[&[0, 0xF0], huge.as_slice()].concat()])]),
        file(
            0,
            960,
            &[track(&[
                [&[0, 0xFF, 0x7F], huge.as_slice(), &[1, 2]].concat()
            ])],
        ),
    ];
    for bytes in claims {
        assert!(read(&bytes).is_err(), "{bytes:?}");
    }
}

#[test]
fn a_file_of_nothing_but_empty_chunks_is_read_without_piling_them_up() {
    // 100,000 empty tracks, which is more than a header can count.
    let mut bytes = header(1, 0, 960);
    for _ in 0..100_000 {
        bytes.extend_from_slice(b"MTrk\0\0\0\0");
    }
    let song = read(&bytes).expect("the file is valid, if useless");
    assert_eq!(song.tracks.len(), 65_534);

    // And as many chunks of some other kind, which hold no track at all.
    let mut bytes = header(1, 1, 960);
    for _ in 0..100_000 {
        bytes.extend_from_slice(b"junk\0\0\0\0");
    }
    assert_eq!(read(&bytes), Err(MidiError::NoTracks));
}

#[test]
fn a_track_of_nothing_but_note_ons_ends_every_one_of_them() {
    let mut events = Vec::new();
    for index in 0..20_000_u32 {
        events.push(event(
            1,
            &[0x90, (index % 128) as u8, 1 + (index % 127) as u8],
        ));
    }
    events.push(end(5));
    let song = read(&file(0, 960, &[track(&events)])).expect("the file is valid");
    assert_eq!(song.note_count(), 20_000);
    assert!(song.tracks[0].notes.iter().all(|note| note.end() == 20_005));
}
