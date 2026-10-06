//! The audio device: a cpal output stream fed by a [`Processor`].

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{
    BufferSize, ErrorKind, FromSample, I24, SampleFormat, SizedSample, StreamConfig,
    SupportedBufferSize, SupportedStreamConfig, U24,
};
use windfall_ipc::{AudioDevice, AudioHost, AudioSettings, EngineStatus};

use crate::controller::Controller;
use crate::mixer::{Frame, MAX_BLOCK};
use crate::processor::Processor;
use crate::shared::Shared;

/// How long a driver gets to bring a stream up before opening counts as
/// failed. Not every driver honors it.
const OPEN_TIMEOUT: Duration = Duration::from_secs(5);

/// Shortest time between two automatic reopens after a device error, so a
/// device that keeps failing cannot spin the device thread.
const RECOVERY_INTERVAL: Duration = Duration::from_secs(1);

/// Device buffers per stretch of time that [`Coverage`] judges as a whole.
const COVERAGE_WINDOW: u32 = 32;

/// Sample rates offered for a device when the driver gives a range.
const STANDARD_RATES: [u32; 13] = [
    8_000, 11_025, 16_000, 22_050, 32_000, 44_100, 48_000, 88_200, 96_000, 176_400, 192_000,
    352_800, 384_000,
];

/// The running audio engine: an output device stream and the processor that
/// feeds it.
///
/// The stream lives on a thread the engine owns, because on some platforms a
/// stream must stay on the thread that opened it. The engine itself can be
/// shared freely between threads. Dropping it closes the stream.
pub struct Engine {
    controller: Controller,
    status: Arc<Mutex<EngineStatus>>,
    requests: Mutex<Sender<Request>>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

enum Request {
    /// Close the stream and open one with these settings. The sender is
    /// signalled when the attempt is over.
    Open(AudioSettings, Sender<()>),
    /// The stream reported that its device is gone or changed. Try the last
    /// settings again.
    Recover,
    Close,
}

impl Engine {
    /// Opens the requested output, or the system default where the settings
    /// leave something out, and starts the stream.
    ///
    /// This never fails and never panics. When no stream could be opened,
    /// [`Engine::status`] has `running` false and `error` says why, and the
    /// controller still accepts every call.
    pub fn start(settings: &AudioSettings) -> Engine {
        let controller = Controller::new();
        let status = Arc::new(Mutex::new(stopped(
            settings,
            "the audio engine is starting",
        )));
        let (requests, inbox) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("windfall-audio-device".to_owned())
            .spawn({
                let controller = controller.clone();
                let status = status.clone();
                let requests = requests.clone();
                move || device_thread(inbox, requests, controller, status)
            })
            .ok();
        let engine = Engine {
            controller,
            status,
            requests: Mutex::new(requests),
            thread: Mutex::new(thread),
        };
        engine.reconfigure(settings);
        engine
    }

    /// Closes the stream and opens one with new settings. Returns once the
    /// attempt is over; [`Engine::status`] tells how it went. The project,
    /// transport settings and playhead carry over, and playback stops.
    pub fn reconfigure(&self, settings: &AudioSettings) {
        let (done, wait) = mpsc::channel();
        let sent = lock(&self.requests)
            .send(Request::Open(settings.clone(), done))
            .is_ok();
        if !sent || wait.recv().is_err() {
            *lock(&self.status) = stopped(settings, "the audio device thread is not running");
        }
    }

    /// What is open right now. `buffer_frames` is the size of the buffers
    /// the device actually asks for, which can differ from the size
    /// requested: WASAPI shared mode, for one, always uses the device period.
    pub fn status(&self) -> EngineStatus {
        let mut status = lock(&self.status).clone();
        let delivered = self.controller.shared().buffer_frames();
        if status.running && delivered > 0 {
            status.buffer_frames = delivered;
            status.latency_ms = latency_ms(delivered, status.sample_rate);
        }
        status
    }

    /// Lists every audio host on this machine with its output devices.
    pub fn devices() -> Vec<AudioHost> {
        catch_unwind(list_hosts).unwrap_or_default()
    }

