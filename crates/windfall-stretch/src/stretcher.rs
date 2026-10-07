//! The streaming stretcher.
//!
//! The spectral processing is a port of Signalsmith Stretch (MIT, see
//! `LICENSE-THIRD-PARTY`). What surrounds it is Windfall's own: the input
//! clock, which makes the output independent of how a host divides the
//! audio into blocks, the handling of silence, and seeking.

use crate::fft::{Complex32, ZERO};
use crate::stft::Stft;

/// The shortest a sound can be made: a quarter of its length.
pub const MIN_TIME_RATIO: f64 = 0.25;

/// The longest a sound can be made: four times its length.
pub const MAX_TIME_RATIO: f64 = 4.0;

/// The furthest a sound can be transposed, up or down, in semitones.
pub const MAX_PITCH_SEMITONES: f64 = 24.0;

/// Time a change of the time ratio or the pitch takes to arrive.
const GLIDE_SECONDS: f64 = 0.5;

/// The time-stretch beyond which the phase steps between neighbouring bins
/// are no longer scaled, but scattered around this value. Scaling them
/// further turns one transient into a row of separate ones.
const MAX_CLEAN_STRETCH: f32 = 2.0;

/// Added to a bin's energy before dividing by it. A full-scale sine is a
/// bin of energy 256.
const ENERGY_FLOOR: f32 = 1e-19;

/// A predicted phase with less power than this is too weak to trust, and
/// the bin takes the phase of the input. It is the product of three bins,
/// squared, which puts it at a bin 118 dB below full scale.
const PREDICTION_FLOOR: f32 = 1e-27;

/// How much of the main lobe around a peak moves as one piece when the
/// pitch is shifted.
const LOBE_SHARE: f32 = 1.0;

/// Steps in the table of turns for places between two bins.
const TURN_STEPS: usize = 1_024;

/// A sample below this, 200 dB under full scale, counts as silence.
const SILENCE: f32 = 1e-10;

/// How much computing the stretcher spends on how good a result.
///
/// The stretcher works on overlapping blocks of sound. A longer block
/// tells notes that lie close together apart, and blocks that come more
/// often cost more and follow a changing sound more closely. The crate
/// documentation has what each preset measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Quality {
    /// Blocks of 100 ms every 15 ms. The cheapest, for many clips at once
    /// and for modest changes of length, up to half as long again.
    Fast,
    /// Blocks of 120 ms every 15 ms.
    #[default]
    Standard,
    /// Blocks of 120 ms every 10 ms. More frequent updates than
    /// [`Quality::Standard`], and at home with low chords and long
    /// stretches. For rendering, and for the clips that matter.
    High,
}

impl Quality {
    /// All presets, cheapest first.
    pub const ALL: [Quality; 3] = [Quality::Fast, Quality::Standard, Quality::High];

    /// Length of a block in seconds.
    pub fn block_seconds(self) -> f64 {
        match self {
            Quality::Fast => 0.1,
            Quality::Standard => 0.12,
            Quality::High => 0.12,
        }
    }

    /// Seconds of output between two blocks.
    pub fn interval_seconds(self) -> f64 {
        match self {
            Quality::Fast => 0.015,
            Quality::Standard => 0.015,
            Quality::High => 0.01,
        }
    }

    /// Block length and interval in frames at `sample_rate`.
    fn frames(self, sample_rate: u32) -> (usize, usize) {
        let rate = f64::from(sample_rate);
        let interval = ((rate * self.interval_seconds()) as usize).max(4);
        let block = ((rate * self.block_seconds()) as usize).max(2 * interval);
        (block, interval)
    }
}

/// How far a [`Stretcher`]'s output lags its input, in two parts.
///
/// The stretcher works on the moment `input` frames before the newest
/// input frame it has been given, and what it makes of that moment comes
/// out `output` frames later. An output frame is therefore
/// [`Latency::output_frames`] behind the input it was made from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Latency {
    /// Input frames the stretcher looks ahead.
    pub input: usize,
    /// Output frames it takes to put out what it has worked out.
    pub output: usize,
}

impl Latency {
    /// The whole latency in output frames at a steady `time_ratio`:
    /// `output + input * time_ratio`. This is how many frames of a fresh
    /// stretcher's output come before the first frame of its input.
    pub fn output_frames(self, time_ratio: f64) -> f64 {
        self.output as f64 + self.input as f64 * time_ratio
    }

    /// The whole latency in input frames at a steady `time_ratio`:
    /// `input + output / time_ratio`. This is how far past the frame that
    /// is being heard the input has to have been fed.
    pub fn input_frames(self, time_ratio: f64) -> f64 {
        self.input as f64 + self.output as f64 / time_ratio
    }
}

/// A value that moves to its target in a straight line over a fixed number
/// of frames.
#[derive(Debug, Clone, Copy)]
struct Glide {
    value: f64,
    target: f64,
    step: f64,
    remaining: u32,
}

impl Glide {
    fn at(value: f64) -> Self {
        Self {
            value,
            target: value,
            step: 0.0,
            remaining: 0,
        }
    }

    fn set_target(&mut self, target: f64, frames: u32) {
        if target == self.target {
            return;
        }
        self.target = target;
        self.step = (target - self.value) / f64::from(frames.max(1));
        self.remaining = frames.max(1);
    }

    fn snap(&mut self) {
        *self = Self::at(self.target);
    }

    #[inline]
    fn tick(&mut self) -> f64 {
        if self.remaining > 0 {
            self.remaining -= 1;
            self.value = if self.remaining == 0 {
                self.target
            } else {
                self.value + self.step
            };
        }
        self.value
    }
}

