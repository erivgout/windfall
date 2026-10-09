use crate::blocks::oscillator::sine;
use crate::blocks::smooth::LinearRamp;
use crate::instrument::Instrument;
use crate::param::ParamSet;

use super::common::{Bank, Controls, output, rate};
use super::{FourOpAlgorithm, FourOpParams};

/// Four sine operators with eight graphs and operator 4 feedback.
pub struct FourOp {
    sample_rate: f32,
    controls: Controls<FourOpParams>,
    bank: Bank<4>,
    // Destination-major edges, followed by the four carrier sends.
    graph: [LinearRamp; 20],
}

fn graph(algorithm: FourOpAlgorithm) -> [f32; 20] {
    use FourOpAlgorithm::*;
    let (edges, carriers): (&[(usize, usize)], &[usize]) = match algorithm {
        Stack => (&[(3, 2), (2, 1), (1, 0)], &[0]),
        Pairs => (&[(3, 2), (1, 0)], &[0, 2]),
        FanIn => (&[(3, 0), (2, 0), (1, 0)], &[0]),
        FanOut => (&[(3, 0), (3, 1), (3, 2)], &[0, 1, 2]),
        Branch => (&[(3, 2), (2, 0), (1, 0)], &[0]),
        Merge => (&[(3, 1), (2, 1), (1, 0)], &[0]),
        Triple => (&[(3, 2), (2, 1)], &[0, 1]),
        Parallel => (&[], &[0, 1, 2, 3]),
    };
    let mut weights = [0.0; 20];
    for &(source, destination) in edges {
        weights[destination * 4 + source] = 1.0;
    }
    for &carrier in carriers {
        weights[16 + carrier] = 1.0;
    }
    weights
}

impl Default for FourOp {
    fn default() -> Self {
        Self {
            sample_rate: 48_000.0,
            controls: Controls::default(),
            bank: Bank::default(),
            graph: graph(FourOpParams::default().algorithm).map(LinearRamp::new),
        }
    }
}

impl Instrument for FourOp {
    type Params = FourOpParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.sample_rate = rate(sample_rate);
        self.controls.prepare(self.sample_rate);
        self.reset();
    }
    fn reset(&mut self) {
        self.bank.reset();
        self.controls.reset();
        for (ramp, value) in self
            .graph
            .iter_mut()
            .zip(graph(self.controls.current.algorithm))
        {
            ramp.snap(value);
        }
    }
    fn set_params(&mut self, params: &Self::Params) {
        let p = params.sanitized();
        let samples = self.controls.ramp_samples();
        self.controls.set(&p);
        for (ramp, value) in self.graph.iter_mut().zip(graph(p.algorithm)) {
            ramp.set_target(value, samples);
        }
    }
    fn note_on(&mut self, key: u8, velocity: f32) {
        self.bank.note_on(key, velocity);
    }
    fn note_off(&mut self, key: u8) {
        self.bank.note_off(key);
    }
    fn all_notes_off(&mut self) {
        self.bank.all_notes_off(self.sample_rate);
    }
    fn active_voices(&self) -> usize {
        self.bank.active()
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        left.fill(0.0);
        right.fill(0.0);
        for (l, r) in left.iter_mut().zip(right) {
            let p = self.controls.tick();
            let weights = self.graph.each_mut().map(LinearRamp::tick);
            let normalization = weights[16..].iter().sum::<f32>().max(1.0);
            let mut mix = 0.0;
            for voice in &mut self.bank.voices {
                if !voice.active {
                    continue;
                }
                let mut values = [0.0; 4];
                for i in (0..4).rev() {
                    let op = &p.operators[i];
                    let env = voice.envelopes[i].tick(&op.envelope, self.sample_rate);
                    let mut modulation = 0.0;
                    for source in i + 1..4 {
                        modulation += weights[i * 4 + source] * values[source];
                    }
                    if i == 3 {
                        modulation += p.feedback * 0.5 * (voice.previous[3] + voice.older[3]);
                    }
                    let phase = voice.advance(i, op.ratio, self.sample_rate);
                    values[i] = sine(phase + modulation / std::f32::consts::TAU) * op.level * env;
                }
                voice.older = voice.previous;
                voice.previous = values;
                let sample =
                    (0..4).map(|i| values[i] * weights[16 + i]).sum::<f32>() / normalization;
                mix += sample * voice.gain();
                voice.finish_envelopes();
            }
            *l = output(mix * p.gain);
            *r = *l;
        }
    }
}
