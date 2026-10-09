use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::param::param_set;

/// Maximum number of codes in a phrase, including silence.
pub const MAX_PHONEMES: usize = 32;

/// Synthetic articulations; these codes do not represent natural-language text.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[repr(u8)]
pub enum Phoneme {
    #[default]
    Silence,
    Ah,
    Ee,
    Oo,
    Eh,
    Oh,
    NoiseBurst,
    Nasal,
}

/// Inline phrase storage. Only the first `length` codes play; zero is empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct PhonemeBuffer {
    pub codes: [Phoneme; MAX_PHONEMES],
    pub length: u8,
}

impl PhonemeBuffer {
    /// Copies at most 32 codes, without allocating.
    pub fn from_slice(codes: &[Phoneme]) -> Self {
        let length = codes.len().min(MAX_PHONEMES);
        let mut phrase = Self {
            codes: [Phoneme::Silence; MAX_PHONEMES],
            length: length as u8,
        };
        phrase.codes[..length].copy_from_slice(&codes[..length]);
        phrase
    }
}

impl Default for PhonemeBuffer {
    fn default() -> Self {
        Self::from_slice(&[
            Phoneme::Ah,
            Phoneme::Ee,
            Phoneme::Oo,
            Phoneme::Eh,
            Phoneme::Oh,
        ])
    }
}

/// Controls for Speech Voice. All storage is fixed and can be copied to audio.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct SpeechVoiceParams {
    pub phrase: PhonemeBuffer,
    /// Phonemes per second, 0.5–30; default 6.
    pub rate: f32,
    /// Transposition relative to the note key, -24–24 semitones; default 0.
    pub pitch: f32,
    /// Release duration, 5–2000 milliseconds; default 80.
    pub release_ms: f32,
    /// Output gain, 0–1; default 0.8.
    pub level: f32,
}

impl Default for SpeechVoiceParams {
    fn default() -> Self {
        Self {
            phrase: PhonemeBuffer::default(),
            rate: 6.0,
            pitch: 0.0,
            release_ms: 80.0,
            level: 0.8,
        }
    }
}

macro_rules! speech_params {
    ($( $slot:tt $id:literal $default:ident ),+ $(,)?) => {
        param_set!(SpeechVoiceParams, "Speech Voice", {
            float [rate] "rate" "Rate" { Hertz, Logarithmic, 0.5, 30.0, 6.0 }
            float [pitch] "pitch" "Pitch" { Semitones, Linear, -24.0, 24.0, 0.0 }
            float [release_ms] "releaseMs" "Release" { Milliseconds, Logarithmic, 5.0, 2000.0, 80.0 }
            float [level] "level" "Level" { Gain, Linear, 0.0, 1.0, 0.8 }
            int [phrase.length] "phrase.length" "Phrase length" { None, 0, 32, 5 }
            $(choice [phrase.codes[$slot]] $id "Phoneme" {
                Phoneme, $default, [
                    Silence "silence" "Silence", Ah "ah" "Ah", Ee "ee" "Ee",
                    Oo "oo" "Oo", Eh "eh" "Eh", Oh "oh" "Oh",
                    NoiseBurst "noiseBurst" "Noise burst", Nasal "nasal" "Nasal"
                ]
            })+
        });
    };
}

speech_params!(
    0 "phrase.codes.0" Ah, 1 "phrase.codes.1" Ee, 2 "phrase.codes.2" Oo,
    3 "phrase.codes.3" Eh, 4 "phrase.codes.4" Oh, 5 "phrase.codes.5" Silence,
    6 "phrase.codes.6" Silence, 7 "phrase.codes.7" Silence, 8 "phrase.codes.8" Silence,
    9 "phrase.codes.9" Silence, 10 "phrase.codes.10" Silence, 11 "phrase.codes.11" Silence,
    12 "phrase.codes.12" Silence, 13 "phrase.codes.13" Silence, 14 "phrase.codes.14" Silence,
    15 "phrase.codes.15" Silence, 16 "phrase.codes.16" Silence, 17 "phrase.codes.17" Silence,
    18 "phrase.codes.18" Silence, 19 "phrase.codes.19" Silence, 20 "phrase.codes.20" Silence,
    21 "phrase.codes.21" Silence, 22 "phrase.codes.22" Silence, 23 "phrase.codes.23" Silence,
    24 "phrase.codes.24" Silence, 25 "phrase.codes.25" Silence, 26 "phrase.codes.26" Silence,
    27 "phrase.codes.27" Silence, 28 "phrase.codes.28" Silence, 29 "phrase.codes.29" Silence,
    30 "phrase.codes.30" Silence, 31 "phrase.codes.31" Silence,
);
