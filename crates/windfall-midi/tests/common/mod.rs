//! Helpers the test files share: MIDI files built byte by byte, and random
//! songs.
#![allow(dead_code)]

use proptest::collection::vec;
use proptest::prelude::*;
use windfall_midi::*;

/// A variable-length number, as a file writes a time or a length.
pub fn number(value: u32) -> Vec<u8> {
    let mut bytes = vec![(value & 0x7F) as u8];
    let mut rest = value >> 7;
    while rest > 0 {
        bytes.insert(0, (rest & 0x7F) as u8 | 0x80);
        rest >>= 7;
    }
    bytes
}

/// An event: a wait of `delta` ticks and then `bytes`.
pub fn event(delta: u32, bytes: &[u8]) -> Vec<u8> {
    [number(delta).as_slice(), bytes].concat()
}

/// A meta event of `kind` holding `data`, after a wait of `delta` ticks.
pub fn meta(delta: u32, kind: u8, data: &[u8]) -> Vec<u8> {
    let length = number(data.len() as u32);
    [
        number(delta).as_slice(),
        &[0xFF, kind],
        length.as_slice(),
        data,
    ]
    .concat()
}

/// The end of a track, `delta` ticks after its last event.
pub fn end(delta: u32) -> Vec<u8> {
    meta(delta, 0x2F, &[])
}

/// A tempo change to `micros` microseconds per quarter note.
pub fn tempo(delta: u32, micros: u32) -> Vec<u8> {
    meta(delta, 0x51, &micros.to_be_bytes()[1..])
}

/// A chunk: four letters, the length of the body, the body.
pub fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let length = (body.len() as u32).to_be_bytes();
    [id.as_slice(), length.as_slice(), body].concat()
}

/// A track chunk made of these events.
pub fn track(events: &[Vec<u8>]) -> Vec<u8> {
    chunk(b"MTrk", &events.concat())
}

/// A header chunk.
pub fn header(format: u16, tracks: u16, division: u16) -> Vec<u8> {
    let body = [
        format.to_be_bytes(),
        tracks.to_be_bytes(),
        division.to_be_bytes(),
    ]
    .concat();
    chunk(b"MThd", &body)
}

/// A whole file with these track chunks, counted right in the header.
pub fn file(format: u16, division: u16, tracks: &[Vec<u8>]) -> Vec<u8> {
    [
        header(format, tracks.len() as u16, division),
        tracks.concat(),
    ]
    .concat()
}

/// A note as most tests want it: on channel 0, struck at 100, with the
/// release velocity of a file that gives none.
pub fn note(start: u32, length: u32, key: u8) -> MidiNote {
    MidiNote {
        start,
        length,
        key,
        velocity: 100,
        release: DEFAULT_RELEASE,
        channel: 0,
    }
}

/// How far a random song strays from what a file can say.
#[derive(Debug, Clone, Copy)]
pub struct Wildness {
    /// Values may lie outside their ranges, and lists may be out of order.
    pub out_of_range: bool,
    /// The last tick anything may be on.
    pub ticks: u32,
}

impl Wildness {
    /// Anything a `MidiSong` can hold.
    pub const ANY: Self = Self {
        out_of_range: true,
        ticks: 40_000,
    };

    /// Values inside their ranges, as a real file has them.
    pub const TAME: Self = Self {
        out_of_range: false,
        ticks: 40_000,
    };
}

fn text() -> impl Strategy<Value = String> {
    prop_oneof![
        "[A-Za-z0-9 #]{0,12}",
        "[a-zäöüß€♪]{1,6}",
        Just("Lead\0".to_owned()),
    ]
}

fn name() -> impl Strategy<Value = Option<String>> {
    prop::option::of(text())
}

fn byte(wild: Wildness, max: u8) -> BoxedStrategy<u8> {
    if wild.out_of_range {
        prop_oneof![9 => 0..=max, 1 => any::<u8>()].boxed()
    } else {
        (0..=max).boxed()
    }
}

