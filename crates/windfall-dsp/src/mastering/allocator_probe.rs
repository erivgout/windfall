//! Standalone probe to avoid competing global allocators in library tests.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use windfall_dsp::Effect;
use windfall_dsp::mastering::{StageStack, StageStackParams};

thread_local! {
    static WATCH: Cell<bool> = const { Cell::new(false) };
    static CALLS: Cell<[usize; 3]> = const { Cell::new([0; 3]) };
}

struct Probe;

fn count(index: usize) {
    if WATCH.try_with(Cell::get).unwrap_or(false) {
        let _ = CALLS.try_with(|calls| {
            let mut counts = calls.get();
            counts[index] += 1;
            calls.set(counts);
        });
    }
}

// SAFETY: every pointer and layout is forwarded unchanged to System.
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
fn mastering_callback_has_zero_alloc_realloc_free() {
    let mut effect = StageStack::default();
    effect.prepare(48_000.0, 128);
    let mut left = [0.0; 128];
    let mut right = left;
    CALLS.with(|calls| calls.set([0; 3]));
    WATCH.with(|watch| watch.set(true));
    for block in 0..128 {
        for (n, sample) in left.iter_mut().enumerate() {
            *sample = ((block * 128 + n) as f32 * 0.1).sin() * 8.0;
        }
        right.copy_from_slice(&left);
        effect.set_params(&StageStackParams {
            drive_db: if block % 2 == 0 { 36.0 } else { f32::NAN },
            tone: block as f32 / 127.0,
            ceiling_db: -((block % 25) as f32),
            mix: (block % 3) as f32 * 0.5,
        });
        effect.set_tempo(120.0);
        effect.process(&mut left, &mut right);
        effect.process(&mut [], &mut []);
        if block % 7 == 0 {
            effect.reset();
        }
        std::hint::black_box((&left, &right, effect.latency_samples()));
    }
    effect.reset();
    WATCH.with(|watch| watch.set(false));
    assert_eq!(CALLS.with(Cell::get), [0, 0, 0], "[alloc, realloc, free]");
}

#[test]
fn mastering_probe_observes_all_three_operations() {
    let layout = Layout::from_size_align(64, 8).unwrap();
    CALLS.with(|calls| calls.set([0; 3]));
    WATCH.with(|watch| watch.set(true));
    // SAFETY: valid allocation layouts; the final allocation is released once.
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
