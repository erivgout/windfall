//! MIDI files built byte by byte: what the reader makes of each corner of
//! the format, what it refuses, and the exact bytes the writer produces.

mod common;

use common::*;
use windfall_midi::*;

const NOTE_ON: u8 = 0x90;
const NOTE_OFF: u8 = 0x80;

/// A file with one track that plays key 60 from `start` to `end`, counted
/// in the file's own ticks.
fn one_note(division: u16, start: u32, end: u32) -> Vec<u8> {
    let events = [
        event(start, &[NOTE_ON, 60, 100]),
        event(end - start, &[NOTE_OFF, 60, 64]),
        common::end(0),
    ];
    file(0, division, &[track(&events)])
}

fn only_track(song: &MidiSong) -> &MidiTrack {
    assert_eq!(song.tracks.len(), 1, "{song:?}");
    &song.tracks[0]
}

#[track_caller]
fn refuse(bytes: &[u8]) -> MidiError {
    match read(bytes) {
        Ok(song) => panic!("the file was read: {song:?}"),
        Err(error) => {
            assert!(!error.to_string().is_empty());
            error
        }
    }
}

#[test]
fn a_format_0_file_becomes_one_track_with_everything_it_says() {
    let events = [
        meta(0, 0x03, b"Solo"),
        meta(0, 0x04, b"Nylon Guitar"),
        meta(0, 0x58, &[3, 3, 24, 8]),
        meta(0, 0x59, &[0xFE, 1]),
        tempo(0, 600_000),
        event(0, &[0xC0, 24]),
        event(0, &[NOTE_ON, 60, 100]),
        event(480, &[NOTE_OFF, 60, 40]),
        meta(0, 0x06, b"Bridge"),
        common::end(240),
    ];
    let song = read(&file(0, 480, &[track(&events)])).expect("the file is valid");
    let expected = MidiSong {
        name: None,
        tracks: vec![MidiTrack {
            name: Some("Solo".to_owned()),
            instrument: Some("Nylon Guitar".to_owned()),
            notes: vec![MidiNote {
                start: 0,
                length: 960,
                key: 60,
                velocity: 100,
                release: 40,
                channel: 0,
            }],
            programs: vec![ProgramChange {
                tick: 0,
                channel: 0,
                program: 24,
            }],
            controls: Vec::new(),
        }],
        tempos: vec![TempoChange {
            tick: 0,
            micros_per_quarter: 600_000,
        }],
        time_signatures: vec![TimeSignatureChange {
            tick: 0,
            numerator: 3,
            denominator: 8,
        }],
        key_signatures: vec![KeySignatureChange {
            tick: 0,
            sharps: -2,
            minor: true,
        }],
        markers: vec![Marker {
            tick: 960,
            text: "Bridge".to_owned(),
        }],
        length: 1_440,
    };
    assert_eq!(song, expected);
    assert_eq!(song.tempos[0].bpm(), 100.0);
}

#[test]
fn the_first_track_of_a_format_1_file_names_the_song_when_it_plays_nothing() {
    let conductor = track(&[
        meta(0, 0x03, b"My Song"),
        tempo(0, 500_000),
        tempo(1_920, 400_000),
        common::end(0),
    ]);
    let piano = track(&[
        meta(0, 0x03, b"Piano"),
        event(0, &[NOTE_ON, 60, 90]),
        event(960, &[NOTE_ON, 60, 0]),
        common::end(0),
    ]);
    let bass = track(&[
        meta(0, 0x03, b"Bass"),
        // A tempo change outside the first track still counts.
        tempo(960, 450_000),
        event(0, &[NOTE_ON | 1, 36, 80]),
        event(480, &[NOTE_OFF | 1, 36, 0]),
        common::end(0),
    ]);
    let song = read(&file(1, 960, &[conductor, piano, bass])).expect("the file is valid");
    assert_eq!(song.name.as_deref(), Some("My Song"));
    assert_eq!(song.tracks.len(), 2);
    assert_eq!(song.tracks[0].name.as_deref(), Some("Piano"));
    assert_eq!(
        song.tracks[0].notes,
        [MidiNote {
            velocity: 90,
            ..note(0, 960, 60)
        }]
    );
    assert_eq!(song.tracks[1].name.as_deref(), Some("Bass"));
    assert_eq!(song.tracks[1].notes[0].channel, 1);
    assert_eq!(song.tracks[1].notes[0].release, 0);
    let tempos: Vec<_> = song
        .tempos
        .iter()
        .map(|tempo| (tempo.tick, tempo.micros_per_quarter))
        .collect();
    assert_eq!(tempos, [(0, 500_000), (960, 450_000), (1_920, 400_000)]);
    assert_eq!(song.length, 1_920);
    assert_eq!(song.micros_per_quarter_at(1_000), 450_000);
}

