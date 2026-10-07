//! A seven-band parametric equaliser.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::blocks::biquad::{Biquad, BiquadCoeffs, CoeffRamp};
use crate::blocks::math::{db_to_gain, ms_to_samples, smoothing_coefficient};
use crate::blocks::smooth::LinearRamp;
use crate::blocks::{CONTROL_PERIOD, SMOOTHING_MS};
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};

/// Time a band takes to cover 63% of the way to a new frequency, gain or Q.
const BAND_SMOOTHING_MS: f32 = 15.0;

/// Longest tail the equaliser reports, in seconds.
const MAX_TAIL_SECONDS: f32 = 10.0;

/// Most second-order sections a cut filter uses.
const MAX_CUT_STAGES: usize = 4;

/// How steeply a cut filter falls away past its corner.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CutSlope {
    /// 12 dB per octave: gentle.
    #[default]
    Db12,
    /// 24 dB per octave.
    Db24,
    /// 48 dB per octave: close to a wall.
    Db48,
}

impl CutSlope {
    /// Second-order sections in the filter.
    fn stages(self) -> usize {
        match self {
            CutSlope::Db12 => 1,
            CutSlope::Db24 => 2,
            CutSlope::Db48 => 4,
        }
    }
}

/// A shelf or bell band of the equaliser.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct EqBand {
    /// A band that is off leaves the signal alone.
    pub enabled: bool,
    /// Centre of a bell, or corner of a shelf, in Hz. 20 to 20000.
    pub frequency_hz: f32,
    /// Boost or cut in dB. -24 to 24, default 0.
    pub gain_db: f32,
    /// Sharpness. For a bell, higher is narrower: 0.1 to 18, default 1.
    /// For a shelf, 0.707 is the smoothest slope and higher values add a
    /// bump before the shelf: 0.1 to 2, default 0.707.
    pub q: f32,
}

impl Default for EqBand {
    fn default() -> Self {
        Self {
            enabled: true,
            frequency_hz: 1_000.0,
            gain_db: 0.0,
            q: 1.0,
        }
    }
}

/// The low cut or high cut band of the equaliser.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct EqCutBand {
    /// Off by default.
    pub enabled: bool,
    /// The corner in Hz, where the filter is 3 dB down at the default Q.
    /// 20 to 20000.
    pub frequency_hz: f32,
    /// Resonance at the corner. 0.707 is flat right up to the corner;
    /// higher values add a peak there. 0.25 to 8, default 0.707.
    pub q: f32,
    /// How steeply the filter falls. Default 12 dB per octave.
    pub slope: CutSlope,
}

impl Default for EqCutBand {
    fn default() -> Self {
        Self {
            enabled: false,
            frequency_hz: 30.0,
            q: std::f32::consts::FRAC_1_SQRT_2,
            slope: CutSlope::Db12,
        }
    }
}

/// Settings of the [`ParametricEq`]. The defaults change nothing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct EqParams {
    /// Removes everything below its corner. Default 30 Hz, off.
    pub low_cut: EqCutBand,
    /// Boosts or cuts everything below its corner. Default 100 Hz.
    pub low_shelf: EqBand,
    /// A bell. Default 400 Hz.
    pub peak1: EqBand,
    /// A bell. Default 1 kHz.
    pub peak2: EqBand,
    /// A bell. Default 3.5 kHz.
    pub peak3: EqBand,
    /// Boosts or cuts everything above its corner. Default 8 kHz.
    pub high_shelf: EqBand,
    /// Removes everything above its corner. Default 18 kHz, off.
    pub high_cut: EqCutBand,
    /// Gain applied after all bands, in dB. -24 to 24, default 0.
    pub output_gain_db: f32,
}

impl Default for EqParams {
    fn default() -> Self {
        let shelf = |frequency_hz| EqBand {
            frequency_hz,
            q: std::f32::consts::FRAC_1_SQRT_2,
            ..EqBand::default()
        };
        let peak = |frequency_hz| EqBand {
            frequency_hz,
            ..EqBand::default()
        };
        Self {
            low_cut: EqCutBand::default(),
            low_shelf: shelf(100.0),
            peak1: peak(400.0),
            peak2: peak(1_000.0),
            peak3: peak(3_500.0),
            high_shelf: shelf(8_000.0),
            high_cut: EqCutBand {
                frequency_hz: 18_000.0,
                ..EqCutBand::default()
            },
            output_gain_db: 0.0,
        }
    }
}

