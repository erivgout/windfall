//! The interface every built-in instrument implements, and one type that
//! can hold any of them.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::param::{ParamInfo, ParamSet};
use crate::synth::{SubtractiveSynth, SynthParams};

/// A stereo instrument that turns notes into audio.
///
/// The threading rules are those of [`Effect`](crate::Effect): build,
/// [`prepare`](Instrument::prepare) and drop it away from the audio thread;
/// everything else is safe on it and never allocates, locks or blocks.
///
/// Notes carry no position inside a block. A host that wants a note to
/// start on an exact sample processes up to that sample, calls
/// [`note_on`](Instrument::note_on), and processes the rest. The output
/// does not otherwise depend on how the audio is divided into blocks.
pub trait Instrument: Send {
    type Params: ParamSet;

    /// Sets the sample rate and readies the instrument to play.
    /// `max_block` is the longest block the host will pass. It also
    /// resets.
    fn prepare(&mut self, sample_rate: f32, max_block: usize);

    /// Silences every voice at once and returns to the state the
    /// instrument had after `prepare`, with its current parameters.
    fn reset(&mut self);

    /// Takes new parameter values. They are forced into range first, and
    /// sounding notes glide to them. Values set before the first block
    /// after `prepare` or `reset` apply at once.
    fn set_params(&mut self, params: &Self::Params);

    /// Tells the instrument the project tempo in beats per minute.
    fn set_tempo(&mut self, bpm: f32) {
        let _ = bpm;
    }

    /// Starts a note. `key` is a MIDI key number, where 60 is middle C and
    /// 69 is the A at 440 Hz. `velocity` runs from 0 to 1; zero or less is
    /// taken as a note-off.
    fn note_on(&mut self, key: u8, velocity: f32);

    /// Lets go of a note, which starts its release.
    fn note_off(&mut self, key: u8);

    /// Stops every note with a fade of a few milliseconds, skipping their
    /// releases. For stopping the transport without a click.
    fn all_notes_off(&mut self);

    /// Writes the next block of output, replacing whatever the two slices
    /// held. The slices have the same length, which may be anything from 1
    /// up.
    fn process(&mut self, left: &mut [f32], right: &mut [f32]);

    /// Voices sounding right now, including ones fading out.
    fn active_voices(&self) -> usize;

    /// Samples by which the output lags the notes, for delay compensation.
    fn latency_samples(&self) -> usize {
        0
    }

    /// Samples for which output can continue after
    /// [`active_voices`](Instrument::active_voices) has dropped to zero.
    /// After that the instrument puts out exact silence until the next
    /// note.
    fn tail_samples(&self) -> usize {
        0
    }
}

/// Which instrument an [`AnyInstrument`] or an [`InstrumentParams`] holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum InstrumentKind {
    SubtractiveSynth,
}

impl InstrumentKind {
    pub const ALL: [InstrumentKind; 1] = [InstrumentKind::SubtractiveSynth];

    /// The instrument's name as shown to the user.
    pub fn name(self) -> &'static str {
        match self {
            InstrumentKind::SubtractiveSynth => SynthParams::NAME,
        }
    }

    /// The instrument's controls.
    pub fn descriptors(self) -> &'static [ParamInfo] {
        match self {
            InstrumentKind::SubtractiveSynth => SynthParams::descriptors(),
        }
    }

    /// The instrument's default settings.
    pub fn default_params(self) -> InstrumentParams {
        match self {
            InstrumentKind::SubtractiveSynth => {
                InstrumentParams::SubtractiveSynth(SynthParams::default())
            }
        }
    }
}

/// The settings of any one instrument. This is what a project stores for
/// an instrument channel.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum InstrumentParams {
    SubtractiveSynth(SynthParams),
}

impl InstrumentParams {
    pub fn kind(&self) -> InstrumentKind {
        match self {
            InstrumentParams::SubtractiveSynth(_) => InstrumentKind::SubtractiveSynth,
        }
    }

    /// A copy with every value forced into its range.
    pub fn sanitized(&self) -> Self {
        match self {
            InstrumentParams::SubtractiveSynth(params) => {
                InstrumentParams::SubtractiveSynth(params.sanitized())
            }
        }
    }