#[test]
fn a_first_track_that_plays_is_a_track_like_any_other() {
    let first = track(&[
        meta(0, 0x03, b"Keys"),
        event(0, &[NOTE_ON, 60, 90]),
        event(10, &[NOTE_OFF, 60, 0]),
        common::end(0),
    ]);
    let second = track(&[event(0, &[0xB0, 7, 100]), common::end(0)]);
    let song = read(&file(1, 960, &[first, second])).expect("the file is valid");
    assert_eq!(song.name, None);
    assert_eq!(song.tracks.len(), 2);
    assert_eq!(song.tracks[0].name.as_deref(), Some("Keys"));

    // A format 0 file that has more than one track after all is read the
    // same way.
    let conductor = track(&[meta(0, 0x03, b"Two"), common::end(0)]);
    let notes = track(&[
        event(0, &[NOTE_ON, 60, 90]),
        event(10, &[NOTE_OFF, 60, 0]),
        common::end(0),
    ]);
    let song = read(&file(0, 960, &[conductor, notes])).expect("the file is valid");
    assert_eq!(song.name.as_deref(), Some("Two"));
    assert_eq!(song.tracks.len(), 1);
}

#[test]
fn the_patterns_of_a_format_2_file_follow_one_another() {
    let first = track(&[
        meta(0, 0x03, b"Intro"),
        tempo(0, 500_000),
        event(0, &[NOTE_ON, 60, 100]),
        event(480, &[NOTE_OFF, 60, 64]),
        common::end(480),
    ]);
    let second = track(&[
        meta(0, 0x03, b"Verse"),
        tempo(240, 250_000),
        event(0, &[NOTE_ON, 62, 100]),
        event(240, &[NOTE_OFF, 62, 64]),
        common::end(0),
    ]);
    let third = track(&[
        event(0, &[NOTE_ON, 64, 100]),
        event(120, &[NOTE_OFF, 64, 64]),
        common::end(0),
    ]);
    let song = read(&file(2, 960, &[first, second, third])).expect("the file is valid");
    assert_eq!(song.name, None);
    assert_eq!(song.tracks.len(), 3);
    assert_eq!(song.tracks[0].notes, [note(0, 480, 60)]);
    assert_eq!(song.tracks[1].name.as_deref(), Some("Verse"));
    assert_eq!(song.tracks[1].notes, [note(1_200, 240, 62)]);
    assert_eq!(song.tracks[2].notes, [note(1_440, 120, 64)]);
    let tempos: Vec<_> = song.tempos.iter().map(|tempo| tempo.tick).collect();
    assert_eq!(tempos, [0, 1_200]);
    assert_eq!(song.length, 1_560);
}

#[test]
fn any_resolution_is_rescaled_exactly_and_rounded_to_the_nearest_tick() {
    // (ticks per quarter, start, end, start in Windfall ticks, length)
    let cases = [
        (960, 7, 8, 7, 1),
        (480, 1, 3, 2, 4),
        (96, 1, 2, 10, 10),
        (120, 3, 4, 24, 8),
        (384, 7, 8, 18, 2),  // 17.5 rounds up to 18
        (1_920, 1, 2, 1, 1), // 0.5 rounds up, and the note keeps a tick
        (1_920, 2, 5, 1, 2), // 1 to 2.5
        (1, 3, 4, 2_880, 960),
        (7, 2, 3, 274, 137), // 274.29 to 411.43
        (0x7FFF, 0x7FFF, 0xFFFE, 960, 960),
        (0x7FFF, 17, 18, 0, 1), // 0.498 to 0.527, which rounds to 1
    ];
    for (division, start, end, tick, length) in cases {
        let song = read(&one_note(division, start, end)).expect("the file is valid");
        let notes = &only_track(&song).notes;
        assert_eq!(
            notes,
            &[note(tick, length, 60)],
            "{division} ticks per quarter"
        );
    }
}

#[test]
fn smpte_time_becomes_beats_through_the_tempo_map() {
    // 25 frames a second and 40 ticks a frame: a tick is a millisecond.
    let division = u16::from_be_bytes([(-25_i8) as u8, 40]);
    let events = [
        event(500, &[NOTE_ON, 60, 100]),
        // After a second at 120 bpm, two beats have passed.
        tempo(500, 1_000_000),
        // Half a second at 60 bpm is half a beat.
        event(500, &[NOTE_OFF, 60, 64]),
        common::end(1_000),
    ];
    let song = read(&file(0, division, &[track(&events)])).expect("the file is valid");
    assert_eq!(only_track(&song).notes, [note(960, 1_440, 60)]);
    assert_eq!(song.tempos[0].tick, 1_920);
    assert_eq!(song.length, 3_360);

    // 29.97 frames a second and 100 ticks a frame: 2,997 ticks are a
    // thousandth short of a second.
    let division = u16::from_be_bytes([(-29_i8) as u8, 100]);
    let song = read(&one_note(division, 2_997, 5_994)).expect("the file is valid");
    assert_eq!(only_track(&song).notes, [note(1_920, 1_920, 60)]);

    for (frames, ticks_per_second) in [(-24_i8, 24), (-30, 30)] {
        let division = u16::from_be_bytes([frames as u8, 1]);
        let song = read(&one_note(division, ticks_per_second, ticks_per_second * 2))
            .expect("the file is valid");
        assert_eq!(only_track(&song).notes, [note(1_920, 1_920, 60)]);
    }
}

