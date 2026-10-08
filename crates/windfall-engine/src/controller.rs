//! The control side of the engine: everything other threads may ask of the
//! audio thread.

use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, MutexGuard};

use rtrb::{Consumer, Producer, PushError, RingBuffer};
use windfall_core::AudioBuffer;
use windfall_ipc::{
    AutomatedValue, GainReduction, PlayMode, RealtimeFrame, TransportPatch, TransportState,
};
use windfall_project::{AutomationId, ChannelId, MAX_GAIN, MAX_KEY, PatternId, Project};

use crate::message::{GARBAGE_CAPACITY, Garbage, MESSAGE_CAPACITY, Message};
use crate::plan::{Plan, compile};
use crate::pool::SamplePool;
use crate::processor::Processor;
use crate::shared::Shared;
use crate::state::{Ledger, PlanState};

/// An immutable project compiled on a worker, ready for a short installation.
pub struct PreparedProject {
    plan: Plan,
    sampler_pool: SamplePool,
}

impl PreparedProject {
    pub fn sampler_pool(&self) -> &SamplePool {
        &self.sampler_pool
    }
}

/// Sends requests to the audio thread and reads back what it publishes.
///
/// Clones share one engine. A controller outlives device changes. When the
/// engine replaces its stream because the audio settings changed, the
/// controller hands the project, transport settings and playhead to the new
/// stream's processor, and playback that was running carries on from the
/// playhead. When a stream is lost and none could be opened in its place,
/// playback stops and the playhead returns to where playback started, as
/// it does on [`Controller::stop`].
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
    sampler_error: Mutex<Option<crate::sampler_processing::SamplerPreparationError>>,
}

