//! Review of an FL import before the open project is replaced.
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use windfall_flp::report::ImportReport;
use windfall_project::SampleId;

/// Optional folders supplied by the user; Windfall ships no FL Studio content.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct FlpImportOptions {
    pub factory_data_dir: Option<String>,
    pub user_data_dir: Option<String>,
    pub sample_search_folders: Vec<String>,
}

/// A missing or unreadable sample, with its identity for later recovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FlpMissingSample {
    pub sample: SampleId,
    pub name: String,
    pub path: String,
}

/// A prepared import. Its token opens exactly this checked conversion.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FlpImportPreview {
    #[ts(type = "number")]
    pub token: u64,
    pub name: String,
    pub report: ImportReport,
    pub warnings: Vec<String>,
    pub missing_samples: Vec<FlpMissingSample>,
    pub retained_plugins: u32,
}