#[test]
fn smpte_time_follows_a_tempo_set_in_another_track() {
    let division = u16::from_be_bytes([(-25_i8) as u8, 40]);
    let conductor = track(&[tempo(0, 250_000), common::end(0)]);
    let notes = track(&[
        event(250, &[NOTE_ON, 60, 100]),
        event(250, &[NOTE_OFF, 60, 64]),
        common::end(0),
    ]);
    let song = read(&file(1, division, &[conductor, notes])).expect("the file is valid");
    // At 240 bpm a quarter of a second is a beat.
    assert_eq!(only_track(&song).notes, [note(960, 960, 60)]);
}

#[test]
fn running_status_carries_the_last_status_on() {
    let events = [
        event(0, &[NOTE_ON, 60, 100]),
        event(10, &[62, 90]),
        event(10, &[60, 0]),
        // Strictly a meta event ends running status. Files that carry on
        // regardless exist, and what they mean is not in doubt.
        meta(0, 0x06, b"x"),
        event(10, &[62, 0]),
        event(0, &[0xB0, 7, 100]),
        event(5, &[10, 64]),
        event(5, &[0xC0, 3]),
        event(5, &[4]),
        common::end(0),
    ];
    let song = read(&file(0, 960, &[track(&events)])).expect("the file is valid");
    let track = only_track(&song);
    assert_eq!(
        track.notes,
        [
            note(0, 20, 60),
            MidiNote {
                velocity: 90,
                ..note(10, 20, 62)
            }
        ]
    );
    let volume = track.control(0, ControlKind::Controller(CC_VOLUME));
    assert_eq!(volume.expect("there is a volume").points[0].value, 100);
    let pan = track.control(0, ControlKind::Controller(CC_PAN));
    assert_eq!(pan.expect("there is a pan").points[0].tick, 35);
    let programs: Vec<_> = track.programs.iter().map(|p| (p.tick, p.program)).collect();
    assert_eq!(programs, [(40, 3), (45, 4)]);
    assert_eq!(track.program(0), Some(3));
}

#[test]
fn a_note_on_with_velocity_0_ends_a_note() {
    let events = [
        event(0, &[NOTE_ON | 2, 60, 1]),
        event(100, &[NOTE_ON | 2, 60, 0]),
        common::end(0),
    ];
    let song = read(&file(0, 960, &[track(&events)])).expect("the file is valid");
    assert_eq!(
        only_track(&song).notes,
        [MidiNote {
            start: 0,
            length: 100,
            key: 60,
            velocity: 1,
            release: DEFAULT_RELEASE,
            channel: 2,
        }]
    );
}

#[test]
fn notes_that_overlap_on_one_key_end_in_the_order_they_began() {
    let events = [
        event(0, &[NOTE_ON, 60, 100]),
        event(10, &[NOTE_ON, 60, 50]),
        // The same key on another channel is another key.
        event(0, &[NOTE_ON | 1, 60, 70]),
        // Nothing is sounding on key 61: this ends nothing.
        event(5, &[NOTE_OFF, 61, 0]),
        event(5, &[NOTE_OFF, 60, 11]),
        event(10, &[NOTE_OFF, 60, 22]),
        event(10, &[NOTE_OFF | 1, 60, 33]),
        common::end(0),
    ];
    let song = read(&file(0, 960, &[track(&events)])).expect("the file is valid");
    let key = |start, length, velocity, release, channel| MidiNote {
        start,
        length,
        key: 60,
        velocity,
        release,
        channel,
    };
    assert_eq!(
        only_track(&song).notes,
        [
            key(0, 20, 100, 11, 0),
            key(10, 20, 50, 22, 0),
            key(10, 30, 70, 33, 1),
        ]
    );
    assert_eq!(only_track(&song).note_channels(), [0, 1]);
}

#[test]
fn two_notes_struck_together_on_one_key_keep_their_velocities() {
    let events = [
        event(0, &[NOTE_ON, 60, 100]),
        event(0, &[NOTE_ON, 60, 50]),
        event(5, &[NOTE_OFF, 60, 64]),
        event(5, &[NOTE_OFF, 60, 64]),
        common::end(0),
    ];
    let song = read(&file(0, 960, &[track(&events)])).expect("the file is valid");
    let notes = &only_track(&song).notes;
    assert_eq!((notes[0].length, notes[0].velocity), (5, 100));
    assert_eq!((notes[1].length, notes[1].velocity), (10, 50));
}

