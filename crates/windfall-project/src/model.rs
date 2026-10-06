//! The project model: everything that is saved in a `.windfall` file.
//!
//! These types are the contract between the engine, the app shell and the UI.
//! They serialize to camelCase JSON, and `ts-rs` exports matching TypeScript
//! types. New fields must carry `#[serde(default)]` so older files still load;
//! anything that cannot be expressed that way needs a [`FORMAT_VERSION`] bump
//! and a migration.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use windfall_core::{PPQ, TICKS_PER_STEP};

/// Version of the on-disk project format.
pub const FORMAT_VERSION: u32 = 1;

/// Mixer tracks including the master. The engine sizes its meter storage
/// from this, so it is a hard limit.
pub const MAX_MIXER_TRACKS: usize = 128;

/// MIDI key a step-sequencer step plays, and the default root key of a
/// sampler. FL Studio labels this key C5.
pub const DEFAULT_KEY: u8 = 60;

pub const DEFAULT_VELOCITY: f32 = 0.8;
pub const DEFAULT_CHANNEL_VOLUME: f32 = 0.8;
pub const DEFAULT_PATTERN_STEPS: u32 = 16;

/// Longest pattern, in sixteenth-note steps.
pub const MAX_PATTERN_STEPS: u32 = 1024;

pub const MIN_TEMPO_BPM: f64 = 10.0;
pub const MAX_TEMPO_BPM: f64 = 522.0;

/// Highest linear gain a channel or mixer fader allows, about +6 dB.
pub const MAX_GAIN: f32 = 2.0;

/// Furthest a sampler can be tuned up or down, in semitones.
pub const MAX_TUNE_SEMITONES: f32 = 48.0;

/// Longest attack, decay or release time of an envelope, in milliseconds.
pub const MAX_ENVELOPE_MS: f32 = 60_000.0;

/// Highest MIDI key.
pub const MAX_KEY: u8 = 127;

/// Colors handed to new channels, patterns and mixer tracks, as 0xRRGGBB.
/// Neighbours are far apart in hue so adjacent rows are easy to tell apart.
pub const PALETTE: [u32; 12] = [
    0xE5484D, 0x12A594, 0xFFB224, 0x6E56CF, 0x46A758, 0xD6409F, 0x05A2C2, 0xF76B15, 0x3E63DD,
    0x99D52A, 0x8E4EC6, 0xE93D82,
];

/// Color of the master mixer track in a new project.
pub const MASTER_COLOR: u32 = 0x8B8D98;

/// The palette color for the `index`th item of a list. Cycles.
pub fn palette_color(index: usize) -> u32 {
    PALETTE[index % PALETTE.len()]
}

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS,
        )]
        #[serde(transparent)]
        #[ts(export)]
        pub struct $name(pub u32);

        impl From<u32> for $name {
            fn from(value: u32) -> Self {
                Self(value)
            }
        }

        impl From<$name> for u32 {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

id_type!(
    /// Identifies a sample in the project's sample pool.
    SampleId
);
id_type!(
    /// Identifies a channel in the channel rack.
    ChannelId
);
id_type!(
    /// Identifies a pattern.
    PatternId
);
id_type!(
    /// Identifies a note. Unique across the whole project.
    NoteId
);
id_type!(
    /// Identifies a mixer track. The master is always [`TrackId::MASTER`].
    TrackId
);
id_type!(
    /// Identifies a playlist track (a lane of the song timeline).
    PlaylistTrackId
);
id_type!(
    /// Identifies a clip on the playlist.
    ClipId
);

impl TrackId {
    pub const MASTER: TrackId = TrackId(0);
}

/// A whole project. Ids are allocated from `next_id` and never reused, so an
/// id means the same thing for the life of the project, across undo and redo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Project {
    pub format_version: u32,
    /// The next id to hand out. Id 0 is reserved for the master mixer track.
    pub next_id: u32,
    pub settings: ProjectSettings,
    pub samples: Vec<SampleAsset>,
    /// Channel rack, in display order.
    pub channels: Vec<Channel>,
    /// Patterns, in display order. A project always has at least one.
    pub patterns: Vec<Pattern>,
    pub mixer: Mixer,
    pub playlist: Playlist,
}

