//! The control side of the engine: everything other threads may ask of the
//! audio thread.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard};

use rtrb::{Consumer, Producer, PushError, RingBuffer};
use windfall_core::AudioBuffer;
use windfall_ipc::{PlayMode, RealtimeFrame, TransportPatch, TransportState};
use windfall_project::{ChannelId, MAX_GAIN, MAX_KEY, PatternId, Project};

use crate::message::{GARBAGE_CAPACITY, Garbage, MESSAGE_CAPACITY, Message};
use crate::plan::{Plan, PlanState, compile};
use crate::pool::SamplePool;
use crate::processor::Processor;
use crate::shared::Shared;

/// Sends requests to the audio thread and reads back what it publishes.
///
/// Clones share one engine. A controller outlives device changes: when the
/// engine reopens its stream, the controller hands the project, transport
/// settings and playhead to the new stream's processor. Playback itself
/// does not carry over and has to be started again.
///
/// Calls never wait for the audio thread. A request takes effect at the
/// start of the next audio buffer, in the order the requests were made.
/// While no stream is open, requests that only make sound (playing, notes,
/// previews) are dropped, and the project, transport settings and playhead
/// are remembered for the next stream.
#[derive(Clone)]
pub struct Controller {
    inner: Arc<Inner>,
}

struct Inner {
    shared: Arc<Shared>,
    state: Mutex<State>,
}

struct State {
    link: Option<Link>,
    /// The plan most recently compiled, kept to hand to a new processor.
    plan: Arc<Plan>,
    /// The transport as last requested. `playing` here is what was asked
    /// for; the audio thread has the final say once it has caught up.
    transport: TransportState,
    /// Counts play and stop requests, so the audio thread's published
    /// state can be matched to the request it answers.
    sequence: u32,
    output_gain: f32,
    /// Messages that did not fit in the queue, oldest first.
    backlog: VecDeque<Message>,
}

/// The queues to and from the processor currently attached.
struct Link {
    messages: Producer<Message>,
    garbage: Consumer<Garbage>,
}

/// Measurements the device callback takes while a stream runs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StreamStats {
    /// Frames the device has asked for since the stream started.
    pub frames: u64,
    /// Dropouts since the stream started: the engine took longer to fill a
    /// buffer than the buffer lasts, the driver reported an underrun, or the
    /// device went without audio because it was served too late.
    pub xruns: u32,
    /// Running average of the share of each buffer's duration the engine
    /// spent filling it, 0 to 1.
    pub cpu: f32,
    /// The highest single-buffer share since the last call.
    pub cpu_peak: f32,
}

