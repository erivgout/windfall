use std::f64::consts::TAU;
use std::mem::size_of;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use rtrb::{Consumer, Producer, PushError, RingBuffer};
use rustfft::algorithm::Radix4;
use rustfft::num_complex::Complex;
use rustfft::{Fft, FftDirection};

use super::transport::{Packet, Shared, add, pool_fault, same_clock};
use super::*;

const WORKER_STACK: usize = 2 * 1024 * 1024;

#[derive(Clone, Copy, Default)]
struct Captured {
    pcm: [f32; 2],
    gain: Option<GainReduction>,
    invalid_metadata: bool,
}

fn storage<T: Clone>(len: usize, value: T) -> Result<Vec<T>, PrepareError> {
    let mut out = Vec::new();
    out.try_reserve_exact(len)
        .map_err(|_| PrepareError::Allocation)?;
    out.resize(len, value);
    Ok(out)
}

fn empty_range() -> ClockRange {
    ClockRange {
        clock: ClockEpoch(0),
        first_frame: 0,
        end_frame: 0,
        sample_rate: 48_000,
        pdc_frames: 0,
        device_latency_frames: None,
    }
}

fn slices(config: Config) -> Result<Vec<SpectrogramSlice>, PrepareError> {
    let mut out = Vec::new();
    out.try_reserve_exact(SPECTROGRAM_SLICES)
        .map_err(|_| PrepareError::Allocation)?;
    for _ in 0..SPECTROGRAM_SLICES {
        out.push(SpectrogramSlice {
            range: empty_range(),
            valid: [false; 2],
            mean_square: storage(config.fft_size / 2 + 1, [f64::NAN; 2])?,
        });
    }
    Ok(out)
}

fn empty_snapshot(selection: Selection, config: Config) -> Result<Box<Snapshot>, PrepareError> {
    let mut history = storage(config.fft_size * 2, [0.0; 2])?;
    history.clear();
    let mut envelope = storage(
        ENVELOPE_BINS,
        EnvelopeBin {
            first_frame: 0,
            end_frame: 0,
            min: [None; 2],
            max: [None; 2],
            invalid_samples: [0; 2],
        },
    )?;
    envelope.clear();
    let mut vectorscope = storage(VECTOR_POINTS, [0.0; 2])?;
    vectorscope.clear();
    Ok(Box::new(Snapshot {
        selection,
        stream_epoch: 0,
        range: empty_range(),
        config,
        calibration: WindowCalibration {
            coherent_gain: 0.0,
            mean_square_gain: 0.0,
            enbw_bins: 0.0,
        },
        channels: [ChannelStats::default(); 2],
        stereo: StereoStats::default(),
        spectrum_valid: [false; 2],
        spectrum: storage(config.fft_size / 2 + 1, SpectrumBin::default())?,
        history_first_frame: 0,
        history,
        envelope,
        vectorscope,
        spectrogram: slices(config)?,
        spectrogram_count: 0,
        gain_reduction: None,
        invalid_metadata: 0,
        counters: Counters::default(),
    }))
}

/// Deterministic bounded consumer. `pump` is worker/control-side only. It
/// consumes at most configured input_packets (<=64), including stale packets.
pub struct AnalyzerWorker {
    selection: Selection,
    config: Config,
    input: Consumer<Packet>,
    ready: Producer<Box<Snapshot>>,
    recycled: Consumer<Box<Snapshot>>,
    spare: Option<Box<Snapshot>>,
    fft: Radix4<f64>,
    work: Vec<Complex<f64>>,
    scratch: Vec<Complex<f64>>,
    window: Vec<f64>,
    calibration: WindowCalibration,
    history: Vec<Captured>,
    head: usize,
    count: usize,
    until_window: usize,
    epoch: u64,
    last: Option<Stamp>,
    spectrum: Vec<SpectrumBin>,
    spectrogram: Vec<SpectrogramSlice>,
    slice_head: usize,
    slice_count: usize,
    // Declaration-order destruction keeps quota credit until every buffer/ring
    // above has been reclaimed, including deterministic worker-last shutdown.
    shared: Arc<Shared>,
}

