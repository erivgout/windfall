use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::param::param_set;

/// Controls for [`super::Pluck`]. Decay is the nominal low-frequency RT60.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct PluckParams {
    /// Nominal decay, 0.1–12 seconds; default 2.
    pub decay_seconds: f32,
    /// Loop brightness, 0–1; default 0.65.
    pub brightness: f32,
    /// Excitation position along the string, 0.05–0.95; default 0.23.
    pub pick_position: f32,
    /// Output gain, 0–1; default 0.8.
    pub level: f32,
}

impl Default for PluckParams {
    fn default() -> Self {
        Self {
            decay_seconds: 2.0,
            brightness: 0.65,
            pick_position: 0.23,
            level: 0.8,
        }
    }
}

param_set!(PluckParams, "Pluck", {
    float [decay_seconds] "decaySeconds" "Decay" { Seconds, Logarithmic, 0.1, 12.0, 2.0 }
    float [brightness] "brightness" "Brightness" { Fraction, Linear, 0.0, 1.0, 0.65 }
    float [pick_position] "pickPosition" "Pick position" { Fraction, Linear, 0.05, 0.95, 0.23 }
    float [level] "level" "Level" { Gain, Linear, 0.0, 1.0, 0.8 }
});

/// Controls for [`super::FingerBass`], including two physical body peaks.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct FingerBassParams {
    /// Nominal decay, 0.1–8 seconds; default 1.8.
    pub decay_seconds: f32,
    /// String and body tone, 0–1; default 0.45.
    pub tone: f32,
    /// Extra loop loss, 0–1; default 0.25.
    pub damping: f32,
    /// Amount of the body bandpasses, 0–1; default 0.6.
    pub body: f32,
    /// Excitation position, 0.05–0.95; default 0.18.
    pub pick_position: f32,
    /// Output gain, 0–1; default 0.8.
    pub level: f32,
}

impl Default for FingerBassParams {
    fn default() -> Self {
        Self {
            decay_seconds: 1.8,
            tone: 0.45,
            damping: 0.25,
            body: 0.6,
            pick_position: 0.18,
            level: 0.8,
        }
    }
}

param_set!(FingerBassParams, "Finger Bass", {
    float [decay_seconds] "decaySeconds" "Decay" { Seconds, Logarithmic, 0.1, 8.0, 1.8 }
    float [tone] "tone" "Tone" { Fraction, Linear, 0.0, 1.0, 0.45 }
    float [damping] "damping" "Damping" { Fraction, Linear, 0.0, 1.0, 0.25 }
    float [body] "body" "Body" { Fraction, Linear, 0.0, 1.0, 0.6 }
    float [pick_position] "pickPosition" "Pick position" { Fraction, Linear, 0.05, 0.95, 0.18 }
    float [level] "level" "Level" { Gain, Linear, 0.0, 1.0, 0.8 }
});

/// Controls for [`super::AcousticString`], a dispersive dual-string model.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct AcousticStringParams {
    /// Nominal decay, 0.2–24 seconds; default 6.
    pub decay_seconds: f32,
    /// Loop brightness, 0–1; default 0.75.
    pub brightness: f32,
    /// First-order allpass dispersion, 0–1; default 0.3.
    pub stiffness: f32,
    /// Contribution of the second, three-cent-detuned string, 0–1; default 0.45.
    pub sympathetic: f32,
    /// Excitation position, 0.05–0.95; default 0.27.
    pub pick_position: f32,
    /// Output gain, 0–1; default 0.8.
    pub level: f32,
}

impl Default for AcousticStringParams {
    fn default() -> Self {
        Self {
            decay_seconds: 6.0,
            brightness: 0.75,
            stiffness: 0.3,
            sympathetic: 0.45,
            pick_position: 0.27,
            level: 0.8,
        }
    }
}

param_set!(AcousticStringParams, "Acoustic String", {
    float [decay_seconds] "decaySeconds" "Decay" { Seconds, Logarithmic, 0.2, 24.0, 6.0 }
    float [brightness] "brightness" "Brightness" { Fraction, Linear, 0.0, 1.0, 0.75 }
    float [stiffness] "stiffness" "String stiffness" { Fraction, Linear, 0.0, 1.0, 0.3 }
    float [sympathetic] "sympathetic" "Second string" { Fraction, Linear, 0.0, 1.0, 0.45 }
    float [pick_position] "pickPosition" "Pick position" { Fraction, Linear, 0.05, 0.95, 0.27 }
    float [level] "level" "Level" { Gain, Linear, 0.0, 1.0, 0.8 }
});
