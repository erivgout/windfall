//! The interface every built-in effect implements, and what a host needs to
//! run a chain of them: one type that can hold any effect, and a slot that
//! adds bypass and a mix control.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::balance::{Balance, BalanceParams};
use crate::channel_mute::{ChannelMute, ChannelMuteParams};
use crate::dc_block::{DcBlock, DcBlockParams};
use crate::distortion::{DISTORTION_LATENCY_SAMPLES, Distortion, DistortionParams};
use crate::polarity::{Polarity, PolarityParams};
use crate::soft_clipper::{SoftClipper, SoftClipperParams};
use crate::stereo_matrix::{StereoMatrix, StereoMatrixParams};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::blocks::delay_line::DelayLine;
use crate::blocks::math::ms_to_samples;
use crate::blocks::smooth::LinearRamp;
use crate::blocks::tap_crossfade::TapCrossfade;
use crate::compressor::{Compressor, CompressorParams};
use crate::delay::{Delay, DelayParams};
use crate::eq::{EqParams, ParametricEq};
use crate::filter_family::{
    BassShelf, BassShelfParams, FastLowpass, FastLowpassParams, SelectableFilter,
    SelectableFilterParams,
};
use crate::limiter::{LOOKAHEAD_FADE_MS, Limiter, LimiterParams};
use crate::param::{ParamInfo, ParamSet};
use crate::reverb::{Reverb, ReverbParams};

/// A stereo effect that processes audio in place.
///
/// # Threads
///
/// Build an effect, [`prepare`](Effect::prepare) it and set its first
/// parameters away from the audio thread, then hand it over. On the audio
/// thread, [`set_params`](Effect::set_params),
/// [`set_tempo`](Effect::set_tempo), [`reset`](Effect::reset) and
/// [`process`](Effect::process) never allocate, free, lock or block.
/// Dropping an effect frees memory, so send it back to be dropped
/// elsewhere.
///
/// # Parameters
///
/// `set_params` may be called between any two blocks. The effect glides to
/// the new values over a few milliseconds, so a change never clicks. The
/// one exception is the first block after `prepare` or `reset`: values set
/// before it apply at once, so a freshly loaded effect starts where its
/// parameters say and not somewhere on the way from the defaults.
///
/// # Blocks
///
/// The output does not depend on how the audio is divided into blocks:
/// processing a signal in one piece or in pieces of any length gives the
/// same samples.
pub trait Effect: Send {
    type Params: ParamSet;

    /// Sets the sample rate and allocates everything `process` will need.
    /// `max_block` is the longest block the host will pass. This is the
    /// only method besides construction that allocates. It also resets.
    fn prepare(&mut self, sample_rate: f32, max_block: usize);

    /// Clears all memory of past audio, as if the effect had just been
    /// prepared and given its current parameters.
    fn reset(&mut self);

    /// Takes new parameter values. They are forced into range first.
    fn set_params(&mut self, params: &Self::Params);

    /// Tells the effect the project tempo in beats per minute. Effects with
    /// nothing synced to tempo ignore it.
    fn set_tempo(&mut self, bpm: f32) {
        let _ = bpm;
    }

    /// Processes one block in place. The two slices have the same length,
    /// which may be anything from 1 up.
    fn process(&mut self, left: &mut [f32], right: &mut [f32]);

    /// Samples by which the output lags the input, for delay compensation.
    /// It can change when parameters change, so read it after
    /// `set_params`.
    fn latency_samples(&self) -> usize {
        0
    }

    /// Samples to run unheard after waking from cleared state, so every
    /// delayed output is ready before the slot fades it in. This can exceed
    /// the shared latency used for compensation, for example in a matrix.
    fn warm_up_samples(&self) -> usize {
        self.latency_samples()
    }

    /// History required before starting a live latency-tap edit. This is
    /// separate from reported PDC and includes both outputs of a matrix.
    fn delay_readiness_samples(&self) -> usize {
        self.latency_samples()
    }
    /// Remaining input/fade frames before a requested latency edit settles.
    fn latency_transition_samples_remaining(&self) -> usize {
        0
    }

    /// Samples for which the output can keep sounding after the input has
    /// gone silent, including the latency. A host can stop processing a
    /// track this long after its last input.
    fn tail_samples(&self) -> usize {
        0
    }

    /// Samples for which the output can fall silent and still come back
    /// with nothing new going in: the longest wait for something that is
    /// already inside the effect, such as a delay's next echo, a reverb's
    /// pre-delay or a compressor letting go. It includes the latency. A
    /// host that has seen silence go in and come out for this long can
    /// take the effect to be done, without waiting for all of
    /// [`tail_samples`](Effect::tail_samples).
    fn gap_samples(&self) -> usize {
        self.latency_samples()
    }
}

