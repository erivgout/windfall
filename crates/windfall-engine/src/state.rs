//! Everything of a plan that moves while it plays: the gain and pan ramps,
//! the effects and instruments themselves, and the delays that line paths
//! of different latency up.
//!
//! # Hosting
//!
//! The control side builds a [`PlanState`] for every plan it sends. It
//! keeps a [`Ledger`] of what the audio thread holds, so the new state
//! comes with only what is new: an effect or instrument nobody has built
//! yet, or a delay line longer than the one in use. Every other seat is
//! empty, and the audio thread fills it in [`PlanState::take_over`] by
//! moving the unit across from the state it is replacing, matched by id.
//! That is what lets a reverb tail or a compressor's envelope live through
//! any number of edits. What the new plan has no seat for stays in the old
//! state and is dropped with it on the control side.
//!
//! # Delay compensation
//!
//! An instrument or effect may put its output out late: the limiter by its
//! look-ahead, the synth by a few samples. Every signal that meets another
//! is delayed to match the slowest of them, so all paths from a note to
//! the output take the same time. A track's input has three kinds of
//! source, each with a delay of its own: its sampler voices together, each
//! instrument that plays into it, and each track that feeds it through an
//! output or a send. The time a track's input is behind is the longest
//! time any of its sources is behind, and its output is behind by that
//! plus the latency of its own effects. The master's figure is the latency
//! of the whole engine, [`PlanState::latency`].

use std::collections::HashMap;
use std::ops::Range;

use windfall_core::{db_to_gain, pan_gains};
use windfall_dsp::GainReductionMeter;
use windfall_project::{
    ChannelId, EffectId, EffectKind, InstrumentKind, MAX_MIXER_TRACKS, TrackId,
};

use crate::automation::{self, LaneState};
use crate::plan::Plan;
use crate::rack::{Compensation, DelayStage, EffectUnit, InstrumentUnit};
use crate::ramp::Ramp;
use crate::render::TAIL_SILENCE_DB;
use crate::sequencer::Clock;

/// Time a gain or pan change takes to arrive.
const RAMP_SECONDS: f64 = 0.005;

/// Time an automated value takes to return to the stored one while the
/// transport is stopped. Something may still be ringing then, so it is long
/// enough to be heard as a fade and not as a jump.
const RESTORE_SECONDS: f64 = 0.1;

/// Time a compensation delay takes to change length. It is the time the
/// limiter takes to change its look-ahead.
const DELAY_FADE_SECONDS: f64 = 0.005;

/// Time an effect takes to fade into or out of a chain. It is the time the
/// delays that make up for the effect's latency take to change, so the
/// effect and those delays move as one and every path stays lined up
/// while they do.
const SPLICE_SECONDS: f64 = DELAY_FADE_SECONDS;

/// How long after its last sound, in frames, a track or instrument still
/// counts as sounding when a plan changes.
const RECENT_FRAMES: u64 = 64;

/// Frames a track's effects are given, on top of what they say they need,
/// before a tail that has died away is taken to be over. A filter in an
/// echo's path spreads the echo over a few frames more than its delay.
const TAIL_SLACK_FRAMES: u64 = 256;

/// Lengths, in frames, of the fades a change of plan can start.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Fades {
    /// A gain or pan on its way to a new value.
    pub ramp: u64,
    /// An automated value on its way back to the stored one while the
    /// transport is stopped.
    pub restore: u64,
    /// An effect joining or leaving a chain.
    pub splice: u32,
    /// A compensation delay changing length.
    pub delay: u32,
}

impl Fades {
    pub fn at(sample_rate: u32) -> Self {
        let frames = |seconds: f64| (seconds * f64::from(sample_rate)).round().max(1.0);
        Self {
            ramp: frames(RAMP_SECONDS) as u64,
            restore: frames(RESTORE_SECONDS) as u64,
            splice: frames(SPLICE_SECONDS) as u32,
            delay: frames(DELAY_FADE_SECONDS) as u32,
        }
    }
}

/// The smoothed gain and pan of a channel or mixer track.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Strip {
    pub gain: Ramp,
    pub pan: Ramp,
}

impl Strip {
    pub fn at_rest(gain: f32, pan: f32) -> Self {
        Self {
            gain: Ramp::at_rest(gain),
            pan: Ramp::at_rest(pan),
        }
    }

    fn retarget(&mut self, gain: f32, pan: f32, now: u64, frames: u64) {
        self.gain.retarget(gain, now, frames);
        self.pan.retarget(pan, now, frames);
    }
}

/// Where something sounding plays into, for [`PlanState::take_over`].
#[derive(Debug, Clone, Copy)]
pub(crate) enum Heard {
    /// Through the channel at this index of the new plan.
    Channel(usize),
    /// Straight into the track at this index of the new plan.
    Track(usize),
}

/// What a compensation delay delays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DelayKey {
    /// The sampler voices that play into a track.
    Direct(TrackId),
    /// An instrument channel on its way into its track.
    Instrument(ChannelId),
    /// A track's output or send on its way into another track.
    Edge {
        from: TrackId,
        to: TrackId,
        send: bool,
    },
}

/// A place for a compensation delay.
pub(crate) struct DelaySlot {
    /// Frames to delay by.
    target: usize,
    /// The line. `None` while it waits to be moved across from the state
    /// being replaced, and for good in a place that needs no delay.
    pub line: Option<Compensation>,
    /// The ordered shared-delay transfer to mirror, if it contains a matrix.
    path: Vec<DelayStage>,
    /// The place needs a line.
    used: bool,
}

impl DelaySlot {
    fn none() -> Self {
        Self {
            target: 0,
            line: None,
            path: Vec::new(),
            used: false,
        }
    }