impl AnalyzerWorker {
    pub(super) fn layout(c: Config) -> Layout {
        let n = c.fft_size;
        let bins = n / 2 + 1;
        let spectral_history =
            SPECTROGRAM_SLICES * (size_of::<SpectrogramSlice>() + bins * size_of::<[f64; 2]>());
        let snapshot = size_of::<Snapshot>()
            + n * 2 * size_of::<[f32; 2]>()
            + ENVELOPE_BINS * size_of::<EnvelopeBin>()
            + VECTOR_POINTS * size_of::<[f64; 2]>()
            + bins * size_of::<SpectrumBin>()
            + spectral_history;
        let pool = c.snapshot_capacity + 2;
        let payload_bytes = size_of::<Self>()
            + size_of::<SnapshotReader>()
            + size_of::<AudioTap>()
            + size_of::<Shared>()
            + c.input_packets * size_of::<Packet>()
            + (2 * c.snapshot_capacity + 1) * size_of::<Box<Snapshot>>()
            + pool * snapshot
            + n * 2 * size_of::<Captured>()
            + n * size_of::<f64>()
            + n * 2 * size_of::<Complex<f64>>()
            + bins * size_of::<SpectrumBin>()
            + spectral_history;
        // Version-audited scalar Radix4: twiddle construction reserves at most
        // 2*N complex values, fixed Butterfly16/32 base; scratch is N. 64 KiB
        // covers dependency objects/Arc/ring headers and control bookkeeping.
        let reserved_bytes =
            payload_bytes + n * 2 * size_of::<Complex<f64>>() + 64 * 1024 + WORKER_STACK;
        Layout {
            payload_bytes,
            reserved_bytes,
            snapshot_pool: pool,
            input_frames: c.input_packets * MAX_PUBLICATION_FRAMES,
        }
    }

    pub(super) fn prepare(
        selection: Selection,
        config: Config,
        input: Consumer<Packet>,
        shared: Arc<Shared>,
    ) -> Result<(Self, SnapshotReader), PrepareError> {
        let n = config.fft_size;
        let fft = Radix4::new(n, FftDirection::Forward);
        let mut window = storage(n, 1.0)?;
        if config.window == Window::PeriodicHann {
            for (i, w) in window.iter_mut().enumerate() {
                *w = 0.5 - 0.5 * (TAU * i as f64 / n as f64).cos();
            }
        }
        let sum: f64 = window.iter().sum();
        let squared_sum: f64 = window.iter().map(|w| w * w).sum();
        let calibration = WindowCalibration {
            coherent_gain: sum / n as f64,
            mean_square_gain: squared_sum / n as f64,
            enbw_bins: n as f64 * squared_sum / (sum * sum),
        };
        let (ready_tx, ready_rx) = RingBuffer::new(config.snapshot_capacity);
        let (mut recycle_tx, recycle_rx) = RingBuffer::new(config.snapshot_capacity + 1);
        let spare = empty_snapshot(selection, config)?;
        for _ in 0..config.snapshot_capacity + 1 {
            // Initialization is off-thread and capacity exactly matches count.
            recycle_tx
                .push(empty_snapshot(selection, config)?)
                .map_err(|_| PrepareError::Allocation)?;
        }
        let worker = Self {
            selection,
            config,
            input,
            ready: ready_tx,
            recycled: recycle_rx,
            spare: Some(spare),
            work: storage(n, Complex::new(0.0, 0.0))?,
            scratch: storage(fft.get_inplace_scratch_len(), Complex::new(0.0, 0.0))?,
            fft,
            window,
            calibration,
            history: storage(n * 2, Captured::default())?,
            head: 0,
            count: 0,
            until_window: n,
            epoch: 0,
            last: None,
            spectrum: storage(n / 2 + 1, SpectrumBin::default())?,
            spectrogram: slices(config)?,
            slice_head: 0,
            slice_count: 0,
            shared: shared.clone(),
        };
        let reader = SnapshotReader {
            selection,
            capacity: config.snapshot_capacity,
            ready: ready_rx,
            recycled: recycle_tx,
            latest: None,
            shared,
        };
        Ok((worker, reader))
    }

    fn clear(&mut self, epoch: u64) {
        self.head = 0;
        self.count = 0;
        self.until_window = self.config.fft_size;
        self.slice_head = 0;
        self.slice_count = 0;
        self.last = None;
        self.epoch = epoch;
    }

