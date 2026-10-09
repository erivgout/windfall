use crate::balance::{Controls, audio, rate};
use crate::blocks::dc::DcBlocker;
use crate::blocks::math::db_to_gain;
use crate::blocks::svf::{OnePoleFilter, Svf, SvfCoeffs};
use crate::effect::Effect;
use crate::param::{ParamSet, param_set};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct OverdriveParams {
    /// Input drive in dB, 0..36; default 18.
    pub drive_db: f32,
    /// Unequal clip ceilings, 0..1; default 0.7.
    pub asymmetry: f32,
    /// Treble emphasis before clipping, 0..1; default 0.5.
    pub emphasis: f32,
    /// Post lowpass cutoff in Hz, 200..18000; default 6000.
    pub tone_hz: f32,
    /// Output trim in dB, -24..12; default -6.
    pub output_db: f32,
}
impl Default for OverdriveParams {
    fn default() -> Self {
        Self {
            drive_db: 18.0,
            asymmetry: 0.7,
            emphasis: 0.5,
            tone_hz: 6_000.0,
            output_db: -6.0,
        }
    }
}
param_set!(OverdriveParams, "Overdrive", {
    float [drive_db] "driveDb" "Drive" { Decibels, Linear, 0.0, 36.0, 18.0 }
    float [asymmetry] "asymmetry" "Asymmetry" { Fraction, Linear, 0.0, 1.0, 0.7 }
    float [emphasis] "emphasis" "Pre-emphasis" { Fraction, Linear, 0.0, 1.0, 0.5 }
    float [tone_hz] "toneHz" "Tone" { Hertz, Logarithmic, 200.0, 18_000.0, 6_000.0 }
    float [output_db] "outputDb" "Output" { Decibels, Linear, -24.0, 12.0, -6.0 }
});
impl OverdriveParams {
    /// Settled static clip curve; excludes the emphasis, tone and DC filters.
    pub fn transfer(&self, input: f32) -> f32 {
        let p = self.sanitized();
        audio(
            super::asymmetric(audio(input) * db_to_gain(p.drive_db), p.asymmetry)
                * db_to_gain(p.output_db),
        )
    }
}

struct Channel {
    pre: OnePoleFilter,
    post: Svf,
    dc: DcBlocker,
}
impl Channel {
    fn new(sample_rate: f32) -> Self {
        Self {
            pre: OnePoleFilter::default(),
            post: Svf::default(),
            dc: DcBlocker::new(5.0, sample_rate),
        }
    }
    fn reset(&mut self) {
        self.pre.reset();
        self.post.reset();
        self.dc.reset();
    }
    fn tick(&mut self, input: f32, values: &[f32; 5], pre: f32, post: &SvfCoeffs) -> f32 {
        let high = self.pre.high_pass(pre, input);
        let clipped = super::asymmetric((input + 3.0 * values[2] * high) * values[0], values[1]);
        let filtered = self.post.tick(post, clipped).low;
        self.pre.flush();
        self.post.flush();
        audio(self.dc.tick(filtered) * values[4])
    }
}

pub struct Overdrive {
    controls: Controls<5>,
    channels: [Channel; 2],
    sample_rate: f32,
    pre: f32,
}
impl Default for Overdrive {
    fn default() -> Self {
        Self {
            controls: Controls::new(Self::values(&OverdriveParams::default())),
            channels: std::array::from_fn(|_| Channel::new(48_000.0)),
            sample_rate: 48_000.0,
            pre: OnePoleFilter::coefficient(900.0, 48_000.0),
        }
    }
}
impl Overdrive {
    fn values(p: &OverdriveParams) -> [f32; 5] {
        [
            db_to_gain(p.drive_db),
            p.asymmetry,
            p.emphasis,
            p.tone_hz,
            db_to_gain(p.output_db),
        ]
    }
}
impl Effect for Overdrive {
    type Params = OverdriveParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.sample_rate = rate(sample_rate);
        self.pre = OnePoleFilter::coefficient(900.0, self.sample_rate);
        self.channels = std::array::from_fn(|_| Channel::new(self.sample_rate));
        self.controls.prepare(self.sample_rate);
    }
    fn reset(&mut self) {
        self.controls.reset();
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
            let post = SvfCoeffs::new(values[3], std::f32::consts::FRAC_1_SQRT_2, self.sample_rate);
            *l = self.channels[0].tick(audio(*l), &values, self.pre, &post);
            *r = self.channels[1].tick(audio(*r), &values, self.pre, &post);
        }
    }
    fn tail_samples(&self) -> usize {
        (self.sample_rate * 2.0).ceil() as usize
    }
}