fn midi_note(wild: Wildness) -> impl Strategy<Value = MidiNote> {
    // A few keys and channels, so that notes often meet on one key.
    let key = prop_oneof![4 => 58_u8..63, 1 => byte(wild, 127)];
    let velocity = if wild.out_of_range {
        byte(wild, 127)
    } else {
        (1_u8..=127).boxed()
    };
    let release = prop_oneof![1 => Just(DEFAULT_RELEASE), 1 => byte(wild, 127)];
    let length = if wild.out_of_range {
        (0_u32..3_000).boxed()
    } else {
        (1_u32..3_000).boxed()
    };
    (0..wild.ticks, length, key, velocity, release, byte(wild, 3)).prop_map(
        |(start, length, key, velocity, release, channel)| MidiNote {
            start,
            length,
            key,
            velocity,
            release,
            channel,
        },
    )
}

fn control_kind(wild: Wildness) -> impl Strategy<Value = ControlKind> {
    prop_oneof![
        Just(ControlKind::Controller(CC_VOLUME)),
        Just(ControlKind::Controller(CC_PAN)),
        byte(wild, 127).prop_map(ControlKind::Controller),
        Just(ControlKind::PitchBend),
        Just(ControlKind::ChannelPressure),
        byte(wild, 127).prop_map(ControlKind::KeyPressure),
    ]
}

fn control_curve(wild: Wildness) -> impl Strategy<Value = ControlCurve> {
    let value = if wild.out_of_range {
        any::<u16>().boxed()
    } else {
        (0_u16..=127).boxed()
    };
    let point = (0..wild.ticks, value).prop_map(|(tick, value)| ControlPoint { tick, value });
    (byte(wild, 3), control_kind(wild), vec(point, 0..6)).prop_map(|(channel, kind, points)| {
        ControlCurve {
            channel,
            kind,
            points,
        }
    })
}

fn midi_track(wild: Wildness) -> impl Strategy<Value = MidiTrack> {
    let program =
        (0..wild.ticks, byte(wild, 3), byte(wild, 127)).prop_map(|(tick, channel, program)| {
            ProgramChange {
                tick,
                channel,
                program,
            }
        });
    (
        name(),
        name(),
        vec(midi_note(wild), 0..24),
        vec(program, 0..4),
        vec(control_curve(wild), 0..4),
    )
        .prop_map(|(name, instrument, notes, programs, controls)| MidiTrack {
            name,
            instrument,
            notes,
            programs,
            controls,
        })
}

/// A random song. With [`Wildness::TAME`] every value is inside its range,
/// though the song is still not in its normal form: lists are unsorted and
/// notes on one key may lie inside each other.
pub fn midi_song(wild: Wildness) -> impl Strategy<Value = MidiSong> {
    let micros = if wild.out_of_range {
        any::<u32>().boxed()
    } else {
        (1_u32..=MAX_MICROS_PER_QUARTER).boxed()
    };
    let tempo = (0..wild.ticks, micros).prop_map(|(tick, micros_per_quarter)| TempoChange {
        tick,
        micros_per_quarter,
    });
    let numerator = if wild.out_of_range {
        any::<u8>().boxed()
    } else {
        (1_u8..=32).boxed()
    };
    let denominator = if wild.out_of_range {
        any::<u8>().boxed()
    } else {
        (0_u32..8).prop_map(|power| 1 << power).boxed()
    };
    let time_signature =
        (0..wild.ticks, numerator, denominator).prop_map(|(tick, numerator, denominator)| {
            TimeSignatureChange {
                tick,
                numerator,
                denominator,
            }
        });
    let sharps = if wild.out_of_range {
        any::<i8>().boxed()
    } else {
        (-7_i8..=7).boxed()
    };
    let key_signature = (0..wild.ticks, sharps, any::<bool>()).prop_map(|(tick, sharps, minor)| {
        KeySignatureChange {
            tick,
            sharps,
            minor,
        }
    });
    let marker = (0..wild.ticks, text()).prop_map(|(tick, text)| Marker { tick, text });
    (
        name(),
        vec(midi_track(wild), 0..4),
        vec(tempo, 0..5),
        vec(time_signature, 0..3),
        vec(key_signature, 0..3),
        vec(marker, 0..3),
        0..wild.ticks * 2,
    )
        .prop_map(
            |(name, tracks, tempos, time_signatures, key_signatures, markers, length)| MidiSong {
                name,
                tracks,
                tempos,
                time_signatures,
                key_signatures,
                markers,
                length,
            },
        )
}
