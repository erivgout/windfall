//! LR4 multiband dynamics, transient splitting and harmonic enhancement.
//! Unity crossover recombination has flat magnitude and all-pass phase.

mod crossover;
mod dynamics;
mod harmonics;
mod params;
mod transient;

pub use dynamics::{BandSplit, MultibandCompressor, MultibandMaximizer, OneKnob};
pub use harmonics::{BassHarmonics, Exciter, HARMONICS_LATENCY_SAMPLES};
pub use params::*;
pub use transient::{TransientShaper, TransientSplit};

#[cfg(test)]
mod tests;
