//! Bounded, sample-driven FM and ring-modulation instruments.
//!
//! FM uses phase modulation in radians. Matrix routes have one sample of
//! delay, making arbitrary cycles deterministic without an implicit solver.

mod common;
mod four_op;
mod matrix;
mod params;
mod ring;

pub use four_op::FourOp;
pub use matrix::MatrixFm;
pub use params::{
    FmEnvelopeParams, FmOperatorParams, FourOpAlgorithm, FourOpParams, HybridOscillatorParams,
    HybridWaveform, MatrixFmParams, MatrixOperatorParams, RingHybridParams,
};
pub use ring::RingHybrid;

/// Fixed number of voices, including releasing voices, per instrument.
pub const MAX_POLYPHONY: usize = 16;

#[cfg(test)]
mod tests;
