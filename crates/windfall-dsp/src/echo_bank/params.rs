use crate::NoteDivision;
use crate::param::param_set;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Filter order and response are independent of delay routing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum EchoFilterMode {
    #[default]
    Off,
    Lowpass,
    Bandpass,
    Notch,
    Highpass,
    LowShelf,
    Peak,
    HighShelf,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum EchoFeedbackMode {
    Off,
    #[default]
    Normal,
    Inverted,
    PingPong,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct EchoFilterParams {
    pub mode: EchoFilterMode,
    pub frequency_hz: f32,
    pub q: f32,
    pub gain_db: f32,
    pub gain: f32,
    pub sections: u8,
}
impl Default for EchoFilterParams {
    fn default() -> Self {
        Self {
            mode: EchoFilterMode::Off,
            frequency_hz: 1000.0,
            q: std::f32::consts::FRAC_1_SQRT_2,
            gain_db: 0.0,
            gain: 1.0,
            sections: 1,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct EchoUnitParams {
    pub enabled: bool,
    pub input_gain: f32,
    pub input_pan: f32,
    pub sync: bool,
    pub time_ms: f32,
    pub division: NoteDivision,
    pub stereo_offset_ms: f32,
    pub feedback: f32,
    pub feedback_mode: EchoFeedbackMode,
    pub feedback_pan: f32,
    pub separation: f32,
    pub filter: EchoFilterParams,
    pub filter_post: bool,
    pub feedback_filter: EchoFilterParams,
    pub output_gain: f32,
    pub output_pan: f32,
    /// Signed send to the following unit; ignored and cleared for unit 7.
    pub next_send: f32,
}
impl Default for EchoUnitParams {
    fn default() -> Self {
        Self {
            enabled: true,
            input_gain: 1.0,
            input_pan: 0.0,
            sync: true,
            time_ms: 250.0,
            division: NoteDivision::Eighth,
            stereo_offset_ms: 0.0,
            feedback: 0.35,
            feedback_mode: EchoFeedbackMode::Normal,
            feedback_pan: 0.0,
            separation: 1.0,
            filter: EchoFilterParams::default(),
            filter_post: false,
            feedback_filter: EchoFilterParams::default(),
            output_gain: 1.0,
            output_pan: 0.0,
            next_send: 0.0,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct EchoBankParams {
    pub dry: f32,
    pub wet: f32,
    pub input_gain: f32,
    pub feedback: f32,
    pub units: [EchoUnitParams; 8],
}
impl Default for EchoBankParams {
    fn default() -> Self {
        let mut units = [EchoUnitParams::default(); 8];
        for u in &mut units[1..] {
            u.enabled = false;
        }
        Self {
            dry: 1.0,
            wet: 0.3,
            input_gain: 1.0,
            feedback: 1.0,
            units,
        }
    }
}
// Stable indices: global 0..4, then each unit in ascending order.
param_set!(EchoBankParams, "Echo bank", {
    float [dry] "dry" "Dry" { Gain, Linear, 0.0, 1.0, 1.0 }
    float [wet] "wet" "Wet" { Gain, Linear, 0.0, 1.0, 0.3 }
    float [input_gain] "inputGain" "Input" { Gain, Linear, 0.0, 2.0, 1.0 }
    float [feedback] "feedback" "Global feedback" { Fraction, Linear, 0.0, 1.0, 1.0 }
    toggle [units[0].enabled] "units.0.enabled" "Unit 1 enabled" { true }
    float [units[0].input_gain] "units.0.inputGain" "Unit 1 input" { Gain, Linear, 0.0, 2.0, 1.0 }
    float [units[0].input_pan] "units.0.inputPan" "Unit 1 input pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [units[0].sync] "units.0.sync" "Unit 1 sync" { true }
    float [units[0].time_ms] "units.0.timeMs" "Unit 1 time" { Milliseconds, Logarithmic, 1.0, 24_000.0, 250.0 }
    choice [units[0].division] "units.0.division" "Unit 1 division" { NoteDivision, Eighth, [Whole "whole" "1/1", HalfDotted "halfDotted" "1/2 dotted", Half "half" "1/2", HalfTriplet "halfTriplet" "1/2 triplet", QuarterDotted "quarterDotted" "1/4 dotted", Quarter "quarter" "1/4", QuarterTriplet "quarterTriplet" "1/4 triplet", EighthDotted "eighthDotted" "1/8 dotted", Eighth "eighth" "1/8", EighthTriplet "eighthTriplet" "1/8 triplet", SixteenthDotted "sixteenthDotted" "1/16 dotted", Sixteenth "sixteenth" "1/16", SixteenthTriplet "sixteenthTriplet" "1/16 triplet", ThirtySecond "thirtySecond" "1/32"] }
    float [units[0].stereo_offset_ms] "units.0.stereoOffsetMs" "Unit 1 offset" { Milliseconds, Linear, -1000.0, 1000.0, 0.0 }
    float [units[0].feedback] "units.0.feedback" "Unit 1 feedback" { Fraction, Linear, 0.0, 1.0, 0.35 }
    choice [units[0].feedback_mode] "units.0.feedbackMode" "Unit 1 feedback mode" { EchoFeedbackMode, Normal, [Off "off" "Off", Normal "normal" "Normal", Inverted "inverted" "Inverted", PingPong "pingPong" "Ping-pong"] }
    float [units[0].feedback_pan] "units.0.feedbackPan" "Unit 1 feedback pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    float [units[0].separation] "units.0.separation" "Unit 1 separation" { Fraction, Linear, 0.0, 1.0, 1.0 }
    choice [units[0].filter.mode] "units.0.filter.mode" "Unit 1 filter mode" { EchoFilterMode, Off, [Off "off" "Off", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Notch "notch" "Notch", Highpass "highpass" "Highpass", LowShelf "lowShelf" "Low shelf", Peak "peak" "Peak", HighShelf "highShelf" "High shelf"] }
    float [units[0].filter.frequency_hz] "units.0.filter.frequencyHz" "Unit 1 filter frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1000.0 }
    float [units[0].filter.q] "units.0.filter.q" "Unit 1 filter Q" { Ratio, Logarithmic, 0.5, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    float [units[0].filter.gain_db] "units.0.filter.gainDb" "Unit 1 filter shelf/peak gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [units[0].filter.gain] "units.0.filter.gain" "Unit 1 filter gain" { Gain, Linear, 0.1, 2.0, 1.0 }
    int [units[0].filter.sections] "units.0.filter.sections" "Unit 1 filter sections" { None, 1, 3, 1 }
    choice [units[0].feedback_filter.mode] "units.0.feedbackFilter.mode" "Unit 1 feedback filter mode" { EchoFilterMode, Off, [Off "off" "Off", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Notch "notch" "Notch", Highpass "highpass" "Highpass", LowShelf "lowShelf" "Low shelf", Peak "peak" "Peak", HighShelf "highShelf" "High shelf"] }
    float [units[0].feedback_filter.frequency_hz] "units.0.feedbackFilter.frequencyHz" "Unit 1 feedback filter frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1000.0 }
    float [units[0].feedback_filter.q] "units.0.feedbackFilter.q" "Unit 1 feedback filter Q" { Ratio, Logarithmic, 0.5, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    float [units[0].feedback_filter.gain_db] "units.0.feedbackFilter.gainDb" "Unit 1 feedback filter shelf/peak gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [units[0].feedback_filter.gain] "units.0.feedbackFilter.gain" "Unit 1 feedback filter gain" { Gain, Linear, 0.1, 2.0, 1.0 }
    int [units[0].feedback_filter.sections] "units.0.feedbackFilter.sections" "Unit 1 feedback filter sections" { None, 1, 3, 1 }
    toggle [units[0].filter_post] "units.0.filterPost" "Unit 1 post filter" { false }
    float [units[0].output_gain] "units.0.outputGain" "Unit 1 output" { None, Linear, -1.0, 1.0, 1.0 }
    float [units[0].output_pan] "units.0.outputPan" "Unit 1 output pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    float [units[0].next_send] "units.0.nextSend" "Unit 1 next send" { None, Linear, -1.0, 1.0, 0.0 }
    toggle [units[1].enabled] "units.1.enabled" "Unit 2 enabled" { false }
    float [units[1].input_gain] "units.1.inputGain" "Unit 2 input" { Gain, Linear, 0.0, 2.0, 1.0 }
    float [units[1].input_pan] "units.1.inputPan" "Unit 2 input pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [units[1].sync] "units.1.sync" "Unit 2 sync" { true }
    float [units[1].time_ms] "units.1.timeMs" "Unit 2 time" { Milliseconds, Logarithmic, 1.0, 24_000.0, 250.0 }
    choice [units[1].division] "units.1.division" "Unit 2 division" { NoteDivision, Eighth, [Whole "whole" "1/1", HalfDotted "halfDotted" "1/2 dotted", Half "half" "1/2", HalfTriplet "halfTriplet" "1/2 triplet", QuarterDotted "quarterDotted" "1/4 dotted", Quarter "quarter" "1/4", QuarterTriplet "quarterTriplet" "1/4 triplet", EighthDotted "eighthDotted" "1/8 dotted", Eighth "eighth" "1/8", EighthTriplet "eighthTriplet" "1/8 triplet", SixteenthDotted "sixteenthDotted" "1/16 dotted", Sixteenth "sixteenth" "1/16", SixteenthTriplet "sixteenthTriplet" "1/16 triplet", ThirtySecond "thirtySecond" "1/32"] }
    float [units[1].stereo_offset_ms] "units.1.stereoOffsetMs" "Unit 2 offset" { Milliseconds, Linear, -1000.0, 1000.0, 0.0 }
    float [units[1].feedback] "units.1.feedback" "Unit 2 feedback" { Fraction, Linear, 0.0, 1.0, 0.35 }
    choice [units[1].feedback_mode] "units.1.feedbackMode" "Unit 2 feedback mode" { EchoFeedbackMode, Normal, [Off "off" "Off", Normal "normal" "Normal", Inverted "inverted" "Inverted", PingPong "pingPong" "Ping-pong"] }
    float [units[1].feedback_pan] "units.1.feedbackPan" "Unit 2 feedback pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    float [units[1].separation] "units.1.separation" "Unit 2 separation" { Fraction, Linear, 0.0, 1.0, 1.0 }
    choice [units[1].filter.mode] "units.1.filter.mode" "Unit 2 filter mode" { EchoFilterMode, Off, [Off "off" "Off", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Notch "notch" "Notch", Highpass "highpass" "Highpass", LowShelf "lowShelf" "Low shelf", Peak "peak" "Peak", HighShelf "highShelf" "High shelf"] }
    float [units[1].filter.frequency_hz] "units.1.filter.frequencyHz" "Unit 2 filter frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1000.0 }
    float [units[1].filter.q] "units.1.filter.q" "Unit 2 filter Q" { Ratio, Logarithmic, 0.5, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    float [units[1].filter.gain_db] "units.1.filter.gainDb" "Unit 2 filter shelf/peak gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [units[1].filter.gain] "units.1.filter.gain" "Unit 2 filter gain" { Gain, Linear, 0.1, 2.0, 1.0 }
    int [units[1].filter.sections] "units.1.filter.sections" "Unit 2 filter sections" { None, 1, 3, 1 }
    choice [units[1].feedback_filter.mode] "units.1.feedbackFilter.mode" "Unit 2 feedback filter mode" { EchoFilterMode, Off, [Off "off" "Off", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Notch "notch" "Notch", Highpass "highpass" "Highpass", LowShelf "lowShelf" "Low shelf", Peak "peak" "Peak", HighShelf "highShelf" "High shelf"] }
    float [units[1].feedback_filter.frequency_hz] "units.1.feedbackFilter.frequencyHz" "Unit 2 feedback filter frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1000.0 }
    float [units[1].feedback_filter.q] "units.1.feedbackFilter.q" "Unit 2 feedback filter Q" { Ratio, Logarithmic, 0.5, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    float [units[1].feedback_filter.gain_db] "units.1.feedbackFilter.gainDb" "Unit 2 feedback filter shelf/peak gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [units[1].feedback_filter.gain] "units.1.feedbackFilter.gain" "Unit 2 feedback filter gain" { Gain, Linear, 0.1, 2.0, 1.0 }
    int [units[1].feedback_filter.sections] "units.1.feedbackFilter.sections" "Unit 2 feedback filter sections" { None, 1, 3, 1 }
    toggle [units[1].filter_post] "units.1.filterPost" "Unit 2 post filter" { false }
    float [units[1].output_gain] "units.1.outputGain" "Unit 2 output" { None, Linear, -1.0, 1.0, 1.0 }
    float [units[1].output_pan] "units.1.outputPan" "Unit 2 output pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    float [units[1].next_send] "units.1.nextSend" "Unit 2 next send" { None, Linear, -1.0, 1.0, 0.0 }
    toggle [units[2].enabled] "units.2.enabled" "Unit 3 enabled" { false }
    float [units[2].input_gain] "units.2.inputGain" "Unit 3 input" { Gain, Linear, 0.0, 2.0, 1.0 }
    float [units[2].input_pan] "units.2.inputPan" "Unit 3 input pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [units[2].sync] "units.2.sync" "Unit 3 sync" { true }
    float [units[2].time_ms] "units.2.timeMs" "Unit 3 time" { Milliseconds, Logarithmic, 1.0, 24_000.0, 250.0 }
    choice [units[2].division] "units.2.division" "Unit 3 division" { NoteDivision, Eighth, [Whole "whole" "1/1", HalfDotted "halfDotted" "1/2 dotted", Half "half" "1/2", HalfTriplet "halfTriplet" "1/2 triplet", QuarterDotted "quarterDotted" "1/4 dotted", Quarter "quarter" "1/4", QuarterTriplet "quarterTriplet" "1/4 triplet", EighthDotted "eighthDotted" "1/8 dotted", Eighth "eighth" "1/8", EighthTriplet "eighthTriplet" "1/8 triplet", SixteenthDotted "sixteenthDotted" "1/16 dotted", Sixteenth "sixteenth" "1/16", SixteenthTriplet "sixteenthTriplet" "1/16 triplet", ThirtySecond "thirtySecond" "1/32"] }
    float [units[2].stereo_offset_ms] "units.2.stereoOffsetMs" "Unit 3 offset" { Milliseconds, Linear, -1000.0, 1000.0, 0.0 }
    float [units[2].feedback] "units.2.feedback" "Unit 3 feedback" { Fraction, Linear, 0.0, 1.0, 0.35 }
    choice [units[2].feedback_mode] "units.2.feedbackMode" "Unit 3 feedback mode" { EchoFeedbackMode, Normal, [Off "off" "Off", Normal "normal" "Normal", Inverted "inverted" "Inverted", PingPong "pingPong" "Ping-pong"] }
    float [units[2].feedback_pan] "units.2.feedbackPan" "Unit 3 feedback pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    float [units[2].separation] "units.2.separation" "Unit 3 separation" { Fraction, Linear, 0.0, 1.0, 1.0 }
    choice [units[2].filter.mode] "units.2.filter.mode" "Unit 3 filter mode" { EchoFilterMode, Off, [Off "off" "Off", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Notch "notch" "Notch", Highpass "highpass" "Highpass", LowShelf "lowShelf" "Low shelf", Peak "peak" "Peak", HighShelf "highShelf" "High shelf"] }
    float [units[2].filter.frequency_hz] "units.2.filter.frequencyHz" "Unit 3 filter frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1000.0 }
    float [units[2].filter.q] "units.2.filter.q" "Unit 3 filter Q" { Ratio, Logarithmic, 0.5, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    float [units[2].filter.gain_db] "units.2.filter.gainDb" "Unit 3 filter shelf/peak gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [units[2].filter.gain] "units.2.filter.gain" "Unit 3 filter gain" { Gain, Linear, 0.1, 2.0, 1.0 }
    int [units[2].filter.sections] "units.2.filter.sections" "Unit 3 filter sections" { None, 1, 3, 1 }
    choice [units[2].feedback_filter.mode] "units.2.feedbackFilter.mode" "Unit 3 feedback filter mode" { EchoFilterMode, Off, [Off "off" "Off", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Notch "notch" "Notch", Highpass "highpass" "Highpass", LowShelf "lowShelf" "Low shelf", Peak "peak" "Peak", HighShelf "highShelf" "High shelf"] }
    float [units[2].feedback_filter.frequency_hz] "units.2.feedbackFilter.frequencyHz" "Unit 3 feedback filter frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1000.0 }
    float [units[2].feedback_filter.q] "units.2.feedbackFilter.q" "Unit 3 feedback filter Q" { Ratio, Logarithmic, 0.5, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    float [units[2].feedback_filter.gain_db] "units.2.feedbackFilter.gainDb" "Unit 3 feedback filter shelf/peak gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [units[2].feedback_filter.gain] "units.2.feedbackFilter.gain" "Unit 3 feedback filter gain" { Gain, Linear, 0.1, 2.0, 1.0 }
    int [units[2].feedback_filter.sections] "units.2.feedbackFilter.sections" "Unit 3 feedback filter sections" { None, 1, 3, 1 }
    toggle [units[2].filter_post] "units.2.filterPost" "Unit 3 post filter" { false }
    float [units[2].output_gain] "units.2.outputGain" "Unit 3 output" { None, Linear, -1.0, 1.0, 1.0 }
    float [units[2].output_pan] "units.2.outputPan" "Unit 3 output pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    float [units[2].next_send] "units.2.nextSend" "Unit 3 next send" { None, Linear, -1.0, 1.0, 0.0 }
    toggle [units[3].enabled] "units.3.enabled" "Unit 4 enabled" { false }
    float [units[3].input_gain] "units.3.inputGain" "Unit 4 input" { Gain, Linear, 0.0, 2.0, 1.0 }
    float [units[3].input_pan] "units.3.inputPan" "Unit 4 input pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [units[3].sync] "units.3.sync" "Unit 4 sync" { true }
    float [units[3].time_ms] "units.3.timeMs" "Unit 4 time" { Milliseconds, Logarithmic, 1.0, 24_000.0, 250.0 }
    choice [units[3].division] "units.3.division" "Unit 4 division" { NoteDivision, Eighth, [Whole "whole" "1/1", HalfDotted "halfDotted" "1/2 dotted", Half "half" "1/2", HalfTriplet "halfTriplet" "1/2 triplet", QuarterDotted "quarterDotted" "1/4 dotted", Quarter "quarter" "1/4", QuarterTriplet "quarterTriplet" "1/4 triplet", EighthDotted "eighthDotted" "1/8 dotted", Eighth "eighth" "1/8", EighthTriplet "eighthTriplet" "1/8 triplet", SixteenthDotted "sixteenthDotted" "1/16 dotted", Sixteenth "sixteenth" "1/16", SixteenthTriplet "sixteenthTriplet" "1/16 triplet", ThirtySecond "thirtySecond" "1/32"] }
    float [units[3].stereo_offset_ms] "units.3.stereoOffsetMs" "Unit 4 offset" { Milliseconds, Linear, -1000.0, 1000.0, 0.0 }
    float [units[3].feedback] "units.3.feedback" "Unit 4 feedback" { Fraction, Linear, 0.0, 1.0, 0.35 }
    choice [units[3].feedback_mode] "units.3.feedbackMode" "Unit 4 feedback mode" { EchoFeedbackMode, Normal, [Off "off" "Off", Normal "normal" "Normal", Inverted "inverted" "Inverted", PingPong "pingPong" "Ping-pong"] }
    float [units[3].feedback_pan] "units.3.feedbackPan" "Unit 4 feedback pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    float [units[3].separation] "units.3.separation" "Unit 4 separation" { Fraction, Linear, 0.0, 1.0, 1.0 }
    choice [units[3].filter.mode] "units.3.filter.mode" "Unit 4 filter mode" { EchoFilterMode, Off, [Off "off" "Off", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Notch "notch" "Notch", Highpass "highpass" "Highpass", LowShelf "lowShelf" "Low shelf", Peak "peak" "Peak", HighShelf "highShelf" "High shelf"] }
    float [units[3].filter.frequency_hz] "units.3.filter.frequencyHz" "Unit 4 filter frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1000.0 }
    float [units[3].filter.q] "units.3.filter.q" "Unit 4 filter Q" { Ratio, Logarithmic, 0.5, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    float [units[3].filter.gain_db] "units.3.filter.gainDb" "Unit 4 filter shelf/peak gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [units[3].filter.gain] "units.3.filter.gain" "Unit 4 filter gain" { Gain, Linear, 0.1, 2.0, 1.0 }
    int [units[3].filter.sections] "units.3.filter.sections" "Unit 4 filter sections" { None, 1, 3, 1 }
    choice [units[3].feedback_filter.mode] "units.3.feedbackFilter.mode" "Unit 4 feedback filter mode" { EchoFilterMode, Off, [Off "off" "Off", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Notch "notch" "Notch", Highpass "highpass" "Highpass", LowShelf "lowShelf" "Low shelf", Peak "peak" "Peak", HighShelf "highShelf" "High shelf"] }
    float [units[3].feedback_filter.frequency_hz] "units.3.feedbackFilter.frequencyHz" "Unit 4 feedback filter frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1000.0 }
    float [units[3].feedback_filter.q] "units.3.feedbackFilter.q" "Unit 4 feedback filter Q" { Ratio, Logarithmic, 0.5, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    float [units[3].feedback_filter.gain_db] "units.3.feedbackFilter.gainDb" "Unit 4 feedback filter shelf/peak gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [units[3].feedback_filter.gain] "units.3.feedbackFilter.gain" "Unit 4 feedback filter gain" { Gain, Linear, 0.1, 2.0, 1.0 }
    int [units[3].feedback_filter.sections] "units.3.feedbackFilter.sections" "Unit 4 feedback filter sections" { None, 1, 3, 1 }
    toggle [units[3].filter_post] "units.3.filterPost" "Unit 4 post filter" { false }
    float [units[3].output_gain] "units.3.outputGain" "Unit 4 output" { None, Linear, -1.0, 1.0, 1.0 }
    float [units[3].output_pan] "units.3.outputPan" "Unit 4 output pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    float [units[3].next_send] "units.3.nextSend" "Unit 4 next send" { None, Linear, -1.0, 1.0, 0.0 }
    toggle [units[4].enabled] "units.4.enabled" "Unit 5 enabled" { false }
    float [units[4].input_gain] "units.4.inputGain" "Unit 5 input" { Gain, Linear, 0.0, 2.0, 1.0 }
    float [units[4].input_pan] "units.4.inputPan" "Unit 5 input pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [units[4].sync] "units.4.sync" "Unit 5 sync" { true }
    float [units[4].time_ms] "units.4.timeMs" "Unit 5 time" { Milliseconds, Logarithmic, 1.0, 24_000.0, 250.0 }
    choice [units[4].division] "units.4.division" "Unit 5 division" { NoteDivision, Eighth, [Whole "whole" "1/1", HalfDotted "halfDotted" "1/2 dotted", Half "half" "1/2", HalfTriplet "halfTriplet" "1/2 triplet", QuarterDotted "quarterDotted" "1/4 dotted", Quarter "quarter" "1/4", QuarterTriplet "quarterTriplet" "1/4 triplet", EighthDotted "eighthDotted" "1/8 dotted", Eighth "eighth" "1/8", EighthTriplet "eighthTriplet" "1/8 triplet", SixteenthDotted "sixteenthDotted" "1/16 dotted", Sixteenth "sixteenth" "1/16", SixteenthTriplet "sixteenthTriplet" "1/16 triplet", ThirtySecond "thirtySecond" "1/32"] }
    float [units[4].stereo_offset_ms] "units.4.stereoOffsetMs" "Unit 5 offset" { Milliseconds, Linear, -1000.0, 1000.0, 0.0 }
    float [units[4].feedback] "units.4.feedback" "Unit 5 feedback" { Fraction, Linear, 0.0, 1.0, 0.35 }
    choice [units[4].feedback_mode] "units.4.feedbackMode" "Unit 5 feedback mode" { EchoFeedbackMode, Normal, [Off "off" "Off", Normal "normal" "Normal", Inverted "inverted" "Inverted", PingPong "pingPong" "Ping-pong"] }
    float [units[4].feedback_pan] "units.4.feedbackPan" "Unit 5 feedback pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    float [units[4].separation] "units.4.separation" "Unit 5 separation" { Fraction, Linear, 0.0, 1.0, 1.0 }
    choice [units[4].filter.mode] "units.4.filter.mode" "Unit 5 filter mode" { EchoFilterMode, Off, [Off "off" "Off", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Notch "notch" "Notch", Highpass "highpass" "Highpass", LowShelf "lowShelf" "Low shelf", Peak "peak" "Peak", HighShelf "highShelf" "High shelf"] }
    float [units[4].filter.frequency_hz] "units.4.filter.frequencyHz" "Unit 5 filter frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1000.0 }
    float [units[4].filter.q] "units.4.filter.q" "Unit 5 filter Q" { Ratio, Logarithmic, 0.5, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    float [units[4].filter.gain_db] "units.4.filter.gainDb" "Unit 5 filter shelf/peak gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [units[4].filter.gain] "units.4.filter.gain" "Unit 5 filter gain" { Gain, Linear, 0.1, 2.0, 1.0 }
    int [units[4].filter.sections] "units.4.filter.sections" "Unit 5 filter sections" { None, 1, 3, 1 }
    choice [units[4].feedback_filter.mode] "units.4.feedbackFilter.mode" "Unit 5 feedback filter mode" { EchoFilterMode, Off, [Off "off" "Off", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Notch "notch" "Notch", Highpass "highpass" "Highpass", LowShelf "lowShelf" "Low shelf", Peak "peak" "Peak", HighShelf "highShelf" "High shelf"] }
    float [units[4].feedback_filter.frequency_hz] "units.4.feedbackFilter.frequencyHz" "Unit 5 feedback filter frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1000.0 }
    float [units[4].feedback_filter.q] "units.4.feedbackFilter.q" "Unit 5 feedback filter Q" { Ratio, Logarithmic, 0.5, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    float [units[4].feedback_filter.gain_db] "units.4.feedbackFilter.gainDb" "Unit 5 feedback filter shelf/peak gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [units[4].feedback_filter.gain] "units.4.feedbackFilter.gain" "Unit 5 feedback filter gain" { Gain, Linear, 0.1, 2.0, 1.0 }
    int [units[4].feedback_filter.sections] "units.4.feedbackFilter.sections" "Unit 5 feedback filter sections" { None, 1, 3, 1 }
    toggle [units[4].filter_post] "units.4.filterPost" "Unit 5 post filter" { false }
    float [units[4].output_gain] "units.4.outputGain" "Unit 5 output" { None, Linear, -1.0, 1.0, 1.0 }
    float [units[4].output_pan] "units.4.outputPan" "Unit 5 output pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    float [units[4].next_send] "units.4.nextSend" "Unit 5 next send" { None, Linear, -1.0, 1.0, 0.0 }
    toggle [units[5].enabled] "units.5.enabled" "Unit 6 enabled" { false }
    float [units[5].input_gain] "units.5.inputGain" "Unit 6 input" { Gain, Linear, 0.0, 2.0, 1.0 }
    float [units[5].input_pan] "units.5.inputPan" "Unit 6 input pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [units[5].sync] "units.5.sync" "Unit 6 sync" { true }
    float [units[5].time_ms] "units.5.timeMs" "Unit 6 time" { Milliseconds, Logarithmic, 1.0, 24_000.0, 250.0 }
    choice [units[5].division] "units.5.division" "Unit 6 division" { NoteDivision, Eighth, [Whole "whole" "1/1", HalfDotted "halfDotted" "1/2 dotted", Half "half" "1/2", HalfTriplet "halfTriplet" "1/2 triplet", QuarterDotted "quarterDotted" "1/4 dotted", Quarter "quarter" "1/4", QuarterTriplet "quarterTriplet" "1/4 triplet", EighthDotted "eighthDotted" "1/8 dotted", Eighth "eighth" "1/8", EighthTriplet "eighthTriplet" "1/8 triplet", SixteenthDotted "sixteenthDotted" "1/16 dotted", Sixteenth "sixteenth" "1/16", SixteenthTriplet "sixteenthTriplet" "1/16 triplet", ThirtySecond "thirtySecond" "1/32"] }
    float [units[5].stereo_offset_ms] "units.5.stereoOffsetMs" "Unit 6 offset" { Milliseconds, Linear, -1000.0, 1000.0, 0.0 }
    float [units[5].feedback] "units.5.feedback" "Unit 6 feedback" { Fraction, Linear, 0.0, 1.0, 0.35 }
    choice [units[5].feedback_mode] "units.5.feedbackMode" "Unit 6 feedback mode" { EchoFeedbackMode, Normal, [Off "off" "Off", Normal "normal" "Normal", Inverted "inverted" "Inverted", PingPong "pingPong" "Ping-pong"] }
    float [units[5].feedback_pan] "units.5.feedbackPan" "Unit 6 feedback pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    float [units[5].separation] "units.5.separation" "Unit 6 separation" { Fraction, Linear, 0.0, 1.0, 1.0 }
    choice [units[5].filter.mode] "units.5.filter.mode" "Unit 6 filter mode" { EchoFilterMode, Off, [Off "off" "Off", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Notch "notch" "Notch", Highpass "highpass" "Highpass", LowShelf "lowShelf" "Low shelf", Peak "peak" "Peak", HighShelf "highShelf" "High shelf"] }
    float [units[5].filter.frequency_hz] "units.5.filter.frequencyHz" "Unit 6 filter frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1000.0 }
    float [units[5].filter.q] "units.5.filter.q" "Unit 6 filter Q" { Ratio, Logarithmic, 0.5, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    float [units[5].filter.gain_db] "units.5.filter.gainDb" "Unit 6 filter shelf/peak gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [units[5].filter.gain] "units.5.filter.gain" "Unit 6 filter gain" { Gain, Linear, 0.1, 2.0, 1.0 }
    int [units[5].filter.sections] "units.5.filter.sections" "Unit 6 filter sections" { None, 1, 3, 1 }
    choice [units[5].feedback_filter.mode] "units.5.feedbackFilter.mode" "Unit 6 feedback filter mode" { EchoFilterMode, Off, [Off "off" "Off", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Notch "notch" "Notch", Highpass "highpass" "Highpass", LowShelf "lowShelf" "Low shelf", Peak "peak" "Peak", HighShelf "highShelf" "High shelf"] }
    float [units[5].feedback_filter.frequency_hz] "units.5.feedbackFilter.frequencyHz" "Unit 6 feedback filter frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1000.0 }
    float [units[5].feedback_filter.q] "units.5.feedbackFilter.q" "Unit 6 feedback filter Q" { Ratio, Logarithmic, 0.5, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    float [units[5].feedback_filter.gain_db] "units.5.feedbackFilter.gainDb" "Unit 6 feedback filter shelf/peak gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [units[5].feedback_filter.gain] "units.5.feedbackFilter.gain" "Unit 6 feedback filter gain" { Gain, Linear, 0.1, 2.0, 1.0 }
    int [units[5].feedback_filter.sections] "units.5.feedbackFilter.sections" "Unit 6 feedback filter sections" { None, 1, 3, 1 }
    toggle [units[5].filter_post] "units.5.filterPost" "Unit 6 post filter" { false }
    float [units[5].output_gain] "units.5.outputGain" "Unit 6 output" { None, Linear, -1.0, 1.0, 1.0 }
    float [units[5].output_pan] "units.5.outputPan" "Unit 6 output pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    float [units[5].next_send] "units.5.nextSend" "Unit 6 next send" { None, Linear, -1.0, 1.0, 0.0 }
    toggle [units[6].enabled] "units.6.enabled" "Unit 7 enabled" { false }
    float [units[6].input_gain] "units.6.inputGain" "Unit 7 input" { Gain, Linear, 0.0, 2.0, 1.0 }
    float [units[6].input_pan] "units.6.inputPan" "Unit 7 input pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [units[6].sync] "units.6.sync" "Unit 7 sync" { true }
    float [units[6].time_ms] "units.6.timeMs" "Unit 7 time" { Milliseconds, Logarithmic, 1.0, 24_000.0, 250.0 }
    choice [units[6].division] "units.6.division" "Unit 7 division" { NoteDivision, Eighth, [Whole "whole" "1/1", HalfDotted "halfDotted" "1/2 dotted", Half "half" "1/2", HalfTriplet "halfTriplet" "1/2 triplet", QuarterDotted "quarterDotted" "1/4 dotted", Quarter "quarter" "1/4", QuarterTriplet "quarterTriplet" "1/4 triplet", EighthDotted "eighthDotted" "1/8 dotted", Eighth "eighth" "1/8", EighthTriplet "eighthTriplet" "1/8 triplet", SixteenthDotted "sixteenthDotted" "1/16 dotted", Sixteenth "sixteenth" "1/16", SixteenthTriplet "sixteenthTriplet" "1/16 triplet", ThirtySecond "thirtySecond" "1/32"] }
    float [units[6].stereo_offset_ms] "units.6.stereoOffsetMs" "Unit 7 offset" { Milliseconds, Linear, -1000.0, 1000.0, 0.0 }
    float [units[6].feedback] "units.6.feedback" "Unit 7 feedback" { Fraction, Linear, 0.0, 1.0, 0.35 }
    choice [units[6].feedback_mode] "units.6.feedbackMode" "Unit 7 feedback mode" { EchoFeedbackMode, Normal, [Off "off" "Off", Normal "normal" "Normal", Inverted "inverted" "Inverted", PingPong "pingPong" "Ping-pong"] }
    float [units[6].feedback_pan] "units.6.feedbackPan" "Unit 7 feedback pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    float [units[6].separation] "units.6.separation" "Unit 7 separation" { Fraction, Linear, 0.0, 1.0, 1.0 }
    choice [units[6].filter.mode] "units.6.filter.mode" "Unit 7 filter mode" { EchoFilterMode, Off, [Off "off" "Off", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Notch "notch" "Notch", Highpass "highpass" "Highpass", LowShelf "lowShelf" "Low shelf", Peak "peak" "Peak", HighShelf "highShelf" "High shelf"] }
    float [units[6].filter.frequency_hz] "units.6.filter.frequencyHz" "Unit 7 filter frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1000.0 }
    float [units[6].filter.q] "units.6.filter.q" "Unit 7 filter Q" { Ratio, Logarithmic, 0.5, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    float [units[6].filter.gain_db] "units.6.filter.gainDb" "Unit 7 filter shelf/peak gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [units[6].filter.gain] "units.6.filter.gain" "Unit 7 filter gain" { Gain, Linear, 0.1, 2.0, 1.0 }
    int [units[6].filter.sections] "units.6.filter.sections" "Unit 7 filter sections" { None, 1, 3, 1 }
    choice [units[6].feedback_filter.mode] "units.6.feedbackFilter.mode" "Unit 7 feedback filter mode" { EchoFilterMode, Off, [Off "off" "Off", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Notch "notch" "Notch", Highpass "highpass" "Highpass", LowShelf "lowShelf" "Low shelf", Peak "peak" "Peak", HighShelf "highShelf" "High shelf"] }
    float [units[6].feedback_filter.frequency_hz] "units.6.feedbackFilter.frequencyHz" "Unit 7 feedback filter frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1000.0 }
    float [units[6].feedback_filter.q] "units.6.feedbackFilter.q" "Unit 7 feedback filter Q" { Ratio, Logarithmic, 0.5, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    float [units[6].feedback_filter.gain_db] "units.6.feedbackFilter.gainDb" "Unit 7 feedback filter shelf/peak gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [units[6].feedback_filter.gain] "units.6.feedbackFilter.gain" "Unit 7 feedback filter gain" { Gain, Linear, 0.1, 2.0, 1.0 }
    int [units[6].feedback_filter.sections] "units.6.feedbackFilter.sections" "Unit 7 feedback filter sections" { None, 1, 3, 1 }
    toggle [units[6].filter_post] "units.6.filterPost" "Unit 7 post filter" { false }
    float [units[6].output_gain] "units.6.outputGain" "Unit 7 output" { None, Linear, -1.0, 1.0, 1.0 }
    float [units[6].output_pan] "units.6.outputPan" "Unit 7 output pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    float [units[6].next_send] "units.6.nextSend" "Unit 7 next send" { None, Linear, -1.0, 1.0, 0.0 }
    toggle [units[7].enabled] "units.7.enabled" "Unit 8 enabled" { false }
    float [units[7].input_gain] "units.7.inputGain" "Unit 8 input" { Gain, Linear, 0.0, 2.0, 1.0 }
    float [units[7].input_pan] "units.7.inputPan" "Unit 8 input pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [units[7].sync] "units.7.sync" "Unit 8 sync" { true }
    float [units[7].time_ms] "units.7.timeMs" "Unit 8 time" { Milliseconds, Logarithmic, 1.0, 24_000.0, 250.0 }
    choice [units[7].division] "units.7.division" "Unit 8 division" { NoteDivision, Eighth, [Whole "whole" "1/1", HalfDotted "halfDotted" "1/2 dotted", Half "half" "1/2", HalfTriplet "halfTriplet" "1/2 triplet", QuarterDotted "quarterDotted" "1/4 dotted", Quarter "quarter" "1/4", QuarterTriplet "quarterTriplet" "1/4 triplet", EighthDotted "eighthDotted" "1/8 dotted", Eighth "eighth" "1/8", EighthTriplet "eighthTriplet" "1/8 triplet", SixteenthDotted "sixteenthDotted" "1/16 dotted", Sixteenth "sixteenth" "1/16", SixteenthTriplet "sixteenthTriplet" "1/16 triplet", ThirtySecond "thirtySecond" "1/32"] }
    float [units[7].stereo_offset_ms] "units.7.stereoOffsetMs" "Unit 8 offset" { Milliseconds, Linear, -1000.0, 1000.0, 0.0 }
    float [units[7].feedback] "units.7.feedback" "Unit 8 feedback" { Fraction, Linear, 0.0, 1.0, 0.35 }
    choice [units[7].feedback_mode] "units.7.feedbackMode" "Unit 8 feedback mode" { EchoFeedbackMode, Normal, [Off "off" "Off", Normal "normal" "Normal", Inverted "inverted" "Inverted", PingPong "pingPong" "Ping-pong"] }
    float [units[7].feedback_pan] "units.7.feedbackPan" "Unit 8 feedback pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    float [units[7].separation] "units.7.separation" "Unit 8 separation" { Fraction, Linear, 0.0, 1.0, 1.0 }
    choice [units[7].filter.mode] "units.7.filter.mode" "Unit 8 filter mode" { EchoFilterMode, Off, [Off "off" "Off", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Notch "notch" "Notch", Highpass "highpass" "Highpass", LowShelf "lowShelf" "Low shelf", Peak "peak" "Peak", HighShelf "highShelf" "High shelf"] }
    float [units[7].filter.frequency_hz] "units.7.filter.frequencyHz" "Unit 8 filter frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1000.0 }
    float [units[7].filter.q] "units.7.filter.q" "Unit 8 filter Q" { Ratio, Logarithmic, 0.5, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    float [units[7].filter.gain_db] "units.7.filter.gainDb" "Unit 8 filter shelf/peak gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [units[7].filter.gain] "units.7.filter.gain" "Unit 8 filter gain" { Gain, Linear, 0.1, 2.0, 1.0 }
    int [units[7].filter.sections] "units.7.filter.sections" "Unit 8 filter sections" { None, 1, 3, 1 }
    choice [units[7].feedback_filter.mode] "units.7.feedbackFilter.mode" "Unit 8 feedback filter mode" { EchoFilterMode, Off, [Off "off" "Off", Lowpass "lowpass" "Lowpass", Bandpass "bandpass" "Bandpass", Notch "notch" "Notch", Highpass "highpass" "Highpass", LowShelf "lowShelf" "Low shelf", Peak "peak" "Peak", HighShelf "highShelf" "High shelf"] }
    float [units[7].feedback_filter.frequency_hz] "units.7.feedbackFilter.frequencyHz" "Unit 8 feedback filter frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1000.0 }
    float [units[7].feedback_filter.q] "units.7.feedbackFilter.q" "Unit 8 feedback filter Q" { Ratio, Logarithmic, 0.5, 10.0, std::f32::consts::FRAC_1_SQRT_2 }
    float [units[7].feedback_filter.gain_db] "units.7.feedbackFilter.gainDb" "Unit 8 feedback filter shelf/peak gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
    float [units[7].feedback_filter.gain] "units.7.feedbackFilter.gain" "Unit 8 feedback filter gain" { Gain, Linear, 0.1, 2.0, 1.0 }
    int [units[7].feedback_filter.sections] "units.7.feedbackFilter.sections" "Unit 8 feedback filter sections" { None, 1, 3, 1 }
    toggle [units[7].filter_post] "units.7.filterPost" "Unit 8 post filter" { false }
    float [units[7].output_gain] "units.7.outputGain" "Unit 8 output" { None, Linear, -1.0, 1.0, 1.0 }
    float [units[7].output_pan] "units.7.outputPan" "Unit 8 output pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    float [units[7].next_send] "units.7.nextSend" "Unit 8 next send (no destination)" { None, Linear, 0.0, 0.0, 0.0 }
});
