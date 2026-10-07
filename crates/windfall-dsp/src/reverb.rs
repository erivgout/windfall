//! An algorithmic stereo reverb.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::blocks::delay_line::DelayLine;
use crate::blocks::math::{flush, ms_to_samples, smoothing_coefficient};
use crate::blocks::smooth::LinearRamp;
use crate::blocks::svf::{OnePoleFilter, Svf, SvfCoeffs};
use crate::blocks::{CONTROL_PERIOD, SMOOTHING_MS};
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};

/// Delay lines in the feedback network.
const LINES: usize = 16;

/// Loop time in ms of the shortest line, at a size of 0 and at a size of 1,
/// and the same for the longest line. The other lines are spread between
/// the two on a logarithmic scale.
///
/// Size moves the short end much further than the long end. The short
/// lines set how soon and how densely the first echoes arrive, which is
/// what makes a space sound small or large. The long lines are what give
/// the tail enough resonances per Hz to sound smooth instead of hollow, and
/// a small space needs those just as much.
const SHORTEST_LINE_MS: (f32, f32) = (12.0, 45.0);
const LONGEST_LINE_MS: (f32, f32) = (50.0, 145.0);

/// Nudges each line a few percent off the even spread, so that sums of
/// line lengths do not coincide.
const LINE_OFFSETS: [f32; LINES] = [
    0.000, 0.013, -0.021, 0.008, -0.011, 0.024, -0.004, 0.017, -0.026, 0.005, 0.019, -0.014, 0.010,
    -0.007, 0.022, 0.000,
];

/// Scale of the early reflection times at a size of 0 and at a size of 1.
const SCALE_RANGE: (f32, f32) = (0.25, 1.5);

/// Lengths in ms of the four allpass filters that smear the input of each
/// channel before it enters the network.
const DIFFUSER_MS: [[f32; 4]; 2] = [[4.77, 3.59, 12.73, 9.31], [5.03, 3.83, 13.19, 9.67]];

/// Arrival times in ms of the early reflections of each channel at a size
/// scale of 1, and their levels.
const EARLY_MS: [[f32; EARLY_TAPS]; 2] = [
    [7.1, 13.3, 21.7, 30.1, 43.3, 61.9],
    [8.9, 15.7, 19.9, 33.7, 47.9, 57.1],
];
const EARLY_GAINS: [f32; EARLY_TAPS] = [0.80, -0.65, 0.52, 0.42, -0.33, 0.25];
const EARLY_TAPS: usize = 6;

const MAX_PRE_DELAY_MS: f32 = 250.0;

/// Corner above which damping shortens the decay.
const DAMPING_HZ: f32 = 3_500.0;

/// Share of the mid decay time left at the top of the spectrum with
/// damping at 1.
const DAMPING_FLOOR: f32 = 0.1;

/// Swing of the modulated lines in ms with modulation at 1.
const MODULATION_MS: f32 = 0.35;

/// Slowest and fastest modulation rate in Hz. Each modulated line gets its
/// own rate in between, so they never line up.
const MODULATION_HZ: (f32, f32) = (0.31, 1.13);

/// Time the delay lengths take to glide to a new size or pre-delay.
const GLIDE_MS: f32 = 200.0;

/// Time the slowly moving controls take to cover 63% of a change.
const CONTROL_SMOOTHING_MS: f32 = 20.0;

/// Level of the signal fed into each line, of the tail and of the early
/// reflections. Set so that at the default settings a fully wet reverb is
/// about as loud as its input.
const INJECT_GAIN: f32 = 0.5;
const LATE_GAIN: f32 = 0.6;
const EARLY_GAIN: f32 = 0.4;

/// Decay time at which the tail gets no loudness correction.
const REFERENCE_DECAY_S: f32 = 1.8;

