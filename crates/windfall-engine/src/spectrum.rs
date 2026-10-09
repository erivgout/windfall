//! The existing analyzer's mix-output tap and control-side snapshot owner.
//! Preparation/retirement follow the processor and Link's off-audio lifetime.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::analyzers::{
    self, AnalyzerWorker, AudioTap, ClockEpoch, Config, ProjectGeneration, Selection,
    SelectionGeneration, Snapshot, SnapshotReader, Source, Stamp,
};

// A ticket is never reused, even across separate controllers or failed attaches.
static NEXT_TICKET: AtomicU64 = AtomicU64::new(1);

pub(crate) struct SpectrumReader {
    worker: AnalyzerWorker,
    reader: SnapshotReader,
    selection: Selection,
    published_plan: Arc<AtomicUsize>,
}

pub(crate) struct SpectrumTap {
    tap: AudioTap,
    published_plan: Arc<AtomicUsize>,
    plan: usize,
    clock: u64,
    disabled: bool,
}

#[allow(
    deprecated,
    reason = "fetch_update supports the workspace Rust 1.90 minimum"
)]
pub(crate) fn prepare(project: u64, rate: u32) -> Option<(SpectrumTap, SpectrumReader)> {
    if !(8_000..=384_000).contains(&rate) {
        return None;
    }
    let ticket = NEXT_TICKET
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |old| {
            old.checked_add(1)
        })
        .ok()?;
    let selection = Selection {
        project: ProjectGeneration(project),
        generation: SelectionGeneration(ticket),
        source: Source::MixOutput,
    };
    let (tap, worker, reader) = analyzers::prepare(selection, Config::default()).ok()?;
    let published_plan = Arc::new(AtomicUsize::new(0));
    Some((
        SpectrumTap {
            tap,
            published_plan: published_plan.clone(),
            plan: 0,
            clock: 0,
            disabled: false,
        },
        SpectrumReader {
            worker,
            reader,
            selection,
            published_plan,
        },
    ))
}

impl SpectrumReader {
    /// Runs the existing bounded worker on the frame caller, never on audio.
    pub(crate) fn snapshot(&mut self, expected_plan: usize) -> Option<&Snapshot> {
        self.reader.poll(self.selection);
        self.worker.pump();
        if self.published_plan.load(Ordering::Acquire) != expected_plan {
            return None;
        }
        self.reader.poll(self.selection)
    }
}

impl SpectrumTap {
    pub(crate) fn reset(&mut self) {
        let Some(clock) = self.clock.checked_add(1) else {
            self.disabled = true;
            return;
        };
        self.clock = clock;
        if self.tap.reset().is_err() {
            self.disabled = true;
        }
    }

    /// Callback work is bounded PCM copies to the already prepared SPSC queue.
    pub(crate) fn publish(
        &mut self,
        pcm: &[[f32; 2]],
        base: u64,
        rate: u32,
        pdc: u64,
        plan: usize,
    ) {
        if self.disabled {
            return;
        }
        // Analyzer metadata supports at most one second of PDC. Refuse
        // unavailable metadata instead of truncating the engine's u64 latency.
        if pdc > u64::from(rate) {
            self.reset();
            return;
        }
        if plan != self.plan {
            self.reset();
            self.plan = plan;
            self.published_plan.store(plan, Ordering::Release);
        }
        if self.disabled {
            return;
        }
        for (index, chunk) in pcm.chunks(analyzers::MAX_PUBLICATION_FRAMES).enumerate() {
            let stamp = Stamp {
                selection: self.tap.selection(),
                clock: ClockEpoch(self.clock),
                first_frame: base + (index * analyzers::MAX_PUBLICATION_FRAMES) as u64,
                sample_rate: rate,
                pdc_frames: pdc as u32,
                device_latency_frames: None,
                gain_reduction: None,
            };
            if self.tap.publish(stamp, chunk).is_err() {
                break;
            }
        }
    }
}

/// Sum contiguous power bins, preserving the whole DC-through-Nyquist range.
pub(crate) fn summarize(snapshot: &Snapshot) -> Vec<f32> {
    let mut result = Vec::with_capacity(snapshot.spectrum.len().min(64));
    if !append_bands(
        &mut result,
        snapshot.spectrum_valid,
        snapshot.spectrum.iter().map(|bin| bin.mean_square),
    ) {
        result.clear();
    }
    result
}