    /// The slot for a delay of `delay` frames that could grow to `reach`
    /// with the plan's effects at other settings. A place that neither
    /// needs a delay nor has one fading away gets none. A matrix reference
    /// path retains its zero-delay stages to prime the first delayed edit;
    /// a plain project still pays nothing.
    fn seat(
        key: DelayKey,
        delay: usize,
        reach: usize,
        path: Vec<DelayStage>,
        held: Option<&Ledger>,
        ledger: &mut Ledger,
    ) -> Self {
        let before = held.and_then(|held| held.delays.get(&key));
        let was = before.map_or(0, |(target, _, _)| *target);
        if delay == 0 && was == 0 && path.is_empty() {
            return Self::none();
        }
        let needed = delay.max(was);
        let line = match before {
            Some((_, capacity, old_path))
                if *capacity >= needed
                    && old_path.len() == path.len()
                    && old_path
                        .iter()
                        .zip(&path)
                        .all(|(old, new)| old.same_line(new)) =>
            {
                ledger.delays.insert(key, (delay, *capacity, path.clone()));
                None
            }
            _ => {
                // With a processor running the line starts out as no delay
                // at all, and `take_over` decides how it gets to its length.
                let start = if held.is_some() { 0 } else { delay };
                let line = if path.is_empty() {
                    Compensation::new(needed.max(reach), start)
                } else {
                    Compensation::with_path(needed.max(reach), start, &path, held.is_some())
                };
                ledger
                    .delays
                    .insert(key, (delay, line.capacity(), path.clone()));
                Some(line)
            }
        };
        Self {
            target: delay,
            line,
            path,
            used: true,
        }
    }

    /// A serial reference mirrors each generation's actual outer clock.
    /// Scalar fallback keeps the existing aggregate wait policy.
    fn align_splices(&mut self, plan: &Plan, chains: &[Vec<Option<EffectUnit>>]) {
        for stage in &mut self.path {
            stage.wait = plan
                .tracks
                .iter()
                .enumerate()
                .find_map(|(track, entry)| {
                    let place = entry
                        .effects
                        .iter()
                        .position(|effect| effect.life.generation == stage.generation)?;
                    chains[track][place]
                        .as_ref()
                        .map(EffectUnit::insertion_wait)
                })
                .unwrap_or(0);
        }
    }

    /// Carries on from the slot that delayed the same thing under the plan
    /// being replaced: its line is moved here, or its past copied into the
    /// longer line built to replace it, and the delay crossfades to its
    /// new length once `wait` frames have passed.
    ///
    /// A line with no line before it has no past. If `sounding` says sound
    /// is passing where it sits, it stays at no delay until it has taken
    /// in as much as it is to delay by, and only then crossfades to its
    /// length. With nothing passing it is at its length at once.
    fn take_over(
        &mut self,
        old: Option<&mut DelaySlot>,
        sounding: bool,
        fade_frames: u32,
        wait: u32,
    ) {
        if !self.used {
            return;
        }
        let target = self.target;
        let before = old.filter(|old| old.line.is_some());
        match (&mut self.line, before) {
            // The old line is left where it is, to be freed with its state.
            (Some(line), Some(old)) => {
                if let Some(before) = &old.line {
                    line.take_history(before);
                }
                line.retarget_path(target, &self.path, fade_frames, wait);
            }
            (None, Some(old)) => {
                self.line = old.line.take();
                if let Some(line) = &mut self.line {
                    line.retarget_path(target, &self.path, fade_frames, wait);
                }
            }
            (Some(line), None) if sounding => {
                let fill = u32::try_from(target).unwrap_or(u32::MAX);
                line.retarget_path(target, &self.path, fade_frames, wait.max(fill));
            }
            (Some(line), None) => line.snap(target, &self.path),
            // The line that was to come never did, so there is no delay.
            (None, None) => self.used = false,
        }
    }

    /// Frames of sound the line can still be holding after its input has
    /// gone silent.
    fn holds(&self) -> u64 {
        self.line.as_ref().map_or(0, Compensation::capacity) as u64
    }
}

/// An instrument channel's seat: the instrument and the delay that lines
/// it up with the rest of its track.
pub(crate) struct InstrumentSeat {
    /// `None` while the unit waits to be moved across from the state being
    /// replaced.
    pub unit: Option<InstrumentUnit>,
    pub delay: DelaySlot,
}

/// When a track last had sound at each end of its effects, as one past the
/// frame of the last sample that counted. Zero means never.
///
/// Two things count. Any sample that is not zero is a signal that a change
/// of plan must not make click. Only a sample at or above the level of
/// [`TAIL_SILENCE_DB`] is sound that a tail has to wait for: a reverb goes
/// on putting out numbers that are not zero long after it has died away.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Activity {
    /// Going into the effects.
    pub input: u64,
    /// Coming out of them.
    pub output: u64,
    /// The same two for sound a tail waits for.
    loud_input: u64,
    loud_output: u64,
}

impl Activity {
    /// Notes the sound in a block of track input that starts on frame
    /// `base`. `silence` is the level below which a tail is over.
    pub fn hear_input(&mut self, base: u64, block: &[[f32; 2]], silence: f32) {
        let (last, loud) = last_sound(block, silence);
        if let Some(last) = last {
            self.input = base + last as u64 + 1;
        }
        if let Some(loud) = loud {
            self.loud_input = base + loud as u64 + 1;
        }
    }

    pub fn hear_output(&mut self, base: u64, block: &[[f32; 2]], silence: f32) {
        let (last, loud) = last_sound(block, silence);
        if let Some(last) = last {
            self.output = base + last as u64 + 1;
        }
        if let Some(loud) = loud {
            self.loud_output = base + loud as u64 + 1;
        }
    }

    fn recent(&self, now: u64) -> bool {
        self.input.max(self.output) + RECENT_FRAMES > now && self.input.max(self.output) > 0
    }
}

/// The last frame of a block that is not zero, and the last that is at or
/// above `silence` on either side.
fn last_sound(block: &[[f32; 2]], silence: f32) -> (Option<usize>, Option<usize>) {
    let last = block
        .iter()
        .rposition(|frame| frame[0] != 0.0 || frame[1] != 0.0);
    let loud = last.and_then(|last| {
        block[..=last]
            .iter()
            .rposition(|frame| frame[0].abs() >= silence || frame[1].abs() >= silence)
    });
    (last, loud)
}

/// What the control side knows the audio thread to hold once it has taken
/// the last plan sent: enough to build the next [`PlanState`] with only
/// what is missing.
pub(crate) struct Ledger {
    plugins: HashMap<windfall_project::PluginTarget, (u64, usize)>,
    /// Latency metadata only, excluded from active native-owner reuse.
    departing_plugins: HashMap<(windfall_project::PluginTarget, u64), (u64, usize)>,
    pub sample_rate: u32,
    effects: HashMap<EffectId, EffectKind>,
    instruments: HashMap<ChannelId, InstrumentKind>,
    /// The length each compensation delay is set to and the longest its
    /// line can give, plus the identity and prepared bounds of its stages.
    delays: HashMap<DelayKey, (usize, usize, Vec<DelayStage>)>,
    /// The gain reduction meters of the compressors and limiters, in mixer
    /// order.
    pub meters: Vec<(EffectId, GainReductionMeter)>,
    /// Frames by which the output lags the notes.
    pub latency: u32,
}