impl Project {
    /// An empty project: 120 bpm in 4/4, one empty pattern, and a mixer that
    /// holds only the master track.
    pub fn new(name: impl Into<String>) -> Self {
        let pattern = PatternId(1);
        Self {
            format_version: FORMAT_VERSION,
            next_id: pattern.0 + 1,
            settings: ProjectSettings {
                name: name.into(),
                tempo_bpm: 120.0,
                time_signature: TimeSignature {
                    numerator: 4,
                    denominator: 4,
                },
                swing: 0.0,
            },
            samples: Vec::new(),
            channels: Vec::new(),
            patterns: vec![Pattern {
                id: pattern,
                name: "Pattern 1".to_owned(),
                color: palette_color(0),
                length_steps: DEFAULT_PATTERN_STEPS,
                lanes: Vec::new(),
            }],
            mixer: Mixer {
                tracks: vec![MixerTrack {
                    id: TrackId::MASTER,
                    name: "Master".to_owned(),
                    color: MASTER_COLOR,
                    volume: 1.0,
                    pan: 0.0,
                    muted: false,
                    solo: false,
                    output: None,
                    sends: Vec::new(),
                }],
            },
            playlist: Playlist {
                tracks: Vec::new(),
                clips: Vec::new(),
            },
        }
    }

    pub fn sample(&self, id: SampleId) -> Option<&SampleAsset> {
        self.samples.iter().find(|sample| sample.id == id)
    }

    pub fn channel(&self, id: ChannelId) -> Option<&Channel> {
        self.channels.iter().find(|channel| channel.id == id)
    }

    pub fn pattern(&self, id: PatternId) -> Option<&Pattern> {
        self.patterns.iter().find(|pattern| pattern.id == id)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProjectSettings {
    pub name: String,
    /// Tempo in beats per minute, [`MIN_TEMPO_BPM`] to [`MAX_TEMPO_BPM`].
    pub tempo_bpm: f64,
    pub time_signature: TimeSignature,
    /// Swing amount from 0 to 1. It delays every second sixteenth-note step.
    /// At 1 that step lands two thirds of the way through its pair of steps,
    /// which is a full triplet feel.
    pub swing: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TimeSignature {
    /// Beats per bar, 1 to 16.
    pub numerator: u8,
    /// Beat unit: 2, 4, 8 or 16.
    pub denominator: u8,
}

impl TimeSignature {
    /// Length of one bar in ticks.
    pub fn ticks_per_bar(self) -> u32 {
        self.numerator as u32 * PPQ * 4 / self.denominator as u32
    }
}

/// An audio file the project uses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SampleAsset {
    pub id: SampleId,
    pub name: String,
    pub path: SamplePath,
}

/// Where a sample's audio file lives.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(tag = "kind", content = "path", rename_all = "camelCase")]
#[ts(export)]
pub enum SamplePath {
    /// Shipped with Windfall. The path is relative to the factory content
    /// folder and uses forward slashes.
    Factory(String),
    /// Inside the project folder. The path is relative to it and uses forward
    /// slashes.
    Project(String),
    /// Anywhere else on disk, as an absolute path.
    External(String),
}

impl SamplePath {
    /// Says what is wrong with the stored path, or `None` when it is well
    /// formed. Relative paths must use forward slashes and stay inside their
    /// folder. Whether the file exists is not checked.
    pub fn problem(&self) -> Option<&'static str> {
        match self {
            SamplePath::External(path) if path.is_empty() => Some("the path is empty"),
            SamplePath::External(_) => None,
            SamplePath::Factory(path) | SamplePath::Project(path) => relative_path_problem(path),
        }
    }
}

/// Says what stops `path` being stored as a relative sample path.
pub(crate) fn relative_path_problem(path: &str) -> Option<&'static str> {
    if path.is_empty() {
        Some("the path is empty")
    } else if path.contains('\\') {
        Some("a relative path must use forward slashes")
    } else if path
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        Some("a relative path must stay inside its folder")
    } else {
        None
    }
}

/// One row of the channel rack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Channel {
    pub id: ChannelId,
    pub name: String,
    /// Display color as 0xRRGGBB.
    pub color: u32,
    /// Linear gain, 0 to [`MAX_GAIN`].
    pub volume: f32,
    /// -1 is hard left, 1 is hard right.
    pub pan: f32,
    pub muted: bool,
    pub solo: bool,
    /// The mixer track this channel plays into.
    pub mixer_track: TrackId,
    pub source: ChannelSource,
}

/// What makes the sound of a channel. Synths and hosted plugins are added as
/// new variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum ChannelSource {
    Sampler(SamplerSettings),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SamplerSettings {
    pub sample: Option<SampleId>,
    /// The key that plays the sample at its recorded pitch.
    pub root_key: u8,
    /// Tuning offset in semitones, -48 to 48. Fractions are fine tuning.
    pub tune: f32,
    /// Extra linear gain applied to the sample, 0 to [`MAX_GAIN`].
    pub gain: f32,
    /// Playback start as a fraction of the sample, 0 to 1.
    pub start: f32,
    /// Playback end as a fraction of the sample, 0 to 1, greater than `start`.
    pub end: f32,
    pub reverse: bool,
    /// With no envelope the sample plays to its end and ignores note length,
    /// which is what drum hits want. With one, the note length gates it.
    pub envelope: Option<Envelope>,
    /// A new note on this channel stops the notes already playing on it.
    pub cut_self: bool,
    /// Channels that share a nonzero cut group stop each other, such as an
    /// open and a closed hi-hat. 0 means no group.
    pub cut_group: u8,
}

