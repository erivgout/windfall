//! A plugin's parameters as the host sees them.
//!
//! A plugin describes its parameters at run time, so these hold owned
//! strings where `windfall_dsp::ParamInfo` holds static ones. Apart from
//! that [`PluginParamInfo`] has the same fields and the same JSON, so an
//! editor built for the descriptors of a built-in effect can lay out a
//! plugin's parameters too.

use serde::Serialize;
use windfall_dsp::{ParamKind, ParamScale, ParamUnit};

/// One parameter of a plugin, in the plugin's own terms.
#[derive(Debug, Clone, PartialEq)]
pub struct PluginParam {
    /// The plugin's permanent number for the parameter. Automation and
    /// saved values refer to this, never to the position in the list, which
    /// a plugin update can change.
    pub id: u32,
    pub name: String,
    /// The group the plugin puts the parameter in, with `/` between levels,
    /// such as `Oscillators/Wavetable 1`. Empty if it has none.
    pub module: String,
    /// The range and default, in the plugin's own units.
    pub min: f64,
    pub max: f64,
    pub default: f64,
    /// The value is a whole number.
    pub stepped: bool,
    /// The value is one of a list of named choices. Implies `stepped`.
    pub choice: bool,
    /// The host may automate it.
    pub automatable: bool,
    /// The host cannot set it. It is a reading, such as a meter.
    pub read_only: bool,
    /// The plugin asks for it not to be shown.
    pub hidden: bool,
    /// The plugin's own bypass switch.
    pub bypass: bool,
    /// The value wraps around, as a phase does.
    pub periodic: bool,
}

/// The most choices a parameter may have for the host to list them by name.
pub const MAX_LISTED_CHOICES: usize = 128;

impl PluginParam {
    /// Which of the app's controls fits the parameter.
    pub fn kind(&self) -> ParamKind {
        if !self.stepped {
            ParamKind::Float
        } else if self.choice {
            ParamKind::Choice
        } else if self.min == 0.0 && self.max == 1.0 {
            ParamKind::Toggle
        } else {
            ParamKind::Integer
        }
    }

    /// `value` forced into the parameter's range, and onto a whole number
    /// if the parameter is stepped. Anything that is not a number becomes
    /// the default.
    pub fn clamp(&self, value: f64) -> f64 {
        if !value.is_finite() {
            return self.default;
        }
        let (low, high) = if self.min <= self.max {
            (self.min, self.max)
        } else {
            (self.max, self.min)
        };
        let value = value.clamp(low, high);
        if self.stepped { value.round() } else { value }
    }

    /// How many values a stepped parameter has, if that is few enough to
    /// list.
    pub fn step_count(&self) -> Option<usize> {
        if !self.stepped {
            return None;
        }
        let steps = (self.max - self.min).round();
        (steps >= 0.0 && steps < MAX_LISTED_CHOICES as f64).then(|| steps as usize + 1)
    }

    /// The parameter in the shape the app's own descriptors have. `choices`
    /// are the names of a choice parameter's values in order, which only
    /// the plugin knows: see
    /// [`PluginInstance::param_info`](crate::PluginInstance::param_info).
    pub fn info(&self, choices: Vec<String>) -> PluginParamInfo {
        let choices = choices
            .into_iter()
            .enumerate()
            .map(|(index, label)| PluginParamChoice {
                value: index.to_string(),
                label,
            })
            .collect();
        PluginParamInfo {
            id: self.id.to_string(),
            name: self.name.clone(),
            kind: self.kind(),
            unit: ParamUnit::None,
            scale: ParamScale::Linear,
            min: self.min as f32,
            max: self.max as f32,
            default: self.default as f32,
            choices,
        }
    }
}

/// One named value of a choice parameter, shaped like
/// `windfall_dsp::ParamChoice`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginParamChoice {
    /// The choice's position in the list, as text.
    pub value: String,
    /// The name the plugin gives the value.
    pub label: String,
}

/// A plugin parameter, shaped like `windfall_dsp::ParamInfo`.
///
/// A plugin does not say what unit a parameter is in or how its range
/// should be spread over a knob, so `unit` is always `None` and `scale`
/// always `Linear`. The text a control shows comes from the plugin instead:
/// [`PluginInstance::param_text`](crate::PluginInstance::param_text).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginParamInfo {
    /// The plugin's parameter id, as text.
    pub id: String,
    pub name: String,
    pub kind: ParamKind,
    pub unit: ParamUnit,
    pub scale: ParamScale,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub choices: Vec<PluginParamChoice>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn param(min: f64, max: f64, stepped: bool, choice: bool) -> PluginParam {
        PluginParam {
            id: 7,
            name: "Gain".to_owned(),
            module: String::new(),
            min,
            max,
            default: min,
            stepped,
            choice,
            automatable: true,
            read_only: false,
            hidden: false,
            bypass: false,
            periodic: false,
        }
    }

    #[test]
    fn the_kind_follows_the_flags_and_the_range() {
        assert_eq!(param(0.0, 2.0, false, false).kind(), ParamKind::Float);
        assert_eq!(param(0.0, 1.0, true, false).kind(), ParamKind::Toggle);
        assert_eq!(param(1.0, 8.0, true, false).kind(), ParamKind::Integer);
        assert_eq!(param(0.0, 3.0, true, true).kind(), ParamKind::Choice);
    }

    #[test]
    fn values_are_forced_into_range() {
        let gain = param(0.0, 2.0, false, false);
        assert_eq!(gain.clamp(3.0), 2.0);
        assert_eq!(gain.clamp(-1.0), 0.0);
        assert_eq!(gain.clamp(f64::NAN), 0.0);
        assert_eq!(param(0.0, 4.0, true, false).clamp(2.6), 3.0);
    }

    #[test]
    fn the_info_serializes_like_a_built_in_descriptor() {
        let info = param(0.0, 1.0, true, true).info(vec!["Off".to_owned(), "On".to_owned()]);
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "id": "7", "name": "Gain", "kind": "choice", "unit": "none", "scale": "linear",
                "min": 0.0, "max": 1.0, "default": 0.0,
                "choices": [
                    { "value": "0", "label": "Off" },
                    { "value": "1", "label": "On" },
                ],
            })
        );
    }
}
