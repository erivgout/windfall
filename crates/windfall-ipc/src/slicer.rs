//! Review-only slicer IPC. Tokens own one document/source snapshot.
use serde::{Deserialize, Serialize};
pub use windfall_project::slicer::SliceOptions;
use windfall_project::{ClipId, slicer::SliceAnalysis};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SliceReview {
    pub token: u32,
    pub clip: ClipId,
    pub length_ticks: u32,
    pub analysis: SliceAnalysis,
}
