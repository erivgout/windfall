//! What crosses between the host and a plugin with each block besides
//! audio: notes and parameter changes going in, the plugin's own changes
//! coming out, and where the song is.
//!
//! These are plain values with no pointers in them, the same for every
//! plugin format. That is what lets a plugin move into another process
//! later: they can be copied into shared memory as they are.

/// Something the host tells a plugin at a frame of the next block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HostEvent {
    /// Start a note. `velocity` runs from 0 to 1.
    NoteOn {
        time: u32,
        key: u8,
        channel: u8,
        velocity: f32,
    },
    /// Let go of a note, which starts its release.
    NoteOff {
        time: u32,
        key: u8,
        channel: u8,
        velocity: f32,
    },
    /// Stop every note at once, without releases.
    AllNotesOff { time: u32 },
    /// Set a parameter, by the plugin's own id and in its own units.
    Param { time: u32, id: u32, value: f64 },
}

impl HostEvent {
    pub(crate) fn valid(&self) -> bool {
        match *self {
            Self::NoteOn {
                key,
                channel,
                velocity,
                ..
            }
            | Self::NoteOff {
                key,
                channel,
                velocity,
                ..
            } => {
                key <= 127
                    && channel <= 15
                    && velocity.is_finite()
                    && (0.0..=1.0).contains(&velocity)
            }
            Self::Param { id, value, .. } => id != u32::MAX && value.is_finite(),
            Self::AllNotesOff { .. } => true,
        }
    }
    /// The frame of the block the event takes effect on.
    pub const fn time(&self) -> u32 {
        match *self {
            Self::NoteOn { time, .. }
            | Self::NoteOff { time, .. }
            | Self::AllNotesOff { time }
            | Self::Param { time, .. } => time,
        }
    }

    pub(crate) const fn set_time(&mut self, new: u32) {
        match self {
            Self::NoteOn { time, .. }
            | Self::NoteOff { time, .. }
            | Self::AllNotesOff { time }
            | Self::Param { time, .. } => *time = new,
        }
    }
}

/// Something a plugin tells the host while it processes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PluginEvent {
    /// The user is about to move this parameter in the plugin's editor.
    /// Everything up to the matching [`PluginEvent::GestureEnd`] is one edit.
    GestureBegin { id: u32 },
    /// The plugin changed a parameter itself, in its own units.
    ParamValue { id: u32, value: f64 },
    /// The user let go of the parameter.
    GestureEnd { id: u32 },
    /// A note has finished sounding, release included.
    NoteEnd { key: u8, channel: u8 },
}

/// Where the song is at the first frame of a block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transport {
    pub playing: bool,
    pub tempo_bpm: f64,
    /// Position in quarter notes from the start of the song.
    pub position_beats: f64,
    /// Position in seconds from the start of the song.
    pub position_seconds: f64,
    /// The time signature, as in 4/4.
    pub numerator: u16,
    pub denominator: u16,
}

impl Default for Transport {
    fn default() -> Self {
        Self {
            playing: false,
            tempo_bpm: 120.0,
            position_beats: 0.0,
            position_seconds: 0.0,
            numerator: 4,
            denominator: 4,
        }
    }
}

impl Transport {
    /// Moves the position on by `frames` at `sample_rate`, if the song is
    /// playing. The processor does this after every block, so a host that
    /// sets the position once gets a running clock.
    pub(crate) fn advance(&mut self, frames: usize, sample_rate: f64) {
        if self.playing && sample_rate > 0.0 {
            let seconds = frames as f64 / sample_rate;
            self.position_seconds += seconds;
            self.position_beats += seconds * self.tempo_bpm / 60.0;
        }
    }
}