    /// The latency a prepared instrument has with these settings, which is
    /// what its [`latency_samples`](Instrument::latency_samples) reports. A
    /// host can plan delay compensation from this without holding the
    /// instrument.
    pub fn latency_samples(&self, sample_rate: f32) -> usize {
        let _ = sample_rate;
        match self {
            InstrumentParams::SubtractiveSynth(_) => SubtractiveSynth::LATENCY_SAMPLES,
        }
    }

    /// The value of the control at `index` of the instrument's
    /// [descriptors](InstrumentKind::descriptors).
    pub fn get(&self, index: usize) -> Option<f32> {
        match self {
            InstrumentParams::SubtractiveSynth(params) => params.get(index),
        }
    }

    /// Sets the control at `index`, forcing the value into range. Returns
    /// false if there is no such control. Does not allocate.
    pub fn set(&mut self, index: usize, value: f32) -> bool {
        match self {
            InstrumentParams::SubtractiveSynth(params) => params.set(index, value),
        }
    }
}

/// Any one of the built-in instruments behind one type, so a host can
/// choose what a channel plays at run time.
///
/// The instrument lives on the heap. Create it, prepare it and set its
/// parameters off the audio thread, and send it back off the audio thread
/// to be dropped.
pub enum AnyInstrument {
    SubtractiveSynth(Box<SubtractiveSynth>),
}

impl AnyInstrument {
    /// Builds the instrument `params` belongs to, with those settings. It
    /// still needs [`AnyInstrument::prepare`].
    pub fn new(params: &InstrumentParams) -> Self {
        match params {
            InstrumentParams::SubtractiveSynth(params) => {
                let mut synth = Box::<SubtractiveSynth>::default();
                synth.set_params(params);
                AnyInstrument::SubtractiveSynth(synth)
            }
        }
    }

    pub fn kind(&self) -> InstrumentKind {
        match self {
            AnyInstrument::SubtractiveSynth(_) => InstrumentKind::SubtractiveSynth,
        }
    }

    /// See [`Instrument::prepare`].
    pub fn prepare(&mut self, sample_rate: f32, max_block: usize) {
        match self {
            AnyInstrument::SubtractiveSynth(synth) => synth.prepare(sample_rate, max_block),
        }
    }

    /// See [`Instrument::reset`].
    pub fn reset(&mut self) {
        match self {
            AnyInstrument::SubtractiveSynth(synth) => synth.reset(),
        }
    }

    /// See [`Instrument::set_params`]. Returns false, and changes nothing,
    /// if `params` are for a different instrument.
    pub fn set_params(&mut self, params: &InstrumentParams) -> bool {
        match (self, params) {
            (
                AnyInstrument::SubtractiveSynth(synth),
                InstrumentParams::SubtractiveSynth(params),
            ) => {
                synth.set_params(params);
            }
        }
        true
    }

    /// See [`Instrument::set_tempo`].
    pub fn set_tempo(&mut self, bpm: f32) {
        match self {
            AnyInstrument::SubtractiveSynth(synth) => synth.set_tempo(bpm),
        }
    }

    /// See [`Instrument::note_on`].
    pub fn note_on(&mut self, key: u8, velocity: f32) {
        match self {
            AnyInstrument::SubtractiveSynth(synth) => synth.note_on(key, velocity),
        }
    }

    /// See [`Instrument::note_off`].
    pub fn note_off(&mut self, key: u8) {
        match self {
            AnyInstrument::SubtractiveSynth(synth) => synth.note_off(key),
        }
    }

    /// See [`Instrument::all_notes_off`].
    pub fn all_notes_off(&mut self) {
        match self {
            AnyInstrument::SubtractiveSynth(synth) => synth.all_notes_off(),
        }
    }

    /// See [`Instrument::process`].
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        match self {
            AnyInstrument::SubtractiveSynth(synth) => synth.process(left, right),
        }
    }

    /// See [`Instrument::active_voices`].
    pub fn active_voices(&self) -> usize {
        match self {
            AnyInstrument::SubtractiveSynth(synth) => synth.active_voices(),
        }
    }

    /// See [`Instrument::latency_samples`].
    pub fn latency_samples(&self) -> usize {
        match self {
            AnyInstrument::SubtractiveSynth(synth) => synth.latency_samples(),
        }
    }

    /// See [`Instrument::tail_samples`].
    pub fn tail_samples(&self) -> usize {
        match self {
            AnyInstrument::SubtractiveSynth(synth) => synth.tail_samples(),
        }
    }
}