/// A gain reduction reading that the audio thread writes and any other
/// thread reads, without locking.
///
/// It holds the deepest reduction since it was last read, so a meter that
/// polls 60 times a second still sees a transient that lasted a
/// millisecond.
#[derive(Debug, Clone, Default)]
pub struct GainReductionMeter(Arc<AtomicU32>);

impl GainReductionMeter {
    /// Records a reduction in dB, zero or more, if it is deeper than what
    /// is already held.
    pub(crate) fn raise(&self, reduction_db: f32) {
        // Positive floats order the same way as their bit patterns. Nothing
        // else may get in: the bits of -0.0, which is what a limiter with
        // nothing to do reports, are larger than those of any reduction and
        // would hide every reading until the meter is next read.
        if reduction_db > 0.0 {
            self.0.fetch_max(reduction_db.to_bits(), Ordering::Relaxed);
        }
    }

    /// The deepest gain reduction since the last call, in dB: 0 means the
    /// signal was not turned down, 6 means it was turned down by 6 dB.
    /// Reading resets it.
    pub fn take_db(&self) -> f32 {
        f32::from_bits(self.0.swap(0, Ordering::Relaxed))
    }
}

/// Which effect an [`AnyEffect`] or an [`EffectParams`] holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum EffectKind {
    Eq,
    Compressor,
    Limiter,
    Reverb,
    Delay,
    Balance,
    DcBlock,
    ChannelMute,
    Polarity,
    StereoMatrix,
    SoftClipper,
    Distortion,
    FastLowpass,
    SelectableFilter,
    BassShelf,
}

impl EffectKind {
    pub const ALL: [EffectKind; 15] = [
        EffectKind::Eq,
        EffectKind::Compressor,
        EffectKind::Limiter,
        EffectKind::Reverb,
        EffectKind::Delay,
        EffectKind::Balance,
        EffectKind::DcBlock,
        EffectKind::ChannelMute,
        EffectKind::Polarity,
        EffectKind::StereoMatrix,
        EffectKind::SoftClipper,
        EffectKind::Distortion,
        EffectKind::FastLowpass,
        EffectKind::SelectableFilter,
        EffectKind::BassShelf,
    ];

    /// The effect's name as shown to the user.
    pub fn name(self) -> &'static str {
        match self {
            EffectKind::Eq => EqParams::NAME,
            EffectKind::Compressor => CompressorParams::NAME,
            EffectKind::Limiter => LimiterParams::NAME,
            EffectKind::Reverb => ReverbParams::NAME,
            EffectKind::Delay => DelayParams::NAME,
            EffectKind::Balance => BalanceParams::NAME,
            EffectKind::DcBlock => DcBlockParams::NAME,
            EffectKind::ChannelMute => ChannelMuteParams::NAME,
            EffectKind::Polarity => PolarityParams::NAME,
            EffectKind::StereoMatrix => StereoMatrixParams::NAME,
            EffectKind::SoftClipper => SoftClipperParams::NAME,
            EffectKind::Distortion => DistortionParams::NAME,
            EffectKind::FastLowpass => FastLowpassParams::NAME,
            EffectKind::SelectableFilter => SelectableFilterParams::NAME,
            EffectKind::BassShelf => BassShelfParams::NAME,
        }
    }

    /// The effect's controls.
    pub fn descriptors(self) -> &'static [ParamInfo] {
        match self {
            EffectKind::Eq => EqParams::descriptors(),
            EffectKind::Compressor => CompressorParams::descriptors(),
            EffectKind::Limiter => LimiterParams::descriptors(),
            EffectKind::Reverb => ReverbParams::descriptors(),
            EffectKind::Delay => DelayParams::descriptors(),
            EffectKind::Balance => BalanceParams::descriptors(),
            EffectKind::DcBlock => DcBlockParams::descriptors(),
            EffectKind::ChannelMute => ChannelMuteParams::descriptors(),
            EffectKind::Polarity => PolarityParams::descriptors(),
            EffectKind::StereoMatrix => StereoMatrixParams::descriptors(),
            EffectKind::SoftClipper => SoftClipperParams::descriptors(),
            EffectKind::Distortion => DistortionParams::descriptors(),
            EffectKind::FastLowpass => FastLowpassParams::descriptors(),
            EffectKind::SelectableFilter => SelectableFilterParams::descriptors(),
            EffectKind::BassShelf => BassShelfParams::descriptors(),
        }
    }

    /// The largest latency any setting of this effect can have at
    /// `sample_rate`.
    pub fn max_latency_samples(self, sample_rate: f32) -> usize {
        match self {
            EffectKind::Limiter => Limiter::max_latency_samples(sample_rate.max(1.0)),
            EffectKind::StereoMatrix => StereoMatrix::max_latency_samples(sample_rate),
            EffectKind::Distortion => DISTORTION_LATENCY_SAMPLES,
            EffectKind::Eq
            | EffectKind::Compressor
            | EffectKind::Reverb
            | EffectKind::Delay
            | EffectKind::Balance
            | EffectKind::DcBlock
            | EffectKind::ChannelMute
            | EffectKind::Polarity
            | EffectKind::SoftClipper
            | EffectKind::FastLowpass
            | EffectKind::SelectableFilter
            | EffectKind::BassShelf => 0,
        }
    }

    /// The effect's default settings.
    pub fn default_params(self) -> EffectParams {
        match self {
            EffectKind::Eq => EffectParams::Eq(EqParams::default()),
            EffectKind::Compressor => EffectParams::Compressor(CompressorParams::default()),
            EffectKind::Limiter => EffectParams::Limiter(LimiterParams::default()),
            EffectKind::Reverb => EffectParams::Reverb(ReverbParams::default()),
            EffectKind::Delay => EffectParams::Delay(DelayParams::default()),
            EffectKind::Balance => EffectParams::Balance(BalanceParams::default()),
            EffectKind::DcBlock => EffectParams::DcBlock(DcBlockParams::default()),
            EffectKind::ChannelMute => EffectParams::ChannelMute(ChannelMuteParams::default()),
            EffectKind::Polarity => EffectParams::Polarity(PolarityParams::default()),
            EffectKind::StereoMatrix => EffectParams::StereoMatrix(StereoMatrixParams::default()),
            EffectKind::SoftClipper => EffectParams::SoftClipper(SoftClipperParams::default()),
            EffectKind::Distortion => EffectParams::Distortion(DistortionParams::default()),
            EffectKind::FastLowpass => EffectParams::FastLowpass(FastLowpassParams::default()),
            EffectKind::SelectableFilter => {
                EffectParams::SelectableFilter(SelectableFilterParams::default())
            }
            EffectKind::BassShelf => EffectParams::BassShelf(BassShelfParams::default()),
        }
    }
}