/// Settings of the [`Reverb`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ReverbParams {
    /// Size of the space, from a small room (0) to a large hall (1).
    /// Default 0.5. It sets how far apart the echoes are, not how long they
    /// last.
    pub size: f32,
    /// Time for the tail to fade by 60 dB (RT60), in seconds. 0.1 to 20,
    /// default 1.8.
    pub decay_s: f32,
    /// Gap between the dry sound and the start of the reverb, in ms. 0 to
    /// 250, default 10.
    pub pre_delay_ms: f32,
    /// How much faster the treble fades than the rest. At 0 all
    /// frequencies last the same; at 1 the top of the spectrum lasts a
    /// tenth as long. 0 to 1, default 0.5.
    pub damping: f32,
    /// How smeared the echoes are from the start. Low values let single
    /// echoes through at the beginning; high values give a smooth wash. 0
    /// to 1, default 0.8.
    pub diffusion: f32,
    /// Level of the first distinct reflections, which tell the ear how
    /// close the walls are. 0 to 1, default 0.5.
    pub early_level: f32,
    /// How much the tail slowly shimmers in pitch, which keeps it from
    /// ringing. 0 to 1, default 0.25.
    pub modulation: f32,
    /// Removes lows from what enters the reverb, in Hz. 20 to 1000,
    /// default 100.
    pub low_cut_hz: f32,
    /// Removes highs from what enters the reverb, in Hz. 1000 to 20000,
    /// default 10000.
    pub high_cut_hz: f32,
    /// Stereo width of the reverb, from mono (0) to full (1). Default 1.
    pub width: f32,
    /// Balance between the dry input (0) and the reverb (1). Default 0.3.
    /// Use 1 on a send track.
    pub mix: f32,
}

impl Default for ReverbParams {
    fn default() -> Self {
        Self {
            size: 0.5,
            decay_s: 1.8,
            pre_delay_ms: 10.0,
            damping: 0.5,
            diffusion: 0.8,
            early_level: 0.5,
            modulation: 0.25,
            low_cut_hz: 100.0,
            high_cut_hz: 10_000.0,
            width: 1.0,
            mix: 0.3,
        }
    }
}

param_set!(ReverbParams, "Reverb", {
    float [size] "size" "Size" { Fraction, Linear, 0.0, 1.0, 0.5 }
    float [decay_s] "decayS" "Decay" { Seconds, Logarithmic, 0.1, 20.0, 1.8 }
    float [pre_delay_ms] "preDelayMs" "Pre-delay" { Milliseconds, Linear, 0.0, 250.0, 10.0 }
    float [damping] "damping" "Damping" { Fraction, Linear, 0.0, 1.0, 0.5 }
    float [diffusion] "diffusion" "Diffusion" { Fraction, Linear, 0.0, 1.0, 0.8 }
    float [early_level] "earlyLevel" "Early reflections" { Fraction, Linear, 0.0, 1.0, 0.5 }
    float [modulation] "modulation" "Modulation" { Fraction, Linear, 0.0, 1.0, 0.25 }
    float [low_cut_hz] "lowCutHz" "Low cut" { Hertz, Logarithmic, 20.0, 1000.0, 100.0 }
    float [high_cut_hz] "highCutHz" "High cut" { Hertz, Logarithmic, 1000.0, 20_000.0, 10_000.0 }
    float [width] "width" "Width" { Fraction, Linear, 0.0, 1.0, 1.0 }
    float [mix] "mix" "Mix" { Fraction, Linear, 0.0, 1.0, 0.3 }
});

fn is_prime(candidate: usize) -> bool {
    if candidate < 4 {
        return candidate >= 2;
    }
    if candidate.is_multiple_of(2) {
        return false;
    }
    let mut divisor = 3;
    while divisor * divisor <= candidate {
        if candidate.is_multiple_of(divisor) {
            return false;
        }
        divisor += 2;
    }
    true
}

fn size_scale(size: f32) -> f32 {
    SCALE_RANGE.0 + (SCALE_RANGE.1 - SCALE_RANGE.0) * size
}

/// Loop time of line `index` in ms at a size.
fn line_ms(index: usize, size: f32) -> f32 {
    let between = |(small, large): (f32, f32)| small * (large / small).powf(size);
    let (shortest, longest) = (between(SHORTEST_LINE_MS), between(LONGEST_LINE_MS));
    let position = index as f32 / (LINES - 1) as f32;
    shortest * (longest / shortest).powf(position) * (1.0 + LINE_OFFSETS[index])
}

