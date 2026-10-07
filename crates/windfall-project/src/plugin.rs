//! Saved references to hosted plugins, independent of native hosting code.

use crate::{ChannelId, EffectId};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// The project object a plugin replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum PluginTarget {
    Instrument { channel: ChannelId },
    Effect { effect: EffectId },
}

/// A discovered parameter in native units. Its id survives list reordering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PluginParameter {
    pub id: u32,
    pub name: String,
    pub min: f32,
    pub max: f32,
    pub value: f32,
    pub stepped: bool,
    pub read_only: bool,
    pub automatable: bool,
}

/// Everything needed to reopen an instance. Missing files retain this data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PluginBinding {
    pub target: PluginTarget,
    /// "clap" or "vst3"; the model does not load either format.
    pub format: String,
    pub path: String,
    pub id: String,
    pub name: String,
    /// Host-wrapped opaque state, including its version header.
    #[serde(default)]
    pub state: Vec<u8>,
    #[serde(default)]
    pub parameters: Vec<PluginParameter>,
}

impl PluginBinding {
    /// Validates bounded saved data before it reaches native code.
    pub fn validate(&self) -> Result<(), &'static str> {
        if !["clap", "vst3"].contains(&self.format.as_str())
            || self.id.is_empty()
            || self.path.is_empty()
        {
            return Err("a plugin needs its format, file and identifier");
        }
        if self.state.len() > 256 * 1024 * 1024 || self.parameters.len() > 4096 {
            return Err("the saved plugin state or parameter list is too large");
        }
        let mut ids = std::collections::HashSet::new();
        for param in &self.parameters {
            if !ids.insert(param.id)
                || !param.min.is_finite()
                || !param.max.is_finite()
                || !param.value.is_finite()
                || param.min > param.max
                || !(param.min..=param.max).contains(&param.value)
            {
                return Err(
                    "a plugin parameter has an invalid range, value or repeated identifier",
                );
            }
        }
        Ok(())
    }
}

impl crate::Project {
    /// The saved plugin that replaces this instrument or effect.
    pub fn plugin(&self, target: PluginTarget) -> Option<&PluginBinding> {
        self.plugins.iter().find(|plugin| plugin.target == target)
    }
}
