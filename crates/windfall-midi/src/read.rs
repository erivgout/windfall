//! Reading a Standard MIDI File into a [`MidiSong`].
//!
//! The reader trusts nothing in the file. Every length is checked against
//! the bytes that are really there before anything is taken, nothing is
//! allocated on the word of a header, and whatever is wrong comes back as a
//! [`MidiError`]: no input makes it panic.
//!
//! It is forgiving where the meaning is not in doubt. Chunks it does not
//! know are skipped, as are system exclusive messages and meta events it has
//! no use for. A track may lack its end marker, running status may carry on
//! across a meta event, the header may count more tracks than the file
//! has, and bytes after the last track are ignored.

use std::collections::{BTreeMap, HashMap, VecDeque};

use crate::error::MidiError;
use crate::song::{
    ControlCurve, ControlKind, ControlPoint, DEFAULT_MICROS_PER_QUARTER, DEFAULT_RELEASE,
    KeySignatureChange, Marker, MidiNote, MidiSong, MidiTrack, ProgramChange, TICKS_PER_QUARTER,
    TempoChange, TimeSignatureChange, decode_text, rescale,
};

/// The largest input [`read`] accepts, 16 MiB. A MIDI file of a whole
/// symphony is a few hundred kilobytes.
pub const MAX_FILE_BYTES: usize = 16 * 1024 * 1024;

/// The most tracks read from one file, which is also the most a header can
/// count.
const MAX_TRACKS: usize = u16::MAX as usize;

const META_TRACK_NAME: u8 = 0x03;
const META_INSTRUMENT_NAME: u8 = 0x04;
const META_MARKER: u8 = 0x06;
const META_END_OF_TRACK: u8 = 0x2F;
const META_TEMPO: u8 = 0x51;
const META_TIME_SIGNATURE: u8 = 0x58;
const META_KEY_SIGNATURE: u8 = 0x59;

/// Reads a Standard MIDI File of format 0, 1 or 2.
///
/// # Time
///
/// Every time in the file becomes a Windfall tick, 960 to the quarter note.
/// A file that counts `d` ticks to the quarter note has its tick `t` at
/// `t * 960 / d`, worked out exactly and rounded to the nearest tick, a
/// half rounding up. Each time is rounded by itself, from the start of the
/// file, so nothing drifts, and a note's length is the distance between its
/// rounded start and its rounded end. Resolutions that divide 960, such as
/// 96, 120, 192, 240, 480 and 960, lose nothing.
///
/// A file that counts in SMPTE frames keeps clock time and not beats. Its
/// ticks are turned into beats through the file's own tempo map: a stretch
/// of `n` ticks at a tempo of `m` microseconds per quarter note is
/// `n * 960,000,000 / (ticks per second * m)` Windfall ticks, exact and
/// rounded as above, and each tempo change begins on the rounded tick the
/// stretch before it ends on.
///
/// # Notes
///
/// A "note on" with velocity 0 is a "note off". Of several notes sounding
/// on one key of one channel, a "note off" ends the one that began first. A
/// "note off" for a key that is not sounding is ignored. A note still
/// sounding when its track ends is ended there. A note the file gives no
/// length, or one too short to survive the rounding, is one tick long.
///
/// # Formats
///
/// The single track of a format 0 file becomes one [`MidiTrack`] with all
/// its channels. In a format 1 file, a first track that holds no channel
/// message is the tempo track: its name becomes the song's name and it is
/// not listed among the tracks. Tempo, time signature, key signature and
/// markers are collected from every track. The tracks of a format 2 file
/// are separate patterns, each with its own tempo, and are laid out one
/// after another: each begins where the one before it ends. A pattern
/// that sets no tempo keeps the one the pattern before it ended on.
///
/// The result is in the normal form [`MidiSong::normalized`] describes.
pub fn read(bytes: &[u8]) -> Result<MidiSong, MidiError> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(MidiError::TooLarge {
            bytes: bytes.len() as u64,
            limit: MAX_FILE_BYTES as u64,
        });
    }
    if !bytes.starts_with(b"MThd") {
        return Err(MidiError::NotMidi);
    }
    let mut chunks = Chunks { data: bytes, at: 0 };
    let header = chunks.next()?.ok_or(MidiError::NotMidi)?;
    let Some(&[f0, f1, n0, n1, d0, d1]) = header.body.first_chunk::<6>() else {
        return Err(MidiError::BadHeader(
            "it is shorter than the six bytes a header has",
        ));
    };
    let format = u16::from_be_bytes([f0, f1]);
    if format > 2 {
        return Err(MidiError::UnsupportedFormat(format));
    }
    let division = Division::parse(u16::from_be_bytes([d0, d1]))?;

    // The count in the header only says when to stop looking. A file that
    // counts none is read to its end.
    let declared = usize::from(u16::from_be_bytes([n0, n1]));
    let limit = if declared == 0 { MAX_TRACKS } else { declared };
    let mut tracks = Vec::new();
    while tracks.len() < limit {
        let Some(chunk) = chunks.next()? else {
            break;
        };
        if &chunk.id == b"MTrk" {
            tracks.push(chunk);
        }
    }
    if tracks.is_empty() {
        return Err(MidiError::NoTracks);
    }

    let sequential = format == 2;
    let has_tempo_track = !sequential && (format == 1 || tracks.len() > 1);
    let shared = if sequential {
        None
    } else {
        Some(TimeMap::new(division, &tracks, DEFAULT_MICROS_PER_QUARTER)?)
    };

    let mut song = MidiSong::default();
    let mut offset = 0;
    for (index, chunk) in tracks.iter().enumerate() {
        let own;
        let map = match &shared {
            Some(map) => map,
            None => {
                // The tempo the pattern before this one ended on.
                let tempo = song.tempos.last();
                let tempo = tempo.map_or(DEFAULT_MICROS_PER_QUARTER, |t| t.micros_per_quarter);
                own = TimeMap::new(division, std::slice::from_ref(chunk), tempo)?;
                &own
            }
        };
        let read = read_track(chunk, index + 1, map, offset, &mut song)?;
        song.length = song.length.max(read.end);
        if sequential {
            offset = u64::from(read.end);
        }
        if has_tempo_track && index == 0 && !read.has_channel_messages {
            song.name = read.track.name;
        } else {
            song.tracks.push(read.track);
        }
    }
    Ok(song.normalized())
}

