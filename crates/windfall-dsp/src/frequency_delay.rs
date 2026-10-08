//! Sixteen independently delayed stereo frequency bands with complementary
//! cumulative crossovers. See docs/DELAY-FAMILY.md for the exact transfers.
mod params;
use crate::blocks::svf::{OnePoleFilter, cutoff_gain};
use crate::echo_bank::support::{History, Ramps, Tap, audio, balance, bounded, rate};
pub use crate::echo_bank::{
    PreparationBudget, PreparationError, PreparationRefusal, PreparationRequirements,
    PreparationStatus,
};
use crate::{Effect, ParamSet};
pub use params::*;

pub const FREQUENCY_BANDS: usize = 16;
/// Nominal crossover before bandwidth scaling and the sample-rate ceiling.
pub fn nominal_crossover_hz(index: usize) -> Option<f32> {
    (index < 15).then(|| 20.0 * 1000.0_f32.powf((index + 1) as f32 / 16.0))
}
struct Band {
    lines: [History; 2],
    taps: [Tap; 2],
    controls: Ramps<3>,
    last: [f32; 2],
}
impl Default for Band {
    fn default() -> Self {
        Self {
            lines: std::array::from_fn(|_| History::default()),
            taps: [Tap::default(); 2],
            controls: Ramps::default(),
            last: [0.0; 2],
        }
    }
}
/// A real sixteen-band delay, unrelated to the ordinary stereo Delay.
pub struct FrequencyDelay {
    params: FrequencyDelayParams,
    bands: [Band; FREQUENCY_BANDS],
    filters: [[[OnePoleFilter; 2]; 2]; 15],
    coefficients: Ramps<15>,
    controls: Ramps<4>,
    rate: f32,
    prepared: bool,
    horizon: usize,
    clock: u32,
    refusal: Option<PreparationRefusal>,
}
impl Default for FrequencyDelay {
    fn default() -> Self {
        Self {
            params: FrequencyDelayParams::default(),
            bands: std::array::from_fn(|_| Band::default()),
            filters: [[[OnePoleFilter::default(); 2]; 2]; 15],
            coefficients: Ramps::default(),
            controls: Ramps::default(),
            rate: 48_000.0,
            prepared: false,
            horizon: 0,
            clock: 0,
            refusal: None,
        }
    }
}
impl FrequencyDelay {
    /// Final payload estimate. Use checked requirements and `try_prepare` to
    /// admit peak storage and handle allocation refusal off the audio thread.
    pub fn preparation_bytes(sample_rate: f32) -> usize {
        PreparationRequirements::checked::<32>(
            rate(sample_rate),
            rate(sample_rate).ceil() as usize,
            size_of::<Self>(),
            0,
        )
        .map_or(usize::MAX, |r| r.retained_bytes)
    }
    /// Includes the old histories and the fixed staging headers in peak bytes.
    /// Read-only and allocation-free; rate sanitation matches legacy prepare.
    pub fn preparation_requirements(
        &self,
        sample_rate: f32,
    ) -> Result<PreparationRequirements, PreparationError> {
        PreparationRequirements::checked::<32>(
            rate(sample_rate),
            rate(sample_rate).ceil() as usize,
            size_of::<Self>(),
            self.prepared_bytes() - size_of::<Self>(),
        )
    }
    /// Control-thread only. Every history is reserved before publishing any
    /// new clock/control/filter state. Refusal preserves the prior audio state.
    /// `max_block` is accepted for Effect compatibility; work is frame-based.
    pub fn try_prepare(
        &mut self,
        sample_rate: f32,
        _max_block: usize,
        budget: PreparationBudget,
    ) -> Result<(), PreparationError> {
        let sample_rate = rate(sample_rate);
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
        let mut staged = crate::echo_bank::support::stage_histories::<32>(
            sample_rate,
            sample_rate.ceil() as usize,
            size_of::<Self>(),
            self.prepared_bytes() - size_of::<Self>(),
            budget,
        )?;
        for (live, replacement) in self
            .bands
            .iter_mut()
            .flat_map(|b| &mut b.lines)
            .zip(&mut staged)
        {
            std::mem::swap(live, replacement);
        }
        self.rate = sample_rate;
        for band in &mut self.bands {
            band.controls.prepare(self.rate);
        }
        self.coefficients.prepare(self.rate);
        self.controls.prepare(self.rate);
        self.prepared = true;
        // Both one-pole banks, including all bandwidth/sample-rate extremes.
        let min_c = OnePoleFilter::coefficient(0.0, self.rate) as f64;
        let max_c = OnePoleFilter::coefficient(self.rate * 0.49, self.rate) as f64;
        let radius = (1.0 - 2.0 * min_c).abs().max((1.0 - 2.0 * max_c).abs());
        self.horizon = (128.0 / -radius.ln()).ceil() as usize
            + self.rate.ceil() as usize
            + (self.rate * 0.1).ceil() as usize
            + 64;
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
                .bands
                .iter()
                .flat_map(|b| &b.lines)
                .map(History::bytes)
                .sum::<usize>()
    }
    /// The complete sanitized target; ramps may still be on their way.
    pub fn params(&self) -> FrequencyDelayParams {
        self.params
    }
    fn apply(&mut self) {
        let p = self.params;
        self.controls.set([
            p.dry,
            p.wet,
            p.feedback,
            if p.split == FrequencySplit::Steep {
                1.0
            } else {
                0.0
            },
        ]);
        self.coefficients.set(std::array::from_fn(|i| {
            let center = (20.0_f32 * 20_000.0).sqrt();
            let frequency = center * (nominal_crossover_hz(i).unwrap() / center).powf(p.bandwidth);
            let g = cutoff_gain(frequency, self.rate);
            g / (1.0 + g)
        }));
        for (band, b) in self.bands.iter_mut().zip(p.bands) {
            band.controls
                .set([b.level, b.pan, if b.enabled { 1.0 } else { 0.0 }]);
            let ms = if p.scale >= 0.0 {
                p.scale * b.delay_ms
            } else {
                -p.scale * (1000.0 - b.delay_ms)
            };
            let ms = ms * if p.short_range { 0.1 } else { 1.0 };
            let delay = (f64::from(ms) * f64::from(self.rate) / 1000.0).round() as usize;
            for tap in &mut band.taps {
                tap.set(
                    delay,
                    (self.rate * 0.03).round().max(1.0) as u32,
                    band.controls.fresh,
                );
            }
        }
    }
    fn split(&mut self, input: [f32; 2], coefficients: [f32; 15], steep: f32) -> [[f32; 2]; 16] {
        let mut bands = [[0.0; 2]; 16];
        let mut previous = [0.0; 2];
        for i in 0..15 {
            let mut low = [0.0; 2];
            for side in 0..2 {
                let gentle = self.filters[i][0][side].low_pass(coefficients[i], input[side]);
                let narrow = self.filters[i][1][side].low_pass(coefficients[i], gentle);
                low[side] = bounded(gentle + (narrow - gentle) * steep);
                bands[i][side] = low[side] - previous[side];
                if !gentle.is_finite() || !narrow.is_finite() {
                    self.filters[i][0][side].reset();
                    self.filters[i][1][side].reset();
                }
            }
            previous = low;
        }
        bands[15] = [input[0] - previous[0], input[1] - previous[1]];
        bands
    }
}
impl Effect for FrequencyDelay {
    type Params = FrequencyDelayParams;
    fn prepare(&mut self, sample_rate: f32, max_block: usize) {
        let _ = self.try_prepare(sample_rate, max_block, PreparationBudget::UNLIMITED);
    }
    fn reset(&mut self) {
        self.filters = [[[OnePoleFilter::default(); 2]; 2]; 15];
        self.clock = 0;
        for band in &mut self.bands {
            for line in &mut band.lines {
                line.reset();
            }
            band.last = [0.0; 2];
            band.controls.reset();
        }
        self.coefficients.reset();
        self.controls.reset();
        self.apply();
    }
    fn set_params(&mut self, params: &Self::Params) {
        let p = params.sanitized();
        if p != self.params || self.controls.fresh {
            self.params = p;
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
            let coefficients = self.coefficients.tick();
            let input = self.split(dry, coefficients, c[3]);
            let mut out = [0.0; 2];
            for (band, x) in self.bands.iter_mut().zip(input) {
                let controls = band.controls.tick();
                let mut delayed = [0.0; 2];
                for side in 0..2 {
                    // Zero time's feedback is causal one-sample memory.
                    let current = bounded(x[side] + c[2] * band.last[side]);
                    delayed[side] = band.taps[side].read(&band.lines[side], current);
                    band.lines[side].push(bounded(x[side] + c[2] * delayed[side]));
                    band.last[side] = delayed[side];
                }
                let processed = balance(delayed, controls[1]).map(|y| y * controls[0]);
                for side in 0..2 {
                    out[side] += x[side] + (processed[side] - x[side]) * controls[2];
                }
            }
            *l = dry[0] * c[0] + out[0] * c[1];
            *r = dry[1] * c[0] + out[1] * c[1];
            self.clock = (self.clock + 1) % 64;
            if self.clock == 0 {
                for bank in &mut self.filters {
                    for stages in bank {
                        for filter in stages {
                            filter.flush();
                        }
                    }
                }
            }
        }
    }
    fn warm_up_samples(&self) -> usize {
        self.bands
            .iter()
            .filter(|band| band.controls.max(2) > 0.0 && band.controls.max(0) > 0.0)
            .flat_map(|b| b.taps)
            .map(|t| t.longest())
            .max()
            .unwrap_or(0)
    }
    fn delay_readiness_samples(&self) -> usize {
        self.warm_up_samples()
    }
    fn latency_transition_samples_remaining(&self) -> usize {
        self.bands
            .iter()
            .flat_map(|b| b.taps.iter().zip(&b.lines))
            .map(|(t, l)| t.transition(l.valid()))
            .max()
            .unwrap_or(0)
    }
    fn tail_samples(&self) -> usize {
        if self.controls.max(2) > 0.0 {
            usize::MAX
        } else {
            self.horizon
        }
    }
    fn gap_samples(&self) -> usize {
        self.horizon
    }
}
