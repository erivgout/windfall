//! The transport: play, stop, and where the playhead is.

use windfall_ipc::{TransportPatch, TransportState};

use super::Session;
use crate::events::Event;
use crate::sync::lock;

impl Session {
    pub fn transport_state(&self) -> TransportState {
        self.controller().transport()
    }

    /// Starts playback. With no audio device open nothing plays, and the
    /// state returned says so.
    pub fn transport_play(&self) -> TransportState {
        self.controller().play();
        self.sync_transport()
    }

    /// Stops playback. The playhead returns to where playback started.
    pub fn transport_stop(&self) -> TransportState {
        self.controller().stop();
        self.sync_transport()
    }

    pub fn transport_toggle(&self) -> TransportState {
        if self.controller().transport().playing {
            self.transport_stop()
        } else {
            self.transport_play()
        }
    }

    pub fn transport_seek(&self, tick: f64) {
        self.controller().seek(tick);
    }

    /// Changes the play mode, the pattern that plays in pattern mode, or
    /// song looping. Fails for a pattern the project does not have.
    pub fn transport_set(&self, patch: TransportPatch) -> Result<TransportState, String> {
        // Held across the change, so the pattern cannot be deleted between
        // the check and the engine hearing of it.
        let state = self.state();
        if let Some(pattern) = patch.pattern
            && state.document.project().pattern(pattern).is_none()
        {
            return Err(format!("pattern {} does not exist", pattern.0));
        }
        self.controller().set_transport(patch);
        Ok(self.sync_transport())
    }

    /// Tells the UI the transport state if it is not what it was last told.
    /// Everything that can change the transport ends by calling this, and
    /// so does the realtime loop, which is how playback that ends by itself
    /// gets announced.
    pub(super) fn sync_transport(&self) -> TransportState {
        let mut told = lock(&self.inner.transport);
        let now = self.controller().transport();
        if *told != now {
            *told = now;
            self.emit(Event::TransportState(now));
        }
        now
    }

    /// Tells the UI the transport state whether or not it changed, as after
    /// loading a project, when the UI starts over.
    pub(super) fn announce_transport(&self) -> TransportState {
        let mut told = lock(&self.inner.transport);
        let now = self.controller().transport();
        *told = now;
        self.emit(Event::TransportState(now));
        now
    }
}
