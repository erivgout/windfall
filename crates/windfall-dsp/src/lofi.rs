//! Original resolution reduction, timed sample replacement and resonant tone.
//!
//! See `docs/LOFI.md` for the original equations, source context and limits.

use crate::blocks::math::{clean, db_to_gain, flush64, ms_to_samples};
use crate::param::param_set;
use crate::{Effect, ParamSet};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// All signal-space controls arrive after this many milliseconds of frames.
pub const LOFI_SMOOTHING_MS: f32 = 5.0;
/// Largest hold interval, in host frames. No delay buffer is required.
pub const LOFI_MAX_RATIO: f32 = 64.0;
/// Largest resonant quality factor; bounds the frozen tail estimate.
pub const LOFI_MAX_Q: f64 = 4.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum RunRelation {
    #[default]
    Independent,
    Equal,
    Double,
    Half,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FilterPlacement {
    #[default]
    Post,
    Pre,
}

// Evaluate from a frozen start and integer frame position, avoiding accumulated
// f32 slope error on long (e.g. 192 kHz) ramps. Retarget from the last emitted
// value; unchanged targets preserve their arrival time.
#[derive(Clone, Copy)]
struct FrameRamp {
    value: f32,
    start: f32,
    target: f32,
    elapsed: u32,
    length: u32,
}

impl FrameRamp {
    fn new(value: f32) -> Self {
        Self {
            value,
            start: value,
            target: value,
            elapsed: 0,
            length: 0,
        }
    }

    fn snap(&mut self, value: f32) {
        *self = Self::new(value);
    }

    fn target(&self) -> f32 {
        self.target
    }

    fn set_target(&mut self, target: f32, length: u32) {
        if length == 0 {
            self.snap(target);
        } else if target != self.target {
            self.start = self.value;
            self.target = target;
            self.length = length;
            self.elapsed = 0;
        }
    }

    #[inline]
    fn tick(&mut self) -> f32 {
        if self.elapsed < self.length {
            self.elapsed += 1;
            self.value = if self.elapsed == self.length {
                self.target
            } else {
                let t = f64::from(self.elapsed) / f64::from(self.length);
                (f64::from(self.start) + (f64::from(self.target) - f64::from(self.start)) * t)
                    as f32
            };
        }
        self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct LofiParams {
    /// Symmetric midtread resolution, 2..=16 bits; default 8.
    pub bits: u8,
    /// Blend from unquantized to quantized, 0..=1; default 1.
    pub quantize: f32,
    /// Host rate / capture rate, 1..=64; default 4. Fractional ratios work.
    pub rate_ratio: f32,
    /// Drive before rational saturation, 0..=36 dB; default 6.
    pub drive_db: f32,
    /// Blend from original to driven saturation, 0..=1; default 0.25.
    pub distortion: f32,
    /// Tone cutoff frequency, 20..=20000 Hz; default 8000.
    /// Effective frequency is capped at 0.45 * prepared sample rate.
    pub cutoff_hz: f32,
    /// Blend from unfiltered to tone-filtered audio, 0..=1; default 1.
    pub filter: f32,
    /// Dry/wet blend, 0..=1; default 1. Histories keep running when dry.
    pub mix: f32,
    /// Final dry-and-wet trim, -24..=12 dB; default -3.
    pub output_db: f32,
    /// Unreplaced run duration, 0..=1000 ms; default 10.
    pub preserve_ms: f32,
    /// Independent replaced run duration, 0..=1000 ms; default 2.
    pub replace_ms: f32,
    /// Replaced duration independent/equal/double/half of the preserved run.
    pub run_relation: RunRelation,
    /// Fixed positive nominal amplitude during replaced runs, 0..=1; default 0.
    pub replacement_value: f32,
    /// Blend toward the fixed value during replaced runs, 0..=1; default 0.
    pub replacement: f32,
    /// Morph from monotonic tone to resonant lowpass, 0..=1; default 0.
    /// The resonant path's Q increases from 1/sqrt(2) to 4.
    pub resonance: f32,
    /// Tone filter before or after drive/quantization/replacement/hold.
    pub placement: FilterPlacement,
}

impl Default for LofiParams {
    fn default() -> Self {
        Self {
            bits: 8,
            quantize: 1.0,
            rate_ratio: 4.0,
            drive_db: 6.0,
            distortion: 0.25,
            cutoff_hz: 8_000.0,
            filter: 1.0,
            mix: 1.0,
            output_db: -3.0,
            preserve_ms: 10.0,
            replace_ms: 2.0,
            run_relation: RunRelation::Independent,
            replacement_value: 0.0,
            replacement: 0.0,
            resonance: 0.0,
            placement: FilterPlacement::Post,
        }
    }
}

impl LofiParams {
    /// Exact unity, including finite samples above full scale and signed zero.
    pub const fn neutral() -> Self {
        Self {
            bits: 16,
            quantize: 0.0,
            rate_ratio: 1.0,
            drive_db: 0.0,
            distortion: 0.0,
            cutoff_hz: 20_000.0,
            filter: 0.0,
            mix: 1.0,
            output_db: 0.0,
            preserve_ms: 10.0,
            replace_ms: 2.0,
            run_relation: RunRelation::Independent,
            replacement_value: 0.0,
            replacement: 0.0,
            resonance: 0.0,
            placement: FilterPlacement::Post,
        }
    }
}

// Persisted/automation positions: append new controls; never reorder these.
param_set!(LofiParams, "Lo-fi reduction", {
    int [bits] "bits" "Resolution" { None, 2, 16, 8 }
    float [quantize] "quantize" "Quantization" { Fraction, Linear, 0.0, 1.0, 1.0 }
    float [rate_ratio] "rateRatio" "Rate reduction" { Ratio, Logarithmic, 1.0, LOFI_MAX_RATIO, 4.0 }
    float [drive_db] "driveDb" "Drive" { Decibels, Linear, 0.0, 36.0, 6.0 }
    float [distortion] "distortion" "Distortion" { Fraction, Linear, 0.0, 1.0, 0.25 }
    float [cutoff_hz] "cutoffHz" "Tone cutoff" { Hertz, Logarithmic, 20.0, 20_000.0, 8_000.0 }
    float [filter] "filter" "Tone filter" { Fraction, Linear, 0.0, 1.0, 1.0 }
    float [mix] "mix" "Mix" { Fraction, Linear, 0.0, 1.0, 1.0 }
    float [output_db] "outputDb" "Output" { Decibels, Linear, -24.0, 12.0, -3.0 }
    float [preserve_ms] "preserveMs" "Preserve time" { Milliseconds, Linear, 0.0, 1_000.0, 10.0 }
    float [replace_ms] "replaceMs" "Replace time" { Milliseconds, Linear, 0.0, 1_000.0, 2.0 }
    choice [run_relation] "runRelation" "Run relationship" { RunRelation, Independent, [
        Independent "independent" "Independent", Equal "equal" "Equal",
        Double "double" "Double", Half "half" "Half"
    ] }
    float [replacement_value] "replacementValue" "Replacement value" { Fraction, Linear, 0.0, 1.0, 0.0 }
    float [replacement] "replacement" "Replacement" { Fraction, Linear, 0.0, 1.0, 0.0 }
    float [resonance] "resonance" "Resonance" { Fraction, Linear, 0.0, 1.0, 0.0 }
    choice [placement] "placement" "Filter placement" { FilterPlacement, Post, [Post "post" "Post", Pre "pre" "Pre"] }
});

#[inline]
fn blend(dry: f64, wet: f64, amount: f32) -> f64 {
    // Exact endpoints preserve headroom and signed zero without cancellation.
    if amount == 0.0 {
        dry
    } else if amount == 1.0 {
        wet
    } else {
        dry + (wet - dry) * f64::from(amount)
    }
}

#[inline]
fn quantize(input: f64, bits: u32) -> f64 {
    let endpoint = f64::from((1_u32 << (bits - 1)) - 1);
    (input.clamp(-1.0, 1.0) * endpoint).round() / endpoint
}

#[inline]
fn spacing(bits: u32) -> f32 {
    1.0 / ((1_u32 << (bits - 1)) - 1) as f32
}

#[inline]
fn morph_quantize(input: f64, step: f32) -> f64 {
    if step >= spacing(2) {
        return quantize(input, 2);
    }
    if step <= spacing(16) {
        return quantize(input, 16);
    }
    // Fixed four comparisons locate adjacent grids in the 15-depth ladder.
    // Linear step-space motion devotes more of the 5 ms to coarse grids,
    // which have the largest potential signal change.
    let (mut lower, mut higher) = (2, 16);
    for _ in 0..4 {
        if higher - lower > 1 {
            let middle = (lower + higher) / 2;
            if step > spacing(middle) {
                higher = middle;
            } else {
                lower = middle;
            }
        }
    }
    let amount = (spacing(lower) - step) / (spacing(lower) - spacing(higher));
    blend(quantize(input, lower), quantize(input, higher), amount)
}

#[derive(Default)]
struct ToneFilter {
    low: [f64; 2],
    integrators: [f64; 2],
}

impl ToneFilter {
    fn tick(&mut self, input: f64, controls: &[f32; 17]) -> f64 {
        let alpha = f64::from(controls[5]);
        self.low[0] = flush64(self.low[0] + alpha * (input - self.low[0]));
        self.low[1] = flush64(self.low[1] + alpha * (self.low[0] - self.low[1]));
        // Original solution of the two trapezoidal integrator equations in
        // double precision, retaining the established monotonic path at zero
        // resonance. g and damping are ramped, never tan-computed per frame.
        let g = f64::from(controls[9]);
        let k = f64::from(controls[10]);
        let a = 1.0 / (1.0 + g * (g + k));
        let band = a * (self.integrators[0] + g * (input - self.integrators[1]));
        let low = self.integrators[1] + g * band;
        self.integrators[0] = flush64(2.0 * band - self.integrators[0]);
        self.integrators[1] = flush64(2.0 * low - self.integrators[1]);
        blend(self.low[1], low, controls[11])
    }
}

#[derive(Default)]
struct Channel {
    phase: f64,
    held: f64,
    tone: [ToneFilter; 2],
    run_phase: f64,
    run_error: f64,
    activity: u32,
}

impl Channel {
    fn reset(&mut self) {
        *self = Self {
            phase: 1.0, // Capture the very first frame, including after reset.
            ..Self::default()
        };
    }

    #[inline]
    fn tick(&mut self, input: f32, controls: &[f32; 17]) -> f32 {
        // The direct DSP contract requires finite input. Release builds expose
        // invalid samples unchanged instead of converting them to plausible audio.
        if !input.is_finite() {
            return input;
        }
        let [
            step,
            amount,
            ratio,
            drive,
            distortion,
            _,
            filter,
            mix,
            output,
            ..,
        ] = *controls;
        let dry = f64::from(input);
        let pre = blend(dry, self.tone[0].tick(dry, controls), filter);
        let before_drive = blend(dry, pre, controls[12]);
        let driven = before_drive * f64::from(drive);
        let shaped = blend(before_drive, driven / (1.0 + driven.abs()), distortion);
        let mut crushed = if amount == 0.0 {
            shaped
        } else {
            // Morph adjacent integer grids. No per-frame exponentials and no
            // discontinuity when a resolution ramp passes an integer boundary.
            let grid = morph_quantize(shaped, step);
            blend(shaped, grid, amount)
        };
        let preserved = f64::from(controls[13]).round();
        let replaced = f64::from(controls[14]).round();
        let period = preserved + replaced;
        if period > 0.0 {
            // Input-triggered finite activity window: fixed-value replacement
            // may ring for a whole run cycle after the last non-silent input,
            // but cannot turn an idle processor into an indefinite DC source.
            if dry.abs() >= 1.0e-20 {
                self.activity = period as u32;
            }
            let replacing = replaced > 0.0 && self.run_phase >= preserved / period - 1.0e-12;
            if replacing && self.activity > 0 {
                crushed = blend(crushed, f64::from(controls[15]), controls[16]);
            }
            self.activity = self.activity.saturating_sub(1);
            let increment = 1.0 / period - self.run_error;
            let next = self.run_phase + increment;
            self.run_error = (next - self.run_phase) - increment;
            self.run_phase = next;
            if self.run_phase >= 1.0 - 1.0e-12 {
                self.run_phase = (self.run_phase - 1.0).max(0.0);
                self.run_error = 0.0;
            }
        } else {
            self.activity = 0;
        }
        if ratio == 1.0 {
            self.phase = 1.0;
        }
        // Tolerance only removes floating-point error at exact clock crossings;
        // it is 1e-12 of a capture cycle, not a frame-sized early capture.
        if self.phase >= 1.0 - 1.0e-12 {
            self.held = crushed;
            self.phase = (self.phase - 1.0).max(0.0);
        }
        self.phase += 1.0 / f64::from(ratio);

        let post = blend(self.held, self.tone[1].tick(self.held, controls), filter);
        let wet = blend(post, self.held, controls[12]);
        (blend(dry, wet, mix) * f64::from(output)) as f32
    }
}

/// Fixed-storage stereo processor. No heap storage, even during prepare.
///
/// Pre tone -> drive/quantization/replacement/hold -> post tone -> mix -> trim.
/// Finite input and equal slice lengths are required by the direct DSP contract.
/// Aliasing from the drive, quantizer and capture clock is intentional.
pub struct Lofi {
    params: LofiParams,
    rate: f32,
    ramps: [FrameRamp; 17],
    smoothing: u32,
    fresh: bool,
    channels: [Channel; 2],
    tail: usize,
}

impl Default for Lofi {
    fn default() -> Self {
        let mut effect = Self {
            params: LofiParams::default(),
            rate: 48_000.0,
            ramps: [FrameRamp::new(0.0); 17],
            smoothing: 240,
            fresh: true,
            channels: std::array::from_fn(|_| Channel::default()),
            tail: 0,
        };
        effect.prepare(48_000.0, 0);
        effect
    }
}

impl Lofi {
    fn update(&mut self) {
        let p = self.params;
        let cutoff = f64::from(p.cutoff_hz).min(0.45 * f64::from(self.rate));
        let alpha = -(-std::f64::consts::TAU * cutoff / f64::from(self.rate)).exp_m1();
        let preserved = (f64::from(p.preserve_ms) * f64::from(self.rate) * 0.001).round();
        let replaced = match p.run_relation {
            RunRelation::Independent => {
                (f64::from(p.replace_ms) * f64::from(self.rate) * 0.001).round()
            }
            RunRelation::Equal => preserved,
            RunRelation::Double => 2.0 * preserved,
            RunRelation::Half => (preserved * 0.5).round(),
        };
        let q = std::f64::consts::FRAC_1_SQRT_2
            + (LOFI_MAX_Q - std::f64::consts::FRAC_1_SQRT_2) * f64::from(p.resonance);
        let values = [
            spacing(u32::from(p.bits)),
            p.quantize,
            p.rate_ratio,
            db_to_gain(p.drive_db),
            p.distortion,
            alpha as f32,
            p.filter,
            p.mix,
            db_to_gain(p.output_db),
            (std::f64::consts::PI * cutoff / f64::from(self.rate)).tan() as f32,
            (1.0 / q) as f32,
            p.resonance,
            if p.placement == FilterPlacement::Pre {
                1.0
            } else {
                0.0
            },
            preserved as f32,
            replaced as f32,
            p.replacement_value,
            p.replacement,
        ];
        for (ramp, value) in self.ramps.iter_mut().zip(values) {
            ramp.set_target(value, if self.fresh { 0 } else { self.smoothing });
        }
    }
}

impl Effect for Lofi {
    type Params = LofiParams;

    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.rate = clean(sample_rate, 1.0, 384_000.0, 48_000.0);
        self.smoothing = ms_to_samples(LOFI_SMOOTHING_MS, self.rate);
        // Frozen bound for two tone stages, maximum resonant pole radius,
        // a full activity cycle, maximal hold and a complete control ramp.
        let minimum_cutoff = 20.0_f64.min(0.45 * f64::from(self.rate));
        let maximum_cutoff = 20_000.0_f64.min(0.45 * f64::from(self.rate));
        let radius = |cutoff: f64| {
            let g = (std::f64::consts::PI * cutoff / f64::from(self.rate)).tan();
            ((1.0 - g / LOFI_MAX_Q + g * g) / (1.0 + g / LOFI_MAX_Q + g * g)).sqrt()
        };
        let maximum_radius = radius(minimum_cutoff).max(radius(maximum_cutoff));
        self.tail = (160.0 / -maximum_radius.ln()).ceil() as usize
            + (3.0 * self.rate).ceil() as usize
            + LOFI_MAX_RATIO as usize
            + self.smoothing as usize;
        self.reset();
    }

    fn reset(&mut self) {
        for channel in &mut self.channels {
            channel.reset();
        }
        self.fresh = true;
        // Snap even if the stored target equals the one of an unfinished ramp.
        for ramp in &mut self.ramps {
            ramp.snap(ramp.target());
        }
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
        debug_assert_eq!(left.len(), right.len());
        for (left, right) in left.iter_mut().zip(right) {
            debug_assert!(
                left.is_finite() && right.is_finite(),
                "non-finite lo-fi input"
            );
            self.fresh = false;
            let controls = self.ramps.each_mut().map(FrameRamp::tick);
            *left = self.channels[0].tick(*left, &controls);
            *right = self.channels[1].tick(*right, &controls);
        }
    }

    fn tail_samples(&self) -> usize {
        self.tail
    }

    fn gap_samples(&self) -> usize {
        // Safe conservative fallback for hold + filter and automation histories.
        self.tail
    }
}
