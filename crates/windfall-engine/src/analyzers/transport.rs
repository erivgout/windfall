use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use rtrb::{Consumer, Producer, PushError, RingBuffer};

use super::*;

static TAPS: AtomicUsize = AtomicUsize::new(0);
static SLOTS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);

pub(super) struct Lease {
    bytes: usize,
    slot: bool,
}

impl Lease {
    #[allow(
        deprecated,
        reason = "fetch_update supports the workspace Rust 1.90 MSRV; try_update is newer"
    )]
    fn reserve(bytes: usize, slot: bool) -> Result<Self, PrepareError> {
        let (count, limit, error) = if slot {
            (&SLOTS, MAX_TAP_SLOTS, PrepareError::SlotLimit)
        } else {
            (&TAPS, MAX_LIVE_TAPS, PrepareError::InstanceLimit)
        };
        count
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < limit).then_some(n + 1)
            })
            .map_err(|_| error)?;
        if BYTES
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                n.checked_add(bytes)
                    .filter(|&sum| sum <= MAX_RETAINED_BYTES)
            })
            .is_err()
        {
            count.fetch_sub(1, Ordering::AcqRel);
            return Err(PrepareError::ByteLimit);
        }
        Ok(Self { bytes, slot })
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        BYTES.fetch_sub(self.bytes, Ordering::AcqRel);
        if self.slot { &SLOTS } else { &TAPS }.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Independent atomic observations; not a transaction across concurrent prep.
pub fn usage() -> Usage {
    Usage {
        taps: TAPS.load(Ordering::Acquire),
        slots: SLOTS.load(Ordering::Acquire),
        reserved_bytes: BYTES.load(Ordering::Acquire),
    }
}

// Each counter passed here has one writer (tap, worker or reader). Saturating
// load/store avoids an unbounded CAS retry on audio. Readers are observational.
pub(super) fn add(counter: &AtomicU64, amount: u64) {
    counter.store(
        counter.load(Ordering::Relaxed).saturating_add(amount),
        Ordering::Relaxed,
    );
}

#[allow(
    deprecated,
    reason = "fetch_update supports the workspace Rust 1.90 MSRV; try_update is newer"
)]
pub(super) fn pool_fault(shared: &Shared) {
    // Two writers, exclusively off audio; saturation uses an atomic update.
    let _ = shared
        .counters
        .pool_faults
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
            Some(n.saturating_add(1))
        });
    shared.alive.store(false, Ordering::Release);
}

#[derive(Default)]
pub(super) struct AtomicCounters {
    pub published_packets: AtomicU64,
    pub published_frames: AtomicU64,
    pub dropped_packets: AtomicU64,
    pub dropped_frames: AtomicU64,
    pub refused_publications: AtomicU64,
    pub invalid_samples: AtomicU64,
    pub invalid_metadata: AtomicU64,
    pub resets: AtomicU64,
    pub stale_packets: AtomicU64,
    pub stale_frames: AtomicU64,
    pub consumed_packets: AtomicU64,
    pub consumed_frames: AtomicU64,
    pub analyzed_windows: AtomicU64,
    pub dropped_snapshots: AtomicU64,
    pub stale_snapshots: AtomicU64,
    pub pool_faults: AtomicU64,
}

impl AtomicCounters {
    fn read(&self) -> Counters {
        Counters {
            published_packets: self.published_packets.load(Ordering::Relaxed),
            published_frames: self.published_frames.load(Ordering::Relaxed),
            dropped_packets: self.dropped_packets.load(Ordering::Relaxed),
            dropped_frames: self.dropped_frames.load(Ordering::Relaxed),
            refused_publications: self.refused_publications.load(Ordering::Relaxed),
            invalid_samples: self.invalid_samples.load(Ordering::Relaxed),
            invalid_metadata: self.invalid_metadata.load(Ordering::Relaxed),
            resets: self.resets.load(Ordering::Relaxed),
            stale_packets: self.stale_packets.load(Ordering::Relaxed),
            stale_frames: self.stale_frames.load(Ordering::Relaxed),
            consumed_packets: self.consumed_packets.load(Ordering::Relaxed),
            consumed_frames: self.consumed_frames.load(Ordering::Relaxed),
            analyzed_windows: self.analyzed_windows.load(Ordering::Relaxed),
            dropped_snapshots: self.dropped_snapshots.load(Ordering::Relaxed),
            stale_snapshots: self.stale_snapshots.load(Ordering::Relaxed),
            pool_faults: self.pool_faults.load(Ordering::Relaxed),
        }
    }
}

pub(super) struct Shared {
    pub alive: AtomicBool,
    pub epoch: AtomicU64,
    pub counters: AtomicCounters,
    pub layout: Layout,
    _lease: Lease,
}

