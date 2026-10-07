//! Native MIDI control. One worker owns all port handles and output sends.
//! The input callback only parses fixed-size supported messages, reads atomics,
//! and pushes a Copy packet into a preallocated SPSC queue. No project access.
mod native;
#[cfg(test)]
mod tests;

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rtrb::{Consumer, Producer, RingBuffer};
use windfall_ipc::{MidiHardwareSettings, MidiHardwareState, MidiPort};

use crate::Controller;

pub const INPUT_CAPACITY: usize = 1024;
const POLL: Duration = Duration::from_millis(500);
const SERVICE: Duration = Duration::from_millis(2);

#[derive(Clone, Copy, Debug, PartialEq)]
enum Event {
    Note { channel: u8, key: u8, velocity: u8 },
    Sustain { channel: u8, down: bool },
    Panic { channel: u8 },
}

impl Event {
    fn parse(bytes: &[u8]) -> Option<Self> {
        let [status, a, b] = *bytes else {
            return None;
        };
        if a > 127 || b > 127 {
            return None;
        }
        let channel = status & 15;
        match status >> 4 {
            8 => Some(Self::Note {
                channel,
                key: a,
                velocity: 0,
            }),
            9 => Some(Self::Note {
                channel,
                key: a,
                velocity: b,
            }),
            11 if a == 64 => Some(Self::Sustain {
                channel,
                down: b >= 64,
            }),
            11 if a == 121 => Some(Self::Sustain {
                channel,
                down: false,
            }),
            11 if a == 120 || a == 123 => Some(Self::Panic { channel }),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
struct Packet {
    epoch: u64,
    event: Event,
}

/// Moved into the backend callback and used by deterministic fake inputs too.
pub struct Ingress {
    tx: Producer<Packet>,
    controller: Controller,
    dropped: Arc<AtomicU32>,
}

impl Ingress {
    pub fn receive(&mut self, bytes: &[u8]) {
        let Some(event) = Event::parse(bytes) else {
            return;
        };
        let packet = Packet {
            epoch: self.controller.hardware_epoch(),
            event,
        };
        if self.tx.push(packet).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            self.controller.panic_hardware();
        }
    }
}

/// Invoked on the control worker, never on either hardware callback.
/// Verify the session generation/target and enqueue the engine note atomically
/// with respect to project changes. False invalidates the whole held-note set.
pub trait Audition: Send + Sync + 'static {
    fn note(&self, epoch: u64, key: u8, velocity: u8) -> bool;
}

pub trait Output: Send {
    fn send(&mut self, bytes: &[u8]) -> Result<(), String>;
}

/// Device API seam. Construction and all calls happen on the owning worker.
pub trait Ports: Send + 'static {
    fn enumerate(&mut self) -> Result<(Vec<MidiPort>, Vec<MidiPort>), String>;
    fn input(&mut self, id: &str, ingress: Ingress) -> Result<Box<dyn Send>, String>;
    fn output(&mut self, id: &str) -> Result<Box<dyn Output>, String>;
}

enum Request {
    Configure(MidiHardwareSettings, mpsc::SyncSender<MidiHardwareState>),
    Refresh(mpsc::SyncSender<MidiHardwareState>),
}

/// Thread-safe control handle; native handles never leave its worker.
pub struct Runtime {
    tx: Option<mpsc::SyncSender<Request>>,
    state: Arc<Mutex<MidiHardwareState>>,
    controller: Controller,
    thread: Option<JoinHandle<()>>,
}

impl Runtime {
    pub fn start(
        controller: Controller,
        audition: Arc<dyn Audition>,
        settings: MidiHardwareSettings,
    ) -> Result<Self, String> {
        Self::start_with(controller, audition, settings, || Box::new(native::Native))
    }