/// How far behind each track's signal is.
struct Layout {
    /// Frames by which the input of each track is behind.
    arrival: Vec<usize>,
    /// The same for what leaves its effects.
    out: Vec<usize>,
    /// The most `arrival` could become with the same effects at other
    /// settings. Delay lines are made this long, so that moving a
    /// limiter's look-ahead never needs a longer one.
    reach: Vec<usize>,
    arrival_path: Vec<Vec<DelayStage>>,
    out_path: Vec<Vec<DelayStage>>,
}

impl Layout {
    fn of(
        plan: &Plan,
        sample_rate: u32,
        plugins: &HashMap<windfall_project::PluginTarget, (u64, usize)>,
        known_plugins: &HashMap<(windfall_project::PluginTarget, u64), (u64, usize)>,
    ) -> Self {
        let rate = sample_rate as f32;
        // No path is compensated for more than a second of latency. Past
        // that it plays late instead.
        let most = sample_rate as usize;
        let count = plan.tracks.len();
        let mut arrival = vec![0_usize; count];
        let mut reach = vec![0_usize; count];
        let mut out = vec![0_usize; count];
        let mut arrival_path = vec![Vec::new(); count];
        let mut out_path = vec![Vec::new(); count];
        // An instrument that has left the project is only fading out, and
        // nothing waits for it.
        for channel in plan.channels.iter().filter(|channel| !channel.leaving) {
            if let Some(params) = &channel.instrument {
                let latency = plugins
                    .get(&windfall_project::PluginTarget::Instrument {
                        channel: channel.id,
                    })
                    .map_or_else(|| params.latency_samples(rate), |record| record.1)
                    .min(most);
                if latency > arrival[channel.track] {
                    arrival_path[channel.track] = vec![DelayStage {
                        key: windfall_project::PluginTarget::Instrument {
                            channel: channel.id,
                        },
                        generation: 0,
                        wait: 0,
                        delay: latency,
                        maximum: latency,
                        readiness: latency,
                        matrix: false,
                        leaving: false,
                    }];
                }
                arrival[channel.track] = arrival[channel.track].max(latency);
                reach[channel.track] = reach[channel.track].max(latency);
            }
        }
        // In routing order every track that feeds a track is done before it.
        for &index in &plan.order {
            let track = &plan.tracks[index];
            let chain = track.effects.iter();
            let mut path = arrival_path[index].clone();
            let (latency, longest) = chain.fold((0, 0), |(latency, longest), effect| {
                let key = windfall_project::PluginTarget::Effect { effect: effect.id };
                if effect.leaving {
                    let native = known_plugins.get(&(key, effect.life.generation));
                    let maximum = native.map_or_else(
                        || effect.params.kind().max_latency_samples(rate),
                        |record| record.1,
                    );
                    if maximum > 0 {
                        path.push(DelayStage {
                            key,
                            generation: effect.life.generation,
                            wait: 0,
                            leaving: true,
                            delay: 0,
                            maximum,
                            readiness: 0,
                            matrix: native.is_none()
                                && effect.params.kind() == EffectKind::StereoMatrix,
                        });
                        // Every departing delayed processor keeps its stage
                        // until the outer removal splice drains through the route.
                        return (latency, longest + maximum);
                    }
                    return (latency, longest);
                }
                if let Some(record) = plugins.get(&key) {
                    if record.1 > 0 {
                        path.push(DelayStage {
                            key,
                            generation: effect.life.generation,
                            wait: 0,
                            leaving: false,
                            delay: record.1,
                            maximum: record.1,
                            readiness: record.1,
                            matrix: false,
                        });
                    }
                    return (latency + record.1, longest + record.1);
                }
                let delay = effect.params.latency_samples(rate);
                let maximum = effect.params.kind().max_latency_samples(rate);
                if maximum > 0 {
                    path.push(DelayStage {
                        key,
                        generation: effect.life.generation,
                        wait: 0,
                        leaving: false,
                        delay,
                        maximum,
                        readiness: match effect.params {
                            windfall_project::EffectParams::StereoMatrix(params) => {
                                params.delay_readiness_samples(rate)
                            }
                            _ => delay,
                        },
                        matrix: effect.params.kind() == EffectKind::StereoMatrix,
                    });
                }
                (latency + delay, longest + maximum)
            });
            out[index] = arrival[index] + latency;
            out_path[index] = path;
            let out_reach = reach[index] + longest;
            for edge in &track.edges {
                // A zero-delay matrix is still a potential reference path.
                // Keep its stages primed while it shares latency zero.
                if out[index] > arrival[edge.target]
                    || (out[index] == arrival[edge.target] && out_reach > reach[edge.target])
                {
                    arrival_path[edge.target] = out_path[index].clone();
                }
                arrival[edge.target] = arrival[edge.target].max(out[index]).min(most);
                reach[edge.target] = reach[edge.target].max(out_reach).min(most);
            }
        }
        Self {
            arrival,
            out,
            reach,
            arrival_path,
            out_path,
        }
    }

    /// Factor a reference transfer into the incoming prefix and the serial
    /// stages still needed. Fixed instrument/plugin prefixes can be removed
    /// by their length. Distinct varying branches have no causal inverse;
    /// those retain scalar PDC, as do paths beyond the one-second host bound.
    fn compensation_path(
        &self,
        to: usize,
        input: &[DelayStage],
        delay: usize,
        most: usize,
    ) -> Vec<DelayStage> {
        let reference = &self.arrival_path[to];
        let mut path = if reference.starts_with(input) {
            reference[input.len()..].to_vec()
        } else if input
            .iter()
            .all(|stage| !stage.matrix && stage.maximum == stage.delay)
        {
            let mut remove: usize = input.iter().map(|stage| stage.delay).sum();
            let mut path = reference.clone();
            for stage in &mut path {
                if stage.matrix || stage.maximum != stage.delay {
                    break;
                }
                let taken = remove.min(stage.delay);
                stage.delay -= taken;
                stage.maximum -= taken;
                remove -= taken;
            }
            if remove > 0 {
                return Vec::new();
            }
            path.retain(|stage| stage.maximum > 0);
            path
        } else {
            return Vec::new();
        };
        if !path.iter().any(|stage| stage.matrix)
            || path.iter().map(|stage| stage.delay).sum::<usize>() != delay
            || path.iter().map(|stage| stage.maximum).sum::<usize>() > most
        {
            path.clear();
        }
        path
    }
}