/// The settings of any one effect. This is what a project stores for an
/// effect slot.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum EffectParams {
    Eq(EqParams),
    Compressor(CompressorParams),
    Limiter(LimiterParams),
    Reverb(ReverbParams),
    Delay(DelayParams),
    Balance(BalanceParams),
    DcBlock(DcBlockParams),
    ChannelMute(ChannelMuteParams),
    Polarity(PolarityParams),
    StereoMatrix(StereoMatrixParams),
    SoftClipper(SoftClipperParams),
    Distortion(DistortionParams),
    FastLowpass(FastLowpassParams),
    SelectableFilter(SelectableFilterParams),
    BassShelf(BassShelfParams),
}

/// Runs `$body` with `$params` bound to the settings inside an
/// [`EffectParams`], whichever effect they belong to.
macro_rules! each_params {
    ($value:expr, $params:ident => $body:expr) => {
        match $value {
            EffectParams::Eq($params) => $body,
            EffectParams::Compressor($params) => $body,
            EffectParams::Limiter($params) => $body,
            EffectParams::Reverb($params) => $body,
            EffectParams::Delay($params) => $body,
            EffectParams::Balance($params) => $body,
            EffectParams::DcBlock($params) => $body,
            EffectParams::ChannelMute($params) => $body,
            EffectParams::Polarity($params) => $body,
            EffectParams::StereoMatrix($params) => $body,
            EffectParams::SoftClipper($params) => $body,
            EffectParams::Distortion($params) => $body,
            EffectParams::FastLowpass($params) => $body,
            EffectParams::SelectableFilter($params) => $body,
            EffectParams::BassShelf($params) => $body,
        }
    };
}

impl EffectParams {
    pub fn kind(&self) -> EffectKind {
        match self {
            EffectParams::Eq(_) => EffectKind::Eq,
            EffectParams::Compressor(_) => EffectKind::Compressor,
            EffectParams::Limiter(_) => EffectKind::Limiter,
            EffectParams::Reverb(_) => EffectKind::Reverb,
            EffectParams::Delay(_) => EffectKind::Delay,
            EffectParams::Balance(_) => EffectKind::Balance,
            EffectParams::DcBlock(_) => EffectKind::DcBlock,
            EffectParams::ChannelMute(_) => EffectKind::ChannelMute,
            EffectParams::Polarity(_) => EffectKind::Polarity,
            EffectParams::StereoMatrix(_) => EffectKind::StereoMatrix,
            EffectParams::SoftClipper(_) => EffectKind::SoftClipper,
            EffectParams::Distortion(_) => EffectKind::Distortion,
            EffectParams::FastLowpass(_) => EffectKind::FastLowpass,
            EffectParams::SelectableFilter(_) => EffectKind::SelectableFilter,
            EffectParams::BassShelf(_) => EffectKind::BassShelf,
        }
    }