/// The input clock: how many input frames each output frame uses up.
#[derive(Debug, Clone, Copy)]
struct Clock {
    /// Input frames per output frame.
    rate: Glide,
    /// Input the output has moved past but not taken yet, from -0.5 to
    /// 0.5 of a frame.
    owed: f64,
}

impl Clock {
    /// Moves one output frame on and returns how many input frames that
    /// takes.
    ///
    /// The clock moves frame by frame even where the rate is steady, so
    /// that it lands on the same numbers however the output is divided.
    #[inline]
    fn tick(&mut self) -> usize {
        self.owed += self.rate.tick();
        let take = (self.owed + 0.5).floor();
        self.owed -= take;
        take as usize
    }

    fn advance(&mut self, frames: usize) -> usize {
        (0..frames).map(|_| self.tick()).sum()
    }
}

/// Where a bin of the output comes from in the input.
#[derive(Debug, Clone, Copy)]
struct MapPoint {
    /// The input bin, which may lie between two bins.
    input_bin: f32,
    /// How many input bins one output bin covers here.
    gradient: f32,
}

/// A peak of the input spectrum and the output bin it moves to.
#[derive(Debug, Clone, Copy)]
struct Peak {
    input: f32,
    output: f32,
}

#[inline]
fn band<T: Copy + Default>(values: &[T], index: isize) -> T {
    if index >= 0 && (index as usize) < values.len() {
        values[index as usize]
    } else {
        T::default()
    }
}

/// The value at a place between two bins, in a straight line from one to
/// the other. Past either end of the spectrum there is nothing.
#[inline]
fn between(values: &[Complex32], low: isize, fraction: f32) -> Complex32 {
    let (below, above) = (band(values, low), band(values, low + 1));
    below + (above - below) * fraction
}

#[inline]
fn at(values: &[Complex32], index: f32) -> Complex32 {
    let low = index.floor();
    between(values, low as isize, index - low)
}

/// Gives a bin its energy and the phase of `phase`. Where the power of
/// `phase` is at or below `floor`, too weak to mean anything, the input's
/// own phase is used.
#[inline]
fn with_energy(phase: Complex32, energy: f32, input: Complex32, floor: f32) -> Complex32 {
    let power = phase.norm_sqr();
    if power <= floor {
        input * (energy / (input.norm_sqr() + ENERGY_FLOOR)).sqrt()
    } else {
        phase * (energy / power).sqrt()
    }
}

/// A time-stretcher and pitch-shifter for a stream of audio.
///
/// See the crate documentation for how it works and what it costs. In
/// short: [`Stretcher::new`] and [`Stretcher::prepare`] allocate, and
/// nothing else does. Every other method is safe on a realtime thread.
/// Dropping a stretcher frees memory, so a host sends it to another thread
/// to be dropped.
///
/// # Input and output
///
/// [`Stretcher::process`] fills the output it is given and takes as much
/// input as that needs: `1 / time_ratio` input frames for every output
/// frame, counted by a clock that carries the fraction over, so that over
/// any stretch of `n` output frames at a steady ratio it takes
/// `n / time_ratio` input frames, give or take one.
/// [`Stretcher::input_frames_needed`] says exactly how many the next call
/// will take. The output does not depend on how the calls divide it.
///
/// # Latency
///
/// Output lags input by [`Stretcher::latency`]. A fresh or reset stretcher
/// behaves as if it had been fed silence for ever, so the first frame of
/// its input comes out [`Latency::output_frames`] frames into its output.
pub struct Stretcher {
    channels: usize,
    sample_rate: u32,
    quality: Quality,
    stft: Stft,
    bins: usize,
    /// Output frames a glide takes.
    glide_frames: u32,

    clock: Clock,
    /// The time ratio the clock's rate is gliding to.
    time_ratio: f64,
    /// Transposition in semitones.
    pitch: Glide,
    formants: bool,

    /// Output frames since the last block.
    since_block: usize,
    /// Input frames taken since the last block.
    taken_since_block: usize,
    /// Whether `previous` holds the spectrum one block back.
    has_previous: bool,
    /// Input frames since the last one that was not silent.
    silent_run: usize,
    /// Whether the spectra hold nothing, as after a reset.
    cleared: bool,
    random: u32,

    // One spectrum per channel, one after the other.
    input: Vec<Complex32>,
    previous: Vec<Complex32>,
    output: Vec<Complex32>,
    input_energy: Vec<f32>,
    predicted_energy: Vec<f32>,
    predicted_input: Vec<Complex32>,

    // One value per bin.
    /// The turn of each bin's centre frequency over one interval.
    rotation: Vec<Complex32>,
    /// The turn back over one interval of a frequency that many
    /// [`TURN_STEPS`]ths of a bin wide.
    part_turn: Vec<Complex32>,
    energy: Vec<f32>,
    smoothed_energy: Vec<f32>,
    map: Vec<MapPoint>,
    old_map: Vec<MapPoint>,
    old_output: Vec<Complex32>,
    old_energy: Vec<f32>,
    peaks: Vec<Peak>,
    /// The spectral envelope, with two spare values past the top bin.
    envelope: Vec<f32>,
    /// A slow estimate of the fundamental, as a bin, and its weight.
    fundamental_weighted: f32,
    fundamental_weight: f32,
}

