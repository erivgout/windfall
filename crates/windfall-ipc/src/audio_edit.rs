//! An editor token identifies one immutable rendered clip view, never a file to overwrite.
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use windfall_project::ClipId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AudioEditPreview {
    pub token: u32,
    pub clip: ClipId,
    pub name: String,
    pub frames: u32,
    pub sample_rate: u32,
    /// Stereo min/max envelope, retaining opposite-phase channels.
    pub peaks: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AudioEditOperation {
    Trim,
    Extract,
    Normalize,
    Reverse,
    FadeIn,
    FadeOut,
    Silence,
    Cut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AudioEditRequest {
    pub token: u32,
    pub operation: AudioEditOperation,
    /// Inclusive first frame and exclusive end frame of the clip view.
    pub start_frame: u32,
    pub end_frame: u32,
}