    pub fn start_with(
        controller: Controller,
        audition: Arc<dyn Audition>,
        settings: MidiHardwareSettings,
        factory: impl FnOnce() -> Box<dyn Ports> + Send + 'static,
    ) -> Result<Self, String> {
        settings.validate()?;
        let (tx, rx) = mpsc::sync_channel(16);
        let state = Arc::new(Mutex::new(MidiHardwareState::default()));
        let published = state.clone();
        let worker_controller = controller.clone();
        let thread = thread::Builder::new()
            .name("windfall-midi".into())
            .spawn(move || {
                let mut worker = Worker::new(factory(), worker_controller, audition);
                worker.configure(settings);
                let mut polled = Instant::now();
                loop {
                    match rx.recv_timeout(SERVICE) {
                        Ok(Request::Configure(settings, reply)) => {
                            worker.configure(settings);
                            let _ = reply.send(worker.state());
                        }
                        Ok(Request::Refresh(reply)) => {
                            worker.refresh();
                            let _ = reply.send(worker.state());
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }
                    if polled.elapsed() >= POLL {
                        worker.refresh();
                        polled = Instant::now();
                    }
                    worker.service();
                    *published.lock().unwrap_or_else(|e| e.into_inner()) = worker.state();
                }
                worker.panic();
                // All connections drop here, after output cleanup, on this worker.
            })
            .map_err(|error| error.to_string())?;
        Ok(Self {
            tx: Some(tx),
            state,
            controller,
            thread: Some(thread),
        })
    }

    pub fn state(&self) -> MidiHardwareState {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn configure(&self, settings: MidiHardwareSettings) -> Result<MidiHardwareState, String> {
        settings.validate()?;
        let (tx, rx) = mpsc::sync_channel(1);
        self.request(Request::Configure(settings, tx), rx)
    }

    pub fn refresh(&self) -> Result<MidiHardwareState, String> {
        let (tx, rx) = mpsc::sync_channel(1);
        self.request(Request::Refresh(tx), rx)
    }

    fn request(
        &self,
        request: Request,
        reply: mpsc::Receiver<MidiHardwareState>,
    ) -> Result<MidiHardwareState, String> {
        self.tx
            .as_ref()
            .ok_or("MIDI worker stopped.")?
            .try_send(request)
            .map_err(|_| "MIDI worker is busy or stopped.".to_string())?;
        reply.recv().map_err(|_| "MIDI worker stopped.".to_string())
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.controller.panic_hardware();
        self.tx.take();
        // A session can be released by its MIDI worker's last weak upgrade.
        // Never join ourselves; the disconnected control queue ends that worker.
        if let Some(thread) = self.thread.take()
            && thread.thread().id() != thread::current().id()
        {
            let _ = thread.join();
        }
    }
}

struct Worker {
    ports: Box<dyn Ports>,
    controller: Controller,
    audition: Arc<dyn Audition>,
    state: MidiHardwareState,
    input: Option<Box<dyn Send>>,
    output: Option<Box<dyn Output>>,
    rx: Option<Consumer<Packet>>,
    dropped: Arc<AtomicU32>,
    epoch: u64,
    /// One bit per input channel/key, and a separate sustain bit. Fixed storage.
    down: [[bool; 128]; 16],
    sounding: [[bool; 128]; 16],
    sustain: [bool; 16],
    output_keys: [bool; 128],
}

impl Worker {
    fn new(ports: Box<dyn Ports>, controller: Controller, audition: Arc<dyn Audition>) -> Self {
        let epoch = controller.hardware_epoch();
        Self {
            ports,
            controller,
            audition,
            state: MidiHardwareState::default(),
            input: None,
            output: None,
            rx: None,
            dropped: Arc::new(AtomicU32::new(0)),
            epoch,
            down: [[false; 128]; 16],
            sounding: [[false; 128]; 16],
            sustain: [false; 16],
            output_keys: [false; 128],
        }
    }

    fn state(&self) -> MidiHardwareState {
        MidiHardwareState {
            input_connected: self.input.is_some(),
            output_connected: self.output.is_some(),
            dropped_events: self.dropped.load(Ordering::Relaxed),
            ..self.state.clone()
        }
    }

    fn configure(&mut self, settings: MidiHardwareSettings) {
        self.panic();
        self.input.take();
        self.output.take();
        self.rx.take();
        self.state.settings = settings;
        self.state.error = None;
        self.refresh();
        if let Some(id) = self.state.settings.output.clone() {
            match self.ports.output(&id) {
                Ok(output) => self.output = Some(output),
                Err(error) => self.state.error = Some(error),
            }
        }
        if let Some(id) = self.state.settings.input.clone() {
            let (tx, rx) = RingBuffer::new(INPUT_CAPACITY);
            let ingress = Ingress {
                tx,
                controller: self.controller.clone(),
                dropped: self.dropped.clone(),
            };
            match self.ports.input(&id, ingress) {
                Ok(input) => {
                    self.input = Some(input);
                    self.rx = Some(rx);
                }
                Err(error) => self.state.error = Some(error),
            }
        }
    }

    fn refresh(&mut self) {
        match self.ports.enumerate() {
            Ok((inputs, outputs)) => {
                let lost_input = self.input.is_some()
                    && !inputs
                        .iter()
                        .any(|p| Some(&p.id) == self.state.settings.input.as_ref());
                let lost_output = self.output.is_some()
                    && !outputs
                        .iter()
                        .any(|p| Some(&p.id) == self.state.settings.output.as_ref());
                if lost_input || lost_output {
                    self.panic();
                    if lost_input {
                        self.input.take();
                        self.rx.take();
                    }
                    if lost_output {
                        self.output.take();
                    }
                    self.state.error = Some("MIDI device disconnected. Refresh and apply the device settings to reconnect.".into());
                }
                self.state.inputs = inputs;
                self.state.outputs = outputs;
            }
            Err(error) => {
                self.panic();
                self.input.take();
                self.output.take();
                self.rx.take();
                self.state.error = Some(error);
            }
        }
    }

    fn panic(&mut self) {
        self.controller.panic_hardware();
        self.reset();
    }

    fn reset(&mut self) {
        // Exact note-offs plus standard panic messages; no output call from audio.
        let channel = self.state.settings.output_channel.saturating_sub(1);
        for key in 0..128 {
            if self.output_keys[key] {
                self.send([0x80 | channel, key as u8, 0]);
            }
        }
        self.send([0xb0 | channel, 64, 0]);
        self.send([0xb0 | channel, 123, 0]);
        self.send([0xb0 | channel, 120, 0]);
        self.down = [[false; 128]; 16];
        self.sounding = [[false; 128]; 16];
        self.sustain = [false; 16];
        self.output_keys = [false; 128];
        self.epoch = self.controller.hardware_epoch();
    }

    fn send(&mut self, bytes: [u8; 3]) {
        if let Some(output) = &mut self.output
            && let Err(error) = output.send(&bytes)
        {
            self.state.error = Some(error);
            self.output.take();
            self.controller.panic_hardware();
        }
    }

    fn service(&mut self) {
        if self.epoch != self.controller.hardware_epoch() {
            self.reset();
        }
        // Bounded service budget; control requests and disconnect checks cannot starve.
        for _ in 0..INPUT_CAPACITY {
            let Some(packet) = self.rx.as_mut().and_then(|rx| rx.pop().ok()) else {
                break;
            };
            if self.epoch != self.controller.hardware_epoch() {
                self.reset();
            }
            if packet.epoch != self.epoch {
                continue;
            }
            self.event(packet.event);
        }
    }

    fn event(&mut self, event: Event) {
        let channel = match event {
            Event::Note { channel, .. }
            | Event::Sustain { channel, .. }
            | Event::Panic { channel } => channel,
        };
        if self
            .state
            .settings
            .input_channel
            .is_some_and(|filter| filter != channel + 1)
        {
            return;
        }
        let c = usize::from(channel);
        match event {
            Event::Note { key, velocity, .. } => {
                let k = usize::from(key);
                self.down[c][k] = velocity > 0;
                if velocity > 0 {
                    self.sounding[c][k] = true;
                    self.deliver(key, velocity);
                } else if !self.sustain[c] {
                    self.release(c, k);
                }
            }
            Event::Sustain { down, .. } => {
                self.sustain[c] = down;
                if !down {
                    for key in 0..128 {
                        if !self.down[c][key] {
                            self.release(c, key);
                        }
                    }
                }
            }
            Event::Panic { .. } => self.panic(),
        }
    }

    fn release(&mut self, channel: usize, key: usize) {
        if self.sounding[channel][key] {
            self.sounding[channel][key] = false;
            // All accepted MIDI channels feed one destination. Keep the key held
            // until every contributing channel releases it.
            if !self.sounding.iter().any(|keys| keys[key]) {
                self.deliver(key as u8, 0);
            }
        }
    }

    fn deliver(&mut self, key: u8, velocity: u8) {
        if !self.audition.note(self.epoch, key, velocity) {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            self.panic();
            return;
        }
        if self.epoch != self.controller.hardware_epoch() {
            self.reset();
            return;
        }
        let status = if velocity > 0 { 0x90 } else { 0x80 };
        // The engine holds one note per key. Balance output retriggers too,
        // including when several accepted input channels play the same key.
        if velocity > 0 && self.output_keys[usize::from(key)] {
            self.send([0x80 | (self.state.settings.output_channel - 1), key, 0]);
        }
        self.send([
            status | (self.state.settings.output_channel - 1),
            key,
            velocity,
        ]);
        self.output_keys[usize::from(key)] = velocity > 0 && self.output.is_some();
    }
}
