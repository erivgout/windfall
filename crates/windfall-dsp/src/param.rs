//! Parameter descriptions shared by every processor.
//!
//! Each processor has one plain parameter struct. That struct is what gets
//! saved in a project, sent to the audio thread and shown in the UI. Next to
//! it sits a table with one [`ParamInfo`] per control, which lets the UI
//! build an editor without knowing the processor, and lets automation
//! address any control of any processor by a number.

use serde::Serialize;
use ts_rs::TS;

/// What kind of control edits a parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ParamKind {
    /// A continuous value.
    Float,
    /// A whole number.
    Integer,
    /// On or off. As a number it is 0 or 1.
    Toggle,
    /// One of `choices`. As a number it is the index of the choice.
    Choice,
}

/// The unit a parameter's value is in, for labelling a control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ParamUnit {
    /// No unit: a count, a toggle or a choice.
    None,
    Decibels,
    Hertz,
    Milliseconds,
    Seconds,
    /// A share from 0 to 1, usually shown as a percentage.
    Fraction,
    Semitones,
    Cents,
    Octaves,
    /// A ratio such as a compressor's 4:1.
    Ratio,
    /// A linear gain factor, where 1 is unity.
    Gain,
    /// A position from -1 (left) to 1 (right).
    Pan,
}

/// How a control should spread a parameter's range over its travel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ParamScale {
    /// Equal steps of the value.
    Linear,
    /// Equal ratios of the value, as for frequencies and times. The minimum
    /// is always above zero.
    Logarithmic,
}

/// One option of a [`ParamKind::Choice`] parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ParamChoice {
    /// The value as it appears in the parameter struct's JSON.
    pub value: &'static str,
    /// The name to show.
    pub label: &'static str,
}

/// Describes one control of a processor.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ParamInfo {
    /// Where the value lives in the parameter struct's JSON, with dots
    /// between levels: `"thresholdDb"`, `"lowShelf.gainDb"`,
    /// `"oscillators.0.level"`.
    pub id: &'static str,
    /// The name to show.
    pub name: &'static str,
    pub kind: ParamKind,
    pub unit: ParamUnit,
    pub scale: ParamScale,
    /// Smallest value. For a toggle or a choice this is 0.
    pub min: f32,
    /// Largest value. For a toggle this is 1, and for a choice it is the
    /// index of the last choice.
    pub max: f32,
    pub default: f32,
    /// The options of a choice, in index order. Empty for the other kinds.
    pub choices: &'static [ParamChoice],
}

/// A processor's parameters: plain data that can be copied to the audio
/// thread, checked, described and addressed one control at a time.
///
/// The `index` every method takes is a position in
/// [`ParamSet::descriptors`]. Every control reads and writes as an `f32`:
/// toggles as 0 or 1, choices as the index of the choice.
pub trait ParamSet: Copy + Default + PartialEq + std::fmt::Debug + Send + Sync + 'static {
    /// The processor's name as shown to the user.
    const NAME: &'static str;

    /// One entry per control. The order never changes within a version of
    /// the processor.
    fn descriptors() -> &'static [ParamInfo];

    /// The value of the control at `index`, or `None` past the end.
    fn get(&self, index: usize) -> Option<f32>;

    /// Sets the control at `index`, forcing the value into range. Returns
    /// false past the end.
    fn set(&mut self, index: usize, value: f32) -> bool;

    /// A copy with every value forced into its range and anything that is
    /// not a number replaced by the default. Processors call this on
    /// whatever they are given, so a damaged project file cannot make one
    /// misbehave.
    fn sanitized(&self) -> Self;

    /// Moves every continuous value `amount` (0 to 1) of the way to its
    /// value in `target` and snaps onto it once the rest is negligible.
    /// Toggles, choices and whole numbers jump. Returns true while
    /// anything is still on its way.
    fn approach(&mut self, target: &Self, amount: f32) -> bool;

    /// The index of the control with the given id.
    fn index_of(id: &str) -> Option<usize> {
        Self::descriptors().iter().position(|info| info.id == id)
    }
}

