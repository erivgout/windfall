//! One function per factory sound, grouped by instrument. Each returns the
//! raw render; [`crate::dsp::finish`] trims and normalizes it afterwards.

pub mod bass;
pub mod claps;
pub mod cymbals;
pub mod hats;
pub mod kicks;
pub mod percussion;
pub mod snares;
pub mod toms;
