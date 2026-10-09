use crate::blocks::oscillator::Waveform;
use crate::instrument::Instrument;

use super::WaveLaneParams;
use super::common::{Bank, Controls, Tables, increment, output, rate};

/// Eight wavetable voices scanning three original harmonic shapes.
pub struct WaveLane {
    sample_rate: f32,
    controls: Controls<WaveLaneParams>,
    pub(super) bank: Bank,
    tables: Tables,
}

impl Default for WaveLane {
    fn default() -> Self {
        Self {
            sample_rate: 48_000.0,
            controls: Controls::default(),
            bank: Bank::default(),
            tables: Tables::default(),
        }
    }
}

impl Instrument for WaveLane {
    type Params = WaveLaneParams;
    fn prepare(&mut self, sample_rate: f32, _max_block: usize) {
        self.sample_rate = rate(sample_rate);
        self.tables.generate();
        self.controls.prepare(self.sample_rate);
        self.reset();
    }
    fn reset(&mut self) {
        self.controls.reset();
        self.bank.reset();
    }
    fn set_params(&mut self, params: &Self::Params) {
        self.controls.set(params);
    }
    fn note_on(&mut self, key: u8, velocity: f32) {
        self.bank.note_on(key, velocity, self.sample_rate);
    }
    fn note_off(&mut self, key: u8) {
        self.bank.note_off(key);
    }
    fn all_notes_off(&mut self) {
        self.bank.stop(self.sample_rate);
    }
    fn active_voices(&self) -> usize {
        self.bank.active()
    }
    fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        for (left, right) in left.iter_mut().zip(right) {
            let p = self.controls.tick();
            let mut sum = 0.0;
            for voice in &mut self.bank.voices {
                if !voice.active() {
                    continue;
                }
                let inc = increment(voice.key as f32, self.sample_rate);
                let raw = self
                    .tables
                    .sample(voice.oscillators[0].phase(), inc, p.scan);
                voice.oscillators[0].tick(Waveform::Sine, inc, 0.5);
                let envelope = voice.envelope(p.envelope, self.sample_rate);
                let cutoff = p.cutoff_hz * (p.envelope_amount * envelope * 4.0).exp2();
                sum += voice.finish(raw, envelope, cutoff, p.resonance, self.sample_rate);
            }
            *left = output(sum * p.gain * 0.35);
            *right = *left;
        }
    }
}