param_set!(EqParams, "Parametric EQ", {
    toggle [low_cut.enabled] "lowCut.enabled" "Low cut" { false }
    float [low_cut.frequency_hz] "lowCut.frequencyHz" "Low cut frequency"
        { Hertz, Logarithmic, 20.0, 20_000.0, 30.0 }
    float [low_cut.q] "lowCut.q" "Low cut Q"
        { None, Logarithmic, 0.25, 8.0, std::f32::consts::FRAC_1_SQRT_2 }
    choice [low_cut.slope] "lowCut.slope" "Low cut slope" {
        CutSlope, Db12,
        [Db12 "db12" "12 dB/oct", Db24 "db24" "24 dB/oct", Db48 "db48" "48 dB/oct"]
    }

    toggle [low_shelf.enabled] "lowShelf.enabled" "Low shelf" { true }
    float [low_shelf.frequency_hz] "lowShelf.frequencyHz" "Low shelf frequency"
        { Hertz, Logarithmic, 20.0, 20_000.0, 100.0 }
    float [low_shelf.gain_db] "lowShelf.gainDb" "Low shelf gain"
        { Decibels, Linear, -24.0, 24.0, 0.0 }
    float [low_shelf.q] "lowShelf.q" "Low shelf Q"
        { None, Logarithmic, 0.1, 2.0, std::f32::consts::FRAC_1_SQRT_2 }

    toggle [peak1.enabled] "peak1.enabled" "Peak 1" { true }
    float [peak1.frequency_hz] "peak1.frequencyHz" "Peak 1 frequency"
        { Hertz, Logarithmic, 20.0, 20_000.0, 400.0 }
    float [peak1.gain_db] "peak1.gainDb" "Peak 1 gain" { Decibels, Linear, -24.0, 24.0, 0.0 }
    float [peak1.q] "peak1.q" "Peak 1 Q" { None, Logarithmic, 0.1, 18.0, 1.0 }

    toggle [peak2.enabled] "peak2.enabled" "Peak 2" { true }
    float [peak2.frequency_hz] "peak2.frequencyHz" "Peak 2 frequency"
        { Hertz, Logarithmic, 20.0, 20_000.0, 1_000.0 }
    float [peak2.gain_db] "peak2.gainDb" "Peak 2 gain" { Decibels, Linear, -24.0, 24.0, 0.0 }
    float [peak2.q] "peak2.q" "Peak 2 Q" { None, Logarithmic, 0.1, 18.0, 1.0 }

    toggle [peak3.enabled] "peak3.enabled" "Peak 3" { true }
    float [peak3.frequency_hz] "peak3.frequencyHz" "Peak 3 frequency"
        { Hertz, Logarithmic, 20.0, 20_000.0, 3_500.0 }
    float [peak3.gain_db] "peak3.gainDb" "Peak 3 gain" { Decibels, Linear, -24.0, 24.0, 0.0 }
    float [peak3.q] "peak3.q" "Peak 3 Q" { None, Logarithmic, 0.1, 18.0, 1.0 }

    toggle [high_shelf.enabled] "highShelf.enabled" "High shelf" { true }
    float [high_shelf.frequency_hz] "highShelf.frequencyHz" "High shelf frequency"
        { Hertz, Logarithmic, 20.0, 20_000.0, 8_000.0 }
    float [high_shelf.gain_db] "highShelf.gainDb" "High shelf gain"
        { Decibels, Linear, -24.0, 24.0, 0.0 }
    float [high_shelf.q] "highShelf.q" "High shelf Q"
        { None, Logarithmic, 0.1, 2.0, std::f32::consts::FRAC_1_SQRT_2 }

    toggle [high_cut.enabled] "highCut.enabled" "High cut" { false }
    float [high_cut.frequency_hz] "highCut.frequencyHz" "High cut frequency"
        { Hertz, Logarithmic, 20.0, 20_000.0, 18_000.0 }
    float [high_cut.q] "highCut.q" "High cut Q"
        { None, Logarithmic, 0.25, 8.0, std::f32::consts::FRAC_1_SQRT_2 }
    choice [high_cut.slope] "highCut.slope" "High cut slope" {
        CutSlope, Db12,
        [Db12 "db12" "12 dB/oct", Db24 "db24" "24 dB/oct", Db48 "db48" "48 dB/oct"]
    }

    float [output_gain_db] "outputGainDb" "Output gain" { Decibels, Linear, -24.0, 24.0, 0.0 }
});

