//! Three independent filter processors. See `docs/FILTER-FAMILY.md` for
//! transfer functions, transition behavior and the pending host integration.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::blocks::math::{clean, ms_to_samples};
use crate::blocks::smooth::LinearRamp;
use crate::blocks::svf::{OnePoleFilter, Svf, SvfCoeffs, cutoff_gain};
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};

/// Every live edit reaches its target after this many milliseconds of audio.
pub const FILTER_SMOOTHING_MS: f32 = 5.0;
const DEFAULT_Q: f32 = std::f32::consts::FRAC_1_SQRT_2;
const MAX_Q: f32 = 10.0;
const FLUSH_PERIOD: u32 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct FastLowpassParams {
    /// Cutoff in Hz, 20..=20000; default 20000. Limited to 0.49 * rate.
    pub cutoff_hz: f32,
    /// Q, 0.5..=10; default 1/sqrt(2). Gain at the cutoff equals Q.
    pub q: f32,
}

impl Default for FastLowpassParams {
    fn default() -> Self {
        Self {
            cutoff_hz: 20_000.0,
            q: DEFAULT_Q,
        }
    }
}

param_set!(FastLowpassParams, "Fast lowpass", {
    float [cutoff_hz] "cutoffHz" "Cutoff" { Hertz, Logarithmic, 20.0, 20_000.0, 20_000.0 }
    float [q] "q" "Resonance" { Ratio, Logarithmic, 0.5, MAX_Q, DEFAULT_Q }
});

/// The order is the stable automation/descriptor order, independent of Rust
/// enum discriminants. Gain affects only the last three modes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum SelectableFilterMode {
    #[default]
    Lowpass,
    Highpass,
    Bandpass,
    Notch,
    LowShelf,
    Peak,
    HighShelf,
}

impl SelectableFilterMode {
    pub const ALL: [Self; 7] = [
        Self::Lowpass,
        Self::Highpass,
        Self::Bandpass,
        Self::Notch,
        Self::LowShelf,
        Self::Peak,
        Self::HighShelf,
    ];

