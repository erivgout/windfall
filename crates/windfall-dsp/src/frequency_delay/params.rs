use crate::param::param_set;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum FrequencySplit {
    #[default]
    Gentle,
    Steep,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct FrequencyBandParams {
    /// Milliseconds before scaling, 0..1000; default zero.
    pub delay_ms: f32,
    /// Linear gain, 0..1; default unity.
    pub level: f32,
    /// Stereo balance, -1..1; default center.
    pub pan: f32,
    /// False passes the undelayed band; history remains live.
    pub enabled: bool,
}
impl Default for FrequencyBandParams {
    fn default() -> Self {
        Self {
            delay_ms: 0.0,
            level: 1.0,
            pan: 0.0,
            enabled: true,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct FrequencyDelayParams {
    pub dry: f32,
    pub wet: f32,
    pub feedback: f32,
    pub scale: f32,
    pub short_range: bool,
    pub bandwidth: f32,
    pub split: FrequencySplit,
    pub bands: [FrequencyBandParams; 16],
}
impl Default for FrequencyDelayParams {
    fn default() -> Self {
        Self {
            dry: 0.0,
            wet: 1.0,
            feedback: 0.0,
            scale: 1.0,
            short_range: false,
            bandwidth: 1.0,
            split: FrequencySplit::Gentle,
            bands: [FrequencyBandParams::default(); 16],
        }
    }
}
param_set!(FrequencyDelayParams, "Frequency delay", {
    float [dry] "dry" "Dry" { Gain, Linear, 0.0, 1.0, 0.0 }
    float [wet] "wet" "Wet" { Gain, Linear, 0.0, 1.0, 1.0 }
    float [feedback] "feedback" "Feedback" { Fraction, Linear, 0.0, 1.0, 0.0 }
    float [scale] "scale" "Scale" { Fraction, Linear, -1.0, 1.0, 1.0 }
    toggle [short_range] "shortRange" "Short range" { false }
    float [bandwidth] "bandwidth" "Bandwidth" { Ratio, Linear, 0.5, 2.0, 1.0 }
    choice [split] "split" "Splitter" { FrequencySplit, Gentle, [Gentle "gentle" "Gentle", Steep "steep" "Steep"] }
    float [bands[0].delay_ms] "bands.0.delayMs" "Band 1 delay" { Milliseconds, Linear, 0.0, 1000.0, 0.0 }
    float [bands[0].level] "bands.0.level" "Band 1 level" { Gain, Linear, 0.0, 1.0, 1.0 }
    float [bands[0].pan] "bands.0.pan" "Band 1 pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [bands[0].enabled] "bands.0.enabled" "Band 1 enabled" { true }
    float [bands[1].delay_ms] "bands.1.delayMs" "Band 2 delay" { Milliseconds, Linear, 0.0, 1000.0, 0.0 }
    float [bands[1].level] "bands.1.level" "Band 2 level" { Gain, Linear, 0.0, 1.0, 1.0 }
    float [bands[1].pan] "bands.1.pan" "Band 2 pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [bands[1].enabled] "bands.1.enabled" "Band 2 enabled" { true }
    float [bands[2].delay_ms] "bands.2.delayMs" "Band 3 delay" { Milliseconds, Linear, 0.0, 1000.0, 0.0 }
    float [bands[2].level] "bands.2.level" "Band 3 level" { Gain, Linear, 0.0, 1.0, 1.0 }
    float [bands[2].pan] "bands.2.pan" "Band 3 pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [bands[2].enabled] "bands.2.enabled" "Band 3 enabled" { true }
    float [bands[3].delay_ms] "bands.3.delayMs" "Band 4 delay" { Milliseconds, Linear, 0.0, 1000.0, 0.0 }
    float [bands[3].level] "bands.3.level" "Band 4 level" { Gain, Linear, 0.0, 1.0, 1.0 }
    float [bands[3].pan] "bands.3.pan" "Band 4 pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [bands[3].enabled] "bands.3.enabled" "Band 4 enabled" { true }
    float [bands[4].delay_ms] "bands.4.delayMs" "Band 5 delay" { Milliseconds, Linear, 0.0, 1000.0, 0.0 }
    float [bands[4].level] "bands.4.level" "Band 5 level" { Gain, Linear, 0.0, 1.0, 1.0 }
    float [bands[4].pan] "bands.4.pan" "Band 5 pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [bands[4].enabled] "bands.4.enabled" "Band 5 enabled" { true }
    float [bands[5].delay_ms] "bands.5.delayMs" "Band 6 delay" { Milliseconds, Linear, 0.0, 1000.0, 0.0 }
    float [bands[5].level] "bands.5.level" "Band 6 level" { Gain, Linear, 0.0, 1.0, 1.0 }
    float [bands[5].pan] "bands.5.pan" "Band 6 pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [bands[5].enabled] "bands.5.enabled" "Band 6 enabled" { true }
    float [bands[6].delay_ms] "bands.6.delayMs" "Band 7 delay" { Milliseconds, Linear, 0.0, 1000.0, 0.0 }
    float [bands[6].level] "bands.6.level" "Band 7 level" { Gain, Linear, 0.0, 1.0, 1.0 }
    float [bands[6].pan] "bands.6.pan" "Band 7 pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [bands[6].enabled] "bands.6.enabled" "Band 7 enabled" { true }
    float [bands[7].delay_ms] "bands.7.delayMs" "Band 8 delay" { Milliseconds, Linear, 0.0, 1000.0, 0.0 }
    float [bands[7].level] "bands.7.level" "Band 8 level" { Gain, Linear, 0.0, 1.0, 1.0 }
    float [bands[7].pan] "bands.7.pan" "Band 8 pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [bands[7].enabled] "bands.7.enabled" "Band 8 enabled" { true }
    float [bands[8].delay_ms] "bands.8.delayMs" "Band 9 delay" { Milliseconds, Linear, 0.0, 1000.0, 0.0 }
    float [bands[8].level] "bands.8.level" "Band 9 level" { Gain, Linear, 0.0, 1.0, 1.0 }
    float [bands[8].pan] "bands.8.pan" "Band 9 pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [bands[8].enabled] "bands.8.enabled" "Band 9 enabled" { true }
    float [bands[9].delay_ms] "bands.9.delayMs" "Band 10 delay" { Milliseconds, Linear, 0.0, 1000.0, 0.0 }
    float [bands[9].level] "bands.9.level" "Band 10 level" { Gain, Linear, 0.0, 1.0, 1.0 }
    float [bands[9].pan] "bands.9.pan" "Band 10 pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [bands[9].enabled] "bands.9.enabled" "Band 10 enabled" { true }
    float [bands[10].delay_ms] "bands.10.delayMs" "Band 11 delay" { Milliseconds, Linear, 0.0, 1000.0, 0.0 }
    float [bands[10].level] "bands.10.level" "Band 11 level" { Gain, Linear, 0.0, 1.0, 1.0 }
    float [bands[10].pan] "bands.10.pan" "Band 11 pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [bands[10].enabled] "bands.10.enabled" "Band 11 enabled" { true }
    float [bands[11].delay_ms] "bands.11.delayMs" "Band 12 delay" { Milliseconds, Linear, 0.0, 1000.0, 0.0 }
    float [bands[11].level] "bands.11.level" "Band 12 level" { Gain, Linear, 0.0, 1.0, 1.0 }
    float [bands[11].pan] "bands.11.pan" "Band 12 pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [bands[11].enabled] "bands.11.enabled" "Band 12 enabled" { true }
    float [bands[12].delay_ms] "bands.12.delayMs" "Band 13 delay" { Milliseconds, Linear, 0.0, 1000.0, 0.0 }
    float [bands[12].level] "bands.12.level" "Band 13 level" { Gain, Linear, 0.0, 1.0, 1.0 }
    float [bands[12].pan] "bands.12.pan" "Band 13 pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [bands[12].enabled] "bands.12.enabled" "Band 13 enabled" { true }
    float [bands[13].delay_ms] "bands.13.delayMs" "Band 14 delay" { Milliseconds, Linear, 0.0, 1000.0, 0.0 }
    float [bands[13].level] "bands.13.level" "Band 14 level" { Gain, Linear, 0.0, 1.0, 1.0 }
    float [bands[13].pan] "bands.13.pan" "Band 14 pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [bands[13].enabled] "bands.13.enabled" "Band 14 enabled" { true }
    float [bands[14].delay_ms] "bands.14.delayMs" "Band 15 delay" { Milliseconds, Linear, 0.0, 1000.0, 0.0 }
    float [bands[14].level] "bands.14.level" "Band 15 level" { Gain, Linear, 0.0, 1.0, 1.0 }
    float [bands[14].pan] "bands.14.pan" "Band 15 pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [bands[14].enabled] "bands.14.enabled" "Band 15 enabled" { true }
    float [bands[15].delay_ms] "bands.15.delayMs" "Band 16 delay" { Milliseconds, Linear, 0.0, 1000.0, 0.0 }
    float [bands[15].level] "bands.15.level" "Band 16 level" { Gain, Linear, 0.0, 1.0, 1.0 }
    float [bands[15].pan] "bands.15.pan" "Band 16 pan" { Pan, Linear, -1.0, 1.0, 0.0 }
    toggle [bands[15].enabled] "bands.15.enabled" "Band 16 enabled" { true }
});
