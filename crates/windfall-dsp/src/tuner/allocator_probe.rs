//! Standalone allocator probe against the compiled Windfall DSP library.
//! Kept outside the library test modules to avoid competing global allocators.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use windfall_dsp::Effect;
use windfall_dsp::tuner::{Tuner, TunerParams};

thread_local! {
    static WATCH: Cell<bool> = const { Cell::new(false) };
    static CALLS: Cell<[usize; 3]> = const { Cell::new([0; 3]) };
}

struct Probe;
fn count(index: usize) {
    if WATCH.try_with(Cell::get).unwrap_or(false) {
        let _ = CALLS.try_with(|calls| {
            let mut values = calls.get();
            values[index] += 1;
            calls.set(values);
        });
    }
}

// SAFETY: all pointers and layouts are forwarded unchanged to System.
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
fn tuner_callback_has_zero_alloc_realloc_free() {
    let mut tuner = Tuner::default();
    tuner.prepare(48_000.0, 128);
    let mut left = [0.0; 128];
    let mut right = left;
    CALLS.with(|calls| calls.set([0; 3]));
    WATCH.with(|watch| watch.set(true));
    for block in 0..128 {
        for (i, sample) in left.iter_mut().enumerate() {
            *sample = (std::f32::consts::TAU * 440.0 * (block * 128 + i) as f32 / 48_000.0).sin();
        }
        right.copy_from_slice(&left);
        tuner.set_params(&TunerParams {
            reference_hz: 442.0,
        });
        tuner.set_tempo(120.0);
        tuner.process(&mut left, &mut right);
        std::hint::black_box(tuner.readout());
    }
    tuner.process(&mut [], &mut []);
    tuner.reset();
    WATCH.with(|watch| watch.set(false));
    assert_eq!(CALLS.with(Cell::get), [0, 0, 0], "[alloc, realloc, free]");
}

#[test]
fn tuner_allocator_probe_observes_all_three_operations() {
    let layout = Layout::from_size_align(64, 8).unwrap();
    CALLS.with(|calls| calls.set([0; 3]));
    WATCH.with(|watch| watch.set(true));
    // SAFETY: use valid layouts and release the allocation with its final layout.
    unsafe {
        let pointer = std::alloc::alloc_zeroed(layout);
        assert!(!pointer.is_null());
        let pointer = std::alloc::realloc(pointer, layout, 128);
        assert!(!pointer.is_null());
        std::alloc::dealloc(pointer, Layout::from_size_align(128, 8).unwrap());
    }
    WATCH.with(|watch| watch.set(false));
    assert_eq!(CALLS.with(Cell::get), [1, 1, 1]);
}