/// The length of every line in samples: its time at this size, moved up to
/// the next prime number no shorter line has taken. Lengths with no common
/// factor never pile their echoes onto the same instants, which is what
/// would make a tail sound metallic.
fn line_lengths(size: f32, sample_rate: f32) -> [usize; LINES] {
    let mut lengths = [0; LINES];
    let mut previous = 16;
    for (index, length) in lengths.iter_mut().enumerate() {
        let time_ms = line_ms(index, size);
        let mut candidate =
            ((time_ms * 0.001 * sample_rate).round() as usize).max(previous + 1) | 1;
        while !is_prime(candidate) {
            candidate += 2;
        }
        *length = candidate;
        previous = candidate;
    }
    lengths
}

/// Feedback gain that makes a loop of `length` samples fade by 60 dB in
/// `decay_s` seconds.
fn decay_gain(length: f32, decay_s: f32, sample_rate: f32) -> f32 {
    (-3.0 * std::f32::consts::LN_10 * length / (decay_s * sample_rate)).exp()
}

/// A smooth wave between -1 and 1 for a phase from 0 to 1: two parabolas,
/// close to a sine and much cheaper.
#[inline]
fn parabolic_sine(phase: f32) -> f32 {
    let x = 2.0 * phase - 1.0;
    -4.0 * x * (1.0 - x.abs())
}

/// Mixes sixteen signals with a Hadamard matrix, scaled so that the total
/// energy is unchanged. Every output is a different sum and difference of
/// all sixteen inputs, which spreads each echo over every line.
#[inline]
fn hadamard(values: &mut [f32; LINES]) {
    let mut span = 1;
    while span < LINES {
        let mut start = 0;
        while start < LINES {
            for index in start..start + span {
                let (a, b) = (values[index], values[index + span]);
                values[index] = a + b;
                values[index + span] = a - b;
            }
            start += 2 * span;
        }
        span *= 2;
    }
    for value in values.iter_mut() {
        *value *= 0.25;
    }
}

/// A Schroeder allpass: passes every frequency at the same level but
/// smears it in time.
struct Allpass {
    line: DelayLine,
    length: usize,
}

impl Allpass {
    fn new(time_ms: f32, sample_rate: f32) -> Self {
        let length = ms_to_samples(time_ms, sample_rate) as usize;
        Self {
            line: DelayLine::new(length),
            length,
        }
    }

    #[inline]
    fn tick(&mut self, input: f32, coefficient: f32) -> f32 {
        let delayed = self.line.tap(self.length);
        let fed = flush(input - coefficient * delayed);
        self.line.push(fed);
        delayed + coefficient * fed
    }
}

/// One delay line of the feedback network with the filter that sets its
/// decay.
struct Line {
    delay: DelayLine,
    /// Loop length in samples. It glides when the size changes.
    length: LinearRamp,
    damping: OnePoleFilter,
    /// Feedback gain at low and at high frequencies, moving toward their
    /// targets.
    gain_low: f32,
    gain_high: f32,
    target_low: f32,
    target_high: f32,
    /// Position and speed of the line's modulation, in cycles.
    phase: f32,
    rate: f32,
    modulated: bool,
}

/// What the input section needs per channel.
struct InputChannel {
    pre_delay: DelayLine,
    low_cut: Svf,
    high_cut: Svf,
    early: DelayLine,
    diffusers: [Allpass; 4],
}

impl InputChannel {
    fn new(channel: usize, sample_rate: f32) -> Self {
        let early_ms = EARLY_MS[channel][EARLY_TAPS - 1] * SCALE_RANGE.1;
        Self {
            pre_delay: DelayLine::new(ms_to_samples(MAX_PRE_DELAY_MS, sample_rate) as usize + 2),
            low_cut: Svf::default(),
            high_cut: Svf::default(),
            early: DelayLine::new(ms_to_samples(early_ms, sample_rate) as usize + 4),
            diffusers: DIFFUSER_MS[channel].map(|time_ms| Allpass::new(time_ms, sample_rate)),
        }
    }