pub(crate) struct PlanState {
    pub channels: Vec<Strip>,
    pub tracks: Vec<Strip>,
    pub edges: Vec<Ramp>,
    /// The effects of each track, seat for seat with the plan's. A seat is
    /// empty while its effect waits to be moved across from the state
    /// being replaced, and stays empty if it never turns up, which lets
    /// the signal pass.
    pub chains: Vec<Vec<Option<EffectUnit>>>,
    /// The instrument of each channel. `None` for a sampler channel.
    pub instruments: Vec<Option<InstrumentSeat>>,
    /// The delay of each track's own sampler voices.
    pub direct: Vec<DelaySlot>,
    /// The delay on each routing edge, by the edge's slot.
    pub edge_delays: Vec<DelaySlot>,
    pub activity: Vec<Activity>,
    /// For each track, whether anything between its input and the output
    /// has a memory: an effect or a compensation delay on the track or
    /// further along. A voice cannot leave such a path without a jump.
    pub shaped: Vec<bool>,
    /// Frames by which the output lags the notes that make it.
    pub latency: u64,
    /// The same for what leaves each track, past its effects and its
    /// fader. The master's is `latency`.
    pub behind: Vec<usize>,
    /// The level below which a tail counts as over, as a linear gain.
    pub silence: f32,
    /// What each automation lane of the plan is doing.
    pub lanes: Vec<LaneState>,
    /// How many of them have their target in hand.
    pub engaged: usize,
    /// How many of them are walking a setting back to its stored value.
    pub returning: usize,
    /// The tempo the effects and instruments were last told, in beats per
    /// minute. It follows the tempo map while the song plays.
    pub tempo_told: f64,
}

impl PlanState {
    pub fn tempo(&self) -> f64 {
        self.tempo_told
    }
    /// Builds the state for `plan` on a processor that runs at
    /// `sample_rate`. With `held`, what the processor holds now, only what
    /// it lacks is built and the rest is left for
    /// [`PlanState::take_over`]. Without it, as for a new processor,
    /// everything is built and nothing is fading in or out.
    ///
    /// Allocates and prepares effects, so it is for the control side.
    pub fn build(plan: &Plan, sample_rate: u32, held: Option<&Ledger>) -> (Self, Ledger) {
        let sample_rate = sample_rate.max(1);
        let empty = HashMap::new();
        let known_plugins = held
            .filter(|held| held.sample_rate == sample_rate)
            .map_or(&empty, |held| &held.plugins);
        let (mut prepared, plugins) = crate::plugins::prepare(plan, sample_rate, known_plugins);
        let departing_plugins = plan
            .tracks
            .iter()
            .flat_map(|track| &track.effects)
            .filter(|effect| effect.leaving)
            .filter_map(|effect| {
                let key = windfall_project::PluginTarget::Effect { effect: effect.id };
                let generation = (key, effect.life.generation);
                let record = held
                    .filter(|held| held.sample_rate == sample_rate)
                    .and_then(|held| held.departing_plugins.get(&generation))
                    .or_else(|| known_plugins.get(&key))?;
                Some((generation, *record))
            })
            .collect();
        let layout = Layout::of(plan, sample_rate, &plugins, &departing_plugins);
        let mut ledger = Ledger {
            plugins,
            departing_plugins,
            sample_rate,
            effects: HashMap::new(),
            instruments: HashMap::new(),
            delays: HashMap::new(),
            meters: Vec::new(),
            latency: u32::try_from(layout.out[0]).unwrap_or(u32::MAX),
        };

        let mut edges = vec![Ramp::at_rest(0.0); plan.edge_count];
        let mut edge_delays: Vec<DelaySlot> =
            (0..plan.edge_count).map(|_| DelaySlot::none()).collect();
        let mut direct = Vec::with_capacity(plan.tracks.len());
        let mut chains = Vec::with_capacity(plan.tracks.len());
        for (index, track) in plan.tracks.iter().enumerate() {
            for edge in &track.edges {
                edges[edge.slot] = Ramp::at_rest(edge.gain);
                let key = DelayKey::Edge {
                    from: track.id,
                    to: edge.target_id,
                    send: edge.send,
                };
                let delay = layout.arrival[edge.target].saturating_sub(layout.out[index]);
                let reach = layout.reach[edge.target];
                let path = layout.compensation_path(
                    edge.target,
                    &layout.out_path[index],
                    delay,
                    sample_rate as usize,
                );
                edge_delays[edge.slot] =
                    DelaySlot::seat(key, delay, reach, path, held, &mut ledger);
            }
            direct.push(DelaySlot::seat(
                DelayKey::Direct(track.id),
                layout.arrival[index],
                layout.reach[index],
                layout.compensation_path(index, &[], layout.arrival[index], sample_rate as usize),
                held,
                &mut ledger,
            ));

            let chain = track.effects.iter().map(|effect| {
                let kind = effect.params.kind();
                let target = windfall_project::PluginTarget::Effect { effect: effect.id };
                let known = held.and_then(|held| held.effects.get(&effect.id)) == Some(&kind)
                    && held.is_some_and(|held| {
                        held.plugins.get(&target) == ledger.plugins.get(&target)
                    });
                if effect.leaving {
                    // Only there to fade out what is already playing.
                    return None;
                }
                ledger.effects.insert(effect.id, kind);
                if known {
                    let meters = held.map_or(&[][..], |held| &held.meters);
                    let meter = meters.iter().find(|(id, _)| *id == effect.id);
                    ledger.meters.extend(meter.cloned());
                    return None;
                }
                let (mut unit, meter) =
                    EffectUnit::build(effect, track.id, sample_rate, plan.tempo_bpm);
                if let Some(binding) = plan.plugins.iter().find(|binding| binding.target == target)
                {
                    let native = match prepared.remove(&target) {
                        Some(crate::plugins::PreparedPlugin::Effect(unit)) => unit,
                        _ => None,
                    };
                    unit.install_plugin(native, binding);
                }
                ledger.meters.extend(meter.map(|meter| (effect.id, meter)));
                Some(unit)
            });
            chains.push(chain.collect());
        }

        let instruments = plan.channels.iter().map(|channel| {
            let params = channel.instrument.as_ref()?;
            let kind = params.kind();
            let target = windfall_project::PluginTarget::Instrument {
                channel: channel.id,
            };
            let known = held.and_then(|held| held.instruments.get(&channel.id)) == Some(&kind)
                && held
                    .is_some_and(|held| held.plugins.get(&target) == ledger.plugins.get(&target));
            let unit = if channel.leaving {
                if !known {
                    return None;
                }
                None
            } else {
                ledger.instruments.insert(channel.id, kind);
                (!known).then(|| {
                    let mut unit = InstrumentUnit::build(params, sample_rate, plan.tempo_bpm);
                    if let Some(binding) =
                        plan.plugins.iter().find(|binding| binding.target == target)
                    {
                        let native = match prepared.remove(&target) {
                            Some(crate::plugins::PreparedPlugin::Instrument(unit)) => unit,
                            _ => None,
                        };
                        unit.install_plugin(native, binding);
                    }
                    unit
                })
            };
            let behind = ledger.plugins.get(&target).map_or_else(
                || params.latency_samples(sample_rate as f32),
                |record| record.1,
            );
            Some(InstrumentSeat {
                unit,
                delay: DelaySlot::seat(
                    DelayKey::Instrument(channel.id),
                    layout.arrival[channel.track].saturating_sub(behind),
                    layout.reach[channel.track],
                    layout.compensation_path(
                        channel.track,
                        &[DelayStage {
                            key: target,
                            generation: 0,
                            wait: 0,
                            leaving: false,
                            delay: behind,
                            maximum: behind,
                            readiness: behind,
                            matrix: false,
                        }],
                        layout.arrival[channel.track].saturating_sub(behind),
                        sample_rate as usize,
                    ),
                    held,
                    &mut ledger,
                ),
            })
        });
        let instruments = instruments.collect();

        // Against the routing order, so that the tracks a track feeds are
        // done before it.
        let mut shaped = vec![false; plan.tracks.len()];
        for &index in plan.order.iter().rev() {
            let track = &plan.tracks[index];
            let mut edges = track.edges.iter();
            let onward = edges.any(|edge| edge_delays[edge.slot].used || shaped[edge.target]);
            shaped[index] = onward || !track.effects.is_empty() || direct[index].used;
        }

        let state = Self {
            channels: plan
                .channels
                .iter()
                .map(|channel| Strip::at_rest(channel.gain, channel.pan))
                .collect(),
            tracks: plan
                .tracks
                .iter()
                .map(|track| Strip::at_rest(track.gain, track.pan))
                .collect(),
            edges,
            chains,
            instruments,
            direct,
            edge_delays,
            activity: vec![Activity::default(); plan.tracks.len()],
            shaped,
            latency: u64::from(ledger.latency),
            behind: layout.out,
            silence: db_to_gain(TAIL_SILENCE_DB),
            lanes: vec![LaneState::default(); plan.lanes.len()],
            engaged: 0,
            returning: 0,
            tempo_told: plan.tempo_bpm,
        };
        (state, ledger)
    }

