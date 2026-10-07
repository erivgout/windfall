//! Plugins in the shape the engine hosts: the same operations
//! `windfall_dsp::AnyEffect` and `AnyInstrument` have.
//!
//! The engine prepares a built-in processor off the audio thread, sets its
//! parameters by position in a descriptor list, tells it the tempo, and
//! calls `process(left, right)`. A [`PluginEffect`] or [`PluginInstrument`]
//! does all of that for a third-party plugin, so an effect slot or a
//! channel can hold one with the machinery the engine has.
//!
//! One step differs. A built-in processor prepares itself. A plugin is
//! prepared by its [`PluginInstance`], on the main thread, because that is
//! where plugins allocate: [`PluginInstance::prepare_effect`] and
//! [`PluginInstance::prepare_instrument`] stand in for `prepare`. To run at
//! another sample rate, send the adapter back, [`release`] it, and prepare
//! again.
//!
//! [`release`]: PluginInstance::release_effect

use crate::containment::PluginHealth;
use crate::error::PluginError;
use crate::events::Transport;
use crate::instance::PluginInstance;
use crate::params::PluginParam;
use crate::processor::{PluginProcessor, ProcessStatus};

/// What the audio thread needs to know of one parameter to set it by its
/// position in the list.
#[derive(Debug, Clone, Copy)]
struct ParamSlot {
    id: u32,
    min: f64,
    max: f64,
    stepped: bool,
    read_only: bool,
}

impl ParamSlot {
    fn of(param: &PluginParam) -> Self {
        Self {
            id: param.id,
            min: param.min.min(param.max),
            max: param.max.max(param.min),
            stepped: param.stepped,
            read_only: param.read_only,
        }
    }

    fn clamp(&self, value: f32) -> Option<f64> {
        if self.read_only || !value.is_finite() {
            return None;
        }
        let value = f64::from(value).clamp(self.min, self.max);
        Some(if self.stepped { value.round() } else { value })
    }
}

fn slots(instance: &PluginInstance) -> Box<[ParamSlot]> {
    instance.params().iter().map(ParamSlot::of).collect()
}

/// The longest tail the adapters report for a plugin that says its tail
/// never ends: one minute at the plugin's sample rate. The engine decides
/// when a track is done by listening, and uses the tail only as a bound.
const ENDLESS_TAIL_SECONDS: f64 = 60.0;

fn tail_of(processor: &PluginProcessor) -> usize {
    let latency = processor.latency_samples() as usize;
    match processor.tail_samples() {
        Some(tail) => tail as usize + latency,
        None => (processor.sample_rate() * ENDLESS_TAIL_SECONDS) as usize + latency,
    }
}

/// A plugin as a stereo effect that processes in place.
///
/// Everything here is for the audio thread and never allocates, locks or
/// blocks in the host's own code.
pub struct PluginEffect {
    processor: PluginProcessor,
    params: Box<[ParamSlot]>,
}

impl PluginEffect {
    /// Clears the plugin's memory of past audio.
    pub fn reset(&mut self) {
        self.processor.reset();
    }

    /// Sets the parameter at `index` of the list
    /// [`PluginInstance::params`] returned when the plugin was prepared, in
    /// the plugin's own units. The value is forced into range, and the
    /// change takes effect at the start of the next block. Returns false
    /// past the end of the list and for a parameter that cannot be set.
    pub fn set_param(&mut self, index: usize, value: f32) -> bool {
        let Some(slot) = self.params.get(index) else {
            return false;
        };
        match slot.clamp(value) {
            Some(value) => self.processor.set_param(0, slot.id, value),
            None => false,
        }
    }

    /// Tells the plugin the tempo, with the transport information of the
    /// next block.
    pub fn set_tempo(&mut self, bpm: f32) {
        self.processor.set_tempo(f64::from(bpm));
    }

    /// Tells the plugin where the song is. The engine's built-in effects
    /// have no use for this. Plugins that sync to the beat do.
    pub fn set_transport(&mut self, transport: Transport) {
        self.processor.set_transport(transport);
    }

    /// Processes one block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.processor.process(left, right);
    }

    /// Samples by which the output lags the input.
    pub fn latency_samples(&self) -> usize {
        self.processor.latency_samples() as usize
    }

    /// Samples for which the plugin can keep sounding after its input has
    /// gone silent, including the latency.
    pub fn tail_samples(&self) -> usize {
        tail_of(&self.processor)
    }

    /// Samples for which the output can fall silent and still come back
    /// with nothing new going in. A plugin does not say, so this is its
    /// whole tail.
    pub fn gap_samples(&self) -> usize {
        self.tail_samples()
    }

    /// How the plugin has behaved since it was prepared.
    pub fn health(&self) -> PluginHealth {
        self.processor.health()
    }

    /// The processor underneath, for what the engine's own effects do not
    /// have: events at a frame of the block, the plugin's status.
    pub fn processor(&mut self) -> &mut PluginProcessor {
        &mut self.processor
    }
}

/// How many keys there are.
const KEYS: usize = 128;

