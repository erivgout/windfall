//! Deterministic selected-note tools, shared by desktop and the WASM document.
//! These run on the document thread; they never participate in audio callbacks.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{CommandError, MAX_KEY, MAX_PATTERN_TICKS, Note, NoteId};

/// A bound on both the input selection and the output of a tool. In particular,
/// a one-tick chop cannot accidentally create hundreds of thousands of notes.
pub const MAX_TOOL_NOTES: usize = 16_384;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum NoteEdge {
    Start,
    End,
}

/// Original, repeating timing patterns. Offsets are fractions of the grid:
/// Swing delays odd divisions by 1/6, LatePairs by 1/4, and PushFour advances
/// every fourth division by 1/6. Straight has no offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum NoteGroove {
    Straight,
    Swing,
    LatePairs,
    PushFour,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[ts(export)]
pub enum NoteTransform {
    /// Extend/trim to the next distinct selected onset; keep the last chord.
    Legato,
    /// Multiply lengths by a finite factor in (0, 1]. Round, minimum one tick.
    Staccato {
        factor: f64,
    },
    /// Split at absolute grid boundaries. Keep the first segment's id.
    Chop {
        grid: u32,
    },
    /// Union touching/overlapping notes with identical key, velocity and pan.
    Glue,
    /// Each exact-onset chord, ordered by pitch then id (or the reverse pitch
    /// order), gets successive delays and velocity offsets. Lengths stay put.
    Strum {
        spacing: u32,
        velocity_step: f64,
        descending: bool,
    },
    FlipTime,
    FlipPitch,
    /// Transpose, then clamp to the inclusive MIDI range. With octaves, choose
    /// the nearest octave-equivalent key in range, falling back to clamping
    /// when the chosen range has no representative of the pitch class.
    KeyRange {
        low: u8,
        high: u8,
        transpose: i32,
        octaves: bool,
    },
    ScaleVelocity {
        factor: f64,
    },
    Quantize {
        grid: u32,
        strength: f64,
        edge: NoteEdge,
        groove: NoteGroove,
    },
}

