use super::crossover::Crossover;
use super::params::{BassHarmonicsParams, ExciterParams};
use crate::balance::{Controls, audio, rate};
use crate::blocks::biquad::{Biquad, BiquadCoeffs, CoeffRamp};
use crate::blocks::halfband::HalfbandDecimator;
use crate::blocks::math::ms_to_samples;
use crate::effect::Effect;
use crate::param::ParamSet;

// Causal linear interpolation contributes one frame; the FIR contributes 12.
pub const HARMONICS_LATENCY_SAMPLES: usize = HalfbandDecimator::LATENCY + 1;

struct HarmonicCore {
    split: Crossover,
    decimators: [HalfbandDecimator; 2],
    previous: [f32; 2],
    dry: [[f32; 2]; HARMONICS_LATENCY_SAMPLES],
    position: usize,
    highpass: [Biquad; 2],
    coefficients: CoeffRamp,
    controls: Controls<3>,
    frequency: f32,
    sample_rate: f32,
    fresh: bool,
    bass: bool,
}
impl HarmonicCore {
    fn new(bass: bool) -> Self {
        let frequency = if bass { 180.0 } else { 3_000.0 };
        let drive = if bass { 3.0 } else { 2.0 };
        let mut effect = Self {
            split: Crossover::default(),
            decimators: [HalfbandDecimator::default(); 2],
            previous: [0.0; 2],
            dry: [[0.0; 2]; HARMONICS_LATENCY_SAMPLES],
            position: 0,
            highpass: [Biquad::default(); 2],
            coefficients: CoeffRamp::new(BiquadCoeffs::IDENTITY),
            controls: Controls::new([drive, 0.0, 0.0]),
            frequency,
            sample_rate: 48_000.0,
            fresh: true,
            bass,
        };
        effect.set(frequency, drive, 0.0, 0.0);
        effect
    }
    fn prepare(&mut self, sample_rate: f32) {
        self.sample_rate = rate(sample_rate).max(8.0);
        self.split.prepare(self.sample_rate);
        self.controls.prepare(self.sample_rate);
        self.reset();
    }
    fn reset(&mut self) {
        self.split.reset();
        for d in &mut self.decimators {
            d.reset();
        }
        self.previous = [0.0; 2];
        self.dry.fill([0.0; 2]);
        self.position = 0;
        self.highpass = [Biquad::default(); 2];
        self.controls.reset();
        self.fresh = true;
        self.update_filters();
    }
    fn update_filters(&mut self) {
        if self.bass {
            self.split.set(self.frequency, 2_500.0);
        } else {
            self.split.set(200.0, self.frequency);
        }
        let cutoff = (if self.bass {
            self.frequency * 0.5
        } else {
            self.frequency
        })
        .clamp(1.0, self.sample_rate * 0.45);
        let target =
            BiquadCoeffs::high_pass(cutoff, std::f32::consts::FRAC_1_SQRT_2, self.sample_rate);
        self.coefficients.set_target(
            target,
            if self.fresh {
                0
            } else {
                ms_to_samples(20.0, self.sample_rate)
            },
        );
    }
    fn set(&mut self, frequency: f32, drive: f32, mix: f32, color: f32) {
        if self.fresh || self.frequency != frequency {
            self.frequency = frequency;
            self.update_filters();
        }
        self.controls.set([drive, mix, color]);
    }
    fn tick(&mut self, input: [f32; 2]) -> [f32; 2] {
        self.fresh = false;
        let bands = self.split.tick(input);
        let source = bands[if self.bass { 0 } else { 2 }];
        let dry = self.dry[self.position];
        self.dry[self.position] =
            std::array::from_fn(|c| audio(bands[0][c] + bands[1][c] + bands[2][c]));
        self.position = (self.position + 1) % HARMONICS_LATENCY_SAMPLES;
        let [drive, mix, color] = self.controls.tick();
        let bias = color * 0.75;
        let bias_tanh = bias.tanh();
        let derivative = 1.0 - bias_tanh * bias_tanh;
        let shape = |x: f32| {
            if self.bass {
                // Even harmonics carry bass an octave upward. Saturation
                // bounds the rectifier; the wet high-pass removes its DC.
                let saturated = (drive * x).tanh();
                0.5 * saturated * saturated
            } else {
                // Subtract the small-signal tangent, retaining nonlinear
                // residual only. Bias permits even harmonics and color.
                ((drive * x + bias).tanh() - bias_tanh) / drive - derivative * x
            }
        };
        let coeffs = *self.coefficients.tick();
        std::array::from_fn(|c| {
            let previous = self.previous[c];
            self.previous[c] = source[c];
            let wet = self.decimators[c].tick(shape(previous), shape(0.5 * (previous + source[c])));
            let wet = self.highpass[c].tick(&coeffs, f64::from(wet)) as f32;
            self.highpass[c].flush();
            audio(dry[c] + mix * wet)
        })
    }
    fn tail_samples(&self) -> usize {
        // Includes the DC-removing wet filter (which can be slower than the crossover).
        let cutoff = (self.frequency * if self.bass { 0.5 } else { 1.0 })
            .clamp(1.0, self.sample_rate * 0.45);
        self.split
            .tail_samples()
            .max((64.0 * self.sample_rate / cutoff).ceil() as usize)
            + HARMONICS_LATENCY_SAMPLES
    }
}

pub struct BassHarmonics {
    core: HarmonicCore,
}
impl Default for BassHarmonics {
    fn default() -> Self {
        Self {
            core: HarmonicCore::new(true),
        }
    }
}
impl Effect for BassHarmonics {
    type Params = BassHarmonicsParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.core.prepare(sample_rate);
    }
    fn reset(&mut self) {
        self.core.reset();
    }
    fn set_params(&mut self, params: &Self::Params) {
        let p = params.sanitized();
        self.core.set(p.cutoff_hz, p.drive, p.amount, 0.0);
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let out = self.core.tick([*l, *r]);
            *l = out[0];
            *r = out[1];
        }
    }
    fn latency_samples(&self) -> usize {
        HARMONICS_LATENCY_SAMPLES
    }
    fn tail_samples(&self) -> usize {
        self.core.tail_samples()
    }
}

pub struct Exciter {
    core: HarmonicCore,
}
impl Default for Exciter {
    fn default() -> Self {
        Self {
            core: HarmonicCore::new(false),
        }
    }
}
impl Effect for Exciter {
    type Params = ExciterParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.core.prepare(sample_rate);
    }
    fn reset(&mut self) {
        self.core.reset();
    }
    fn set_params(&mut self, params: &Self::Params) {
        let p = params.sanitized();
        self.core.set(p.frequency_hz, p.drive, p.mix, p.color);
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            let out = self.core.tick([*l, *r]);
            *l = out[0];
            *r = out[1];
        }
    }
    fn latency_samples(&self) -> usize {
        HARMONICS_LATENCY_SAMPLES
    }
    fn tail_samples(&self) -> usize {
        self.core.tail_samples()
    }
}
