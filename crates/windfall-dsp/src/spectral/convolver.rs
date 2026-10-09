use super::{audio, output, rate};
use crate::balance::Controls;
use crate::blocks::math::clean;
use crate::blocks::smooth::LinearRamp;
use crate::effect::Effect;
use crate::param::{ParamInfo, ParamSet, param_set};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const MAX_IMPULSE_SAMPLES: usize = 2048;
pub const CONVOLUTION_PARTITION: usize = 128;
const FFT_SIZE: usize = CONVOLUTION_PARTITION * 2;
const PARTITIONS: usize = MAX_IMPULSE_SAMPLES / CONVOLUTION_PARTITION;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
pub struct ConvolverParams {
    /// Mono IR shared by the independent stereo paths. Only impulse_length
    /// samples are used. A zero length selects a unit impulse.
    #[serde(with = "impulse_serde")]
    #[ts(type = "number[]")]
    pub impulse: [f32; MAX_IMPULSE_SAMPLES],
    pub impulse_length: u16,
    /// Wet share, 0..1; dry receives the same 128-sample transport delay.
    pub mix: f32,
    /// Output linear gain, 0..4.
    pub gain: f32,
}
impl Default for ConvolverParams {
    fn default() -> Self {
        Self {
            impulse: [0.0; MAX_IMPULSE_SAMPLES],
            impulse_length: 0,
            mix: 1.0,
            gain: 1.0,
        }
    }
}
// The fixed payload is an atomic asset edit, not 2048 automation controls.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Knobs {
    mix: f32,
    gain: f32,
}
impl Default for Knobs {
    fn default() -> Self {
        Self {
            mix: 1.0,
            gain: 1.0,
        }
    }
}
param_set!(Knobs, "Convolver", {
    float [mix] "mix" "Mix" { Fraction, Linear, 0.0, 1.0, 1.0 }
    float [gain] "gain" "Gain" { Gain, Linear, 0.0, 4.0, 1.0 }
});
impl ParamSet for ConvolverParams {
    const NAME: &'static str = "Convolver";
    fn descriptors() -> &'static [ParamInfo] {
        Knobs::descriptors()
    }
    fn get(&self, index: usize) -> Option<f32> {
        Knobs {
            mix: self.mix,
            gain: self.gain,
        }
        .get(index)
    }
    fn set(&mut self, index: usize, value: f32) -> bool {
        let mut knobs = Knobs {
            mix: self.mix,
            gain: self.gain,
        };
        let changed = knobs.set(index, value);
        self.mix = knobs.mix;
        self.gain = knobs.gain;
        changed
    }
    fn sanitized(&self) -> Self {
        let mut p = *self;
        p.impulse_length = p.impulse_length.min(MAX_IMPULSE_SAMPLES as u16);
        for (i, sample) in p.impulse.iter_mut().enumerate() {
            *sample = if i < usize::from(p.impulse_length) {
                clean(*sample, -8.0, 8.0, 0.0)
            } else {
                0.0
            };
        }
        p.mix = clean(p.mix, 0.0, 1.0, 1.0);
        p.gain = clean(p.gain, 0.0, 4.0, 1.0);
        p
    }
    fn approach(&mut self, target: &Self, amount: f32) -> bool {
        let mut knobs = Knobs {
            mix: self.mix,
            gain: self.gain,
        };
        let moving = knobs.approach(
            &Knobs {
                mix: target.mix,
                gain: target.gain,
            },
            amount,
        );
        self.mix = knobs.mix;
        self.gain = knobs.gain;
        self.impulse = target.impulse;
        self.impulse_length = target.impulse_length;
        moving
    }
}

// Decode directly into fixed storage: serde only implements short arrays.
mod impulse_serde {
    use super::MAX_IMPULSE_SAMPLES;
    use serde::de::{Error, SeqAccess, Visitor};
    use serde::ser::SerializeSeq;
    use serde::{Deserializer, Serializer};
    pub fn serialize<S: Serializer>(
        samples: &[f32; MAX_IMPULSE_SAMPLES],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(MAX_IMPULSE_SAMPLES))?;
        for sample in samples {
            seq.serialize_element(sample)?;
        }
        seq.end()
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[f32; MAX_IMPULSE_SAMPLES], D::Error> {
        struct Samples;
        impl<'de> Visitor<'de> for Samples {
            type Value = [f32; MAX_IMPULSE_SAMPLES];
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "at most {MAX_IMPULSE_SAMPLES} impulse samples")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut samples = [0.0; MAX_IMPULSE_SAMPLES];
                for sample in &mut samples {
                    match seq.next_element()? {
                        Some(value) => *sample = value,
                        None => return Ok(samples),
                    }
                }
                if seq.next_element::<f32>()?.is_some() {
                    return Err(A::Error::custom("impulse exceeds 2048 samples"));
                }
                Ok(samples)
            }
        }
        deserializer.deserialize_seq(Samples)
    }
}