    fn clear(&mut self) {
        self.pre_delay.clear();
        self.low_cut.reset();
        self.high_cut.reset();
        self.early.clear();
        for diffuser in &mut self.diffusers {
            diffuser.line.clear();
        }
    }
}

/// Values worked out from the parameters at the control rate.
#[derive(Clone, Copy)]
struct Derived {
    low_cut: SvfCoeffs,
    high_cut: SvfCoeffs,
    /// Coefficients of the first two and the last two diffusers.
    diffusion: [f32; 2],
    damping: f32,
    modulation_samples: f32,
    late_gain: f32,
}

/// An algorithmic stereo reverb built on a feedback delay network.
///
/// Sixteen delay lines feed back into each other through a Hadamard
/// matrix, which loses no energy, so the only thing that makes the sound
/// die away is one small filter per line. That filter is set from the
/// decay time and the line's length, which is why the decay control reads
/// in seconds of RT60 and means it (Jot and Chaigne, "Digital delay
/// networks for designing artificial reverberators", AES 1991). A network
/// was chosen over a plate topology for that reason: its decay time, and
/// how much faster the treble decays, are set directly instead of being
/// tuned by ear.
///
/// The line lengths are distinct primes and half the lines are slowly
/// modulated, which together keep the tail free of metallic ringing. The
/// input passes through four allpass diffusers per channel, so the tail is
/// dense from its first moments, and a set of early reflections is taken
/// from the input directly. The left input feeds half the lines and the
/// right input the other half, and each output listens to its own half.
///
/// It adds no latency.
pub struct Reverb {
    sample_rate: f32,
    params: ReverbParams,
    /// The parameters on their way to `params`.
    current: ReverbParams,
    settling: bool,
    derived: Derived,
    inputs: [InputChannel; 2],
    lines: [Line; LINES],
    /// Pre-delay in samples, and the size scale the early reflections use.
    pre_delay: LinearRamp,
    early_scale: LinearRamp,
    early_level: LinearRamp,
    width: LinearRamp,
    mix: LinearRamp,
    until_control: usize,
    amount: f32,
    fresh: bool,
}

impl Default for Reverb {
    fn default() -> Self {
        Self::with_rate(48_000.0, false)
    }
}

impl Reverb {
    /// Builds the reverb for a sample rate. Without `allocate` every delay
    /// line is a stub, which is all an unprepared reverb needs.
    fn with_rate(sample_rate: f32, allocate: bool) -> Self {
        let params = ReverbParams::default();
        let longest = if allocate {
            let top = LONGEST_LINE_MS.1 + MODULATION_MS;
            // Rounding a length up to a free prime moves it by far less
            // than this.
            ms_to_samples(top, sample_rate) as usize + 512
        } else {
            0
        };
        let mut modulated = 0;
        let lines = std::array::from_fn(|index| {
            let is_modulated = (index / 2) % 2 == 1;
            let spread = modulated as f32 / (LINES / 2 - 1) as f32;
            if is_modulated {
                modulated += 1;
            }
            Line {
                delay: DelayLine::new(longest),
                length: LinearRamp::new(32.0),
                damping: OnePoleFilter::default(),
                gain_low: 0.0,
                gain_high: 0.0,
                target_low: 0.0,
                target_high: 0.0,
                phase: spread,
                rate: (MODULATION_HZ.0 + (MODULATION_HZ.1 - MODULATION_HZ.0) * spread)
                    / sample_rate,
                modulated: is_modulated,
            }
        });
        let inputs = if allocate {
            [
                InputChannel::new(0, sample_rate),
                InputChannel::new(1, sample_rate),
            ]
        } else {
            std::array::from_fn(|_| InputChannel {
                pre_delay: DelayLine::default(),
                low_cut: Svf::default(),
                high_cut: Svf::default(),
                early: DelayLine::default(),
                diffusers: std::array::from_fn(|_| Allpass {
                    line: DelayLine::default(),
                    length: 1,
                }),
            })
        };
        let mut reverb = Self {
            sample_rate,
            params,
            current: params,
            settling: false,
            derived: Derived {
                low_cut: SvfCoeffs::new(100.0, 0.707, sample_rate),
                high_cut: SvfCoeffs::new(10_000.0, 0.707, sample_rate),
                diffusion: [0.0; 2],
                damping: 0.0,
                modulation_samples: 0.0,
                late_gain: 0.0,
            },
            inputs,
            lines,
            pre_delay: LinearRamp::new(0.0),
            early_scale: LinearRamp::new(1.0),
            early_level: LinearRamp::new(0.0),
            width: LinearRamp::new(1.0),
            mix: LinearRamp::new(0.0),
            until_control: 0,
            amount: 1.0,
            fresh: true,
        };
        reverb.amount =
            smoothing_coefficient(CONTROL_SMOOTHING_MS, sample_rate / CONTROL_PERIOD as f32);
        reverb.apply(true);
        reverb
    }