    /// A copy with every value forced into its range.
    pub fn sanitized(&self) -> Self {
        match self {
            EffectParams::Eq(params) => EffectParams::Eq(params.sanitized()),
            EffectParams::Compressor(params) => EffectParams::Compressor(params.sanitized()),
            EffectParams::Limiter(params) => EffectParams::Limiter(params.sanitized()),
            EffectParams::Reverb(params) => EffectParams::Reverb(params.sanitized()),
            EffectParams::Delay(params) => EffectParams::Delay(params.sanitized()),
            EffectParams::Balance(params) => EffectParams::Balance(params.sanitized()),
            EffectParams::DcBlock(params) => EffectParams::DcBlock(params.sanitized()),
            EffectParams::ChannelMute(params) => EffectParams::ChannelMute(params.sanitized()),
            EffectParams::Polarity(params) => EffectParams::Polarity(params.sanitized()),
            EffectParams::StereoMatrix(params) => EffectParams::StereoMatrix(params.sanitized()),
            EffectParams::SoftClipper(params) => EffectParams::SoftClipper(params.sanitized()),
            EffectParams::Distortion(params) => EffectParams::Distortion(params.sanitized()),
            EffectParams::FastLowpass(params) => EffectParams::FastLowpass(params.sanitized()),
            EffectParams::SelectableFilter(params) => {
                EffectParams::SelectableFilter(params.sanitized())
            }
            EffectParams::BassShelf(params) => EffectParams::BassShelf(params.sanitized()),
        }
    }

    /// The latency a prepared effect has with these settings at
    /// `sample_rate`, which is what its
    /// [`latency_samples`](Effect::latency_samples) reports once it has
    /// been given them. A host can plan delay compensation from this
    /// without holding the effect.
    pub fn latency_samples(&self, sample_rate: f32) -> usize {
        match self {
            EffectParams::Limiter(params) => params.latency_samples(sample_rate),
            EffectParams::StereoMatrix(params) => params.latency_samples(sample_rate),
            EffectParams::Distortion(_) => DISTORTION_LATENCY_SAMPLES,
            EffectParams::Eq(_)
            | EffectParams::Compressor(_)
            | EffectParams::Reverb(_)
            | EffectParams::Delay(_)
            | EffectParams::Balance(_)
            | EffectParams::DcBlock(_)
            | EffectParams::ChannelMute(_)
            | EffectParams::Polarity(_)
            | EffectParams::SoftClipper(_)
            | EffectParams::FastLowpass(_)
            | EffectParams::SelectableFilter(_)
            | EffectParams::BassShelf(_) => 0,
        }
    }

    /// The value of the control at `index` of the effect's
    /// [descriptors](EffectKind::descriptors).
    pub fn get(&self, index: usize) -> Option<f32> {
        each_params!(self, params => params.get(index))
    }

    /// Sets the control at `index`, forcing the value into range. Returns
    /// false if there is no such control. Does not allocate, so automation
    /// can call it on the audio thread.
    pub fn set(&mut self, index: usize, value: f32) -> bool {
        each_params!(self, params => params.set(index, value))
    }
}

/// Any one of the built-in effects, behind one type, so a host can keep a
/// chain whose contents are chosen at run time.
///
/// The effect itself lives on the heap, which keeps this small enough to
/// pass through a queue. Create it, prepare it and set its parameters off
/// the audio thread; and since dropping it frees that memory, send it back
/// off the audio thread to be dropped.
pub enum AnyEffect {
    Eq(Box<ParametricEq>),
    Compressor(Box<Compressor>),
    Limiter(Box<Limiter>),
    Reverb(Box<Reverb>),
    Delay(Box<Delay>),
    Balance(Box<Balance>),
    DcBlock(Box<DcBlock>),
    ChannelMute(Box<ChannelMute>),
    Polarity(Box<Polarity>),
    StereoMatrix(Box<StereoMatrix>),
    SoftClipper(Box<SoftClipper>),
    Distortion(Box<Distortion>),
    FastLowpass(Box<FastLowpass>),
    SelectableFilter(Box<SelectableFilter>),
    BassShelf(Box<BassShelf>),
}