/// A plugin as a stereo instrument.
///
/// Like the built-in instruments it takes notes with no position in time:
/// a note starts on the first frame of the next block. The engine already
/// splits its blocks at every note, which puts each on its exact frame.
pub struct PluginInstrument {
    processor: PluginProcessor,
    params: Box<[ParamSlot]>,
    /// The keys that are down.
    held: [bool; KEYS],
    held_count: usize,
}

impl PluginInstrument {
    /// Silences every voice at once and clears the plugin's memory.
    pub fn reset(&mut self) {
        self.processor.reset();
        self.held = [false; KEYS];
        self.held_count = 0;
    }

    /// See [`PluginEffect::set_param`].
    pub fn set_param(&mut self, index: usize, value: f32) -> bool {
        let Some(slot) = self.params.get(index) else {
            return false;
        };
        match slot.clamp(value) {
            Some(value) => self.processor.set_param(0, slot.id, value),
            None => false,
        }
    }

    /// See [`PluginEffect::set_tempo`].
    pub fn set_tempo(&mut self, bpm: f32) {
        self.processor.set_tempo(f64::from(bpm));
    }

    /// See [`PluginEffect::set_transport`].
    pub fn set_transport(&mut self, transport: Transport) {
        self.processor.set_transport(transport);
    }

    /// Starts a note on the first frame of the next block. `key` is a MIDI
    /// key number and `velocity` runs from 0 to 1. Zero or less is taken as
    /// a note-off.
    pub fn note_on(&mut self, key: u8, velocity: f32) {
        if velocity <= 0.0 {
            return self.note_off(key);
        }
        let key = key.min(127);
        if self.processor.note_on(0, key, velocity) && !self.held[usize::from(key)] {
            self.held[usize::from(key)] = true;
            self.held_count += 1;
        }
    }

    /// Lets go of a note on the first frame of the next block.
    pub fn note_off(&mut self, key: u8) {
        let key = key.min(127);
        self.processor.note_off(0, key);
        if self.held[usize::from(key)] {
            self.held[usize::from(key)] = false;
            self.held_count -= 1;
        }
    }

    /// Stops every note without its release, as when the transport stops.
    pub fn all_notes_off(&mut self) {
        self.processor.all_notes_off(0);
        self.held = [false; KEYS];
        self.held_count = 0;
    }

    /// Writes the next block of output, replacing whatever the two slices
    /// held.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        // A plugin with an audio input, such as a vocoder, would otherwise
        // be fed whatever the buffers held.
        if self.processor.has_audio_input() {
            left.fill(0.0);
            right.fill(0.0);
        }
        self.processor.process(left, right);
    }

    /// Notes that are held down. A plugin does not say how many voices are
    /// sounding, so a note in its release is not counted. The engine finds
    /// out when an instrument has gone quiet by listening to it, bounded by
    /// [`tail_samples`](Self::tail_samples).
    pub fn active_voices(&self) -> usize {
        self.held_count
    }

    /// True once the plugin has said that nothing more will come out until
    /// the next note.
    pub fn is_asleep(&self) -> bool {
        self.held_count == 0 && self.processor.status() == ProcessStatus::Sleep
    }

    /// Samples by which the output lags the notes.
    pub fn latency_samples(&self) -> usize {
        self.processor.latency_samples() as usize
    }

    /// Samples for which output can continue after the last note was let
    /// go, including the latency.
    pub fn tail_samples(&self) -> usize {
        tail_of(&self.processor)
    }

    /// See [`PluginEffect::health`].
    pub fn health(&self) -> PluginHealth {
        self.processor.health()
    }

    /// See [`PluginEffect::processor`].
    pub fn processor(&mut self) -> &mut PluginProcessor {
        &mut self.processor
    }
}

impl PluginInstance {
    /// Activates the plugin at `sample_rate` for blocks of up to
    /// `max_block` frames and returns it as an effect. This is the plugin's
    /// `prepare`: it allocates, so it belongs on the main thread.
    pub fn prepare_effect(
        &mut self,
        sample_rate: f32,
        max_block: usize,
    ) -> Result<PluginEffect, PluginError> {
        let params = slots(self);
        let processor = self.activate(f64::from(sample_rate), max_block)?;
        Ok(PluginEffect { processor, params })
    }

    /// Activates the plugin and returns it as an instrument. See
    /// [`prepare_effect`](Self::prepare_effect).
    pub fn prepare_instrument(
        &mut self,
        sample_rate: f32,
        max_block: usize,
    ) -> Result<PluginInstrument, PluginError> {
        let params = slots(self);
        let processor = self.activate(f64::from(sample_rate), max_block)?;
        Ok(PluginInstrument {
            processor,
            params,
            held: [false; KEYS],
            held_count: 0,
        })
    }

    /// Takes an effect back from the audio thread and deactivates the
    /// plugin.
    pub fn release_effect(&mut self, effect: PluginEffect) {
        self.deactivate(effect.processor);
    }

    /// Takes an instrument back from the audio thread and deactivates the
    /// plugin.
    pub fn release_instrument(&mut self, instrument: PluginInstrument) {
        self.deactivate(instrument.processor);
    }
}
