//! Property tests: whatever song is written, reading the file gives the
//! song's normal form back.

mod common;

use common::*;
use proptest::prelude::*;
use windfall_midi::*;

fn options(format: SmfFormat, running_status: bool) -> WriteOptions {
    WriteOptions {
        format,
        resolution: Resolution::Ppq960,
        running_status,
    }
}

/// The song with every time doubled, so that all of them are even.
fn doubled(mut song: MidiSong) -> MidiSong {
    for track in &mut song.tracks {
        for note in &mut track.notes {
            note.start *= 2;
            note.length *= 2;
        }
        for program in &mut track.programs {
            program.tick *= 2;
        }
        let points = track
            .controls
            .iter_mut()
            .flat_map(|curve| &mut curve.points);
        for point in points {
            point.tick *= 2;
        }
    }
    song.tempos.iter_mut().for_each(|tempo| tempo.tick *= 2);
    song.time_signatures
        .iter_mut()
        .for_each(|signature| signature.tick *= 2);
    song.key_signatures.iter_mut().for_each(|key| key.tick *= 2);
    song.markers.iter_mut().for_each(|marker| marker.tick *= 2);
    song.length *= 2;
    song
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    #[test]
    fn the_normal_form_of_a_normal_form_is_itself(song in midi_song(Wildness::ANY)) {
        let normal = song.normalized();
        prop_assert_eq!(normal.clone().normalized(), normal);
    }

    #[test]
    fn the_normal_form_keeps_every_note_and_message(song in midi_song(Wildness::TAME)) {
        let normal = song.clone().normalized();
        prop_assert_eq!(normal.note_count(), song.note_count());
        for (track, original) in normal.tracks.iter().zip(&song.tracks) {
            // Starts and velocities stay together, as do ends and release
            // velocities, whatever notes they are paired into.
            let mut starts: Vec<_> = track.notes.iter().map(|n| (n.channel, n.key, n.start, n.velocity)).collect();
            let mut wanted: Vec<_> = original.notes.iter().map(|n| (n.channel, n.key, n.start, n.velocity)).collect();
            starts.sort_unstable();
            wanted.sort_unstable();
            prop_assert_eq!(starts, wanted);
            let mut ends: Vec<_> = track.notes.iter().map(|n| (n.channel, n.key, n.end(), n.release)).collect();
            let mut wanted: Vec<_> = original.notes.iter().map(|n| (n.channel, n.key, n.end(), n.release)).collect();
            ends.sort_unstable();
            wanted.sort_unstable();
            prop_assert_eq!(ends, wanted);

            let points = |track: &MidiTrack| track.controls.iter().map(|c| c.points.len()).sum::<usize>();
            prop_assert_eq!(points(track), points(original));
            prop_assert_eq!(track.programs.len(), original.programs.len());
        }
    }

    #[test]
    fn a_format_1_file_reads_back_as_the_normal_form(
        song in midi_song(Wildness::ANY),
        running_status in any::<bool>(),
    ) {
        let bytes = write(&song, &options(SmfFormat::Multi, running_status));
        prop_assert_eq!(read(&bytes), Ok(song.normalized()));
    }

    #[test]
    fn a_format_0_file_reads_back_as_the_flattened_normal_form(
        song in midi_song(Wildness::ANY),
        running_status in any::<bool>(),
    ) {
        let bytes = write(&song, &options(SmfFormat::Single, running_status));
        let back = read(&bytes);
        prop_assert_eq!(back.as_ref().map(|song| song.tracks.len()), Ok(1));
        prop_assert_eq!(back, Ok(song.flattened().normalized()));
    }

    #[test]
    fn a_song_on_even_ticks_survives_480_ticks_to_the_quarter(
        song in midi_song(Wildness::TAME),
        running_status in any::<bool>(),
    ) {
        let song = doubled(song);
        let halved = WriteOptions {
            format: SmfFormat::Multi,
            resolution: Resolution::Ppq480,
            running_status,
        };
        let bytes = write(&song, &halved);
        prop_assert_eq!(read(&bytes), Ok(song.normalized()));
    }

    #[test]
    fn halving_moves_nothing_by_more_than_a_tick(song in midi_song(Wildness::TAME)) {
        let halved = WriteOptions {
            resolution: Resolution::Ppq480,
            ..WriteOptions::default()
        };
        let normal = song.clone().normalized();
        let back = read(&write(&song, &halved)).expect("the file is valid");
        prop_assert_eq!(back.note_count(), normal.note_count());
        for (track, original) in back.tracks.iter().zip(&normal.tracks) {
            let mut starts: Vec<_> = track.notes.iter().map(|n| (n.channel, n.key, n.start)).collect();
            let mut wanted: Vec<_> = original.notes.iter().map(|n| (n.channel, n.key, n.start)).collect();
            starts.sort_unstable();
            wanted.sort_unstable();
            for (got, wanted) in starts.iter().zip(&wanted) {
                prop_assert_eq!((got.0, got.1), (wanted.0, wanted.1));
                // An odd tick rounds up to the next even one.
                prop_assert_eq!(got.2, wanted.2.div_ceil(2) * 2);
            }
        }
        prop_assert_eq!(back.tempos.len(), normal.tempos.len());
    }

    #[test]
    fn the_same_song_always_gives_the_same_bytes(
        song in midi_song(Wildness::ANY),
        single in any::<bool>(),
        running_status in any::<bool>(),
    ) {
        let format = if single { SmfFormat::Single } else { SmfFormat::Multi };
        let options = options(format, running_status);
        let bytes = write(&song, &options);
        prop_assert_eq!(&write(&song, &options), &bytes);
        // What was read writes the very same file again.
        let back = read(&bytes).expect("the file is valid");
        prop_assert_eq!(&write(&back, &options), &bytes);
        // The header counts the tracks that are there.
        let chunks = bytes.windows(4).filter(|window| window == b"MTrk").count();
        prop_assert!(chunks >= usize::from(u16::from_be_bytes([bytes[10], bytes[11]])));
    }

    #[test]
    fn running_status_only_makes_the_file_smaller(song in midi_song(Wildness::TAME)) {
        let short = write(&song, &options(SmfFormat::Multi, true));
        let long = write(&song, &options(SmfFormat::Multi, false));
        prop_assert!(short.len() <= long.len());
        prop_assert_eq!(read(&short), read(&long));
    }
}