/// How the header says the file counts time.
#[derive(Debug, Clone, Copy)]
enum Division {
    /// Ticks to the quarter note.
    Metrical(u64),
    /// Ticks to the second, as a fraction: frames per second times ticks
    /// per frame.
    Smpte { numerator: u64, denominator: u64 },
}

impl Division {
    fn parse(raw: u16) -> Result<Self, MidiError> {
        if raw & 0x8000 == 0 {
            if raw == 0 {
                return Err(MidiError::BadHeader(
                    "it counts zero ticks to the quarter note",
                ));
            }
            return Ok(Division::Metrical(u64::from(raw)));
        }
        let [frames, ticks_per_frame] = raw.to_be_bytes();
        if ticks_per_frame == 0 {
            return Err(MidiError::BadHeader("it counts zero ticks to the frame"));
        }
        // The frame rate is stored as a negative number. 29 stands for the
        // 29.97 frames a second of drop-frame timecode.
        let (numerator, denominator) = match (frames as i8).unsigned_abs() {
            24 => (24, 1),
            25 => (25, 1),
            29 => (30_000, 1_001),
            30 => (30, 1),
            _ => {
                return Err(MidiError::BadHeader(
                    "its frame rate is none of 24, 25, 29.97 and 30 frames a second",
                ));
            }
        };
        Ok(Division::Smpte {
            numerator: numerator * u64::from(ticks_per_frame),
            denominator,
        })
    }
}

/// Turns a tick of the file into a Windfall tick.
#[derive(Debug)]
enum TimeMap {
    /// The file counts this many ticks to the quarter note.
    Metrical(u64),
    Smpte(Smpte),
}

/// The time of a file that counts clock time, at `numerator / denominator`
/// ticks a second. Each stretch has one tempo and begins on a whole tick.
#[derive(Debug)]
struct Smpte {
    numerator: u64,
    denominator: u64,
    stretches: Vec<Stretch>,
}

#[derive(Debug, Clone, Copy)]
struct Stretch {
    /// The tick of the file the stretch begins on.
    file: u64,
    /// The Windfall tick it begins on.
    tick: u64,
    micros_per_quarter: u32,
}

