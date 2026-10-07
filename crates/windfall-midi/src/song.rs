//! [`MidiSong`]: what a MIDI file says, in Windfall's own time.
//!
//! Reading a file gives a `MidiSong`, and a `MidiSong` is what gets written
//! and what a project is imported from and exported to. Every tick in it is
//! a Windfall tick, [`PPQ`] to the quarter note, whatever resolution the
//! file had.
//!
//! It keeps more than a project can hold yet: program changes, controllers,
//! pitch bend, aftertouch, key signatures and markers all stay, so that
//! nothing is lost between reading a file and deciding what to do with it.
//!
//! # The normal form
//!
//! Many `MidiSong` values mean the same file. [`MidiSong::normalized`]
//! picks one of them: lists are sorted, values are inside what a MIDI file
//! can say, and notes that overlap on one key are paired the only way a
//! file can pair them. Reading always gives the normal form, and writing a
//! song and reading it back gives the song's normal form.

use std::collections::BTreeMap;

use windfall_core::PPQ;

/// The tempo a MIDI file has until it says otherwise: 120 beats a minute,
/// in microseconds per quarter note.
pub const DEFAULT_MICROS_PER_QUARTER: u32 = 500_000;

/// The slowest tempo a MIDI file can hold: 24 bits of microseconds per
/// quarter note, about 3.58 beats a minute.
pub const MAX_MICROS_PER_QUARTER: u32 = 0xFF_FFFF;

/// The channel General MIDI keeps for drums, counted from 0. Musicians call
/// it channel 10.
pub const DRUM_CHANNEL: u8 = 9;

/// The release velocity of a note whose file did not give one.
pub const DEFAULT_RELEASE: u8 = 64;

/// The pitch bend value that bends nothing.
pub const PITCH_BEND_CENTER: u16 = 8192;

/// Controller number of the channel volume.
pub const CC_VOLUME: u8 = 7;

/// Controller number of the channel pan.
pub const CC_PAN: u8 = 10;

/// A whole MIDI file, with every time in Windfall ticks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MidiSong {
    /// The name of the sequence: what the first track of a multi-track
    /// file is called when it only holds the tempo.
    pub name: Option<String>,
    /// The tracks that can hold notes, in file order.
    pub tracks: Vec<MidiTrack>,
    /// Sorted by tick. Before the first one the tempo is
    /// [`DEFAULT_MICROS_PER_QUARTER`]. Of several on one tick the last
    /// counts.
    pub tempos: Vec<TempoChange>,
    /// Sorted by tick. Before the first one the song is in 4/4.
    pub time_signatures: Vec<TimeSignatureChange>,
    /// Sorted by tick.
    pub key_signatures: Vec<KeySignatureChange>,
    /// Sorted by tick.
    pub markers: Vec<Marker>,
    /// Where the song ends, in ticks: the latest end of any track. A file
    /// can end after its last note, which is how a loop says how long it
    /// is. It is never before the end of the last note or event.
    pub length: u32,
}

/// One track of a file: the notes and channel messages it holds.
///
/// A track of a multi-track file usually plays on one MIDI channel. The
/// single track of a format 0 file holds every channel at once, which is
/// why each note and message carries its channel.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MidiTrack {
    pub name: Option<String>,
    /// The instrument name the file gives the track, which is free text and
    /// not a program number.
    pub instrument: Option<String>,
    /// Sorted by start, then key, channel, length, velocity and release.
    pub notes: Vec<MidiNote>,
    /// Sorted by tick.
    pub programs: Vec<ProgramChange>,
    /// One curve for each channel and kind of message that the track
    /// holds, sorted by channel and then kind. None is empty.
    pub controls: Vec<ControlCurve>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MidiNote {
    /// Start in ticks from the beginning of the song.
    pub start: u32,
    /// Length in ticks, at least 1.
    pub length: u32,
    /// MIDI key, 0 to 127.
    pub key: u8,
    /// How hard the key was struck, 1 to 127.
    pub velocity: u8,
    /// How fast the key was let go, 0 to 127. [`DEFAULT_RELEASE`] when the
    /// file did not say.
    pub release: u8,
    /// MIDI channel, 0 to 15.
    pub channel: u8,
}

