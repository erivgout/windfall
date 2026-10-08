//! Windows plugin owner thread. Native instances never leave this thread;
//! only prepared audio adapters cross into the engine.

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};
use windfall_engine::plugins::{HostedEffect, HostedInstrument, PluginFactory};
use windfall_plugin_host::{
    PluginEffect, PluginHost, PluginInstance, PluginInstrument, PluginNotification, PluginState,
    ownership::{AudioOwnership, ControlOwnership, exchange},
};
use windfall_project::{PluginBinding, PluginTarget, Project};

type Job = Box<dyn FnOnce(&mut Owner) + Send>;
/// A control caller can cancel queued work, but joins work already running.
/// Its recording guard therefore outlives every native call and recovery.
#[derive(Default)]
struct JobLifetime {
    // Queued, running, completed/deliverable, cancelled queued, cancelled running.
    phase: std::sync::atomic::AtomicU8,
}
impl JobLifetime {
    fn cancelled(&self) -> bool {
        matches!(self.phase.load(std::sync::atomic::Ordering::Acquire), 3 | 4)
    }
}
type Approved = Arc<Mutex<BTreeMap<(String, String), (u64, Option<std::time::SystemTime>)>>>;
type Selection =
    Arc<Mutex<std::collections::HashMap<PluginTarget, Arc<std::sync::atomic::AtomicU64>>>>;
fn selected(selection: &Selection, target: PluginTarget) -> Option<u64> {
    selection
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .get(&target)
        .map(|token| token.load(std::sync::atomic::Ordering::Relaxed))
        .filter(|token| *token != 0)
}
#[derive(Clone)]
pub enum Update {
    Parameter {
        target: PluginTarget,
        id: u32,
        value: f32,
        gesture: u64,
    },
    Capture {
        target: PluginTarget,
        serial: u64,
    },
}
#[derive(Clone)]
pub struct PendingUpdate {
    pub binding: u64,
    pub token: u64,
    pub revision: u64,
    pub update: Update,
}
pub(crate) struct CapturedState {
    pub bytes: Vec<u8>,
    pub parameters: Vec<(u32, f32)>,
    pub restart: bool,
    serial: u64,
}
struct Instance {
    identity: u64,
    target: PluginTarget,
    plugin: PluginInstance,
    binding: PluginBinding,
    revision: u64,
    playback: bool,
    ownership: ControlOwnership<Adapter>,
    rate: u32,
    block: usize,
    instrument: bool,
    latency: usize,
    tail: usize,
    returned: Option<Adapter>,
    dirty: bool,
    dirty_serial: u64,
    capture_sent: bool,
    restart: bool,
    notifications: Vec<PluginNotification>,
}
struct OwnerCapture {
    state: PluginState,
    recovery_error: Option<String>,
}

impl Instance {
    fn retire(&mut self, adapter: Adapter) {
        // Even refusal tears down only after the returned audio half is
        // destroyed on this owner. It must never be queued for resume here.
        let _ = self.discard(adapter);
    }
    fn discard(&mut self, adapter: Adapter) -> Result<(), String> {
        match adapter {
            Adapter::Effect(adapter) => self
                .plugin
                .release_effect(adapter)
                .map_err(|error| error.error.to_string()),
            Adapter::Instrument(adapter) => self
                .plugin
                .release_instrument(adapter)
                .map_err(|error| error.error.to_string()),
        }
    }
    fn release(&mut self, adapter: Adapter) -> Result<(), String> {
        let result = match adapter {
            Adapter::Effect(adapter) => self
                .plugin
                .release_effect(adapter)
                .map_err(|error| (error.error.to_string(), Adapter::Effect(error.returned))),
            Adapter::Instrument(adapter) => self
                .plugin
                .release_instrument(adapter)
                .map_err(|error| (error.error.to_string(), Adapter::Instrument(error.returned))),
        };
        match result {
            Ok(()) => Ok(()),
            Err((error, adapter)) => {
                self.resume_returned(adapter);
                Err(error)
            }
        }
    }

    fn resume_returned(&mut self, adapter: Adapter) {
        self.ownership.cancel();
        self.returned = self.ownership.resume(adapter).err();
    }

    fn restore(&mut self) -> Result<(), String> {
        let adapter = if self.instrument {
            Adapter::Instrument(
                self.plugin
                    .prepare_instrument(self.rate as f32, self.block)
                    .map_err(|error| error.to_string())?,
            )
        } else {
            Adapter::Effect(
                self.plugin
                    .prepare_effect(self.rate as f32, self.block)
                    .map_err(|error| error.to_string())?,
            )
        };
        if adapter.latency() != self.latency || adapter.tail() != self.tail {
            self.discard(adapter)?;
            return Err(
                "Plugin latency/tail changed during state capture; retry the plugin".into(),
            );
        }
        if let Err(adapter) = self.ownership.resume(adapter) {
            self.discard(adapter)?;
            return Err("The plugin resume queue is unexpectedly full".into());
        }
        Ok(())
    }