#[derive(Clone, Copy, Default)]
struct Complex {
    re: f32,
    im: f32,
}
impl Complex {
    const ZERO: Self = Self { re: 0.0, im: 0.0 };
    fn mul(self, b: Self) -> Self {
        Self {
            re: self.re * b.re - self.im * b.im,
            im: self.re * b.im + self.im * b.re,
        }
    }
}
/// Fixed radix-2 FFT plan with no callback scratch allocation.
struct Fft {
    roots: [Complex; FFT_SIZE / 2],
}
impl Default for Fft {
    fn default() -> Self {
        Self {
            roots: std::array::from_fn(|i| {
                let a = -std::f32::consts::TAU * i as f32 / FFT_SIZE as f32;
                Complex {
                    re: a.cos(),
                    im: a.sin(),
                }
            }),
        }
    }
}
impl Fft {
    fn transform(&self, data: &mut [Complex; FFT_SIZE], inverse: bool) {
        for i in 0..FFT_SIZE {
            let j = i.reverse_bits() >> (usize::BITS - FFT_SIZE.ilog2());
            if j > i {
                data.swap(i, j);
            }
        }
        let mut width = 2;
        while width <= FFT_SIZE {
            for start in (0..FFT_SIZE).step_by(width) {
                for i in 0..width / 2 {
                    let mut root = self.roots[i * FFT_SIZE / width];
                    if inverse {
                        root.im = -root.im;
                    }
                    let a = data[start + i];
                    let b = data[start + i + width / 2].mul(root);
                    data[start + i] = Complex {
                        re: a.re + b.re,
                        im: a.im + b.im,
                    };
                    data[start + i + width / 2] = Complex {
                        re: a.re - b.re,
                        im: a.im - b.im,
                    };
                }
            }
            width *= 2;
        }
        if inverse {
            for value in data {
                value.re /= FFT_SIZE as f32;
                value.im /= FFT_SIZE as f32;
            }
        }
    }
}