/// Which response a shelf or bell band has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BellKind {
    LowShelf,
    Peak,
    HighShelf,
}

/// The three values that set a band's response, in the form they are
/// smoothed in: frequency and Q move in equal ratios, gain in equal dB.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Shape {
    log_frequency: f32,
    gain_db: f32,
    log_q: f32,
}

impl Shape {
    fn new(frequency_hz: f32, gain_db: f32, q: f32) -> Self {
        Self {
            log_frequency: frequency_hz.ln(),
            gain_db,
            log_q: q.ln(),
        }
    }

    fn frequency_hz(&self) -> f32 {
        self.log_frequency.exp()
    }

    fn q(&self) -> f32 {
        self.log_q.exp()
    }

    /// Moves `amount` of the way to `target`. Returns true while it has
    /// not arrived.
    fn approach(&mut self, target: &Shape, amount: f32) -> bool {
        let mut moving = false;
        let mut step = |value: &mut f32, target: f32, close_enough: f32| {
            let gap = target - *value;
            if gap.abs() <= close_enough {
                *value = target;
            } else {
                *value += gap * amount;
                moving = true;
            }
        };
        step(&mut self.log_frequency, target.log_frequency, 1.0e-4);
        step(&mut self.gain_db, target.gain_db, 1.0e-3);
        step(&mut self.log_q, target.log_q, 1.0e-4);
        moving
    }
}

/// One second-order section with coefficients that glide, for both
/// channels.
#[derive(Debug, Clone, Copy)]
struct Stage {
    coeffs: CoeffRamp,
    left: Biquad,
    right: Biquad,
}

impl Stage {
    fn new() -> Self {
        Self {
            coeffs: CoeffRamp::new(BiquadCoeffs::IDENTITY),
            left: Biquad::default(),
            right: Biquad::default(),
        }
    }

    fn clear(&mut self) {
        self.left.reset();
        self.right.reset();
    }

    fn flush(&mut self) {
        self.left.flush();
        self.right.flush();
    }

    fn run(&mut self, left: &mut [f32], right: &mut [f32]) {
        if self.coeffs.is_settled() {
            let coeffs = *self.coeffs.current();
            for (left, right) in left.iter_mut().zip(right.iter_mut()) {
                *left = self.left.tick(&coeffs, f64::from(*left)) as f32;
                *right = self.right.tick(&coeffs, f64::from(*right)) as f32;
            }
        } else {
            for (left, right) in left.iter_mut().zip(right.iter_mut()) {
                let coeffs = *self.coeffs.tick();
                *left = self.left.tick(&coeffs, f64::from(*left)) as f32;
                *right = self.right.tick(&coeffs, f64::from(*right)) as f32;
            }
        }
    }
}

fn bell_coeffs(kind: BellKind, shape: &Shape, sample_rate: f32) -> BiquadCoeffs {
    let (frequency, q, gain) = (shape.frequency_hz(), shape.q(), shape.gain_db);
    match kind {
        BellKind::LowShelf => BiquadCoeffs::low_shelf(frequency, q, gain, sample_rate),
        BellKind::Peak => BiquadCoeffs::peak(frequency, q, gain, sample_rate),
        BellKind::HighShelf => BiquadCoeffs::high_shelf(frequency, q, gain, sample_rate),
    }
}

/// Coefficients of section `stage` of a cut filter made of `stages`
/// sections.
///
/// The sections together form a Butterworth filter, the one with the
/// flattest passband, of order `2 * stages`. The band's Q scales the
/// sharpest section, so 0.707 gives the pure Butterworth response and
/// higher values add a resonant peak at the corner.
fn cut_coeffs(
    high_pass: bool,
    stage: usize,
    stages: usize,
    shape: &Shape,
    sample_rate: f32,
) -> BiquadCoeffs {
    let angle = std::f32::consts::PI * (2 * stage + 1) as f32 / (4 * stages) as f32;
    let mut q = 1.0 / (2.0 * angle.cos());
    if stage + 1 == stages {
        q *= shape.q() / std::f32::consts::FRAC_1_SQRT_2;
    }
    if high_pass {
        BiquadCoeffs::high_pass(shape.frequency_hz(), q, sample_rate)
    } else {
        BiquadCoeffs::low_pass(shape.frequency_hz(), q, sample_rate)
    }
}