impl MidiNote {
    /// The tick the note ends on.
    pub fn end(&self) -> u32 {
        self.start.saturating_add(self.length)
    }

    /// The order notes keep inside a track.
    pub(crate) fn order(&self) -> (u32, u8, u8, u32, u8, u8) {
        (
            self.start,
            self.key,
            self.channel,
            self.length,
            self.velocity,
            self.release,
        )
    }
}

/// A change of sound on one channel: a General MIDI program number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProgramChange {
    pub tick: u32,
    /// MIDI channel, 0 to 15.
    pub channel: u8,
    /// Program number, 0 to 127.
    pub program: u8,
}

/// Every message of one kind on one channel of a track, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlCurve {
    /// MIDI channel, 0 to 15.
    pub channel: u8,
    pub kind: ControlKind,
    /// Sorted by tick. Of several on one tick the last counts.
    pub points: Vec<ControlPoint>,
}

/// What a [`ControlCurve`] moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ControlKind {
    /// A control change with this controller number, 0 to 127. Values run
    /// from 0 to 127.
    Controller(u8),
    /// The pitch wheel. Values run from 0 to 16,383 with
    /// [`PITCH_BEND_CENTER`] at rest.
    PitchBend,
    /// Aftertouch for the whole channel. Values run from 0 to 127.
    ChannelPressure,
    /// Aftertouch for this one key, 0 to 127. Values run from 0 to 127.
    KeyPressure(u8),
}

impl ControlKind {
    /// The largest value a message of this kind can carry.
    pub fn max_value(self) -> u16 {
        match self {
            ControlKind::PitchBend => 0x3FFF,
            ControlKind::Controller(_)
            | ControlKind::ChannelPressure
            | ControlKind::KeyPressure(_) => 0x7F,
        }
    }

