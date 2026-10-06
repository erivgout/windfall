//! Mixer tracks: summing, faders, pan, routing and meters.

use windfall_core::pan_gains;
use windfall_project::MAX_MIXER_TRACKS;

use crate::plan::{Plan, PlanState};
use crate::ramp::Ramp;
use crate::shared::Shared;

/// Most frames the engine processes in one go. Larger device buffers are
/// worked through in pieces of this size.
pub(crate) const MAX_BLOCK: usize = 256;

/// One stereo frame: left, then right.
pub(crate) type Frame = [f32; 2];

pub(crate) struct Mixer {
    /// [`MAX_BLOCK`] frames per possible mixer track, end to end. Voices add
    /// into them, then [`Mixer::mix`] turns them into post-fader signals in
    /// routing order.
    buffers: Box<[Frame]>,
}

impl Mixer {
    pub fn new() -> Self {
        Self {
            buffers: vec![[0.0; 2]; MAX_MIXER_TRACKS * MAX_BLOCK].into_boxed_slice(),
        }
    }

    /// Silences the first `frames` frames of the first `tracks` tracks.
    pub fn clear(&mut self, tracks: usize, frames: usize) {
        for track in 0..tracks {
            self.track_mut(track)[..frames].fill([0.0; 2]);
        }
    }

    pub fn track_mut(&mut self, track: usize) -> &mut [Frame] {
        &mut self.buffers[track * MAX_BLOCK..][..MAX_BLOCK]
    }

    /// Runs the mixer over one block of `out.len()` frames starting on frame
    /// `base`, and writes the master to `out`.
    ///
    /// Tracks are taken in the plan's routing order. Each gets its fader and
    /// pan, is metered, and is then added to its output and sends, so by the
    /// time a track is reached everything feeding it has arrived.
    pub fn mix(
        &mut self,
        plan: &Plan,
        state: &PlanState,
        shared: &Shared,
        base: u64,
        out: &mut [Frame],
    ) {
        let frames = out.len();
        for &index in &plan.order {
            let track = &plan.tracks[index];
            let strip = &state.tracks[index];
            let buffer = &mut self.track_mut(index)[..frames];

            let steady = strip.gain.settled(base) && strip.pan.settled(base);
            let (mut left, mut right) = fader_gains(strip.gain.at(base), strip.pan.at(base));
            let (mut peak_left, mut peak_right) = (0.0_f32, 0.0_f32);
            for (offset, frame) in buffer.iter_mut().enumerate() {
                if !steady {
                    let at = base + offset as u64;
                    (left, right) = fader_gains(strip.gain.at(at), strip.pan.at(at));
                }
                frame[0] *= left;
                frame[1] *= right;
                peak_left = peak_left.max(frame[0].abs());
                peak_right = peak_right.max(frame[1].abs());
            }
            shared.raise_meter(index, peak_left, peak_right);

            for edge in &track.edges {
                let (source, target) = self.pair(index, edge.target);
                add_scaled(
                    &source[..frames],
                    &mut target[..frames],
                    &state.edges[edge.slot],
                    base,
                );
            }
        }
        out.copy_from_slice(&self.track_mut(0)[..frames]);
    }

    /// The buffers of two different tracks, the first to read and the second
    /// to write.
    fn pair(&mut self, source: usize, target: usize) -> (&[Frame], &mut [Frame]) {
        let (low, high) = self.buffers.split_at_mut(source.max(target) * MAX_BLOCK);
        let low = &mut low[source.min(target) * MAX_BLOCK..][..MAX_BLOCK];
        let high = &mut high[..MAX_BLOCK];
        if source < target {
            (&*low, high)
        } else {
            (&*high, low)
        }
    }
}

#[inline]
fn fader_gains(gain: f32, pan: f32) -> (f32, f32) {
    let (left, right) = pan_gains(pan);
    (gain * left, gain * right)
}

/// Adds `source` times the ramp's gain to `target`.
fn add_scaled(source: &[Frame], target: &mut [Frame], gain: &Ramp, base: u64) {
    let frames = source.iter().zip(target);
    if gain.settled(base) {
        let gain = gain.at(base);
        for (source, target) in frames {
            target[0] += source[0] * gain;
            target[1] += source[1] * gain;
        }
    } else {
        for (offset, (source, target)) in frames.enumerate() {
            let gain = gain.at(base + offset as u64);
            target[0] += source[0] * gain;
            target[1] += source[1] * gain;
        }
    }
}