/// A shelf or bell band while it runs.
#[derive(Debug, Clone, Copy)]
struct Bell {
    kind: BellKind,
    target: Shape,
    shape: Shape,
    moving: bool,
    stage: Stage,
}

impl Bell {
    fn new(kind: BellKind) -> Self {
        let shape = Shape::new(1_000.0, 0.0, 1.0);
        Self {
            kind,
            target: shape,
            shape,
            moving: false,
            stage: Stage::new(),
        }
    }

    fn set_target(&mut self, band: &EqBand, snap: bool, sample_rate: f32) {
        // A band that is off is the same band at 0 dB, which lets it fade
        // out and in instead of switching.
        let gain_db = if band.enabled { band.gain_db } else { 0.0 };
        self.target = Shape::new(band.frequency_hz, gain_db, band.q);
        // A flat band can be retuned at once: nothing of it is heard.
        if snap || (self.is_flat() && gain_db == 0.0) {
            self.shape = self.target;
            self.moving = false;
            self.stage
                .coeffs
                .snap(bell_coeffs(self.kind, &self.shape, sample_rate));
        } else if self.target != self.shape {
            self.moving = true;
        }
    }

    /// True when the band does nothing and can be skipped.
    fn is_flat(&self) -> bool {
        !self.moving && self.stage.coeffs.is_settled() && self.shape.gain_db == 0.0
    }

    fn control(&mut self, amount: f32, sample_rate: f32) {
        if self.moving {
            self.moving = self.shape.approach(&self.target, amount);
            self.stage.coeffs.set_target(
                bell_coeffs(self.kind, &self.shape, sample_rate),
                CONTROL_PERIOD as u32,
            );
        }
        if self.is_flat() {
            self.stage.clear();
        } else {
            self.stage.flush();
        }
    }
}

/// A cut band while it runs.
#[derive(Debug, Clone, Copy)]
struct Cut {
    high_pass: bool,
    enabled: bool,
    target: Shape,
    shape: Shape,
    moving: bool,
    /// The slope in use, and the one asked for. Changing slope fades the
    /// filter out, swaps it, and fades it back in.
    slope: CutSlope,
    wanted_slope: CutSlope,
    /// Share of the filtered signal in the band's output.
    mix: LinearRamp,
    stages: [Stage; MAX_CUT_STAGES],
}

impl Cut {
    fn new(high_pass: bool) -> Self {
        let shape = Shape::new(1_000.0, 0.0, std::f32::consts::FRAC_1_SQRT_2);
        Self {
            high_pass,
            enabled: false,
            target: shape,
            shape,
            moving: false,
            slope: CutSlope::Db12,
            wanted_slope: CutSlope::Db12,
            mix: LinearRamp::new(0.0),
            stages: [Stage::new(); MAX_CUT_STAGES],
        }
    }

    fn snap_coeffs(&mut self, sample_rate: f32) {
        let stages = self.slope.stages();
        for (index, stage) in self.stages[..stages].iter_mut().enumerate() {
            stage.coeffs.snap(cut_coeffs(
                self.high_pass,
                index,
                stages,
                &self.shape,
                sample_rate,
            ));
        }
    }

    fn clear(&mut self) {
        for stage in &mut self.stages {
            stage.clear();
        }
    }

    fn set_target(&mut self, band: &EqCutBand, snap: bool, sample_rate: f32) {
        self.enabled = band.enabled;
        self.wanted_slope = band.slope;
        self.target = Shape::new(band.frequency_hz, 0.0, band.q);
        if snap {
            self.mix.snap(if self.enabled { 1.0 } else { 0.0 });
        }
        // A filter that is faded out can be retuned at once.
        if snap || self.is_off() {
            self.shape = self.target;
            self.moving = false;
            self.slope = self.wanted_slope;
            self.clear();
            self.snap_coeffs(sample_rate);
        } else if self.target != self.shape {
            self.moving = true;
        }
    }

    fn is_off(&self) -> bool {
        self.mix.is_settled() && self.mix.value() == 0.0
    }