impl TimeMap {
    /// The map for tracks that share one tempo map, which begins at a tempo
    /// of `micros_per_quarter`. Only a file in SMPTE time needs its tempo
    /// to tell where a tick falls.
    fn new(
        division: Division,
        tracks: &[Chunk<'_>],
        micros_per_quarter: u32,
    ) -> Result<Self, MidiError> {
        let (numerator, denominator) = match division {
            Division::Metrical(ticks) => return Ok(TimeMap::Metrical(ticks)),
            Division::Smpte {
                numerator,
                denominator,
            } => (numerator, denominator),
        };
        let mut tempos = Vec::new();
        for (index, chunk) in tracks.iter().enumerate() {
            walk(chunk, index + 1, |tick, event| {
                if let Event::Meta { kind, data } = event
                    && let Some(micros) = tempo_of(kind, data)
                {
                    tempos.push((tick, micros));
                }
                Ok(())
            })?;
        }
        tempos.sort_by_key(|(tick, _)| *tick);

        let mut map = Smpte {
            numerator,
            denominator,
            stretches: vec![Stretch {
                file: 0,
                tick: 0,
                micros_per_quarter,
            }],
        };
        for (file, micros_per_quarter) in tempos {
            let tick = map.at(file);
            map.stretches.push(Stretch {
                file,
                tick,
                micros_per_quarter,
            });
        }
        Ok(TimeMap::Smpte(map))
    }

    /// The Windfall tick of a tick of the file. One too large for a `u64`
    /// is the largest `u64`.
    fn at(&self, file: u64) -> u64 {
        match self {
            TimeMap::Metrical(division) => rescale(file, TICKS_PER_QUARTER, *division),
            TimeMap::Smpte(map) => map.at(file),
        }
    }
}

impl Smpte {
    fn at(&self, file: u64) -> u64 {
        // Of several tempo changes on one tick the last counts.
        let after = self
            .stretches
            .partition_point(|stretch| stretch.file <= file);
        let Some(stretch) = after.checked_sub(1).and_then(|at| self.stretches.get(at)) else {
            return 0;
        };
        let ticks = rescale(
            file - stretch.file,
            self.denominator * TICKS_PER_QUARTER * 1_000_000,
            self.numerator * u64::from(stretch.micros_per_quarter),
        );
        stretch.tick.saturating_add(ticks)
    }
}

/// A chunk of the file: four letters, a length, and that many bytes.
#[derive(Debug, Clone, Copy)]
struct Chunk<'a> {
    id: [u8; 4],
    /// Where in the file the body begins.
    offset: usize,
    body: &'a [u8],
}

struct Chunks<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Chunks<'a> {
    /// The next chunk, or `None` at the end of the file.
    fn next(&mut self) -> Result<Option<Chunk<'a>>, MidiError> {
        let rest = self.data.get(self.at..).unwrap_or_default();
        if rest.is_empty() {
            return Ok(None);
        }
        let Some((&[a, b, c, d, l0, l1, l2, l3], rest)) = rest.split_first_chunk::<8>() else {
            return Err(MidiError::Truncated {
                what: "the chunk header",
                offset: self.at,
            });
        };
        let length = u32::from_be_bytes([l0, l1, l2, l3]) as usize;
        let Some(body) = rest.get(..length) else {
            return Err(MidiError::Truncated {
                what: "the chunk",
                offset: self.at,
            });
        };
        let offset = self.at + 8;
        self.at = offset + length;
        Ok(Some(Chunk {
            id: [a, b, c, d],
            offset,
            body,
        }))
    }
}

/// One event of a track, as far as the reader has a use for it.
#[derive(Debug, Clone, Copy)]
enum Event<'a> {
    NoteOn {
        channel: u8,
        key: u8,
        velocity: u8,
    },
    NoteOff {
        channel: u8,
        key: u8,
        velocity: u8,
    },
    Control {
        channel: u8,
        kind: ControlKind,
        value: u16,
    },
    Program {
        channel: u8,
        program: u8,
    },
    Meta {
        kind: u8,
        data: &'a [u8],
    },
}

/// Reads through the bytes of a track.
struct Cursor<'a> {
    data: &'a [u8],
    at: usize,
    /// Where in the file `data` begins, and which track it is, for errors.
    offset: usize,
    track: usize,
}

impl<'a> Cursor<'a> {
    fn corrupt(&self, problem: &'static str) -> MidiError {
        MidiError::Corrupt {
            track: self.track,
            offset: self.offset + self.at,
            problem,
        }
    }

    fn byte(&mut self) -> Result<u8, MidiError> {
        let byte = self.data.get(self.at).copied();
        let byte = byte.ok_or_else(|| self.corrupt("the track ends in the middle of an event"))?;
        self.at += 1;
        Ok(byte)
    }