    /// Takes `self.params` as the new targets. `retune` says the size may
    /// have changed, so the line lengths have to be worked out again.
    fn apply(&mut self, retune: bool) {
        let (params, rate) = (self.params, self.sample_rate);
        let ramp = if self.fresh {
            0
        } else {
            ms_to_samples(SMOOTHING_MS, rate)
        };
        let glide = if self.fresh {
            0
        } else {
            ms_to_samples(GLIDE_MS, rate)
        };
        self.mix.set_target(params.mix, ramp);
        self.width.set_target(params.width, ramp);
        self.early_level
            .set_target(params.early_level * EARLY_GAIN, ramp);
        // Whole samples, so that once the glide is over the pre-delay is
        // read without interpolation.
        let pre_delay = (params.pre_delay_ms * 0.001 * rate)
            .round()
            .min(self.inputs[0].pre_delay.max_delay() as f32 - 2.0)
            .max(0.0);
        self.pre_delay.set_target(pre_delay, glide);
        self.early_scale.set_target(size_scale(params.size), glide);
        if retune {
            let lengths = line_lengths(params.size, rate);
            for (line, length) in self.lines.iter_mut().zip(lengths) {
                let longest = line.delay.max_delay().saturating_sub(64).max(8);
                line.length.set_target(length.min(longest) as f32, glide);
            }
        }
        if self.fresh {
            self.current = params;
            self.settling = false;
            self.derive();
            for line in &mut self.lines {
                line.gain_low = line.target_low;
                line.gain_high = line.target_high;
            }
        } else {
            self.settling = true;
        }
    }

    /// Works out everything that follows from the smoothed parameters.
    fn derive(&mut self) {
        let (current, rate) = (self.current, self.sample_rate);
        let butterworth = std::f32::consts::FRAC_1_SQRT_2;
        let high_decay = current.decay_s * (1.0 - (1.0 - DAMPING_FLOOR) * current.damping);
        for line in &mut self.lines {
            // The gains follow the length the line is heading for, so the
            // decay time is right as soon as a size change has finished.
            let length = line.length.target();
            line.target_low = decay_gain(length, current.decay_s, rate);
            line.target_high = decay_gain(length, high_decay, rate);
        }
        self.derived = Derived {
            low_cut: SvfCoeffs::new(current.low_cut_hz, butterworth, rate),
            high_cut: SvfCoeffs::new(current.high_cut_hz, butterworth, rate),
            diffusion: [0.75 * current.diffusion, 0.625 * current.diffusion],
            damping: OnePoleFilter::coefficient(DAMPING_HZ, rate),
            modulation_samples: current.modulation * MODULATION_MS * 0.001 * rate,
            // A longer tail holds more energy. Taking back part of that
            // keeps the loudness of long and short settings within a few
            // dB of each other.
            late_gain: LATE_GAIN * (REFERENCE_DECAY_S / current.decay_s).powf(0.25),
        };
    }

    fn control(&mut self) {
        if self.settling {
            self.settling = self.current.approach(&self.params, self.amount);
            self.derive();
        }
        let amount = self.amount;
        for line in &mut self.lines {
            line.gain_low += (line.target_low - line.gain_low) * amount;
            line.gain_high += (line.target_high - line.gain_high) * amount;
        }
        for input in &mut self.inputs {
            input.low_cut.flush();
            input.high_cut.flush();
        }
    }

