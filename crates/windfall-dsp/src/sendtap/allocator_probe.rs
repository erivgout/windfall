//! Isolated allocator probe linked to the production Windfall DSP library.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use windfall_dsp::Effect;
use windfall_dsp::sendtap::{SendTap, SendTapParams};

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

// SAFETY: pointers and layouts are forwarded unchanged to System.
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
fn probe_observes_alloc_realloc_and_free() {
    for zeroed in [false, true] {
        let layout = Layout::from_size_align(64, 8).unwrap();
        CALLS.with(|calls| calls.set([0; 3]));
        WATCH.with(|watch| watch.set(true));
        // SAFETY: allocate with valid layouts and free with the final layout.
        unsafe {
            let pointer = if zeroed {
                ALLOCATOR.alloc_zeroed(layout)
            } else {
                ALLOCATOR.alloc(layout)
            };
            assert!(!pointer.is_null());
            let pointer = ALLOCATOR.realloc(pointer, layout, 128);
            assert!(!pointer.is_null());
            ALLOCATOR.dealloc(pointer, Layout::from_size_align(128, 8).unwrap());
        }
        WATCH.with(|watch| watch.set(false));
        assert_eq!(CALLS.with(Cell::get), [1, 1, 1]);
    }
}

#[test]
fn callback_has_zero_alloc_realloc_free() {
    let mut effect = SendTap::default();
    effect.prepare(48_000.0, 257);
    let mut left = [0.25; 257];
    let mut right = [-0.5; 257];
    let mut send_left = [0.0; 257];
    let mut send_right = [0.0; 257];
    CALLS.with(|calls| calls.set([0; 3]));
    WATCH.with(|watch| watch.set(true));
    for iteration in 0..128 {
        let level = [0.0, 0.25, 1.0, f32::NAN][iteration % 4];
        effect.set_params(&std::hint::black_box(SendTapParams { level }));
        effect.set_tempo(120.0);
        for (start, end) in [(0, 0), (0, 1), (1, 18), (18, 257)] {
            effect.process(&mut left[start..end], &mut right[start..end]);
            effect.send(
                &left[start..end],
                &right[start..end],
                &mut send_left[start..end],
                &mut send_right[start..end],
            );
            std::hint::black_box((&send_left, &send_right));
        }
        effect.reset();
    }
    WATCH.with(|watch| watch.set(false));
    assert_eq!(CALLS.with(Cell::get), [0, 0, 0], "[alloc, realloc, free]");
}
