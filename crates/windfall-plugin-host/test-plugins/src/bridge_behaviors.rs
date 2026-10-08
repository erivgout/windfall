//! Shared deterministic DSP for newly appended bridge fixtures.

pub(crate) const DELAY: usize = 37;
/// Native cap exercised with valid8KiB stream chunks, never persisted assets.
pub(crate) const STATE_LIMIT: usize = 256 << 20;
pub(crate) fn hang() -> ! {
    loop {
        std::thread::park();
    }
}

pub(crate) struct DelayedEffect {
    samples: [[f32; 2]; DELAY],
    cursor: usize,
}
impl Default for DelayedEffect {
    fn default() -> Self {
        Self {
            samples: [[0.0; 2]; DELAY],
            cursor: 0,
        }
    }
}
impl DelayedEffect {
    pub(crate) fn reset(&mut self) {
        self.samples.fill([0.0; 2]);
        self.cursor = 0;
    }
    pub(crate) fn tick(&mut self, input: [f32; 2], gain: f32) -> [f32; 2] {
        let output = self.samples[self.cursor];
        self.samples[self.cursor] = [input[0] * gain, input[1] * gain];
        self.cursor += 1;
        if self.cursor == DELAY {
            self.cursor = 0;
        }
        output
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) enum CaptureFault {
    #[default]
    None,
    Exit,
    Hang,
    PartialStream,
}
pub(crate) fn before_capture(fault: CaptureFault, value: f64) -> bool {
    if value < 0.75 {
        return true;
    }
    match fault {
        CaptureFault::None => true,
        CaptureFault::Exit => std::process::exit(83),
        CaptureFault::Hang => hang(),
        CaptureFault::PartialStream => false,
    }
}