#[test]
fn a_note_that_never_ends_is_ended_where_its_track_ends() {
    let events = [
        event(0, &[NOTE_ON, 64, 100]),
        event(100, &[NOTE_ON, 67, 100]),
        common::end(400),
    ];
    let song = read(&file(0, 960, &[track(&events)])).expect("the file is valid");
    assert_eq!(
        only_track(&song).notes,
        [note(0, 500, 64), note(100, 400, 67)]
    );
    assert_eq!(song.length, 500);

    // Without an end marker the track ends on its last event.
    let events = [event(0, &[NOTE_ON, 64, 100]), event(100, &[0xB0, 1, 0])];
    let song = read(&file(0, 960, &[track(&events)])).expect("the file is valid");
    assert_eq!(only_track(&song).notes, [note(0, 100, 64)]);

    // A note struck on the last tick is one tick long.
    let events = [event(7, &[NOTE_ON, 64, 100]), common::end(0)];
    let song = read(&file(0, 960, &[track(&events)])).expect("the file is valid");
    assert_eq!(only_track(&song).notes, [note(7, 1, 64)]);
    assert_eq!(song.length, 8);
}

#[test]
fn a_note_with_no_length_is_one_tick_long() {
    let events = [
        event(5, &[NOTE_ON, 60, 100]),
        event(0, &[NOTE_OFF, 60, 64]),
        event(0, &[NOTE_ON, 60, 80]),
        event(3, &[NOTE_OFF, 60, 64]),
        common::end(0),
    ];
    let song = read(&file(0, 960, &[track(&events)])).expect("the file is valid");
    assert_eq!(
        only_track(&song).notes,
        [
            note(5, 1, 60),
            MidiNote {
                velocity: 80,
                ..note(5, 3, 60)
            }
        ]
    );
}

#[test]
fn controllers_bends_and_pressure_are_kept_as_curves() {
    let events = [
        event(0, &[0xB3, 7, 90]),
        event(0, &[0xB3, 10, 0]),
        event(0, &[0xB3, 64, 127]),
        event(10, &[0xE3, 0x00, 0x40]),
        event(10, &[0xE3, 0x7F, 0x7F]),
        event(10, &[0xE3, 0x00, 0x00]),
        event(10, &[0xD3, 55]),
        event(10, &[0xA3, 60, 44]),
        event(10, &[0xB3, 7, 30]),
        event(0, &[0xB3, 7, 31]),
        common::end(0),
    ];
    let song = read(&file(0, 960, &[track(&events)])).expect("the file is valid");
    let track = only_track(&song);
    let points = |kind| {
        let curve = track.control(3, kind).expect("the curve is there");
        let points = curve.points.iter();
        points
            .map(|point| (point.tick, point.value))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        points(ControlKind::Controller(CC_VOLUME)),
        [(0, 90), (60, 30), (60, 31)]
    );
    assert_eq!(points(ControlKind::Controller(CC_PAN)), [(0, 0)]);
    assert_eq!(points(ControlKind::Controller(64)), [(0, 127)]);
    assert_eq!(
        points(ControlKind::PitchBend),
        [(10, PITCH_BEND_CENTER), (20, 16_383), (30, 0)]
    );
    assert_eq!(points(ControlKind::ChannelPressure), [(40, 55)]);
    assert_eq!(points(ControlKind::KeyPressure(60)), [(50, 44)]);
    assert_eq!(track.controls.len(), 6);
    assert!(track.notes.is_empty());
    assert_eq!(song.length, 60);
}

#[test]
fn what_the_reader_has_no_use_for_is_passed_over() {
    let events = [
        // System exclusive, in both of its forms.
        event(0, &[0xF0, 3, 0x7E, 0x7F, 0xF7]),
        event(0, &[0xF7, 2, 0x01, 0x02]),
        // Text, copyright, lyric, cue point, channel prefix, port, SMPTE
        // offset, sequencer data, and a kind that does not exist.
        meta(0, 0x01, b"text"),
        meta(0, 0x02, b"(c)"),
        meta(0, 0x05, b"la"),
        meta(0, 0x07, b"cue"),
        meta(0, 0x20, &[1]),
        meta(0, 0x21, &[0]),
        meta(0, 0x54, &[0, 0, 0, 0, 0]),
        meta(0, 0x7F, &[0x41, 0x00]),
        meta(0, 0x6A, &[1, 2, 3, 4, 5, 6, 7, 8, 9]),
        // Events too short for their kind, and a tempo of nothing.
        meta(0, 0x51, &[0x07, 0xA1]),
        meta(0, 0x51, &[0, 0, 0]),
        meta(0, 0x58, &[4]),
        meta(0, 0x58, &[0, 2, 24, 8]),
        meta(0, 0x58, &[4, 9, 24, 8]),
        meta(0, 0x59, &[1]),
        event(0, &[NOTE_ON, 60, 100]),
        event(10, &[NOTE_OFF, 60, 64]),
        common::end(0),
    ];
    let bytes = [
        // A header that is longer than it has to be.
        chunk(b"MThd", &[0, 1, 0, 1, 0x03, 0xC0, 0xAA, 0xBB]),
        chunk(b"XFIH", b"some other program's chunk"),
        track(&events),
    ]
    .concat();
    let song = read(&bytes).expect("the file is valid");
    assert_eq!(only_track(&song).notes, [note(0, 10, 60)]);
    assert!(song.tempos.is_empty());
    assert!(song.time_signatures.is_empty());
    assert!(song.key_signatures.is_empty());
    assert!(song.markers.is_empty());
}