    /// Owner-thread wait only. Audio returns at a boundary without waiting on
    /// this thread or any session lock. A timeout leaves active state untouched.
    fn capture(
        &mut self,
        lifetime: &JobLifetime,
        rebuilding: bool,
    ) -> Result<OwnerCapture, String> {
        if lifetime.cancelled() {
            return Err("Plugin capture was cancelled".into());
        }
        if self.binding.format == "clap" {
            return self
                .plugin
                .save_state()
                .map(|state| OwnerCapture {
                    state,
                    recovery_error: None,
                })
                .map_err(|error| error.to_string());
        }
        self.ownership.request();
        let deadline = std::time::Instant::now() + Duration::from_millis(500);
        let adapter = loop {
            if lifetime.cancelled() {
                self.ownership.cancel();
                return Err("Plugin capture was cancelled".into());
            }
            if let Some(adapter) = self.ownership.take_returned() {
                break adapter;
            }
            if std::time::Instant::now() >= deadline {
                self.ownership.cancel();
                return Err("Plugin state capture needs a live audio block boundary; the processor was not returned".into());
            }
            std::thread::sleep(Duration::from_millis(1));
        };
        if lifetime.cancelled() {
            self.resume_returned(adapter);
            return Err("Plugin capture was cancelled".into());
        }
        let failed = adapter.failed();
        self.release(adapter)?;
        if failed {
            self.ownership.cancel();
            return Err(
                "A failed plugin remains bypassed/silent; retry before capturing state".into(),
            );
        }
        // Deactivation can leave controller edits, rescans and dirty flags.
        // Service them while inactive, before serializing. They are included
        // in this capture rather than scheduling an endless dirty-capture loop.
        self.plugin.idle(&mut |notification| match notification {
            PluginNotification::StateChanged => {
                self.dirty = true;
                self.dirty_serial = self.dirty_serial.saturating_add(1);
            }
            PluginNotification::RestartRequested
            | PluginNotification::ParamsRescanned
            | PluginNotification::LatencyChanged { .. } => {
                self.restart = true;
                self.dirty = true;
                self.dirty_serial = self.dirty_serial.saturating_add(1);
            }
            notification => self.notifications.push(notification),
        });
        let state = if lifetime.cancelled() {
            Err("Plugin capture was cancelled".into())
        } else {
            self.plugin.save_state().map_err(|error| error.to_string())
        };
        // Even a failed save must give the sounding instance back when safe.
        let restore = self.restore();
        let recovery_error = match restore {
            Ok(()) => None,
            // A requested restart will install a new plan and its correct
            // latency buffers. Preserve proven inactive state for that plan;
            // never resume an adapter with changed metadata into the old slot.
            Err(error) if rebuilding && self.restart && state.is_ok() => Some(error),
            Err(error) => return Err(error),
        };
        if lifetime.cancelled() {
            return Err("Plugin capture was cancelled".into());
        }
        state.map(|state| OwnerCapture {
            state,
            recovery_error,
        })
    }
}
pub(crate) fn binding_identity(binding: &PluginBinding) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    binding.format.hash(&mut hash);
    binding.path.hash(&mut hash);
    binding.id.hash(&mut hash);
    binding.state.hash(&mut hash);
    hash.finish()
}
struct Owner {
    selection: Selection,
    host: PluginHost,
    instances: BTreeMap<u64, Instance>,
    next: u64,
    approved: Approved,
}