    /// A handle for driving the engine. It stays valid across
    /// [`Engine::reconfigure`].
    pub fn controller(&self) -> Controller {
        self.controller.clone()
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = lock(&self.requests).send(Request::Close);
        if let Some(thread) = lock(&self.thread).take() {
            let _ = thread.join();
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    // A panic elsewhere must not take the engine's bookkeeping with it.
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn latency_ms(frames: u32, sample_rate: u32) -> f32 {
    if sample_rate == 0 {
        0.0
    } else {
        frames as f32 * 1000.0 / sample_rate as f32
    }
}

/// The status of an engine with no stream, echoing what was asked for.
fn stopped(settings: &AudioSettings, error: impl Into<String>) -> EngineStatus {
    EngineStatus {
        running: false,
        host: settings.host.clone().unwrap_or_default(),
        device: settings.device.clone(),
        sample_rate: settings.sample_rate.unwrap_or(0),
        buffer_frames: settings.buffer_frames.unwrap_or(0),
        latency_ms: 0.0,
        error: Some(error.into()),
    }
}

/// Owns the stream. Everything that opens, closes or drops one happens here.
fn device_thread(
    inbox: Receiver<Request>,
    requests: Sender<Request>,
    controller: Controller,
    status: Arc<Mutex<EngineStatus>>,
) {
    let mut stream: Option<cpal::Stream> = None;
    let mut settings = AudioSettings::default();
    let mut opened_at = Instant::now();
    for request in inbox {
        let done = match request {
            Request::Open(new_settings, done) => {
                settings = new_settings;
                Some(done)
            }
            Request::Recover if opened_at.elapsed() >= RECOVERY_INTERVAL => None,
            Request::Recover => continue,
            Request::Close => break,
        };
        // The old stream goes first: its device may be the one wanted next.
        drop(stream.take());
        controller.detach();
        opened_at = Instant::now();

        let mut report = stopped(&settings, "");
        let attempt = catch_unwind(AssertUnwindSafe(|| {
            open(&settings, &controller, &status, &requests, &mut report)
        }));
        let failure = match attempt {
            Ok(Ok(opened)) => {
                stream = Some(opened);
                None
            }
            Ok(Err(reason)) => Some(reason),
            Err(_) => Some("the audio driver crashed while opening the device".to_owned()),
        };
        if let Some(reason) = failure {
            controller.detach();
            report.error = Some(reason);
            *lock(&status) = report;
        }
        if let Some(done) = done {
            let _ = done.send(());
        }
    }
}

/// Opens and starts a stream. `report` collects the host and device names
/// found along the way, so a failure can still say how far it got.
fn open(
    settings: &AudioSettings,
    controller: &Controller,
    status: &Arc<Mutex<EngineStatus>>,
    requests: &Sender<Request>,
    report: &mut EngineStatus,
) -> Result<cpal::Stream, String> {
    let host = find_host(settings.host.as_deref())?;
    report.host = host.id().name().to_owned();
    let device = find_device(&host, settings.device.as_deref())?;
    let name = device_name(&device);
    report.device = Some(name.clone());

    let default = device
        .default_output_config()
        .map_err(|error| format!("could not read the output format of \"{name}\": {error}"))?;
    let preferred = choose_config(&device, default, settings.sample_rate);
    let buffer = settings
        .buffer_frames
        .map(|frames| fit_buffer(frames, preferred.buffer_size()));

    // Each fallback gives up one request: first the buffer size, then the
    // sample rate.
    let mut attempts = vec![(preferred, buffer)];
    if buffer.is_some() {
        attempts.push((preferred, None));
    }
    if preferred != default {
        attempts.push((default, None));
    }

    let mut last_error = String::new();
    for (config, buffer) in attempts {
        let stream = match build(&device, config, buffer, controller, status, requests) {
            Ok(stream) => stream,
            Err(error) => {
                last_error = error;
                continue;
            }
        };
        let buffer_frames = stream.buffer_size().ok().or(buffer).unwrap_or(0);
        // The status is written before the stream starts, so an error the
        // stream reports right away is not overwritten.
        *lock(status) = EngineStatus {
            running: true,
            host: report.host.clone(),
            device: report.device.clone(),
            sample_rate: config.sample_rate(),
            buffer_frames,
            latency_ms: latency_ms(buffer_frames, config.sample_rate()),
            error: None,
        };
        controller.shared().begin_stream();
        stream
            .play()
            .map_err(|error| format!("could not start the stream on \"{name}\": {error}"))?;
        return Ok(stream);
    }
    Err(format!("could not open \"{name}\": {last_error}"))
}

fn find_host(name: Option<&str>) -> Result<cpal::Host, String> {
    let Some(name) = name else {
        return Ok(cpal::default_host());
    };
    let id = cpal::available_hosts()
        .into_iter()
        .find(|id| id.name().eq_ignore_ascii_case(name))
        .ok_or_else(|| format!("audio host \"{name}\" is not available"))?;
    cpal::host_from_id(id).map_err(|error| format!("could not open audio host \"{name}\": {error}"))
}

fn find_device(host: &cpal::Host, name: Option<&str>) -> Result<cpal::Device, String> {
    let Some(name) = name else {
        return host
            .default_output_device()
            .ok_or_else(|| "no audio output device was found".to_owned());
    };
    host.output_devices()
        .map_err(|error| format!("could not list audio devices: {error}"))?
        .find(|device| device_name(device) == name)
        .ok_or_else(|| format!("audio output device \"{name}\" was not found"))
}

fn device_name(device: &cpal::Device) -> String {
    device
        .description()
        .map(|description| description.name().to_owned())
        .unwrap_or_else(|_| "unknown device".to_owned())
}

/// The device's default format, or its best format at the wanted sample
/// rate when it has one.
fn choose_config(
    device: &cpal::Device,
    default: SupportedStreamConfig,
    sample_rate: Option<u32>,
) -> SupportedStreamConfig {
    let Some(rate) = sample_rate.filter(|rate| *rate != default.sample_rate()) else {
        return default;
    };
    device
        .supported_output_configs()
        .ok()
        .and_then(|configs| {
            configs
                .filter(|range| writable(range.sample_format()))
                .filter(|range| range.contains_rate(rate))
                .max_by(|a, b| a.cmp_default_heuristics(b))
        })
        .and_then(|range| range.try_with_sample_rate(rate))
        .unwrap_or(default)
}

/// Brings a requested buffer size inside what the driver says it accepts.
fn fit_buffer(frames: u32, supported: &SupportedBufferSize) -> u32 {
    match *supported {
        SupportedBufferSize::Range { min, max } if min <= max => frames.clamp(min, max),
        _ => frames,
    }
    .max(1)
}

fn build(
    device: &cpal::Device,
    config: SupportedStreamConfig,
    buffer: Option<u32>,
    controller: &Controller,
    status: &Arc<Mutex<EngineStatus>>,
    requests: &Sender<Request>,
) -> Result<cpal::Stream, String> {
    let format = config.sample_format();
    if !writable(format) {
        return Err(format!("the sample format {format} is not supported"));
    }
    let stream_config = StreamConfig {
        channels: config.channels(),
        sample_rate: config.sample_rate(),
        buffer_size: buffer.map_or(BufferSize::Default, BufferSize::Fixed),
    };
    let mut feeder = Feeder {
        processor: controller.attach(config.sample_rate()),
        shared: controller.shared().clone(),
        channels: usize::from(config.channels()),
        scratch: vec![[0.0; 2]; MAX_BLOCK].into_boxed_slice(),
        coverage: Coverage::new(config.sample_rate()),
    };

    let shared = controller.shared().clone();
    let status = status.clone();
    let controller = controller.clone();
    let requests = requests.clone();
    let on_error = move |error: cpal::Error| match error.kind() {
        ErrorKind::Xrun => shared.record_xrun(),
        // The stream carries on after these.
        ErrorKind::DeviceChanged | ErrorKind::RealtimeDenied => {}
        kind => {
            controller.detach();
            let mut status = lock(&status);
            status.running = false;
            status.error = Some(error.to_string());
            drop(status);
            if matches!(
                kind,
                ErrorKind::DeviceNotAvailable | ErrorKind::StreamInvalidated
            ) {
                let _ = requests.send(Request::Recover);
            }
        }
    };
    device
        .build_output_stream_raw(
            stream_config,
            format,
            move |data: &mut cpal::Data, _: &cpal::OutputCallbackInfo| feeder.fill(data),
            on_error,
            Some(OPEN_TIMEOUT),
        )
        .map_err(|error| error.to_string())
}

/// True for the sample formats [`Feeder::fill`] can write.
fn writable(format: SampleFormat) -> bool {
    format.is_int() || format.is_uint() || format.is_float()
}

/// The device callback: runs the processor and writes its stereo float
/// output in whatever layout and sample format the device wants.
struct Feeder {
    processor: Processor,
    shared: Arc<Shared>,
    /// Channels per frame of the device buffer.
    channels: usize,
    /// Stereo float audio from the processor, before conversion.
    scratch: Box<[Frame]>,
    coverage: Coverage,
}

impl Feeder {
    fn fill(&mut self, data: &mut cpal::Data) {
        let started = Instant::now();
        let frames = data.len() / self.channels;
        if self.coverage.fell_behind(started, frames) {
            self.shared.record_xrun();
        }
        match data.sample_format() {
            SampleFormat::I8 => self.write::<i8>(data),
            SampleFormat::I16 => self.write::<i16>(data),
            SampleFormat::I24 => self.write::<I24>(data),
            SampleFormat::I32 => self.write::<i32>(data),
            SampleFormat::I64 => self.write::<i64>(data),
            SampleFormat::U8 => self.write::<u8>(data),
            SampleFormat::U16 => self.write::<u16>(data),
            SampleFormat::U24 => self.write::<U24>(data),
            SampleFormat::U32 => self.write::<u32>(data),
            SampleFormat::U64 => self.write::<u64>(data),
            SampleFormat::F32 => self.write::<f32>(data),
            SampleFormat::F64 => self.write::<f64>(data),
            // cpal fills the buffer with silence before the callback, so a
            // format that cannot be written stays silent.
            _ => {}
        }
        self.shared
            .record_buffer(frames, started.elapsed(), self.processor.sample_rate());
    }

    /// Fills a device buffer of any length, working through it in pieces
    /// the scratch buffer can hold. Stereo goes to the first two channels
    /// and a mono device gets the average of both sides.
    fn write<T>(&mut self, data: &mut cpal::Data)
    where
        T: SizedSample + FromSample<f32>,
    {
        let Some(out) = data.as_slice_mut::<T>() else {
            return;
        };
        let clip = !T::FORMAT.is_float();
        for block in out.chunks_mut(MAX_BLOCK * self.channels) {
            let frames = block.len() / self.channels;
            let scratch = &mut self.scratch[..frames];
            self.processor.process(scratch.as_flattened_mut());
            if clip {
                for sample in scratch.as_flattened_mut() {
                    *sample = sample.clamp(-1.0, 1.0);
                }
            }
            if self.channels == 1 {
                for (out, frame) in block.iter_mut().zip(scratch.iter()) {
                    *out = T::from_sample((frame[0] + frame[1]) * 0.5);
                }
            } else {
                let device_frames = block.chunks_exact_mut(self.channels);
                for (out, frame) in device_frames.zip(scratch.iter()) {
                    out[0] = T::from_sample(frame[0]);
                    out[1] = T::from_sample(frame[1]);
                }
            }
        }
    }
}

/// Notices when the device asks for less audio than the time that passes.
///
/// A device that is served on time asks for one second of audio every
/// second. When a buffer comes too late the device runs dry, and the time it
/// spent with nothing to play is never asked for: the count of frames
/// requested drops behind the clock and stays there. That lasting drop is
/// what this looks for, and it catches dropouts no driver reports, such as
/// a callback the system scheduled too late.
struct Coverage {
    sample_rate: f64,
    started: Option<Instant>,
    /// Frames asked for before the buffer being looked at.
    frames: u64,
    /// How far ahead of the clock the request count was at its best in the
    /// current window of buffers. A buffer that is merely late pulls its
    /// own reading down, so the best of a window is the on-time figure.
    best: f64,
    /// The same figure for the window before.
    previous_best: Option<f64>,
    buffers: u32,
}

impl Coverage {
    fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate: f64::from(sample_rate),
            started: None,
            frames: 0,
            best: f64::NEG_INFINITY,
            previous_best: None,
            buffers: 0,
        }
    }

    /// Takes note of a buffer of `frames` frames asked for at `now`. Returns
    /// true when this buffer completes a window in which the requests fell
    /// behind the clock.
    fn fell_behind(&mut self, now: Instant, frames: usize) -> bool {
        let started = *self.started.get_or_insert(now);
        let due = now.duration_since(started).as_secs_f64() * self.sample_rate;
        self.best = self.best.max(self.frames as f64 - due);
        self.frames += frames as u64;
        self.buffers += 1;
        if self.buffers < COVERAGE_WINDOW {
            return false;
        }
        // Half a buffer, and never less than a millisecond, is well clear of
        // timing jitter and of the drift between the system clock and the
        // device's own over one window.
        let tolerance = (frames as f64 / 2.0).max(self.sample_rate / 1000.0);
        let behind = self
            .previous_best
            .is_some_and(|previous| self.best < previous - tolerance);
        self.previous_best = Some(self.best);
        self.best = f64::NEG_INFINITY;
        self.buffers = 0;
        behind
    }
}

fn list_hosts() -> Vec<AudioHost> {
    let default_host = cpal::default_host().id();
    cpal::available_hosts()
        .into_iter()
        .filter_map(|id| {
            let host = cpal::host_from_id(id).ok()?;
            // The default device is a handle that follows the system
            // setting, so it is matched to a listed device by id.
            let default_id = host
                .default_output_device()
                .and_then(|device| device.id().ok());
            let devices = host
                .output_devices()
                .ok()?
                .map(|device| {
                    let is_default = default_id.is_some() && device.id().ok() == default_id;
                    describe(&device, is_default)
                })
                .collect();
            Some(AudioHost {
                name: id.name().to_owned(),
                is_default: id == default_host,
                devices,
            })
        })
        .collect()
}

fn describe(device: &cpal::Device, is_default: bool) -> AudioDevice {
    let mut sample_rates = Vec::new();
    for range in device.supported_output_configs().into_iter().flatten() {
        sample_rates.extend(
            STANDARD_RATES
                .into_iter()
                .filter(|rate| range.contains_rate(*rate)),
        );
    }
    sample_rates.sort_unstable();
    sample_rates.dedup();
    // Buffer sizes are in frames, so a range only means something at one
    // sample rate. The one reported is for the device's default format.
    let buffer_range =
        device
            .default_output_config()
            .ok()
            .and_then(|config| match *config.buffer_size() {
                SupportedBufferSize::Range { min, max } => Some((min, max)),
                SupportedBufferSize::Unknown => None,
            });
    AudioDevice {
        name: device_name(device),
        is_default,
        sample_rates,
        min_buffer_frames: buffer_range.map(|(low, _)| low),
        max_buffer_frames: buffer_range.map(|(_, high)| high),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feeds `Coverage` buffers of 480 frames at 48 kHz, nominally one every
    /// 10 ms, each `delay_ms(index)` later than that, and counts the windows
    /// in which it saw the requests fall behind.
    fn gaps(delay_ms: impl Fn(u32) -> f64, buffers: u32) -> usize {
        let mut coverage = Coverage::new(48_000);
        let start = Instant::now();
        (0..buffers)
            .filter(|&index| {
                let at = f64::from(index) * 0.01 + delay_ms(index) / 1000.0;
                coverage.fell_behind(start + Duration::from_secs_f64(at), 480)
            })
            .count()
    }

    #[test]
    fn buffers_served_on_time_leave_no_gap() {
        assert_eq!(gaps(|_| 0.0, 1_000), 0);
    }

    #[test]
    fn late_buffers_that_catch_up_are_not_a_gap() {
        // Most buffers arrive a few milliseconds late, some on time.
        assert_eq!(gaps(|index| f64::from(index % 5) * 1.5, 1_000), 0);
    }

    #[test]
    fn a_slow_device_clock_is_not_a_gap() {
        // The device runs a tenth of a percent slow against the system.
        assert_eq!(gaps(|index| f64::from(index) * 0.01, 5_000), 0);
    }

    #[test]
    fn time_the_device_never_asked_for_is_a_gap() {
        // From the 500th buffer on everything is 30 ms behind: the device
        // sat dry for that long and never asked for those frames.
        let stalled = |index| if index >= 500 { 30.0 } else { 0.0 };
        assert_eq!(gaps(stalled, 1_000), 1);
    }

    #[test]
    fn a_requested_buffer_size_is_kept_inside_the_range_the_driver_gives() {
        let range = SupportedBufferSize::Range { min: 480, max: 480 };
        assert_eq!(fit_buffer(128, &range), 480);
        assert_eq!(fit_buffer(128, &SupportedBufferSize::Unknown), 128);
        assert_eq!(fit_buffer(0, &SupportedBufferSize::Unknown), 1);
    }
}
