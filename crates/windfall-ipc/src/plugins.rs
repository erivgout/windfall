//! Plugin manager messages, independent of native host types.
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use windfall_project::PluginTarget;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PluginEntry {
    pub path: String,
    pub format: String,
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub instrument: bool,
    pub usable: bool,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PluginInstanceStatus {
    pub target: PluginTarget,
    pub error: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PluginManagerState {
    pub folders: Vec<String>,
    pub entries: Vec<PluginEntry>,
    pub blocked: Vec<PluginEntry>,
    pub scanning: bool,
    pub completed: u32,
    pub total: u32,
    pub current: Option<String>,
    pub error: Option<String>,
    pub instances: Vec<PluginInstanceStatus>,
}
