//! Tests of the stretcher through the crate's public API. The transforms
//! it is built on have their own tests next to their code.

mod bench;
mod examples;
mod offline;
mod quality;
mod realtime;
mod streaming;
mod support;
mod tempo;

/// Counts allocator calls for the realtime tests. It does nothing on a
/// thread that is not being watched, so the other tests are unaffected.
#[global_allocator]
static ALLOCATOR: realtime::CountingAllocator = realtime::CountingAllocator;
