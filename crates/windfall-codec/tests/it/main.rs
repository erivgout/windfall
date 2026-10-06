//! Integration tests for the codec crate. They live in one test binary so
//! the helpers in `common` are shared.

mod common;
mod fixtures;
mod robustness;
mod roundtrip;
mod writer;
