//! Short synthesized clicks, rendered without allocation on the output clock.
use crate::mixer::{Frame, MAX_BLOCK};
use windfall_ipc::MetronomeSettings;

pub(crate) struct CountIn {
    pub start: u64,
    pub frames_per_beat: f64,
    pub beats: u32,
    pub beats_per_bar: u32,
    pub emitted: u32,
}
impl CountIn {
    pub fn end(&self) -> u64 { self.start.saturating_add((self.frames_per_beat * f64::from(self.beats)).ceil() as u64) }
    pub fn next(&self) -> u64 { self.start.saturating_add((self.frames_per_beat * f64::from(self.emitted)).ceil() as u64) }
}

pub(crate) struct Metronome {
    settings: MetronomeSettings,
    phase: f32,
    phase_step: f32,
    age: u32,
    duration: u32,
    amplitude: f32,
    block: [f32; MAX_BLOCK],
}
impl Metronome {
    pub fn new() -> Self { Self { settings: MetronomeSettings::default(), phase: 0.0, phase_step: 0.0, age: 0, duration: 0, amplitude: 0.0, block: [0.0; MAX_BLOCK] } }
    pub fn configure(&mut self, settings: MetronomeSettings) {
        self.settings = settings;
        if !settings.enabled { self.duration = 0; }
    }
    pub fn clear_block(&mut self) { self.block.fill(0.0); }
    pub fn trigger(&mut self, accent: bool, sample_rate: u32) {
        let accent = accent && self.settings.accent;
        self.phase = 0.0;
        self.phase_step = std::f32::consts::TAU * if accent { 2100.0 } else { 1400.0 } / sample_rate.max(1) as f32;
        self.age = 0;
        self.duration = (sample_rate / 40).max(1);
        self.amplitude = self.settings.gain * if accent { 1.0 } else { 0.7 };
    }
    pub fn render(&mut self, from: usize, to: usize) {
        for sample in &mut self.block[from..to] {
            if self.age >= self.duration { break; }
            let progress = self.age as f32 / self.duration as f32;
            let envelope = (1.0 - progress).powi(3) * (self.age as f32 / 8.0).min(1.0);
            *sample += self.phase.sin() * envelope * self.amplitude;
            self.phase = (self.phase + self.phase_step) % std::f32::consts::TAU;
            self.age += 1;
        }
    }
    pub fn render_count_in(&mut self, count: &mut CountIn, shared: &crate::shared::Shared, base: u64, from: usize, to: usize, rate: u32) {
        let mut cursor = from;
        while count.emitted < count.beats && count.next() < base + to as u64 {
            let offset = count.next().saturating_sub(base) as usize;
            self.render(cursor, offset.max(cursor).min(to));
            self.trigger(count.emitted % count.beats_per_bar == 0, rate);
            shared.count_in_remaining.store(count.beats - count.emitted, std::sync::atomic::Ordering::Release);
            count.emitted += 1;
            cursor = offset.max(cursor).min(to);
        }
        self.render(cursor, to);
    }
    /// Runtime output only: click audio does not enter mixer meters or stems.
    pub fn mix(&self, out: &mut [Frame]) {
        for (frame, click) in out.iter_mut().zip(&self.block) { frame[0] += click; frame[1] += click; }
    }
}