    pub fn pump(&mut self) -> PumpReport {
        let mut report = PumpReport::default();
        let epoch = self.shared.epoch.load(Ordering::Acquire);
        if self.epoch != epoch {
            self.clear(epoch);
        }
        for _ in 0..self.config.input_packets {
            if !self.shared.alive.load(Ordering::Acquire) {
                break;
            }
            let Ok(packet) = self.input.pop() else {
                break;
            };
            report.packets += 1;
            let epoch = self.shared.epoch.load(Ordering::Acquire);
            if self.epoch != epoch {
                self.clear(epoch);
            }
            if packet.epoch != epoch || packet.stamp.selection != self.selection {
                add(&self.shared.counters.stale_packets, 1);
                add(&self.shared.counters.stale_frames, packet.len as u64);
                report.stale_packets += 1;
                continue;
            }
            if self.last.is_some_and(|last| {
                last.first_frame != packet.stamp.first_frame || !same_clock(last, packet.stamp)
            }) {
                // Producer guarantees discontinuity epochs; defend against any
                // inconsistent internal packet without combining windows.
                self.clear(epoch);
            }
            for i in 0..packet.len {
                self.history[self.head] = Captured {
                    pcm: packet.pcm[i],
                    gain: packet.stamp.gain_reduction,
                    invalid_metadata: packet.invalid_metadata,
                };
                self.head = (self.head + 1) % self.history.len();
                self.count = (self.count + 1).min(self.history.len());
                self.until_window -= 1;
                if self.until_window == 0 {
                    let end = packet.stamp.first_frame + i as u64 + 1;
                    let range = ClockRange {
                        clock: packet.stamp.clock,
                        first_frame: end - self.config.fft_size as u64,
                        end_frame: end,
                        sample_rate: packet.stamp.sample_rate,
                        pdc_frames: packet.stamp.pdc_frames,
                        device_latency_frames: packet.stamp.device_latency_frames,
                    };
                    if self.analyze(range) {
                        report.snapshots += 1;
                    }
                    self.until_window = self.config.hop_size;
                }
            }
            add(&self.shared.counters.consumed_packets, 1);
            add(&self.shared.counters.consumed_frames, packet.len as u64);
            self.last = Some(Stamp {
                first_frame: packet.stamp.first_frame + packet.len as u64,
                ..packet.stamp
            });
        }
        report
    }

    fn sample(&self, into_window: usize) -> Captured {
        self.history[(self.head + self.history.len() - self.config.fft_size + into_window)
            % self.history.len()]
    }

