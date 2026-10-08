//! Volatile native analysis protocol. Every u64 is a canonical decimal string;
//! no job/ticket/request may be saved in a project or reused after process exit.
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use windfall_project::ClipId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AnalysisProvenance {
    pub origin: String,
    pub source_revision: String,
    pub author: String,
    pub license_spdx: String,
    pub license_reference: String,
    pub adapter_id: String,
    pub adapter_version: String,
    pub device: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AnalysisOutputRole {
    pub role: String,
    pub channels: u16,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AnalysisModel {
    pub id: String,
    pub version: String,
    pub revision: String,
    pub sha256: String,
    pub bytes: String,
    pub max_bytes: String,
    pub provenance: AnalysisProvenance,
    pub sample_rate: u32,
    pub input_channels: u16,
    pub max_input_frames: String,
    pub outputs: Vec<AnalysisOutputRole>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AnalysisCapability {
    pub native: bool,
    pub available: bool,
    pub reason: Option<String>,
    pub models: Vec<AnalysisModel>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AnalysisSubmit {
    pub clip: ClipId,
    pub model_id: String,
    pub model_version: String,
    pub model_revision: String,
    pub start_frame: String,
    pub end_frame: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AnalysisStatus {
    Queued,
    Running,
    Cancelling,
    Cancelled,
    Failed,
    Ready,
    Consumed,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AnalysisJob {
    pub job: String,
    pub ticket: String,
    pub request: String,
    pub sequence: String,
    pub status: AnalysisStatus,
    pub completed_work: String,
    pub maximum_work: String,
    pub failure: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AnalysisArtifact {
    pub name: String,
    pub role: String,
    pub frames: String,
    pub channels: u16,
    pub sample_rate: u32,
    pub frame_origin: String,
    pub bytes: String,
    pub sha256: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AnalysisReview {
    pub job: AnalysisJob,
    pub clip: ClipId,
    pub generation: String,
    pub edit_revision: String,
    pub source_sha256: String,
    pub binding_sha256: String,
    pub start_frame: String,
    pub end_frame: String,
    pub input_frames: String,
    pub model: AnalysisModel,
    pub artifacts: Vec<AnalysisArtifact>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AnalysisApply {
    pub ticket: String,
    /// The submit request shown by the review. A mismatched/stale UI is refused.
    pub request: String,
    /// Replacement requires the complete rendered clip selection. Otherwise
    /// derived clips are added without deleting any original source or clip.
    pub replace_original: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identifiers_and_model_sizes_remain_strings_above_javascript_integer_limit() {
        let job = AnalysisJob {
            job: "18446744073709551614".into(),
            ticket: "9007199254740993".into(),
            request: "9007199254740994".into(),
            sequence: "9007199254740995".into(),
            status: AnalysisStatus::Ready,
            completed_work: "9007199254740996".into(),
            maximum_work: "9007199254740997".into(),
            failure: None,
        };
        let json = serde_json::to_value(&job).unwrap();
        assert_eq!(json["job"], job.job);
        assert_eq!(json["status"], "ready");
        assert_eq!(serde_json::from_value::<AnalysisJob>(json).unwrap(), job);
        assert!(AnalysisJob::decl(&ts_rs::Config::default()).contains("job: string"));
    }
}
