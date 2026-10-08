//! Per-instance process/control owner. No method here belongs on audio.

use super::{
    adapter::{Audio, ParameterSpec, Signals},
    control::{ControlParameter, Decoder, Message, Owner, Packet},
    mapping::Mapping,
    protocol::{Config, Identity},
    slots::Region,
};
use std::{
    io,
    net::{TcpListener, TcpStream},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

static SESSIONS: AtomicU64 = AtomicU64::new(1);
pub fn fresh_identity(token: u64, revision: u64, binding: u64) -> io::Result<Identity> {
    // Control-side CAS loop keeps the workspace's Rust 1.90 API baseline and
    // latches exhaustion without ever wrapping a session component.
    let mut serial = SESSIONS.load(Ordering::Acquire);
    loop {
        if serial > u32::MAX as u64 {
            return Err(io::Error::other("bridge session counter exhausted"));
        }
        match SESSIONS.compare_exchange(serial, serial + 1, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => break,
            Err(current) => serial = current,
        }
    }
    let session = (u64::from(std::process::id()) << 32) | serial;
    if serial > u32::MAX as u64 || token == 0 {
        return Err(io::Error::other("bridge session/token exhausted"));
    }
    Ok(Identity {
        session,
        token,
        revision,
        binding,
    })
}

#[derive(Clone)]
pub struct Launch {
    pub helper: PathBuf,
    pub plugin: PathBuf,
    pub id: String,
    pub format: String,
    pub approved_binary: crate::paths::PluginFileIdentity,
    pub config: Config,
    pub parameters: Vec<ParameterSpec>,
    pub state: Vec<u8>,
    pub offline: bool,
    pub startup_timeout: Duration,
    pub audio_timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct Captured {
    pub state: Vec<u8>,
    pub parameters: Vec<ControlParameter>,
    pub processed_generation: u64,
    /// Inactive state reconciliation, distinct from completed native DSP.
    pub reconciled_generation: u64,
    pub epoch: u64,
}
#[derive(Debug, Clone, Default)]
pub struct Status {
    pub process_id: u32,
    pub failed: bool,
    pub error: Option<String>,
    pub reaped: bool,
}
enum Job {
    Request {
        body: Box<Message>,
        deadline: Instant,
        cancelled: Arc<AtomicBool>,
        response: mpsc::SyncSender<Result<Packet, String>>,
    },
    Stop,
}
struct Inner {
    jobs: mpsc::SyncSender<Job>,
    worker: Mutex<Option<JoinHandle<()>>>,
    status: Arc<Mutex<Status>>,
    signals: Arc<Signals>,
    owner: Owner,
    last_state: Mutex<Option<Captured>>,
}
impl Drop for Inner {
    fn drop(&mut self) {
        // All facade/control references retire away from the callback. Queue
        // fullness cannot prevent stop: disconnect also stops the worker.
        let _ = self.jobs.try_send(Job::Stop);
        self.signals.retired.store(true, Ordering::Release);
        self.signals.failed.store(true, Ordering::Release);
        if let Some(worker) = self
            .worker
            .get_mut()
            .unwrap_or_else(|error| error.into_inner())
            .take()
        {
            let _ = worker.join();
        }
    }
}
#[derive(Clone)]
pub struct Control(Arc<Inner>);
impl Control {
    pub fn status(&self) -> Status {
        self.0
            .status
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }
    pub fn owner(&self) -> Owner {
        self.0.owner
    }
    pub fn last_valid_state(&self) -> Option<Captured> {
        self.0
            .last_state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }
    fn call(&self, body: Message, timeout: Duration) -> Result<Packet, String> {
        if self.0.signals.failed.load(Ordering::Acquire) {
            return Err(self
                .status()
                .error
                .unwrap_or_else(|| "audio helper is unavailable".into()));
        }
        let (response, receiver) = mpsc::sync_channel(1);
        let cancelled = Arc::new(AtomicBool::new(false));
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or("invalid bridge control deadline")?;
        self.0
            .jobs
            .try_send(Job::Request {
                body: Box::new(body),
                deadline,
                cancelled: cancelled.clone(),
                response,
            })
            .map_err(|_| "audio helper control queue is full or stopped".to_owned())?;
        match receiver.recv_timeout(timeout) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                cancelled.store(true, Ordering::Release);
                // Join native work or its confirmed process termination before
                // a session caller can release its recording exclusion guard.
                receiver
                    .recv()
                    .unwrap_or_else(|_| Err("audio helper stopped while cancelling control".into()))
            }
            Err(_) => Err("audio helper control worker stopped".into()),
        }
    }
    pub fn capture(
        &self,
        epoch: u64,
        desired_generation: u64,
        pending: &[ParameterSpec],
        timeout: Duration,
    ) -> Result<Captured, String> {
        let packet = self.call(
            Message::Capture {
                epoch,
                desired_generation,
                pending: pending.iter().copied().map(Into::into).collect(),
            },
            timeout,
        )?;
        let captured = match packet.body {
            Message::Captured {
                epoch: reply_epoch,
                processed_generation,
                reconciled_generation,
                parameters,
            } if reply_epoch == epoch
                && processed_generation <= desired_generation
                && reconciled_generation <= desired_generation =>
            {
                if parameters
                    .iter()
                    .any(|parameter| !ParameterSpec::from(*parameter).valid())
                    || packet.state.is_empty()
                {
                    return Err("malformed native capture response".into());
                }
                Captured {
                    epoch,
                    state: packet.state,
                    parameters,
                    processed_generation,
                    reconciled_generation,
                }
            }
            Message::Error { message } => return Err(message),
            _ => return Err("stale or unsupported native capture response".into()),
        };
        *self
            .0
            .last_state
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(captured.clone());
        Ok(captured)
    }
    pub fn editor(&self, open: bool) -> Result<(), String> {
        let packet = self.call(Message::Editor { open }, Duration::from_secs(2))?;
        match packet.body {
            Message::Error { message } => Err(message),
            _ => Err("unexpected editor bridge response".into()),
        }
    }
}

