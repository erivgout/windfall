//! Standalone allocation tests against the real compiled windfall-dsp library.
//! A separate test binary avoids replacing the library's test allocator.
//! Compile with rustc --test and the package's built dependency rlibs.
#![allow(dead_code)]

use windfall_dsp::drive;
mod effect {
    pub use windfall_dsp::Effect;
}
mod param {
    pub use windfall_dsp::ParamSet;
}
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static WATCH: Cell<bool> = const { Cell::new(false) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
}
struct CountingAllocator;
fn count() {
    if WATCH.try_with(Cell::get).unwrap_or(false) {
        let _ = CALLS.try_with(|n| n.set(n.get() + 1));
    }
}
// SAFETY: forward every allocation/deallocation unchanged to System.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count();
        unsafe { System.realloc(p, layout, size) }
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        count();
        unsafe { System.dealloc(p, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn drive_callbacks_never_allocate_or_free() {
    use effect::Effect;
    use param::ParamSet;
    fn exercise<E: Effect + Default>() {
        let mut effect = E::default();
        effect.prepare(48_000.0, 64);
        let mut params = E::Params::default();
        let mut left = [0.25; 64];
        let mut right = [-0.2; 64];
        CALLS.with(|n| n.set(0));
        WATCH.with(|n| n.set(true));
        for edit in 0..128 {
            for (index, info) in E::Params::descriptors().iter().enumerate() {
                params.set(index, if edit % 2 == 0 { info.max } else { info.min });
            }
            effect.set_params(&params);
            effect.set_tempo(120.0);
            effect.process(&mut left, &mut right);
            if edit % 11 == 0 {
                effect.reset();
            }
        }
        effect.process(&mut [], &mut []);
        WATCH.with(|n| n.set(false));
        assert_eq!(
            CALLS.with(Cell::get),
            0,
            "{} callback heap activity",
            E::Params::NAME
        );
    }
    exercise::<drive::Waveshaper>();
    exercise::<drive::Overdrive>();
    exercise::<drive::GuitarRack>();
    exercise::<drive::DriveChain>();
}

#[test]
fn drive_allocator_guard_observes_allocations_and_frees() {
    CALLS.with(|n| n.set(0));
    WATCH.with(|n| n.set(true));
    let layout = Layout::from_size_align(64, 8).unwrap();
    // SAFETY: allocate a valid layout and free the returned pointer with
    // that same layout. No audio processor or ownership state is involved.
    unsafe {
        let p = std::alloc::alloc_zeroed(layout);
        assert!(!p.is_null());
        std::alloc::dealloc(p, layout);
    }
    WATCH.with(|n| n.set(false));
    assert_eq!(CALLS.with(Cell::get), 2);
}
