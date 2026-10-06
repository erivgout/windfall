//! The audio device, and the sounds that are not part of playback: notes
//! played by hand and files previewed from the browser.

use std::sync::atomic::Ordering;

use windfall_ipc::{AudioHost, AudioSettings, EngineStatus};
use windfall_project::ChannelId;

use super::Session;
use crate::events::Event;
use crate::paths;
use crate::sync::lock;

impl Session {
    pub fn engine_status(&self) -> EngineStatus {
        self.inner.audio.status()
    }

    /// Lists the audio hosts and devices of this machine. Can take a while.
    pub fn engine_devices(&self) -> Vec<AudioHost> {
        self.inner.audio.devices()
    }

    /// Opens the audio output with new settings and remembers them for the
    /// next start. Slow: a driver can take seconds to open. The returned
    /// status says how it went; a device that could not be opened is not an
    /// error of this call.
    pub fn engine_configure(&self, settings: AudioSettings) -> EngineStatus {
        self.store()
            .update(|stored| stored.audio = settings.clone());
        self.inner.audio.reconfigure(&settings);
        {
            let state = self.state();
            // Reopening the device stops playback.
            self.push_project(&state);
        }
        let mut told = lock(&self.inner.status);
        let status = self.inner.audio.status();
        *told = status.clone();
        self.emit(Event::EngineStatus(status.clone()));
        status
    }

    /// Tells the UI the engine status if it is not what it was last told.
    /// The realtime loop calls this about once a second, which is how a
    /// device that is unplugged or comes back gets announced.
    pub fn poll_engine_status(&self) {
        let mut told = lock(&self.inner.status);
        let status = self.inner.audio.status();
        if *told != status {
            *told = status.clone();
            self.emit(Event::EngineStatus(status));
        }
    }

    /// Plays a note on a channel right away, as from the UI keyboard.
    pub fn audition_note_on(&self, channel: ChannelId, key: u8, velocity: f32) {
        self.controller().note_on(channel, key, velocity);
    }

    pub fn audition_note_off(&self, channel: ChannelId, key: u8) {
        self.controller().note_off(channel, key);
    }

    /// Plays an audio file once, straight to the master. Slow the first
    /// time a file is asked for, because it has to be decoded. A file that
    /// finishes decoding after a newer preview was asked for, or after
    /// [`preview_stop`](Self::preview_stop), is not played.
    pub fn preview_play(&self, path: &str) -> Result<(), String> {
        let file = paths::absolute(path)?;
        let ticket = self.inner.preview.fetch_add(1, Ordering::SeqCst) + 1;
        let buffer = self.inner.cache.decode(&file)?;
        if self.inner.preview.load(Ordering::SeqCst) == ticket {
            self.controller().preview(buffer);
        }
        Ok(())
    }

    pub fn preview_stop(&self) {
        // Also cancels a preview that is still being decoded.
        self.inner.preview.fetch_add(1, Ordering::SeqCst);
        self.controller().stop_preview();
    }
}