impl Shared {
    pub fn counters(&self) -> Counters {
        self.counters.read()
    }
}

#[derive(Clone, Copy)]
pub(super) struct Packet {
    pub stamp: Stamp,
    pub epoch: u64,
    pub len: usize,
    pub invalid_metadata: bool,
    pub pcm: [[f32; 2]; MAX_PUBLICATION_FRAMES],
}

/// Control/worker-side bounded preparation. Failure neither cancels nor mutates
/// any previously prepared tap. Rust/dependency allocator OOM can still abort;
/// see the documented distinction from recoverable configuration/quota errors.
pub fn prepare(
    selection: Selection,
    config: Config,
) -> Result<(AudioTap, AnalyzerWorker, SnapshotReader), PrepareError> {
    let config = config.validate()?;
    let layout = AnalyzerWorker::layout(config);
    let lease = Lease::reserve(layout.reserved_bytes, false)?;
    let shared = Arc::new(Shared {
        alive: AtomicBool::new(true),
        epoch: AtomicU64::new(0),
        counters: AtomicCounters::default(),
        layout,
        _lease: lease,
    });
    let (tx, rx) = RingBuffer::new(config.input_packets);
    let (worker, reader) = AnalyzerWorker::prepare(selection, config, rx, shared.clone())?;
    let tap = AudioTap {
        selection,
        tx,
        shared,
        last: None,
    };
    Ok((tap, worker, reader))
}

/// Single audio owner. Contains heap-owning endpoints: move to control through
/// [`AudioTapSlot`] when replacing it. Never destroy this value on audio.
pub struct AudioTap {
    selection: Selection,
    tx: Producer<Packet>,
    shared: Arc<Shared>,
    last: Option<Continuity>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Continuity {
    clock: ClockEpoch,
    next_frame: u64,
    sample_rate: u32,
    pdc_frames: u32,
    device_latency_frames: Option<u32>,
}

impl Continuity {
    fn at(stamp: Stamp, next_frame: u64) -> Self {
        Self {
            clock: stamp.clock,
            next_frame,
            sample_rate: stamp.sample_rate,
            pdc_frames: stamp.pdc_frames,
            device_latency_frames: stamp.device_latency_frames,
        }
    }
}

impl AudioTap {
    pub fn selection(&self) -> Selection {
        self.selection
    }
    pub fn layout(&self) -> Layout {
        self.shared.layout
    }

    /// Callback-safe reset, immediately invalidating queued/published evidence.
    /// There is no buffer clearing, allocation or destruction here.
    pub fn reset(&mut self) -> Result<(), Refusal> {
        self.last = None;
        let Some(next) = self.shared.epoch.load(Ordering::Relaxed).checked_add(1) else {
            self.shared.alive.store(false, Ordering::Release);
            return Err(Refusal::EpochExhausted);
        };
        self.shared.epoch.store(next, Ordering::Release);
        add(&self.shared.counters.resets, 1);
        Ok(())
    }

    fn refuse(&mut self, why: Refusal, frames: usize) -> Result<(), Refusal> {
        add(&self.shared.counters.refused_publications, 1);
        add(&self.shared.counters.dropped_frames, frames as u64);
        if why != Refusal::Cancelled {
            self.reset()?;
        }
        Err(why)
    }

