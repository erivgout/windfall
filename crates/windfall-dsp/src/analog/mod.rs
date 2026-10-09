//! Windfall's bounded analog, wavetable and macro instruments.
//! All callback state is inline; harmonic tables are generated in prepare.

mod acid_line;
mod common;
mod macro_voice;
mod params;
mod triple_osc;
mod wave_lane;

pub use acid_line::AcidLine;
pub use macro_voice::MacroVoice;
pub use params::{
    AcidLineParams, AcidStep, AnalogEnvelopeParams, AnalogOscParams, MacroVoiceParams,
    TripleOscParams, WaveLaneParams,
};
pub use triple_osc::TripleOsc;
pub use wave_lane::WaveLane;

/// Fixed capacity of each polyphonic instrument, including releases.
pub const MAX_POLYPHONY: usize = 8;

#[cfg(test)]
mod tests;