/// Copies the existing chronological slices on the frame caller, never audio.
pub(crate) fn summarize_history(snapshot: &Snapshot) -> Vec<f32> {
    let slices = snapshot.spectrogram_slices();
    let columns = snapshot.spectrum.len().min(64);
    let mut result = Vec::with_capacity(slices.len().min(16) * columns);
    for slice in &slices[slices.len().saturating_sub(16)..] {
        // All rows must describe the same frequency partition as spectrum.
        if slice.mean_square.len() != snapshot.spectrum.len() {
            continue;
        }
        let start = result.len();
        if !append_bands(&mut result, slice.valid, slice.mean_square.iter().copied()) {
            result.truncate(start);
        }
    }
    result
}

/// Shared spectrum/history math: sum bins, then average valid stereo channels.
fn append_bands(
    result: &mut Vec<f32>,
    channels: [bool; 2],
    mut bins: impl ExactSizeIterator<Item = [f64; 2]>,
) -> bool {
    let valid = channels.iter().filter(|&&valid| valid).count();
    let source_count = bins.len();
    let count = source_count.min(64);
    if valid == 0 || count == 0 {
        return false;
    }
    for index in 0..count {
        let from = index * source_count / count;
        let end = (index + 1) * source_count / count;
        let mut power = 0.0;
        for bin in bins.by_ref().take(end - from) {
            for (side, &valid) in channels.iter().enumerate() {
                if valid {
                    let value = bin[side];
                    if !value.is_finite() || value < 0.0 {
                        return false;
                    }
                    power += value;
                }
            }
        }
        let power = (power / valid as f64) as f32;
        if !power.is_finite() {
            return false;
        }
        result.push(power);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Processor, SamplePool};
    use windfall_project::Project;

    #[test]
    fn power_summary_preserves_stereo_power_and_invalidity() {
        for (pcm, expected) in [
            ([0.5, -0.5], Some(0.25)),
            ([f32::NAN, 0.5], Some(0.25)),
            ([f32::NAN, f32::NAN], None),
            ([f32::MAX, f32::MAX], None),
        ] {
            let (mut tap, mut reader) = prepare(0, 48_000).expect("prepared analyzer");
            assert!(reader.snapshot(1).is_none());
            tap.publish(&[pcm; 1024], 0, 48_000, 0, 1);
            let snapshot = reader.snapshot(1).expect("full analysis window");
            assert_eq!(snapshot.spectrum.len(), 513);
            let summary = summarize(snapshot);
            if let Some(expected) = expected {
                assert_eq!(summary.len(), 64);
                assert!((summary.iter().sum::<f32>() - expected).abs() < 1e-6);
            } else {
                assert!(summary.is_empty());
            }
        }
    }

    #[test]
    fn engine_frames_invalidate_on_seek_project_and_stream_changes_without_audio_allocation() {
        let (mut processor, controller) = Processor::new(48_000);
        assert!(controller.frame().spectrum.is_empty());
        let mut pcm = [0.0; 2048];
        assert_eq!(
            crate::test_alloc::allocator_calls(|| processor.process(&mut pcm)),
            0
        );
        assert_eq!(controller.frame().spectrum, vec![0.0; 64]);

        controller.seek(1.0);
        processor.process(&mut []);
        assert!(controller.frame().spectrum.is_empty());
        processor.process(&mut pcm);
        assert_eq!(controller.frame().spectrum.len(), 64);

        controller.set_project(&Project::new("new spectrum source"), &SamplePool::new());
        assert!(
            controller.frame().spectrum.is_empty(),
            "old plan is hidden before audio adoption"
        );
        processor.process(&mut []);
        assert!(controller.frame().spectrum.is_empty());
        processor.process(&mut pcm);
        assert_eq!(controller.frame().spectrum.len(), 64);

        drop(processor);
        assert!(controller.frame().spectrum.is_empty());
        let mut processor = controller.attach(44_100);
        assert!(controller.frame().spectrum.is_empty());
        processor.process(&mut pcm);
        assert_eq!(controller.frame().spectrum.len(), 64);
    }

    #[test]
    fn queue_overflow_invalidates_and_recovers_after_a_fresh_window() {
        let (mut tap, mut reader) = prepare(0, 48_000).expect("prepared analyzer");
        let pcm = [[0.5; 2]; 256];
        for index in 0..33 {
            tap.publish(&pcm, index * 256, 48_000, 0, 1);
        }
        assert!(
            reader.snapshot(1).is_none(),
            "full queue reset removes stale evidence"
        );
        tap.publish(&[[0.5; 2]; 1024], 33 * 256, 48_000, 0, 1);
        assert_eq!(summarize(reader.snapshot(1).unwrap()).len(), 64);
    }
}
