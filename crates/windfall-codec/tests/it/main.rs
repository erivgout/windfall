//! Integration tests for the codec crate. They live in one test binary so
//! the helpers in `common` are shared.

mod common;
mod encoder;
mod fixtures;
mod flac;
mod heap;
mod lossy;
mod metadata;
mod robustness;
mod roundtrip;
mod writer;

/// Counts the heap each test thread holds, for the tests in `metadata` and
/// `robustness`.
#[global_allocator]
static ALLOCATOR: heap::Metered = heap::Metered;
