//! Constant builders for the rows of a parameter table.
//!
//! The other processors write their tables with the
//! [`param_set!`](crate::param::param_set) macro, which needs one line of
//! source per control. An additive instrument has one control per partial,
//! so its table is built by a `while` loop in a constant instead, and these
//! are the pieces it assembles.

use crate::blocks::math::clean;
use crate::param::{ParamInfo, ParamKind, ParamScale, ParamSet, ParamUnit, approach_value};

/// What an unfilled row of a table under construction holds. Every table
/// overwrites all of its rows, and the tests check that none is left blank.
pub(super) const BLANK: ParamInfo = ParamInfo {
    id: "",
    name: "",
    kind: ParamKind::Float,
    unit: ParamUnit::None,
    scale: ParamScale::Linear,
    min: 0.0,
    max: 1.0,
    default: 0.0,
    choices: &[],
};

/// A row for a continuous control.
pub(super) const fn float_row(
    id: &'static str,
    name: &'static str,
    unit: ParamUnit,
    scale: ParamScale,
    min: f32,
    max: f32,
    default: f32,
) -> ParamInfo {
    ParamInfo {
        id,
        name,
        kind: ParamKind::Float,
        unit,
        scale,
        min,
        max,
        default,
        choices: &[],
    }
}

/// A row for a continuous control on a plain 0-to-1 scale.
pub(super) const fn fraction_row(
    id: &'static str,
    name: &'static str,
    min: f32,
    max: f32,
    default: f32,
) -> ParamInfo {
    float_row(
        id,
        name,
        ParamUnit::Fraction,
        ParamScale::Linear,
        min,
        max,
        default,
    )
}

/// A row for a whole-number control.
pub(super) const fn int_row(
    id: &'static str,
    name: &'static str,
    min: i32,
    max: i32,
    default: i32,
) -> ParamInfo {
    ParamInfo {
        id,
        name,
        kind: ParamKind::Integer,
        unit: ParamUnit::None,
        scale: ParamScale::Linear,
        min: min as f32,
        max: max as f32,
        default: default as f32,
        choices: &[],
    }
}

/// A row for an on-or-off control.
pub(super) const fn toggle_row(id: &'static str, name: &'static str, default: bool) -> ParamInfo {
    ParamInfo {
        id,
        name,
        kind: ParamKind::Toggle,
        unit: ParamUnit::None,
        scale: ParamScale::Linear,
        min: 0.0,
        max: 1.0,
        default: if default { 1.0 } else { 0.0 },
        choices: &[],
    }
}

/// The level a partial has by default: the spectrum of a saw, where the
/// `index`th partial above the fundamental is `1 / (index + 1)` as loud.
///
/// Defaulting to a shape that falls away keeps a stack of 64 partials from
/// adding up to a level no one asked for.
pub(super) const fn saw_gain(index: usize) -> f32 {
    1.0 / (index + 1) as f32
}

/// [`saw_gain`] for the odd partials and silence for the even ones: the
/// hollow spectrum of a square.
pub(super) const fn hollow_gain(index: usize) -> f32 {
    if (index + 1).is_multiple_of(2) {
        0.0
    } else {
        saw_gain(index)
    }
}

/// Forces `value` into the range of row `index` of `info`, replacing
/// anything that is not a number with the row's default. `None` past the
/// end of the table.
pub(super) fn cleaned_at(info: &[ParamInfo], index: usize, value: f32) -> Option<f32> {
    let row = info.get(index)?;
    Some(clean(value, row.min, row.max, row.default))
}

/// [`ParamSet::sanitized`] for any parameter struct whose `set` cleans what
/// it is given, which is every one in this module: reading each control and
/// writing it straight back forces the whole struct into range.
pub(super) fn cleaned<P: ParamSet>(params: &P) -> P {
    let mut clean_params = *params;
    for (index, row) in P::descriptors().iter().enumerate() {
        let value = params.get(index).unwrap_or(row.default);
        clean_params.set(index, value);
    }
    clean_params
}

/// [`ParamSet::approach`] for any parameter struct in this module.
/// Continuous controls close `amount` of the remaining distance; counts and
/// toggles jump.
pub(super) fn approached<P: ParamSet>(current: &mut P, target: &P, amount: f32) -> bool {
    let mut moving = false;
    for (index, row) in P::descriptors().iter().enumerate() {
        let (Some(from), Some(to)) = (current.get(index), target.get(index)) else {
            continue;
        };
        if row.kind == ParamKind::Float {
            let (value, unsettled) = approach_value(from, to, amount, row.max - row.min);
            current.set(index, value);
            moving |= unsettled;
        } else {
            current.set(index, to);
        }
    }
    moving
}