    /// The gain at which each side of what is put into each track of `plan`
    /// comes out of the master on frame `now`: through the track's own
    /// fader and pan, and on through every output and send from there.
    /// Effects are left out of it. Left, then right, by track index.
    pub fn gains_to_output(&self, plan: &Plan, now: u64) -> [[f32; 2]; MAX_MIXER_TRACKS] {
        let mut gains = [[0.0; 2]; MAX_MIXER_TRACKS];
        // Against the routing order, so that the tracks a track feeds are
        // done before it.
        for &index in plan.order.iter().rev() {
            let mut onward = [0.0; 2];
            if index == 0 {
                onward = [1.0; 2];
            }
            for edge in &plan.tracks[index].edges {
                let gain = self.edges[edge.slot].at(now);
                onward[0] += gain * gains[edge.target][0];
                onward[1] += gain * gains[edge.target][1];
            }
            let strip = &self.tracks[index];
            let (left, right) = pan_gains(strip.pan.at(now));
            let gain = strip.gain.at(now);
            gains[index] = [gain * left * onward[0], gain * right * onward[1]];
        }
        gains
    }

    /// Takes over from the state of the plan being replaced on frame
    /// `now`. `sounding` says where each sampler voice in the mix plays
    /// into.
    ///
    /// First the effects, instruments and delay lines the new plan still
    /// has are moved across, matched by id, and told whatever changed
    /// about them. They keep everything they remember.
    ///
    /// Then the changes are made gentle where a listener would hear them
    /// as a click, and only there. Sound is passing through a channel with
    /// a voice or a sounding instrument on it, through a track whose
    /// effects were sounding a moment ago, and through every track either
    /// of those reaches. Such a channel or track carries its gain and pan
    /// over and starts each value that changed moving to its new target;
    /// an effect that is new to such a track fades in from the signal as
    /// it enters, and one that left the project fades out to it. Everything
    /// else is left as [`PlanState::build`] made it, resting on its new
    /// values: nothing is heard through it, so there is nothing to glide
    /// from. That is how the first project a processor is given, a project
    /// that replaces another, and a channel or track that is new all start
    /// exactly at their values.
    ///
    /// A compensation delay that changes length crossfades whether or not
    /// anything is known to be sounding, because its line may hold sound
    /// that no voice accounts for. It waits first for as long as an effect
    /// that is coming in stays unheard, so that the two change together
    /// and the paths stay lined up throughout. Only a delay in a place
    /// that had none, with nothing sounding there, is at its length at
    /// once.
    ///
    /// Runs on the audio thread, so it must not allocate.
    pub fn take_over(
        &mut self,
        plan: &Plan,
        old_plan: &Plan,
        old: &mut PlanState,
        sounding: impl Iterator<Item = Heard>,
        now: u64,
        fades: Fades,
    ) {
        // Under a tempo map the processors may have been told any tempo
        // along the way.
        let retempo = plan.tempo_bpm != old_plan.tempo_bpm || old.tempo_told != plan.tempo_bpm;
        let mut heard = [false; MAX_MIXER_TRACKS];
        for destination in sounding {
            let index = match destination {
                Heard::Track(track) => {
                    heard[track] = true;
                    continue;
                }
                Heard::Channel(index) => index,
            };
            let channel = &plan.channels[index];
            heard[channel.track] = true;
            if let Some(found) = old_plan.channel(channel.id) {
                // Several voices on one channel do this again to the same
                // end.
                let strip = &mut self.channels[index];
                *strip = old.channels[found];
                strip.retarget(channel.gain, channel.pan, now, fades.ramp);
            }
        }

        for (index, channel) in plan.channels.iter().enumerate() {
            let (Some(seat), Some(params)) = (&mut self.instruments[index], &channel.instrument)
            else {
                continue;
            };
            let found = old_plan.channel(channel.id);
            let mut before = found.and_then(|found| old.instruments[found].as_mut());
            if let (Some(unit), Some(before)) = (
                &mut seat.unit,
                before.as_ref().and_then(|seat| seat.unit.as_ref()),
            ) {
                let target = windfall_project::PluginTarget::Instrument {
                    channel: channel.id,
                };
                let binding = plan.plugins.iter().find(|binding| binding.target == target);
                let old_binding = old_plan
                    .plugins
                    .iter()
                    .find(|binding| binding.target == target);
                if !channel.leaving
                    && binding.zip(old_binding).is_some_and(|(binding, old)| {
                        binding.path == old.path
                            && binding.id == old.id
                            && binding.format == old.format
                    })
                {
                    unit.inherit_plugin_notes(before);
                }
            }
            if seat.unit.is_none() {
                seat.unit = before
                    .as_mut()
                    .and_then(|before| before.unit.take_if(|unit| unit.kind() == params.kind()));
                if let Some(unit) = &mut seat.unit {
                    unit.apply(params);
                    if let Some(binding) = plan.plugins.iter().find(|binding| {
                        binding.target
                            == windfall_project::PluginTarget::Instrument {
                                channel: channel.id,
                            }
                    }) {
                        unit.apply_plugin(binding);
                    }
                    if retempo {
                        unit.set_tempo(plan.tempo_bpm);
                    }
                    if channel.leaving {
                        unit.silence();
                    }
                }
            }
            let sounding = seat
                .unit
                .as_ref()
                .is_some_and(|unit| unit.sounding(now, RECENT_FRAMES));
            if let (true, Some(found)) = (sounding, found) {
                heard[channel.track] = true;
                let strip = &mut self.channels[index];
                *strip = old.channels[found];
                strip.retarget(channel.gain, channel.pan, now, fades.ramp);
            }
        }
        // The tracks sampler voices play straight into, before `heard`
        // spreads along the routing.
        let voiced = heard;

        for (index, track) in plan.tracks.iter().enumerate() {
            if let Some(found) = old_plan.track_ids.get(track.id.0) {
                self.activity[index] = old.activity[found];
                // Without effects a track has no sound of its own to keep.
                let rings = !old_plan.tracks[found].effects.is_empty();
                heard[index] |= rings && self.activity[index].recent(now);
            }
        }

        // In routing order a track comes after everything that feeds it, so
        // by the time one is reached it is known whether sound gets there.
        for &index in &plan.order {
            if !heard[index] {
                continue;
            }
            let track = &plan.tracks[index];
            for edge in &track.edges {
                heard[edge.target] = true;
            }
            let Some(found) = old_plan.track_ids.get(track.id.0) else {
                continue;
            };
            let strip = &mut self.tracks[index];
            *strip = old.tracks[found];
            strip.retarget(track.gain, track.pan, now, fades.ramp);
            for edge in &track.edges {
                let before = old_plan.tracks[found]
                    .edges
                    .iter()
                    .find(|old_edge| {
                        old_edge.target_id == edge.target_id && old_edge.send == edge.send
                    })
                    .map(|old_edge| old.edges[old_edge.slot]);
                // A route that did not exist a moment ago fades in.
                let mut ramp = before.unwrap_or(Ramp::at_rest(0.0));
                ramp.retarget(edge.gain, now, fades.ramp);
                self.edges[edge.slot] = ramp;
            }
        }

        // How long the effects that are coming in stay unheard. The delays
        // that change to make up for them wait as long.
        let mut wait = 0;
        for (index, track) in plan.tracks.iter().enumerate() {
            for (place, effect) in track.effects.iter().enumerate() {
                // The outgoing definition is serially before its fresh active
                // successor, so its actual splice is already adopted here.
                let departure = effect
                    .after
                    .as_ref()
                    .and_then(|life| {
                        track.effects.iter().position(|before| {
                            before.leaving && std::sync::Arc::ptr_eq(&before.life, life)
                        })
                    })
                    .and_then(|before| self.chains[index][before].as_ref())
                    .map_or(0, EffectUnit::removal_remaining);
                let seat = &mut self.chains[index][place];
                let new = seat.is_some();
                if !new {
                    // Departing and restored active definitions may share
                    // a project id. Only the exact progress generation owns
                    // this seat; the id index deliberately finds active units.
                    let active = old_plan
                        .effect_ids
                        .get(effect.id.0)
                        .map(|index| old_plan.effect_places[index])
                        .filter(|&(track, place)| {
                            std::sync::Arc::ptr_eq(
                                &old_plan.tracks[track].effects[place].life,
                                &effect.life,
                            )
                        });
                    let held = active.or_else(|| {
                        old_plan
                            .tracks
                            .iter()
                            .enumerate()
                            .find_map(|(track, entry)| {
                                entry
                                    .effects
                                    .iter()
                                    .position(|before| {
                                        before.id == effect.id
                                            && std::sync::Arc::ptr_eq(&before.life, &effect.life)
                                    })
                                    .map(|place| (track, place))
                            })
                    });
                    let Some((old_track, old_place)) = held else {
                        if effect.leaving {
                            effect.life.finish();
                        }
                        continue;
                    };
                    *seat = old.chains[old_track][old_place]
                        .take_if(|unit| unit.kind() == effect.params.kind());
                }
                let Some(unit) = seat else {
                    if effect.leaving {
                        effect.life.finish();
                    }
                    continue;
                };
                if !new && !effect.leaving {
                    unit.apply(effect);
                    if let Some(binding) = plan.plugins.iter().find(|binding| {
                        binding.target
                            == windfall_project::PluginTarget::Effect { effect: effect.id }
                    }) {
                        unit.apply_plugin(binding);
                    }
                    if retempo {
                        unit.set_tempo(plan.tempo_bpm);
                    }
                }
                let arrived = new || unit.track != track.id;
                unit.track = track.id;
                match (effect.leaving, heard[index]) {
                    (true, true) => unit.fade_out(fades.splice),
                    (true, false) => unit.drop_out(),
                    (false, true) if arrived => {
                        unit.fade_in(fades.splice);
                        // Concrete preparation/arrival decides this wait,
                        // never inheritance of a shared progress marker.
                        unit.wait_for_departure(departure);
                    }
                    (false, _) => wait = wait.max(unit.insertion_wait()),
                }
            }
        }

        for unit in self.chains.iter().flatten().flatten() {
            wait = wait.max(unit.insertion_wait());
        }

        for (index, channel) in plan.channels.iter().enumerate() {
            let Some(seat) = &mut self.instruments[index] else {
                continue;
            };
            let found = old_plan.channel(channel.id);
            let before = found.and_then(|found| old.instruments[found].as_mut());
            let sounding = seat
                .unit
                .as_ref()
                .is_some_and(|unit| unit.sounding(now, RECENT_FRAMES));
            seat.delay.align_splices(plan, &self.chains);
            seat.delay.take_over(
                before.map(|before| &mut before.delay),
                sounding,
                fades.delay,
                wait,
            );
        }
        for (index, track) in plan.tracks.iter().enumerate() {
            let found = old_plan.track_ids.get(track.id.0);
            self.direct[index].align_splices(plan, &self.chains);
            self.direct[index].take_over(
                found.map(|found| &mut old.direct[found]),
                voiced[index],
                fades.delay,
                wait,
            );
            for edge in &track.edges {
                let before = found.and_then(|found| {
                    let edges = &old_plan.tracks[found].edges;
                    edges.iter().find(|old_edge| {
                        old_edge.target_id == edge.target_id && old_edge.send == edge.send
                    })
                });
                self.edge_delays[edge.slot].align_splices(plan, &self.chains);
                self.edge_delays[edge.slot].take_over(
                    before.map(|before| &mut old.edge_delays[before.slot]),
                    heard[index],
                    fades.delay,
                    wait,
                );
            }
        }
        automation::take_over(plan, self, old_plan, old);
    }