impl Stretcher {
    /// A stretcher for `channels` channels at `sample_rate`, at
    /// [`Quality::Standard`], ready to process.
    pub fn new(channels: usize, sample_rate: u32) -> Self {
        Self::with_quality(channels, sample_rate, Quality::Standard)
    }

    /// A stretcher at the given quality, ready to process.
    pub fn with_quality(channels: usize, sample_rate: u32, quality: Quality) -> Self {
        assert!(channels >= 1, "a stretcher needs at least one channel");
        assert!(sample_rate > 0, "sample rate must be positive");
        let (block, interval) = quality.frames(sample_rate);
        let stft = Stft::new(channels, block, interval);
        let bins = stft.bins();
        let size = stft.fft_size() as f64;
        let turn = |bins: f64| {
            let angle = (bins * interval as f64 / size).fract() * std::f64::consts::TAU;
            Complex32::new(angle.cos() as f32, angle.sin() as f32)
        };
        let rotation = (0..bins).map(|bin| turn(bin as f64 + 0.5)).collect();
        let part_turn = (0..=TURN_STEPS)
            .map(|step| turn(step as f64 / TURN_STEPS as f64).conj())
            .collect();
        Self {
            channels,
            sample_rate,
            quality,
            bins,
            glide_frames: ((f64::from(sample_rate) * GLIDE_SECONDS) as u32).max(1),
            clock: Clock {
                rate: Glide::at(1.0),
                owed: 0.0,
            },
            time_ratio: 1.0,
            pitch: Glide::at(0.0),
            formants: false,
            since_block: interval,
            taken_since_block: 0,
            has_previous: false,
            silent_run: stft.input_len(),
            cleared: true,
            random: RANDOM_SEED,
            input: vec![ZERO; bins * channels],
            previous: vec![ZERO; bins * channels],
            output: vec![ZERO; bins * channels],
            input_energy: vec![0.0; bins * channels],
            predicted_energy: vec![0.0; bins * channels],
            predicted_input: vec![ZERO; bins * channels],
            rotation,
            part_turn,
            energy: vec![0.0; bins],
            smoothed_energy: vec![0.0; bins],
            map: vec![
                MapPoint {
                    input_bin: 0.0,
                    gradient: 1.0,
                };
                bins
            ],
            old_map: vec![
                MapPoint {
                    input_bin: 0.0,
                    gradient: 1.0
                };
                bins
            ],
            old_output: vec![ZERO; bins * channels],
            old_energy: vec![0.0; bins * channels],
            peaks: Vec::with_capacity(bins / 2 + 1),
            envelope: vec![0.0; bins + 2],
            fundamental_weighted: 0.0,
            fundamental_weight: 0.0,
            stft,
        }
    }

    /// Sets the stretcher up anew for another format or quality. The time
    /// ratio, the pitch and the formant setting carry over, in effect at
    /// once. Allocates.
    pub fn prepare(&mut self, channels: usize, sample_rate: u32, quality: Quality) {
        let mut fresh = Self::with_quality(channels, sample_rate, quality);
        fresh.set_time_ratio(self.time_ratio);
        fresh.set_pitch_semitones(self.pitch.target);
        fresh.formants = self.formants;
        fresh.reset();
        *self = fresh;
    }

    /// Number of prepared channels.
    pub fn channels(&self) -> usize {
        self.channels
    }

    /// Prepared sample rate in Hz.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Prepared quality preset.
    pub fn quality(&self) -> Quality {
        self.quality
    }

    /// Sets how long the output is for a given input: 2 plays at half
    /// speed, 0.5 at double speed, and the pitch stays where it is. Forced
    /// into [`MIN_TIME_RATIO`] to [`MAX_TIME_RATIO`]. A change glides in
    /// over 50 ms of output.
    pub fn set_time_ratio(&mut self, ratio: f64) {
        let ratio = if ratio.is_finite() { ratio } else { 1.0 };
        self.time_ratio = ratio.clamp(MIN_TIME_RATIO, MAX_TIME_RATIO);
        self.clock
            .rate
            .set_target(1.0 / self.time_ratio, (self.sample_rate / 20).max(1));
    }

    /// The time ratio last set.
    pub fn time_ratio(&self) -> f64 {
        self.time_ratio
    }

    /// Sets the transposition in semitones, which leaves the length alone.
    /// Forced into plus or minus [`MAX_PITCH_SEMITONES`]. A change glides
    /// in over 500 ms of output to keep overlapping spectra coherent.
    pub fn set_pitch_semitones(&mut self, semitones: f64) {
        let semitones = if semitones.is_finite() {
            semitones
        } else {
            0.0
        };
        self.pitch.set_target(
            semitones.clamp(-MAX_PITCH_SEMITONES, MAX_PITCH_SEMITONES),
            self.glide_frames,
        );
    }

    /// The transposition last set, in semitones.
    pub fn pitch_semitones(&self) -> f64 {
        self.pitch.target
    }

    /// Chooses whether a transposed sound keeps the broad shape of its
    /// spectrum, which is what keeps a transposed voice from sounding
    /// like a smaller or larger person. Off by default. It matters only
    /// while the pitch is not zero.
    pub fn set_formant_preservation(&mut self, preserve: bool) {
        self.formants = preserve;
    }

    /// Whether approximate spectral-envelope correction is enabled.
    pub fn formant_preservation(&self) -> bool {
        self.formants
    }

    /// How far the output lags the input. It depends on the sample rate
    /// and the quality alone.
    pub fn latency(&self) -> Latency {
        Latency {
            input: self.stft.analysis_latency(),
            output: self.stft.synthesis_latency(),
        }
    }