/// Runs `$body` with `$effect` bound to the effect inside an [`AnyEffect`].
macro_rules! each_effect {
    ($value:expr, $effect:ident => $body:expr) => {
        match $value {
            AnyEffect::Eq($effect) => $body,
            AnyEffect::Compressor($effect) => $body,
            AnyEffect::Limiter($effect) => $body,
            AnyEffect::Reverb($effect) => $body,
            AnyEffect::Delay($effect) => $body,
            AnyEffect::Balance($effect) => $body,
            AnyEffect::DcBlock($effect) => $body,
            AnyEffect::ChannelMute($effect) => $body,
            AnyEffect::Polarity($effect) => $body,
            AnyEffect::StereoMatrix($effect) => $body,
            AnyEffect::SoftClipper($effect) => $body,
            AnyEffect::Distortion($effect) => $body,
            AnyEffect::FastLowpass($effect) => $body,
            AnyEffect::SelectableFilter($effect) => $body,
            AnyEffect::BassShelf($effect) => $body,
        }
    };
}

impl AnyEffect {
    /// Builds the effect `params` belongs to, with those settings. It
    /// still needs [`AnyEffect::prepare`].
    pub fn new(params: &EffectParams) -> Self {
        let mut effect = match params.kind() {
            EffectKind::Eq => AnyEffect::Eq(Box::default()),
            EffectKind::Compressor => AnyEffect::Compressor(Box::default()),
            EffectKind::Limiter => AnyEffect::Limiter(Box::default()),
            EffectKind::Reverb => AnyEffect::Reverb(Box::default()),
            EffectKind::Delay => AnyEffect::Delay(Box::default()),
            EffectKind::Balance => AnyEffect::Balance(Box::default()),
            EffectKind::DcBlock => AnyEffect::DcBlock(Box::default()),
            EffectKind::ChannelMute => AnyEffect::ChannelMute(Box::default()),
            EffectKind::Polarity => AnyEffect::Polarity(Box::default()),
            EffectKind::StereoMatrix => AnyEffect::StereoMatrix(Box::default()),
            EffectKind::SoftClipper => AnyEffect::SoftClipper(Box::default()),
            EffectKind::Distortion => AnyEffect::Distortion(Box::default()),
            EffectKind::FastLowpass => AnyEffect::FastLowpass(Box::default()),
            EffectKind::SelectableFilter => AnyEffect::SelectableFilter(Box::default()),
            EffectKind::BassShelf => AnyEffect::BassShelf(Box::default()),
        };
        effect.set_params(params);
        effect
    }

    pub fn kind(&self) -> EffectKind {
        match self {
            AnyEffect::Eq(_) => EffectKind::Eq,
            AnyEffect::Compressor(_) => EffectKind::Compressor,
            AnyEffect::Limiter(_) => EffectKind::Limiter,
            AnyEffect::Reverb(_) => EffectKind::Reverb,
            AnyEffect::Delay(_) => EffectKind::Delay,
            AnyEffect::Balance(_) => EffectKind::Balance,
            AnyEffect::DcBlock(_) => EffectKind::DcBlock,
            AnyEffect::ChannelMute(_) => EffectKind::ChannelMute,
            AnyEffect::Polarity(_) => EffectKind::Polarity,
            AnyEffect::StereoMatrix(_) => EffectKind::StereoMatrix,
            AnyEffect::SoftClipper(_) => EffectKind::SoftClipper,
            AnyEffect::Distortion(_) => EffectKind::Distortion,
            AnyEffect::FastLowpass(_) => EffectKind::FastLowpass,
            AnyEffect::SelectableFilter(_) => EffectKind::SelectableFilter,
            AnyEffect::BassShelf(_) => EffectKind::BassShelf,
        }
    }

    /// See [`Effect::prepare`].
    pub fn prepare(&mut self, sample_rate: f32, max_block: usize) {
        each_effect!(self, effect => effect.prepare(sample_rate, max_block));
    }

    /// See [`Effect::reset`].
    pub fn reset(&mut self) {
        each_effect!(self, effect => effect.reset());
    }

