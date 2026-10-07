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
};
use windfall_project::{PluginBinding, PluginTarget, Project};

type Job = Box<dyn FnOnce(&mut Owner) + Send>;
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
pub enum Update {
    Parameter {
        target: PluginTarget,
        id: u32,
        value: f32,
        gesture: u64,
    },
    State {
        target: PluginTarget,
        state: Vec<u8>,
    },
    Restart,
}
pub struct PendingUpdate {
    pub binding: u64,
    pub token: u64,
    pub revision: u64,
    pub update: Update,
}
struct Instance {
    identity: u64,
    target: PluginTarget,
    plugin: PluginInstance,
    binding: PluginBinding,
    revision: u64,
    playback: bool,
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
                        instance.idle(&mut |notification| match notification {
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
                                }
                            }
                            PluginNotification::StateChanged => state_changed = true,
                            PluginNotification::LatencyChanged { .. }
                            | PluginNotification::RestartRequested
                            | PluginNotification::ParamsRescanned => {
                                send(Update::Restart);
                            }
                            _ => {}
                        });
                        if current
                            && state_changed
                            && let Ok(state) = instance.save_state()
                        {
                            send(Update::State {
                                target: *target,
                                state: state.into_bytes(),
                            });
                        }
                    }
                }
            })
            .map_err(|error| error.to_string())?;
        Ok(Self {
            selection,
            jobs,
            errors: Arc::new(Mutex::new(Vec::new())),
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
        let (sender, receiver) = mpsc::sync_channel(1);
        self.jobs
            .send(Box::new(move |owner| {
                let _ = sender.send(job(owner));
            }))
            .map_err(|_| "The plugin owner stopped".to_owned())?;
        receiver
            .recv_timeout(Duration::from_secs(15))
            .map_err(|_| {
                "The plugin owner stopped or did not respond within 15 seconds".to_owned()
            })?
    }
    fn create(owner: &mut Owner, binding: &PluginBinding) -> Result<PluginInstance, String> {
        binding.validate().map_err(str::to_owned)?;
        if binding.format != "clap" {
            return Err("VST3 desktop hosting requires safe inactive-instance state capture and is not enabled".into());
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
    pub fn capture(&self, mut project: Project) -> Result<Project, String> {
        let revision = self.revision.load(std::sync::atomic::Ordering::Relaxed);
        self.call(move |owner| {
            for binding in &mut project.plugins {
                if let Some(record) = selected(&owner.selection, binding.target)
                    .and_then(|token| owner.instances.get_mut(&token))
                    && record.revision == revision
                    && record.binding.path == binding.path
                    && record.binding.id == binding.id
                    && record.binding.format == binding.format
                    && record.binding.state == binding.state
                {
                    binding.state = record
                        .plugin
                        .save_state()
                        .map_err(|error| error.to_string())?
                        .into_bytes();
                }
            }
            Ok(project)
        })
    }
    pub fn editor(&self, target: PluginTarget, open: bool) -> Result<(), String> {
        self.editor_binding(target, None, open)
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
struct Audio {
    instrument: bool,
    selection: Option<Arc<std::sync::atomic::AtomicU64>>,
    adapter: Option<Adapter>,
    token: u64,
    jobs: mpsc::Sender<Job>,
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
        if let Some(adapter) = self.adapter.take() {
            let token = self.token;
            let _ = self.jobs.send(Box::new(move |owner| {
                if let Some(record) = owner.instances.remove(&token) {
                    let mut instance = record.plugin;
                    instance.close_editor();
                    match adapter {
                        Adapter::Effect(adapter) => instance.release_effect(adapter),
                        Adapter::Instrument(adapter) => instance.release_instrument(adapter),
                    }
                }
            }));
        }
    }
}
impl HostedEffect for Audio {
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
        match self.adapter.as_mut() {
            Some(Adapter::Effect(adapter)) => adapter.set_transport(transport),
            Some(Adapter::Instrument(adapter)) => adapter.set_transport(transport),
            None => {}
        }
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        match self.adapter.as_mut() {
            Some(Adapter::Effect(adapter)) => adapter.process(left, right),
            Some(Adapter::Instrument(adapter)) => adapter.process(left, right),
            None if self.instrument => {
                left.fill(0.0);
                right.fill(0.0);
            }
            None => {}
        }
    }
    fn set_param(&mut self, id: u32, value: f32) {
        match self.adapter.as_mut() {
            Some(Adapter::Effect(adapter)) => {
                adapter.processor().set_param(0, id, f64::from(value));
            }
            Some(Adapter::Instrument(adapter)) => {
                adapter.processor().set_param(0, id, f64::from(value));
            }
            None => {}
        }
    }
    fn set_tempo(&mut self, bpm: f32) {
        match self.adapter.as_mut() {
            Some(Adapter::Effect(adapter)) => adapter.set_tempo(bpm),
            Some(Adapter::Instrument(adapter)) => adapter.set_tempo(bpm),
            None => {}
        }
    }
    fn latency(&self) -> usize {
        match self.adapter.as_ref() {
            Some(Adapter::Effect(adapter)) => adapter.latency_samples(),
            Some(Adapter::Instrument(adapter)) => adapter.latency_samples(),
            None => 0,
        }
    }
    fn tail(&self) -> usize {
        match self.adapter.as_ref() {
            Some(Adapter::Effect(adapter)) => adapter.tail_samples(),
            Some(Adapter::Instrument(adapter)) => adapter.tail_samples(),
            None => 0,
        }
    }
}
impl HostedInstrument for Audio {
    fn note_on(&mut self, key: u8, velocity: f32) {
        if let Some(Adapter::Instrument(adapter)) = &mut self.adapter {
            adapter.note_on(key, velocity);
        }
    }
    fn note_off(&mut self, key: u8) {
        if let Some(Adapter::Instrument(adapter)) = &mut self.adapter {
            adapter.note_off(key);
        }
    }
    fn all_notes_off(&mut self) {
        if let Some(Adapter::Instrument(adapter)) = &mut self.adapter {
            adapter.all_notes_off();
        }
    }
    fn voices(&self) -> usize {
        match &self.adapter {
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
                    instance.release_effect(adapter);
                    return Err("Plugin latency exceeds the one-second compensation limit".into());
                }
                owner.next += 1;
                let token = owner.next;
                owner.instances.insert(
                    token,
                    Instance {
                        identity: binding_identity(&binding),
                        target,
                        plugin: instance,
                        binding,
                        revision,
                        playback,
                    },
                );

                Ok(Box::new(Audio {
                    instrument: false,
                    selection,
                    adapter: Some(Adapter::Effect(adapter)),
                    token,
                    jobs,
                }) as Box<dyn HostedEffect>)
            }),
        )
        .or_else(|_| {
            Ok(Box::new(Audio {
                instrument: false,
                selection: fallback_selection,
                adapter: None,
                token: 0,
                jobs: self.jobs.clone(),
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
                    instance.release_instrument(adapter);
                    return Err("Plugin latency exceeds the one-second compensation limit".into());
                }
                owner.next += 1;
                let token = owner.next;
                owner.instances.insert(
                    token,
                    Instance {
                        identity: binding_identity(&binding),
                        target,
                        plugin: instance,
                        binding,
                        revision,
                        playback,
                    },
                );

                Ok(Box::new(Audio {
                    instrument: true,
                    selection,
                    adapter: Some(Adapter::Instrument(adapter)),
                    token,
                    jobs,
                }) as Box<dyn HostedInstrument>)
            }),
        )
        .or_else(|_| {
            Ok(Box::new(Audio {
                instrument: true,
                selection: fallback_selection,
                adapter: None,
                token: 0,
                jobs: self.jobs.clone(),
            }) as Box<dyn HostedInstrument>)
        })
    }
}