    /// Exactly how many input frames the next [`Stretcher::process`] call
    /// takes if it is asked for `output_frames` frames. It walks the
    /// clock, so it costs a few operations per frame.
    pub fn input_frames_needed(&self, output_frames: usize) -> usize {
        let mut clock = self.clock;
        clock.advance(output_frames)
    }

    /// Forgets everything heard and jumps to the time ratio and pitch
    /// last set. Afterwards the stretcher is as if it had been fed
    /// silence for ever.
    pub fn reset(&mut self) {
        self.stft.reset();
        self.clock.rate.snap();
        self.clock.owed = 0.0;
        self.pitch.snap();
        self.since_block = self.stft.interval();
        self.taken_since_block = 0;
        self.silent_run = self.stft.input_len();
        self.random = RANDOM_SEED;
        self.clear_spectra();
    }

    /// Input frames of pre-roll [`Stretcher::seek`] makes full use of at
    /// the time ratio last set.
    pub fn seek_frames(&self) -> usize {
        let run = self.seek_blocks() * self.stft.interval();
        (run as f64 / self.time_ratio).ceil() as usize + self.stft.input_len()
    }

    /// Starts over in the middle of a sound: resets, then runs
    /// `pre_roll`, the input that comes right before the input the next
    /// [`Stretcher::process`] call is given, through the stretcher without
    /// putting anything out.
    ///
    /// The output that follows is what playing through that place would
    /// have given, to within what the crate documentation states, as long
    /// as `pre_roll` holds [`Stretcher::seek_frames`] frames. With fewer
    /// frames what is missing counts as silence, which is right at the
    /// start of a file; more are ignored. The time ratio and pitch last
    /// set apply from the first frame.
    ///
    /// As after any input, the frame that comes out next lies
    /// [`Latency::input_frames`] before the end of `pre_roll`. To hear
    /// frame `s` of a file first, end the pre-roll that much after `s`.
    ///
    /// This costs as much as producing about `block + interval` frames of
    /// output, all at once.
    pub fn seek(&mut self, pre_roll: &[&[f32]]) {
        self.reset();
        let frames = pre_roll.iter().map(|channel| channel.len()).min();
        let frames = frames.unwrap_or(0) as i64;
        let run = self.seek_blocks() * self.stft.interval();
        // The clock starts off by the fraction that makes it come out
        // even at the end of the run, so that the place the output is at
        // is counted from the end of the pre-roll exactly.
        let exact = run as f64 * self.clock.rate.value;
        let taken = exact.round();
        self.clock.owed = taken - exact;
        let taken = taken as i64;
        self.take_input(pre_roll, 0, (frames - taken).max(0) as usize);
        self.run(pre_roll, frames - taken, None, run);
    }

    /// Fills `output` with the next frames and takes the input they need
    /// from the front of `input`. Returns how many input frames it took,
    /// which is [`Stretcher::input_frames_needed`] of the output's length
    /// whatever `input` holds.
    ///
    /// `input` and `output` have one slice per channel. Where `input` runs
    /// out, the rest counts as silence: this is how the end of a file is
    /// played, and a missing channel is silent throughout. Input past what
    /// is taken is left alone. The output slices should be equally long;
    /// the shortest decides. Output channels the stretcher does not have
    /// are filled with silence.
    pub fn process(&mut self, input: &[&[f32]], output: &mut [&mut [f32]]) -> usize {
        let frames = output.iter().map(|channel| channel.len()).min();
        self.run(input, 0, Some(output), frames.unwrap_or(0))
    }

    /// Fills `output` with what is still to come when no more input
    /// follows: [`Stretcher::process`] with silence for input. After the
    /// last input frame the end of the sound takes
    /// [`Latency::output_frames`] frames to come out, and the output is
    /// silent a block later.
    pub fn flush(&mut self, output: &mut [&mut [f32]]) {
        self.process(&[], output);
    }

    /// Resets for a render that spreads `input_frames` over exactly
    /// `output_frames`, and returns how many frames of output come before
    /// the one that is frame 0 of the input.
    pub(crate) fn begin_exact(&mut self, input_frames: usize, output_frames: usize) -> usize {
        let rate = input_frames as f64 / output_frames as f64;
        self.time_ratio = 1.0 / rate;
        self.clock.rate = Glide::at(rate);
        self.reset();
        let latency = self.latency();
        let lead = latency.output as f64 + latency.input as f64 / rate;
        // The latency is seldom a whole number of output frames. The
        // clock starts ahead by the rest, which puts frame 0 of the input
        // on an output frame exactly.
        self.clock.owed = (lead - lead.floor()) * rate;
        lead.floor() as usize
    }

    fn seek_blocks(&self) -> usize {
        self.stft.block().div_ceil(self.stft.interval()) + 1
    }

    fn clear_spectra(&mut self) {
        self.has_previous = false;
        if self.cleared {
            return;
        }
        self.cleared = true;
        self.input.fill(ZERO);
        self.previous.fill(ZERO);
        self.output.fill(ZERO);
        self.input_energy.fill(0.0);
        self.predicted_energy.fill(0.0);
        self.predicted_input.fill(ZERO);
        self.fundamental_weighted = 0.0;
        self.fundamental_weight = 0.0;
    }