    /// See [`Effect::set_params`]. Returns false, and changes nothing, if
    /// `params` are for a different effect: the host then has to build a
    /// new one.
    pub fn set_params(&mut self, params: &EffectParams) -> bool {
        match (self, params) {
            (AnyEffect::Eq(effect), EffectParams::Eq(params)) => effect.set_params(params),
            (AnyEffect::Compressor(effect), EffectParams::Compressor(params)) => {
                effect.set_params(params);
            }
            (AnyEffect::Limiter(effect), EffectParams::Limiter(params)) => {
                effect.set_params(params);
            }
            (AnyEffect::Reverb(effect), EffectParams::Reverb(params)) => effect.set_params(params),
            (AnyEffect::Delay(effect), EffectParams::Delay(params)) => effect.set_params(params),
            (AnyEffect::Balance(effect), EffectParams::Balance(params)) => {
                effect.set_params(params)
            }
            (AnyEffect::DcBlock(effect), EffectParams::DcBlock(params)) => {
                effect.set_params(params)
            }
            (AnyEffect::ChannelMute(effect), EffectParams::ChannelMute(params)) => {
                effect.set_params(params)
            }
            (AnyEffect::Polarity(effect), EffectParams::Polarity(params)) => {
                effect.set_params(params)
            }
            (AnyEffect::StereoMatrix(effect), EffectParams::StereoMatrix(params)) => {
                effect.set_params(params)
            }
            (AnyEffect::SoftClipper(effect), EffectParams::SoftClipper(params)) => {
                effect.set_params(params)
            }
            (AnyEffect::Distortion(effect), EffectParams::Distortion(params)) => {
                effect.set_params(params)
            }
            (AnyEffect::FastLowpass(effect), EffectParams::FastLowpass(params)) => {
                effect.set_params(params)
            }
            (AnyEffect::SelectableFilter(effect), EffectParams::SelectableFilter(params)) => {
                effect.set_params(params)
            }
            (AnyEffect::BassShelf(effect), EffectParams::BassShelf(params)) => {
                effect.set_params(params)
            }
            _ => return false,
        }
        true
    }

    /// See [`Effect::set_tempo`].
    pub fn set_tempo(&mut self, bpm: f32) {
        each_effect!(self, effect => effect.set_tempo(bpm));
    }

    /// See [`Effect::process`].
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        each_effect!(self, effect => effect.process(left, right));
    }

    /// See [`Effect::latency_samples`].
    pub fn latency_samples(&self) -> usize {
        each_effect!(self, effect => effect.latency_samples())
    }

    /// See [`Effect::warm_up_samples`].
    pub fn warm_up_samples(&self) -> usize {
        each_effect!(self, effect => effect.warm_up_samples())
    }

    /// See [`Effect::delay_readiness_samples`].
    pub fn delay_readiness_samples(&self) -> usize {
        each_effect!(self, effect => effect.delay_readiness_samples())
    }
    pub fn latency_transition_samples_remaining(&self) -> usize {
        each_effect!(self, effect => effect.latency_transition_samples_remaining())
    }

    /// The largest latency any setting of this effect can have at
    /// `sample_rate`. A host that reserves this much never has to grow a
    /// compensation buffer while playing.
    pub fn max_latency_samples(&self, sample_rate: f32) -> usize {
        self.kind().max_latency_samples(sample_rate)
    }

    /// See [`Effect::tail_samples`].
    pub fn tail_samples(&self) -> usize {
        each_effect!(self, effect => effect.tail_samples())
    }

    /// See [`Effect::gap_samples`].
    pub fn gap_samples(&self) -> usize {
        each_effect!(self, effect => effect.gap_samples())
    }

    /// The gain reduction meter of a compressor or limiter. Take it before
    /// the effect goes to the audio thread.
    pub fn gain_reduction(&self) -> Option<GainReductionMeter> {
        match self {
            AnyEffect::Compressor(effect) => Some(effect.meter()),
            AnyEffect::Limiter(effect) => Some(effect.meter()),
            _ => None,
        }
    }
}

/// Time the slot takes to fade an effect in or out, and to follow its mix.
const SLOT_FADE_MS: f32 = 10.0;

/// The loudest sample a slot lets through to its effect: 60 dB over full
/// scale. Nothing a mixer produces on purpose comes near it.
const INPUT_LIMIT: f32 = 1.0e3;

/// One place in an effect chain: an effect plus the two controls every
/// slot has, an on/off switch and a mix.
///
/// Switching the slot off crossfades to the untouched input over a few
/// milliseconds instead of cutting, and then stops running the effect,
/// which costs nothing while it is off. Switched back on, the effect
/// starts with its memory cleared and runs unheard until every delayed
/// output is primed, so the fade back in finds its output already there.
/// The mix blends the untouched input
/// with the effect's output. Both use an input delayed by the effect's
/// latency, so the slot's delay is the same whether it is on, off or
/// half-mixed and compensation never has to change. When the latency
/// itself changes, as when a limiter's look-ahead is moved, that delay
/// crossfades to its new length the way the limiter does.
///
/// The slot also keeps bad input away from the effect: samples that are
/// not numbers become silence and absurdly loud ones are held to 60 dB
/// over full scale, so a misbehaving plugin upstream cannot leave an
/// effect's memory poisoned.
///
/// Like the effect inside it, a slot is built and prepared off the audio
/// thread and dropped off it.
pub struct EffectSlot {
    effect: AnyEffect,
    sample_rate: f32,
    enabled: bool,
    mix: f32,
    /// Share of the effect's output in what the slot puts out.
    wet: LinearRamp,
    dry_left: DelayLine,
    dry_right: DelayLine,
    /// Shared-delay transition, preserving every currently audible dry tap.
    dry_fade: TapCrossfade,
    dry_history: usize,
    /// Whether each prepared scratch frame uses the final dry tap.
    dry_aligned: Box<[bool]>,
    scratch_left: Box<[f32]>,
    scratch_right: Box<[f32]>,
    /// The effect has faded out and has not run since, so its memory is
    /// stale and must be cleared before it is heard again.
    dormant: bool,
    /// Samples the effect still has to run unheard after it was woken. An
    /// effect with delayed outputs puts out nothing for that long once its memory
    /// has been cleared, and fading it in at once would let its output
    /// burst in part way through the fade.
    warm_up: usize,
    /// Samples already run since the dormant effect was cleared. A delay
    /// increased during priming extends the wait without losing this history.
    warm_up_elapsed: usize,
    warming: bool,
    fresh: bool,
}