/// One step of [`ParamSet::approach`] for a continuous value. The second
/// result is true while the value has not arrived.
#[inline]
pub(crate) fn approach_value(current: f32, target: f32, amount: f32, span: f32) -> (f32, bool) {
    let gap = target - current;
    if gap.abs() <= span * 1.0e-4 {
        (target, false)
    } else {
        (current + gap * amount, true)
    }
}

/// Implements [`ParamSet`] for a parameter struct from a table with one row
/// per control:
///
/// ```text
/// float  [field.path] "json.path" "Name" { Unit, Scale, min, max, default }
/// int    [field.path] "json.path" "Name" { Unit, min, max, default }
/// toggle [field.path] "json.path" "Name" { default }
/// choice [field.path] "json.path" "Name" { Enum, Default, [Variant "json" "Label", ...] }
/// ```
///
/// The defaults in the table must agree with the struct's `Default`, and the
/// JSON paths and choice values with its serde names. The crate's tests
/// check both for every processor.
macro_rules! param_set {
    (
        $ty:ty, $name:literal, {
            $( $kind:ident [$($path:tt)+] $id:literal $label:literal { $($args:tt)* } )+
        }
    ) => {
        impl $crate::param::ParamSet for $ty {
            const NAME: &'static str = $name;

            fn descriptors() -> &'static [$crate::param::ParamInfo] {
                static INFO: &[$crate::param::ParamInfo] = &[
                    $( $crate::param::param_row!(@info $kind $id $label { $($args)* }) ),+
                ];
                INFO
            }

            #[allow(unused_assignments)]
            fn get(&self, index: usize) -> Option<f32> {
                let mut at = 0_usize;
                $(
                    if index == at {
                        return Some($crate::param::param_row!(
                            @get $kind (self.$($path)+) { $($args)* }
                        ));
                    }
                    at += 1;
                )+
                None
            }

            #[allow(unused_assignments)]
            fn set(&mut self, index: usize, value: f32) -> bool {
                let mut at = 0_usize;
                $(
                    if index == at {
                        $crate::param::param_row!(
                            @set $kind (self.$($path)+) value { $($args)* }
                        );
                        return true;
                    }
                    at += 1;
                )+
                false
            }

            #[allow(unused_mut)] // A struct containing only toggles needs no cleaning.
            fn sanitized(&self) -> Self {
                let mut clean = *self;
                $( $crate::param::param_row!(@clean $kind (clean.$($path)+) { $($args)* }); )+
                clean
            }

            #[allow(unused_mut, unused_variables)] // Toggle-only structs jump without interpolation.
            fn approach(&mut self, target: &Self, amount: f32) -> bool {
                let mut moving = false;
                $(
                    $crate::param::param_row!(
                        @approach $kind (self.$($path)+) (target.$($path)+) amount moving
                        { $($args)* }
                    );
                )+
                moving
            }
        }
    };
}

