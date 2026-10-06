//! Tests of the engine through its public API. None of them needs an audio
//! device: they drive a `Processor` directly or use the offline renderer.

mod device;
mod mixer;
mod realtime;
mod rendering;
mod sampler;
mod sequencing;
mod support;

/// Counts allocator calls for the realtime tests. It does nothing on a
/// thread that is not being watched, so the other tests are unaffected.
#[global_allocator]
static ALLOCATOR: realtime::CountingAllocator = realtime::CountingAllocator;