#[test]
fn key_and_time_signatures_are_held_to_what_they_can_mean() {
    let events = [
        meta(0, 0x59, &[7, 0]),
        meta(10, 0x59, &[100, 5]),
        meta(0, 0x59, &[0x80, 0]),
        meta(0, 0x58, &[255, 7, 24, 8]),
        meta(0, 0x58, &[1, 0, 24, 8]),
        common::end(0),
    ];
    let song = read(&file(0, 960, &[track(&events)])).expect("the file is valid");
    let keys: Vec<_> = song
        .key_signatures
        .iter()
        .map(|key| (key.tick, key.sharps, key.minor))
        .collect();
    assert_eq!(keys, [(0, 7, false), (10, 7, true), (10, -7, false)]);
    let signatures: Vec<_> = song
        .time_signatures
        .iter()
        .map(|signature| (signature.numerator, signature.denominator))
        .collect();
    assert_eq!(signatures, [(255, 128), (1, 1)]);
}

#[test]
fn names_in_any_encoding_are_read() {
    let events = [
        meta(0, 0x03, &[b'F', b'l', 0xF6, b't', b'e', 0]),
        meta(0, 0x04, "Flöte ♪".as_bytes()),
        // A second name does not replace the first.
        meta(0, 0x03, b"Other"),
        meta(0, 0x06, &[]),
        common::end(0),
    ];
    let song = read(&file(0, 960, &[track(&events)])).expect("the file is valid");
    assert_eq!(only_track(&song).name.as_deref(), Some("Flöte"));
    assert_eq!(only_track(&song).instrument.as_deref(), Some("Flöte ♪"));
    assert_eq!(song.markers[0].text, "");

    let events = [
        meta(0, 0x03, &[0, 0]),
        meta(0, 0x03, b"Real"),
        common::end(0),
    ];
    let song = read(&file(0, 960, &[track(&events)])).expect("the file is valid");
    assert_eq!(only_track(&song).name.as_deref(), Some("Real"));
}

#[test]
fn what_follows_the_end_of_a_track_is_ignored() {
    let events = [
        event(0, &[NOTE_ON, 60, 100]),
        event(10, &[NOTE_OFF, 60, 64]),
        common::end(0),
        event(0, &[NOTE_ON, 61, 100]),
        vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
    ];
    let song = read(&file(0, 960, &[track(&events)])).expect("the file is valid");
    assert_eq!(only_track(&song).notes, [note(0, 10, 60)]);
    assert_eq!(song.length, 10);
}

#[test]
fn the_header_count_only_says_when_to_stop() {
    let notes = |key| {
        track(&[
            event(0, &[NOTE_ON, key, 100]),
            event(10, &[NOTE_OFF, key, 64]),
            common::end(0),
        ])
    };
    // More tracks promised than the file has.
    let bytes = [header(1, 65_535, 960), notes(60), notes(61)].concat();
    assert_eq!(read(&bytes).expect("the file is valid").tracks.len(), 2);

    // None promised: every track is read.
    let bytes = [header(1, 0, 960), notes(60), notes(61), notes(62)].concat();
    assert_eq!(read(&bytes).expect("the file is valid").tracks.len(), 3);

    // Fewer promised than there are, and rubbish after them.
    let bytes = [header(1, 1, 960), notes(60), notes(61), vec![1, 2, 3]].concat();
    assert_eq!(read(&bytes).expect("the file is valid").tracks.len(), 1);
}

#[test]
fn what_is_not_a_midi_file_is_refused() {
    assert_eq!(refuse(b""), MidiError::NotMidi);
    assert_eq!(refuse(b"MTh"), MidiError::NotMidi);
    assert_eq!(refuse(b"RIFF\0\0\0\0WAVE"), MidiError::NotMidi);
    assert_eq!(refuse(&[0; 64]), MidiError::NotMidi);
    assert!(refuse(b"").to_string().contains("not a MIDI file"));
}