    fn index(self) -> usize {
        match self {
            Self::Lowpass => 0,
            Self::Highpass => 1,
            Self::Bandpass => 2,
            Self::Notch => 3,
            Self::LowShelf => 4,
            Self::Peak => 5,
            Self::HighShelf => 6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct SelectableFilterParams {
    /// Response type; default lowpass.
    pub mode: SelectableFilterMode,
    /// Cutoff, center or shelf midpoint, Hz, 20..=20000; default 1000.
    pub frequency_hz: f32,
    /// Q, 0.5..=10; default 1/sqrt(2). Bandpass peak is unity at every Q.
    pub q: f32,
    /// Shelf/peak gain in dB, -18..=18; default 0. Ignored by cut/notch modes.
    pub gain_db: f32,
}

impl Default for SelectableFilterParams {
    fn default() -> Self {
        Self {
            mode: SelectableFilterMode::Lowpass,
            frequency_hz: 1_000.0,
            q: DEFAULT_Q,
            gain_db: 0.0,
        }
    }
}

param_set!(SelectableFilterParams, "Selectable filter", {
    choice [mode] "mode" "Mode" { SelectableFilterMode, Lowpass, [
        Lowpass "lowpass" "Lowpass", Highpass "highpass" "Highpass",
        Bandpass "bandpass" "Bandpass", Notch "notch" "Notch",
        LowShelf "lowShelf" "Low shelf", Peak "peak" "Peak",
        HighShelf "highShelf" "High shelf"
    ] }
    float [frequency_hz] "frequencyHz" "Frequency" { Hertz, Logarithmic, 20.0, 20_000.0, 1_000.0 }
    float [q] "q" "Resonance" { Ratio, Logarithmic, 0.5, MAX_Q, DEFAULT_Q }
    float [gain_db] "gainDb" "Gain" { Decibels, Linear, -18.0, 18.0, 0.0 }
});

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct BassShelfParams {
    /// Shelf midpoint in Hz, 40..=1000; default 150. Limited to 0.49 * rate.
    pub frequency_hz: f32,
    /// Low-frequency boost in dB, 0..=18; default 0 (exact unity).
    /// Reserve this much steady-state headroom upstream; no limiter is added.
    pub gain_db: f32,
}

impl Default for BassShelfParams {
    fn default() -> Self {
        Self {
            frequency_hz: 150.0,
            gain_db: 0.0,
        }
    }
}

param_set!(BassShelfParams, "Bass shelf", {
    float [frequency_hz] "frequencyHz" "Frequency" { Hertz, Logarithmic, 40.0, 1_000.0, 150.0 }
    float [gain_db] "gainDb" "Boost" { Decibels, Linear, 0.0, 18.0, 0.0 }
});

fn sample_rate(value: f32) -> f32 {
    clean(value, 1.0, 384_000.0, 48_000.0)
}

/// Signal-space ramps. Repeated target writes do not extend a ramp. Only
/// actual audio frames consume the fresh state or advance the flush clock.
struct Controls<const N: usize> {
    ramps: [LinearRamp; N],
    samples: u32,
    fresh: bool,
    clock: u32,
}

impl<const N: usize> Controls<N> {
    fn new() -> Self {
        Self {
            ramps: [LinearRamp::new(0.0); N],
            samples: 240,
            fresh: true,
            clock: 0,
        }
    }

    fn prepare(&mut self, rate: f32) {
        self.samples = ms_to_samples(FILTER_SMOOTHING_MS, rate);
        self.reset();
    }

    fn reset(&mut self) {
        self.fresh = true;
        self.clock = 0;
    }

    fn set(&mut self, values: [f32; N]) {
        for (ramp, value) in self.ramps.iter_mut().zip(values) {
            ramp.set_target(value, if self.fresh { 0 } else { self.samples });
        }
    }

    // A vector crossfade must give ALL its weights the same arrival time,
    // even those whose target remains zero. Call only for a new mode, so
    // duplicate writes and unrelated parameter edits keep their timelines.
    fn restart_group(&mut self, from: usize) {
        for ramp in &mut self.ramps[from..] {
            ramp.snap(ramp.value());
        }
    }

    fn tick(&mut self) -> [f32; N] {
        self.fresh = false;
        self.clock = (self.clock + 1) % FLUSH_PERIOD;
        std::array::from_fn(|i| self.ramps[i].tick())
    }

    fn flush_due(&self) -> bool {
        self.clock == 0
    }
}

/// A single trapezoidal SVF per channel: 12 dB/octave lowpass, no oversampling,
/// no delay buffers and no per-frame transcendental coefficient calculation.
pub struct FastLowpass {
    params: FastLowpassParams,
    rate: f32,
    controls: Controls<2>,
    filters: [Svf; 2],
    tail: usize,
}

impl Default for FastLowpass {
    fn default() -> Self {
        let mut filter = Self {
            params: FastLowpassParams::default(),
            rate: 48_000.0,
            controls: Controls::new(),
            filters: [Svf::default(); 2],
            tail: 0,
        };
        filter.prepare(48_000.0, 0);
        filter
    }
}

impl FastLowpass {
    fn update(&mut self) {
        self.controls.set([
            cutoff_gain(self.params.cutoff_hz, self.rate),
            1.0 / self.params.q,
        ]);
    }
}

impl Effect for FastLowpass {
    type Params = FastLowpassParams;

    fn prepare(&mut self, rate: f32, _max_block: usize) {
        self.rate = sample_rate(rate);
        self.controls.prepare(self.rate);
        self.tail = svf_tail(self.rate, 1.0, 1.0 / MAX_Q, 2.0);
        self.reset();
    }

    fn reset(&mut self) {
        self.filters = [Svf::default(); 2];
        self.controls.reset();
        self.update();
    }

    fn set_params(&mut self, params: &Self::Params) {
        let params = params.sanitized();
        if params != self.params {
            self.params = params;
            self.update();
        }
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let [g, k] = self.controls.tick();
            let coeffs = SvfCoeffs::from_gain(g, k);
            *l = self.filters[0].tick(&coeffs, *l).low;
            *r = self.filters[1].tick(&coeffs, *r).low;
            if self.controls.flush_due() {
                for filter in &mut self.filters {
                    filter.flush();
                }
            }
        }
    }

    fn tail_samples(&self) -> usize {
        self.tail
    }
    fn gap_samples(&self) -> usize {
        self.tail
    }
}

/// Four continuously running SVF pairs produce seven responses. Shelves
/// and peak use their own tuned integrators; no mode wakes stale history.
pub struct SelectableFilter {
    params: SelectableFilterParams,
    rate: f32,
    controls: Controls<19>,
    filters: [[Svf; 2]; 4],
    tail: usize,
}

impl Default for SelectableFilter {
    fn default() -> Self {
        let mut filter = Self {
            params: SelectableFilterParams::default(),
            rate: 48_000.0,
            controls: Controls::new(),
            filters: [[Svf::default(); 2]; 4],
            tail: 0,
        };
        filter.prepare(48_000.0, 0);
        filter
    }
}

impl SelectableFilter {
    fn update(&mut self) {
        let g = cutoff_gain(self.params.frequency_hz, self.rate);
        let k = 1.0 / self.params.q;
        let a = 10.0_f32.powf(self.params.gain_db / 40.0);
        let mut values = [0.0; 19];
        // Main g/k; low-shelf g/k/m1/m2; peak k/m1 (shared g);
        // high-shelf g/m0/m1/m2 (shared k).
        values[..12].copy_from_slice(&[
            g,
            k,
            g / a.sqrt(),
            k,
            k * (a - 1.0),
            a * a - 1.0,
            k / a,
            (k / a) * (a * a - 1.0),
            g * a.sqrt(),
            a * a,
            k * (1.0 - a) * a,
            1.0 - a * a,
        ]);
        values[12 + self.params.mode.index()] = 1.0;
        self.controls.set(values);
    }
}

impl Effect for SelectableFilter {
    type Params = SelectableFilterParams;

    fn prepare(&mut self, rate: f32, _max_block: usize) {
        self.rate = sample_rate(rate);
        self.controls.prepare(self.rate);
        // Includes every inactive bank and all gain/Q extremes. Keep the
        // shelf and peak bounds separate: their different damping ranges
        // cannot coincide in the same bank, even during automation.
        let a = 10.0_f32.powf(18.0 / 40.0);
        self.tail = svf_tail(self.rate, a.sqrt(), 1.0 / MAX_Q, 2.0).max(svf_tail(
            self.rate,
            1.0,
            1.0 / (MAX_Q * a),
            2.0 * a,
        ));
        self.reset();
    }

    fn reset(&mut self) {
        self.filters = [[Svf::default(); 2]; 4];
        self.controls.reset();
        self.update();
    }

    fn set_params(&mut self, params: &Self::Params) {
        let params = params.sanitized();
        if params != self.params {
            if params.mode != self.params.mode {
                self.controls.restart_group(12);
            }
            self.params = params;
            self.update();
        }
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let v = self.controls.tick();
            let coeffs = [
                SvfCoeffs::from_gain(v[0], v[1]),
                SvfCoeffs::from_gain(v[2], v[3]),
                SvfCoeffs::from_gain(v[0], v[6]),
                SvfCoeffs::from_gain(v[8], v[1]),
            ];
            for (side, sample) in [l, r].into_iter().enumerate() {
                let input = *sample;
                let out: [_; 4] =
                    std::array::from_fn(|i| self.filters[i][side].tick(&coeffs[i], input));
                let responses = [
                    out[0].low,
                    out[0].high,
                    v[1] * out[0].band,
                    out[0].low + out[0].high,
                    input + v[4] * out[1].band + v[5] * out[1].low,
                    input + v[7] * out[2].band,
                    v[9] * input + v[10] * out[3].band + v[11] * out[3].low,
                ];
                *sample = responses
                    .iter()
                    .zip(&v[12..])
                    .map(|(response, weight)| response * weight)
                    .sum();
            }
            if self.controls.flush_due() {
                for bank in &mut self.filters {
                    for filter in bank {
                        filter.flush();
                    }
                }
            }
        }
    }

    fn tail_samples(&self) -> usize {
        self.tail
    }
    fn gap_samples(&self) -> usize {
        self.tail
    }
}

/// A monotonic first-order low shelf. Its two controls do not alias an EQ
/// preset. The lowpass integrator stays live even while boost is zero.
pub struct BassShelf {
    params: BassShelfParams,
    rate: f32,
    controls: Controls<2>,
    filters: [OnePoleFilter; 2],
    tail: usize,
}

impl Default for BassShelf {
    fn default() -> Self {
        let mut filter = Self {
            params: BassShelfParams::default(),
            rate: 48_000.0,
            controls: Controls::new(),
            filters: [OnePoleFilter::default(); 2],
            tail: 0,
        };
        filter.prepare(48_000.0, 0);
        filter
    }
}

impl BassShelf {
    fn update(&mut self) {
        let boost = 10.0_f32.powf(self.params.gain_db / 20.0);
        let g = cutoff_gain(self.params.frequency_hz, self.rate) / boost.sqrt();
        self.controls.set([g / (1.0 + g), boost - 1.0]);
    }
}

impl Effect for BassShelf {
    type Params = BassShelfParams;

