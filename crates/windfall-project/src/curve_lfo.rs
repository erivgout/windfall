//! Bounded, seeded musical LFOs shared by note event tools and automation.

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use crate::{AutomationPoint, CommandError, MAX_AUTOMATION_POINTS, MAX_SONG_TICKS};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CurveLfoWave { Sine, Triangle, SawUp, SawDown, Square, SampleHold }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(export)]
pub struct CurveLfo {
    pub wave: CurveLfoWave,
    /// Musical ticks per cycle.
    pub period: u32,
    /// Cycle fraction, zero through one.
    pub phase: f64,
    /// Center and bipolar amplitude in normalized target units.
    pub center: f64,
    pub depth: f64,
    /// Fraction of each square-wave cycle spent high.
    pub width: f64,
    pub seed: u32,
}

impl CurveLfo {
    pub fn check(self) -> Result<(), CommandError> {
        if self.period == 0 || self.period > MAX_SONG_TICKS ||
            [self.phase, self.center, self.depth, self.width].into_iter().any(|value| !value.is_finite() || !(0.0..=1.0).contains(&value)) {
            return Err(CommandError::invalid("invalid LFO period, phase, center, depth or pulse width"));
        }
        Ok(())
    }
    pub fn value_at(self, tick: f64) -> f32 { self.at_phase(self.phase + tick / f64::from(self.period)) }
    fn at_phase(self, phase: f64) -> f32 {
        let cycle = phase.floor();
        let part = phase.rem_euclid(1.0);
        let wave = match self.wave {
            CurveLfoWave::Sine => (part * std::f64::consts::TAU).sin(),
            CurveLfoWave::Triangle => 1.0 - (part * 4.0 - 2.0).abs(),
            CurveLfoWave::SawUp => part * 2.0 - 1.0,
            CurveLfoWave::SawDown => 1.0 - part * 2.0,
            CurveLfoWave::Square => if part < self.width { 1.0 } else { -1.0 },
            CurveLfoWave::SampleHold => {
                let cycle = cycle as i64 as u64;
                let mut value = self.seed ^ cycle as u32 ^ (cycle >> 32) as u32 ^ 0x9e37_79b9;
                value = (value ^ (value >> 16)).wrapping_mul(0x7feb_352d);
                value = (value ^ (value >> 15)).wrapping_mul(0x846c_a68b);
                value ^= value >> 16;
                f64::from(value) / 4_294_967_296.0 * 2.0 - 1.0
            }
        };
        (self.center + self.depth * wave).clamp(0.0, 1.0) as f32
    }
    fn hold(self) -> bool { matches!(self.wave, CurveLfoWave::Square | CurveLfoWave::SampleHold) }
}

/// Points include both ends. Discontinuities use two ordered points on the
/// first whole tick at/after the boundary, retaining the left and right values.
pub fn lfo_points(start: u32, end: u32, resolution: u32, lfo: CurveLfo) -> Result<Vec<AutomationPoint>, CommandError> {
    lfo.check()?;
    if start >= end || end > MAX_SONG_TICKS || resolution == 0 || resolution > MAX_SONG_TICKS {
        return Err(CommandError::invalid("use an increasing LFO range and a positive tick resolution"));
    }
    let span = end - start;
    let cells = span.div_ceil(resolution) as usize;
    let cycles = f64::from(span) / f64::from(lfo.period);
    if cells + 1 > MAX_AUTOMATION_POINTS || cycles.ceil() > MAX_AUTOMATION_POINTS as f64 {
        return Err(CommandError::invalid("LFO generation would exceed 4096 points; increase period or resolution"));
    }
    let mut values: BTreeMap<u32, (f32, Option<f32>)> = BTreeMap::new();
    for cell in 0..cells {
        let offset = (cell as u64 * u64::from(resolution)).min(u64::from(span)) as u32;
        values.insert(start + offset, (lfo.value_at(f64::from(offset)), None));
    }
    values.insert(end, (lfo.value_at(f64::from(span)), None));
    if matches!(lfo.wave, CurveLfoWave::SawUp | CurveLfoWave::SawDown | CurveLfoWave::Square | CurveLfoWave::SampleHold) {
        let mut boundaries = Vec::new();
        let first = lfo.phase.floor() as i64;
        let last = (lfo.phase + cycles).ceil() as i64;
        for cycle in first..=last {
            boundaries.push(cycle as f64);
            if lfo.wave == CurveLfoWave::Square && lfo.width > 0.0 && lfo.width < 1.0 {
                boundaries.push(cycle as f64 + lfo.width);
            }
        }
        boundaries.sort_by(f64::total_cmp);
        for phase in boundaries {
            let offset = (phase - lfo.phase) * f64::from(lfo.period);
            if offset <= 0.0 || offset >= f64::from(span) { continue; }
            let tick = start + offset.ceil() as u32;
            let before = lfo.at_phase(phase.next_down());
            let after = lfo.value_at(f64::from(tick - start));
            match values.get_mut(&tick) {
                Some((value, right @ None)) => { *value = before; *right = Some(after); }
                Some((_, Some(right))) => *right = after,
                None => { values.insert(tick, (before, Some(after))); }
            }
        }
    }
    let count = values.values().map(|(_, right)| 1 + usize::from(right.is_some())).sum::<usize>();
    if count > MAX_AUTOMATION_POINTS { return Err(CommandError::invalid("LFO generation would exceed 4096 points")); }
    let mut points = Vec::with_capacity(count);
    for (tick, (left, right)) in values {
        points.push(AutomationPoint { tick, value: left, curve: 0.0, hold: lfo.hold() });
        if let Some(value) = right { points.push(AutomationPoint { tick, value, curve: 0.0, hold: lfo.hold() }); }
    }
    Ok(points)
}

/// Splice a generated range while preserving the existing curve on either
/// side. Splitting an exponential segment scales its bend by the retained
/// fraction; duplicate endpoint values make jumps local to the chosen range.
pub fn write_lfo(points: &[AutomationPoint], start: u32, end: u32, resolution: u32, lfo: CurveLfo) -> Result<Vec<AutomationPoint>, CommandError> {
    let generated = lfo_points(start, end, resolution, lfo)?;
    let mut result: Vec<_> = points.iter().copied().filter(|point| point.tick < start).collect();
    let start_value = points.iter().find(|point| point.tick == start).map_or_else(
        || crate::curve_value(points, f64::from(start)), |point| point.value);
    if let Some(left) = result.last_mut() {
        if let Some(right) = points.iter().find(|point| point.tick > left.tick) {
            if right.tick > start && !left.hold {
                left.curve *= (start - left.tick) as f32 / (right.tick - left.tick) as f32;
            }
        }
    }
    result.push(AutomationPoint { tick: start, value: start_value, curve: 0.0, hold: false });
    result.extend(generated);
    let mut restore = points.iter().rev().find(|point| point.tick <= end).copied()
        .unwrap_or(AutomationPoint { tick: end, value: crate::curve_value(points, f64::from(end)), curve: 0.0, hold: false });
    if let Some(right) = points.iter().find(|point| point.tick > end) {
        if restore.tick < end && !restore.hold {
            restore.curve *= (right.tick - end) as f32 / (right.tick - restore.tick) as f32;
        }
    }
    restore.tick = end;
    restore.value = crate::curve_value(points, f64::from(end));
    result.push(restore);
    result.extend(points.iter().copied().filter(|point| point.tick > end));
    if result.len() > MAX_AUTOMATION_POINTS { return Err(CommandError::invalid("the combined curve would exceed 4096 points")); }
    Ok(result)
}