struct State {
    link: Option<Link>,
    /// The plan most recently compiled, kept to hand to a new processor.
    plan: Arc<Plan>,
    /// What the attached processor holds once it has taken `plan`: which
    /// effects and instruments, how long its delay lines are, its meters.
    /// `None` while no processor is attached.
    hosted: Option<Ledger>,
    /// The transport as last requested. `playing` here is what was asked
    /// for; the audio thread has the final say once it has caught up.
    transport: TransportState,
    /// Counts play and stop requests, so the audio thread's published
    /// state can be matched to the request it answers.
    sequence: u32,
    output_gain: f32,
    /// Messages that did not fit in the queue, oldest first.
    backlog: VecDeque<Message>,
    /// The stream that went away was playing, and the one that takes its
    /// place carries on. [`Controller::suspend`] decides this and
    /// [`Controller::detach`] calls it off.
    resume: bool,
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
                sampler_error: Mutex::new(None),
                state: Mutex::new(State {
                    link: None,
                    plan: Arc::new(Plan::empty()),
                    hosted: None,
                    transport: TransportState {
                        playing: false,
                        mode: PlayMode::Pattern,
                        pattern: PatternId(0),
                        loop_song: false,
                    },
                    sequence: 0,
                    output_gain: 1.0,
                    backlog: VecDeque::new(),
                    resume: false,
                }),
            }),
        }
    }

    /// Makes the engine play this project. Call it again after every edit:
    /// sounding notes carry on, and gain, pan and mute changes glide on
    /// whatever those notes are heard through. A channel or mixer track
    /// that nothing is sounding through takes its new values at once, and
    /// notes that are already fading out, as after [`Controller::stop`], do
    /// not count where only faders lie between them and the output: they
    /// finish at the levels they had. So the first project, and one that
    /// replaces another, play at their own levels from the first frame.
    ///
    /// Effects and instruments are told apart by their ids. One the engine
    /// already runs keeps running across the edit with everything it
    /// remembers, such as a reverb's tail, and is only told what changed
    /// about it; it glides to new settings. One that is new is built and
    /// prepared here, on the caller's thread, which allocates and can take
    /// a moment. An effect that joins or leaves a track sound is passing
    /// through fades in or out over 5 ms, and an instrument whose channel
    /// is gone stops with a short fade.
    ///
    /// Samples missing from `pool` leave their channels silent. If the
    /// transport's pattern is not in the project, the transport moves to the
    /// project's first pattern.
    pub fn set_project(&self, project: &Project, pool: &SamplePool) {
        match Self::prepare_project(project, pool) {
            Ok(prepared) => {
                self.set_prepared_project(project, prepared);
            }
            Err(error) => {
                *self
                    .inner
                    .sampler_error
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(error);
            }
        }
    }

    /// A refused convenience publication keeps the old plan and exposes its reason.
    pub fn sampler_preparation_error(
        &self,
    ) -> Option<crate::sampler_processing::SamplerPreparationError> {
        self.inner
            .sampler_error
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn sampler_key_supported(&self, channel: ChannelId, key: u8) -> bool {
        let state = self.lock();
        state.plan.channel(channel).is_some_and(|index| {
            let sampler = &state.plan.channels[index].sampler;
            !sampler.spectral
                || sampler
                    .bank
                    .as_ref()
                    .is_some_and(|bank| bank.at(key).is_some())
        })
    }

    /// Compiles and renders spectral clips on the caller's worker/control thread.
    pub fn prepare_project(
        project: &Project,
        pool: &SamplePool,
    ) -> Result<PreparedProject, crate::sampler_processing::SamplerPreparationError> {
        Self::try_prepare_project(project, pool)
    }

    /// Fallible bounded preparation. Must run outside document/audio locks.
    pub fn try_prepare_project(
        project: &Project,
        pool: &SamplePool,
    ) -> Result<PreparedProject, crate::sampler_processing::SamplerPreparationError> {
        let pool = pool.prepare_samplers(project, &mut || true, &mut |_, _, _| {})?;
        Ok(Self::compile_prepared_project(project, &pool))
    }

    /// Compile already prepared sampler banks; never performs sampler DSP.
    pub fn compile_prepared_project(project: &Project, pool: &SamplePool) -> PreparedProject {
        PreparedProject {
            plan: compile(project, pool),
            sampler_pool: pool.clone(),
        }
    }

    /// Installs a precompiled snapshot. The caller must verify it still matches the project.
    pub fn set_prepared_project(&self, project: &Project, prepared: PreparedProject) {
        *self
            .inner
            .sampler_error
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        self.set_plan(prepared.plan);
        let mut state = self.lock();
        let pattern = state.transport.pattern;
        if project.pattern(pattern).is_none()
            && let Some(first) = project.patterns.first()
        {
            state.transport.pattern = first.id;
            state.send_transport();
        }
    }

    pub(crate) fn set_plan(&self, mut plan: Plan) {
        let mut state = self.lock();
        let held = state.hosted.as_ref().filter(|_| state.link.is_some());
        let Some(held) = held else {
            // With no stream there is nobody to tell: the next processor
            // starts out on the plan kept here.
            plan.snapshot_native_owners();
            state.plan = Arc::new(plan);
            return;
        };
        plan.keep_leaving(&state.plan, Some(held));
        let plan = Arc::new(plan);
        let (plan_state, hosted) = PlanState::build(&plan, held.sample_rate, Some(held));
        state.hosted = Some(hosted);
        state.plan = plan.clone();
        state.send(Message::SetPlan {
            state: Box::new(plan_state),
            plan,
        });
    }

    /// Frames by which the project's instruments and effects delay the
    /// output of the stream that is open, at its sample rate. Zero with no
    /// stream. Every path through the mixer is delayed to match the slowest
    /// one, so this one figure holds for the whole mix.
    pub fn latency_frames(&self) -> u32 {
        let state = self.lock();
        state.hosted.as_ref().map_or(0, |hosted| hosted.latency)
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
        state.send(Message::Play {
            sequence,
            passes,
            from: None,
        });
    }

    /// Stops playback, fades out every note, and returns the playhead to
    /// where playback last started. What automation was moving stays where
    /// the song had it until everything has rung out. Stopping again
    /// returns it to the stored values right away, over 100 ms.
    pub fn stop(&self) {
        let mut state = self.lock();
        self.panic_hardware();
        state.transport.playing = false;
        state.sequence = state.sequence.wrapping_add(1);
        let sequence = state.sequence;
        state.send(Message::Stop { sequence });
        if state.link.is_none() {
            // No audio thread is there to act on this, so the playhead is
            // returned here, and a stream on its way does not start playing.
            state.resume = false;
            let shared = &self.inner.shared;
            shared.set_tick(shared.start());
        }
    }

    /// Moves the playhead to `tick`. While playing, sequenced notes fade out
    /// and playback carries on from there.
    pub fn seek(&self, tick: f64) {
        let tick = if tick.is_finite() { tick.max(0.0) } else { 0.0 };
        let mut state = self.lock();
        // Shown at once, and kept for the next stream when none is open.
        self.inner.shared.set_tick(tick);
        self.inner.shared.set_start(tick);
        state.send(Message::Seek(tick));
    }

    /// Changes the play mode, the pattern that plays in pattern mode, or
    /// song looping. Changing the mode moves the playhead to the start.
    pub fn set_transport(&self, patch: TransportPatch) {
        let mut state = self.lock();
        if let Some(mode) = patch.mode {
            if mode != state.transport.mode {
                self.inner.shared.set_tick(0.0);
                self.inner.shared.set_start(0.0);
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

    /// Hardware input uses this epoch both at capture and at audio delivery.
    pub fn hardware_epoch(&self) -> u64 {
        self.inner.shared.hardware_epoch.load(Ordering::Acquire)
    }

    /// Queue-independent panic. The next audio buffer fades hardware voices and
    /// discards old hardware messages, even when its message/garbage queues are full.
    pub fn panic_hardware(&self) {
        self.inner
            .shared
            .hardware_epoch
            .fetch_add(1, Ordering::AcqRel);
    }

    /// Control-worker only. Never grows the general control backlog for MIDI.
    /// False means unavailable/full/stale; the MIDI worker must panic on failure.
    pub fn hardware_note(&self, epoch: u64, channel: ChannelId, key: u8, velocity: u8) -> bool {
        let mut state = self.lock();
        state.maintain();
        if epoch != self.hardware_epoch()
            || key > MAX_KEY
            || velocity > 127
            || !state.backlog.is_empty()
            || state.plan.channel(channel).is_none()
        {
            return false;
        }
        state.link.as_mut().is_some_and(|link| {
            link.messages
                .push(Message::HardwareNote {
                    epoch,
                    channel,
                    key,
                    velocity,
                })
                .is_ok()
        })
    }

    /// Plays a sample once, straight into the master track, whether or not
    /// the transport is running. A new preview replaces the one playing.
    /// Previews play at [`PREVIEW_GAIN_DB`](crate::PREVIEW_GAIN_DB), so one
    /// heard over a full mix does not clip the master.
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
    ///
    /// The playhead is where the sound now leaving the engine is in the
    /// song: the latency of instruments and effects is taken off it, so it
    /// is no further ahead of what is heard than the device's own buffer
    /// puts it.
    pub fn frame(&self) -> RealtimeFrame {
        let mut state = self.lock();
        state.maintain();
        let shared = &self.inner.shared;
        let meters = state
            .hosted
            .as_ref()
            .map_or(&[][..], |hosted| &hosted.meters);
        let mut automated = Vec::new();
        if state.link.is_some() {
            shared.automated(&mut automated);
        }
        RealtimeFrame {
            playing: self.playing(&state),
            tick: shared.tick(),
            meters: (0..state.plan.tracks.len() * 2)
                .map(|index| shared.take_meter(index))
                .collect(),
            cpu: shared.load().clamp(0.0, 1.0),
            xruns: shared.xruns(),
            voices: shared.voices(),
            audio_clips: shared.audio_clips(),
            dropped_clips: shared.dropped_clips(),
            gain_reductions: meters
                .iter()
                .map(|(effect, meter)| GainReduction {
                    effect: *effect,
                    db: meter.take_db(),
                })
                .collect(),
            automated: automated
                .into_iter()
                .map(|(automation, value)| AutomatedValue {
                    automation: AutomationId(automation),
                    value,
                })
                .collect(),
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
    /// controller talks to. It starts on the current project and transport
    /// settings, with stop returning to where it did before. After
    /// [`Controller::suspend`] found the transport playing, it carries on
    /// from the playhead; otherwise it starts out stopped.
    ///
    /// May be called again before any audio was processed, as when a stream
    /// could not be built and another kind is tried: the processor made
    /// last is the one that counts.
    pub(crate) fn attach(&self, sample_rate: u32) -> Processor {
        let (message_tx, message_rx) = RingBuffer::new(MESSAGE_CAPACITY);
        let (garbage_tx, garbage_rx) = RingBuffer::new(GARBAGE_CAPACITY);
        let shared = self.inner.shared.clone();

        let mut state = self.lock();
        self.panic_hardware();
        // Every effect and instrument is built anew, prepared for this
        // stream's sample rate. What the last stream ran went with it.
        // The returned ledger freezes the identities actually prepared,
        // including revisions changed since this plan was installed.
        let (plan_state, hosted) = PlanState::build(&state.plan, sample_rate, None);
        let processor = Processor::with_queues(
            sample_rate,
            state.plan.clone(),
            Box::new(plan_state),
            state.output_gain,
            message_rx,
            garbage_tx,
            shared,
        );
        state.hosted = Some(hosted);
        state.link = Some(Link {
            messages: message_tx,
            garbage: garbage_rx,
        });
        state.backlog.clear();
        // What the last stream's processor published about its automation
        // is not true of this one, which has yet to play anything.
        self.inner.shared.publish_automated(std::iter::empty());
        state.send_transport();

        let shared = &self.inner.shared;
        // Where stop returns to is not where the playhead was when the last
        // stream went away, so the two travel separately.
        state.send(Message::Seek(shared.start()));
        if state.resume {
            state.transport.playing = true;
            state.sequence = state.sequence.wrapping_add(1);
            let sequence = state.sequence;
            state.send(Message::Play {
                sequence,
                passes: None,
                from: Some(shared.tick()),
            });
        } else {
            state.transport.playing = false;
            shared.publish_transport(state.sequence, false);
        }
        processor
    }

    /// Lets go of the processor of a stream that is being replaced on
    /// purpose. If the transport was playing, the processor of the next
    /// stream carries on from where the playhead is now.
    pub(crate) fn suspend(&self) {
        let mut state = self.lock();
        self.panic_hardware();
        state.resume = self.playing(&state);
        state.link = None;
        state.hosted = None;
        state.backlog.clear();
        let shared = &self.inner.shared;
        shared.publish_position(shared.tick(), shared.start(), 0);
        shared.publish_clips(0, 0);
    }

    /// Forgets the processor after its stream failed, or after no stream
    /// could be opened, so requests stop queueing up for it. Playback is
    /// over: the transport reads stopped and the playhead goes back to
    /// where playback started.
    ///
    /// This frees whatever was still queued for the processor, so it is for
    /// the control side only.
    pub(crate) fn detach(&self) {
        let mut state = self.lock();
        self.panic_hardware();
        state.link = None;
        state.hosted = None;
        state.backlog.clear();
        state.resume = false;
        state.transport.playing = false;
        let shared = &self.inner.shared;
        shared.publish_transport(state.sequence, false);
        shared.publish_position(shared.start(), shared.start(), 0);
        shared.publish_clips(0, 0);
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

#[cfg(test)]
mod tests {
    use windfall_core::TICKS_PER_STEP;
    use windfall_project::{
        Channel, ChannelSource, Lane, Note, NoteId, SampleId, SamplerSettings, TrackId,
    };

    use super::*;

    /// A project and pool with one channel that plays a single full-scale
    /// frame on each of `steps`.
    fn clicks(steps: &[u32]) -> (Project, SamplePool) {
        let mut project = Project::new("clicks");
        let mut pool = SamplePool::new();
        pool.insert(
            SampleId(900),
            AudioBuffer::from_interleaved(48_000, 1, vec![1.0]),
        );
        project.channels.push(Channel {
            id: ChannelId(901),
            name: String::new(),
            color: 0,
            volume: 1.0,
            pan: 0.0,
            muted: false,
            solo: false,
            mixer_track: TrackId::MASTER,
            source: ChannelSource::Sampler(SamplerSettings {
                sample: Some(SampleId(900)),
                ..SamplerSettings::default()
            }),
        });
        project.patterns[0].lanes.push(Lane {
            channel: ChannelId(901),
            notes: steps
                .iter()
                .map(|step| Note {
                    id: NoteId(1_000 + step),
                    start: step * TICKS_PER_STEP,
                    length: TICKS_PER_STEP,
                    key: 60,
                    velocity: 1.0,
                    pan: 0.0,
                })
                .collect(),
        });
        (project, pool)
    }

    /// Processes `frames` frames and returns the frames that were not
    /// silent.
    fn run(processor: &mut Processor, frames: usize) -> Vec<usize> {
        let mut out = vec![0.0; frames * 2];
        for block in out.chunks_mut(256) {
            processor.process(block);
        }
        (0..frames).filter(|frame| out[frame * 2] != 0.0).collect()
    }

    #[derive(Debug, Default)]
    struct R6Native {
        revision: std::sync::atomic::AtomicU64,
        creates: std::sync::atomic::AtomicUsize,
        prepared_revision: [std::sync::atomic::AtomicU64; 8],
        processes: [std::sync::atomic::AtomicUsize; 8],
        drops: [std::sync::atomic::AtomicUsize; 8],
    }
    #[derive(Debug)]
    struct R6Factory(Arc<R6Native>);
    impl crate::plugins::PluginFactory for R6Factory {
        fn revision(&self) -> u64 {
            self.0.revision.load(Ordering::Relaxed)
        }
        fn effect(
            &self,
            _: &windfall_project::PluginBinding,
            _: u32,
            _: usize,
        ) -> Result<Box<dyn crate::plugins::HostedEffect>, String> {
            let owner = self.0.creates.fetch_add(1, Ordering::Relaxed);
            self.0.prepared_revision[owner].store(self.revision(), Ordering::Relaxed);
            Ok(Box::new(R6Delay {
                ring: [[0.0; 2]; 32],
                write: 0,
                owner,
                stats: self.0.clone(),
            }))
        }
        fn instrument(
            &self,
            _: &windfall_project::PluginBinding,
            _: u32,
            _: usize,
        ) -> Result<Box<dyn crate::plugins::HostedInstrument>, String> {
            Err("effect fixture".into())
        }
    }
    struct R6Delay {
        ring: [[f32; 2]; 32],
        write: usize,
        owner: usize,
        stats: Arc<R6Native>,
    }
    impl Drop for R6Delay {
        fn drop(&mut self) {
            self.stats.drops[self.owner].fetch_add(1, Ordering::Relaxed);
        }
    }
    impl crate::plugins::HostedEffect for R6Delay {
        fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
            self.stats.processes[self.owner].fetch_add(1, Ordering::Relaxed);
            for (left, right) in left.iter_mut().zip(right) {
                let old = self.ring[self.write];
                self.ring[self.write] = [*left, *right];
                self.write = (self.write + 1) % 32;
                [*left, *right] = old;
            }
        }
        fn set_param(&mut self, _: u32, _: f32) {}
        fn set_tempo(&mut self, _: f32) {}
        fn latency(&self) -> usize {
            32
        }
        fn tail(&self) -> usize {
            32
        }
    }
    fn r6_fixture(cancel: bool) -> (Project, SamplePool, Arc<R6Native>) {
        use windfall_project::{EffectId, EffectParams, EffectSlot, PluginBinding, PluginTarget};
        let (mut project, mut pool) = clicks(&[0]);
        let tone: Vec<f32> = (0..48_000)
            .map(|n| (std::f64::consts::TAU * n as f64 * 173.0 / 48_000.0).sin() as f32 * 0.5)
            .collect();
        pool.insert(
            SampleId(900),
            AudioBuffer::from_interleaved(48_000, 1, tone.clone()),
        );
        let mut first = project.mixer.tracks[0].clone();
        first.id = TrackId(11);
        first.output = Some(TrackId::MASTER);
        first.effects = vec![
            EffectSlot {
                id: EffectId(20),
                enabled: true,
                mix: 1.0,
                params: windfall_project::EffectKind::Balance.default_params(),
            },
            EffectSlot {
                id: EffectId(21),
                enabled: true,
                mix: 1.0,
                params: EffectParams::StereoMatrix(windfall_dsp::StereoMatrixParams {
                    left_delay_ms: 1.0,
                    right_delay_ms: 1.0,
                    ..Default::default()
                }),
            },
        ];
        project.mixer.tracks.push(first);
        project.channels[0].mixer_track = TrackId(11);
        project.channels[0].volume = 0.5;
        if cancel {
            let mut second = project.mixer.tracks[0].clone();
            second.id = TrackId(12);
            second.output = Some(TrackId::MASTER);
            project.mixer.tracks.push(second);
            pool.insert(
                SampleId(901),
                AudioBuffer::from_interleaved(
                    48_000,
                    1,
                    tone.iter().map(|sample| -*sample).collect(),
                ),
            );
            let mut channel = project.channels[0].clone();
            channel.id = ChannelId(902);
            channel.mixer_track = TrackId(12);
            channel.source = ChannelSource::Sampler(SamplerSettings {
                sample: Some(SampleId(901)),
                ..Default::default()
            });
            project.channels.push(channel);
            let mut lane = project.patterns[0].lanes[0].clone();
            lane.channel = ChannelId(902);
            lane.notes[0].id = NoteId(1100);
            project.patterns[0].lanes.push(lane);
        }
        project.plugins.push(PluginBinding {
            target: PluginTarget::Effect {
                effect: EffectId(20),
            },
            format: "clap".into(),
            path: "r6-detached-delay.clap".into(),
            id: "r6-delay".into(),
            name: "Prepared delay".into(),
            state: vec![],
            parameters: vec![],
        });
        let stats = Arc::new(R6Native::default());
        pool.set_plugin_factory(Arc::new(R6Factory(stats.clone())));
        (project, pool, stats)
    }
    fn r6_audio(processor: &mut Processor, frames: usize, block: usize) -> Vec<f32> {
        let mut audio = vec![0.0; frames * 2];
        for out in audio.chunks_mut(block * 2) {
            assert_eq!(
                crate::test_alloc::allocator_calls(|| processor.process(out)),
                0
            );
        }
        audio
    }
    fn r6_detached_precompiled_adoption(cancel: bool, revise_before_attach: bool) {
        let (mut project, pool, stats) = r6_fixture(cancel);
        let controller = Controller::new();
        let prepared = Controller::prepare_project(&project, &pool);
        stats.revision.store(1, Ordering::Relaxed);
        controller.set_prepared_project(&project, prepared);
        if revise_before_attach {
            stats.revision.store(2, Ordering::Relaxed);
        }
        let mut processor = controller.attach(48_000);
        controller.play();
        let prime = r6_audio(&mut processor, 4092, 137);
        if cancel {
            assert!(prime.iter().all(|sample| sample.abs() < 1e-6));
        }
        assert_eq!(controller.latency_frames(), 80);
        assert_eq!(stats.creates.load(Ordering::Relaxed), 1);
        assert_eq!(
            stats.prepared_revision[0].load(Ordering::Relaxed),
            if revise_before_attach { 2 } else { 1 }
        );
        let mut audio = prime[prime.len() - 2..].to_vec();
        let before = stats.processes[0].load(Ordering::Relaxed);
        controller.set_project(&project, &pool);
        let adopted = r6_audio(&mut processor, 1000, 1);
        if cancel {
            let residual = adopted.iter().copied().map(f32::abs).fold(0.0, f32::max);
            println!("detached prepared adoption residual {residual}");
            assert!(
                residual < 1e-6,
                "detached prepared adoption residual {residual}"
            );
        }
        audio.extend(adopted);
        if !cancel {
            let step = audio
                .as_chunks::<2>()
                .0
                .windows(2)
                .map(|frames| (frames[1][0] - frames[0][0]).abs())
                .fold(0.0, f32::max);
            println!("detached prepared adoption solo step {step}");
            assert!(step < 0.04, "detached prepared adoption solo step {step}");
        }
        assert!(
            stats.processes[0].load(Ordering::Relaxed) > before,
            "running native owner stopped processing"
        );
        controller.frame();
        assert_eq!(stats.creates.load(Ordering::Relaxed), 1);
        assert_eq!(stats.drops[0].load(Ordering::Relaxed), 0);
        for block in [7, 137, 1] {
            let before = stats.processes[0].load(Ordering::Relaxed);
            if block == 137 {
                controller
                    .set_prepared_project(&project, Controller::prepare_project(&project, &pool));
            } else {
                controller.set_project(&project, &pool);
            }
            let continuation = r6_audio(&mut processor, 137, block);
            if cancel {
                assert!(continuation.iter().all(|sample| sample.abs() < 1e-6));
            }
            assert!(stats.processes[0].load(Ordering::Relaxed) > before);
            assert_eq!(controller.latency_frames(), 80);
            controller.frame();
            assert_eq!(stats.creates.load(Ordering::Relaxed), 1);
            assert_eq!(stats.drops[0].load(Ordering::Relaxed), 0);
        }
        project.mixer.tracks[1].effects.remove(0);
        project.plugins.clear();
        controller.set_project(&project, &pool);
        let removal = r6_audio(&mut processor, 1000, 29);
        if cancel {
            assert!(removal.iter().all(|sample| sample.abs() < 1e-6));
        }
        controller.set_project(&project, &pool);
        r6_audio(&mut processor, 137, 137);
        controller.frame();
        assert_eq!(stats.drops[0].load(Ordering::Relaxed), 1);
        assert_eq!(controller.latency_frames(), 48);
    }
    #[test]
    fn utility_r6_detached_precompiled_adoption_keeps_native_cancellation() {
        r6_detached_precompiled_adoption(true, false);
    }
    #[test]
    fn utility_r6_detached_precompiled_adoption_keeps_native_continuity_and_owner() {
        r6_detached_precompiled_adoption(false, false);
    }

    #[test]
    fn utility_r6_revision_after_detached_install_uses_actual_preparation() {
        r6_detached_precompiled_adoption(true, true);
    }

    #[test]
    fn utility_r6_attachment_retry_and_reopen_freeze_native_identity() {
        let (project, pool, stats) = r6_fixture(true);
        let controller = Controller::new();
        controller.set_project(&project, &pool);
        stats.revision.store(1, Ordering::Relaxed);
        let abandoned = controller.attach(48_000);
        assert_eq!(stats.prepared_revision[0].load(Ordering::Relaxed), 1);
        drop(abandoned);
        assert_eq!(stats.processes[0].load(Ordering::Relaxed), 0);
        assert_eq!(stats.drops[0].load(Ordering::Relaxed), 1);

        stats.revision.store(2, Ordering::Relaxed);
        let mut processor = controller.attach(48_000);
        controller.play();
        let prime = r6_audio(&mut processor, 4092, 29);
        assert!(prime.iter().all(|sample| sample.abs() < 1e-6));
        assert_eq!(stats.prepared_revision[1].load(Ordering::Relaxed), 2);
        let before = stats.processes[1].load(Ordering::Relaxed);
        controller.set_project(&project, &pool);
        let adopted = r6_audio(&mut processor, 1000, 7);
        assert!(adopted.iter().all(|sample| sample.abs() < 1e-6));
        assert!(stats.processes[1].load(Ordering::Relaxed) > before);
        controller.frame();
        assert_eq!(controller.latency_frames(), 80);
        assert_eq!(stats.creates.load(Ordering::Relaxed), 2);
        assert_eq!(stats.drops[1].load(Ordering::Relaxed), 0);

        controller.suspend();
        drop(processor);
        assert_eq!(stats.drops[1].load(Ordering::Relaxed), 1);
        stats.revision.store(3, Ordering::Relaxed);
        let mut processor = controller.attach(48_000);
        let resumed = r6_audio(&mut processor, 4092, 137);
        assert!(resumed.iter().all(|sample| sample.abs() < 1e-6));
        assert_eq!(stats.prepared_revision[2].load(Ordering::Relaxed), 3);
        let before = stats.processes[2].load(Ordering::Relaxed);
        controller.set_prepared_project(&project, Controller::prepare_project(&project, &pool));
        let adopted = r6_audio(&mut processor, 1000, 1);
        assert!(adopted.iter().all(|sample| sample.abs() < 1e-6));
        assert!(stats.processes[2].load(Ordering::Relaxed) > before);
        controller.frame();
        assert_eq!(controller.latency_frames(), 80);
        assert_eq!(stats.creates.load(Ordering::Relaxed), 3);
        assert_eq!(stats.drops[2].load(Ordering::Relaxed), 0);
        controller.detach();
        drop(processor);
        assert_eq!(stats.drops[2].load(Ordering::Relaxed), 1);
    }

    /// A controller playing from tick 240, with its playhead at tick 604 as
    /// the stream is about to go away.
    fn playing_at_604() -> (Processor, Controller) {
        let (mut processor, controller) = Processor::new(48_000);
        let (project, pool) = clicks(&[2, 3, 4]);
        controller.set_project(&project, &pool);
        controller.seek(240.0);
        controller.play();
        // 9100 frames at 25 frames a tick are 364 ticks.
        assert_eq!(run(&mut processor, 9_100), [6_000]);
        assert_eq!(controller.frame().tick, 604.0);
        (processor, controller)
    }

    #[test]
    fn new_settings_keep_playing_and_keep_the_place_stop_returns_to() {
        let (processor, controller) = playing_at_604();
        controller.suspend();
        drop(processor);
        assert!(controller.transport().playing);
        assert_eq!(controller.frame().tick, 604.0);

        // The new stream runs at another rate: 22.96875 frames a tick.
        let mut processor = controller.attach(44_100);
        assert!(controller.transport().playing);
        // Playback picks up at tick 604. Step 3, at tick 720, comes 116
        // ticks later, and step 2, which is behind, does not play again.
        assert_eq!(run(&mut processor, 4_410), [2_665]);
        let frame = controller.frame();
        assert!(frame.playing);
        assert!((frame.tick - (604.0 + 192.0)).abs() < 1e-9);

        controller.stop();
        run(&mut processor, 64);
        assert!(!controller.transport().playing);
        assert_eq!(controller.frame().tick, 240.0);
        controller.play();
        run(&mut processor, 64);
        controller.stop();
        run(&mut processor, 64);
        assert_eq!(controller.frame().tick, 240.0);
    }

    #[test]
    fn a_stream_built_twice_before_it_runs_still_carries_on() {
        let (processor, controller) = playing_at_604();
        controller.suspend();
        drop(processor);
        // The first kind of stream could not be built, so its processor is
        // thrown away unused and another is made.
        drop(controller.attach(96_000));
        let mut processor = controller.attach(48_000);
        assert_eq!(run(&mut processor, 4_800), [116 * 25]);
        assert!(controller.frame().playing);
        controller.stop();
        run(&mut processor, 64);
        assert_eq!(controller.frame().tick, 240.0);
    }

    #[test]
    fn settings_that_cannot_be_opened_stop_playback_at_its_start() {
        let (processor, controller) = playing_at_604();
        controller.suspend();
        drop(processor);
        drop(controller.attach(48_000));
        controller.detach();
        assert!(!controller.transport().playing);
        let frame = controller.frame();
        assert!(!frame.playing);
        assert_eq!(frame.tick, 240.0);

        // A stream that opens later starts out stopped, at the same place.
        let mut processor = controller.attach(48_000);
        assert!(run(&mut processor, 4_800).is_empty());
        assert!(!controller.transport().playing);
        assert_eq!(controller.frame().tick, 240.0);
        controller.play();
        assert_eq!(run(&mut processor, 6_100), [6_000]);
    }

    #[test]
    fn a_lost_stream_stops_playback_at_its_start() {
        let (processor, controller) = playing_at_604();
        controller.detach();
        drop(processor);
        assert!(!controller.transport().playing);
        assert_eq!(controller.frame().tick, 240.0);
        assert_eq!(controller.frame().voices, 0);

        let mut processor = controller.attach(48_000);
        run(&mut processor, 64);
        assert!(!controller.frame().playing);
        assert_eq!(controller.frame().tick, 240.0);
    }

    #[test]
    fn stopping_between_two_streams_is_not_undone_by_the_second() {
        let (processor, controller) = playing_at_604();
        controller.suspend();
        drop(processor);
        controller.stop();
        assert!(!controller.transport().playing);
        assert_eq!(controller.frame().tick, 240.0);

        let mut processor = controller.attach(48_000);
        assert!(run(&mut processor, 4_800).is_empty());
        assert!(!controller.frame().playing);
        assert_eq!(controller.frame().tick, 240.0);
    }

    #[test]
    fn a_stopped_transport_stays_where_it_is_across_streams() {
        let (mut processor, controller) = Processor::new(48_000);
        let (project, pool) = clicks(&[0]);
        controller.set_project(&project, &pool);
        controller.seek(960.0);
        run(&mut processor, 64);
        controller.suspend();
        drop(processor);
        // A seek made while no stream is open is kept as well.
        controller.seek(480.0);

        let mut processor = controller.attach(44_100);
        run(&mut processor, 64);
        assert!(!controller.transport().playing);
        assert_eq!(controller.frame().tick, 480.0);
        controller.play();
        run(&mut processor, 64);
        controller.stop();
        run(&mut processor, 64);
        assert_eq!(controller.frame().tick, 480.0);
    }

    #[test]
    fn a_new_stream_does_not_report_the_automation_of_the_last_one() {
        use windfall_project::{
            Automation, AutomationId, AutomationPoint, AutomationTarget, Clip, ClipContent, ClipId,
            PlaylistTrack, PlaylistTrackId,
        };

        // A song of one bar in which a curve holds the master's fader.
        let (mut project, pool) = clicks(&[0]);
        let pattern = project.patterns[0].id;
        project.playlist.tracks.push(PlaylistTrack {
            id: PlaylistTrackId(950),
            name: String::new(),
            muted: false,
        });
        project.automations.push(Automation {
            id: AutomationId(960),
            name: String::new(),
            color: 0,
            target: AutomationTarget::TrackVolume {
                track: TrackId::MASTER,
            },
            points: vec![AutomationPoint {
                tick: 0,
                value: 0.5,
                curve: 0.0,
                hold: false,
            }],
        });
        let contents = [
            ClipContent::Pattern { pattern },
            ClipContent::Automation {
                automation: AutomationId(960),
            },
        ];
        for (index, content) in contents.into_iter().enumerate() {
            project.playlist.clips.push(Clip {
                id: ClipId(970 + index as u32),
                track: PlaylistTrackId(950),
                start: 0,
                length: 3_840,
                offset: 0,
                muted: false,
                content,
            });
        }
        let (mut processor, controller) = Processor::new(48_000);
        controller.set_project(&project, &pool);
        controller.set_transport(TransportPatch {
            mode: Some(PlayMode::Song),
            ..TransportPatch::default()
        });
        controller.play();
        run(&mut processor, 4_800);
        let automated = controller.frame().automated;
        assert_eq!(automated.len(), 1);
        assert_eq!(automated[0].automation, AutomationId(960));
        assert_eq!(automated[0].value, 0.5);

        // The stream is lost. With none open, nothing is reported, and the
        // one that opens later starts out stopped, with nothing automated.
        controller.detach();
        drop(processor);
        assert!(controller.frame().automated.is_empty());
        let mut processor = controller.attach(48_000);
        assert!(controller.frame().automated.is_empty());
        run(&mut processor, 480);
        assert!(controller.frame().automated.is_empty());
        controller.play();
        run(&mut processor, 480);
        assert_eq!(controller.frame().automated.len(), 1);
    }

    #[test]
    fn a_new_stream_starts_on_the_project_and_the_output_gain() {
        let (processor, controller) = Processor::new(48_000);
        let (mut project, pool) = clicks(&[0]);
        project.mixer.tracks[0].volume = 0.5;
        controller.set_project(&project, &pool);
        controller.set_output_gain(0.5);
        controller.suspend();
        drop(processor);

        // Nothing has sounded on the new stream, so neither the master
        // fader nor the output gain has anything to glide in from.
        let mut processor = controller.attach(48_000);
        controller.play();
        let mut out = [0.0; 64];
        processor.process(&mut out);
        assert_eq!(out[..2], [0.25, 0.25]);
    }
}
