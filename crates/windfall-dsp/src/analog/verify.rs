//! Standalone callback allocator probe. See docs/integration/seams/analog.md.
//! A separate binary coexists with the library's existing test allocator.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use windfall_dsp::analog::{AcidLine, MacroVoice, TripleOsc, WaveLane};
use windfall_dsp::{Instrument, ParamSet};

thread_local! {
    static WATCH: Cell<bool> = const { Cell::new(false) };
    static COUNTS: Cell<[usize; 3]> = const { Cell::new([0; 3]) };
}
struct Probe;
fn count(index: usize) {
    if WATCH.try_with(Cell::get).unwrap_or(false) {
        let _ = COUNTS.try_with(|counts| {
            let mut value = counts.get();
            value[index] += 1;
            counts.set(value);
        });
    }
}
// SAFETY: every pointer, layout and size is forwarded unchanged to System.
unsafe impl GlobalAlloc for Probe {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count(0);
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count(0);
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count(1);
        unsafe { System.realloc(pointer, layout, size) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        count(2);
        unsafe { System.dealloc(pointer, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Probe = Probe;

#[test]
fn analog_probe_detects_alloc_realloc_and_free() {
    COUNTS.with(|c| c.set([0; 3]));
    WATCH.with(|w| w.set(true));
    let old = Layout::from_size_align(64, 8).unwrap();
    // SAFETY: use valid layouts, check allocations, and free with the resized layout.
    unsafe {
        let pointer = std::alloc::alloc_zeroed(old);
        assert!(!pointer.is_null());
        let pointer = std::alloc::realloc(pointer, old, 128);
        assert!(!pointer.is_null());
        std::alloc::dealloc(pointer, Layout::from_size_align(128, 8).unwrap());
    }
    WATCH.with(|w| w.set(false));
    assert_eq!(COUNTS.with(Cell::get), [1, 1, 1]);
}

#[test]
fn analog_callbacks_do_zero_alloc_realloc_or_free() {
    fn exercise<I: Instrument + Default>() {
        let mut synth = I::default();
        synth.prepare(48000.0, 64);
        let mut p = I::Params::default();
        let mut left = [0.0; 64];
        let mut right = [0.0; 64];
        COUNTS.with(|c| c.set([0; 3]));
        WATCH.with(|w| w.set(true));
        for edit in 0..256 {
            for (index, descriptor) in I::Params::descriptors().iter().enumerate() {
                p.set(
                    index,
                    if edit % 2 == 0 {
                        descriptor.max
                    } else {
                        descriptor.min
                    },
                );
            }
            synth.set_params(&p);
            synth.set_tempo(if edit % 2 == 0 { 91.0 } else { f32::NAN });
            for key in 48..60 {
                synth.note_on(key, 0.8);
            }
            synth.note_on(48, f32::NAN);
            synth.process(&mut left, &mut right);
            synth.note_off(59);
            if edit % 5 == 0 {
                synth.all_notes_off();
            }
            synth.process(&mut left, &mut right);
            if edit % 7 == 0 {
                synth.reset();
            }
            let _ = (
                synth.active_voices(),
                synth.latency_samples(),
                synth.tail_samples(),
            );
        }
        // Exercise internal sequencer boundaries and whole pattern wraps,
        // not only external note triggers and short callback blocks.
        synth.reset();
        p = I::Params::default();
        if let Some(index) = I::Params::index_of("sequencer") {
            p.set(index, 1.0);
            p.set(I::Params::index_of("steps.0.slide").unwrap(), 1.0);
            p.set(I::Params::index_of("steps.1.accent").unwrap(), 1.0);
            p.set(I::Params::index_of("steps.2.enabled").unwrap(), 0.0);
        }
        synth.set_params(&p);
        synth.set_tempo(400.0);
        synth.note_on(48, 0.8);
        for _ in 0..1024 {
            synth.process(&mut left, &mut right);
        }
        synth.note_off(48);
        synth.all_notes_off();
        for _ in 0..8 {
            synth.process(&mut left, &mut right);
        }
        synth.process(&mut [], &mut []);
        synth.reset();
        WATCH.with(|w| w.set(false));
        assert_eq!(
            COUNTS.with(Cell::get),
            [0, 0, 0],
            "{} callback heap activity",
            I::Params::NAME
        );
    }
    exercise::<AcidLine>();
    exercise::<TripleOsc>();
    exercise::<WaveLane>();
    exercise::<MacroVoice>();
}