    /// Runs one stereo sample through the reverb and returns the wet
    /// signal.
    #[inline]
    fn tick(&mut self, left: f32, right: f32) -> (f32, f32) {
        let derived = self.derived;
        let pre_delay = self.pre_delay.tick();
        let pre_delay_settled = self.pre_delay.is_settled();
        let early_scale = self.early_scale.tick() * 0.001 * self.sample_rate;

        let mut fed = [0.0_f32; 2];
        let mut early = [0.0_f32; 2];
        for (channel, input) in self.inputs.iter_mut().enumerate() {
            input
                .pre_delay
                .push(if channel == 0 { left } else { right });
            let delayed = if pre_delay_settled {
                input.pre_delay.tap(pre_delay as usize + 1)
            } else {
                input.pre_delay.tap_linear(pre_delay + 1.0)
            };
            let filtered = input.low_cut.tick(&derived.low_cut, delayed).high;
            let filtered = input.high_cut.tick(&derived.high_cut, filtered).low;

            input.early.push(filtered);
            for (time_ms, gain) in EARLY_MS[channel].iter().zip(EARLY_GAINS) {
                early[channel] += gain * input.early.tap_linear(1.0 + time_ms * early_scale);
            }

            let mut diffused = filtered;
            for (index, diffuser) in input.diffusers.iter_mut().enumerate() {
                diffused = diffuser.tick(diffused, derived.diffusion[index / 2]);
            }
            fed[channel] = diffused * INJECT_GAIN;
        }

        let mut feedback = [0.0_f32; LINES];
        let mut late = [0.0_f32; 2];
        for (index, line) in self.lines.iter_mut().enumerate() {
            let length = line.length.tick();
            let output = if line.modulated {
                line.phase += line.rate;
                if line.phase >= 1.0 {
                    line.phase -= 1.0;
                }
                let swing = derived.modulation_samples * parabolic_sine(line.phase);
                line.delay.tap_cubic(length + swing)
            } else if line.length.is_settled() {
                line.delay.tap(length as usize)
            } else {
                line.delay.tap_cubic(length)
            };
            // Even lines belong to the left output, odd lines to the
            // right. Alternating signs keep the sum from favouring any
            // one frequency.
            let sign = if (index / 2) % 2 == 0 { 1.0 } else { -1.0 };
            late[index % 2] += sign * output;
            let low = line.damping.low_pass(derived.damping, output);
            feedback[index] = line.gain_high * output + (line.gain_low - line.gain_high) * low;
        }
        hadamard(&mut feedback);
        for (index, line) in self.lines.iter_mut().enumerate() {
            let sign = if (index / 4) % 2 == 0 { 1.0 } else { -1.0 };
            line.delay
                .push(flush(feedback[index] + sign * fed[index % 2]));
            line.damping.flush();
        }

        let early_level = self.early_level.tick();
        (
            late[0] * derived.late_gain + early[0] * early_level,
            late[1] * derived.late_gain + early[1] * early_level,
        )
    }
}