    /// Produces `frames` frames, into `output` if there is one. Input
    /// frame `from` is the first to be taken; frames before the start of
    /// `input` and past its end are silence. Returns the input taken.
    fn run(
        &mut self,
        input: &[&[f32]],
        from: i64,
        mut output: Option<&mut [&mut [f32]]>,
        frames: usize,
    ) -> usize {
        let interval = self.stft.interval();
        let (mut done, mut taken) = (0, 0);
        while done < frames {
            if self.since_block >= interval {
                self.block();
                self.since_block = 0;
            }
            // Advancing the silence gate on the sample clock makes tails and
            // automation independent of the host block partition.
            let chunk = 1;
            let after_end = self.silent_run as f64 * self.time_ratio
                - self.latency().output_frames(self.time_ratio);
            let tail_gain = (1.0 - after_end.max(0.0) / (f64::from(self.sample_rate) * 0.008))
                .clamp(0.0, 1.0) as f32;
            if let Some(output) = output.as_deref_mut() {
                for (channel, output) in output.iter_mut().enumerate() {
                    let output = &mut output[done..done + chunk];
                    if channel < self.channels {
                        self.stft.read_output(channel, output);
                        for sample in output {
                            *sample *= tail_gain;
                        }
                    } else {
                        output.fill(0.0);
                    }
                }
            }
            self.stft.move_output(chunk);
            let take = self.clock.advance(chunk);
            for _ in 0..chunk {
                self.pitch.tick();
            }
            self.take_input(input, from + taken as i64, take);
            taken += take;
            done += chunk;
            self.since_block += chunk;
            self.taken_since_block += take;
        }
        taken
    }

    /// Moves `count` frames of `input`, starting at frame `from`, into
    /// the history. Frames outside `input` are silence.
    fn take_input(&mut self, input: &[&[f32]], from: i64, count: usize) {
        // Only what the history can hold is worth writing.
        let skip = count.saturating_sub(self.stft.input_len());
        let (from, kept) = (from + skip as i64, count - skip);
        let mut loudest_at = None;
        for channel in 0..self.channels {
            let source = input.get(channel).copied().unwrap_or(&[]);
            let length = source.len() as i64;
            let lead = (-from).clamp(0, kept as i64) as usize;
            let start = from.clamp(0, length) as usize;
            let end = (from + kept as i64).clamp(0, length) as usize;
            let samples = &source[start..end];
            self.stft.write_silence(channel, 0, lead);
            self.stft.write_input(channel, lead, samples);
            let after = lead + samples.len();
            self.stft.write_silence(channel, after, kept - after);
            let loud = samples.iter().rposition(|sample| sample.abs() >= SILENCE);
            loudest_at = loudest_at.max(loud.map(|index| lead + index));
        }
        self.stft.move_input(kept);
        self.silent_run = match loudest_at {
            Some(index) => kept - 1 - index,
            None => self.silent_run.saturating_add(count),
        };
    }

    /// Works out the next block of output from the input taken so far.
    fn block(&mut self) {
        let interval = self.stft.interval();
        let taken = std::mem::take(&mut self.taken_since_block);
        // With nothing but silence in the history there is nothing to
        // work out, and the spectra start afresh when sound returns.
        if self.silent_run >= self.stft.input_len() {
            self.clear_spectra();
            self.stft.add_block_weight();
            return;
        }
        self.cleared = false;

        // The phase a bin gains from one block to the next is measured
        // over one interval of input, whatever the stretch, by looking
        // again at the block that lies exactly one interval back. That
        // keeps every frequency where it is without rescaling phases.
        let again = !self.has_previous || taken != interval;
        let bins = self.bins;
        for channel in 0..self.channels {
            let range = channel * bins..(channel + 1) * bins;
            if again {
                self.stft
                    .analyse(channel, interval, &mut self.previous[range.clone()]);
            }
            self.stft.analyse(channel, 0, &mut self.input[range]);
        }
        self.has_previous = true;

        let time_factor = if taken == 0 {
            1.0 / self.clock.rate.value
        } else {
            interval as f64 / taken as f64
        };
        let pitch_factor = 2.0_f64.powf(self.pitch.value / 12.0);
        self.process_spectrum(time_factor as f32, pitch_factor as f32);

        self.stft.add_block_weight();
        for channel in 0..self.channels {
            let range = channel * bins..(channel + 1) * bins;
            self.stft.synthesise(channel, &self.output[range]);
        }
    }

    fn bin_to_frequency(&self, bin: f32) -> f32 {
        (bin + 0.5) / self.stft.fft_size() as f32
    }

    fn frequency_to_bin(&self, frequency: f32) -> f32 {
        frequency * self.stft.fft_size() as f32 - 0.5
    }