    fn clamped(self) -> Self {
        match self {
            ControlKind::Controller(number) => ControlKind::Controller(number.min(0x7F)),
            ControlKind::KeyPressure(key) => ControlKind::KeyPressure(key.min(0x7F)),
            ControlKind::PitchBend | ControlKind::ChannelPressure => self,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ControlPoint {
    pub tick: u32,
    /// From 0 to [`ControlKind::max_value`].
    pub value: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TempoChange {
    pub tick: u32,
    /// Length of a quarter note in microseconds, 1 to
    /// [`MAX_MICROS_PER_QUARTER`]. This is how a file stores a tempo, so it
    /// is kept as it is and never goes through a rounded bpm figure.
    pub micros_per_quarter: u32,
}

impl TempoChange {
    /// A tempo change to `bpm` beats a minute, rounded to the nearest
    /// microsecond per quarter note and held to what a file can store.
    pub fn from_bpm(tick: u32, bpm: f64) -> Self {
        Self {
            tick,
            micros_per_quarter: micros_per_quarter(bpm),
        }
    }

    /// The tempo in beats per minute.
    pub fn bpm(&self) -> f64 {
        bpm(self.micros_per_quarter)
    }
}

/// The tempo, in beats per minute, of a quarter note that lasts `micros`
/// microseconds.
pub fn bpm(micros: u32) -> f64 {
    60_000_000.0 / f64::from(micros.max(1))
}

/// The length of a quarter note at `bpm` beats a minute, rounded to the
/// nearest microsecond and held to 1 to [`MAX_MICROS_PER_QUARTER`]. A tempo
/// that is not a positive number gives [`DEFAULT_MICROS_PER_QUARTER`].
pub fn micros_per_quarter(bpm: f64) -> u32 {
    if bpm.is_nan() || bpm <= 0.0 {
        return DEFAULT_MICROS_PER_QUARTER;
    }
    let micros = (60_000_000.0 / bpm).round();
    // The cast saturates, and the clamp brings what is left into range.
    (micros as u32).clamp(1, MAX_MICROS_PER_QUARTER)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeSignatureChange {
    pub tick: u32,
    /// Beats per bar, at least 1.
    pub numerator: u8,
    /// Beat unit: a power of two from 1 to 128.
    pub denominator: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeySignatureChange {
    pub tick: u32,
    /// Sharps in the key signature, 1 to 7, or flats as a negative number,
    /// -1 to -7. 0 is C major or A minor.
    pub sharps: i8,
    pub minor: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Marker {
    pub tick: u32,
    pub text: String,
}

impl MidiSong {
    /// The song in its normal form, which is what reading a file gives and
    /// what writing a song and reading it back gives:
    ///
    /// - Every list is in the order its field documents. Things on one
    ///   tick keep the order they had.
    /// - Every value is inside the range its field documents. A value
    ///   outside is brought to the nearest end.
    /// - Names lose trailing NUL characters, and an empty name is no name.
    /// - Curves of one channel and kind on one track are joined, and empty
    ///   curves are dropped.
    /// - `length` is at least the end of the last note or event.
    /// - Notes that overlap on one key of one channel of one track are
    ///   paired first in, first out. A MIDI file has only "key down" and
    ///   "key up", so when a key is struck twice before it is let go, the
    ///   first "up" ends the note that began first. Two notes of which one
    ///   lies inside the other cannot be told from two that overlap, and
    ///   become those: the earlier start gets the earlier end. Velocities
    ///   stay with the starts and release velocities with the ends.
    pub fn normalized(mut self) -> Self {
        self.name = tidy_name(self.name);
        for track in &mut self.tracks {
            track.normalize();
        }
        for tempo in &mut self.tempos {
            tempo.micros_per_quarter = tempo.micros_per_quarter.clamp(1, MAX_MICROS_PER_QUARTER);
        }
        self.tempos.sort_by_key(|tempo| tempo.tick);
        for signature in &mut self.time_signatures {
            signature.numerator = signature.numerator.max(1);
            signature.denominator = signature.denominator.clamp(1, 128).next_power_of_two();
        }
        self.time_signatures.sort_by_key(|signature| signature.tick);
        for key in &mut self.key_signatures {
            key.sharps = key.sharps.clamp(-7, 7);
        }
        self.key_signatures.sort_by_key(|key| key.tick);
        for marker in &mut self.markers {
            marker.text.truncate(tidy_len(&marker.text));
        }
        self.markers.sort_by_key(|marker| marker.tick);
        self.length = self.end();
        self
    }

    /// The tick everything in the song is over by: `length`, or the end of
    /// the last note or event if that is later.
    pub fn end(&self) -> u32 {
        let tracks = self.tracks.iter().map(MidiTrack::end);
        let ticks = (self.tempos.iter().map(|tempo| tempo.tick))
            .chain(self.time_signatures.iter().map(|signature| signature.tick))
            .chain(self.key_signatures.iter().map(|key| key.tick))
            .chain(self.markers.iter().map(|marker| marker.tick));
        tracks.chain(ticks).fold(self.length, u32::max)
    }

    /// The song with all its tracks merged into one, which is what a
    /// format 0 file holds. Notes and messages keep their channels.
    ///
    /// A format 0 file has room for one name. A song of one track keeps
    /// that track's name and instrument, or the song's name if the track
    /// has none. A song of several tracks keeps the song's name and no
    /// instrument. The result always has exactly one track and no name of
    /// its own.
    pub fn flattened(&self) -> Self {
        let mut track = merged(&self.tracks);
        match self.tracks.as_slice() {
            [only] => {
                track.name = only.name.clone().or_else(|| self.name.clone());
                track.instrument = only.instrument.clone();
            }
            _ => track.name = self.name.clone(),
        }
        Self {
            name: None,
            tracks: vec![track],
            tempos: self.tempos.clone(),
            time_signatures: self.time_signatures.clone(),
            key_signatures: self.key_signatures.clone(),
            markers: self.markers.clone(),
            length: self.length,
        }
    }

    /// The length of a quarter note, in microseconds, at `tick`.
    pub fn micros_per_quarter_at(&self, tick: u32) -> u32 {
        let after = self.tempos.partition_point(|tempo| tempo.tick <= tick);
        after
            .checked_sub(1)
            .and_then(|index| self.tempos.get(index))
            .map_or(DEFAULT_MICROS_PER_QUARTER, |tempo| tempo.micros_per_quarter)
    }

    /// How many notes the song holds.
    pub fn note_count(&self) -> usize {
        self.tracks.iter().map(|track| track.notes.len()).sum()
    }
}

impl MidiTrack {
    /// The tick the track's last note or message is over by.
    pub fn end(&self) -> u32 {
        let notes = self.notes.iter().map(MidiNote::end);
        let programs = self.programs.iter().map(|program| program.tick);
        let controls = self.controls.iter().flat_map(|curve| &curve.points);
        notes
            .chain(programs)
            .chain(controls.map(|point| point.tick))
            .max()
            .unwrap_or(0)
    }

    /// The channels the track has notes on, in order.
    pub fn note_channels(&self) -> Vec<u8> {
        let mut channels: Vec<u8> = self.notes.iter().map(|note| note.channel).collect();
        channels.sort_unstable();
        channels.dedup();
        channels
    }

    /// The curve of one kind of message on one channel, if the track has
    /// any such message.
    pub fn control(&self, channel: u8, kind: ControlKind) -> Option<&ControlCurve> {
        self.controls
            .iter()
            .find(|curve| curve.channel == channel && curve.kind == kind)
    }

    /// The first program the track sets on a channel.
    pub fn program(&self, channel: u8) -> Option<u8> {
        self.programs
            .iter()
            .find(|program| program.channel == channel)
            .map(|program| program.program)
    }

    pub(crate) fn normalize(&mut self) {
        self.name = tidy_name(self.name.take());
        self.instrument = tidy_name(self.instrument.take());

        for note in &mut self.notes {
            note.key = note.key.min(0x7F);
            note.channel = note.channel.min(0x0F);
            note.velocity = note.velocity.clamp(1, 0x7F);
            note.release = note.release.min(0x7F);
            note.length = note.length.clamp(1, (u32::MAX - note.start).max(1));
            // A note on the very last tick has no room to last one.
            note.start = note.start.min(u32::MAX - 1);
        }
        pair_first_in_first_out(&mut self.notes);

        for program in &mut self.programs {
            program.channel = program.channel.min(0x0F);
            program.program = program.program.min(0x7F);
        }
        self.programs.sort_by_key(|program| program.tick);

        let mut curves: BTreeMap<(u8, ControlKind), Vec<ControlPoint>> = BTreeMap::new();
        for curve in self.controls.drain(..) {
            let kind = curve.kind.clamped();
            let points = curves.entry((curve.channel.min(0x0F), kind)).or_default();
            let max = kind.max_value();
            points.extend(curve.points.into_iter().map(|point| ControlPoint {
                tick: point.tick,
                value: point.value.min(max),
            }));
        }
        self.controls = curves
            .into_iter()
            .filter(|(_, points)| !points.is_empty())
            .map(|((channel, kind), mut points)| {
                points.sort_by_key(|point| point.tick);
                ControlCurve {
                    channel,
                    kind,
                    points,
                }
            })
            .collect();
    }
}

/// The tracks merged into one that has no name: every note, program change
/// and message of all of them. Things on one tick keep the order of the
/// tracks they came from.
pub(crate) fn merged(tracks: &[MidiTrack]) -> MidiTrack {
    let mut all = MidiTrack::default();
    for track in tracks {
        all.notes.extend_from_slice(&track.notes);
        all.programs.extend_from_slice(&track.programs);
        all.controls.extend(track.controls.iter().cloned());
    }
    all
}

/// Sorts the notes and pairs the starts and ends of those that share a key
/// and a channel the way a file would: the note that starts first ends
/// first. See [`MidiSong::normalized`].
fn pair_first_in_first_out(notes: &mut [MidiNote]) {
    notes.sort_unstable_by_key(MidiNote::order);

    let keys: Vec<(u8, u8)> = notes.iter().map(|note| (note.channel, note.key)).collect();
    let mut by_key: Vec<usize> = (0..notes.len()).collect();
    by_key.sort_by_key(|&index| (keys[index], index));
    let mut repaired = false;
    for group in by_key.chunk_by(|&a, &b| keys[a] == keys[b]) {
        // The order a file lists the ends in: by tick, and on one tick the
        // note that began first.
        let mut ends = group.to_vec();
        ends.sort_by_key(|&index| (notes[index].end(), notes[index].start, index));
        if ends == group {
            continue;
        }
        let closed: Vec<(u32, u8)> = ends
            .iter()
            .map(|&index| (notes[index].end(), notes[index].release))
            .collect();
        for (&index, (end, release)) in group.iter().zip(closed) {
            notes[index].length = end - notes[index].start;
            notes[index].release = release;
        }
        repaired = true;
    }
    if repaired {
        notes.sort_unstable_by_key(MidiNote::order);
    }
}

/// How much of `text` is left once trailing NUL characters are cut, in
/// bytes. Files written by C programs often end their names with one.
fn tidy_len(text: &str) -> usize {
    text.trim_end_matches('\0').len()
}

fn tidy_name(name: Option<String>) -> Option<String> {
    let mut name = name?;
    name.truncate(tidy_len(&name));
    (!name.is_empty()).then_some(name)
}

/// The text of a meta event. A MIDI file does not say how its text is
/// encoded: it is read as UTF-8 when it is valid UTF-8, and as Latin-1
/// otherwise, which gives every byte a character and so never fails.
pub(crate) fn decode_text(bytes: &[u8]) -> String {
    let mut text = match std::str::from_utf8(bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => bytes.iter().map(|&byte| char::from(byte)).collect(),
    };
    text.truncate(tidy_len(&text));
    text
}

/// Rescales `ticks` counted `from` to the quarter note into ticks counted
/// `to` to the quarter note: `ticks * to / from`, worked out exactly and
/// rounded to the nearest whole tick, with a half rounding up. A result too
/// large for a `u64` is the largest `u64`.
pub(crate) fn rescale(ticks: u64, to: u64, from: u64) -> u64 {
    let from = u128::from(from.max(1));
    let scaled = (u128::from(ticks) * u128::from(to) * 2 + from) / (from * 2);
    u64::try_from(scaled).unwrap_or(u64::MAX)
}

/// Windfall's resolution as the wide integer the time arithmetic uses.
pub(crate) const TICKS_PER_QUARTER: u64 = PPQ as u64;

#[cfg(test)]
mod tests {
    use super::*;

    fn note(start: u32, length: u32, key: u8) -> MidiNote {
        MidiNote {
            start,
            length,
            key,
            velocity: 100,
            release: DEFAULT_RELEASE,
            channel: 0,
        }
    }

    #[test]
    fn rescaling_is_exact_and_rounds_a_half_up() {
        assert_eq!(rescale(480, 960, 480), 960);
        assert_eq!(rescale(1, 960, 96), 10);
        assert_eq!(rescale(7, 960, 384), 18); // 17.5
        assert_eq!(rescale(1, 960, 1920), 1); // 0.5
        assert_eq!(rescale(1, 960, 1921), 0);
        assert_eq!(rescale(3, 480, 960), 2); // 1.5
        assert_eq!(rescale(1, 960, 3), 320);
        assert_eq!(rescale(2, 960, 7), 274); // 274.29
        assert_eq!(rescale(u64::MAX, 960, 1), u64::MAX);
        assert_eq!(rescale(u64::MAX, 1, 1), u64::MAX);
    }

    #[test]
    fn tempo_converts_to_microseconds_and_back() {
        assert_eq!(micros_per_quarter(120.0), 500_000);
        assert_eq!(micros_per_quarter(140.0), 428_571);
        assert_eq!(bpm(500_000), 120.0);
        assert_eq!(micros_per_quarter(0.0), DEFAULT_MICROS_PER_QUARTER);
        assert_eq!(micros_per_quarter(f64::NAN), DEFAULT_MICROS_PER_QUARTER);
        assert_eq!(micros_per_quarter(1.0), MAX_MICROS_PER_QUARTER);
        assert_eq!(micros_per_quarter(f64::INFINITY), 1);
        assert_eq!(
            TempoChange::from_bpm(7, 90.0).bpm(),
            60_000_000.0 / 666_667.0
        );
    }

    #[test]
    fn a_note_inside_another_on_the_same_key_becomes_two_that_overlap() {
        let mut outer = note(0, 100, 60);
        outer.velocity = 90;
        outer.release = 10;
        let mut inner = note(20, 30, 60);
        inner.velocity = 50;
        inner.release = 20;
        let song = MidiSong {
            tracks: vec![MidiTrack {
                notes: vec![inner, outer, note(0, 100, 61)],
                ..MidiTrack::default()
            }],
            ..MidiSong::default()
        }
        .normalized();
        let notes = &song.tracks[0].notes;
        assert_eq!(notes.len(), 3);
        // The first start gets the first end and its release velocity.
        assert_eq!((notes[0].start, notes[0].length), (0, 50));
        assert_eq!((notes[0].velocity, notes[0].release), (90, 20));
        assert_eq!(notes[1], note(0, 100, 61));
        assert_eq!((notes[2].start, notes[2].length), (20, 80));
        assert_eq!((notes[2].velocity, notes[2].release), (50, 10));
        assert_eq!(song.length, 100);
        assert_eq!(song.clone().normalized(), song);
    }

    #[test]
    fn the_normal_form_brings_values_into_range_and_joins_curves() {
        let curve = |channel, kind, tick, value| ControlCurve {
            channel,
            kind,
            points: vec![ControlPoint { tick, value }],
        };
        let song = MidiSong {
            name: Some("Song\0\0".to_owned()),
            tracks: vec![MidiTrack {
                name: Some("\0".to_owned()),
                instrument: None,
                notes: vec![MidiNote {
                    start: u32::MAX,
                    length: 0,
                    key: 200,
                    velocity: 0,
                    release: 255,
                    channel: 99,
                }],
                programs: vec![ProgramChange {
                    tick: 3,
                    channel: 16,
                    program: 128,
                }],
                controls: vec![
                    curve(0, ControlKind::PitchBend, 9, 60_000),
                    curve(0, ControlKind::Controller(7), 5, 300),
                    curve(0, ControlKind::PitchBend, 2, 1),
                    ControlCurve {
                        channel: 4,
                        kind: ControlKind::ChannelPressure,
                        points: Vec::new(),
                    },
                ],
            }],
            tempos: vec![
                TempoChange {
                    tick: 10,
                    micros_per_quarter: 0,
                },
                TempoChange {
                    tick: 0,
                    micros_per_quarter: u32::MAX,
                },
            ],
            time_signatures: vec![TimeSignatureChange {
                tick: 0,
                numerator: 0,
                denominator: 6,
            }],
            key_signatures: vec![KeySignatureChange {
                tick: 0,
                sharps: -100,
                minor: true,
            }],
            markers: vec![Marker {
                tick: 4,
                text: "Verse\0".to_owned(),
            }],
            length: 0,
        }
        .normalized();

        assert_eq!(song.name.as_deref(), Some("Song"));
        let track = &song.tracks[0];
        assert_eq!(track.name, None);
        assert_eq!(
            track.notes,
            [MidiNote {
                start: u32::MAX - 1,
                length: 1,
                key: 127,
                velocity: 1,
                release: 127,
                channel: 15,
            }]
        );
        assert_eq!(track.programs[0].channel, 15);
        assert_eq!(track.programs[0].program, 127);
        assert_eq!(track.controls.len(), 2);
        assert_eq!(track.controls[0].kind, ControlKind::Controller(7));
        assert_eq!(track.controls[0].points[0].value, 127);
        assert_eq!(
            track.controls[1].points,
            [
                ControlPoint { tick: 2, value: 1 },
                ControlPoint {
                    tick: 9,
                    value: 0x3FFF
                },
            ]
        );
        assert_eq!(song.tempos[0].micros_per_quarter, MAX_MICROS_PER_QUARTER);
        assert_eq!(song.tempos[1].micros_per_quarter, 1);
        assert_eq!(song.time_signatures[0].numerator, 1);
        assert_eq!(song.time_signatures[0].denominator, 8);
        assert_eq!(song.key_signatures[0].sharps, -7);
        assert_eq!(song.markers[0].text, "Verse");
        assert_eq!(song.length, u32::MAX);
        assert_eq!(song.clone().normalized(), song);
    }

    #[test]
    fn flattening_merges_the_tracks_and_keeps_one_name() {
        let track = |name: &str, key| MidiTrack {
            name: Some(name.to_owned()),
            instrument: Some("Grand".to_owned()),
            notes: vec![note(0, 10, key)],
            ..MidiTrack::default()
        };
        let mut song = MidiSong {
            name: Some("Song".to_owned()),
            tracks: vec![track("Piano", 60), track("Bass", 36)],
            length: 10,
            ..MidiSong::default()
        };
        let flat = song.flattened();
        assert_eq!(flat.name, None);
        assert_eq!(flat.tracks.len(), 1);
        assert_eq!(flat.tracks[0].name.as_deref(), Some("Song"));
        assert_eq!(flat.tracks[0].instrument, None);
        assert_eq!(flat.note_count(), 2);

        song.tracks.pop();
        let flat = song.flattened();
        assert_eq!(flat.tracks[0].name.as_deref(), Some("Piano"));
        assert_eq!(flat.tracks[0].instrument.as_deref(), Some("Grand"));

        let empty = MidiSong::default().flattened();
        assert_eq!(empty.tracks, [MidiTrack::default()]);
    }

    #[test]
    fn the_tempo_is_the_default_until_the_file_sets_one() {
        let song = MidiSong {
            tempos: vec![
                TempoChange {
                    tick: 960,
                    micros_per_quarter: 600_000,
                },
                TempoChange {
                    tick: 960,
                    micros_per_quarter: 400_000,
                },
            ],
            ..MidiSong::default()
        };
        assert_eq!(song.micros_per_quarter_at(959), DEFAULT_MICROS_PER_QUARTER);
        assert_eq!(song.micros_per_quarter_at(960), 400_000);
        assert_eq!(song.micros_per_quarter_at(u32::MAX), 400_000);
    }

    #[test]
    fn text_is_read_as_utf8_and_falls_back_to_latin1() {
        assert_eq!(decode_text("Flöte".as_bytes()), "Flöte");
        assert_eq!(decode_text(&[b'F', b'l', 0xF6, b't', b'e']), "Flöte");
        assert_eq!(decode_text(b"Piano\0\0"), "Piano");
        assert_eq!(decode_text(b""), "");
    }
}
