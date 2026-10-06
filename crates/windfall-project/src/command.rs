//! Every edit to a project is a [`Command`].
//!
//! The UI, the command palette, scripting and the assistant all change a
//! project the same way: they build a command and dispatch it to the
//! [`Document`](crate::Document). Commands are plain data, so they cross the
//! IPC boundary as JSON.
//!
//! Patch structs hold one `Option` per field. `None` leaves the field alone.
//!
//! A number outside the range its field documents is brought into the range,
//! so a control can send what it has. Input that has no nearest valid value
//! fails the command instead: an id that does not exist, a key above 127, a
//! note or clip with no length, a color above 0xFFFFFF, NaN. An index past
//! the end of a list means the end.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::model::{
    ChannelId, ClipContent, ClipId, Envelope, NoteId, PatternId, PlaylistTrackId, SampleId,
    SamplePath, TimeSignature, TrackId,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum Command {
    // Project
    UpdateSettings {
        patch: SettingsPatch,
    },

    // Sample pool
    /// Registers an audio file. Creates a [`SampleId`]. If a sample with the
    /// same path is already in the pool, nothing is added and its id is
    /// reported as the created id.
    AddSample {
        name: String,
        path: SamplePath,
    },
    /// Fails while any channel still uses the sample.
    RemoveSample {
        id: SampleId,
    },

    // Channel rack
    /// Adds a sampler channel and, unless `mixer_track` is given, a new mixer
    /// track routed to the master that the channel plays into. Creates a
    /// [`ChannelId`] and then, if a track was made, a [`TrackId`]. When the
    /// mixer is full, no track is made and the channel plays into the master.
    AddChannel {
        #[serde(default)]
        #[ts(optional)]
        /// Defaults to the sample name, or "Sampler".
        name: Option<String>,
        #[serde(default)]
        #[ts(optional)]
        sample: Option<SampleId>,
        #[serde(default)]
        #[ts(optional)]
        /// Position in the rack. Defaults to the end.
        index: Option<u32>,
        #[serde(default)]
        #[ts(optional)]
        mixer_track: Option<TrackId>,
    },
    /// Removes the channel and its notes from every pattern. The mixer track
    /// it was routed to stays.
    RemoveChannel {
        id: ChannelId,
    },
    /// Copies the channel and its notes in every pattern. The copy goes right
    /// below the original and shares its mixer track. Its name is the
    /// original's with a number, as in "Kick #2". Creates a [`ChannelId`].
    DuplicateChannel {
        id: ChannelId,
    },
    MoveChannel {
        id: ChannelId,
        index: u32,
    },
    UpdateChannel {
        id: ChannelId,
        patch: ChannelPatch,
    },
    SetChannelSample {
        id: ChannelId,
        #[serde(default)]
        #[ts(optional)]
        sample: Option<SampleId>,
    },
    UpdateSampler {
        id: ChannelId,
        patch: SamplerPatch,
    },
    /// Turns the sampler's volume envelope on (`Some`) or off (`None`).
    SetSamplerEnvelope {
        id: ChannelId,
        #[serde(default)]
        #[ts(optional)]
        envelope: Option<Envelope>,
    },

    // Patterns
    /// Creates a [`PatternId`].
    AddPattern {
        #[serde(default)]
        #[ts(optional)]
        /// Defaults to "Pattern N".
        name: Option<String>,
    },
    /// Removes the pattern and every playlist clip that plays it. Fails on
    /// the last remaining pattern.
    RemovePattern {
        id: PatternId,
    },
    /// Creates a [`PatternId`]. The copy goes right after the original. Its
    /// name is the original's with a number, as in "Pattern 1 #2".
    DuplicatePattern {
        id: PatternId,
    },
    MovePattern {
        id: PatternId,
        index: u32,
    },
    UpdatePattern {
        id: PatternId,
        patch: PatternPatch,
    },

    // Notes
    /// Step sequencer click. If the channel has a note starting exactly at
    /// this step, removes every note that starts there. Otherwise adds a
    /// one-step note at [`DEFAULT_KEY`](crate::model::DEFAULT_KEY) and
    /// creates a [`NoteId`].
    ToggleStep {
        pattern: PatternId,
        channel: ChannelId,
        step: u32,
    },
    /// Creates one [`NoteId`] per note, in order.
    AddNotes {
        pattern: PatternId,
        channel: ChannelId,
        notes: Vec<NoteInit>,
    },
    RemoveNotes {
        pattern: PatternId,
        channel: ChannelId,
        notes: Vec<NoteId>,
    },
    UpdateNotes {
        pattern: PatternId,
        channel: ChannelId,
        updates: Vec<NoteUpdate>,
    },
    /// Removes every note the channel has in the pattern.
    ClearLane {
        pattern: PatternId,
        channel: ChannelId,
    },

    // Mixer
    /// Adds a track routed to the master. Creates a [`TrackId`]. Fails when
    /// the mixer already holds
    /// [`MAX_MIXER_TRACKS`](crate::model::MAX_MIXER_TRACKS).
    AddMixerTrack {
        #[serde(default)]
        #[ts(optional)]
        /// Defaults to "Insert N".
        name: Option<String>,
    },
    /// Fails on the master. Channels and tracks that were routed to the
    /// removed track are rerouted to the master, and sends to it are dropped.
    RemoveMixerTrack {
        id: TrackId,
    },
    UpdateMixerTrack {
        id: TrackId,
        patch: MixerTrackPatch,
    },
    /// Fails on the master, and when the change would create a routing cycle.
    SetTrackOutput {
        id: TrackId,
        #[serde(default)]
        #[ts(optional)]
        output: Option<TrackId>,
    },
    /// Adds or changes a send (`Some`), or removes it (`None`). Fails when
    /// the send would create a routing cycle, and on a send from the master.
    SetSend {
        from: TrackId,
        to: TrackId,
        #[serde(default)]
        #[ts(optional)]
        gain: Option<f32>,
    },

    // Playlist
    /// Creates a [`PlaylistTrackId`].
    AddPlaylistTrack {
        #[serde(default)]
        #[ts(optional)]
        /// Defaults to "Track N".
        name: Option<String>,
    },
    /// Removes the track and its clips.
    RemovePlaylistTrack {
        id: PlaylistTrackId,
    },
    UpdatePlaylistTrack {
        id: PlaylistTrackId,
        patch: PlaylistTrackPatch,
    },
    /// Creates one [`ClipId`] per clip, in order.
    AddClips {
        clips: Vec<ClipInit>,
    },
    RemoveClips {
        clips: Vec<ClipId>,
    },
    UpdateClips {
        updates: Vec<ClipUpdate>,
    },

    /// Runs several commands as one undo step. If any of them fails, none of
    /// them is applied.
    Batch {
        #[serde(default)]
        #[ts(optional)]
        /// History label. Defaults to the label of the first command.
        label: Option<String>,
        commands: Vec<Command>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct SettingsPatch {
    #[ts(optional)]
    pub name: Option<String>,
    #[ts(optional)]
    pub tempo_bpm: Option<f64>,
    #[ts(optional)]
    pub time_signature: Option<TimeSignature>,
    #[ts(optional)]
    pub swing: Option<f32>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ChannelPatch {
    #[ts(optional)]
    pub name: Option<String>,
    #[ts(optional)]
    pub color: Option<u32>,
    #[ts(optional)]
    pub volume: Option<f32>,
    #[ts(optional)]
    pub pan: Option<f32>,
    #[ts(optional)]
    pub muted: Option<bool>,
    #[ts(optional)]
    pub solo: Option<bool>,
    #[ts(optional)]
    pub mixer_track: Option<TrackId>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct SamplerPatch {
    #[ts(optional)]
    pub root_key: Option<u8>,
    #[ts(optional)]
    pub tune: Option<f32>,
    #[ts(optional)]
    pub gain: Option<f32>,
    #[ts(optional)]
    pub start: Option<f32>,
    #[ts(optional)]
    pub end: Option<f32>,
    #[ts(optional)]
    pub reverse: Option<bool>,
    #[ts(optional)]
    pub cut_self: Option<bool>,
    #[ts(optional)]
    pub cut_group: Option<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct PatternPatch {
    #[ts(optional)]
    pub name: Option<String>,
    #[ts(optional)]
    pub color: Option<u32>,
    #[ts(optional)]
    pub length_steps: Option<u32>,
}

/// A note to add. Missing fields take the defaults a step would get.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NoteInit {
    pub start: u32,
    pub length: u32,
    pub key: u8,
    #[serde(default)]
    #[ts(optional)]
    pub velocity: Option<f32>,
    #[serde(default)]
    #[ts(optional)]
    pub pan: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NoteUpdate {
    pub id: NoteId,
    pub patch: NotePatch,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct NotePatch {
    #[ts(optional)]
    pub start: Option<u32>,
    #[ts(optional)]
    pub length: Option<u32>,
    #[ts(optional)]
    pub key: Option<u8>,
    #[ts(optional)]
    pub velocity: Option<f32>,
    #[ts(optional)]
    pub pan: Option<f32>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct MixerTrackPatch {
    #[ts(optional)]
    pub name: Option<String>,
    #[ts(optional)]
    pub color: Option<u32>,
    #[ts(optional)]
    pub volume: Option<f32>,
    #[ts(optional)]
    pub pan: Option<f32>,
    #[ts(optional)]
    pub muted: Option<bool>,
    #[ts(optional)]
    pub solo: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct PlaylistTrackPatch {
    #[ts(optional)]
    pub name: Option<String>,
    #[ts(optional)]
    pub muted: Option<bool>,
}

/// A clip to add.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ClipInit {
    pub track: PlaylistTrackId,
    pub start: u32,
    /// Defaults to the natural length of the content: one pass of a pattern.
    #[serde(default)]
    #[ts(optional)]
    pub length: Option<u32>,
    pub content: ClipContent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ClipUpdate {
    pub id: ClipId,
    pub patch: ClipPatch,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ClipPatch {
    #[ts(optional)]
    pub track: Option<PlaylistTrackId>,
    #[ts(optional)]
    pub start: Option<u32>,
    #[ts(optional)]
    pub length: Option<u32>,
    #[ts(optional)]
    pub offset: Option<u32>,
    #[ts(optional)]
    pub muted: Option<bool>,
}