    /// Turns the input spectra into the output spectra of this block.
    fn process_spectrum(&mut self, time_factor: f32, pitch_factor: f32) {
        let bins = self.bins;
        let mapped = pitch_factor != 1.0;
        self.old_map.copy_from_slice(&self.map);
        self.old_output.copy_from_slice(&self.output);
        self.old_energy.copy_from_slice(&self.predicted_energy);

        for (energy, input) in self.input_energy.iter_mut().zip(&self.input) {
            *energy = input.norm_sqr();
        }
        if mapped {
            self.smooth_energy();
            self.find_peaks(pitch_factor);
            self.update_map();
            if self.formants {
                self.keep_formants(pitch_factor);
            }
        } else {
            for (bin, point) in self.map.iter_mut().enumerate() {
                *point = MapPoint {
                    input_bin: bin as f32,
                    gradient: 1.0,
                };
            }
        }

        // Follow a moving frequency map before phase prediction. Otherwise a
        // pitch glide blends unrelated partials at each old output bin.
        let mut old_bin = 0;
        for bin in 0..bins {
            let source = self.map[bin].input_bin;
            while old_bin + 1 < bins && self.old_map[old_bin + 1].input_bin < source {
                old_bin += 1;
            }
            let low = self.old_map[old_bin].input_bin;
            let high = self.old_map[(old_bin + 1).min(bins - 1)].input_bin;
            let fraction = if high > low {
                ((source - low) / (high - low)).clamp(0.0, 1.0)
            } else {
                0.0
            };
            for channel in 0..self.channels {
                let start = channel * bins;
                let index = start + bin;
                self.output[index] = between(
                    &self.old_output[start..start + bins],
                    old_bin as isize,
                    fraction,
                ) * self.rotation[bin];
                let below = self.old_energy[start + old_bin];
                self.predicted_energy[index] = below
                    + (self.old_energy[start + (old_bin + 1).min(bins - 1)] - below) * fraction;
                let power = self.output[index].norm_sqr();
                if power > ENERGY_FLOOR {
                    self.output[index] *= (self.predicted_energy[index] / power).sqrt();
                }
            }
        }
        self.predict_from_time();
        self.predict_from_neighbours(time_factor);
        self.previous.copy_from_slice(&self.input);
        // Frequency mapping near DC can create a slow offset. Leave the
        // transparent unity path alone and taper only transformed sub-bass.
        if mapped || (time_factor - 1.0).abs() > 1e-6 {
            let hz_per_bin = self.sample_rate as f32 / self.stft.fft_size() as f32;
            for channel in self.output.chunks_exact_mut(bins) {
                for (bin, value) in channel.iter_mut().enumerate() {
                    let hz = (bin as f32 + 0.5) * hz_per_bin;
                    let ratio = (hz / 20.0).powi(4);
                    *value *= ratio / (1.0 + ratio);
                }
            }
        }
    }

    /// Adds the energy of all channels up per bin, and blurs it along the
    /// spectrum. A bin that stands out of the blur belongs to a peak.
    fn smooth_energy(&mut self) {
        let bins = self.bins;
        self.energy.fill(0.0);
        for channel in self.input_energy.chunks_exact(bins) {
            for (sum, energy) in self.energy.iter_mut().zip(channel) {
                *sum += energy;
            }
        }
        self.smoothed_energy.copy_from_slice(&self.energy);
        let spread = self.stft.fft_size() as f32 / self.stft.interval() as f32;
        let slew = 1.0 / (1.0 + spread * 0.5);
        let mut level = 0.0;
        for _ in 0..2 {
            for value in self.smoothed_energy.iter_mut().rev() {
                level += (*value - level) * slew;
                *value = level;
            }
            for value in self.smoothed_energy.iter_mut() {
                level += (*value - level) * slew;
                *value = level;
            }
        }
    }

    /// Finds the peaks of the spectrum and where each moves to.
    fn find_peaks(&mut self, pitch_factor: f32) {
        self.peaks.clear();
        let bins = self.bins;
        // Bins a full turn of phase over one interval stands for.
        let spread = self.stft.fft_size() as f32 / self.stft.interval() as f32;
        let mut start = 0;
        while start < bins {
            if self.energy[start] > self.smoothed_energy[start] {
                let (mut end, mut loudest) = (start, start);
                let (mut weighted, mut total) = (0.0_f32, 0.0_f32);
                while end < bins && self.energy[end] > self.smoothed_energy[end] {
                    weighted += end as f32 * self.energy[end];
                    total += self.energy[end];
                    if self.energy[end] > self.energy[loudest] {
                        loudest = end;
                    }
                    end += 1;
                }
                let mut centre = weighted / total;
                // Where the energy lies says where the peak is to a tenth
                // of a bin or so, and an error there is an error in the
                // pitch of the shifted partial. How far the phase of the
                // loudest bin has turned since the block before says
                // where a steady partial is exactly. It is taken when the
                // two agree, which they do unless the peak is not one
                // steady partial.
                let turn: Complex32 = (0..self.channels)
                    .map(|channel| channel * bins + loudest)
                    .map(|index| self.input[index] * self.previous[index].conj())
                    .sum();
                let turn = turn * self.rotation[loudest].conj();
                let exact = loudest as f32 + turn.arg() / std::f32::consts::TAU * spread;
                if (exact - centre).abs() <= 1.0 {
                    centre = exact;
                }
                let moved = self.bin_to_frequency(centre) * pitch_factor;
                // There is room for every peak there can be; the check
                // keeps the list from ever growing on the audio thread.
                if self.peaks.len() < self.peaks.capacity() {
                    self.peaks.push(Peak {
                        input: centre,
                        output: self.frequency_to_bin(moved),
                    });
                }
                start = end;
            }
            start += 1;
        }
    }