    fn analyze(&mut self, range: ClockRange) -> bool {
        let n = self.config.fft_size;
        let mut channels = [ChannelStats::default(); 2];
        let mut square = [0.0_f64; 2];
        let mut peak = [0.0_f64; 2];
        let (mut cross, mut mid_square, mut side_square) = (0.0, 0.0, 0.0);
        let mut gain = self.sample(0).gain;
        let mut gain_complete = gain.is_some();
        let mut invalid_metadata = 0;
        for i in 0..n {
            let sample = self.sample(i);
            invalid_metadata += u32::from(sample.invalid_metadata);
            match (gain, sample.gain) {
                (Some(old), Some(new)) if old.effect == new.effect => {
                    gain = Some(GainReduction {
                        db: old.db.max(new.db),
                        ..old
                    });
                }
                _ => gain_complete = false,
            }
            let [l, r] = sample.pcm.map(f64::from);
            for (side, value) in [l, r].into_iter().enumerate() {
                if value.is_finite() {
                    square[side] += value * value;
                    peak[side] = peak[side].max(value.abs());
                    channels[side].clip_samples += u32::from(value.abs() >= 1.0);
                } else {
                    channels[side].invalid_samples += 1;
                }
            }
            cross += l * r;
            mid_square += ((l + r) * 0.5).powi(2);
            side_square += ((l - r) * 0.5).powi(2);
        }
        let valid = channels.map(|channel| channel.invalid_samples == 0);
        for side in 0..2 {
            if valid[side] {
                let rms = (square[side] / n as f64).sqrt();
                channels[side].levels = Some(Levels {
                    sample_peak: peak[side],
                    rms,
                    peak_dbfs: dbfs(peak[side]),
                    rms_dbfs: dbfs(rms),
                });
            }
            for i in 0..n {
                self.work[i] =
                    Complex::new(f64::from(self.sample(i).pcm[side]) * self.window[i], 0.0);
            }
            if valid[side] {
                self.fft
                    .process_with_scratch(&mut self.work, &mut self.scratch);
            }
            for (k, bin) in self.spectrum.iter_mut().enumerate() {
                if valid[side] {
                    let factor = if k == 0 || k == n / 2 { 1.0 } else { 2.0 };
                    let power = self.work[k].norm_sqr();
                    bin.coherent_amplitude[side] =
                        factor * power.sqrt() / (n as f64 * self.calibration.coherent_gain);
                    bin.mean_square[side] =
                        factor * power / ((n * n) as f64 * self.calibration.mean_square_gain);
                    bin.phase_radians[side] = if power == 0.0 {
                        f64::NAN
                    } else {
                        self.work[k].arg()
                    };
                } else {
                    bin.coherent_amplitude[side] = f64::NAN;
                    bin.mean_square[side] = f64::NAN;
                    bin.phase_radians[side] = f64::NAN;
                }
            }
        }
        let stereo = if valid == [true; 2] {
            let denominator = square[0].sqrt() * square[1].sqrt();
            StereoStats {
                correlation: (denominator > 0.0).then(|| (cross / denominator).clamp(-1.0, 1.0)),
                mid_rms: Some((mid_square / n as f64).sqrt()),
                side_rms: Some((side_square / n as f64).sqrt()),
            }
        } else {
            StereoStats::default()
        };
        let slice = &mut self.spectrogram[self.slice_head];
        slice.range = range;
        slice.valid = valid;
        for (out, bin) in slice.mean_square.iter_mut().zip(&self.spectrum) {
            *out = bin.mean_square;
        }
        self.slice_head = (self.slice_head + 1) % SPECTROGRAM_SLICES;
        self.slice_count = (self.slice_count + 1).min(SPECTROGRAM_SLICES);
        add(&self.shared.counters.analyzed_windows, 1);
        if self.ready.slots() == 0 || self.recycled.slots() == 0 {
            add(&self.shared.counters.dropped_snapshots, 1);
            return false;
        }
        let Ok(next_spare) = self.recycled.pop() else {
            return false;
        };
        let Some(mut out) = self.spare.replace(next_spare) else {
            pool_fault(&self.shared);
            return false;
        };
        out.stream_epoch = self.epoch;
        out.range = range;
        out.calibration = self.calibration;
        out.channels = channels;
        out.stereo = stereo;
        out.spectrum_valid = valid;
        out.spectrum.copy_from_slice(&self.spectrum);
        out.gain_reduction = if gain_complete { gain } else { None };
        out.invalid_metadata = invalid_metadata;
        out.history.clear();
        for i in 0..self.count {
            let index = (self.head + self.history.len() - self.count + i) % self.history.len();
            out.history.push(self.history[index].pcm);
        }
        out.history_first_frame = range.end_frame - self.count as u64;
        fill_envelope(&mut out);
        out.vectorscope.clear();
        let points = n.min(VECTOR_POINTS);
        for i in 0..points {
            let [l, r] = self.sample(i * (n - 1) / (points - 1)).pcm.map(f64::from);
            if l.is_finite() && r.is_finite() {
                out.vectorscope.push([(l + r) * 0.5, (l - r) * 0.5]);
            }
        }
        out.spectrogram_count = self.slice_count;
        for i in 0..self.slice_count {
            let from = &self.spectrogram[(self.slice_head + SPECTROGRAM_SLICES - self.slice_count
                + i)
                % SPECTROGRAM_SLICES];
            let to = &mut out.spectrogram[i];
            to.range = from.range;
            to.valid = from.valid;
            to.mean_square.copy_from_slice(&from.mean_square);
        }
        out.counters = self.shared.counters();
        if let Err(PushError::Full(out)) = self.ready.push(out) {
            // Preserve pool ownership; consumer never decreases available space,
            // so this is defensive rather than an expected racing path.
            self.spare = Some(out);
            pool_fault(&self.shared);
            return false;
        }
        true
    }

    /// Start one native helper for this consumer. No device is opened. Spawn
    /// failure consumes only this candidate off-thread and cancels its tap.
    pub fn spawn(self) -> std::io::Result<WorkerThread> {
        let shared = self.shared.clone();
        let handle = thread::Builder::new()
            .name("windfall-analyzer".into())
            .stack_size(WORKER_STACK)
            .spawn(move || {
                let mut worker = self;
                while worker.shared.alive.load(Ordering::Acquire) {
                    if worker.pump().packets == 0 {
                        thread::sleep(Duration::from_millis(1));
                    }
                }
                worker
            })?;
        Ok(WorkerThread {
            handle: Some(handle),
            shared,
        })
    }
}

impl Drop for AnalyzerWorker {
    fn drop(&mut self) {
        self.shared.alive.store(false, Ordering::Release);
    }
}

fn dbfs(value: f64) -> Dbfs {
    if value == 0.0 {
        Dbfs::Silence
    } else {
        Dbfs::Finite(20.0 * value.log10())
    }
}