/// Control-side handle; synchronous requests are never made by audio processing.
#[derive(Clone)]
pub struct Runtime {
    selection: Selection,
    jobs: mpsc::Sender<Job>,
    errors: Arc<Mutex<Vec<(PluginTarget, String)>>>,
    approved: Approved,
    updates: Arc<Mutex<mpsc::Receiver<PendingUpdate>>>,
    revision: Arc<std::sync::atomic::AtomicU64>,
    next_revision: Arc<std::sync::atomic::AtomicU64>,
    prepared_revision: Option<u64>,
    rendering: bool,
}
impl std::fmt::Debug for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginRuntime").finish_non_exhaustive()
    }
}
impl Runtime {
    pub fn new() -> Result<Self, String> {
        if !cfg!(windows) {
            return Err("Native plugin integration currently requires Windows".into());
        }
        let (jobs, receiver) = mpsc::channel::<Job>();
        let errors = Arc::new(Mutex::new(Vec::new()));
        let approved: Approved = Default::default();
        let owner_approved = approved.clone();
        let (updates, update_receiver) = mpsc::sync_channel(4096);
        let revision = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let owner_revision = revision.clone();
        let selection: Selection = Default::default();
        let owner_selection = selection.clone();
        std::thread::Builder::new()
            .name("plugin-owner".into())
            .spawn(move || {
                let mut owner = Owner {
                    selection: owner_selection,
                    host: PluginHost::windfall(),
                    instances: BTreeMap::new(),
                    next: 0,
                    approved: owner_approved,
                };
                let mut gestures = std::collections::HashMap::new();
                let mut gesture = 1_u64 << 63;
                loop {
                    match receiver.recv_timeout(Duration::from_millis(10)) {
                        Ok(job) => job(&mut owner),
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }
                    windfall_plugin_host::gui::pump_events(Some(Duration::ZERO));
                    let selection = owner.selection.clone();
                    for (token, record) in &mut owner.instances {
                        // A cancellation can race the callback's return. Always
                        // service late returns, including unselected/stale units.
                        if let Some(adapter) = record
                            .returned
                            .take()
                            .or_else(|| record.ownership.take_returned())
                        {
                            // Cancelled captures resume the exact active adapter.
                            // Notification maintenance never quiesces native work.
                            record.resume_returned(adapter);
                        }
                        let target = &record.target;
                        let instance = &mut record.plugin;
                        let current = record.playback
                            && selected(&selection, *target) == Some(*token)
                            && record.revision
                                == owner_revision.load(std::sync::atomic::Ordering::Relaxed);
                        let send = |update| {
                            if current {
                                let _ = updates.try_send(PendingUpdate {
                                    binding: record.identity,
                                    token: *token,
                                    revision: record.revision,
                                    update,
                                });
                            }
                        };
                        let mut state_changed = false;
                        let mut restart = false;
                        let mut notify = |notification, captured| match notification {
                            PluginNotification::ParamGestureBegin { id } => {
                                gesture = gesture.wrapping_add(1);
                                gestures.insert((*token, id), gesture);
                            }
                            PluginNotification::ParamGestureEnd { id } => {
                                gestures.remove(&(*token, id));
                            }
                            PluginNotification::ParamChanged { id, value } => {
                                if let Some(gesture) = gestures.get(&(*token, id)) {
                                    send(Update::Parameter {
                                        target: *target,
                                        id,
                                        value: value as f32,
                                        gesture: *gesture,
                                    });
                                } else if captured {
                                    if record
                                        .binding
                                        .parameters
                                        .iter()
                                        .any(|param| param.id == id && !param.read_only)
                                    {
                                        send(Update::Parameter {
                                            target: *target,
                                            id,
                                            value: value as f32,
                                            gesture: 0,
                                        });
                                    }
                                } else if record
                                    .binding
                                    .parameters
                                    .iter()
                                    .any(|param| param.id == id && !param.read_only)
                                {
                                    state_changed = true;
                                }
                            }
                            PluginNotification::StateChanged => state_changed = true,
                            PluginNotification::LatencyChanged { .. }
                            | PluginNotification::RestartRequested
                            | PluginNotification::ParamsRescanned => {
                                state_changed = true;
                                restart = true;
                            }
                            _ => {}
                        };
                        for notification in record.notifications.drain(..) {
                            notify(notification, true);
                        }
                        instance.idle(&mut |notification| notify(notification, false));
                        if state_changed {
                            record.dirty = true;
                            record.dirty_serial = record.dirty_serial.saturating_add(1);
                            record.restart |= restart;
                        }
                        if current && record.dirty && !record.capture_sent {
                            record.capture_sent = updates
                                .try_send(PendingUpdate {
                                    binding: record.identity,
                                    token: *token,
                                    revision: record.revision,
                                    update: Update::Capture {
                                        target: *target,
                                        serial: record.dirty_serial,
                                    },
                                })
                                .is_ok();
                        }
                    }
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(Self {
            selection,
            jobs,
            errors,
            approved,
            updates: Arc::new(Mutex::new(update_receiver)),
            revision,
            next_revision: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            prepared_revision: None,
            rendering: false,
        })
    }
    pub fn approve(&self, entries: &[windfall_ipc::PluginEntry]) {
        let mut approved = self
            .approved
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        approved.clear();
        for entry in entries.iter().filter(|entry| entry.usable) {
            if let Ok(metadata) = std::fs::metadata(&entry.path) {
                approved.insert(
                    (entry.path.clone(), entry.id.clone()),
                    (metadata.len(), metadata.modified().ok()),
                );
            }
        }
    }
    pub fn drain(&self) -> Vec<PendingUpdate> {
        self.updates
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .try_iter()
            .take(4096)
            .filter(|update| self.is_current(update.revision, update.token))
            .collect()
    }
    pub fn is_current(&self, revision: u64, token: u64) -> bool {
        revision == self.revision.load(std::sync::atomic::Ordering::Relaxed)
            && self
                .selection
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .values()
                .any(|selected| selected.load(std::sync::atomic::Ordering::Relaxed) == token)
    }
    pub fn document_revision(&self) -> u64 {
        self.revision.load(std::sync::atomic::Ordering::Relaxed)
    }
    #[cfg(all(test, windows))]
    pub(crate) fn selected_token(&self, target: PluginTarget) -> Option<u64> {
        selected(&self.selection, target)
    }
    fn selection(&self, target: PluginTarget) -> Option<Arc<std::sync::atomic::AtomicU64>> {
        if self.rendering {
            None
        } else {
            Some(
                self.selection
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .entry(target)
                    .or_default()
                    .clone(),
            )
        }
    }
    pub fn retry(&self) {
        let revision = self
            .next_revision
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        self.revision
            .store(revision, std::sync::atomic::Ordering::Relaxed);
    }
    /// Prepare a replacement without invalidating the installed document.
    pub fn prepare_document(&self) -> Arc<Self> {
        let mut staged = self.clone();
        staged.prepared_revision = Some(
            self.next_revision
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                + 1,
        );
        Arc::new(staged)
    }
    /// Call only after a replacement has passed its guards and been installed.
    pub fn install_document(&self) {
        if let Some(revision) = self.prepared_revision {
            self.revision
                .store(revision, std::sync::atomic::Ordering::Relaxed);
        }
    }
    fn preparation_revision(&self) -> u64 {
        self.prepared_revision
            .unwrap_or_else(|| self.revision.load(std::sync::atomic::Ordering::Relaxed))
    }
    fn call<T: Send + 'static>(
        &self,
        job: impl FnOnce(&mut Owner) -> Result<T, String> + Send + 'static,
    ) -> Result<T, String> {
        self.call_with_lifetime(Duration::from_secs(15), move |owner, _| job(owner))
    }
    fn call_with_lifetime<T: Send + 'static>(
        &self,
        timeout: Duration,
        job: impl FnOnce(&mut Owner, &JobLifetime) -> Result<T, String> + Send + 'static,
    ) -> Result<T, String> {
        use std::sync::atomic::Ordering;
        let (sender, receiver) = mpsc::sync_channel(1);
        let lifetime = Arc::new(JobLifetime::default());
        let owner_lifetime = lifetime.clone();
        self.jobs
            .send(Box::new(move |owner| {
                if owner_lifetime
                    .phase
                    .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
                    .is_err()
                {
                    return;
                }
                let result = job(owner, &owner_lifetime);
                if owner_lifetime
                    .phase
                    .compare_exchange(1, 2, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    let _ = sender.send(result);
                } else {
                    // Cancellation won the completion race. Native-bearing
                    // results must be destroyed here, on their creating owner.
                    drop(result);
                    let _ = sender.send(Err(
                        "The plugin owner request timed out and was cancelled".into(),
                    ));
                }
            }))
            .map_err(|_| "The plugin owner stopped".to_owned())?;
        match receiver.recv_timeout(timeout) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Disconnected) => Err("The plugin owner stopped".into()),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                loop {
                    let phase = lifetime.phase.load(Ordering::Acquire);
                    match phase {
                        0 | 1 => {
                            if lifetime
                                .phase
                                .compare_exchange(
                                    phase,
                                    phase + 3,
                                    Ordering::AcqRel,
                                    Ordering::Acquire,
                                )
                                .is_err()
                            {
                                continue;
                            }
                            if phase == 1 {
                                // Join native work/recovery while the caller's
                                // exclusion remains held. Only an error crosses
                                // back; cancelled payloads stay on the owner.
                                let _ = receiver.recv();
                            }
                            break;
                        }
                        2 => {
                            return receiver
                                .recv()
                                .map_err(|_| "The plugin owner stopped".to_owned())?;
                        }
                        _ => break,
                    }
                }
                Err("The plugin owner request timed out and was cancelled".into())
            }
        }
    }
    fn create(owner: &mut Owner, binding: &PluginBinding) -> Result<PluginInstance, String> {
        binding.validate().map_err(str::to_owned)?;
        let format = match binding.format.as_str() {
            "clap" => windfall_plugin_host::PluginFormat::Clap,
            "vst3" => windfall_plugin_host::PluginFormat::Vst3,
            _ => return Err("Unsupported native plugin format".into()),
        };
        if windfall_plugin_host::PluginFormat::of(std::path::Path::new(&binding.path))
            != Some(format)
        {
            return Err("Plugin path does not match its declared format".into());
        }
        let stamp = std::fs::metadata(&binding.path)
            .map(|metadata| (metadata.len(), metadata.modified().ok()))
            .map_err(|error| error.to_string())?;
        if owner
            .approved
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(&(binding.path.clone(), binding.id.clone()))
            != Some(&stamp)
        {
            return Err(
                "This plugin is unscanned, changed, or blocked. Scan it before loading".into(),
            );
        }
        let module = owner
            .host
            .load(std::path::Path::new(&binding.path))
            .map_err(|error| error.to_string())?;
        let descriptor = module
            .descriptors()
            .into_iter()
            .find(|descriptor| descriptor.id == binding.id)
            .ok_or_else(|| "The scanned plugin ID is absent from this file".to_owned())?;
        let kind = match binding.target {
            PluginTarget::Effect { .. } => windfall_plugin_host::PluginKind::Effect,
            PluginTarget::Instrument { .. } => windfall_plugin_host::PluginKind::Instrument,
        };
        if descriptor.format != format || descriptor.kind != kind {
            return Err(
                "Plugin format or instrument/effect role does not match its binding".into(),
            );
        }
        let mut instance = module
            .create(&binding.id)
            .map_err(|error| error.to_string())?;
        if !binding.state.is_empty() {
            let state = PluginState::from_bytes(binding.state.clone());
            instance
                .load_state(&state)
                .map_err(|error| error.to_string())?;
        }
        for param in &binding.parameters {
            if !param.read_only {
                instance.set_param(param.id, f64::from(param.value));
            }
        }
        Ok(instance)
    }
    pub fn discover(&self, mut binding: PluginBinding) -> Result<PluginBinding, String> {
        self.call(move |owner| {
            let mut instance = Self::create(owner, &binding)?;
            let params = instance.params().to_vec();
            binding.parameters = params
                .into_iter()
                .filter(|param| !param.hidden)
                .map(|param| windfall_project::PluginParameter {
                    id: param.id,
                    name: param.name,
                    min: param.min as f32,
                    max: param.max as f32,
                    value: instance.param_value(param.id).unwrap_or(param.default) as f32,
                    stepped: param.stepped,
                    read_only: param.read_only,
                    automatable: param.automatable,
                })
                .collect();
            binding.state = instance
                .save_state()
                .map_err(|error| error.to_string())?
                .into_bytes();
            binding.validate().map_err(str::to_owned)?;
            Ok(binding)
        })
    }
    /// Captures live opaque state into a save copy, leaving document history alone.
    pub fn capture(&self, project: Project) -> Result<Project, String> {
        let revision = self.revision.load(std::sync::atomic::Ordering::Relaxed);
        self.capture_at(project, revision)
    }
    /// Captures only from the document revision paired with the save snapshot.
    pub fn capture_at(&self, mut project: Project, revision: u64) -> Result<Project, String> {
        let current_revision = self.revision.clone();
        let errors = self.errors.clone();
        self.call_with_lifetime(Duration::from_secs(15), move |owner, lifetime| {
            if current_revision.load(std::sync::atomic::Ordering::Relaxed) != revision {
                return Ok(project);
            }
            for binding in &mut project.plugins {
                let selected_token = selected(&owner.selection, binding.target);
                if let Some(record) =
                    selected_token.and_then(|token| owner.instances.get_mut(&token))
                    && record.playback
                    && record.revision == revision
                    && record.binding.path == binding.path
                    && record.binding.id == binding.id
                    && record.binding.format == binding.format
                    && record.binding.state == binding.state
                {
                    let state = match record.capture(lifetime, false) {
                        Ok(captured) => captured.state.into_bytes(),
                        Err(error) => {
                            let mut errors =
                                errors.lock().unwrap_or_else(|error| error.into_inner());
                            errors.retain(|(target, _)| *target != binding.target);
                            errors.push((binding.target, error.clone()));
                            return Err(error);
                        }
                    };
                    if current_revision.load(std::sync::atomic::Ordering::Relaxed) != revision
                        || selected(&owner.selection, binding.target) != selected_token
                    {
                        return Err(
                            "Plugin ownership changed during state capture; save again".into()
                        );
                    }
                    binding.state = state;
                    for param in &mut binding.parameters {
                        if let Some(value) = record.plugin.param_value(param.id) {
                            param.value = value as f32;
                        }
                    }
                    errors
                        .lock()
                        .unwrap_or_else(|error| error.into_inner())
                        .retain(|(target, _)| *target != binding.target);
                }
            }
            Ok(project)
        })
    }
    pub fn editor(&self, target: PluginTarget, open: bool) -> Result<(), String> {
        self.editor_binding(target, None, open)
    }
    /// Called by the session worker with recording exclusion, never by idle.
    pub(crate) fn capture_pending(
        &self,
        request: PendingUpdate,
    ) -> Result<Option<CapturedState>, String> {
        let Update::Capture { target, serial } = request.update else {
            return Err("Not a native state request".into());
        };
        let revision = self.revision.clone();
        let errors = self.errors.clone();
        let captured =
            self.call_with_lifetime(Duration::from_secs(15), move |owner, lifetime| {
                if revision.load(std::sync::atomic::Ordering::Relaxed) != request.revision
                    || selected(&owner.selection, target) != Some(request.token)
                {
                    return Ok(None);
                }
                let Some(record) = owner.instances.get_mut(&request.token).filter(|record| {
                    record.playback
                        && record.revision == request.revision
                        && record.identity == request.binding
                        && record.dirty
                        && record.dirty_serial >= serial
                }) else {
                    return Ok(None);
                };
                let captured = record.capture(lifetime, true)?;
                if let Some(error) = captured.recovery_error {
                    let mut errors = errors.lock().unwrap_or_else(|error| error.into_inner());
                    errors.retain(|(before, _)| *before != target);
                    errors.push((target, error));
                }
                let state = captured.state.into_bytes();
                let ids: Vec<_> = record
                    .plugin
                    .params()
                    .iter()
                    .filter(|param| !param.read_only)
                    .map(|param| param.id)
                    .collect();
                let params = ids
                    .into_iter()
                    .filter_map(|id| {
                        record
                            .plugin
                            .param_value(id)
                            .map(|value| (id, value as f32))
                    })
                    .collect();
                let restart = record.restart;
                Ok(Some(CapturedState {
                    bytes: state,
                    parameters: params,
                    restart,
                    serial: record.dirty_serial,
                }))
            })?;
        if let Some(captured) = &captured {
            let serial = captured.serial;
            let token = request.token;
            self.call(move |owner| {
                if let Some(record) = owner.instances.get_mut(&token) {
                    record.capture_sent = false;
                    if record.dirty_serial == serial {
                        record.dirty = false;
                        record.restart = false;
                    }
                }
                Ok(())
            })?;
        }
        Ok(captured)
    }
    pub fn editor_binding(
        &self,
        target: PluginTarget,
        binding: Option<PluginBinding>,
        open: bool,
    ) -> Result<(), String> {
        let revision = self.revision.load(std::sync::atomic::Ordering::Relaxed);
        self.call(move |owner| {
            let record = selected(&owner.selection, target)
                .and_then(|token| owner.instances.get_mut(&token))
                .filter(|record| record.revision == revision)
                .filter(|record| {
                    binding.as_ref().is_none_or(|binding| {
                        record.binding.path == binding.path
                            && record.binding.id == binding.id
                            && record.binding.format == binding.format
                            && record.binding.state == binding.state
                    })
                })
                .ok_or_else(|| "This plugin is missing or has not been prepared".to_owned())?;
            let instance = &mut record.plugin;
            if open {
                instance
                    .open_editor(&windfall_plugin_host::gui::EditorOptions {
                        title: record.binding.name.clone(),
                        ..Default::default()
                    })
                    .map_err(|error| error.to_string())?;
            } else {
                instance.close_editor();
            }
            Ok(())
        })
    }
    pub fn errors(&self) -> Vec<(PluginTarget, String)> {
        self.errors
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }
    fn record<T>(&self, target: PluginTarget, result: Result<T, String>) -> Result<T, String> {
        if self.rendering {
            return result;
        }
        let mut errors = self
            .errors
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        errors.retain(|(owner, _)| *owner != target);
        if let Err(error) = &result {
            errors.push((target, error.clone()));
        }
        result
    }
}