impl EffectSlot {
    /// A slot that is switched on, at full mix. It still needs
    /// [`EffectSlot::prepare`].
    pub fn new(effect: AnyEffect) -> Self {
        Self {
            effect,
            sample_rate: 48_000.0,
            enabled: true,
            mix: 1.0,
            wet: LinearRamp::new(1.0),
            dry_left: DelayLine::default(),
            dry_right: DelayLine::default(),
            dry_fade: TapCrossfade::new(0, 0),
            dry_history: 0,
            dry_aligned: Box::default(),
            scratch_left: Box::default(),
            scratch_right: Box::default(),
            dormant: false,
            warm_up: 0,
            warm_up_elapsed: 0,
            warming: false,
            fresh: true,
        }
    }

    /// Prepares the effect and the slot's own buffers. Allocates.
    pub fn prepare(&mut self, sample_rate: f32, max_block: usize) {
        let max_block = max_block.max(1);
        self.sample_rate = sample_rate.max(1.0);
        self.effect.prepare(self.sample_rate, max_block);
        let latency = self.effect.max_latency_samples(self.sample_rate);
        self.dry_left = DelayLine::new(latency);
        self.dry_right = DelayLine::new(latency);
        self.dry_fade = TapCrossfade::new(latency, self.effect.latency_samples());
        self.scratch_left = vec![0.0; max_block].into_boxed_slice();
        self.scratch_right = vec![0.0; max_block].into_boxed_slice();
        self.dry_aligned = vec![false; max_block].into_boxed_slice();
        self.reset();
    }

    /// Clears the effect's and the slot's memory of past audio.
    pub fn reset(&mut self) {
        self.effect.reset();
        self.dry_left.clear();
        self.dry_right.clear();
        self.dry_fade.snap(self.effect.latency_samples());
        self.dry_history = 0;
        self.dormant = false;
        self.warm_up = 0;
        self.warm_up_elapsed = 0;
        self.warming = false;
        self.fresh = true;
        self.wet.snap(self.wet_target());
    }

    fn wet_target(&self) -> f32 {
        if self.enabled { self.mix } else { 0.0 }
    }

    fn retarget(&mut self) {
        let samples = if self.fresh {
            0
        } else {
            ms_to_samples(SLOT_FADE_MS, self.sample_rate)
        };
        self.wet.set_target(self.wet_target(), samples);
    }

    /// Switches the effect on or off without a click.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.retarget();
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Sets the balance between the untouched input (0) and the effect's
    /// output (1).
    pub fn set_mix(&mut self, mix: f32) {
        self.mix = if mix.is_finite() {
            mix.clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.retarget();
    }

    /// See [`AnyEffect::set_params`].
    pub fn set_params(&mut self, params: &EffectParams) -> bool {
        let accepted = self.effect.set_params(params);
        if accepted && self.warm_up > 0 {
            self.warm_up = self.warm_up.max(
                self.effect
                    .warm_up_samples()
                    .saturating_sub(self.warm_up_elapsed),
            );
        }
        accepted
    }

    /// See [`Effect::set_tempo`].
    pub fn set_tempo(&mut self, bpm: f32) {
        self.effect.set_tempo(bpm);
    }

    pub fn effect(&self) -> &AnyEffect {
        &self.effect
    }

    /// Takes the slot apart, for moving its effect elsewhere.
    pub fn into_effect(self) -> AnyEffect {
        self.effect
    }

    /// Samples by which the slot delays its signal. It is the effect's
    /// latency whether the slot is on or off.
    pub fn latency_samples(&self) -> usize {
        self.effect.latency_samples()
    }

    /// See [`Effect::tail_samples`]. Include wet memory until the bypass or
    /// mix fade settles at zero, as well as current and old aligned dry taps.
    pub fn tail_samples(&self) -> usize {
        self.dry_fade
            .longest_delay()
            .max(self.effect.latency_samples())
            .max(if self.wet.value() > 0.0 || !self.wet.is_settled() {
                self.effect.tail_samples()
            } else {
                0
            })
    }

    /// See [`Effect::gap_samples`]. A bypass/mix fade can still reveal wet
    /// memory. Only a settled zero-wet slot uses dry-tap accounting alone.
    pub fn gap_samples(&self) -> usize {
        self.dry_fade
            .longest_delay()
            .max(self.effect.latency_samples())
            .max(if self.wet.value() > 0.0 || !self.wet.is_settled() {
                self.effect.gap_samples()
            } else {
                0
            })
    }

    /// Processes one block in place. Blocks longer than the `max_block`
    /// given to [`EffectSlot::prepare`] are worked through in pieces.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let frames = left.len().min(right.len());
        let piece = self.scratch_left.len().max(1);
        let mut start = 0;
        while start < frames {
            let end = (start + piece).min(frames);
            self.process_piece(&mut left[start..end], &mut right[start..end]);
            start = end;
        }
    }