#[test]
fn a_bad_header_is_refused() {
    let truncated = refuse(b"MThd\0\0");
    assert!(matches!(truncated, MidiError::Truncated { offset: 0, .. }));
    let cut_short = refuse(b"MThd\0\0\0\x06\0\x01");
    assert!(matches!(cut_short, MidiError::Truncated { offset: 0, .. }));
    assert!(cut_short.to_string().contains("cut short"));

    let short = refuse(&chunk(b"MThd", &[0, 1, 0, 1]));
    assert!(matches!(short, MidiError::BadHeader(_)));

    let empty_track = track(&[common::end(0)]);
    let with =
        |format: u16, division: u16| file(format, division, std::slice::from_ref(&empty_track));
    assert_eq!(refuse(&with(3, 960)), MidiError::UnsupportedFormat(3));
    assert_eq!(
        refuse(&with(0xFFFF, 960)),
        MidiError::UnsupportedFormat(0xFFFF)
    );
    let zero = refuse(&with(1, 0));
    assert!(zero.to_string().contains("zero ticks to the quarter note"));
    let no_frame_ticks = refuse(&with(1, u16::from_be_bytes([(-25_i8) as u8, 0])));
    assert!(matches!(no_frame_ticks, MidiError::BadHeader(_)));
    for frames in [-1_i8, -23, -26, -28, -31, -128] {
        let error = refuse(&with(1, u16::from_be_bytes([frames as u8, 40])));
        assert!(
            error.to_string().contains("frame rate"),
            "{frames}: {error}"
        );
    }
}

#[test]
fn a_file_with_no_track_is_refused() {
    assert_eq!(refuse(&header(1, 0, 960)), MidiError::NoTracks);
    assert_eq!(refuse(&header(1, 3, 960)), MidiError::NoTracks);
    let other = [header(1, 1, 960), chunk(b"JUNK", &[1, 2, 3])].concat();
    assert_eq!(refuse(&other), MidiError::NoTracks);
}

#[test]
fn a_chunk_that_runs_past_the_end_of_the_file_is_refused() {
    let mut bytes = one_note(960, 0, 10);
    bytes.pop();
    assert!(matches!(
        refuse(&bytes),
        MidiError::Truncated { offset: 14, .. }
    ));

    // A length of four gigabytes with nothing behind it.
    let absurd = [header(1, 1, 960), b"MTrk\xFF\xFF\xFF\xFF\x00".to_vec()].concat();
    assert!(matches!(refuse(&absurd), MidiError::Truncated { .. }));

    let half_a_header = [header(1, 2, 960), track(&[common::end(0)]), b"MTr".to_vec()].concat();
    assert!(matches!(
        refuse(&half_a_header),
        MidiError::Truncated { .. }
    ));
}

#[test]
fn a_damaged_track_is_refused_with_the_place_of_the_damage() {
    let damaged = |events: &[Vec<u8>]| refuse(&file(0, 960, &[track(events)]));
    let problem = |error: MidiError| match error {
        MidiError::Corrupt {
            track: 1,
            offset,
            problem,
        } => (offset, problem),
        other => panic!("not a damaged track: {other:?}"),
    };

    // The track ends inside an event. Its body starts at byte 22.
    let (offset, text) = problem(damaged(&[event(0, &[NOTE_ON, 60])]));
    assert_eq!(offset, 25);
    assert!(text.contains("middle of an event"));
    let (_, text) = problem(damaged(&[vec![0x81]]));
    assert!(text.contains("middle of an event"));
    let (_, text) = problem(damaged(&[vec![0x00, 0xFF]]));
    assert!(text.contains("middle of an event"));

    // A time that never ends.
    let (offset, text) = problem(damaged(&[vec![0xFF, 0xFF, 0xFF, 0xFF, 0x7F]]));
    assert_eq!(offset, 26);
    assert!(text.contains("four bytes"));

    // Data with no status before it.
    let (offset, text) = problem(damaged(&[event(0, &[60, 100])]));
    assert_eq!(offset, 23);
    assert!(text.contains("no status byte"));

    // A status byte where a value belongs.
    let (offset, text) = problem(damaged(&[event(0, &[NOTE_ON, 60, 0x90])]));
    assert_eq!(offset, 25);
    assert!(text.contains("still needs a value"));

    // Messages that only exist on a cable.
    for status in [0xF1, 0xF2, 0xF3, 0xF6, 0xF8, 0xFA, 0xFE] {
        let (offset, text) = problem(damaged(&[event(0, &[status, 0])]));
        assert_eq!(offset, 23);
        assert!(text.contains("system message"));
    }

    // Lengths that promise more than the track holds.
    let (_, text) = problem(damaged(&[vec![
        0x00, 0xFF, 0x03, 0xFF, 0xFF, 0xFF, 0x7F, b'x',
    ]]));
    assert!(text.contains("longer than what is left"));
    let (_, text) = problem(damaged(&[vec![0x00, 0xF0, 0x7F, 0x01]]));
    assert!(text.contains("longer than what is left"));

    // The second track is the damaged one.
    let good = track(&[common::end(0)]);
    let bad = track(&[event(0, &[60])]);
    let error = refuse(&file(1, 960, &[good, bad]));
    assert!(matches!(error, MidiError::Corrupt { track: 2, .. }));
    assert!(
        error
            .to_string()
            .starts_with("track 2 of the MIDI file is damaged at byte")
    );
}

