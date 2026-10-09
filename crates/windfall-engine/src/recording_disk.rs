//! Native mixer disk taps. Callback copies; worker gates and writes frames.
use crate::mixer::{Frame, MAX_BLOCK};
use crate::recording_clock::CaptureGate;
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use windfall_ipc::RecordingMixerTap;
use windfall_project::MixerRecordMode;

struct DiskPacket {
    frame: i128,
    len: usize,
    audio: [Frame; MAX_BLOCK],
}
struct Flags {
    accepting: AtomicBool,
    fault: AtomicBool,
}
pub struct DiskReader {
    queue: Consumer<DiskPacket>,
    flags: Arc<Flags>,
    gate: CaptureGate,
    offset: i128,
    next: u64,
    previous: Option<i128>,
    latest: i128,
    output: [f32; MAX_BLOCK * 2],
    used: usize,
}
pub(crate) struct DiskWriter {
    queue: Producer<DiskPacket>,
    flags: Arc<Flags>,
    gate: CaptureGate,
    pub tap: RecordingMixerTap,
}
pub(crate) fn channel(
    tap: RecordingMixerTap,
    gate: CaptureGate,
    rate: u32,
    offset_ms: f64,
) -> (DiskReader, DiskWriter) {
    let (producer, consumer) = RingBuffer::new(256);
    let flags = Arc::new(Flags {
        accepting: AtomicBool::new(true),
        fault: AtomicBool::new(false),
    });
    (
        DiskReader {
            queue: consumer,
            flags: flags.clone(),
            gate: gate.clone(),
            offset: (offset_ms * f64::from(rate) / 1000.0).round() as i128,
            next: 0,
            previous: None,
            latest: i128::MIN,
            output: [0.0; MAX_BLOCK * 2],
            used: 0,
        },
        DiskWriter {
            queue: producer,
            flags,
            gate,
            tap,
        },
    )
}
impl DiskWriter {
    pub(crate) fn copy(
        &mut self,
        base: u64,
        master_delay: u64,
        track_delay: usize,
        stage: MixerRecordMode,
        audio: &[Frame],
    ) {
        if self.tap.mode != stage || !self.flags.accepting.load(Ordering::Acquire) {
            return;
        }
        if !self.gate.current() {
            self.flags.fault.store(true, Ordering::Release);
            return;
        }
        if self.gate.scheduled_frame().is_none() {
            return;
        }
        let mut packet = DiskPacket {
            frame: i128::from(base) + i128::from(master_delay) - track_delay as i128,
            len: audio.len(),
            audio: [[0.0; 2]; MAX_BLOCK],
        };
        packet.audio[..audio.len()].copy_from_slice(audio);
        if audio
            .iter()
            .any(|frame| !frame[0].is_finite() || !frame[1].is_finite())
            || self.queue.push(packet).is_err()
        {
            self.flags.fault.store(true, Ordering::Release);
        }
    }
}
impl DiskReader {
    pub fn failed(&self) -> bool {
        self.flags.fault.load(Ordering::Acquire)
    }
    pub fn frames(&self) -> u64 {
        self.next
    }
    pub fn close(&self) {
        self.flags.accepting.store(false, Ordering::Release);
    }
    /// One bounded route visit. Frames are tagged as the DAC position they
    /// correspond to, compensating the remaining path after this track.
    pub(crate) fn pump<F>(&mut self, sink: &mut F) -> Result<bool, String>
    where
        F: FnMut(&[f32]) -> Result<(), String>,
    {
        let window = self.gate.window()?;
        if self.failed() {
            return Err(
                "Mixer disk tap overflowed or its output clock changed; take discarded.".into(),
            );
        }
        let mut received = false;
        for _ in 0..32 {
            let Ok(packet) = self.queue.pop() else {
                break;
            };
            received = true;
            if self.next > 0 && self.previous.is_some_and(|next| next != packet.frame) {
                return Err(
                    "Mixer processing latency changed during disk recording; take discarded."
                        .into(),
                );
            }
            self.previous = Some(packet.frame + packet.len as i128);
            self.latest = packet.frame + packet.len as i128 - self.offset;
            let Some(start) = window.start_frame else {
                continue;
            };
            let output_clock = self
                .gate
                .clock()
                .output()
                .ok_or("The mixer recording output clock is unavailable.")?;
            for (i, sample) in packet.audio[..packet.len].iter().enumerate() {
                let tag = packet.frame + i as i128 - self.offset;
                let index = tag - i128::from(start);
                if index < i128::from(self.next) {
                    continue;
                }
                if tag < 0
                    || output_clock.nanos_at(tag.min(i128::from(u64::MAX)) as u64) >= window.end
                {
                    continue;
                }
                let index = index.min(i128::from(u64::MAX)) as u64;
                while self.next < index {
                    self.emit([0.0; 2], sink)?;
                    self.next += 1;
                }
                self.emit(*sample, sink)?;
                self.next += 1;
            }
        }
        if self.used > 0 {
            sink(&self.output[..self.used])?;
            self.used = 0;
        }
        Ok(received)
    }
    pub(crate) fn tail_ready(&self) -> Result<bool, String> {
        let window = self.gate.window()?;
        if window.start_frame.is_none() {
            return Ok(window.end != u64::MAX);
        }
        Ok(self.latest >= 0
            && self.gate.clock().output().is_some_and(|clock| {
                clock.nanos_at(self.latest.min(i128::from(u64::MAX)) as u64) >= window.end
            }))
    }
    fn emit<F>(&mut self, sample: Frame, sink: &mut F) -> Result<(), String>
    where
        F: FnMut(&[f32]) -> Result<(), String>,
    {
        self.output[self.used..self.used + 2].copy_from_slice(&sample);
        self.used += 2;
        if self.used == self.output.len() {
            sink(&self.output)?;
            self.used = 0;
        }
        Ok(())
    }
}
impl Drop for DiskReader {
    fn drop(&mut self) {
        self.close();
    }
}