impl Default for SamplerSettings {
    fn default() -> Self {
        Self {
            sample: None,
            root_key: DEFAULT_KEY,
            tune: 0.0,
            gain: 1.0,
            start: 0.0,
            end: 1.0,
            reverse: false,
            envelope: None,
            cut_self: false,
            cut_group: 0,
        }
    }
}

/// An attack, decay, sustain, release volume envelope. The three times run
/// from 0 to [`MAX_ENVELOPE_MS`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Envelope {
    pub attack_ms: f32,
    pub decay_ms: f32,
    /// Sustain level, 0 to 1.
    pub sustain: f32,
    pub release_ms: f32,
}

impl Default for Envelope {
    fn default() -> Self {
        Self {
            attack_ms: 1.0,
            decay_ms: 200.0,
            sustain: 1.0,
            release_ms: 50.0,
        }
    }
}

/// A short musical idea: notes for any number of channels.
///
/// The step sequencer and the piano roll edit the same notes. A step that is
/// on is a note at `step * TICKS_PER_STEP` with key [`DEFAULT_KEY`] and a
/// length of one step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Pattern {
    pub id: PatternId,
    pub name: String,
    /// Display color as 0xRRGGBB.
    pub color: u32,
    /// Length in sixteenth-note steps, 1 to 1024. The pattern loops at this
    /// length, and notes that start at or after it do not play.
    pub length_steps: u32,
    /// Notes per channel, sorted by channel id. Channels with no notes have
    /// no lane.
    pub lanes: Vec<Lane>,
}

impl Pattern {
    pub fn length_ticks(&self) -> u32 {
        self.length_steps * TICKS_PER_STEP
    }

    pub fn lane(&self, channel: ChannelId) -> Option<&Lane> {
        self.lanes.iter().find(|lane| lane.channel == channel)
    }
}

impl Note {
    /// The order notes keep inside a lane.
    pub fn sort_key(&self) -> (u32, u8, NoteId) {
        (self.start, self.key, self.id)
    }
}

/// The notes one channel plays in one pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Lane {
    pub channel: ChannelId,
    /// Sorted by `start`, then `key`, then `id`.
    pub notes: Vec<Note>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Note {
    pub id: NoteId,
    /// Start in ticks from the beginning of the pattern.
    pub start: u32,
    /// Length in ticks, at least 1.
    pub length: u32,
    /// MIDI key, 0 to 127.
    pub key: u8,
    /// 0 to 1.
    pub velocity: f32,
    /// -1 is hard left, 1 is hard right. Added to the channel pan.
    pub pan: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Mixer {
    /// `tracks[0]` is always the master. At most [`MAX_MIXER_TRACKS`].
    pub tracks: Vec<MixerTrack>,
}

impl Mixer {
    pub fn track(&self, id: TrackId) -> Option<&MixerTrack> {
        self.tracks.iter().find(|track| track.id == id)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MixerTrack {
    pub id: TrackId,
    pub name: String,
    /// Display color as 0xRRGGBB.
    pub color: u32,
    /// Fader as linear gain, 0 to [`MAX_GAIN`]. 1 is 0 dB.
    pub volume: f32,
    /// -1 is hard left, 1 is hard right.
    pub pan: f32,
    pub muted: bool,
    pub solo: bool,
    /// Where the track's output goes. `None` on the master, and on a track
    /// that only feeds its sends. Routing never forms a cycle.
    pub output: Option<TrackId>,
    /// Extra copies of the post-fader signal sent to other tracks.
    pub sends: Vec<Send>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Send {
    pub target: TrackId,
    /// Linear gain, 0 to [`MAX_GAIN`].
    pub gain: f32,
}

/// The song timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Playlist {
    /// Lanes of the timeline, in display order.
    pub tracks: Vec<PlaylistTrack>,
    /// Sorted by `start`, then `id`.
    pub clips: Vec<Clip>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlaylistTrack {
    pub id: PlaylistTrackId,
    pub name: String,
    pub muted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Clip {
    pub id: ClipId,
    pub track: PlaylistTrackId,
    /// Start on the timeline in ticks.
    pub start: u32,
    /// Length in ticks, at least 1.
    pub length: u32,
    /// How far into its content the clip starts, in ticks. A pattern clip
    /// loops its pattern, so this is taken modulo the pattern length.
    pub offset: u32,
    pub muted: bool,
    pub content: ClipContent,
}

impl Clip {
    /// The order clips keep on the playlist.
    pub fn sort_key(&self) -> (u32, ClipId) {
        (self.start, self.id)
    }
}

/// What a clip plays. Audio and automation clips are added as new variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum ClipContent {
    #[serde(rename_all = "camelCase")]
    Pattern { pattern: PatternId },
}
