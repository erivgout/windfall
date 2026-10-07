//! Choices and review summaries for desktop MIDI files.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use windfall_project::{PatternId, PlayMode, Project};

/// Append MIDI notes to the current project, optionally in shared bar sections.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct MidiImportOptions {
    /// Zero means one pattern per part, split only at the model limit.
    pub bars: u32,
    pub share_patterns: bool,
    pub factory_drums: bool,
    pub tempo: bool,
    pub time_signature: bool,
    pub channel_mix: bool,
}

impl Default for MidiImportOptions {
    fn default() -> Self {
        Self {
            bars: 0,
            share_patterns: true,
            factory_drums: false,
            tempo: true,
            time_signature: true,
            channel_mix: true,
        }
    }
}

impl MidiImportOptions {
    /// The choices understood by the pure MIDI converter.
    pub fn core(&self) -> windfall_midi::ImportOptions {
        windfall_midi::ImportOptions {
            patterns: if self.bars == 0 {
                windfall_midi::PatternStrategy::PerTrack
            } else {
                windfall_midi::PatternStrategy::Bars {
                    bars: self.bars,
                    share: self.share_patterns,
                }
            },
            drum_kit: self.factory_drums.then(windfall_midi::DrumKit::factory),
            tempo: self.tempo,
            time_signature: self.time_signature,
            channel_mix: self.channel_mix,
        }
    }
}

/// Review of a prepared import; the token refers to exactly the bytes reviewed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MidiImportPreview {
    pub token: u32,
    pub channels: Vec<String>,
    pub patterns: u32,
    pub clips: u32,
    pub notes: u32,
    pub length: u32,
    pub adjustments: Vec<String>,
}

impl MidiImportPreview {
    /// UI facts without exposing project commands or predicted ids.
    pub fn from_plan(token: u32, plan: &windfall_midi::ImportPlan) -> Self {
        Self {
            token,
            channels: plan.channels.iter().map(|c| c.name.clone()).collect(),
            patterns: plan.patterns.len() as u32,
            clips: plan.clips.len() as u32,
            notes: plan.note_count() as u32,
            length: plan.length,
            adjustments: plan.adjustments.iter().map(ToString::to_string).collect(),
        }
    }
}

/// A MIDI file contains notes and tempo, with no rendered audio or effects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MidiExportOptions {
    pub mode: PlayMode,
    pub pattern: PatternId,
    /// Format 0 instead of the default format 1.
    pub single_track: bool,
    /// 480 or 960 ticks per quarter note; other values are refused.
    pub ppq: u16,
    pub running_status: bool,
    pub swing: bool,
}

impl MidiExportOptions {
    /// Converts a checked project snapshot into deterministic MIDI bytes.
    pub fn bytes(&self, project: &Project) -> Result<Vec<u8>, String> {
        project.check().map_err(|error| error.to_string())?;
        let resolution = match self.ppq {
            480 => windfall_midi::Resolution::Ppq480,
            960 => windfall_midi::Resolution::Ppq960,
            _ => return Err("MIDI resolution must be 480 or 960 PPQ.".to_owned()),
        };
        let options = windfall_midi::ExportOptions {
            swing: self.swing,
            ..Default::default()
        };
        let song = match self.mode {
            PlayMode::Pattern => windfall_midi::export_pattern(project, self.pattern, &options),
            PlayMode::Song => windfall_midi::export_song(project, &options),
        }
        .map_err(|error| error.to_string())?;
        Ok(windfall_midi::write(
            &song,
            &windfall_midi::WriteOptions {
                format: if self.single_track {
                    windfall_midi::SmfFormat::Single
                } else {
                    windfall_midi::SmfFormat::Multi
                },
                resolution,
                running_status: self.running_status,
            },
        ))
    }
}
