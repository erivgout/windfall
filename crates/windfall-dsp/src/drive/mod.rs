//! Drawn curves, asymmetric overdrive, guitar pedals and a series drive chain.
//!
//! All processing is at the host sample rate. Sharp curves and clipping can
//! alias; see `docs/integration/seams/drive.md` for the integration limits.

mod chain;
mod guitar;
mod overdrive;
mod waveshaper;

pub use chain::{DriveChain, DriveChainParams, DriveStage, DriveStageParams};
pub use guitar::{GuitarModel, GuitarRack, GuitarRackParams};
pub use overdrive::{Overdrive, OverdriveParams};
pub use waveshaper::{CURVE_POINTS, IDENTITY_CURVE, Waveshaper, WaveshaperParams};

/// Different positive and negative ceilings, continuous with unit slope at
/// zero. Unlike a biased clipper, silence still maps exactly to silence.
#[inline]
fn asymmetric(input: f32, asymmetry: f32) -> f32 {
    let ceiling = if input >= 0.0 {
        1.0 - 0.45 * asymmetry
    } else {
        1.0 + 0.25 * asymmetry
    };
    input / (1.0 + input.abs() / ceiling)
}

#[cfg(test)]
mod tests;
