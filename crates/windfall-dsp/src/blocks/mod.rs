//! The pieces every Windfall processor is built from: smoothers, filters,
//! delay lines, envelopes, oscillators and noise.
//!
//! None of them allocates after it is built, and none holds a lock. Each is
//! small enough to read in one sitting, and each cites the published
//! description it follows.

pub mod adsr;
pub mod biquad;
pub mod dc;
pub mod delay_line;
pub mod envelope;
pub mod halfband;
pub mod lfo;
pub mod math;
pub mod noise;
pub mod oscillator;
pub mod shaper;
pub mod smooth;
pub mod svf;

/// Samples between two updates of the values a processor moves slowly:
/// filter coefficients, envelope targets, parameter glides.
///
/// Processors count these samples themselves instead of updating once per
/// `process` call, so their output does not depend on how the host divides
/// the audio into blocks.
pub const CONTROL_PERIOD: usize = 16;

/// Time a gain-like parameter takes to reach a new value.
pub const SMOOTHING_MS: f32 = 20.0;