#[test]
fn a_file_longer_than_windfall_can_count_is_refused() {
    // One tick to the quarter note makes every tick of the file 960.
    let events = [
        event(0x0FFF_FFFF, &[NOTE_ON, 60, 100]),
        event(1, &[NOTE_OFF, 60, 64]),
        common::end(0),
    ];
    assert_eq!(refuse(&file(0, 1, &[track(&events)])), MidiError::TooLong);

    // The longest wait a file can write, sixteen times over, still fits,
    // and so does everything up to the last tick a `u32` counts.
    let mut events = vec![event(0x0FFF_FFFF, &[0xB0, 1, 1]); 16];
    events.push(event(15, &[NOTE_ON, 60, 100]));
    let song = read(&file(0, 960, &[track(&events)])).expect("the file fits");
    assert_eq!(song.length, u32::MAX);
    // The note struck on that tick is moved back to have a tick to last.
    assert_eq!(only_track(&song).notes, [note(u32::MAX - 1, 1, 60)]);
    events.push(event(1, &[0xB0, 1, 3]));
    assert_eq!(refuse(&file(0, 960, &[track(&events)])), MidiError::TooLong);

    // The patterns of a format 2 file add up.
    let pattern = track(&[common::end(0x0FFF_FFFF)]);
    let patterns = vec![pattern; 17];
    assert_eq!(refuse(&file(2, 960, &patterns)), MidiError::TooLong);
}

#[test]
fn an_input_larger_than_the_limit_is_refused_unread() {
    let bytes = vec![0; MAX_FILE_BYTES + 1];
    let error = refuse(&bytes);
    assert_eq!(
        error,
        MidiError::TooLarge {
            bytes: MAX_FILE_BYTES as u64 + 1,
            limit: MAX_FILE_BYTES as u64
        }
    );
}

fn two_note_song() -> MidiSong {
    MidiSong {
        name: Some("S".to_owned()),
        tracks: vec![MidiTrack {
            name: Some("T".to_owned()),
            notes: vec![note(0, 960, 60), note(960, 960, 62)],
            ..MidiTrack::default()
        }],
        tempos: vec![TempoChange {
            tick: 0,
            micros_per_quarter: 500_000,
        }],
        length: 1_920,
        ..MidiSong::default()
    }
}

#[test]
fn the_writer_produces_these_exact_bytes() {
    let song = two_note_song();
    #[rustfmt::skip]
    let format_1: &[u8] = &[
        b'M', b'T', b'h', b'd', 0, 0, 0, 6, 0, 1, 0, 2, 0x03, 0xC0,
        b'M', b'T', b'r', b'k', 0, 0, 0, 17,
        0x00, 0xFF, 0x03, 0x01, b'S',
        0x00, 0xFF, 0x51, 0x03, 0x07, 0xA1, 0x20,
        0x8F, 0x00, 0xFF, 0x2F, 0x00,
        b'M', b'T', b'r', b'k', 0, 0, 0, 24,
        0x00, 0xFF, 0x03, 0x01, b'T',
        0x00, 0x90, 60, 100,
        0x87, 0x40, 60, 0,
        0x00, 62, 100,
        0x87, 0x40, 62, 0,
        0x00, 0xFF, 0x2F, 0x00,
    ];
    assert_eq!(write(&song, &WriteOptions::default()), format_1);

    #[rustfmt::skip]
    let format_0_at_480_spelled_out: &[u8] = &[
        b'M', b'T', b'h', b'd', 0, 0, 0, 6, 0, 0, 0, 1, 0x01, 0xE0,
        b'M', b'T', b'r', b'k', 0, 0, 0, 34,
        0x00, 0xFF, 0x03, 0x01, b'T',
        0x00, 0xFF, 0x51, 0x03, 0x07, 0xA1, 0x20,
        0x00, 0x90, 60, 100,
        0x83, 0x60, 0x80, 60, 64,
        0x00, 0x90, 62, 100,
        0x83, 0x60, 0x80, 62, 64,
        0x00, 0xFF, 0x2F, 0x00,
    ];
    let options = WriteOptions {
        format: SmfFormat::Single,
        resolution: Resolution::Ppq480,
        running_status: false,
    };
    assert_eq!(write(&song, &options), format_0_at_480_spelled_out);
}