    fn control(&mut self, amount: f32, sample_rate: f32, fade: u32) {
        if self.slope != self.wanted_slope && self.is_off() {
            self.slope = self.wanted_slope;
            self.clear();
            self.snap_coeffs(sample_rate);
        }
        let heard = self.enabled && self.slope == self.wanted_slope;
        self.mix.set_target(if heard { 1.0 } else { 0.0 }, fade);

        let stages = self.slope.stages();
        if self.moving {
            self.moving = self.shape.approach(&self.target, amount);
            for (index, stage) in self.stages[..stages].iter_mut().enumerate() {
                stage.coeffs.set_target(
                    cut_coeffs(self.high_pass, index, stages, &self.shape, sample_rate),
                    CONTROL_PERIOD as u32,
                );
            }
        }
        if self.is_off() {
            self.clear();
        } else {
            for stage in &mut self.stages[..stages] {
                stage.flush();
            }
        }
    }

    /// Filters up to [`CONTROL_PERIOD`] samples in place.
    fn run(&mut self, left: &mut [f32], right: &mut [f32]) {
        if self.is_off() {
            return;
        }
        let frames = left.len();
        let mut dry_left = [0.0_f32; CONTROL_PERIOD];
        let mut dry_right = [0.0_f32; CONTROL_PERIOD];
        dry_left[..frames].copy_from_slice(left);
        dry_right[..frames].copy_from_slice(right);
        for stage in &mut self.stages[..self.slope.stages()] {
            stage.run(left, right);
        }
        if self.mix.is_settled() && self.mix.value() == 1.0 {
            return;
        }
        for index in 0..frames {
            let mix = self.mix.tick();
            left[index] = dry_left[index] + (left[index] - dry_left[index]) * mix;
            right[index] = dry_right[index] + (right[index] - dry_right[index]) * mix;
        }
    }
}

impl EqParams {
    /// Calls `visit` with the coefficients of every section these settings
    /// put in the signal path.
    fn for_each_section(&self, sample_rate: f32, mut visit: impl FnMut(BiquadCoeffs)) {
        let cuts = [(&self.low_cut, true), (&self.high_cut, false)];
        for (band, high_pass) in cuts {
            if band.enabled {
                let shape = Shape::new(band.frequency_hz, 0.0, band.q);
                let stages = band.slope.stages();
                for stage in 0..stages {
                    visit(cut_coeffs(high_pass, stage, stages, &shape, sample_rate));
                }
            }
        }
        let bells = [
            (&self.low_shelf, BellKind::LowShelf),
            (&self.peak1, BellKind::Peak),
            (&self.peak2, BellKind::Peak),
            (&self.peak3, BellKind::Peak),
            (&self.high_shelf, BellKind::HighShelf),
        ];
        for (band, kind) in bells {
            if band.enabled && band.gain_db != 0.0 {
                let shape = Shape::new(band.frequency_hz, band.gain_db, band.q);
                visit(bell_coeffs(kind, &shape, sample_rate));
            }
        }
    }

    /// The equaliser's gain in dB at each of `frequencies_hz`, written to
    /// the matching place in `gains_db`. This is the curve an EQ display
    /// draws. It is exact: it is computed from the same filter coefficients
    /// the audio runs through. Safe to call from any thread.
    pub fn magnitude_response_db(
        &self,
        sample_rate: f32,
        frequencies_hz: &[f32],
        gains_db: &mut [f32],
    ) {
        let params = self.sanitized();
        for gain in gains_db.iter_mut() {
            *gain = params.output_gain_db;
        }
        params.for_each_section(sample_rate, |coeffs| {
            for (frequency, gain) in frequencies_hz.iter().zip(gains_db.iter_mut()) {
                let magnitude = coeffs.magnitude(*frequency, sample_rate).max(1.0e-10);
                *gain += (20.0 * magnitude.log10()) as f32;
            }
        });
    }

    /// Seconds the filters ring on after the input stops, to 60 dB down.
    fn ring_seconds(&self, sample_rate: f32) -> f32 {
        let mut samples = 0.0_f64;
        self.for_each_section(sample_rate, |coeffs| {
            // The slower pole of the section sets how long it rings.
            let discriminant = coeffs.a1 * coeffs.a1 - 4.0 * coeffs.a2;
            let radius = if discriminant < 0.0 {
                coeffs.a2.abs().sqrt()
            } else {
                (coeffs.a1.abs() + discriminant.sqrt()) * 0.5
            };
            if radius > 0.0 && radius < 1.0 {
                samples += (1.0e-3_f64).ln() / radius.ln();
            }
        });
        (samples / f64::from(sample_rate)) as f32
    }
}