enum Adapter {
    Effect(PluginEffect),
    Instrument(PluginInstrument),
}
impl Adapter {
    fn failed(&self) -> bool {
        match self {
            Self::Effect(adapter) => adapter.health().failed,
            Self::Instrument(adapter) => adapter.health().failed,
        }
    }
    fn latency(&self) -> usize {
        match self {
            Self::Effect(adapter) => adapter.latency_samples(),
            Self::Instrument(adapter) => adapter.latency_samples(),
        }
    }
    fn tail(&self) -> usize {
        match self {
            Self::Effect(adapter) => adapter.tail_samples(),
            Self::Instrument(adapter) => adapter.tail_samples(),
        }
    }
}
struct Audio {
    instrument: bool,
    selection: Option<Arc<std::sync::atomic::AtomicU64>>,
    ownership: Option<AudioOwnership<Adapter>>,
    latency: usize,
    tail: usize,
    token: u64,
    jobs: mpsc::Sender<Job>,
    /// Allocated from the binding on control; callbacks only replace values.
    pending_params: Box<[(u32, Option<f32>)]>,
    held: [f32; 128],
    replayed: [f32; 128],
    reconcile_notes: bool,
    reset_notes: bool,
    transport: Option<windfall_plugin_host::Transport>,
    tempo: Option<f32>,
}
impl Audio {
    fn reconcile(&mut self) {
        let Self {
            ownership,
            pending_params,
            held,
            replayed,
            reconcile_notes,
            reset_notes,
            transport,
            tempo,
            ..
        } = self;
        let Some(adapter) = ownership.as_mut().and_then(AudioOwnership::current_mut) else {
            return;
        };
        if let Adapter::Instrument(instrument) = adapter
            && *reconcile_notes
        {
            if *reset_notes {
                if !instrument.all_notes_off() {
                    return;
                }
                replayed.fill(0.0);
                *reset_notes = false;
            }
            let mut complete = true;
            for key in 0..128 {
                if replayed[key] != held[key] {
                    let accepted = if held[key] > 0.0 {
                        instrument.note_on(key as u8, held[key])
                    } else {
                        instrument.note_off(key as u8)
                    };
                    if accepted {
                        replayed[key] = held[key];
                    } else {
                        complete = false;
                    }
                }
            }
            *reconcile_notes = !complete;
        }
        let processor = match adapter {
            Adapter::Effect(adapter) => adapter.processor(),
            Adapter::Instrument(adapter) => adapter.processor(),
        };
        if let Some(transport) = transport {
            processor.set_transport(*transport);
        }
        if let Some(tempo) = tempo {
            processor.set_tempo(f64::from(*tempo));
        }
        for (id, value) in pending_params {
            if let Some(pending) = *value
                && processor.set_param(0, *id, f64::from(pending))
            {
                *value = None;
            }
        }
    }
    fn adapter(&self) -> Option<&Adapter> {
        self.ownership.as_ref().and_then(AudioOwnership::current)
    }
    fn adapter_mut(&mut self) -> Option<&mut Adapter> {
        self.ownership
            .as_mut()
            .and_then(AudioOwnership::current_mut)
    }
}
impl Drop for Audio {
    fn drop(&mut self) {
        if let Some(selection) = &self.selection {
            let _ = selection.compare_exchange(
                self.token,
                0,
                std::sync::atomic::Ordering::Relaxed,
                std::sync::atomic::Ordering::Relaxed,
            );
        }
        if let Some(mut ownership) = self.ownership.take() {
            let token = self.token;
            let job: Job = Box::new(move |owner| {
                if let Some(mut record) = owner.instances.remove(&token) {
                    record.plugin.close_editor();
                    if let Some(adapter) = ownership.retire() {
                        record.retire(adapter);
                    }
                    if let Some(adapter) = record.ownership.take_returned() {
                        record.retire(adapter);
                    }
                    if let Some(adapter) = record.returned.take() {
                        record.retire(adapter);
                    }
                }
            });
            if let Err(error) = self.jobs.send(job) {
                // The native owner has failed. Leaking is preferable to COM
                // teardown/deactivation on a controller or audio thread.
                std::mem::forget(error.0);
            }
        }
    }
}
impl HostedEffect for Audio {
    fn control_boundary(&mut self) {
        if let Some(ownership) = &mut self.ownership {
            if let Some(adapter) = ownership.current() {
                self.latency = adapter.latency();
                self.tail = adapter.tail();
            }
            ownership.boundary();
            if ownership.current().is_none() {
                self.reconcile_notes = true;
                self.reset_notes = true;
            }
        }
    }
    fn transport(&mut self, transport: windfall_engine::plugins::PluginTransport) {
        if let Some(selection) = &self.selection {
            selection.store(self.token, std::sync::atomic::Ordering::Relaxed);
        }
        let transport = windfall_plugin_host::Transport {
            playing: transport.playing,
            tempo_bpm: transport.tempo_bpm,
            position_beats: transport.position_beats,
            position_seconds: transport.position_seconds,
            numerator: transport.numerator,
            denominator: transport.denominator,
        };
        self.transport = Some(transport);
        match self.adapter_mut() {
            Some(Adapter::Effect(adapter)) => adapter.set_transport(transport),
            Some(Adapter::Instrument(adapter)) => adapter.set_transport(transport),
            None => {}
        }
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        // Reconcile after engine commands and note expirations, immediately
        // before sounding. A boundary alone must never resurrect a released key.
        self.reconcile();
        let instrument = self.instrument;
        match self.adapter_mut() {
            Some(Adapter::Effect(adapter)) => adapter.process(left, right),
            Some(Adapter::Instrument(adapter)) => adapter.process(left, right),
            None if instrument => {
                left.fill(0.0);
                right.fill(0.0);
            }
            None => {}
        }
    }
    fn set_param(&mut self, id: u32, value: f32) {
        if !value.is_finite() {
            return;
        }
        let accepted = match self.adapter_mut() {
            Some(Adapter::Effect(adapter)) => {
                adapter.processor().set_param(0, id, f64::from(value))
            }
            Some(Adapter::Instrument(adapter)) => {
                adapter.processor().set_param(0, id, f64::from(value))
            }
            None => false,
        };
        if let Some((_, pending)) = self.pending_params.iter_mut().find(|(key, _)| *key == id) {
            *pending = (!accepted).then_some(value);
        }
    }
    fn set_tempo(&mut self, bpm: f32) {
        self.tempo = Some(bpm);
        match self.adapter_mut() {
            Some(Adapter::Effect(adapter)) => adapter.set_tempo(bpm),
            Some(Adapter::Instrument(adapter)) => adapter.set_tempo(bpm),
            None => {}
        }
    }
    fn latency(&self) -> usize {
        self.adapter().map_or(self.latency, Adapter::latency)
    }
    fn tail(&self) -> usize {
        self.adapter().map_or(self.tail, Adapter::tail)
    }
}

