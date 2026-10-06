//! Runtime state that is not part of the saved project: transport, meters,
//! audio devices, the file browser and export.
//!
//! The engine produces these, the app shell passes them on, and the UI reads
//! them through the TypeScript types `ts-rs` exports. docs/ARCHITECTURE.md
//! lists the IPC calls that carry them.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use windfall_project::PatternId;

/// What the transport plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PlayMode {
    /// Loops the selected pattern.
    Pattern,
    /// Plays the playlist.
    Song,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TransportState {
    pub playing: bool,
    pub mode: PlayMode,
    /// The pattern that plays in pattern mode.
    pub pattern: PatternId,
    /// In song mode, jump back to the start at the end of the last clip.
    pub loop_song: bool,
}

/// A change to the transport. `None` leaves a field alone.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct TransportPatch {
    #[ts(optional)]
    pub mode: Option<PlayMode>,
    #[ts(optional)]
    pub pattern: Option<PatternId>,
    #[ts(optional)]
    pub loop_song: Option<bool>,
}

/// What the engine reports to the UI 60 times a second.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RealtimeFrame {
    pub playing: bool,
    /// Playhead in ticks. In pattern mode it is the position inside the
    /// pattern; in song mode it is the position on the playlist.
    pub tick: f64,
    /// Peak levels since the last frame as linear gain, two per mixer track
    /// in mixer order: left, right, left, right. The master comes first.
    pub meters: Vec<f32>,
    /// Share of the audio buffer's time the engine spent computing it, 0 to
    /// 1. Near 1 means dropouts are close.
    pub cpu: f32,
    /// Audio buffers that arrived late since the stream started.
    pub xruns: u32,
    /// Sampler voices sounding right now.
    pub voices: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EngineStatus {
    /// An output stream is open and running.
    pub running: bool,
    /// Driver family, such as "WASAPI", "ASIO" or "CoreAudio".
    pub host: String,
    pub device: Option<String>,
    pub sample_rate: u32,
    /// Frames per audio callback.
    pub buffer_frames: u32,
    /// Output latency of one buffer in milliseconds.
    pub latency_ms: f32,
    /// Why the stream is not running, when it is not.
    pub error: Option<String>,
}

/// The audio output the user asked for. `None` means the system default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct AudioSettings {
    #[ts(optional)]
    pub host: Option<String>,
    #[ts(optional)]
    pub device: Option<String>,
    #[ts(optional)]
    pub sample_rate: Option<u32>,
    #[ts(optional)]
    pub buffer_frames: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AudioHost {
    pub name: String,
    pub is_default: bool,
    pub devices: Vec<AudioDevice>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AudioDevice {
    pub name: String,
    pub is_default: bool,
    /// Sample rates the device accepts, ascending.
    pub sample_rates: Vec<u32>,
    /// Smallest and largest buffer size in frames, when the driver says.
    pub min_buffer_frames: Option<u32>,
    pub max_buffer_frames: Option<u32>,
}

/// A top-level folder in the browser panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BrowserRoot {
    pub name: String,
    pub path: String,
    pub kind: BrowserRootKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum BrowserRootKind {
    /// Content shipped with Windfall.
    Factory,
    /// A folder the user added.
    User,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BrowserEntry {
    pub name: String,
    /// Absolute path.
    pub path: String,
    pub kind: BrowserEntryKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum BrowserEntryKind {
    Folder,
    /// An audio file Windfall can decode.
    Audio,
    /// A `.windfall` project file.
    Project,
    Other,
}

/// Facts about an audio file, with a waveform overview for drawing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SampleInfo {
    pub path: String,
    pub name: String,
    pub sample_rate: u32,
    pub channels: u16,
    #[ts(type = "number")]
    pub frames: u64,
    pub duration_secs: f64,
    /// Waveform overview as minimum and maximum pairs, one pair per bucket
    /// across the file, all channels mixed: min, max, min, max.
    pub peaks: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ExportFormat {
    Wav,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum BitDepth {
    Int16,
    Int24,
    Float32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ExportOptions {
    /// Absolute path of the file to write.
    pub path: String,
    pub format: ExportFormat,
    pub bit_depth: BitDepth,
    pub sample_rate: u32,
    /// Pattern mode renders the transport's pattern `pattern_loops` times.
    /// Song mode renders the playlist from the start to the last clip's end.
    pub mode: PlayMode,
    pub pattern_loops: u32,
    /// Extra time rendered after the end so reverb and release tails finish.
    pub tail_secs: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ExportProgress {
    pub path: String,
    /// 0 to 1.
    pub fraction: f32,
    pub done: bool,
    pub error: Option<String>,
}