    fn data_byte(&mut self) -> Result<u8, MidiError> {
        let byte = self.byte()?;
        if byte >= 0x80 {
            self.at -= 1;
            return Err(self.corrupt("a new event begins where the last one still needs a value"));
        }
        Ok(byte)
    }

    /// A variable-length number: seven bits to the byte, at most four
    /// bytes.
    fn number(&mut self) -> Result<u32, MidiError> {
        let mut value = 0;
        for _ in 0..4 {
            let byte = self.byte()?;
            value = value << 7 | u32::from(byte & 0x7F);
            if byte < 0x80 {
                return Ok(value);
            }
        }
        Err(self.corrupt("a length or a time takes more than four bytes"))
    }

    fn take(&mut self, length: u32) -> Result<&'a [u8], MidiError> {
        let end = self.at.checked_add(length as usize);
        let taken = end.and_then(|end| self.data.get(self.at..end));
        let taken = taken
            .ok_or_else(|| self.corrupt("an event is longer than what is left of the track"))?;
        self.at += taken.len();
        Ok(taken)
    }
}

/// Hands every event of a track to `visit` with its tick, counted in the
/// file's own ticks from the start of the track, and returns the tick the
/// track ends on. System exclusive messages are passed over.
fn walk<'a>(
    chunk: &Chunk<'a>,
    track: usize,
    mut visit: impl FnMut(u64, Event<'a>) -> Result<(), MidiError>,
) -> Result<u64, MidiError> {
    let mut cursor = Cursor {
        data: chunk.body,
        at: 0,
        offset: chunk.offset,
        track,
    };
    let mut tick: u64 = 0;
    let mut running: Option<u8> = None;
    while cursor.at < cursor.data.len() {
        tick += u64::from(cursor.number()?);
        let first = cursor.byte()?;
        let status = match first {
            0xFF => {
                let kind = cursor.byte()?;
                let length = cursor.number()?;
                let data = cursor.take(length)?;
                if kind == META_END_OF_TRACK {
                    return Ok(tick);
                }
                visit(tick, Event::Meta { kind, data })?;
                continue;
            }
            0xF0 | 0xF7 => {
                let length = cursor.number()?;
                cursor.take(length)?;
                continue;
            }
            0xF1..=0xFE => {
                cursor.at -= 1;
                return Err(cursor.corrupt("a system message that has no place in a file"));
            }
            0x80..=0xEF => {
                running = Some(first);
                first
            }
            _ => {
                // A data byte: the event reuses the status of the last one.
                cursor.at -= 1;
                running.ok_or_else(|| cursor.corrupt("an event with no status byte before it"))?
            }
        };
        let channel = status & 0x0F;
        let event = match status >> 4 {
            0x8 => Event::NoteOff {
                channel,
                key: cursor.data_byte()?,
                velocity: cursor.data_byte()?,
            },
            0x9 => Event::NoteOn {
                channel,
                key: cursor.data_byte()?,
                velocity: cursor.data_byte()?,
            },
            0xA => Event::Control {
                channel,
                kind: ControlKind::KeyPressure(cursor.data_byte()?),
                value: u16::from(cursor.data_byte()?),
            },
            0xB => Event::Control {
                channel,
                kind: ControlKind::Controller(cursor.data_byte()?),
                value: u16::from(cursor.data_byte()?),
            },
            0xC => Event::Program {
                channel,
                program: cursor.data_byte()?,
            },
            0xD => Event::Control {
                channel,
                kind: ControlKind::ChannelPressure,
                value: u16::from(cursor.data_byte()?),
            },
            _ => {
                let low = u16::from(cursor.data_byte()?);
                let high = u16::from(cursor.data_byte()?);
                Event::Control {
                    channel,
                    kind: ControlKind::PitchBend,
                    value: high << 7 | low,
                }
            }
        };
        visit(tick, event)?;
    }
    Ok(tick)
}

/// The tempo a meta event sets, if it is a tempo event that sets one.
fn tempo_of(kind: u8, data: &[u8]) -> Option<u32> {
    let &[a, b, c] = data.first_chunk::<3>()?;
    let micros = u32::from_be_bytes([0, a, b, c]);
    (kind == META_TEMPO && micros > 0).then_some(micros)
}