#[cfg(all(test, windows))]
#[path = "ownership_tests.rs"]
pub(crate) mod ownership_tests;
impl HostedInstrument for Audio {
    fn note_on(&mut self, key: u8, velocity: f32) {
        let key = key.min(127);
        self.held[key as usize] = if velocity.is_finite() {
            velocity.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if !self.reconcile_notes {
            let accepted = if let Some(Adapter::Instrument(adapter)) = self.adapter_mut() {
                adapter.note_on(key, velocity)
            } else {
                false
            };
            if !accepted {
                self.reconcile_notes = true;
                self.reset_notes = true;
            }
        }
    }
    fn note_off(&mut self, key: u8) {
        let key = key.min(127);
        self.held[key as usize] = 0.0;
        if !self.reconcile_notes {
            let accepted = if let Some(Adapter::Instrument(adapter)) = self.adapter_mut() {
                adapter.note_off(key)
            } else {
                false
            };
            if !accepted {
                self.reconcile_notes = true;
                self.reset_notes = true;
            }
        }
    }
    fn all_notes_off(&mut self) {
        self.held.fill(0.0);
        if !self.reconcile_notes {
            let accepted = if let Some(Adapter::Instrument(adapter)) = self.adapter_mut() {
                adapter.all_notes_off()
            } else {
                false
            };
            if !accepted {
                self.reconcile_notes = true;
                self.reset_notes = true;
            }
        }
    }
    fn voices(&self) -> usize {
        if self.reconcile_notes {
            return self.held.iter().filter(|velocity| **velocity > 0.0).count();
        }
        match self.adapter() {
            Some(Adapter::Instrument(adapter)) => adapter.active_voices(),
            _ => 0,
        }
    }
}
impl PluginFactory for Runtime {
    fn provider_identity(&self) -> u64 {
        Arc::as_ptr(&self.revision) as usize as u64
    }
    fn render_factory(&self) -> Option<Arc<dyn PluginFactory>> {
        let mut runtime = self.clone();
        runtime.rendering = true;
        Some(Arc::new(runtime))
    }
    fn revision(&self) -> u64 {
        self.preparation_revision()
    }
    fn effect(
        &self,
        binding: &PluginBinding,
        rate: u32,
        block: usize,
    ) -> Result<Box<dyn HostedEffect>, String> {
        let binding = binding.clone();
        let target = binding.target;
        let jobs = self.jobs.clone();
        let revision = self.preparation_revision();
        let playback = !self.rendering;
        let selection = self.selection(target);
        let fallback_selection = selection.clone();
        self.record(
            target,
            self.call(move |owner| {
                let mut instance = Self::create(owner, &binding)?;
                let adapter = instance
                    .prepare_effect(rate as f32, block)
                    .map_err(|error| error.to_string())?;
                if adapter.latency_samples() > rate as usize {
                    let _ = instance.release_effect(adapter);
                    return Err("Plugin latency exceeds the one-second compensation limit".into());
                }
                owner.next += 1;
                let token = owner.next;
                let adapter = Adapter::Effect(adapter);
                let latency = adapter.latency();
                let tail = adapter.tail();
                let (ownership, audio) = exchange(adapter);
                let pending_params = binding
                    .parameters
                    .iter()
                    .filter(|param| !param.read_only)
                    .map(|param| (param.id, None))
                    .collect();
                owner.instances.insert(
                    token,
                    Instance {
                        identity: binding_identity(&binding),
                        target,
                        plugin: instance,
                        binding,
                        revision,
                        playback,
                        ownership,
                        rate,
                        block,
                        instrument: false,
                        latency,
                        tail,
                        returned: None,
                        dirty: false,
                        dirty_serial: 0,
                        capture_sent: false,
                        restart: false,
                        notifications: Vec::new(),
                    },
                );

                Ok(Box::new(Audio {
                    instrument: false,
                    selection,
                    ownership: Some(audio),
                    latency,
                    tail,
                    token,
                    jobs,
                    pending_params,
                    held: [0.0; 128],
                    replayed: [0.0; 128],
                    reconcile_notes: false,
                    reset_notes: false,
                    transport: None,
                    tempo: None,
                }) as Box<dyn HostedEffect>)
            }),
        )
        .or_else(|_| {
            Ok(Box::new(Audio {
                instrument: false,
                selection: fallback_selection,
                ownership: None,
                latency: 0,
                tail: 0,
                token: 0,
                jobs: self.jobs.clone(),
                pending_params: Box::new([]),
                held: [0.0; 128],
                replayed: [0.0; 128],
                reconcile_notes: false,
                reset_notes: false,
                transport: None,
                tempo: None,
            }) as Box<dyn HostedEffect>)
        })
    }
    fn instrument(
        &self,
        binding: &PluginBinding,
        rate: u32,
        block: usize,
    ) -> Result<Box<dyn HostedInstrument>, String> {
        let binding = binding.clone();
        let target = binding.target;
        let jobs = self.jobs.clone();
        let revision = self.preparation_revision();
        let playback = !self.rendering;
        let selection = self.selection(target);
        let fallback_selection = selection.clone();
        self.record(
            target,
            self.call(move |owner| {
                let mut instance = Self::create(owner, &binding)?;
                let adapter = instance
                    .prepare_instrument(rate as f32, block)
                    .map_err(|error| error.to_string())?;
                if adapter.latency_samples() > rate as usize {
                    let _ = instance.release_instrument(adapter);
                    return Err("Plugin latency exceeds the one-second compensation limit".into());
                }
                owner.next += 1;
                let token = owner.next;
                let adapter = Adapter::Instrument(adapter);
                let latency = adapter.latency();
                let tail = adapter.tail();
                let (ownership, audio) = exchange(adapter);
                let pending_params = binding
                    .parameters
                    .iter()
                    .filter(|param| !param.read_only)
                    .map(|param| (param.id, None))
                    .collect();
                owner.instances.insert(
                    token,
                    Instance {
                        identity: binding_identity(&binding),
                        target,
                        plugin: instance,
                        binding,
                        revision,
                        playback,
                        ownership,
                        rate,
                        block,
                        instrument: true,
                        latency,
                        tail,
                        returned: None,
                        dirty: false,
                        dirty_serial: 0,
                        capture_sent: false,
                        restart: false,
                        notifications: Vec::new(),
                    },
                );

                Ok(Box::new(Audio {
                    instrument: true,
                    selection,
                    ownership: Some(audio),
                    latency,
                    tail,
                    token,
                    jobs,
                    pending_params,
                    held: [0.0; 128],
                    replayed: [0.0; 128],
                    reconcile_notes: false,
                    reset_notes: false,
                    transport: None,
                    tempo: None,
                }) as Box<dyn HostedInstrument>)
            }),
        )
        .or_else(|_| {
            Ok(Box::new(Audio {
                instrument: true,
                selection: fallback_selection,
                ownership: None,
                latency: 0,
                tail: 0,
                token: 0,
                jobs: self.jobs.clone(),
                pending_params: Box::new([]),
                held: [0.0; 128],
                replayed: [0.0; 128],
                reconcile_notes: false,
                reset_notes: false,
                transport: None,
                tempo: None,
            }) as Box<dyn HostedInstrument>)
        })
    }
}
