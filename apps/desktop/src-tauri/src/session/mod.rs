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
//! device, runs with the document lock released, on the thread of the
//! caller: the app calls those methods from worker threads and the quick
//! ones from anywhere. Events are emitted while the lock that guards their
//! subject is held, which is what keeps them in order.
//!
//! A thread takes the locks it needs in this order and never the other way
//! round:
//!
//! 1. `save` or `configuring`. Each puts one kind of slow work in a queue:
//!    `save` is held from before a save or a backup copies the project until
//!    its file is written and the document is marked, so files reach the
//!    disk in the order their copies were taken; `configuring` is held while
//!    the audio device is reopened. Nothing is held when either is taken,
//!    and no thread holds both.
//! 2. `state`, the document lock.
//! 3. `transport`, the sample cache and the engine's controller, each for a
//!    moment, under `state` or alone.
//!
//! `preview` is held only around handing a preview to the controller.
//! `status`, `settings` and `subscribers` are never held together with
//! `state` or with each other. The settings file is written under
//! `settings`, which is why that lock stays clear of the document.
//!
//! Who takes what:
//!
//! - Edits, undo and the transport: `state`, then `transport`.
//! - Save and backup (the autosave thread): `save`, then `state` to copy
//!   the project, nothing but `save` while the file is written, then
//!   `state` again; `settings` once `save` is released.
//! - New and open: `state` to take a ticket, no lock while the file is read
//!   and decoded, then `state` to swap the project in; `settings` after.
//! - The sample loader, file imports and sample reload: no lock while
//!   decoding, then `state`.
//! - Export: `state` to copy the project, then no lock at all on its own
//!   thread.
//! - The realtime thread: `subscribers`, then `transport`, and once a second
//!   `status`. It never takes `state`, so no slow edit can stall the meters.
//! - Reopening the audio device: `configuring` throughout, `settings`, then
//!   `state` to hand the project to the new stream, then `status`.
//!
//! # Work that outlives its document
//!
//! Slow work is done for the document that was open when it started. Each
//! document has a number, `generation`, and everything that works with no
//! lock held and then touches the document or the sample pool compares it
//! first and gives up if another document has taken its place.

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
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard, Weak};

use windfall_engine::{Controller, Engine, SamplePool};
use windfall_ipc::{AudioHost, AudioSettings, EngineStatus, TransportState};
use windfall_project::{Document, SampleId};

use crate::events::{Event, EventSink};
use crate::samples::SampleCache;
use crate::settings::SettingsStore;
use crate::sync::lock;
use crate::template::default_project;

pub use audio::openable;
pub use autosave::{AUTOSAVE_INTERVAL, backup_timestamp};
pub use files::{EDITED_WHILE_MOVING, NO_FILE_YET};
pub use library::{ClipPlace, PROJECT_REPLACED};
pub use realtime::{FRAME_INTERVAL, FrameSender};
pub use transport::{EMPTY_PLAYLIST, MUTED_PLAYLIST, SILENT_PLAYLIST};

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
    /// Held by a save or a backup from before it copies the project until
    /// its file is written. Taken before `state`, never under it.
    save: Mutex<()>,
    state: Mutex<State>,
    /// The transport as the UI was last told. Locked after `state`, never
    /// before it.
    transport: Mutex<TransportState>,
    /// Held while the audio device is reopened, so the settings remembered
    /// are the ones of the stream that ends up open.
    configuring: Mutex<()>,
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
    /// The export that is running has been asked to stop.
    export_cancelled: AtomicBool,
    /// Counts preview requests, so a file that finishes decoding after a
    /// newer request was made is not played. Held while a preview is
    /// handed to the engine or stopped, so neither can slip in between
    /// another's look at the count and what it then does.
    preview: Mutex<u64>,
    #[cfg(test)]
    pauses: tests::Pauses,
}

/// The open project and the audio that goes with it.
struct State {
    document: Document,
    /// The file that saving writes to: where the project was last saved or
    /// opened from. `None` for a project that was never saved and for one
    /// opened from a backup, so that saving either asks for a place first.
    path: Option<PathBuf>,
    /// The folder project samples are relative to: the one `path` is in or,
    /// for a project opened from a backup, the one that holds the project
    /// the backup was made of.
    sample_dir: Option<PathBuf>,
    pool: SamplePool,
    /// The samples in `pool`, which cannot list them itself.
    loaded: HashSet<SampleId>,
    /// Samples being decoded on another thread.
    loading: HashSet<SampleId>,
    /// Samples whose file could not be read. They are reported once and not
    /// tried again until the user asks for the samples to be reloaded.
    failed: HashSet<SampleId>,
    /// Counts the documents this session has held. Work that started for an
    /// earlier document checks it and gives up.
    generation: u64,
    /// Counts the changes made to this document's project, undo and redo
    /// included. Equal counts mean the project is the same as it was.
    edits: u64,
    /// Counts the requests to replace the document, whether or not they got
    /// to. Only the last one made may still do it.
    replacements: u64,
}

impl State {
    /// The folder project samples are relative to.
    fn project_dir(&self) -> Option<&Path> {
        self.sample_dir.as_deref()
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
                save: Mutex::new(()),
                state: Mutex::new(State {
                    document: Document::new(project),
                    path: None,
                    sample_dir: None,
                    pool: decoded.pool,
                    loaded: decoded.loaded,
                    loading: HashSet::new(),
                    failed: decoded.failed,
                    generation: 0,
                    edits: 0,
                    replacements: 0,
                }),
                transport: Mutex::new(controller.transport()),
                configuring: Mutex::new(()),
                status: Mutex::new(audio.status()),
                settings: Mutex::new(settings),
                subscribers: Mutex::new(Vec::new()),
                controller,
                audio,
                events,
                cache,
                factory_dir,
                exporting: AtomicBool::new(false),
                export_cancelled: AtomicBool::new(false),
                preview: Mutex::new(0),
                #[cfg(test)]
                pauses: tests::Pauses::default(),
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
