//! Whole-sample delay edits that retain the currently audible tap mixture.

#[derive(Debug, Clone, Copy)]
struct Tap {
    delay: usize,
    weight: f32,
}

/// A linear fade from a fixed mixture to the latest requested tap.
///
/// Retargeting freezes the mixture at its current weights and starts a new
/// fade. There is at most one entry per whole-sample delay in the prepared
/// range. Storage is reserved at construction; edits, reads, reset and
/// history transfer never allocate or free. After `frames` advances without
/// another edit only the final tap remains, regardless of earlier edits.
#[derive(Debug)]
pub struct TapCrossfade {
    from: Vec<Tap>,
    target: usize,
    max_delay: usize,
    remaining: u32,
    frames: u32,
}

impl TapCrossfade {
    pub fn new(max_delay: usize, target: usize) -> Self {
        Self {
            from: Vec::with_capacity(max_delay + 1),
            target: target.min(max_delay),
            max_delay,
            remaining: 0,
            frames: 1,
        }
    }

    fn is_settled(&self) -> bool {
        self.remaining == 0
    }

    pub fn remaining(&self) -> u32 {
        self.remaining
    }

    pub fn snap(&mut self, target: usize) {
        self.target = target.min(self.max_delay);
        self.remaining = 0;
        self.from.clear();
    }

    fn add(&mut self, delay: usize, weight: f32) {
        if weight == 0.0 {
            return;
        }
        if let Some(tap) = self.from.iter_mut().find(|tap| tap.delay == delay) {
            tap.weight += weight;
        } else {
            // Every index is in 0..=max_delay and appears at most once.
            self.from.push(Tap { delay, weight });
        }
    }

    pub fn retarget(&mut self, target: usize, frames: u32) {
        let target = target.min(self.max_delay);
        if frames == 0 {
            self.snap(target);
        } else if target != self.target {
            let old = self.remaining as f32 / self.frames as f32;
            for tap in &mut self.from {
                tap.weight *= old;
            }
            self.from.retain(|tap| tap.weight > 0.0);
            self.add(self.target, 1.0 - old);
            self.target = target;
            self.frames = frames;
            self.remaining = frames;
        }
    }

    /// Keep the limiter's existing policy: an edit during its fade replaces
    /// the destination without restarting the fade or changing its source.
    pub fn retarget_joining(&mut self, target: usize, frames: u32) {
        if self.is_settled() || frames == 0 {
            self.retarget(target, frames);
        } else {
            self.target = target.min(self.max_delay);
        }
    }

    /// Read before advancing, so the first sample of an edit uses exactly
    /// the previous mixture. The same clock can serve both stereo channels.
    pub fn read(&self, tap: impl Fn(usize) -> f32) -> f32 {
        let target = tap(self.target);
        let mut out = target;
        let old = self.remaining as f32 / self.frames as f32;
        for from in &self.from {
            out += (tap(from.delay) - target) * (from.weight * old);
        }
        out
    }

    pub fn advance(&mut self) {
        self.remaining = self.remaining.saturating_sub(1);
        if self.remaining == 0 {
            self.from.clear();
        }
    }

    /// Includes old audible taps and the destination for tail accounting.
    pub fn longest_delay(&self) -> usize {
        self.from
            .iter()
            .fold(self.target, |max, tap| max.max(tap.delay))
    }

    /// Copy a transition into already prepared storage, merging any taps
    /// clamped to its range. Used when a host replaces a compensation line.
    pub fn take_history(&mut self, other: &Self) {
        self.snap(other.target);
        for tap in &other.from {
            self.add(tap.delay.min(self.max_delay), tap.weight);
        }
        self.remaining = other.remaining;
        self.frames = other.frames;
    }
}
