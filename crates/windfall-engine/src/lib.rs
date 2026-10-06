//! The Windfall audio engine: realtime playback, offline rendering and the
//! audio device.
//!
//! The control side compiles a [`Project`](windfall_project::Project) and a
//! [`SamplePool`] into an immutable plan and sends it to the audio thread
//! over a lock-free queue. The audio thread swaps plans between buffers and
//! sends the old one back to be freed, so it never allocates, locks or
//! blocks. A [`Processor`] is that audio thread's whole job with no device
//! attached; [`Engine`] feeds one to a sound card, [`render`] runs one as
//! fast as it will go, and both therefore produce the same audio.
//!
//! # What plays
//!
//! - Pattern mode loops the transport's pattern. Song mode plays the
//!   playlist's pattern clips and stops, or loops, at the end of the last
//!   clip.
//! - A note starts on the first output frame at or after its tick, whatever
//!   the buffer size.
//! - Swing delays the second sixteenth-note step of each pair of steps. At
//!   full swing that step lands two thirds of the way through the pair.
//! - Stopping returns the playhead to where playback last started.
//!
//! # Mute and solo
//!
//! A muted channel or mixer track is silent, soloed or not. While any
//! channel is soloed, only soloed channels are heard. While any mixer track
//! is soloed, a track is heard only if it is soloed, feeds a soloed track
//! through its output or sends, or lies on the way from a soloed track to
//! the master. Channel solo and track solo do not affect each other.

mod controller;
mod device;
mod message;
mod mixer;
mod plan;
mod pool;
mod processor;
mod ramp;
mod render;
mod sequencer;
mod shared;
mod voice;

pub use controller::{Controller, StreamStats};
pub use device::Engine;
pub use pool::SamplePool;
pub use processor::Processor;
pub use render::{RenderOptions, render};