macro_rules! param_row {
    (@info float $id:literal $label:literal
        { $unit:ident, $scale:ident, $min:expr, $max:expr, $default:expr }) => {
        $crate::param::ParamInfo {
            id: $id,
            name: $label,
            kind: $crate::param::ParamKind::Float,
            unit: $crate::param::ParamUnit::$unit,
            scale: $crate::param::ParamScale::$scale,
            min: $min,
            max: $max,
            default: $default,
            choices: &[],
        }
    };
    (@get float ($place:expr) { $($args:tt)* }) => {
        $place
    };
    (@set float ($place:expr) $value:ident
        { $unit:ident, $scale:ident, $min:expr, $max:expr, $default:expr }) => {
        $place = $crate::blocks::math::clean($value, $min, $max, $default)
    };
    (@clean float ($place:expr)
        { $unit:ident, $scale:ident, $min:expr, $max:expr, $default:expr }) => {
        $place = $crate::blocks::math::clean($place, $min, $max, $default)
    };
    (@approach float ($place:expr) ($target:expr) $amount:ident $moving:ident
        { $unit:ident, $scale:ident, $min:expr, $max:expr, $default:expr }) => {
        let (value, unsettled) =
            $crate::param::approach_value($place, $target, $amount, $max - $min);
        $place = value;
        $moving |= unsettled;
    };

    (@info int $id:literal $label:literal
        { $unit:ident, $min:expr, $max:expr, $default:expr }) => {
        $crate::param::ParamInfo {
            id: $id,
            name: $label,
            kind: $crate::param::ParamKind::Integer,
            unit: $crate::param::ParamUnit::$unit,
            scale: $crate::param::ParamScale::Linear,
            min: $min as f32,
            max: $max as f32,
            default: $default as f32,
            choices: &[],
        }
    };
    (@get int ($place:expr) { $($args:tt)* }) => {
        $place as f32
    };
    (@set int ($place:expr) $value:ident
        { $unit:ident, $min:expr, $max:expr, $default:expr }) => {
        $place = $crate::blocks::math::clean(
            $value, $min as f32, $max as f32, $default as f32,
        )
        .round() as _
    };
    (@clean int ($place:expr) { $unit:ident, $min:expr, $max:expr, $default:expr }) => {
        $place = $place.clamp($min, $max)
    };
    (@approach int ($place:expr) ($target:expr) $amount:ident $moving:ident
        { $($args:tt)* }) => {
        $place = $target;
    };

    (@info toggle $id:literal $label:literal { $default:expr }) => {
        $crate::param::ParamInfo {
            id: $id,
            name: $label,
            kind: $crate::param::ParamKind::Toggle,
            unit: $crate::param::ParamUnit::None,
            scale: $crate::param::ParamScale::Linear,
            min: 0.0,
            max: 1.0,
            default: if $default { 1.0 } else { 0.0 },
            choices: &[],
        }
    };
    (@get toggle ($place:expr) { $($args:tt)* }) => {
        if $place { 1.0 } else { 0.0 }
    };
    (@set toggle ($place:expr) $value:ident { $($args:tt)* }) => {
        $place = $value >= 0.5
    };
    (@clean toggle ($place:expr) { $($args:tt)* }) => {};
    (@approach toggle ($place:expr) ($target:expr) $amount:ident $moving:ident
        { $($args:tt)* }) => {
        $place = $target;
    };

    (@info choice $id:literal $label:literal
        { $enum:ident, $default:ident, [ $( $variant:ident $value:literal $name:literal ),+ ] }) => {
        $crate::param::ParamInfo {
            id: $id,
            name: $label,
            kind: $crate::param::ParamKind::Choice,
            unit: $crate::param::ParamUnit::None,
            scale: $crate::param::ParamScale::Linear,
            min: 0.0,
            max: {
                let mut count = -1.0_f32;
                $(
                    let _ = $value;
                    count += 1.0;
                )+
                count
            },
            default: {
                let mut index = 0.0_f32;
                let mut at = 0.0_f32;
                $(
                    if matches!($enum::$default, $enum::$variant) {
                        index = at;
                    }
                    at += 1.0;
                )+
                let _ = at;
                index
            },
            choices: &[
                $( $crate::param::ParamChoice { value: $value, label: $name } ),+
            ],
        }
    };
    (@get choice ($place:expr)
        { $enum:ident, $default:ident, [ $( $variant:ident $value:literal $name:literal ),+ ] }) => {{
        let mut index = 0.0_f32;
        let mut at = 0.0_f32;
        $(
            if matches!($place, $enum::$variant) {
                index = at;
            }
            at += 1.0;
        )+
        let _ = at;
        index
    }};
    (@set choice ($place:expr) $number:ident
        { $enum:ident, $default:ident, [ $( $variant:ident $value:literal $name:literal ),+ ] }) => {{
        let mut last = -1.0_f32;
        $(
            let _ = $value;
            last += 1.0;
        )+
        // Anything that is not a number leaves the choice as it is.
        let wanted = if $number.is_finite() { $number.round().clamp(0.0, last) } else { -1.0 };
        let mut at = 0.0_f32;
        $(
            if wanted == at {
                $place = $enum::$variant;
            }
            at += 1.0;
        )+
        let _ = at;
    }};
    (@clean choice ($place:expr) { $($args:tt)* }) => {};
    (@approach choice ($place:expr) ($target:expr) $amount:ident $moving:ident
        { $($args:tt)* }) => {
        $place = $target;
    };
}

pub(crate) use {param_row, param_set};