impl NoteTransform {
    pub fn label(self) -> &'static str {
        match self {
            Self::Legato => "Legato notes",
            Self::Staccato { .. } => "Staccato notes",
            Self::Chop { .. } => "Chop notes",
            Self::Glue => "Glue notes",
            Self::Strum { .. } => "Strum notes",
            Self::FlipTime => "Flip note time",
            Self::FlipPitch => "Flip note pitch",
            Self::KeyRange { .. } => "Limit and transpose notes",
            Self::ScaleVelocity { .. } => "Scale note velocities",
            Self::Quantize {
                edge: NoteEdge::Start,
                ..
            } => "Quantize note starts",
            Self::Quantize {
                edge: NoteEdge::End,
                ..
            } => "Quantize note ends",
        }
    }

    fn check(self) -> Result<(), CommandError> {
        match self {
            Self::Staccato { factor } => number("length factor", factor, f64::MIN_POSITIVE, 1.0),
            Self::ScaleVelocity { factor } => number("velocity factor", factor, 0.0, 4.0),
            Self::Chop { grid: value } => grid(value),
            Self::Quantize {
                grid: value,
                strength,
                ..
            } => {
                grid(value)?;
                number("quantize strength", strength, 0.0, 1.0)
            }
            Self::Strum {
                spacing,
                velocity_step,
                ..
            } => {
                if spacing > MAX_PATTERN_TICKS {
                    return Err(CommandError::invalid("strum spacing is too large"));
                }
                number("strum velocity step", velocity_step, -1.0, 1.0)
            }
            Self::KeyRange {
                low,
                high,
                transpose,
                ..
            } => {
                if low > high || high > MAX_KEY || !(-127..=127).contains(&transpose) {
                    return Err(CommandError::invalid("invalid MIDI key range or transpose"));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

fn number(name: &str, value: f64, low: f64, high: f64) -> Result<(), CommandError> {
    if !value.is_finite() || !(low..=high).contains(&value) {
        return Err(CommandError::invalid(format!("invalid {name}")));
    }
    Ok(())
}

fn grid(value: u32) -> Result<(), CommandError> {
    if value == 0 || value > MAX_PATTERN_TICKS {
        return Err(CommandError::invalid("grid must be 1 to 245760 ticks"));
    }
    Ok(())
}

fn valid_note(note: &Note) -> Result<(), CommandError> {
    if note.id.0 == 0
        || note.key > MAX_KEY
        || note.length == 0
        || u64::from(note.start) + u64::from(note.length) > u64::from(MAX_PATTERN_TICKS)
        || !note.velocity.is_finite()
        || !(0.0..=1.0).contains(&note.velocity)
        || !note.pan.is_finite()
        || !(-1.0..=1.0).contains(&note.pan)
    {
        return Err(CommandError::invalid(
            "selected note is invalid or outside the pattern limit",
        ));
    }
    Ok(())
}

/// Normalize a captured selection. Identical duplicate ids are harmless;
/// conflicting snapshots of one id are rejected. Result is in lane order.
pub(crate) fn selection(notes: &[Note]) -> Result<Vec<Note>, CommandError> {
    if notes.is_empty() || notes.len() > MAX_TOOL_NOTES {
        return Err(CommandError::invalid(
            "select 1 to 16384 notes for this tool",
        ));
    }
    let mut unique = BTreeMap::new();
    for note in notes {
        valid_note(note)?;
        if unique
            .insert(note.id, *note)
            .is_some_and(|old| old != *note)
        {
            return Err(CommandError::invalid("conflicting selected note snapshots"));
        }
    }
    let mut notes: Vec<_> = unique.into_values().collect();
    notes.sort_by_key(Note::sort_key);
    Ok(notes)
}

/// Pure tool calculation. New chop segments use id 0 until the transaction
/// allocates ids; all other surviving notes retain their original ids.
pub fn transform_selected_notes(
    notes: &[Note],
    tool: NoteTransform,
) -> Result<Vec<Note>, CommandError> {
    tool.check()?;
    let mut notes = selection(notes)?;
    match tool {
        NoteTransform::Legato => {
            let mut starts: Vec<_> = notes.iter().map(|n| n.start).collect();
            starts.dedup();
            for note in &mut notes {
                if let Some(next) = starts.get(starts.partition_point(|s| *s <= note.start)) {
                    note.length = next - note.start;
                }
            }
        }
        NoteTransform::Staccato { factor } => {
            for note in &mut notes {
                note.length = (f64::from(note.length) * factor).round().max(1.0) as u32;
            }
        }
        NoteTransform::Chop { grid } => {
            let count: usize = notes
                .iter()
                .map(|n| ((n.start + n.length - 1) / grid - n.start / grid + 1) as usize)
                .sum();
            if count > MAX_TOOL_NOTES {
                return Err(CommandError::invalid(
                    "chop would exceed 16384 notes; choose a coarser grid",
                ));
            }
            let mut pieces = Vec::with_capacity(count);
            for note in notes {
                let end = note.start + note.length;
                let mut start = note.start;
                while start < end {
                    let next = ((start / grid + 1) * grid).min(end);
                    pieces.push(Note {
                        id: if start == note.start {
                            note.id
                        } else {
                            NoteId(0)
                        },
                        start,
                        length: next - start,
                        ..note
                    });
                    start = next;
                }
            }
            notes = pieces;
        }
        NoteTransform::Glue => {
            notes.sort_by(|a, b| {
                a.key
                    .cmp(&b.key)
                    // Inputs are finite. Numeric ordering keeps -0 and +0
                    // together, matching the compatibility comparison below.
                    .then(a.velocity.partial_cmp(&b.velocity).unwrap())
                    .then(a.pan.partial_cmp(&b.pan).unwrap())
                    .then(a.start.cmp(&b.start))
                    .then(a.id.cmp(&b.id))
            });
            let mut glued: Vec<Note> = Vec::with_capacity(notes.len());
            for note in notes {
                if let Some(last) = glued.last_mut()
                    && last.key == note.key
                    && last.velocity == note.velocity
                    && last.pan == note.pan
                    && note.start <= last.start + last.length
                {
                    last.length =
                        (last.start + last.length).max(note.start + note.length) - last.start;
                } else {
                    glued.push(note);
                }
            }
            notes = glued;
        }
        NoteTransform::Strum {
            spacing,
            velocity_step,
            descending,
        } => {
            let mut first = 0;
            while first < notes.len() {
                let onset = notes[first].start;
                let end = first + notes[first..].partition_point(|n| n.start == onset);
                if descending {
                    notes[first..end].sort_by(|a, b| b.key.cmp(&a.key).then(a.id.cmp(&b.id)));
                }
                for (rank, note) in notes[first..end].iter_mut().enumerate() {
                    let start = u64::from(onset) + rank as u64 * u64::from(spacing);
                    if start + u64::from(note.length) > u64::from(MAX_PATTERN_TICKS) {
                        return Err(CommandError::invalid(
                            "strum would end past the longest pattern",
                        ));
                    }
                    note.start = start as u32;
                    note.velocity = (f64::from(note.velocity) + rank as f64 * velocity_step)
                        .clamp(0.0, 1.0) as f32;
                }
                first = end;
            }
        }
        NoteTransform::FlipTime => {
            let left = notes.iter().map(|n| n.start).min().unwrap();
            let right = notes.iter().map(|n| n.start + n.length).max().unwrap();
            for note in &mut notes {
                note.start = left + right - note.start - note.length;
            }
        }
        NoteTransform::FlipPitch => {
            let low = notes.iter().map(|n| n.key).min().unwrap();
            let high = notes.iter().map(|n| n.key).max().unwrap();
            for note in &mut notes {
                note.key = (u16::from(low) + u16::from(high) - u16::from(note.key)) as u8;
            }
        }
        NoteTransform::KeyRange {
            low,
            high,
            transpose,
            octaves,
        } => {
            for note in &mut notes {
                let key = i32::from(note.key) + transpose;
                let target = if octaves {
                    (i32::from(low)..=i32::from(high))
                        .filter(|k| (k - key).rem_euclid(12) == 0)
                        .min_by_key(|k| ((k - key).abs(), *k))
                } else {
                    None
                };
                note.key =
                    target.unwrap_or_else(|| key.clamp(i32::from(low), i32::from(high))) as u8;
            }
        }
        NoteTransform::ScaleVelocity { factor } => {
            for note in &mut notes {
                note.velocity = (f64::from(note.velocity) * factor).clamp(0.0, 1.0) as f32;
            }
        }
        NoteTransform::Quantize {
            grid,
            strength,
            edge,
            groove,
        } => {
            for note in &mut notes {
                let old = match edge {
                    NoteEdge::Start => note.start,
                    NoteEdge::End => note.start + note.length,
                };
                let target = nearest(old, grid, groove);
                let tick = (f64::from(old) + (f64::from(target) - f64::from(old)) * strength)
                    .round() as u32;
                match edge {
                    NoteEdge::Start => note.start = tick.min(MAX_PATTERN_TICKS - note.length),
                    NoteEdge::End => {
                        note.length = tick.clamp(note.start + 1, MAX_PATTERN_TICKS) - note.start
                    }
                }
            }
        }
    }
    notes.sort_by_key(Note::sort_key);
    Ok(notes)
}

fn nearest(tick: u32, grid: u32, groove: NoteGroove) -> u32 {
    let division = i64::from(tick / grid);
    ((division - 2).max(0)..=division + 2)
        .map(|i| {
            let offset = match groove {
                NoteGroove::Swing if i % 2 == 1 => i64::from(grid) / 6,
                NoteGroove::LatePairs if i % 2 == 1 => i64::from(grid) / 4,
                NoteGroove::PushFour if i % 4 == 3 => -i64::from(grid) / 6,
                _ => 0,
            };
            (i * i64::from(grid) + offset).clamp(0, i64::from(MAX_PATTERN_TICKS)) as u32
        })
        .min_by_key(|target| (target.abs_diff(tick), std::cmp::Reverse(*target)))
        .unwrap()
}