fn fill_envelope(out: &mut Snapshot) {
    out.envelope.clear();
    let count = out.history.len().min(ENVELOPE_BINS);
    for bin in 0..count {
        let from = bin * out.history.len() / count;
        let end = (bin + 1) * out.history.len() / count;
        let mut envelope = EnvelopeBin {
            first_frame: out.history_first_frame + from as u64,
            end_frame: out.history_first_frame + end as u64,
            min: [None; 2],
            max: [None; 2],
            invalid_samples: [0; 2],
        };
        for frame in &out.history[from..end] {
            for (side, &value) in frame.iter().enumerate() {
                if value.is_finite() {
                    envelope.min[side] =
                        Some(envelope.min[side].map_or(value, |old| old.min(value)));
                    envelope.max[side] =
                        Some(envelope.max[side].map_or(value, |old| old.max(value)));
                } else {
                    envelope.invalid_samples[side] += 1;
                }
            }
        }
        out.envelope.push(envelope);
    }
}

/// Off-thread UI/IPC endpoint. `poll` drains at most snapshot_capacity and
/// retains at most one snapshot. The returned borrow cannot outlive the next
/// mutable poll, preventing an unbounded retained snapshot backlog.
pub struct SnapshotReader {
    selection: Selection,
    capacity: usize,
    ready: Consumer<Box<Snapshot>>,
    recycled: Producer<Box<Snapshot>>,
    latest: Option<Box<Snapshot>>,
    shared: Arc<Shared>,
}

impl SnapshotReader {
    pub fn counters(&self) -> Counters {
        self.shared.counters()
    }
    pub fn layout(&self) -> Layout {
        self.shared.layout
    }
    pub fn is_cancelled(&self) -> bool {
        !self.shared.alive.load(Ordering::Acquire)
    }

    /// Source/project replacement caller must pass the current exact ticket.
    /// Mismatching, cancelled and old stream-epoch evidence is never returned.
    pub fn poll(&mut self, expected: Selection) -> Option<&Snapshot> {
        if self
            .latest
            .as_ref()
            .is_some_and(|s| !self.accepts(s, expected))
        {
            add(&self.shared.counters.stale_snapshots, 1);
            let old = self.latest.take().expect("checked latest");
            self.recycle(old);
        }
        for _ in 0..self.capacity {
            let Ok(snapshot) = self.ready.pop() else {
                break;
            };
            if self.accepts(&snapshot, expected) {
                if let Some(old) = self.latest.replace(snapshot) {
                    self.recycle(old);
                }
            } else {
                add(&self.shared.counters.stale_snapshots, 1);
                self.recycle(snapshot);
            }
        }
        // Audio may reset or control may cancel during the drain.
        if self
            .latest
            .as_ref()
            .is_some_and(|s| !self.accepts(s, expected))
        {
            add(&self.shared.counters.stale_snapshots, 1);
            let old = self.latest.take().expect("checked latest");
            self.recycle(old);
        }
        self.latest.as_deref()
    }

    fn accepts(&self, s: &Snapshot, expected: Selection) -> bool {
        self.shared.alive.load(Ordering::Acquire)
            && expected == self.selection
            && s.selection == expected
            && s.stream_epoch == self.shared.epoch.load(Ordering::Acquire)
    }

    fn recycle(&mut self, snapshot: Box<Snapshot>) {
        if self.recycled.push(snapshot).is_err() {
            // This reader is off audio. Quarantine a broken pool, without leak
            // or callback reclamation. Capacity conservation makes it unreachable.
            pool_fault(&self.shared);
        }
    }

    /// Off-thread cancellation is immediate for future polls/publications.
    pub fn cancel(&mut self) {
        self.shared.alive.store(false, Ordering::Release);
    }
}

impl Drop for SnapshotReader {
    fn drop(&mut self) {
        self.shared.alive.store(false, Ordering::Release);
    }
}

/// Control-side RAII shutdown; drop/shutdown joins, never from audio or while
/// holding document/recording locks. Cancellation is checked each packet.
pub struct WorkerThread {
    handle: Option<JoinHandle<AnalyzerWorker>>,
    shared: Arc<Shared>,
}

impl WorkerThread {
    pub fn shutdown(mut self) -> Result<(), &'static str> {
        self.shared.alive.store(false, Ordering::Release);
        self.handle
            .take()
            .expect("owned worker")
            .join()
            .map(|_| ())
            .map_err(|_| "Analyzer worker panicked")
    }
}

impl Drop for WorkerThread {
    fn drop(&mut self) {
        self.shared.alive.store(false, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