impl Controller {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                shared: Arc::new(Shared::new()),
                state: Mutex::new(State {
                    link: None,
                    plan: Arc::new(Plan::empty()),
                    transport: TransportState {
                        playing: false,
                        mode: PlayMode::Pattern,
                        pattern: PatternId(0),
                        loop_song: false,
                    },
                    sequence: 0,
                    output_gain: 1.0,
                    backlog: VecDeque::new(),
                }),
            }),
        }
    }

    /// Makes the engine play this project. Call it again after every edit:
    /// sounding notes carry on, and gain, pan and mute changes glide.
    ///
    /// Samples missing from `pool` leave their channels silent. If the
    /// transport's pattern is not in the project, the transport moves to the
    /// project's first pattern.
    pub fn set_project(&self, project: &Project, pool: &SamplePool) {
        self.set_plan(Arc::new(compile(project, pool)));
        let mut state = self.lock();
        let pattern = state.transport.pattern;
        if project.pattern(pattern).is_none()
            && let Some(first) = project.patterns.first()
        {
            state.transport.pattern = first.id;
            state.send_transport();
        }
    }

    pub(crate) fn set_plan(&self, plan: Arc<Plan>) {
        let mut state = self.lock();
        state.plan = plan.clone();
        state.send(Message::SetPlan {
            state: Box::new(PlanState::new(&plan)),
            plan,
        });
    }

    /// Starts playback from the playhead. Does nothing while playing.
    pub fn play(&self) {
        self.start(None);
    }

    /// Plays the pattern or song `passes` times, then stops without cutting
    /// off what is still sounding. The offline renderer uses this.
    pub(crate) fn play_passes(&self, passes: u32) {
        self.start(Some(passes));
    }

    fn start(&self, passes: Option<u32>) {
        let mut state = self.lock();
        if state.link.is_none() || self.playing(&state) {
            return;
        }
        state.transport.playing = true;
        state.sequence = state.sequence.wrapping_add(1);
        let sequence = state.sequence;
        state.send(Message::Play { sequence, passes });
    }

    /// Stops playback, fades out every note, and returns the playhead to
    /// where playback last started.
    pub fn stop(&self) {
        let mut state = self.lock();
        state.transport.playing = false;
        state.sequence = state.sequence.wrapping_add(1);
        let sequence = state.sequence;
        state.send(Message::Stop { sequence });
    }

    /// Moves the playhead to `tick`. While playing, sequenced notes fade out
    /// and playback carries on from there.
    pub fn seek(&self, tick: f64) {
        let tick = if tick.is_finite() { tick.max(0.0) } else { 0.0 };
        let mut state = self.lock();
        // Shown at once, and kept for the next stream when none is open.
        self.inner.shared.set_tick(tick);
        state.send(Message::Seek(tick));
    }

    /// Changes the play mode, the pattern that plays in pattern mode, or
    /// song looping. Changing the mode moves the playhead to the start.
    pub fn set_transport(&self, patch: TransportPatch) {
        let mut state = self.lock();
        if let Some(mode) = patch.mode {
            if mode != state.transport.mode {
                self.inner.shared.set_tick(0.0);
            }
            state.transport.mode = mode;
        }
        if let Some(pattern) = patch.pattern {
            state.transport.pattern = pattern;
        }
        if let Some(loop_song) = patch.loop_song {
            state.transport.loop_song = loop_song;
        }
        state.send_transport();
    }

    pub fn transport(&self) -> TransportState {
        let mut state = self.lock();
        state.maintain();
        TransportState {
            playing: self.playing(&state),
            ..state.transport
        }
    }

    /// Plays a note on a channel right away, as from the UI keyboard.
    /// `velocity` runs from 0 to 1.
    pub fn note_on(&self, channel: ChannelId, key: u8, velocity: f32) {
        let velocity = if velocity.is_finite() {
            velocity.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.lock().send(Message::NoteOn {
            channel,
            key: key.min(MAX_KEY),
            velocity,
        });
    }

    /// Ends a note started with [`Controller::note_on`]. A sampler with no
    /// envelope plays its sample to the end regardless.
    pub fn note_off(&self, channel: ChannelId, key: u8) {
        self.lock().send(Message::NoteOff {
            channel,
            key: key.min(MAX_KEY),
        });
    }

    /// Plays a sample once, straight into the master track, whether or not
    /// the transport is running. A new preview replaces the one playing.
    pub fn preview(&self, sample: AudioBuffer) {
        self.lock().send(Message::Preview(sample));
    }

    pub fn stop_preview(&self) {
        self.lock().send(Message::StopPreview);
    }

    /// Sets a gain applied at the very end of the chain, after the master
    /// track and its meter. At zero the engine still does all its work and
    /// the meters still move, but nothing is heard.
    pub fn set_output_gain(&self, gain: f32) {
        let gain = if gain.is_finite() {
            gain.clamp(0.0, MAX_GAIN)
        } else {
            0.0
        };
        let mut state = self.lock();
        state.output_gain = gain;
        state.send(Message::SetOutputGain(gain));
    }

    /// The playhead, meters and load for the UI. Reading the meters resets
    /// them, so each frame reports the peaks since the frame before.
    pub fn frame(&self) -> RealtimeFrame {
        let mut state = self.lock();
        state.maintain();
        let shared = &self.inner.shared;
        RealtimeFrame {
            playing: self.playing(&state),
            tick: shared.tick(),
            meters: (0..state.plan.tracks.len() * 2)
                .map(|index| shared.take_meter(index))
                .collect(),
            cpu: shared.load().clamp(0.0, 1.0),
            xruns: shared.xruns(),
            voices: shared.voices(),
        }
    }

    /// What the device callback has measured. Reading resets `cpu_peak`.
    pub fn stream_stats(&self) -> StreamStats {
        let shared = &self.inner.shared;
        StreamStats {
            frames: shared.frames(),
            xruns: shared.xruns(),
            cpu: shared.load(),
            cpu_peak: shared.take_load_peak(),
        }
    }

    pub(crate) fn shared(&self) -> &Arc<Shared> {
        &self.inner.shared
    }

    /// Creates the processor for a new stream and makes it the one this
    /// controller talks to. It starts out stopped, with the current project,
    /// transport settings and playhead.
    pub(crate) fn attach(&self, sample_rate: u32) -> Processor {
        let (message_tx, message_rx) = RingBuffer::new(MESSAGE_CAPACITY);
        let (garbage_tx, garbage_rx) = RingBuffer::new(GARBAGE_CAPACITY);
        let shared = self.inner.shared.clone();
        let processor = Processor::with_queues(sample_rate, message_rx, garbage_tx, shared);

        let mut state = self.lock();
        state.link = Some(Link {
            messages: message_tx,
            garbage: garbage_rx,
        });
        state.backlog.clear();
        state.transport.playing = false;
        self.inner.shared.publish_transport(state.sequence, false);

        let plan = state.plan.clone();
        state.send(Message::SetPlan {
            state: Box::new(PlanState::new(&plan)),
            plan,
        });
        state.send_transport();
        // The playhead stays where the previous stream, or a seek made with
        // no stream open, left it.
        state.send(Message::Seek(self.inner.shared.tick()));
        let output_gain = state.output_gain;
        state.send(Message::SetOutputGain(output_gain));
        processor
    }

    /// Forgets the processor after its stream is gone, so requests stop
    /// queueing up for it.
    pub(crate) fn detach(&self) {
        let mut state = self.lock();
        state.link = None;
        state.backlog.clear();
        state.transport.playing = false;
        let shared = &self.inner.shared;
        shared.publish_transport(state.sequence, false);
        shared.publish_position(shared.tick(), 0);
    }

    /// Whether the transport is playing: the audio thread's word once it
    /// has handled the latest play or stop, and the request until then.
    fn playing(&self, state: &State) -> bool {
        let (sequence, playing) = self.inner.shared.transport();
        if sequence == state.sequence {
            playing
        } else {
            state.transport.playing
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        // The state stays usable if a thread panicked while holding it.
        self.inner
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl State {
    /// Queues a message for the audio thread. Without a stream it is
    /// dropped: what a later stream needs is kept in this struct instead.
    fn send(&mut self, message: Message) {
        self.maintain();
        let Some(link) = &mut self.link else {
            return;
        };
        if !self.backlog.is_empty() {
            self.backlog.push_back(message);
        } else if let Err(PushError::Full(message)) = link.messages.push(message) {
            self.backlog.push_back(message);
        }
    }

    fn send_transport(&mut self) {
        self.send(Message::SetTransport {
            mode: self.transport.mode,
            pattern: self.transport.pattern,
            loop_song: self.transport.loop_song,
        });
    }

    /// Drops what the audio thread handed back, then moves waiting messages
    /// into the queue while there is room. Every call into the controller
    /// does this, so memory is freed here rather than on the audio thread.
    fn maintain(&mut self) {
        let Some(link) = &mut self.link else {
            return;
        };
        while link.garbage.pop().is_ok() {}
        while let Some(message) = self.backlog.pop_front() {
            if let Err(PushError::Full(message)) = link.messages.push(message) {
                self.backlog.push_front(message);
                break;
            }
        }
    }
}