/// A parametric equaliser: a low cut, a low shelf, three bells, a high
/// shelf and a high cut, followed by an output gain.
///
/// Each band is a second-order section from the Audio EQ Cookbook (the cut
/// filters are up to four of them in a row). Moving a control glides the
/// band's frequency, gain and Q and interpolates the filter coefficients
/// between updates, so sweeps are silent and stable however fast they are.
/// Switching a band on or off fades it. With every band flat, the output is
/// the input, bit for bit. It adds no latency.
///
/// Like every filter of this kind, bells and shelves placed close to half
/// the sample rate come out narrower than at lower frequencies.
pub struct ParametricEq {
    sample_rate: f32,
    params: EqParams,
    low_cut: Cut,
    bells: [Bell; 5],
    high_cut: Cut,
    output: LinearRamp,
    /// Samples until the next control update.
    until_control: usize,
    /// Share of the way a band moves per control update.
    amount: f32,
    fresh: bool,
}

impl Default for ParametricEq {
    fn default() -> Self {
        let mut eq = Self {
            sample_rate: 48_000.0,
            params: EqParams::default(),
            low_cut: Cut::new(true),
            bells: [
                Bell::new(BellKind::LowShelf),
                Bell::new(BellKind::Peak),
                Bell::new(BellKind::Peak),
                Bell::new(BellKind::Peak),
                Bell::new(BellKind::HighShelf),
            ],
            high_cut: Cut::new(false),
            output: LinearRamp::new(1.0),
            until_control: 0,
            amount: 1.0,
            fresh: true,
        };
        eq.apply();
        eq
    }
}

impl ParametricEq {
    fn apply(&mut self) {
        let (params, snap, rate) = (self.params, self.fresh, self.sample_rate);
        self.amount = smoothing_coefficient(BAND_SMOOTHING_MS, rate / CONTROL_PERIOD as f32);
        self.low_cut.set_target(&params.low_cut, snap, rate);
        self.high_cut.set_target(&params.high_cut, snap, rate);
        let bands = [
            &params.low_shelf,
            &params.peak1,
            &params.peak2,
            &params.peak3,
            &params.high_shelf,
        ];
        for (bell, band) in self.bells.iter_mut().zip(bands) {
            bell.set_target(band, snap, rate);
        }
        let samples = if snap {
            0
        } else {
            ms_to_samples(SMOOTHING_MS, rate)
        };
        self.output
            .set_target(db_to_gain(params.output_gain_db), samples);
    }

    fn control(&mut self) {
        let fade = ms_to_samples(SMOOTHING_MS, self.sample_rate);
        self.low_cut.control(self.amount, self.sample_rate, fade);
        self.high_cut.control(self.amount, self.sample_rate, fade);
        for bell in &mut self.bells {
            bell.control(self.amount, self.sample_rate);
        }
    }
}

impl Effect for ParametricEq {
    type Params = EqParams;

    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.sample_rate = sample_rate.max(1.0);
        self.reset();
    }

    fn reset(&mut self) {
        self.fresh = true;
        self.apply();
        self.low_cut.clear();
        self.high_cut.clear();
        for bell in &mut self.bells {
            bell.stage.clear();
        }
        self.until_control = 0;
    }

    fn set_params(&mut self, params: &EqParams) {
        self.params = params.sanitized();
        self.apply();
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.fresh = false;
        let frames = left.len().min(right.len());
        let mut start = 0;
        while start < frames {
            if self.until_control == 0 {
                self.control();
                self.until_control = CONTROL_PERIOD;
            }
            let end = start + (frames - start).min(self.until_control);
            let (left, right) = (&mut left[start..end], &mut right[start..end]);
            self.low_cut.run(left, right);
            for bell in &mut self.bells {
                if !bell.is_flat() {
                    bell.stage.run(left, right);
                }
            }
            self.high_cut.run(left, right);
            if !(self.output.is_settled() && self.output.value() == 1.0) {
                for (left, right) in left.iter_mut().zip(right.iter_mut()) {
                    let gain = self.output.tick();
                    *left *= gain;
                    *right *= gain;
                }
            }
            self.until_control -= end - start;
            start = end;
        }
    }

    fn tail_samples(&self) -> usize {
        let seconds = self
            .params
            .ring_seconds(self.sample_rate)
            .min(MAX_TAIL_SECONDS);
        (seconds * self.sample_rate).ceil() as usize
    }
}