impl Effect for Reverb {
    type Params = ReverbParams;

    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        let params = self.params;
        *self = Self::with_rate(sample_rate.max(1.0), true);
        self.params = params;
        self.apply(true);
    }

    fn reset(&mut self) {
        self.fresh = true;
        for input in &mut self.inputs {
            input.clear();
        }
        let mut modulated = 0;
        for line in &mut self.lines {
            line.delay.clear();
            line.damping.reset();
            if line.modulated {
                line.phase = modulated as f32 / (LINES / 2 - 1) as f32;
                modulated += 1;
            }
        }
        self.until_control = 0;
        self.apply(true);
    }

    fn set_params(&mut self, params: &ReverbParams) {
        let params = params.sanitized();
        let retune = params.size != self.params.size;
        self.params = params;
        self.apply(retune);
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.fresh = false;
        for (left, right) in left.iter_mut().zip(right.iter_mut()) {
            if self.until_control == 0 {
                self.control();
                self.until_control = CONTROL_PERIOD;
            }
            self.until_control -= 1;

            let (wet_left, wet_right) = self.tick(*left, *right);
            let width = self.width.tick();
            let mid = 0.5 * (wet_left + wet_right);
            let side = 0.5 * (wet_left - wet_right) * width;
            let mix = self.mix.tick();
            *left += (mid + side - *left) * mix;
            *right += (mid - side - *right) * mix;
        }
    }

    fn tail_samples(&self) -> usize {
        let params = &self.params;
        let early_ms = EARLY_MS[1][EARLY_TAPS - 1] * SCALE_RANGE.1;
        // The decay time is to 60 dB down. Half as long again reaches 90.
        let seconds =
            1.5 * params.decay_s + 0.001 * (params.pre_delay_ms + early_ms + LONGEST_LINE_MS.1);
        (seconds * self.sample_rate).ceil() as usize
    }

    fn gap_samples(&self) -> usize {
        let early_ms = EARLY_MS[1][EARLY_TAPS - 1] * SCALE_RANGE.1;
        // Once the tail has begun it only dies away. What can still be on
        // its way is the pre-delay, the early reflections and one trip
        // round the longest line.
        let seconds = 0.001 * (self.params.pre_delay_ms + early_ms + LONGEST_LINE_MS.1);
        (seconds * self.sample_rate).ceil() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_lengths_are_distinct_primes_at_every_size_and_rate() {
        for rate in [22_050.0, 44_100.0, 48_000.0, 88_200.0, 96_000.0, 192_000.0] {
            for step in 0..=20 {
                let lengths = line_lengths(step as f32 / 20.0, rate);
                for (index, length) in lengths.iter().enumerate() {
                    assert!(is_prime(*length), "{length} at {rate}");
                    if index > 0 {
                        assert!(*length > lengths[index - 1]);
                    }
                }
                // Rounding to a prime barely moves a line.
                for (index, length) in lengths.iter().enumerate() {
                    let ideal = line_ms(index, step as f32 / 20.0) * 0.001 * rate;
                    assert!((*length as f32 - ideal).abs() < 0.02 * ideal + 40.0);
                }
            }
        }
    }

    #[test]
    fn primes_are_recognised() {
        let primes: Vec<usize> = (0..60).filter(|n| is_prime(*n)).collect();
        assert_eq!(
            primes,
            [
                2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59
            ]
        );
        assert!(is_prime(7_919) && !is_prime(7_917));
    }

    #[test]
    fn hadamard_keeps_energy_and_undoes_itself() {
        let mut values: [f32; LINES] = std::array::from_fn(|index| (index as f32 * 0.37).sin());
        let original = values;
        let energy: f32 = values.iter().map(|value| value * value).sum();
        hadamard(&mut values);
        let mixed: f32 = values.iter().map(|value| value * value).sum();
        assert!((mixed - energy).abs() < 1e-4);
        hadamard(&mut values);
        for (value, original) in values.iter().zip(original) {
            assert!((value - original).abs() < 1e-5);
        }
    }

    #[test]
    fn decay_gain_gives_sixty_decibels_in_the_decay_time() {
        let gain = decay_gain(1_000.0, 2.0, 48_000.0);
        let passes = 2.0 * 48_000.0 / 1_000.0;
        let total_db = 20.0 * gain.powf(passes).log10();
        assert!((total_db + 60.0).abs() < 0.01, "{total_db}");
    }

    #[test]
    fn modulation_wave_is_smooth_and_bounded() {
        let mut previous = parabolic_sine(0.0);
        for step in 1..=1_000 {
            let value = parabolic_sine(step as f32 / 1_000.0);
            assert!((-1.0..=1.0).contains(&value));
            assert!((value - previous).abs() < 0.01);
            previous = value;
        }
        assert!((parabolic_sine(0.25) - 1.0).abs() < 1e-6);
        assert!((parabolic_sine(0.75) + 1.0).abs() < 1e-6);
    }

    #[test]
    fn an_allpass_keeps_the_energy_of_an_impulse() {
        let mut allpass = Allpass::new(3.0, 48_000.0);
        let mut energy = 0.0_f64;
        for n in 0..48_000 {
            let output = allpass.tick(if n == 0 { 1.0 } else { 0.0 }, 0.7);
            energy += f64::from(output * output);
        }
        assert!((energy - 1.0).abs() < 1e-3, "{energy}");
    }
}