    /// The instrument of the channel at `index`, if it has one.
    pub fn instrument(&mut self, index: usize) -> Option<&mut InstrumentUnit> {
        self.instruments.get_mut(index)?.as_mut()?.unit.as_mut()
    }

    /// Tells every effect and instrument the tempo, if it is not the one
    /// they were last told.
    pub fn tell_tempo(&mut self, tempo_bpm: f64) {
        if tempo_bpm == self.tempo_told {
            return;
        }
        self.tempo_told = tempo_bpm;
        for unit in self.chains.iter_mut().flatten().flatten() {
            unit.set_tempo(tempo_bpm);
        }
        for unit in self.instrument_units() {
            unit.set_tempo(tempo_bpm);
        }
    }

    /// Every instrument.
    pub fn instrument_units(&mut self) -> impl Iterator<Item = &mut InstrumentUnit> {
        let seats = self.instruments.iter_mut().flatten();
        seats.filter_map(|seat| seat.unit.as_mut())
    }

    /// Voices the instruments have sounding between them.
    pub fn instrument_voices(&self) -> u32 {
        let seats = self.instruments.iter().flatten();
        let units = seats.filter_map(|seat| seat.unit.as_ref());
        units.map(InstrumentUnit::voices).sum()
    }

    /// Renders every instrument over `frames` of the block that starts on
    /// frame `base`. `clock` says on which frame each held note ends.
    pub fn render_instruments(&mut self, clock: Clock, base: u64, frames: Range<usize>) {
        for unit in self.instrument_units() {
            unit.render(clock, base, frames.start, frames.end);
        }
    }

