//! The whole audio path with no device attached.

use std::sync::Arc;

use rtrb::{Consumer, Producer};
use windfall_core::AudioBuffer;

use crate::controller::Controller;
use crate::message::{GARBAGE_HEADROOM, Garbage, Message, retire};
use crate::mixer::{Frame, MAX_BLOCK, Mixer};
use crate::plan::{Plan, PlanState};
use crate::ramp::Ramp;
use crate::sequencer::{Sequencer, Triggers};
use crate::shared::Shared;
use crate::voice::{Note, Origin, VoicePool};

/// Time a gain or pan change takes to arrive.
const RAMP_SECONDS: f64 = 0.005;

/// Turns the current plan into audio: sequencer, sampler voices and mixer.
///
/// The device callback, the offline renderer and the tests all drive the
/// same `process`, so an export is the same audio as playback.
///
/// `process` is safe to call from a realtime thread. It never allocates,
/// frees, locks or blocks: every buffer is allocated in [`Processor::new`],
/// and whatever arrives from the [`Controller`] owning heap memory is sent
/// back to it to be dropped.
pub struct Processor {
    sample_rate: u32,
    /// Frames processed so far. It is the engine's clock.
    frame: u64,
    plan: Arc<Plan>,
    /// The moving values that belong to `plan`. Always built for it.
    state: Box<PlanState>,
    sequencer: Sequencer,
    voices: VoicePool,
    mixer: Mixer,
    triggers: Triggers,
    /// Gain applied after the master track and its meter.
    output_gain: Ramp,
    ramp_frames: u64,
    /// Sequence number of the last play or stop request handled.
    transport_sequence: u32,
    messages: Consumer<Message>,
    garbage: Producer<Garbage>,
    shared: Arc<Shared>,
}

impl Processor {
    /// Creates a processor running at `sample_rate` and the controller that
    /// drives it. The device layer does the same when it opens a stream;
    /// call this directly to run the engine without a device.
    pub fn new(sample_rate: u32) -> (Processor, Controller) {
        let controller = Controller::new();
        let processor = controller.attach(sample_rate);
        (processor, controller)
    }

