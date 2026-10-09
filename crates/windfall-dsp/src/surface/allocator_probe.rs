//! Standalone probe linked against the production Windfall DSP library.
//! A separate executable avoids competing library test global allocators.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use windfall_dsp::Effect;
use windfall_dsp::surface::{ControlSurface, ControlSurfaceParams};

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

// SAFETY: pointers, sizes and layouts are forwarded unchanged to System.
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
fn surface_callback_has_zero_alloc_realloc_free() {
    let mut surface = ControlSurface::default();
    surface.prepare(48_000.0, 128);
    let mut left = [0.25; 128];
    let mut right = [-0.75; 128];
    CALLS.with(|calls| calls.set([0; 3]));
    WATCH.with(|watch| watch.set(true));
    for block in 0..128 {
        let value = block as f32 / 127.0;
        surface.set_params(&std::hint::black_box(ControlSurfaceParams {
            knob1: value,
            knob2: 1.0 - value,
            knob3: f32::NAN,
            knob4: -1.0,
            knob5: 2.0,
            knob6: f32::INFINITY,
            knob7: f32::NEG_INFINITY,
            knob8: value,
        }));
        surface.set_tempo(120.0);
        surface.process(
            std::hint::black_box(&mut left),
            std::hint::black_box(&mut right),
        );
        std::hint::black_box(surface.readout());
        surface.reset();
        std::hint::black_box(surface.readout());
    }
    surface.process(&mut [], &mut []);
    surface.prepare(96_000.0, 4096);
    WATCH.with(|watch| watch.set(false));
    assert_eq!(CALLS.with(Cell::get), [0, 0, 0], "[alloc, realloc, free]");
    assert_eq!(left, [0.25; 128]);
    assert_eq!(right, [-0.75; 128]);
    assert_eq!(surface.readout(), [1.0, 0.0, 0.5, 0.0, 1.0, 0.5, 0.5, 1.0]);
}

#[test]
fn surface_allocator_probe_observes_all_three_operations() {
    let layout = Layout::from_size_align(64, 8).unwrap();
    CALLS.with(|calls| calls.set([0; 3]));
    WATCH.with(|watch| watch.set(true));
    // SAFETY: valid layouts, with deallocation using the final allocation size.
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
