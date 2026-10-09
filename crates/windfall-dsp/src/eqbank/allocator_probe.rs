//! Separate test executable invoked by eqbank::tests::callback_allocator_probe.
//! It links the production library and watches only the calling test thread.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use windfall_dsp::eqbank::{FilterBank, MorphEq, SevenBand};
use windfall_dsp::{Effect, ParamSet};

thread_local! {
    static WATCH: Cell<bool> = const { Cell::new(false) };
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
    static REALLOCS: Cell<usize> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
}
struct Probe;
fn count(counter: &'static std::thread::LocalKey<Cell<usize>>) {
    if WATCH.try_with(Cell::get).unwrap_or(false) {
        let _ = counter.try_with(|n| n.set(n.get() + 1));
    }
}
unsafe impl GlobalAlloc for Probe {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count(&ALLOCS);
        // SAFETY: forwarding the original allocation contract unchanged.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count(&ALLOCS);
        // SAFETY: forwarding the original allocation contract unchanged.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count(&REALLOCS);
        // SAFETY: forwarding the original allocation contract unchanged.
        unsafe { System.realloc(pointer, layout, size) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        count(&FREES);
        // SAFETY: forwarding the original allocation contract unchanged.
        unsafe { System.dealloc(pointer, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Probe = Probe;

#[test]
fn probe_detects_alloc_realloc_and_free() {
    let layout = Layout::from_size_align(64, 8).unwrap();
    ALLOCS.with(|n| n.set(0));
    REALLOCS.with(|n| n.set(0));
    FREES.with(|n| n.set(0));
    WATCH.with(|n| n.set(true));
    // SAFETY: use the same allocator and matching valid layouts throughout.
    unsafe {
        let pointer = ALLOCATOR.alloc(layout);
        assert!(!pointer.is_null());
        let pointer = ALLOCATOR.realloc(pointer, layout, 128);
        assert!(!pointer.is_null());
        ALLOCATOR.dealloc(pointer, Layout::from_size_align(128, 8).unwrap());
    }
    WATCH.with(|n| n.set(false));
    assert_eq!(ALLOCS.with(Cell::get), 1);
    assert_eq!(REALLOCS.with(Cell::get), 1);
    assert_eq!(FREES.with(Cell::get), 1);
}

fn exercise<E: Effect + Default>() {
    let mut effect = E::default();
    effect.prepare(48_000.0, 257);
    let mut params = E::Params::default();
    let descriptors = E::Params::descriptors();
    let mut left = [0.1; 257];
    let mut right = [-0.2; 257];
    ALLOCS.with(|n| n.set(0));
    REALLOCS.with(|n| n.set(0));
    FREES.with(|n| n.set(0));
    WATCH.with(|n| n.set(true));
    for iteration in 0..200 {
        for (index, info) in descriptors.iter().enumerate() {
            params.set(
                index,
                if iteration % 2 == 0 {
                    info.max
                } else {
                    info.min
                },
            );
        }
        effect.set_params(&params);
        effect.set_tempo(if iteration % 3 == 0 { f32::NAN } else { 137.0 });
        effect.process(&mut left[..1], &mut right[..1]);
        effect.process(&mut left[1..18], &mut right[1..18]);
        effect.process(&mut left[18..], &mut right[18..]);
        if iteration % 7 == 0 {
            effect.reset();
        }
        let _ = effect.latency_samples();
        let _ = effect.tail_samples();
    }
    effect.reset();
    WATCH.with(|n| n.set(false));
    assert_eq!(
        ALLOCS.with(Cell::get),
        0,
        "{} allocated in callback",
        E::Params::NAME
    );
    assert_eq!(
        REALLOCS.with(Cell::get),
        0,
        "{} reallocated in callback",
        E::Params::NAME
    );
    assert_eq!(
        FREES.with(Cell::get),
        0,
        "{} freed in callback",
        E::Params::NAME
    );
}

#[test]
fn callback_methods_have_zero_alloc_realloc_free() {
    exercise::<SevenBand>();
    exercise::<MorphEq>();
    exercise::<FilterBank>();
}