struct Process(Child);
impl Process {
    fn reap(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        self.reap();
    }
}

/// Startup is synchronous only on the factory/control caller; callback receives
/// an already negotiated, allocated audio half. Production helper discovery is
/// the desktop executable, not a Cargo target lookup.
pub fn launch(options: Launch) -> Result<(Control, Audio, usize), String> {
    options
        .config
        .validate()
        .map_err(|error| error.to_string())?;
    if crate::paths::plugin_file_identity(&options.plugin).map_err(|error| error.to_string())?
        != options.approved_binary
    {
        return Err("approved native plugin binary changed before bridge launch".into());
    }
    if options.config.native_latency != 0
        || !options.helper.is_file()
        || options.parameters.len() > super::protocol::PARAM_CAPACITY
        || options.startup_timeout.is_zero()
        || options.audio_timeout.is_zero()
    {
        return Err("invalid audio helper launch configuration".into());
    }
    let identity = options.config.identity;
    let name = format!(
        "Local\\Windfall-Audio-{:016x}-{:016x}",
        identity.session, identity.token
    );
    let mapping = Mapping::create(&name).map_err(|error| error.to_string())?;
    let mut region =
        Region::initialize(mapping, options.config).map_err(|error| error.to_string())?;
    let listener =
        TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).map_err(|error| error.to_string())?;
    listener
        .set_nonblocking(true)
        .map_err(|error| error.to_string())?;
    let mut command = Command::new(&options.helper);
    command
        .arg("--windfall-audio-helper")
        .arg(
            listener
                .local_addr()
                .map_err(|error| error.to_string())?
                .to_string(),
        )
        .arg(identity.session.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let mut process = Process(command.spawn().map_err(|error| error.to_string())?);
    let deadline = Instant::now() + options.startup_timeout;
    let mut socket = loop {
        match listener.accept() {
            Ok((socket, _)) => break socket,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(error) => return Err(error.to_string()),
        }
        if process
            .0
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_some()
        {
            return Err("audio helper exited during startup".into());
        }
        if Instant::now() >= deadline {
            return Err("audio helper startup timed out".into());
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    socket
        .set_nodelay(true)
        .map_err(|error| error.to_string())?;
    socket
        .set_write_timeout(Some(options.startup_timeout))
        .map_err(|error| error.to_string())?;
    let owner: Owner = identity.into();
    let mut load = Packet::new(
        1,
        owner,
        Message::Load {
            mapping: name,
            settings: options.config.into(),
            path: options.plugin.to_string_lossy().into_owned(),
            id: options.id,
            format: options.format,
            approved_binary: options.approved_binary,
            parameters: options.parameters.iter().copied().map(Into::into).collect(),
            offline: options.offline,
        },
    );
    load.state = options.state;
    load.write(&mut socket).map_err(|error| error.to_string())?;
    socket
        .set_nonblocking(true)
        .map_err(|error| error.to_string())?;
    let mut decoder = Decoder::default();
    let ready = wait_reply(
        &mut process,
        &mut socket,
        &mut decoder,
        owner,
        1,
        deadline,
        None,
    )?;
    let Message::Ready {
        native_latency,
        tail,
        parameters,
    } = ready.body
    else {
        return Err("audio helper did not negotiate activation".into());
    };
    let parameters: Vec<ParameterSpec> = parameters.into_iter().map(Into::into).collect();
    region
        .negotiate_latency(native_latency as usize)
        .map_err(|error| error.to_string())?;
    let signals = Arc::new(Signals::default());
    let audio = Audio::new(region.clone(), signals.clone(), &parameters)
        .map_err(|error| error.to_string())?;
    let status = Arc::new(Mutex::new(Status {
        process_id: process.0.id(),
        ..Default::default()
    }));
    let (jobs, incoming) = mpsc::sync_channel(8);
    let worker_status = status.clone();
    let worker_signals = signals.clone();
    let worker = std::thread::Builder::new()
        .name("plugin-bridge-supervisor".into())
        .spawn(move || {
            let result = supervise(
                &mut process,
                &mut socket,
                &mut decoder,
                owner,
                incoming,
                Supervision {
                    region,
                    signals: worker_signals.clone(),
                    audio_timeout: options.audio_timeout,
                },
            );
            worker_signals.failed.store(true, Ordering::Release);
            process.reap();
            let mut status = worker_status
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            status.reaped = true;
            if let Err(error) = result {
                status.failed = true;
                status.error = Some(error);
            }
        })
        .map_err(|error| error.to_string())?;
    Ok((
        Control(Arc::new(Inner {
            jobs,
            worker: Mutex::new(Some(worker)),
            status,
            signals,
            owner,
            last_state: Mutex::new(None),
        })),
        audio,
        tail.min(usize::MAX as u64) as usize,
    ))
}

fn wait_reply(
    process: &mut Process,
    socket: &mut TcpStream,
    decoder: &mut Decoder,
    owner: Owner,
    request: u64,
    deadline: Instant,
    cancelled: Option<&AtomicBool>,
) -> Result<Packet, String> {
    loop {
        if Instant::now() >= deadline || cancelled.is_some_and(|flag| flag.load(Ordering::Acquire))
        {
            process.reap();
            return Err("audio helper control deadline exceeded; process terminated".into());
        }
        if process
            .0
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_some()
        {
            return Err("audio helper process exited".into());
        }
        if let Some(packet) = decoder.poll(socket).map_err(|error| error.to_string())? {
            if packet.request != request || packet.owner != owner {
                return Err("stale audio helper control response".into());
            }
            return Ok(packet);
        }
        std::thread::sleep(Duration::from_micros(200));
    }
}
struct Supervision {
    region: Region,
    signals: Arc<Signals>,
    audio_timeout: Duration,
}
fn supervise(
    process: &mut Process,
    socket: &mut TcpStream,
    decoder: &mut Decoder,
    owner: Owner,
    incoming: mpsc::Receiver<Job>,
    context: Supervision,
) -> Result<(), String> {
    let Supervision {
        region,
        signals,
        audio_timeout,
    } = context;
    let mut request = 1u64;
    let mut pending: Option<(u64, Instant)> = None;
    loop {
        if signals.retired.load(Ordering::Acquire) {
            return Ok(());
        }
        if signals.failed.load(Ordering::Acquire) || region.helper_failed() {
            return Err("audio helper or callback latched a processing failure".into());
        }
        if process
            .0
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_some()
        {
            return Err("audio helper process exited".into());
        }
        match incoming.try_recv() {
            Ok(Job::Stop) | Err(mpsc::TryRecvError::Disconnected) => return Ok(()),
            Ok(Job::Request {
                body,
                deadline,
                cancelled,
                response,
            }) => {
                if cancelled.load(Ordering::Acquire) || Instant::now() >= deadline {
                    let _ = response.send(Err("queued audio helper control cancelled".into()));
                    continue;
                }
                request = request
                    .checked_add(1)
                    .ok_or("bridge request counter exhausted")?;
                socket
                    .set_nonblocking(false)
                    .map_err(|error| error.to_string())?;
                socket
                    .set_write_timeout(Some(deadline.saturating_duration_since(Instant::now())))
                    .map_err(|error| error.to_string())?;
                let write = Packet::new(request, owner, *body)
                    .write(socket)
                    .map_err(|error| error.to_string());
                socket
                    .set_nonblocking(true)
                    .map_err(|error| error.to_string())?;
                let result = write.and_then(|()| {
                    wait_reply(
                        process,
                        socket,
                        decoder,
                        owner,
                        request,
                        deadline,
                        Some(&cancelled),
                    )
                });
                let failed = result.as_ref().err().cloned();
                let _ = response.send(result);
                if let Some(error) = failed {
                    return Err(error);
                }
                pending = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
        if let Some(sequence) = region.pending_sequence() {
            if pending.is_none_or(|(previous, _)| previous != sequence) {
                pending = Some((sequence, Instant::now()));
            }
            if pending.is_some_and(|(_, started)| started.elapsed() >= audio_timeout) {
                return Err("audio helper stalled; process terminated".into());
            }
        } else {
            pending = None;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}