    fn process_piece(&mut self, left: &mut [f32], right: &mut [f32]) {
        let fresh = std::mem::replace(&mut self.fresh, false);
        let frames = left.len();
        if self.scratch_left.len() < frames {
            // Not prepared: there is nowhere to keep the dry signal.
            return;
        }
        for sample in left.iter_mut().chain(right.iter_mut()) {
            *sample = if sample.is_finite() {
                sample.clamp(-INPUT_LIMIT, INPUT_LIMIT)
            } else {
                0.0
            };
        }

        let latency = self.effect.latency_samples();
        let fade_samples = if fresh {
            0
        } else {
            ms_to_samples(LOOKAHEAD_FADE_MS, self.sample_rate)
        };
        let matrix = self.effect.kind() == EffectKind::StereoMatrix;
        let readiness = self.effect.delay_readiness_samples();
        if !matrix {
            self.dry_fade.retarget_joining(latency, fade_samples);
        }
        let dry_transition = self.dry_fade.remaining() as usize;
        let dry_left = &mut self.scratch_left[..frames];
        let dry_right = &mut self.scratch_right[..frames];
        for index in 0..frames {
            let ready = fresh || !matrix || self.dry_history >= readiness;
            if matrix && ready {
                self.dry_fade.retarget(latency, fade_samples);
            }
            self.dry_aligned[index] = ready && self.dry_fade.remaining() == 0;
            for (line, input, dry) in [
                (&mut self.dry_left, left[index], &mut dry_left[index]),
                (&mut self.dry_right, right[index], &mut dry_right[index]),
            ] {
                *dry = self
                    .dry_fade
                    .read(|delay| if delay == 0 { input } else { line.tap(delay) });
                // Prime even at zero shared latency for later delayed edits.
                line.push(input);
            }
            self.dry_fade.advance();
            self.dry_history = (self.dry_history + 1).min(self.dry_left.max_delay());
        }

        if self.wet.is_settled() && self.wet.value() == 0.0 {
            left.copy_from_slice(dry_left);
            right.copy_from_slice(dry_right);
            self.dormant = true;
            return;
        }
        if self.dormant {
            self.effect.reset();
            self.dormant = false;
            self.warm_up = self.effect.warm_up_samples().max(dry_transition);
            self.warm_up_elapsed = 0;
            self.warming = true;
        }
        if self.warm_up > 0 {
            // Reset snaps the wet taps, so keep them unheard until any
            // simultaneous dry/PDC transition has also reached its target.
            self.warm_up = self.warm_up.max(dry_transition);
        }
        self.effect.process(left, right);
        if self.warm_up == 0 && self.wet.is_settled() && self.wet.value() == 1.0 {
            return;
        }
        for index in 0..frames {
            // The fade waits until the effect's output has arrived.
            let wet = if self.warming && (self.warm_up > 0 || !self.dry_aligned[index]) {
                self.warm_up = self.warm_up.saturating_sub(1);
                0.0
            } else {
                self.warming = false;
                self.wet.tick()
            };
            if wet == 0.0 {
                left[index] = dry_left[index];
                right[index] = dry_right[index];
            } else if wet != 1.0 {
                left[index] = dry_left[index] + (left[index] - dry_left[index]) * wet;
                right[index] = dry_right[index] + (right[index] - dry_right[index]) * wet;
            }
        }
        self.warm_up_elapsed = self.warm_up_elapsed.saturating_add(frames);
        // A fade can finish inside this piece. Waking must clear stale state
        // even if no subsequent fully dry block arrived before re-enabling.
        self.dormant = self.wet.is_settled() && self.wet.value() == 0.0;
    }
}