    pub(crate) fn with_queues(
        sample_rate: u32,
        messages: Consumer<Message>,
        garbage: Producer<Garbage>,
        shared: Arc<Shared>,
    ) -> Self {
        let sample_rate = sample_rate.max(1);
        let plan = Arc::new(Plan::empty());
        Self {
            sample_rate,
            frame: 0,
            state: Box::new(PlanState::new(&plan)),
            sequencer: Sequencer::new(sample_rate, &plan),
            plan,
            voices: VoicePool::new(sample_rate),
            mixer: Mixer::new(),
            triggers: Triggers::new(),
            output_gain: Ramp::at_rest(1.0),
            ramp_frames: ((RAMP_SECONDS * f64::from(sample_rate)).round() as u64).max(1),
            transport_sequence: 0,
            messages,
            garbage,
            shared,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Fills `out`, interleaved stereo, with the next `out.len() / 2` frames.
    ///
    /// Requests from the controller take effect at the start of the call.
    /// The audio does not depend on how the output is divided into calls,
    /// only on which frame each request arrives at.
    pub fn process(&mut self, out: &mut [f32]) {
        self.voices.sweep(&self.plan, &mut self.garbage);
        self.handle_messages();
        let (frames, stray) = out.as_chunks_mut::<2>();
        stray.fill(0.0);
        for block in frames.chunks_mut(MAX_BLOCK) {
            self.process_block(block);
        }
        self.shared
            .publish_transport(self.transport_sequence, self.sequencer.playing());
        self.shared.publish_position(
            self.sequencer.tick(&self.plan, self.frame),
            self.voices.active(),
        );
    }

    fn handle_messages(&mut self) {
        // Handling a message can retire values, and those must have
        // somewhere to go.
        while self.garbage.slots() >= GARBAGE_HEADROOM {
            let Ok(message) = self.messages.pop() else {
                break;
            };
            self.handle(message);
        }
    }

    fn handle(&mut self, message: Message) {
        let now = self.frame;
        match message {
            Message::SetPlan { plan, state } => self.adopt(plan, state),
            Message::Play { sequence, passes } => {
                self.transport_sequence = sequence;
                self.sequencer.play(&self.plan, now, passes);
            }
            Message::Stop { sequence } => {
                self.transport_sequence = sequence;
                self.sequencer.stop();
                self.voices.fade_origin(Origin::Sequenced);
                self.voices.fade_origin(Origin::Live);
            }
            Message::Seek(tick) => {
                if self.sequencer.seek(tick, &self.plan, now) {
                    self.voices.fade_origin(Origin::Sequenced);
                }
            }
            Message::SetTransport {
                mode,
                pattern,
                loop_song,
            } => {
                if self
                    .sequencer
                    .set_transport(mode, pattern, loop_song, &self.plan, now)
                {
                    self.voices.fade_origin(Origin::Sequenced);
                }
            }
            Message::NoteOn {
                channel,
                key,
                velocity,
            } => {
                if let Some(channel) = self.plan.channel_ids.get(channel.0) {
                    let note = Note {
                        channel,
                        key,
                        velocity,
                        pan: 0.0,
                        release_at: u64::MAX,
                        origin: Origin::Live,
                    };
                    self.voices.start(&self.plan, &mut self.garbage, note, now);
                }
            }
            Message::NoteOff { channel, key } => self.voices.release_live(channel, key),
            Message::Preview(sample) => self.preview(sample),
            Message::StopPreview => self.voices.fade_origin(Origin::Preview),
            Message::SetOutputGain(gain) => self.output_gain.retarget(gain, now, self.ramp_frames),
        }
    }

    /// Switches to a new plan between two blocks. Gains glide to their new
    /// values and sounding voices carry on.
    fn adopt(&mut self, plan: Arc<Plan>, mut state: Box<PlanState>) {
        let now = self.frame;
        state.inherit(&plan, &self.plan, &self.state, now, self.ramp_frames);
        self.voices.rebind(&plan, &self.state, now);
        self.sequencer.set_plan(&plan, now);
        let old_plan = std::mem::replace(&mut self.plan, plan);
        let old_state = std::mem::replace(&mut self.state, state);
        retire(&mut self.garbage, Garbage::Plan(old_plan));
        retire(&mut self.garbage, Garbage::State(old_state));
    }

    fn preview(&mut self, sample: AudioBuffer) {
        let unplayed = self
            .voices
            .preview(&self.plan, &mut self.garbage, sample, self.frame);
        if let Some(sample) = unplayed {
            retire(&mut self.garbage, Garbage::Sample(sample));
        }
    }

    /// Processes up to [`MAX_BLOCK`] frames.
    fn process_block(&mut self, out: &mut [Frame]) {
        let frames = out.len();
        let base = self.frame;
        self.mixer.clear(self.plan.tracks.len(), frames);

        self.triggers.clear();
        self.sequencer
            .collect(&self.plan, base, base + frames as u64, &mut self.triggers);

        // Voices are rendered up to each note's frame before the note
        // starts, so everything a note-on does, cutting other voices
        // included, happens on its exact frame.
        let mut rendered = 0;
        for index in 0..self.triggers.len() {
            let trigger = self.triggers.get(index);
            let offset = (trigger.frame - base) as usize;
            self.render_voices(base, rendered, offset);
            rendered = offset;
            let note = Note {
                channel: trigger.channel,
                key: trigger.key,
                velocity: trigger.velocity,
                pan: trigger.pan,
                release_at: trigger.release_at,
                origin: Origin::Sequenced,
            };
            self.voices
                .start(&self.plan, &mut self.garbage, note, trigger.frame);
        }
        self.render_voices(base, rendered, frames);

        self.mixer
            .mix(&self.plan, &self.state, &self.shared, base, out);
        self.finish_output(base, out);
        self.frame += frames as u64;
    }

    fn render_voices(&mut self, base: u64, from: usize, to: usize) {
        if from < to {
            self.voices.render(
                &self.plan,
                &self.state,
                &mut self.garbage,
                &mut self.mixer,
                base,
                from..to,
            );
        }
    }

    /// Applies the output gain and keeps anything that is not a number away
    /// from the device.
    fn finish_output(&self, base: u64, out: &mut [Frame]) {
        let steady = self.output_gain.settled(base);
        let mut gain = self.output_gain.at(base);
        for (offset, frame) in out.iter_mut().enumerate() {
            if !steady {
                gain = self.output_gain.at(base + offset as u64);
            }
            for sample in frame {
                *sample = if sample.is_finite() {
                    *sample * gain
                } else {
                    0.0
                };
            }
        }
    }
}