    fn prepare(&mut self, rate: f32, _max_block: usize) {
        self.rate = sample_rate(rate);
        self.controls.prepare(self.rate);
        let boost = 10.0_f64.powf(18.0 / 20.0);
        let min_g = f64::from(cutoff_gain(40.0, self.rate)) / boost.sqrt();
        let max_g = f64::from(cutoff_gain(1_000.0, self.rate));
        let radius = [min_g, max_g]
            .map(|g| ((1.0 - g) / (1.0 + g)).abs())
            .into_iter()
            .fold(0.0_f64, f64::max);
        self.tail = decay_samples(radius, self.rate);
        self.reset();
    }

    fn reset(&mut self) {
        self.filters = [OnePoleFilter::default(); 2];
        self.controls.reset();
        self.update();
    }

    fn set_params(&mut self, params: &Self::Params) {
        let params = params.sanitized();
        if params != self.params {
            self.params = params;
            self.update();
        }
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let [coefficient, amount] = self.controls.tick();
            for (filter, sample) in self.filters.iter_mut().zip([l, r]) {
                let input = *sample;
                let low = filter.low_pass(coefficient, input);
                *sample = input + amount * low;
            }
            if self.controls.flush_due() {
                for filter in &mut self.filters {
                    filter.flush();
                }
            }
        }
    }