    /// Copy at most 256 stereo frames; larger calls are refused whole, never
    /// truncated. Callers partition larger engine blocks and advance stamps.
    /// No FFT, transcendental math, allocation, free, lock, wait or IO.
    pub fn publish(&mut self, mut stamp: Stamp, pcm: &[[f32; 2]]) -> Result<(), Refusal> {
        if !self.shared.alive.load(Ordering::Acquire) {
            return self.refuse(Refusal::Cancelled, pcm.len());
        }
        if stamp.selection != self.selection {
            return self.refuse(Refusal::WrongSelection, pcm.len());
        }
        if pcm.len() > MAX_PUBLICATION_FRAMES {
            return self.refuse(Refusal::TooManyFrames, pcm.len());
        }
        if !(8_000..=384_000).contains(&stamp.sample_rate)
            || stamp.pdc_frames > stamp.sample_rate
            || stamp
                .device_latency_frames
                .is_some_and(|n| n > stamp.sample_rate)
        {
            return self.refuse(Refusal::InvalidClock, pcm.len());
        }
        let Some(end) = stamp.first_frame.checked_add(pcm.len() as u64) else {
            return self.refuse(Refusal::FrameOverflow, pcm.len());
        };
        if pcm.is_empty() {
            return Ok(());
        }
        if self
            .last
            .is_some_and(|last| last != Continuity::at(stamp, stamp.first_frame))
        {
            self.reset()?;
        }
        let invalid_metadata = stamp
            .gain_reduction
            .is_some_and(|g| !g.db.is_finite() || !(0.0..=120.0).contains(&g.db));
        if invalid_metadata {
            stamp.gain_reduction = None;
            add(&self.shared.counters.invalid_metadata, 1);
        }
        let mut packet = Packet {
            stamp,
            epoch: self.shared.epoch.load(Ordering::Relaxed),
            len: pcm.len(),
            invalid_metadata,
            pcm: [[0.0; 2]; MAX_PUBLICATION_FRAMES],
        };
        packet.pcm[..pcm.len()].copy_from_slice(pcm);
        let invalid = pcm.iter().flatten().filter(|v| !v.is_finite()).count();
        add(&self.shared.counters.invalid_samples, invalid as u64);
        if self.tx.push(packet).is_err() {
            add(&self.shared.counters.dropped_packets, 1);
            return self.refuse(Refusal::QueueFull, pcm.len());
        }
        self.last = Some(Continuity::at(stamp, end));
        add(&self.shared.counters.published_packets, 1);
        add(&self.shared.counters.published_frames, pcm.len() as u64);
        Ok(())
    }
}

impl Drop for AudioTap {
    fn drop(&mut self) {
        self.shared.alive.store(false, Ordering::Release);
    }
}

pub(super) fn same_clock(a: Stamp, b: Stamp) -> bool {
    a.selection == b.selection
        && a.clock == b.clock
        && a.sample_rate == b.sample_rate
        && a.pdc_frames == b.pdc_frames
        && a.device_latency_frames == b.device_latency_frames
}

struct SlotShared {
    deferred: AtomicU64,
    _lease: Lease,
}

/// Fixed one-candidate and one-retirement queues. Destruction is control-side.
pub struct AudioTapSlot {
    active: Option<AudioTap>,
    pending: Option<Option<AudioTap>>,
    incoming: Consumer<Option<AudioTap>>,
    retired: Producer<AudioTap>,
    shared: Arc<SlotShared>,
}

pub struct TapInstaller {
    incoming: Producer<Option<AudioTap>>,
    retired: Consumer<AudioTap>,
    shared: Arc<SlotShared>,
}

pub fn tap_slot() -> Result<(AudioTapSlot, TapInstaller), PrepareError> {
    // Includes all slot objects/rings and conservative dependency overhead.
    let lease = Lease::reserve(128 * 1024, true)?;
    let shared = Arc::new(SlotShared {
        deferred: AtomicU64::new(0),
        _lease: lease,
    });
    let (incoming_tx, incoming_rx) = RingBuffer::new(1);
    let (retired_tx, retired_rx) = RingBuffer::new(1);
    Ok((
        AudioTapSlot {
            active: None,
            pending: None,
            incoming: incoming_rx,
            retired: retired_tx,
            shared: shared.clone(),
        },
        TapInstaller {
            incoming: incoming_tx,
            retired: retired_rx,
            shared,
        },
    ))
}

impl TapInstaller {
    /// Off audio/locks. A full staging queue returns the exact candidate owner;
    /// the existing active tap remains unchanged. No unbounded backlog.
    pub fn stage(&mut self, tap: AudioTap) -> Result<(), AudioTap> {
        self.incoming
            .push(Some(tap))
            .map_err(|PushError::Full(tap)| tap.expect("staged tap"))
    }

    /// Stage deselection. False preserves active/staged owners unchanged.
    pub fn clear(&mut self) -> bool {
        self.incoming.push(None).is_ok()
    }

    /// At most one retired owner is destroyed on this control-side call.
    pub fn collect_retired(&mut self) -> bool {
        self.retired.pop().is_ok()
    }
    pub fn deferred_installs(&self) -> u64 {
        self.shared.deferred.load(Ordering::Relaxed)
    }
}

impl AudioTapSlot {
    /// Callback boundary: at most one swap. Backpressure preserves both old
    /// and staged owners until control collects retirement. No owner is freed.
    pub fn boundary(&mut self) -> bool {
        if self.pending.is_none() && self.incoming.slots() == 0 {
            return false;
        }
        if self.active.is_some() && self.retired.slots() == 0 {
            add(&self.shared.deferred, 1);
            return false;
        }
        let next = self.pending.take().or_else(|| self.incoming.pop().ok());
        let Some(next) = next else {
            return false;
        };
        if let Some(old) = self.active.take() {
            old.shared.alive.store(false, Ordering::Release);
            if let Err(PushError::Full(old)) = self.retired.push(old) {
                // Defensive ownership preservation, even if an invariant breaks.
                self.active = Some(old);
                self.pending = Some(next);
                add(&self.shared.deferred, 1);
                return false;
            }
        }
        self.active = next;
        true
    }

    pub fn active(&mut self) -> Option<&mut AudioTap> {
        self.active.as_mut()
    }
}