    /// Lays out where each output bin comes from. Around a peak the map
    /// moves bins without squeezing them, so a partial keeps its shape
    /// and only changes place; between peaks it bends from one shift to
    /// the next.
    fn update_map(&mut self) {
        let bins = self.bins as isize;
        let whole = |value: f32| value.ceil().clamp(-1.0, bins as f32 + 1.0) as isize;
        let (Some(first), Some(last)) = (self.peaks.first(), self.peaks.last()) else {
            for (bin, point) in self.map.iter_mut().enumerate() {
                *point = MapPoint {
                    input_bin: bin as f32,
                    gradient: 1.0,
                };
            }
            return;
        };
        let (bottom, top) = (first.input - first.output, last.input - last.output);
        for bin in 0..whole(first.output).clamp(0, bins) {
            self.map[bin as usize] = MapPoint {
                input_bin: bin as f32 + bottom,
                gradient: 1.0,
            };
        }
        let lobe = self.stft.lobe_bins() * LOBE_SHARE;
        for pair in self.peaks.windows(2) {
            let (below, above) = (pair[0], pair[1]);
            let gap = above.output - below.output;
            // Each peak keeps its lobe, or its share of the gap where the
            // peaks are too close for that.
            let flat = lobe.min(gap * 0.25);
            let bend_scale = 1.0 / (gap - 2.0 * flat);
            let offset = below.input - below.output;
            let change = above.input - above.output - offset;
            for bin in whole(below.output).max(0)..whole(above.output).min(bins) {
                let along = ((bin as f32 - below.output - flat) * bend_scale).clamp(0.0, 1.0);
                let eased = along * along * (3.0 - 2.0 * along);
                self.map[bin as usize] = MapPoint {
                    input_bin: bin as f32 + offset + eased * change,
                    gradient: 1.0 + 6.0 * along * (1.0 - along) * change * bend_scale,
                };
            }
        }
        let from = (last.output.clamp(-1.0, bins as f32 + 1.0) as isize).max(0);
        for bin in from..bins {
            self.map[bin as usize] = MapPoint {
                input_bin: bin as f32 + top,
                gradient: 1.0,
            };
        }
    }

    /// A very rough estimate of the fundamental, as a bin, from the three
    /// strongest peaks of the summed energy in `envelope`.
    fn estimate_fundamental(&mut self) -> f32 {
        let metric = &self.envelope;
        let mut strongest = [0_usize; 3];
        for bin in 1..self.bins - 1 {
            let energy = metric[bin];
            if energy < metric[bin - 1] || energy <= metric[bin + 1] {
                continue;
            }
            if energy > metric[strongest[0]] {
                if energy > metric[strongest[1]] {
                    if energy > metric[strongest[2]] {
                        strongest = [strongest[1], strongest[2], bin];
                    } else {
                        strongest = [strongest[1], bin, strongest[2]];
                    }
                } else {
                    strongest[0] = bin;
                }
            }
        }
        // Two strong peaks are probably harmonics of one note, and the
        // note is then what is left of one when divided by their distance.
        let mut estimate = strongest[2];
        let narrow = |estimate: usize, other: usize| {
            let distance = estimate.abs_diff(other);
            if distance > estimate / 8 && distance < estimate * 7 / 8 {
                {
                    let remainder = estimate % distance;
                    if remainder < distance / 4 {
                        distance
                    } else {
                        remainder
                    }
                }
            } else {
                estimate
            }
        };
        if metric[strongest[1]] > metric[strongest[2]] * 0.1 {
            estimate = narrow(estimate, strongest[1]);
            if metric[strongest[0]] > metric[strongest[2]] * 0.01 {
                estimate = narrow(estimate, strongest[0]);
            }
        }
        let weight = metric[strongest[2]];
        self.fundamental_weighted += (estimate as f32 * weight - self.fundamental_weighted) * 0.25;
        self.fundamental_weight += (weight - self.fundamental_weight) * 0.25;
        self.fundamental_weighted / (self.fundamental_weight + 1e-30)
    }

    /// Scales the energy of each input bin so that, once it has moved,
    /// the output has the spectral envelope the input had.
    fn keep_formants(&mut self, pitch_factor: f32) {
        let bins = self.bins;
        self.envelope[..bins].copy_from_slice(&self.energy);
        // A fundamental below the first bin would leave no envelope at all.
        let fundamental = self.estimate_fundamental();
        if fundamental < 2.0 {
            return;
        }

        // The envelope is the energy with the harmonics joined up: peaks
        // spread out about as far as the harmonics are apart, and then
        // the result is pulled back in by as much, which leaves the lines
        // from peak to peak.
        let decay = 1.0 - 1.0 / (fundamental * 0.5 + 1.0);
        let envelope = &mut self.envelope[..bins];
        let mut level = 0.0_f32;
        for _ in 0..2 {
            for value in envelope.iter_mut().rev() {
                level = value.max(level * decay);
                *value = level;
            }
            for value in envelope.iter_mut() {
                level = value.max(level * decay);
                *value = level;
            }
        }
        let growth = 1.0 / decay;
        for _ in 0..2 {
            for value in envelope.iter_mut().rev() {
                level = value.min(level * growth);
                *value = level;
            }
            for value in envelope.iter_mut() {
                level = value.min(level * growth);
                *value = level;
            }
        }

        for bin in 0..bins {
            let moved = self.bin_to_frequency(bin as f32) * pitch_factor;
            let target = self.frequency_to_bin(moved).min(bins as f32);
            let wanted = if target < 0.0 {
                0.0
            } else {
                let low = target.floor();
                let (below, above) = (self.envelope[low as usize], self.envelope[low as usize + 1]);
                below + (above - below) * (target - low)
            };
            // A modest expansion compensates for envelope smoothing; cap the
            // gain because silence and unvoiced spectra have no formants.
            let ratio = (wanted / (self.envelope[bin] + 1e-30)).powf(1.4).min(16.0);
            for channel in 0..self.channels {
                self.input_energy[channel * bins + bin] *= ratio;
            }
        }
    }

