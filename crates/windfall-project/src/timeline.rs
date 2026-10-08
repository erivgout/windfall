//! Checked musical meter and navigation data, independent of the tempo clock.

use crate::check::time_signature_problem;
use crate::{MAX_SONG_TICKS, PPQ, TimeSignature};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use ts_rs::TS;

pub const MAX_TIMELINE_ITEMS: usize = 2_048;
pub const MAX_MARKER_NAME_BYTES: usize = 256;

/// Half-open absolute song ticks, checked before transport/export mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TickRange {
    pub start: u32,
    pub end: u32,
}
impl TickRange {
    pub fn check(self) -> Result<(), String> {
        if self.start >= self.end || self.end > MAX_SONG_TICKS {
            return Err(format!(
                "a timeline range needs 0 <= start < end <= {MAX_SONG_TICKS} ticks"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MeterChange {
    /// Monotonic document ID, shared with every other entity.
    pub id: u32,
    pub tick: u32,
    pub signature: TimeSignature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum MarkerKind {
    Named,
    /// Jump from end to the marker tick while song looping is enabled.
    Loop {
        end: u32,
    },
    /// Jump from the marker tick to end.
    Skip {
        end: u32,
    },
    /// Stop here. Resume passes this action once.
    Pause,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TimelineMarker {
    pub id: u32,
    pub tick: u32,
    pub name: String,
    pub kind: MarkerKind,
}
impl TimelineMarker {
    pub fn check(&self) -> Result<(), String> {
        if self.tick > MAX_SONG_TICKS {
            return Err("a marker is past the longest song".to_owned());
        }
        if self.name.trim().is_empty() || self.name.len() > MAX_MARKER_NAME_BYTES {
            return Err(format!(
                "a marker needs a name of 1 to {MAX_MARKER_NAME_BYTES} bytes"
            ));
        }
        if let MarkerKind::Loop { end } | MarkerKind::Skip { end } = self.kind {
            TickRange {
                start: self.tick,
                end,
            }
            .check()?;
        }
        Ok(())
    }
    fn extent(&self) -> Option<(u32, u32)> {
        match self.kind {
            MarkerKind::Named => None,
            MarkerKind::Pause => Some((self.tick, self.tick)),
            MarkerKind::Loop { end } | MarkerKind::Skip { end } => Some((self.tick, end)),
        }
    }
}

/// Optional persisted song musical data. Selection/regions are session data.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct Timeline {
    pub meters: Vec<MeterChange>,
    /// Sorted by tick then ID. Named labels may share a position.
    pub markers: Vec<TimelineMarker>,
}
impl Timeline {
    pub fn is_empty(&self) -> bool {
        self.meters.is_empty() && self.markers.is_empty()
    }
    pub fn check(&self, legacy: TimeSignature, next_id: u32) -> Result<(), String> {
        if self.meters.len() > MAX_TIMELINE_ITEMS || self.markers.len() > MAX_TIMELINE_ITEMS {
            return Err(format!(
                "a timeline holds at most {MAX_TIMELINE_ITEMS} meters and markers each"
            ));
        }
        MeterMap::new(legacy, &self.meters)?;
        let mut ids = HashSet::new();
        for id in self
            .meters
            .iter()
            .map(|m| m.id)
            .chain(self.markers.iter().map(|m| m.id))
        {
            if id == 0 || id >= next_id || !ids.insert(id) {
                return Err("timeline IDs must be unique, positive and below nextId".to_owned());
            }
        }
        let mut last = None;
        let mut active_end = None;
        for marker in &self.markers {
            marker.check()?;
            let key = (marker.tick, marker.id);
            if last.is_some_and(|last| last >= key) {
                return Err("markers must be ordered by tick then ID".to_owned());
            }
            last = Some(key);
            if let Some((start, end)) = marker.extent() {
                if active_end.is_some_and(|previous| start <= previous) {
                    return Err(
                        "executable marker ranges and boundaries must not collide".to_owned()
                    );
                }
                active_end = Some(end);
            }
        }
        Ok(())
    }
}

/// One-based bar/beat and a zero-based tick inside the beat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MusicalPosition {
    pub bar: u32,
    pub beat: u32,
    pub tick: u32,
}
#[derive(Debug, Clone, Copy)]
struct MeterSegment {
    start: u32,
    end: u32,
    bar: u32,
    signature: TimeSignature,
}

/// Control-side conversion; unaligned changes start a new bar and shorten
/// the preceding bar. Never converts ticks into seconds.
pub struct MeterMap {
    segments: Vec<MeterSegment>,
}
impl MeterMap {
    pub fn new(legacy: TimeSignature, changes: &[MeterChange]) -> Result<Self, String> {
        if let Some(problem) = time_signature_problem(legacy) {
            return Err(problem);
        }
        if changes.len() > MAX_TIMELINE_ITEMS {
            return Err("too many meter changes".to_owned());
        }
        let mut segments = vec![MeterSegment {
            start: 0,
            end: MAX_SONG_TICKS,
            bar: 1,
            signature: legacy,
        }];
        let mut previous = None;
        for change in changes {
            if change.tick >= MAX_SONG_TICKS || previous.is_some_and(|tick| change.tick <= tick) {
                return Err(
                    "meter changes must have strictly increasing ticks before the longest song"
                        .to_owned(),
                );
            }
            if let Some(problem) = time_signature_problem(change.signature) {
                return Err(problem);
            }
            previous = Some(change.tick);
            let prior = segments.last_mut().expect("the initial meter exists");
            if change.tick == 0 {
                prior.signature = change.signature;
                continue;
            }
            let bar = prior
                .bar
                .checked_add((change.tick - prior.start).div_ceil(prior.signature.ticks_per_bar()))
                .ok_or_else(|| "musical bar overflow".to_owned())?;
            prior.end = change.tick;
            segments.push(MeterSegment {
                start: change.tick,
                end: MAX_SONG_TICKS,
                bar,
                signature: change.signature,
            });
        }
        Ok(Self { segments })
    }
    pub fn tick_to_position(&self, tick: u32) -> Result<MusicalPosition, String> {
        if tick > MAX_SONG_TICKS {
            return Err("tick is past the longest song".to_owned());
        }
        let segment = &self.segments[self.segments.partition_point(|s| s.start <= tick) - 1];
        let relative = tick - segment.start;
        let beat_ticks = PPQ * 4 / u32::from(segment.signature.denominator);
        let bar_ticks = segment.signature.ticks_per_bar();
        Ok(MusicalPosition {
            bar: segment.bar + relative / bar_ticks,
            beat: (relative % bar_ticks) / beat_ticks + 1,
            tick: relative % beat_ticks,
        })
    }
    pub fn position_to_tick(&self, position: MusicalPosition) -> Result<u32, String> {
        if position.bar == 0 || position.beat == 0 {
            return Err("bars and beats count from one".to_owned());
        }
        let index = self.segments.partition_point(|s| s.bar <= position.bar);
        let segment = &self.segments[index - 1];
        let beat_ticks = PPQ * 4 / u32::from(segment.signature.denominator);
        if position.beat > u32::from(segment.signature.numerator) || position.tick >= beat_ticks {
            return Err("beat or tick is outside this meter".to_owned());
        }
        let tick = u64::from(segment.start)
            + u64::from(position.bar - segment.bar) * u64::from(segment.signature.ticks_per_bar())
            + u64::from(position.beat - 1) * u64::from(beat_ticks)
            + u64::from(position.tick);
        if tick > u64::from(MAX_SONG_TICKS)
            || (index < self.segments.len() && tick >= u64::from(segment.end))
        {
            return Err("position is outside the song or in a shortened bar".to_owned());
        }
        Ok(tick as u32)
    }
}