    /// True when nothing at or above the level of [`TAIL_SILENCE_DB`] is
    /// left to come out as of frame `now`, given that no sampler voice is
    /// sounding and no note starts: every instrument has finished, and
    /// every track is done.
    ///
    /// A track is done once its input has been below that level for as
    /// long as its effects can ring and its delays can hold sound. It is
    /// done sooner once both what goes into its effects and what comes out
    /// of them has been below that level for as long as anything can still
    /// be on its way through them: a tail that has died away is over,
    /// however long the effect's settings say it could have lasted.
    /// Numbers that are not zero but far too small to hear do not keep a
    /// track alive.
    pub fn settled(&self, plan: &Plan, now: u64) -> bool {
        let mut seats = self.instruments.iter().flatten();
        let instruments_done = seats.all(|seat| {
            let unit = seat.unit.as_ref();
            unit.is_none_or(|unit| unit.settled(now.saturating_sub(seat.delay.holds())))
        });
        instruments_done
            && plan.tracks.iter().enumerate().all(|(index, track)| {
                let activity = &self.activity[index];
                if activity.loud_input == 0 && activity.loud_output == 0 {
                    return true;
                }
                let units = || self.chains[index].iter().flatten();
                let ring: usize = units().map(EffectUnit::tail_samples).sum();
                let gap: usize = units().map(EffectUnit::gap_samples).sum();
                let edges = track.edges.iter();
                let onward = edges.map(|edge| self.edge_delays[edge.slot].holds()).max();
                let held = self.direct[index].holds() + onward.unwrap_or(0);
                let rung_out = activity.loud_input + ring as u64 + held;
                let quiet_from = activity.loud_input.max(activity.loud_output);
                let died_away = quiet_from + gap as u64 + TAIL_SLACK_FRAMES + held;
                rung_out.min(died_away) <= now
            })
    }
}

#[cfg(test)]
mod tests {
    use windfall_dsp::{EffectParams, InstrumentParams, LimiterParams};
    use windfall_project::{
        Channel, ChannelSource, EffectSlot, MixerTrack, Project, SamplerSettings, Send,
    };

    use super::*;
    use crate::plan::compile;
    use crate::pool::SamplePool;

    const RATE: u32 = 48_000;

    fn limiter(id: u32, lookahead_ms: f32) -> EffectSlot {
        EffectSlot {
            id: EffectId(id),
            enabled: true,
            mix: 1.0,
            params: EffectParams::Limiter(LimiterParams {
                lookahead_ms,
                ..LimiterParams::default()
            }),
        }
    }

    fn track(id: u32, output: u32) -> MixerTrack {
        MixerTrack {
            id: TrackId(id),
            name: String::new(),
            color: 0,
            volume: 1.0,
            pan: 0.0,
            muted: false,
            solo: false,
            output: Some(TrackId(output)),
            sends: Vec::new(),
            effects: Vec::new(),
        }
    }

