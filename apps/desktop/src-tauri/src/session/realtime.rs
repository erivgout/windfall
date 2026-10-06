//! The realtime feed: the playhead and meters, sixty times a second.

use std::io;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use windfall_ipc::RealtimeFrame;

use super::Session;
use crate::sync::lock;

/// Time between two frames of the realtime feed.
pub const FRAME_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 60);

/// Frames between two looks at the audio device's status: about a second.
const STATUS_EVERY: u32 = 60;

/// Delivers a frame to one window. Returns false once the window is gone,
/// and is then dropped.
pub type FrameSender = Box<dyn FnMut(&RealtimeFrame) -> bool + Send>;

pub(super) struct Subscriber {
    /// Names the window, so a window that subscribes again, as after a
    /// reload, replaces its old sender instead of being sent everything
    /// twice.
    key: String,
    send: FrameSender,
}

impl Session {
    /// Starts sending realtime frames to a window.
    pub fn subscribe_realtime(&self, key: impl Into<String>, send: FrameSender) {
        let key = key.into();
        let mut subscribers = lock(&self.inner.subscribers);
        subscribers.retain(|subscriber| subscriber.key != key);
        subscribers.push(Subscriber { key, send });
    }

    /// Stops sending realtime frames to a window, as when it closes.
    pub fn unsubscribe_realtime(&self, key: &str) {
        lock(&self.inner.subscribers).retain(|subscriber| subscriber.key != key);
    }

    /// Reads one frame from the engine and sends it to every subscriber.
    ///
    /// Reading a frame resets the meter peaks and frees what the audio
    /// thread has handed back, so exactly one caller must do this, and keep
    /// doing it whether or not anyone is listening. That caller is the
    /// thread [`spawn_realtime`](Self::spawn_realtime) starts.
    pub fn realtime_tick(&self) -> RealtimeFrame {
        let frame = self.controller().frame();
        lock(&self.inner.subscribers).retain_mut(|subscriber| (subscriber.send)(&frame));
        // Playback that ended by itself: the end of a song, or a device
        // that went away.
        let told = lock(&self.inner.transport).playing;
        if told != frame.playing {
            self.sync_transport();
        }
        frame
    }

    /// Starts the thread that drives the realtime feed. It stops when the
    /// last handle to the session is dropped.
    pub fn spawn_realtime(&self) -> io::Result<JoinHandle<()>> {
        let session = self.downgrade();
        thread::Builder::new()
            .name("windfall-realtime".to_owned())
            .spawn(move || {
                let mut deadline = Instant::now();
                let mut frames = 0_u32;
                loop {
                    // Frames are timed from a running deadline and not from
                    // the end of the last one, so the rate does not drift
                    // with how long a frame takes to send.
                    deadline += FRAME_INTERVAL;
                    let now = Instant::now();
                    match deadline.checked_duration_since(now) {
                        Some(wait) => thread::sleep(wait),
                        // After the machine slept, carry on from now rather
                        // than sending the missed frames in a burst.
                        None if now.duration_since(deadline) > FRAME_INTERVAL * 4 => {
                            deadline = now;
                        }
                        None => {}
                    }
                    let Some(session) = session.upgrade() else {
                        break;
                    };
                    session.realtime_tick();
                    frames = frames.wrapping_add(1);
                    if frames.is_multiple_of(STATUS_EVERY) {
                        session.poll_engine_status();
                    }
                }
            })
    }
}
