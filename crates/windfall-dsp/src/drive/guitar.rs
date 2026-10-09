use crate::balance::{Controls, audio, rate};
use crate::blocks::dc::DcBlocker;
use crate::blocks::delay_line::DelayLine;
use crate::blocks::envelope::Ballistics;
use crate::blocks::lfo::{Lfo, LfoShape};
use crate::blocks::math::{db_to_gain, ms_to_samples};
use crate::blocks::svf::{OnePoleFilter, Svf, SvfCoeffs};
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum GuitarModel {
    #[default]
    CleanBoost,
    TubeSoftClip,
    HardFuzz,
    OctaveUp,
    TrebleBooster,
    MidScoop,
    Wah,
    CompressorSquash,
    Gate,
    SpringSlap,
    Cabinet,
}
impl GuitarModel {
    pub const ALL: [Self; 11] = [
        Self::CleanBoost,
        Self::TubeSoftClip,
        Self::HardFuzz,
        Self::OctaveUp,
        Self::TrebleBooster,
        Self::MidScoop,
        Self::Wah,
        Self::CompressorSquash,
        Self::Gate,
        Self::SpringSlap,
        Self::Cabinet,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct GuitarRackParams {
    /// One of eleven algorithms; default clean boost.
    pub model: GuitarModel,
    /// Input gain in dB, 0..36; default 6.
    pub drive_db: f32,
    /// Output trim in dB, -24..12; default -6.
    pub output_db: f32,
}
impl Default for GuitarRackParams {
    fn default() -> Self {
        Self {
            model: GuitarModel::CleanBoost,
            drive_db: 6.0,
            output_db: -6.0,
        }
    }
}
param_set!(GuitarRackParams, "Guitar Rack", {
    choice [model] "model" "Pedal" { GuitarModel, CleanBoost, [
        CleanBoost "cleanBoost" "Clean boost",
        TubeSoftClip "tubeSoftClip" "Tube soft clip",
        HardFuzz "hardFuzz" "Hard fuzz",
        OctaveUp "octaveUp" "Octave up",
        TrebleBooster "trebleBooster" "Treble booster",
        MidScoop "midScoop" "Mid scoop",
        Wah "wah" "Wah",
        CompressorSquash "compressorSquash" "Compressor squash",
        Gate "gate" "Gate",
        SpringSlap "springSlap" "Spring slap",
        Cabinet "cabinet" "Cabinet"
    ] }
    float [drive_db] "driveDb" "Drive" { Decibels, Linear, 0.0, 36.0, 6.0 }
    float [output_db] "outputDb" "Output" { Decibels, Linear, -24.0, 12.0, -6.0 }
});

/// Smooth cubic soft knee with different tube-like positive/negative limits.
#[inline]
fn tube(input: f32) -> f32 {
    let ceiling = if input >= 0.0 { 0.85 } else { 1.15 };
    let x = (input / ceiling).clamp(-1.0, 1.0);
    ceiling * (x - x * x * x / 3.0)
}

struct Channel {
    treble: OnePoleFilter,
    scoop: Svf,
    wah: Svf,
    cabinet: [Svf; 2],
    octave_dc: DcBlocker,
    compressor: Ballistics,
    gate_detector: Ballistics,
    gate_gain: Ballistics,
    gate_open: bool,
    spring: DelayLine,
    spring_damping: OnePoleFilter,
    dispersion: f32,
}
impl Channel {
    fn new(sample_rate: f32, delay: usize) -> Self {
        let mut compressor = Ballistics::new(0.0);
        compressor.set_times(2.0, 90.0, sample_rate);
        let mut gate_detector = Ballistics::new(0.0);
        gate_detector.set_times(0.5, 35.0, sample_rate);
        let mut gate_gain = Ballistics::new(0.0);
        gate_gain.set_times(2.0, 20.0, sample_rate);
        Self {
            treble: OnePoleFilter::default(),
            scoop: Svf::default(),
            wah: Svf::default(),
            cabinet: [Svf::default(); 2],
            octave_dc: DcBlocker::new(20.0, sample_rate),
            compressor,
            gate_detector,
            gate_gain,
            gate_open: false,
            spring: DelayLine::new(delay),
            spring_damping: OnePoleFilter::default(),
            dispersion: 0.0,
        }
    }
    fn reset(&mut self) {
        self.treble.reset();
        self.scoop.reset();
        self.wah.reset();
        for c in &mut self.cabinet {
            c.reset();
        }
        self.octave_dc.reset();
        self.compressor.snap(0.0);
        self.gate_detector.snap(0.0);
        self.gate_gain.snap(0.0);
        self.gate_open = false;
        self.spring.clear();
        self.spring_damping.reset();
        self.dispersion = 0.0;
    }
    /// Keep every model's history warm so rapid selection edits crossfade
    /// from the current mixture without replacing or freeing any state.
    fn tick(&mut self, x: f32, config: &Config, wah: &SvfCoeffs) -> [f32; 11] {
        let treble = self.treble.high_pass(config.treble, x);
        let mids = self.scoop.tick(&config.scoop, x).band * config.scoop.k;
        let band = self.wah.tick(wah, x).band * wah.k;
        let cab1 = self.cabinet[0].tick(&config.cabinet, x).low;
        let cab = self.cabinet[1].tick(&config.cabinet, cab1).low;
        let octave = self.octave_dc.tick(x.abs()) * 1.6;
        let envelope = self.compressor.tick(x.abs());
        let squash_gain = if envelope > 0.18 {
            (0.18 / envelope).powf(0.8)
        } else {
            1.0
        };
        let gate_level = self.gate_detector.tick(x.abs());
        if gate_level > 0.045 {
            self.gate_open = true;
        }
        if gate_level < 0.025 {
            self.gate_open = false;
        }
        let gate = self.gate_gain.tick(if self.gate_open { 1.0 } else { 0.0 });
        let first = self.spring.tap(config.slap[0]);
        let second = self.spring.tap(config.slap[1]);
        let damped = self
            .spring_damping
            .low_pass(config.damping, 0.65 * first + 0.35 * second);
        // One-sample allpass dispersion inside a damped, bounded feedback
        // loop: a short metallic slap rather than a physical spring model.
        let dispersed = self.dispersion - 0.55 * damped;
        self.dispersion = audio(damped + 0.55 * dispersed);
        self.spring.push(audio(x + 0.35 * dispersed));
        self.treble.flush();
        self.scoop.flush();
        self.wah.flush();
        for c in &mut self.cabinet {
            c.flush();
        }
        self.spring_damping.flush();
        [
            x,
            tube(x),
            (x * 6.0).clamp(-0.65, 0.65),
            octave,
            x + 2.5 * treble,
            x - 0.85 * mids,
            2.0 * band,
            1.5 * squash_gain * x,
            gate * x,
            x + 0.4 * dispersed,
            cab,
        ]
    }
}

struct Config {
    treble: f32,
    scoop: SvfCoeffs,
    cabinet: SvfCoeffs,
    damping: f32,
    slap: [usize; 2],
}
impl Config {
    fn new(sample_rate: f32) -> Self {
        Self {
            treble: OnePoleFilter::coefficient(1_200.0, sample_rate),
            scoop: SvfCoeffs::new(750.0, 0.7, sample_rate),
            cabinet: SvfCoeffs::new(3_800.0, std::f32::consts::FRAC_1_SQRT_2, sample_rate),
            damping: OnePoleFilter::coefficient(3_200.0, sample_rate),
            slap: [
                ms_to_samples(37.0, sample_rate) as usize,
                ms_to_samples(53.0, sample_rate) as usize,
            ],
        }
    }
}

pub struct GuitarRack {
    controls: Controls<13>,
    channels: [Channel; 2],
    config: Config,
    sample_rate: f32,
    lfo: Lfo,
}
impl Default for GuitarRack {
    fn default() -> Self {
        let config = Config::new(48_000.0);
        Self {
            controls: Controls::new(Self::values(&GuitarRackParams::default())),
            channels: std::array::from_fn(|_| Channel::new(48_000.0, config.slap[1])),
            config,
            sample_rate: 48_000.0,
            lfo: Lfo::new(1),
        }
    }
}
impl GuitarRack {
    fn values(p: &GuitarRackParams) -> [f32; 13] {
        let mut values = [0.0; 13];
        values[0] = db_to_gain(p.drive_db);
        values[1] = db_to_gain(p.output_db);
        values[p.model as usize + 2] = 1.0;
        values
    }
}
impl Effect for GuitarRack {
    type Params = GuitarRackParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.sample_rate = rate(sample_rate);
        self.config = Config::new(self.sample_rate);
        self.channels =
            std::array::from_fn(|_| Channel::new(self.sample_rate, self.config.slap[1]));
        self.controls.prepare(self.sample_rate);
        self.lfo.reset();
    }
    fn reset(&mut self) {
        self.controls.reset();
        self.lfo.reset();
        for c in &mut self.channels {
            c.reset();
        }
    }
    fn set_params(&mut self, params: &Self::Params) {
        self.controls.set(Self::values(&params.sanitized()));
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let values = self.controls.tick();
            let sweep = self.lfo.tick(LfoShape::Sine, 0.65 / self.sample_rate);
            let cutoff = 450.0 * (4.0_f32).powf(0.5 * (sweep + 1.0));
            let wah = SvfCoeffs::new(cutoff, 2.5, self.sample_rate);
            for (side, sample) in [l, r].into_iter().enumerate() {
                let models =
                    self.channels[side].tick(audio(audio(*sample) * values[0]), &self.config, &wah);
                let mixed: f32 = models.iter().zip(&values[2..]).map(|(x, w)| x * w).sum();
                *sample = audio(mixed * values[1]);
            }
        }
    }
    // All histories run continuously, including during a model fade.
    fn tail_samples(&self) -> usize {
        (self.sample_rate * 2.0).ceil() as usize
    }
    fn gap_samples(&self) -> usize {
        self.config.slap[1]
    }
}