struct TrackRead {
    track: MidiTrack,
    /// The Windfall tick the track ends on.
    end: u32,
    /// Whether the track holds anything that plays on a channel.
    has_channel_messages: bool,
}

/// Reads one track, whose first tick is the Windfall tick `offset`. The
/// tempo, signatures and markers it holds go to `song`.
fn read_track(
    chunk: &Chunk<'_>,
    index: usize,
    map: &TimeMap,
    offset: u64,
    song: &mut MidiSong,
) -> Result<TrackRead, MidiError> {
    let place = |file: u64| {
        let tick = offset.saturating_add(map.at(file));
        u32::try_from(tick).map_err(|_| MidiError::TooLong)
    };
    let mut track = MidiTrack::default();
    let mut sounding: HashMap<(u8, u8), VecDeque<(u32, u8)>> = HashMap::new();
    let mut curves: BTreeMap<(u8, ControlKind), Vec<ControlPoint>> = BTreeMap::new();
    let mut has_channel_messages = false;

    let end = walk(chunk, index, |file, event| {
        let tick = place(file)?;
        has_channel_messages |= !matches!(event, Event::Meta { .. });
        match event {
            Event::NoteOn {
                channel,
                key,
                velocity,
            } if velocity > 0 => {
                let held = sounding.entry((channel, key)).or_default();
                held.push_back((tick, velocity));
            }
            Event::NoteOn {
                channel,
                key,
                velocity: release,
            }
            | Event::NoteOff {
                channel,
                key,
                velocity: release,
            } => {
                let held = sounding.get_mut(&(channel, key));
                if let Some((start, velocity)) = held.and_then(VecDeque::pop_front) {
                    let is_note_off = matches!(event, Event::NoteOff { .. });
                    track.notes.push(MidiNote {
                        start,
                        length: (tick - start).max(1),
                        key,
                        velocity,
                        release: if is_note_off {
                            release
                        } else {
                            DEFAULT_RELEASE
                        },
                        channel,
                    });
                }
            }
            Event::Control {
                channel,
                kind,
                value,
            } => {
                let points = curves.entry((channel, kind)).or_default();
                points.push(ControlPoint { tick, value });
            }
            Event::Program { channel, program } => track.programs.push(ProgramChange {
                tick,
                channel,
                program,
            }),
            Event::Meta { kind, data } => read_meta(kind, data, tick, &mut track, song),
        }
        Ok(())
    })?;
    let end = place(end)?;

    for ((channel, key), held) in sounding {
        for (start, velocity) in held {
            track.notes.push(MidiNote {
                start,
                length: (end - start).max(1),
                key,
                velocity,
                release: DEFAULT_RELEASE,
                channel,
            });
        }
    }
    track.controls = curves
        .into_iter()
        .map(|((channel, kind), points)| ControlCurve {
            channel,
            kind,
            points,
        })
        .collect();
    Ok(TrackRead {
        track,
        end,
        has_channel_messages,
    })
}

/// Takes what a meta event says. One that is too short for its kind, or
/// says something no file can mean, is passed over.
fn read_meta(kind: u8, data: &[u8], tick: u32, track: &mut MidiTrack, song: &mut MidiSong) {
    match kind {
        META_TRACK_NAME if track.name.is_none() => {
            track.name = Some(decode_text(data)).filter(|name| !name.is_empty());
        }
        META_INSTRUMENT_NAME if track.instrument.is_none() => {
            track.instrument = Some(decode_text(data)).filter(|name| !name.is_empty());
        }
        META_MARKER => song.markers.push(Marker {
            tick,
            text: decode_text(data),
        }),
        META_TEMPO => {
            if let Some(micros_per_quarter) = tempo_of(kind, data) {
                song.tempos.push(TempoChange {
                    tick,
                    micros_per_quarter,
                });
            }
        }
        META_TIME_SIGNATURE => {
            // The beat unit is stored as a power of two.
            if let Some(&[numerator, power]) = data.first_chunk::<2>()
                && numerator > 0
                && power <= 7
            {
                song.time_signatures.push(TimeSignatureChange {
                    tick,
                    numerator,
                    denominator: 1 << power,
                });
            }
        }
        META_KEY_SIGNATURE => {
            if let Some(&[sharps, minor]) = data.first_chunk::<2>() {
                song.key_signatures.push(KeySignatureChange {
                    tick,
                    sharps: (sharps as i8).clamp(-7, 7),
                    minor: minor != 0,
                });
            }
        }
        _ => {}
    }
}