/// Uniform 128-sample partitioned overlap-save convolution. Stereo histories
/// are independent; the mono IR is shared. Two banks crossfade live IR edits.
pub struct Convolver {
    params: ConvolverParams,
    fft: Fft,
    kernels: Vec<[Complex; FFT_SIZE]>,
    old_kernels: Vec<[Complex; FFT_SIZE]>,
    history: Vec<[Complex; FFT_SIZE]>,
    previous: [[f32; CONVOLUTION_PARTITION]; 2],
    input: [[f32; CONVOLUTION_PARTITION]; 2],
    dry: [[f32; CONVOLUTION_PARTITION]; 2],
    wet: [[[f32; CONVOLUTION_PARTITION]; 2]; 2],
    cursor: usize,
    head: usize,
    controls: Controls<2>,
    fade: LinearRamp,
    fade_samples: u32,
    pending_ir: bool,
    fresh: bool,
}
impl Default for Convolver {
    fn default() -> Self {
        let mut effect = Self {
            params: ConvolverParams::default(),
            fft: Fft::default(),
            kernels: vec![[Complex::ZERO; FFT_SIZE]; PARTITIONS],
            old_kernels: vec![[Complex::ZERO; FFT_SIZE]; PARTITIONS],
            history: vec![[Complex::ZERO; FFT_SIZE]; 2 * PARTITIONS],
            previous: [[0.0; CONVOLUTION_PARTITION]; 2],
            input: [[0.0; CONVOLUTION_PARTITION]; 2],
            dry: [[0.0; CONVOLUTION_PARTITION]; 2],
            wet: [[[0.0; CONVOLUTION_PARTITION]; 2]; 2],
            cursor: 0,
            head: 0,
            controls: Controls::new([1.0, 1.0]),
            fade: LinearRamp::new(1.0),
            fade_samples: 240,
            pending_ir: false,
            fresh: true,
        };
        effect.build_kernels();
        effect.old_kernels.copy_from_slice(&effect.kernels);
        effect
    }
}
impl Convolver {
    fn build_kernels(&mut self) {
        for p in 0..PARTITIONS {
            let block = &mut self.kernels[p];
            block.fill(Complex::ZERO);
            for (i, value) in block.iter_mut().take(CONVOLUTION_PARTITION).enumerate() {
                let n = p * CONVOLUTION_PARTITION + i;
                value.re = if self.params.impulse_length == 0 {
                    if n == 0 { 1.0 } else { 0.0 }
                } else {
                    self.params.impulse[n]
                };
            }
            self.fft.transform(block, false);
        }
    }
    fn finish_partition(&mut self) {
        for ch in 0..2 {
            let block = &mut self.history[ch * PARTITIONS + self.head];
            for i in 0..CONVOLUTION_PARTITION {
                block[i] = Complex {
                    re: self.previous[ch][i],
                    im: 0.0,
                };
                block[i + CONVOLUTION_PARTITION] = Complex {
                    re: self.input[ch][i],
                    im: 0.0,
                };
            }
            self.fft.transform(block, false);
            for bank in 0..2 {
                let kernels = if bank == 0 {
                    &self.old_kernels
                } else {
                    &self.kernels
                };
                let mut sum = [Complex::ZERO; FFT_SIZE];
                for (p, kernel) in kernels.iter().enumerate() {
                    let past =
                        &self.history[ch * PARTITIONS + (self.head + PARTITIONS - p) % PARTITIONS];
                    for i in 0..FFT_SIZE {
                        let product = past[i].mul(kernel[i]);
                        sum[i].re += product.re;
                        sum[i].im += product.im;
                    }
                }
                self.fft.transform(&mut sum, true);
                for i in 0..CONVOLUTION_PARTITION {
                    self.wet[bank][ch][i] = output(sum[i + CONVOLUTION_PARTITION].re);
                }
            }
            self.previous[ch] = self.input[ch];
            self.dry[ch] = self.input[ch];
        }
        self.head = (self.head + 1) % PARTITIONS;
        if self.pending_ir {
            self.fade.set_target(1.0, self.fade_samples);
            self.pending_ir = false;
        }
    }
}
impl Effect for Convolver {
    type Params = ConvolverParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        let sr = rate(sample_rate);
        self.fade_samples = (sr * 0.005).round() as u32;
        self.controls.prepare(sr);
        self.reset();
    }
    fn reset(&mut self) {
        self.history.fill([Complex::ZERO; FFT_SIZE]);
        self.previous.fill([0.0; CONVOLUTION_PARTITION]);
        self.input.fill([0.0; CONVOLUTION_PARTITION]);
        self.dry.fill([0.0; CONVOLUTION_PARTITION]);
        self.wet.fill([[0.0; CONVOLUTION_PARTITION]; 2]);
        self.old_kernels.copy_from_slice(&self.kernels);
        self.cursor = 0;
        self.head = 0;
        self.fade.snap(1.0);
        self.pending_ir = false;
        self.fresh = true;
        self.controls.reset();
    }
    fn set_params(&mut self, params: &Self::Params) {
        let p = params.sanitized();
        self.controls.set([p.mix, p.gain]);
        if p.impulse_length != self.params.impulse_length || p.impulse != self.params.impulse {
            let t = self.fade.value();
            // Preserve queued audio when interrupting an existing fade.
            for ch in 0..2 {
                for i in 0..CONVOLUTION_PARTITION {
                    let current =
                        self.wet[0][ch][i] + (self.wet[1][ch][i] - self.wet[0][ch][i]) * t;
                    self.wet[0][ch][i] = current;
                    self.wet[1][ch][i] = current;
                }
            }
            for (old, new) in self.old_kernels.iter_mut().zip(&self.kernels) {
                for (a, b) in old.iter_mut().zip(new) {
                    a.re += (b.re - a.re) * t;
                    a.im += (b.im - a.im) * t;
                }
            }
            self.params = p;
            self.build_kernels();
            if self.fresh {
                self.old_kernels.copy_from_slice(&self.kernels);
                self.fade.snap(1.0);
            } else {
                self.fade.snap(0.0);
                self.pending_ir = true;
            }
        } else {
            self.params = p;
        }
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (l, r) in left.iter_mut().zip(right) {
            self.fresh = false;
            let [mix, gain] = self.controls.tick();
            let fade = self.fade.tick();
            for (ch, sample) in [l, r].into_iter().enumerate() {
                self.input[ch][self.cursor] = audio(*sample);
                let old = self.wet[0][ch][self.cursor];
                let new = self.wet[1][ch][self.cursor];
                *sample = output(
                    (self.dry[ch][self.cursor] * (1.0 - mix) + (old + (new - old) * fade) * mix)
                        * gain,
                );
            }
            self.cursor += 1;
            if self.cursor == CONVOLUTION_PARTITION {
                self.finish_partition();
                self.cursor = 0;
            }
        }
    }
    fn latency_samples(&self) -> usize {
        CONVOLUTION_PARTITION
    }
    fn tail_samples(&self) -> usize {
        CONVOLUTION_PARTITION + MAX_IMPULSE_SAMPLES - 1
    }
    fn gap_samples(&self) -> usize {
        self.tail_samples()
    }
}
