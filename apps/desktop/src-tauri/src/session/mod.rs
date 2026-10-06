//! The session: the one open project, the engine that plays it, and every
//! operation the UI can ask for.
//!
//! Nothing here knows about Tauri. The app hands the session an
//! [`EventSink`] that emits to its windows and calls one method per IPC
//! command; tests hand it a sink that records and an engine with no device.
//!
//! # Locks
//!
//! The document sits behind one mutex, and every method takes it for as long
//! as an edit takes and no longer. Whatever is slow, such as decoding audio,
//! reading and writing files, rendering an export or opening an audio
//! device, runs with no lock held, on the thread of the caller: the app
//! calls those methods from worker threads and the quick ones from anywhere.
//! Events are emitted while the lock that guards their subject is held,
//! which is what keeps them in order.

mod audio;
mod autosave;
mod edit;
mod export;
mod files;
mod library;
mod realtime;
mod samples;
#[cfg(test)]
mod tests;
mod transport;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex, MutexGuard, Weak};

use windfall_engine::{Controller, Engine, SamplePool};
use windfall_ipc::{AudioHost, AudioSettings, EngineStatus, TransportState};
use windfall_project::{Document, SampleId};

use crate::events::{Event, EventSink};
use crate::samples::SampleCache;
use crate::settings::SettingsStore;
use crate::sync::lock;
use crate::template::default_project;

pub use autosave::{AUTOSAVE_INTERVAL, backup_timestamp};
pub use files::NO_FILE_YET;
pub use realtime::{FRAME_INTERVAL, FrameSender};

/// The audio output as the session needs it. [`Engine`] is the real one.
pub trait AudioDevice: Send + Sync {
    fn status(&self) -> EngineStatus;
    /// Closes the stream and opens one with new settings. May take seconds.
    fn reconfigure(&self, settings: &AudioSettings);
    /// Lists the hosts and devices of this machine. May take a while.
    fn devices(&self) -> Vec<AudioHost>;
}

impl AudioDevice for Engine {
    fn status(&self) -> EngineStatus {
        Engine::status(self)
    }

    fn reconfigure(&self, settings: &AudioSettings) {
        Engine::reconfigure(self, settings);
    }

    fn devices(&self) -> Vec<AudioHost> {
        Engine::devices()
    }
}

/// What a session is built from.
pub struct SessionConfig {
    /// Drives the engine that `audio` feeds to a device.
    pub controller: Controller,
    pub audio: Arc<dyn AudioDevice>,
    pub events: Arc<dyn EventSink>,
    /// The folder that holds the content shipped with Windfall.
    pub factory_dir: PathBuf,
    pub settings: SettingsStore,
}

/// A handle to the session. Clones share it.
#[derive(Clone)]
pub struct Session {
    inner: Arc<Inner>,
}

/// A handle that does not keep the session alive, for background threads.
pub struct WeakSession {
    inner: Weak<Inner>,
}

impl WeakSession {
    pub fn upgrade(&self) -> Option<Session> {
        self.inner.upgrade().map(|inner| Session { inner })
    }
}

struct Inner {
    state: Mutex<State>,
    /// The transport as the UI was last told. Locked after `state`, never
    /// before it.
    transport: Mutex<TransportState>,
    /// The engine status as the UI was last told.
    status: Mutex<EngineStatus>,
    settings: Mutex<SettingsStore>,
    subscribers: Mutex<Vec<realtime::Subscriber>>,
    controller: Controller,
    audio: Arc<dyn AudioDevice>,
    events: Arc<dyn EventSink>,
    cache: SampleCache,
    factory_dir: PathBuf,
    exporting: AtomicBool,
    /// Counts preview requests, so a file that finishes decoding after a
    /// newer request was made is not played.
    preview: AtomicU64,
}

/// The open project and the audio that goes with it.
struct State {
    document: Document,
    /// Where the project was last saved or opened from.
    path: Option<PathBuf>,
    pool: SamplePool,
    /// The samples in `pool`, which cannot list them itself.
    loaded: HashSet<SampleId>,
    /// Samples being decoded on another thread.
    loading: HashSet<SampleId>,
    /// Samples whose file could not be read. They are reported once and not
    /// tried again while this document is open.
    failed: HashSet<SampleId>,
    /// Counts the documents this session has held. Work that started for an
    /// earlier document checks it and gives up.
    generation: u64,
}

impl State {
    /// The folder the project file is in, which project samples are
    /// relative to.
    fn project_dir(&self) -> Option<&Path> {
        self.path.as_deref().and_then(Path::parent)
    }

    fn path_text(&self) -> Option<String> {
        self.path.as_deref().map(crate::paths::display)
    }
}

impl Session {
    /// Starts a session on the default project and hands it to the engine.
    ///
    /// Decodes the default kit, which takes a few milliseconds. A kit file
    /// that is missing leaves its channel silent and is logged: no window
    /// exists yet to tell.
    pub fn new(config: SessionConfig) -> Self {
        let SessionConfig {
            controller,
            audio,
            events,
            factory_dir,
            settings,
        } = config;
        let factory_dir = crate::paths::clean(&factory_dir);
        let cache = SampleCache::new();

        let project = default_project();
        let decoded = samples::decode_all(&cache, &project, None, &factory_dir);
        for warning in &decoded.warnings {
            log::warn!("{warning}");
        }
        controller.set_project(&project, &decoded.pool);

        Self {
            inner: Arc::new(Inner {
                state: Mutex::new(State {
                    document: Document::new(project),
                    path: None,
                    pool: decoded.pool,
                    loaded: decoded.loaded,
                    loading: HashSet::new(),
                    failed: decoded.failed,
                    generation: 0,
                }),
                transport: Mutex::new(controller.transport()),
                status: Mutex::new(audio.status()),
                settings: Mutex::new(settings),
                subscribers: Mutex::new(Vec::new()),
                controller,
                audio,
                events,
                cache,
                factory_dir,
                exporting: AtomicBool::new(false),
                preview: AtomicU64::new(0),
            }),
        }
    }

    pub fn downgrade(&self) -> WeakSession {
        WeakSession {
            inner: Arc::downgrade(&self.inner),
        }
    }

    /// The folder that holds the content shipped with Windfall.
    pub fn factory_dir(&self) -> &Path {
        &self.inner.factory_dir
    }

    fn state(&self) -> MutexGuard<'_, State> {
        lock(&self.inner.state)
    }

    fn store(&self) -> MutexGuard<'_, SettingsStore> {
        lock(&self.inner.settings)
    }

    fn controller(&self) -> &Controller {
        &self.inner.controller
    }

    fn emit(&self, event: Event) {
        self.inner.events.emit(event);
    }
}
