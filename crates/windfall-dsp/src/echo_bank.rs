//! Eight independently controlled, filtered, feed-forward linked stereo echoes.
//! The source/time/headroom contract is in docs/DELAY-FAMILY.md.
mod params;
mod preparation;
pub(crate) mod support;
use crate::{Effect, ParamSet, SelectableFilter, SelectableFilterMode, SelectableFilterParams};
pub use params::*;
pub use preparation::{
    PreparationBudget, PreparationError, PreparationRefusal, PreparationRequirements,
    PreparationStatus,
};
use support::{History, Ramps, Tap, audio, balance, bounded};

/// Number of units; persisted unit indices are always 0..8.
pub const ECHO_UNITS: usize = 8;
/// Covers whole notes at 10 BPM plus the full channel offset.
pub const ECHO_MAX_SECONDS: f32 = 25.0;

struct Filter {
    stages: [SelectableFilter; 3],
    controls: Ramps<5>,
    selection: usize,
}
impl Default for Filter {
    fn default() -> Self {
        Self {
            stages: std::array::from_fn(|_| SelectableFilter::default()),
            controls: Ramps::default(),
            selection: 0,
        }
    }
}
impl Filter {
    fn prepare(&mut self, rate: f32) {
        for stage in &mut self.stages {
            stage.prepare(rate, 1);
        }
        self.controls.prepare(rate);
    }
    fn reset(&mut self) {
        for stage in &mut self.stages {
            stage.reset();
        }
        self.controls.reset();
    }
    fn set(&mut self, p: EchoFilterParams) {
        let mode = match p.mode {
            EchoFilterMode::Off | EchoFilterMode::Lowpass => SelectableFilterMode::Lowpass,
            EchoFilterMode::Bandpass => SelectableFilterMode::Bandpass,
            EchoFilterMode::Notch => SelectableFilterMode::Notch,
            EchoFilterMode::Highpass => SelectableFilterMode::Highpass,
            EchoFilterMode::LowShelf => SelectableFilterMode::LowShelf,
            EchoFilterMode::Peak => SelectableFilterMode::Peak,
            EchoFilterMode::HighShelf => SelectableFilterMode::HighShelf,
        };
        let selected = SelectableFilterParams {
            mode,
            frequency_hz: p.frequency_hz,
            q: p.q,
            gain_db: p.gain_db,
        };
        for stage in &mut self.stages {
            stage.set_params(&selected);
        }
        let mut weights = [0.0; 5];
        weights[0] = p.gain;
        let selection = if p.mode == EchoFilterMode::Off {
            0
        } else {
            p.sections as usize
        };
        weights[1 + selection] = 1.0;
        // Restart all mode/order weights together, retaining their current
        // mixture. Independent unchanged zero ramps otherwise arrive early.
        if selection != self.selection {
            self.controls.restart_group(1..5);
            self.selection = selection;
        }
        self.controls.set(weights);
    }
    fn tick(&mut self, input: [f32; 2]) -> [f32; 2] {
        let weights = self.controls.tick();
        let mut out = input.map(|x| x * weights[1]);
        let mut value = input;
        for (i, stage) in self.stages.iter_mut().enumerate() {
            let (mut l, mut r) = ([value[0]], [value[1]]);
            stage.process(&mut l, &mut r);
            if !l[0].is_finite() || !r[0].is_finite() {
                stage.reset();
            }
            value = [bounded(l[0]), bounded(r[0])];
            out[0] += value[0] * weights[i + 2];
            out[1] += value[1] * weights[i + 2];
        }
        out.map(|x| bounded(x * weights[0]))
    }
    fn tail(&self) -> usize {
        self.stages[0].tail_samples().saturating_mul(3)
    }
}
struct Unit {
    lines: [History; 2],
    taps: [Tap; 2],
    pre: Filter,
    post: Filter,
    feedback_filter: Filter,
    controls: Ramps<14>,
}
impl Default for Unit {
    fn default() -> Self {
        Self {
            lines: std::array::from_fn(|_| History::default()),
            taps: [Tap::default(); 2],
            pre: Filter::default(),
            post: Filter::default(),
            feedback_filter: Filter::default(),
            controls: Ramps::default(),
        }
    }
}
impl Unit {
    fn prepare(&mut self, rate: f32) {
        for filter in [&mut self.pre, &mut self.post, &mut self.feedback_filter] {
            filter.prepare(rate);
        }
        self.controls.prepare(rate);
    }
    fn reset(&mut self) {
        for line in &mut self.lines {
            line.reset();
        }
        for filter in [&mut self.pre, &mut self.post, &mut self.feedback_filter] {
            filter.reset();
        }
        self.controls.reset();
    }
    fn set(&mut self, p: EchoUnitParams, rate: f32, tempo: f32) {
        self.pre.set(p.filter);
        self.post.set(p.filter);
        self.feedback_filter.set(p.feedback_filter);
        let on = if p.enabled { 1.0 } else { 0.0 };
        let normal = if p.feedback_mode == EchoFeedbackMode::Normal {
            1.0
        } else {
            0.0
        };
        let inverted = if p.feedback_mode == EchoFeedbackMode::Inverted {
            1.0
        } else {
            0.0
        };
        let ping = if p.feedback_mode == EchoFeedbackMode::PingPong {
            1.0
        } else {
            0.0
        };
        self.controls.set([
            on,
            p.input_gain,
            p.input_pan,
            p.feedback,
            p.feedback_pan,
            p.separation,
            p.output_gain,
            p.output_pan,
            p.next_send,
            if p.filter_post { 1.0 } else { 0.0 },
            normal,
            inverted,
            ping,
            if p.feedback_mode == EchoFeedbackMode::Off {
                0.0
            } else {
                1.0
            },
        ]);
        let time = if p.sync {
            p.division.beats() * 60_000.0 / tempo
        } else {
            p.time_ms
        };
        for (side, tap) in self.taps.iter_mut().enumerate() {
            let offset = if side == 0 {
                (-p.stereo_offset_ms).max(0.0)
            } else {
                p.stereo_offset_ms.max(0.0)
            };
            let delay =
                ((f64::from(time + offset) * f64::from(rate) / 1000.0).round() as usize).max(1);
            tap.set(
                delay,
                (rate * 0.03).round().max(1.0) as u32,
                self.controls.fresh,
            );
        }
    }
    fn tick(
        &mut self,
        dry: [f32; 2],
        previous: [f32; 2],
        global_input: f32,
        global_fb: f32,
    ) -> ([f32; 2], [f32; 2]) {
        let c = self.controls.tick();
        let input = balance(dry, c[2]).map(|x| x * c[1] * global_input * c[0]);
        let input = [
            bounded(input[0] + previous[0]),
            bounded(input[1] + previous[1]),
        ];
        // Inverted flips injection once; ping-pong preserves injection and
        // flips every loop traversal. The two modes are observably distinct.
        let injection = [
            input[0] * (1.0 - c[11]) + input[1] * c[11],
            input[1] * (1.0 - c[11]) + input[0] * c[11],
        ];
        let pre = self.pre.tick(injection);
        let echo = [
            self.taps[0].read(&self.lines[0], 0.0),
            self.taps[1].read(&self.lines[1], 0.0),
        ];
        let post = self.post.tick(echo);
        let filtered_back = self.feedback_filter.tick(echo);
        let back = balance(
            [
                filtered_back[0] * (1.0 - c[12]) + filtered_back[1] * c[12],
                filtered_back[1] * (1.0 - c[12]) + filtered_back[0] * c[12],
            ],
            c[4],
        );
        for side in 0..2 {
            let into = pre[side] + (injection[side] - pre[side]) * c[9];
            self.lines[side].push(bounded(into + back[side] * c[3] * global_fb * c[13]));
        }
        let output = [
            echo[0] + (post[0] - echo[0]) * c[9],
            echo[1] + (post[1] - echo[1]) * c[9],
        ];
        let mid = (output[0] + output[1]) * 0.5;
        let side = (output[0] - output[1]) * 0.5 * c[5];
        let output = balance([mid + side, mid - side], c[7]).map(|x| bounded(x) * c[0]);
        (output.map(|x| x * c[6]), output.map(|x| x * c[8]))
    }
    fn longest(&self) -> usize {
        self.taps[0].longest().max(self.taps[1].longest())
    }
}
/// Real eight-unit chain/parallel delay; settings contain no runtime owners.
pub struct EchoBank {
    params: EchoBankParams,
    units: [Unit; ECHO_UNITS],
    controls: Ramps<4>,
    rate: f32,
    tempo: f32,
    prepared: bool,
    horizon: usize,
    refusal: Option<PreparationRefusal>,
}
impl Default for EchoBank {
    fn default() -> Self {
        Self {
            params: EchoBankParams::default(),
            units: std::array::from_fn(|_| Unit::default()),
            controls: Ramps::default(),
            rate: 48_000.0,
            tempo: 120.0,
            prepared: false,
            horizon: 0,
            refusal: None,
        }
    }
}
impl EchoBank {
    /// Final payload estimate, including inline state. Use the checked
    /// requirements and `try_prepare` to admit peak storage and handle refusal.
    pub fn preparation_bytes(sample_rate: f32) -> usize {
        PreparationRequirements::checked::<16>(
            support::rate(sample_rate),
            Self::maximum(sample_rate),
            size_of::<Self>(),
            0,
        )
        .map_or(usize::MAX, |r| r.retained_bytes)
    }
    fn maximum(sample_rate: f32) -> usize {
        (f64::from(ECHO_MAX_SECONDS) * f64::from(support::rate(sample_rate))).ceil() as usize
    }
    /// Checked requirements for replacing this instance, retaining its old
    /// histories until ALL reservations succeed. Read-only; no allocation.
    pub fn preparation_requirements(
        &self,
        sample_rate: f32,
    ) -> Result<PreparationRequirements, PreparationError> {
        PreparationRequirements::checked::<16>(
            support::rate(sample_rate),
            Self::maximum(sample_rate),
            size_of::<Self>(),
            self.prepared_bytes() - size_of::<Self>(),
        )
    }
    /// Control-thread only. A refusal changes only the observable status;
    /// clock, params, ramps, filters, taps and audio histories remain intact.
    /// `max_block` is accepted for Effect compatibility; work is frame-based.
    pub fn try_prepare(
        &mut self,
        sample_rate: f32,
        _max_block: usize,
        budget: PreparationBudget,
    ) -> Result<(), PreparationError> {
        let sample_rate = support::rate(sample_rate);
        let result = self.prepare_replacement(sample_rate, budget);
        self.refusal = result
            .err()
            .map(|error| PreparationRefusal { sample_rate, error });
        result
    }
    pub fn preparation_status(&self) -> PreparationStatus {
        PreparationStatus {
            is_prepared: self.prepared,
            last_refusal: self.refusal,
        }
    }
    fn prepare_replacement(
        &mut self,
        sample_rate: f32,
        budget: PreparationBudget,
    ) -> Result<(), PreparationError> {
        let mut staged = support::stage_histories::<16>(
            sample_rate,
            Self::maximum(sample_rate),
            size_of::<Self>(),
            self.prepared_bytes() - size_of::<Self>(),
            budget,
        )?;
        // No live mutation above. All remaining operations are infallible,
        // fixed inline work. Old histories retire with staged on this thread.
        for (live, replacement) in self
            .units
            .iter_mut()
            .flat_map(|u| &mut u.lines)
            .zip(&mut staged)
        {
            std::mem::swap(live, replacement);
        }
        self.rate = sample_rate;
        for unit in &mut self.units {
            unit.prepare(self.rate);
        }
        self.controls.prepare(self.rate);
        self.prepared = true;
        self.horizon = self
            .units
            .iter()
            .map(|u| {
                (ECHO_MAX_SECONDS * self.rate) as usize
                    + u.pre.tail()
                    + u.post.tail()
                    + u.feedback_filter.tail()
            })
            .sum::<usize>()
            + (self.rate * 0.1) as usize;
        self.reset();
        Ok(())
    }
    /// Rate actually used after finite-range sanitation.
    pub fn actual_sample_rate(&self) -> f32 {
        self.rate
    }
    /// All retained payload, including inline state and history capacities.
    pub fn prepared_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self
                .units
                .iter()
                .flat_map(|u| &u.lines)
                .map(History::bytes)
                .sum::<usize>()
    }
    /// The complete sanitized target; ramps may still be on their way.
    pub fn params(&self) -> EchoBankParams {
        self.params
    }
    fn apply(&mut self) {
        self.controls.set([
            self.params.dry,
            self.params.wet,
            self.params.input_gain,
            self.params.feedback,
        ]);
        for (unit, p) in self.units.iter_mut().zip(self.params.units) {
            unit.set(p, self.rate, self.tempo);
        }
    }
}
impl Effect for EchoBank {
    type Params = EchoBankParams;
    fn prepare(&mut self, sample_rate: f32, max_block: usize) {
        let _ = self.try_prepare(sample_rate, max_block, PreparationBudget::UNLIMITED);
    }
    fn reset(&mut self) {
        for unit in &mut self.units {
            unit.reset();
        }
        self.controls.reset();
        self.apply();
    }
    fn set_params(&mut self, params: &Self::Params) {
        let clean = params.sanitized();
        if clean != self.params || self.controls.fresh {
            self.params = clean;
            self.apply();
        }
    }
    fn set_tempo(&mut self, bpm: f32) {
        let bpm = crate::blocks::math::clean(bpm, 10.0, 1000.0, 120.0);
        if bpm != self.tempo {
            self.tempo = bpm;
            self.apply();
        }
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        if !self.prepared {
            for x in left.iter_mut().chain(right) {
                *x = audio(*x);
            }
            return;
        }
        for (l, r) in left.iter_mut().zip(right) {
            let dry = [audio(*l), audio(*r)];
            let c = self.controls.tick();
            let mut send = [0.0; 2];
            let mut out = [0.0; 2];
            for unit in &mut self.units {
                let (wet, next) = unit.tick(dry, send, c[2], c[3]);
                send = next;
                out[0] += wet[0];
                out[1] += wet[1];
            }
            *l = dry[0] * c[0] + out[0] * c[1];
            *r = dry[1] * c[0] + out[1] * c[1];
        }
    }
    fn warm_up_samples(&self) -> usize {
        let mut linked = None;
        let mut longest = 0;
        for unit in &self.units {
            let enabled = unit.controls.max(0) > 0.0;
            let injected = enabled && unit.controls.max(1) > 0.0 && self.controls.max(2) > 0.0;
            let input_path = if injected {
                Some(linked.unwrap_or(0))
            } else {
                linked
            };
            let path = input_path.map(|n: usize| n.saturating_add(unit.longest()));
            if enabled && unit.controls.abs_max(6) > 0.0 {
                longest = longest.max(path.unwrap_or(0));
            }
            linked = if enabled && unit.controls.abs_max(8) > 0.0 {
                path
            } else {
                None
            };
        }
        longest
    }
    fn delay_readiness_samples(&self) -> usize {
        self.warm_up_samples()
    }
    fn latency_transition_samples_remaining(&self) -> usize {
        self.units
            .iter()
            .map(|u| {
                u.taps
                    .iter()
                    .zip(&u.lines)
                    .map(|(t, l)| t.transition(l.valid()))
                    .max()
                    .unwrap_or(0)
            })
            .max()
            .unwrap_or(0)
    }
    fn tail_samples(&self) -> usize {
        if self.controls.max(3) > 0.0
            && self
                .units
                .iter()
                .any(|u| u.controls.max(3) > 0.0 && u.controls.max(13) > 0.0)
        {
            usize::MAX
        } else {
            self.horizon
        }
    }
    fn gap_samples(&self) -> usize {
        self.horizon
    }
}
