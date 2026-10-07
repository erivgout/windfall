//! Keeping what a plugin does wrong inside the plugin.
//!
//! A plugin runs in the app's process, so the host cannot stop it from
//! crashing or from taking too long. It can stop the damage from spreading
//! through the mix: samples that are not numbers never leave the plugin's
//! slot, slow blocks are counted where the app can see them, and a plugin
//! that reports a failure is not called again.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

/// The loudest sample let out of a plugin: 60 dB over full scale, the same
/// bound the built-in effect slots put on what they let in.
pub const OUTPUT_LIMIT: f32 = 1.0e3;

/// Makes every sample a plain number within [`OUTPUT_LIMIT`]. Samples that
/// are not numbers become silence, and so do denormals, which would slow
/// down whatever comes next. Returns how many samples were not numbers.
pub fn scrub(samples: &mut [f32]) -> u32 {
    let mut bad = 0;
    for sample in samples {
        if !sample.is_finite() {
            *sample = 0.0;
            bad += 1;
        } else if sample.is_subnormal() {
            *sample = 0.0;
        } else {
            *sample = sample.clamp(-OUTPUT_LIMIT, OUTPUT_LIMIT);
        }
    }
    bad
}

/// What the audio thread records about a plugin, for any thread to read.
#[derive(Debug, Default)]
pub(crate) struct HealthCells {
    pub failed: AtomicBool,
    pub overruns: AtomicU32,
    pub worst_overrun_micros: AtomicU32,
    pub scrubbed_samples: AtomicU64,
    pub dropped_events: AtomicU32,
}

impl HealthCells {
    pub fn read(&self) -> PluginHealth {
        PluginHealth {
            failed: self.failed.load(Ordering::Relaxed),
            overruns: self.overruns.load(Ordering::Relaxed),
            worst_overrun_micros: self.worst_overrun_micros.load(Ordering::Relaxed),
            scrubbed_samples: self.scrubbed_samples.load(Ordering::Relaxed),
            dropped_events: self.dropped_events.load(Ordering::Relaxed),
        }
    }
}

/// How a plugin has behaved on the audio thread since it was activated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PluginHealth {
    /// The plugin reported a failure from `process` and is no longer
    /// called. An effect passes its input through and an instrument is
    /// silent until the plugin is activated again.
    pub failed: bool,
    /// Blocks that took longer to process than they last when played. One
    /// of these on a live device is a dropout. An offline render sets none:
    /// see [`PluginProcessor::set_realtime`](crate::PluginProcessor::set_realtime).
    pub overruns: u32,
    /// The longest any block ran over, in microseconds.
    pub worst_overrun_micros: u32,
    /// Output samples that were not numbers and were replaced by silence.
    pub scrubbed_samples: u64,
    /// Events rejected as invalid or lost because a queue or translated
    /// event buffer was full.
    pub dropped_events: u32,
}

/// While this exists, the processor rounds denormal numbers to zero instead
/// of computing with them, which can be a hundred times slower. Plugins
/// expect the host to have set this on the audio thread. The previous
/// setting comes back when the guard is dropped.
pub(crate) struct NoDenormals {
    saved: u64,
}

impl NoDenormals {
    #[inline]
    pub fn enter() -> Self {
        let saved = control::read();
        control::write(saved | control::FLUSH_BITS);
        Self { saved }
    }
}

impl Drop for NoDenormals {
    #[inline]
    fn drop(&mut self) {
        control::write(self.saved);
    }
}

#[cfg(target_arch = "x86_64")]
mod control {
    use std::arch::asm;

    /// Flush-to-zero and denormals-are-zero in MXCSR.
    pub const FLUSH_BITS: u64 = (1 << 15) | (1 << 6);

    #[inline]
    pub fn read() -> u64 {
        let mut value = 0_u32;
        // SAFETY: `stmxcsr` writes four bytes to the address it is given,
        // which is a live `u32`.
        unsafe {
            asm!("stmxcsr dword ptr [{}]", in(reg) &raw mut value, options(nostack, preserves_flags));
        }
        u64::from(value)
    }

    #[inline]
    pub fn write(value: u64) {
        let value = value as u32;
        // SAFETY: `ldmxcsr` reads four bytes from a live `u32`. Only the
        // two denormal bits ever differ from what `read` returned, and the
        // only code that runs while they are set is the plugin's.
        unsafe {
            asm!("ldmxcsr dword ptr [{}]", in(reg) &raw const value, options(nostack, preserves_flags, readonly));
        }
    }
}

#[cfg(target_arch = "aarch64")]
mod control {
    use std::arch::asm;

    /// Flush-to-zero in FPCR.
    pub const FLUSH_BITS: u64 = 1 << 24;

    #[inline]
    pub fn read() -> u64 {
        let value: u64;
        // SAFETY: reading FPCR has no effect on anything else.
        unsafe {
            asm!("mrs {}, fpcr", out(reg) value, options(nomem, nostack, preserves_flags));
        }
        value
    }

    #[inline]
    pub fn write(value: u64) {
        // SAFETY: only the flush-to-zero bit ever differs from what `read`
        // returned, and the only code that runs while it is set is the
        // plugin's.
        unsafe {
            asm!("msr fpcr, {}", in(reg) value, options(nomem, nostack, preserves_flags));
        }
    }
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
mod control {
    pub const FLUSH_BITS: u64 = 0;

    pub fn read() -> u64 {
        0
    }

    pub fn write(_value: u64) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrubbing_removes_what_is_not_a_number_and_keeps_the_rest() {
        let mut samples = [
            0.5,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            -0.25,
            1.0e-40,
            5.0e3,
            -5.0e3,
        ];
        assert_eq!(scrub(&mut samples), 3);
        assert_eq!(
            samples,
            [0.5, 0.0, 0.0, 0.0, -0.25, 0.0, OUTPUT_LIMIT, -OUTPUT_LIMIT]
        );
    }

    #[test]
    fn the_guard_flushes_denormals_and_then_puts_the_setting_back() {
        let tiny = || std::hint::black_box(f32::MIN_POSITIVE) * std::hint::black_box(0.5_f32);
        let before = control::read();
        {
            let _guard = NoDenormals::enter();
            if control::FLUSH_BITS != 0 {
                assert_eq!(tiny(), 0.0);
            }
        }
        assert_eq!(control::read(), before);
        assert!(tiny() > 0.0);
    }
}