    fn channel(id: u32, track: u32, source: ChannelSource) -> Channel {
        Channel {
            id: ChannelId(id),
            name: String::new(),
            color: 0,
            volume: 1.0,
            pan: 0.0,
            muted: false,
            solo: false,
            mixer_track: TrackId(track),
            source,
        }
    }

    fn synth() -> ChannelSource {
        ChannelSource::Instrument {
            params: InstrumentParams::SubtractiveSynth(Default::default()),
        }
    }

    /// The delay each slot of a state is set to, and whether it has a line.
    fn delay(slot: &DelaySlot) -> Option<usize> {
        slot.used.then_some(slot.target)
    }

    #[test]
    fn a_project_with_no_latency_in_it_gets_no_delay_lines() {
        let mut project = Project::new("plain");
        project.mixer.tracks.push(track(1, 0));
        project.mixer.tracks[1].sends.push(Send {
            target: TrackId(0),
            gain: 0.5,
        });
        let sampler = ChannelSource::Sampler(SamplerSettings::default());
        project.channels.push(channel(10, 1, sampler));
        let plan = compile(&project, &SamplePool::new());
        let (state, ledger) = PlanState::build(&plan, RATE, None);
        assert_eq!(state.latency, 0);
        assert_eq!(ledger.latency, 0);
        assert!(state.direct.iter().all(|slot| !slot.used));
        assert!(state.edge_delays.iter().all(|slot| !slot.used));
        assert!(state.shaped.iter().all(|shaped| !shaped));
        assert!(ledger.meters.is_empty());
    }

    #[test]
    fn every_path_is_delayed_to_match_the_slowest() {
        // Track 1 has a 5 ms limiter and a synth, track 2 a 2 ms limiter,
        // track 3 nothing, and all three play into a master with a 1 ms
        // limiter of its own. Track 3 also sends to track 2.
        let mut project = Project::new("latent");
        for id in 1..=3 {
            project.mixer.tracks.push(track(id, 0));
        }
        project.mixer.tracks[1].effects.push(limiter(21, 5.0));
        project.mixer.tracks[2].effects.push(limiter(22, 2.0));
        project.mixer.tracks[0].effects.push(limiter(20, 1.0));
        project.mixer.tracks[3].sends.push(Send {
            target: TrackId(2),
            gain: 1.0,
        });
        project.channels.push(channel(10, 1, synth()));
        let plan = compile(&project, &SamplePool::new());
        let (state, ledger) = PlanState::build(&plan, RATE, None);

        // The synth is 12 frames behind, so track 1's own voices wait for
        // it, and the track leaves its limiter 12 + 240 frames behind.
        assert_eq!(delay(&state.direct[1]), Some(12));
        let seat = state.instruments[0].as_ref().unwrap();
        assert_eq!(delay(&seat.delay), None);
        // Track 2 is 96 frames behind and track 3 not at all, and the
        // master waits for the slowest of the three.
        let edge =
            |track: usize, edge: usize| &state.edge_delays[plan.tracks[track].edges[edge].slot];
        assert_eq!(delay(edge(1, 0)), None);
        assert_eq!(delay(edge(2, 0)), Some(252 - 96));
        assert_eq!(delay(edge(3, 0)), Some(252));
        // Nothing but the send feeds track 2, so it is not delayed there.
        assert_eq!(delay(edge(3, 1)), None);
        assert_eq!(delay(&state.direct[2]), None);
        assert_eq!(delay(&state.direct[0]), Some(252));
        assert_eq!(state.latency, 252 + 48);
        assert_eq!(ledger.latency, 300);
        assert_eq!(state.shaped, [true; 4]);
        // One meter per limiter, in mixer order.
        let metered: Vec<u32> = ledger.meters.iter().map(|(id, _)| id.0).collect();
        assert_eq!(metered, [20, 21, 22]);
        // Every line is long enough for any look-ahead the limiters can
        // be set to.
        let line = state.direct[0].line.as_ref().unwrap();
        assert!(line.capacity() >= 12 + 960);
    }

    #[test]
    fn only_what_the_audio_thread_lacks_is_built_again() {
        let mut project = Project::new("held");
        project.mixer.tracks.push(track(1, 0));
        project.mixer.tracks[1].effects.push(limiter(21, 5.0));
        project.mixer.tracks.push(track(2, 0));
        project.channels.push(channel(10, 2, synth()));
        let plan = compile(&project, &SamplePool::new());
        let (first, ledger) = PlanState::build(&plan, RATE, None);
        assert!(first.chains[1][0].is_some());
        assert!(first.instruments[0].as_ref().unwrap().unit.is_some());
        assert!(first.edge_delays[1].line.is_some());

        // A longer look-ahead and a second effect: the limiter, the synth
        // and the delay line are all carried over, and only the new effect
        // is built.
        project.mixer.tracks[1].effects[0] = limiter(21, 15.0);
        project.mixer.tracks[1].effects.push(limiter(23, 1.0));
        let plan = compile(&project, &SamplePool::new());
        let (second, ledger) = PlanState::build(&plan, RATE, Some(&ledger));
        assert!(second.chains[1][0].is_none());
        assert!(second.chains[1][1].is_some());
        assert!(second.instruments[0].as_ref().unwrap().unit.is_none());
        let edge = &second.edge_delays[1];
        assert_eq!(
            (delay(edge), edge.line.is_some()),
            (Some(720 + 48 - 12), false)
        );
        assert_eq!(ledger.meters.len(), 2);
        assert_eq!(second.latency, 768);

        // With the limiters gone the synth is the slow one, and the other
        // track is delayed for it. The delay that is no longer needed
        // stays for one more plan, to fade to nothing.
        project.mixer.tracks[1].effects.clear();
        let plan = compile(&project, &SamplePool::new());
        let (third, ledger) = PlanState::build(&plan, RATE, Some(&ledger));
        assert_eq!(third.latency, 12);
        assert_eq!(delay(&third.edge_delays[1]), Some(0));
        assert_eq!(delay(&third.edge_delays[0]), Some(12));
        assert!(ledger.meters.is_empty());
        let (fourth, _) = PlanState::build(&plan, RATE, Some(&ledger));
        assert_eq!(delay(&fourth.edge_delays[1]), None);
        assert_eq!(delay(&fourth.edge_delays[0]), Some(12));
    }
}