    fn tail_samples(&self) -> usize {
        self.tail
    }
    fn gap_samples(&self) -> usize {
        self.tail
    }
}

// Positive g and k yield stable poles. The largest radius over a rectangular
// g/k domain is at its endpoints: low damping covers complex poles, maximum
// damping covers overdamped real poles. Include both rather than assuming
// every gain-dependent peak bank is underdamped.
fn svf_tail(rate: f32, gain_scale: f32, minimum_k: f32, maximum_k: f32) -> usize {
    let min_g = f64::from(cutoff_gain(20.0, rate) / gain_scale);
    let max_g = f64::from(cutoff_gain(20_000.0, rate) * gain_scale);
    let radius = [min_g, max_g]
        .into_iter()
        .flat_map(|g| [minimum_k, maximum_k].map(|k| pole_radius(g, f64::from(k))))
        .fold(0.0_f64, f64::max);
    decay_samples(radius, rate)
}

fn pole_radius(g: f64, k: f64) -> f64 {
    let divisor = 1.0 + g * k + g * g;
    let a1 = 2.0 * (g * g - 1.0) / divisor;
    let a2 = (1.0 - g * k + g * g) / divisor;
    let discriminant = a1 * a1 - 4.0 * a2;
    if discriminant < 0.0 {
        a2.sqrt()
    } else {
        let root = discriminant.sqrt();
        ((-a1 + root) * 0.5).abs().max(((-a1 - root) * 0.5).abs())
    }
}

fn decay_samples(radius: f64, rate: f32) -> usize {
    (64.0 / -radius.max(1.0e-30).ln()).ceil() as usize
        + ms_to_samples(FILTER_SMOOTHING_MS, rate) as usize
        + FLUSH_PERIOD as usize
}