#[test]
fn a_release_velocity_of_its_own_is_written_as_a_real_note_off() {
    let mut song = two_note_song();
    song.tracks[0].notes[0].release = 10;
    let bytes = write(&song, &WriteOptions::default());
    let off = [0x87, 0x40, 0x80, 60, 10];
    assert!(bytes.windows(off.len()).any(|window| window == off));
    assert_eq!(read(&bytes).expect("the file is valid"), song);
}

#[test]
fn a_wait_too_long_for_one_number_is_bridged() {
    let far = 0x0FFF_FFFF * 3 + 5;
    let song = MidiSong {
        tracks: vec![MidiTrack {
            notes: vec![note(2, 1, 60), note(far, 10, 60)],
            ..MidiTrack::default()
        }],
        length: far + 10,
        ..MidiSong::default()
    };
    for running_status in [true, false] {
        let options = WriteOptions {
            running_status,
            ..WriteOptions::default()
        };
        let bytes = write(&song, &options);
        assert!(bytes.len() < 100);
        assert_eq!(read(&bytes).expect("the file is valid"), song);
    }
}

#[test]
fn halving_the_resolution_keeps_every_note_at_least_a_tick_long() {
    let song = MidiSong {
        tracks: vec![MidiTrack {
            notes: vec![
                note(1, 1, 60),
                note(2, 1, 60),
                note(3, 4, 60),
                note(10, 3, 61),
            ],
            ..MidiTrack::default()
        }],
        length: 13,
        ..MidiSong::default()
    };
    let options = WriteOptions {
        resolution: Resolution::Ppq480,
        ..WriteOptions::default()
    };
    let back = read(&write(&song, &options)).expect("the file is valid");
    assert_eq!(
        only_track(&back).notes,
        // 1 and 2 both halve to 1, where the notes keep a tick each; 3 to
        // 7 is 2 to 4; 10 to 13 is 5 to 7.
        [
            note(2, 2, 60),
            note(2, 2, 60),
            note(4, 4, 60),
            note(10, 4, 61)
        ]
    );
    assert_eq!(back.length, 14);
}

#[test]
fn a_song_with_more_tracks_than_a_header_counts_keeps_all_its_notes() {
    let tracks: Vec<MidiTrack> = (0..70_000_u32)
        .map(|index| MidiTrack {
            notes: vec![note(index, 5, (index % 128) as u8)],
            ..MidiTrack::default()
        })
        .collect();
    let song = MidiSong {
        tracks,
        length: 70_005,
        ..MidiSong::default()
    };
    let bytes = write(&song, &WriteOptions::default());
    assert_eq!(&bytes[10..12], &[0xFF, 0xFF]);
    let back = read(&bytes).expect("the file is valid");
    assert_eq!(back.tracks.len(), 65_534);
    assert_eq!(back.note_count(), 70_000);
    assert_eq!(back.tracks[65_532], song.tracks[65_532]);
    assert_eq!(back.tracks[65_533].notes.len(), 70_000 - 65_533);
}

#[test]
fn an_empty_song_is_a_valid_file_in_both_formats() {
    let empty = MidiSong::default();
    let multi = write(&empty, &WriteOptions::default());
    assert_eq!(read(&multi).expect("the file is valid"), empty);

    let options = WriteOptions {
        format: SmfFormat::Single,
        ..WriteOptions::default()
    };
    let single = read(&write(&empty, &options)).expect("the file is valid");
    assert_eq!(single, empty.flattened());
}

#[test]
fn files_are_read_and_written_on_disk() {
    let folder = tempfile::tempdir().expect("a folder to write in");
    let path = folder.path().join("nested").join("song.mid");
    let song = two_note_song();
    write_file(&path, &song, &WriteOptions::default()).expect("the file is written");
    assert_eq!(read_file(&path).expect("the file is read"), song);

    let missing = read_file(folder.path().join("nothing.mid")).expect_err("there is no file");
    assert!(matches!(missing, MidiError::Io { .. }));
    assert!(missing.to_string().contains("nothing.mid"));

    let large = folder.path().join("large.mid");
    std::fs::write(&large, vec![0; MAX_FILE_BYTES + 2]).expect("the file is written");
    assert_eq!(
        read_file(&large).expect_err("the file is too large"),
        MidiError::TooLarge {
            bytes: MAX_FILE_BYTES as u64 + 2,
            limit: MAX_FILE_BYTES as u64
        }
    );

    let not_midi = folder.path().join("text.mid");
    std::fs::write(&not_midi, b"hello").expect("the file is written");
    assert_eq!(read_file(&not_midi), Err(MidiError::NotMidi));
}
