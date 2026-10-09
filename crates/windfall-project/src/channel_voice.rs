//! Persistent channel voice settings; processors live in the engine.
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use windfall_dsp::EnvelopeParams;

fn finite(value: f32, low: f32, high: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(low, high)
    } else {
        fallback
    }
}
fn sample_rate(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(1.0, 384_000.0)
    } else {
        48_000.0
    }
}
fn tempo(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(1.0, 999.0)
    } else {
        120.0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ChannelNoteDivision")]
pub enum NoteDivision {
    Quarter,
    Eighth,
    #[default]
    Sixteenth,
    ThirtySecond,
}

impl NoteDivision {
    pub fn beats(self) -> f64 {
        match self {
            Self::Quarter => 1.0,
            Self::Eighth => 0.5,
            Self::Sixteenth => 0.25,
            Self::ThirtySecond => 0.125,
        }
    }
    pub fn frames(self, rate: f64, bpm: f64) -> f64 {
        (sample_rate(rate) * 60.0 / tempo(bpm) * self.beats()).max(1.0)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ArpeggiatorMode {
    #[default]
    Off,
    Up,
    Down,
    UpDown,
    AsPlayed,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ArpeggiatorSettings {
    pub mode: ArpeggiatorMode,
    pub rate: NoteDivision,
    pub gate: f32,
    pub range_octaves: u8,
}
impl Default for ArpeggiatorSettings {
    fn default() -> Self {
        Self {
            mode: ArpeggiatorMode::Off,
            rate: NoteDivision::Sixteenth,
            gate: 0.75,
            range_octaves: 1,
        }
    }
}
impl ArpeggiatorSettings {
    pub fn sanitized(self) -> Self {
        Self {
            gate: finite(self.gate, 0.0, 1.0, 0.75),
            range_octaves: self.range_octaves.clamp(1, 4),
            ..self
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "unit", rename_all = "camelCase")]
#[ts(export)]
pub enum EchoTime {
    Milliseconds { ms: f32 },
    Division { division: NoteDivision },
}
impl Default for EchoTime {
    fn default() -> Self {
        Self::Milliseconds { ms: 250.0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct NoteEchoSettings {
    pub enabled: bool,
    pub time: EchoTime,
    pub feedback: f32,
    pub pitch_semitones: i8,
    pub repeats: u8,
}
impl Default for NoteEchoSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            time: EchoTime::default(),
            feedback: 0.5,
            pitch_semitones: 0,
            repeats: 3,
        }
    }
}
impl NoteEchoSettings {
    pub fn sanitized(self) -> Self {
        Self {
            time: match self.time {
                EchoTime::Milliseconds { ms } => EchoTime::Milliseconds {
                    ms: finite(ms, 1.0, 60_000.0, 250.0),
                },
                time => time,
            },
            feedback: finite(self.feedback, 0.0, 0.95, 0.5),
            pitch_semitones: self.pitch_semitones.clamp(-48, 48),
            repeats: self.repeats.min(8),
            ..self
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct PolyphonySettings {
    pub max_voices: u8,
    pub mono_legato: bool,
    pub portamento_ms: f32,
}
impl Default for PolyphonySettings {
    fn default() -> Self {
        Self {
            max_voices: 32,
            mono_legato: false,
            portamento_ms: 0.0,
        }
    }
}
impl PolyphonySettings {
    pub fn sanitized(self) -> Self {
        Self {
            max_voices: self.max_voices.clamp(1, 32),
            portamento_ms: finite(self.portamento_ms, 0.0, 60_000.0, 0.0),
            ..self
        }
    }
    /// Add this fraction of `(target - current)` per sample. Time is the
    /// exponential time constant (63.2% covered); zero snaps to the target.
    pub fn glide_coefficient(self, rate: f64) -> f64 {
        let ms = f64::from(self.sanitized().portamento_ms);
        if ms == 0.0 {
            1.0
        } else {
            -(-1000.0 / (ms * sample_rate(rate))).exp_m1()
        }
    }
}

/// ADSR fields/defaults match synth::EnvelopeParams. Depth is filter octaves,
/// pitch cents, or normalized pan according to the enclosing field.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ModulationEnvelope {
    pub enabled: bool,
    pub attack_ms: f32,
    pub decay_ms: f32,
    pub sustain: f32,
    pub release_ms: f32,
    pub depth: f32,
}
impl Default for ModulationEnvelope {
    fn default() -> Self {
        let shape = EnvelopeParams::default();
        Self {
            enabled: false,
            attack_ms: shape.attack_ms,
            decay_ms: shape.decay_ms,
            sustain: shape.sustain,
            release_ms: shape.release_ms,
            depth: 0.0,
        }
    }
}
impl ModulationEnvelope {
    fn sanitized(self, max_depth: f32) -> Self {
        let defaults = Self::default();
        Self {
            attack_ms: finite(self.attack_ms, 0.0, 10_000.0, defaults.attack_ms),
            decay_ms: finite(self.decay_ms, 1.0, 10_000.0, defaults.decay_ms),
            sustain: finite(self.sustain, 0.0, 1.0, defaults.sustain),
            release_ms: finite(self.release_ms, 1.0, 10_000.0, defaults.release_ms),
            depth: finite(self.depth, -max_depth, max_depth, 0.0),
            ..self
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
#[ts(rename = "ChannelLfoShape")]
pub enum LfoShape {
    #[default]
    Sine,
    Triangle,
    Square,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum LfoTarget {
    #[default]
    Filter,
    Pitch,
    Pan,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ChannelLfoSettings {
    pub enabled: bool,
    pub shape: LfoShape,
    pub target: LfoTarget,
    pub rate_hz: f32,
    /// Normalized bipolar amount. Full depth is 8 octaves / 2400 cents / 1 pan.
    pub depth: f32,
}
impl Default for ChannelLfoSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            shape: LfoShape::Sine,
            target: LfoTarget::Filter,
            rate_hz: 5.0,
            depth: 0.0,
        }
    }
}
impl ChannelLfoSettings {
    pub fn sanitized(self) -> Self {
        Self {
            rate_hz: finite(self.rate_hz, 0.01, 30.0, 5.0),
            depth: finite(self.depth, -1.0, 1.0, 0.0),
            ..self
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ChannelEnvelopes {
    pub filter: ModulationEnvelope,
    pub pitch: ModulationEnvelope,
    pub pan: ModulationEnvelope,
    pub lfo: ChannelLfoSettings,
}
impl ChannelEnvelopes {
    pub fn sanitized(self) -> Self {
        Self {
            filter: self.filter.sanitized(8.0),
            pitch: self.pitch.sanitized(2400.0),
            pan: self.pan.sanitized(1.0),
            lfo: self.lfo.sanitized(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ChannelVoiceSettings {
    pub arpeggiator: ArpeggiatorSettings,
    pub echo: NoteEchoSettings,
    pub polyphony: PolyphonySettings,
    pub envelopes: ChannelEnvelopes,
}
impl ChannelVoiceSettings {
    pub fn sanitized(self) -> Self {
        Self {
            arpeggiator: self.arpeggiator.sanitized(),
            echo: self.echo.sanitized(),
            polyphony: self.polyphony.sanitized(),
            envelopes: self.envelopes.sanitized(),
        }
    }
}

pub(crate) fn deserialize_settings<'de, D: serde::Deserializer<'de>>(
    de: D,
) -> Result<ChannelVoiceSettings, D::Error> {
    ChannelVoiceSettings::deserialize(de).map(ChannelVoiceSettings::sanitized)
}