    /// A first guess at each output bin: the last output, turned by as
    /// much as the input has turned since the block before.
    fn predict_from_time(&mut self) {
        let bins = self.bins;
        for channel in 0..self.channels {
            let range = channel * bins..(channel + 1) * bins;
            let input = &self.input[range.clone()];
            let previous = &self.previous[range.clone()];
            let input_energy = &self.input_energy[range.clone()];
            let output = &mut self.output[range.clone()];
            let predicted_input = &mut self.predicted_input[range.clone()];
            let predicted_energy = &mut self.predicted_energy[range];
            for (bin, point) in self.map.iter().enumerate() {
                let low = point.input_bin.floor();
                let fraction = point.input_bin - low;
                let low = low as isize;

                let energy_below = band(input_energy, low);
                let energy = energy_below + (band(input_energy, low + 1) - energy_below) * fraction;
                // A stretch of the spectrum that the map squeezes into
                // fewer bins would otherwise come out louder.
                let energy = energy * point.gradient.max(0.0);
                let before = std::mem::replace(&mut predicted_energy[bin], energy);
                let now = between(input, low, fraction);
                predicted_input[bin] = now;

                // The turn of the input since the block before, less the
                // turn of the centre of the place it is read at. Taking
                // that turn off after reading between the bins, and not
                // before, is what keeps a shifted partial in tune: bins
                // that have been turned by different amounts no longer
                // blend into the phase that lies between them.
                let centre = match self.rotation.get(low as usize) {
                    Some(rotation) if low >= 0 => {
                        let place = fraction * TURN_STEPS as f32;
                        let step = (place as usize).min(TURN_STEPS - 1);
                        let (below, above) = (self.part_turn[step], self.part_turn[step + 1]);
                        rotation.conj() * (below + (above - below) * (place - step as f32))
                    }
                    _ => ZERO,
                };
                let twist = now * between(previous, low, fraction).conj() * centre;
                output[bin] = output[bin] * twist / (before.max(energy) + ENERGY_FLOOR);
            }
        }
    }

    /// The final output: each bin's phase is predicted again from its
    /// neighbours up and down the spectrum, which is what keeps the
    /// partials of one sound, and the two sides of a transient, in step.
    ///
    /// The loudest channel is predicted, and every other channel gets the
    /// phase difference to it that the input had, so that the stereo
    /// image stays where it was.
    fn predict_from_neighbours(&mut self, time_factor: f32) {
        let bins = self.bins;
        let spread = self.stft.fft_size() as f32 / self.stft.interval() as f32;
        let long_step = (spread.round() as usize).clamp(1, 8);
        let time_factor = time_factor.max(1.0 / MAX_CLEAN_STRETCH);
        let scatter = time_factor > MAX_CLEAN_STRETCH;
        let lowest = MAX_CLEAN_STRETCH * 2.0 - time_factor;

        for bin in 0..bins {
            let mut loudest = 0;
            for channel in 1..self.channels {
                if self.predicted_energy[channel * bins + bin]
                    > self.predicted_energy[loudest * bins + bin]
                {
                    loudest = channel;
                }
            }
            let range = loudest * bins..(loudest + 1) * bins;
            let input = &self.input[range.clone()];
            let predicted = &self.predicted_input[range.clone()];
            let output = &self.output[range.clone()];
            let own = predicted[bin];
            let point = self.map[bin];
            let mut phase = ZERO;

            // From the bins below, which already have this block's
            // output. The input is read `time_factor` bins apart for bins
            // that are one apart in the output: that scales the phase
            // step between them, which is what moves a transient to its
            // place in the stretched time.
            if bin > 0 {
                let step = vertical_step(&mut self.random, scatter, lowest, time_factor);
                let twist = own * at(input, point.input_bin - step).conj();
                phase += output[bin - 1] * twist;
                if bin >= long_step {
                    let far = at(input, point.input_bin - long_step as f32 * step);
                    phase += output[bin - long_step] * (own * far.conj());
                }
            }
            // From the bins above, which still hold the first guess.
            if bin < bins - 1 {
                let step = vertical_step(&mut self.random, scatter, lowest, time_factor);
                let above = self.map[bin + 1].input_bin;
                let twist = predicted[bin + 1] * at(input, above - step).conj();
                phase += output[bin + 1] * twist.conj();
                if bin + long_step < bins {
                    let above = self.map[bin + long_step].input_bin;
                    let far = at(input, above - long_step as f32 * step);
                    let twist = predicted[bin + long_step] * far.conj();
                    phase += output[bin + long_step] * twist.conj();
                }
            }

            let energy = self.predicted_energy[range.start + bin];
            let result = with_energy(phase, energy, own, PREDICTION_FLOOR);
            self.output[range.start + bin] = result;
            for channel in (0..self.channels).filter(|channel| *channel != loudest) {
                let index = channel * bins + bin;
                let other = self.predicted_input[index];
                // Whether the prediction was strong enough has been
                // settled for the loudest channel, and the others follow
                // it either way: only nothing at all has no phase.
                let phase = result * (other * own.conj());
                self.output[index] = with_energy(phase, self.predicted_energy[index], other, 0.0);
            }
        }
    }
}

/// The distance in input bins that stands for one output bin: the time
/// factor, or beyond the clean range a random value around its limit.
#[inline]
fn vertical_step(random: &mut u32, scatter: bool, lowest: f32, time_factor: f32) -> f32 {
    if !scatter {
        return time_factor;
    }
    // Marsaglia's 32-bit xorshift: the same stream after every reset, so
    // that a render repeats.
    *random ^= *random << 13;
    *random ^= *random >> 17;
    *random ^= *random << 5;
    let unit = (*random >> 8) as f32 * (1.0 / 16_777_216.0);
    lowest + (time_factor - lowest) * unit
}

const RANDOM_SEED: u32 = 0x9E37_79B9;
