//! Values the audio thread publishes for the control side to read, all
//! through atomics.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

use windfall_project::MAX_MIXER_TRACKS;

/// How much of each new measurement goes into the running CPU load average.
const LOAD_SMOOTHING: f32 = 0.05;

pub(crate) struct Shared {
    /// The transport sequence number of the last play or stop the audio
    /// thread handled, shifted left one bit, with the playing flag in the
    /// lowest bit. Publishing both in one value lets the control side tell
    /// an answer to its latest request from an older state.
    transport: AtomicU64,
    /// Playhead in ticks, as `f64` bits.
    tick: AtomicU64,
    voices: AtomicU32,
    /// Peak level per mixer track and side since the control side last
    /// looked, as `f32` bits: left, right, left, right.
    meters: [AtomicU32; MAX_MIXER_TRACKS * 2],
    /// Running average of processing time over buffer duration, `f32` bits.
    load: AtomicU32,
    /// Highest single-buffer load since the control side last looked.
    load_peak: AtomicU32,
    xruns: AtomicU32,
    /// Frames the device has asked for since the stream started.
    frames: AtomicU64,
    /// Size of the most recent device buffer in frames.
    buffer_frames: AtomicU32,
}

impl Shared {
    pub fn new() -> Self {
        Self {
            transport: AtomicU64::new(0),
            tick: AtomicU64::new(0.0_f64.to_bits()),
            voices: AtomicU32::new(0),
            meters: [const { AtomicU32::new(0) }; MAX_MIXER_TRACKS * 2],
            load: AtomicU32::new(0),
            load_peak: AtomicU32::new(0),
            xruns: AtomicU32::new(0),
            frames: AtomicU64::new(0),
            buffer_frames: AtomicU32::new(0),
        }
    }

    pub fn publish_transport(&self, sequence: u32, playing: bool) {
        self.transport.store(
            u64::from(sequence) << 1 | u64::from(playing),
            Ordering::Release,
        );
    }

    /// The sequence number of the last transport request handled, and
    /// whether the transport was playing after it.
    pub fn transport(&self) -> (u32, bool) {
        let value = self.transport.load(Ordering::Acquire);
        ((value >> 1) as u32, value & 1 == 1)
    }

    pub fn publish_position(&self, tick: f64, voices: u32) {
        self.tick.store(tick.to_bits(), Ordering::Relaxed);
        self.voices.store(voices, Ordering::Relaxed);
    }

    pub fn set_tick(&self, tick: f64) {
        self.tick.store(tick.to_bits(), Ordering::Relaxed);
    }

    pub fn tick(&self) -> f64 {
        f64::from_bits(self.tick.load(Ordering::Relaxed))
    }

    pub fn voices(&self) -> u32 {
        self.voices.load(Ordering::Relaxed)
    }

    /// Raises a track's meter to these peaks if they are higher.
    pub fn raise_meter(&self, track: usize, left: f32, right: f32) {
        for (side, peak) in [left, right].into_iter().enumerate() {
            // Levels are never negative, and for such floats the bit
            // patterns sort the same way the values do. A level that is not
            // a number would stick, so it is left out.
            if peak > 0.0 && peak.is_finite() {
                self.meters[track * 2 + side].fetch_max(peak.to_bits(), Ordering::Relaxed);
            }
        }
    }

    /// Reads one meter value and resets it.
    pub fn take_meter(&self, index: usize) -> f32 {
        f32::from_bits(self.meters[index].swap(0, Ordering::Relaxed))
    }

    /// Clears the stream measurements when a device stream starts.
    pub fn begin_stream(&self) {
        self.load.store(0, Ordering::Relaxed);
        self.load_peak.store(0, Ordering::Relaxed);
        self.xruns.store(0, Ordering::Relaxed);
        self.frames.store(0, Ordering::Relaxed);
        self.buffer_frames.store(0, Ordering::Relaxed);
    }

    /// Records one device buffer: how many frames it held and how long the
    /// engine took to fill it. Taking longer than the buffer lasts is an
    /// xrun.
    pub fn record_buffer(&self, frames: usize, elapsed: Duration, sample_rate: u32) {
        if frames == 0 {
            return;
        }
        let duration = frames as f64 / f64::from(sample_rate);
        let load = (elapsed.as_secs_f64() / duration) as f32;
        let average = f32::from_bits(self.load.load(Ordering::Relaxed));
        let average = average + (load - average) * LOAD_SMOOTHING;
        self.load.store(average.to_bits(), Ordering::Relaxed);
        self.load_peak.fetch_max(load.to_bits(), Ordering::Relaxed);
        if load > 1.0 {
            self.record_xrun();
        }
        self.frames.fetch_add(frames as u64, Ordering::Relaxed);
        self.buffer_frames.store(frames as u32, Ordering::Relaxed);
    }

    pub fn record_xrun(&self) {
        self.xruns.fetch_add(1, Ordering::Relaxed);
    }

    pub fn load(&self) -> f32 {
        f32::from_bits(self.load.load(Ordering::Relaxed))
    }

    /// Reads the highest single-buffer load and resets it.
    pub fn take_load_peak(&self) -> f32 {
        f32::from_bits(self.load_peak.swap(0, Ordering::Relaxed))
    }

    pub fn xruns(&self) -> u32 {
        self.xruns.load(Ordering::Relaxed)
    }

    pub fn frames(&self) -> u64 {
        self.frames.load(Ordering::Relaxed)
    }

    pub fn buffer_frames(&self) -> u32 {
        self.buffer_frames.load(Ordering::Relaxed)
    }
}
