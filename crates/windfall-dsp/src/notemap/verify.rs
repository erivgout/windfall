//! Standalone allocator probe: the crate's unit tests already have an allocator.
//! Run verify.sh from Git Bash after sourcing scripts/msvc-env.sh.
#![allow(dead_code)]

pub use windfall_dsp::blocks;
#[path = "mod.rs"]
pub mod notemap;
#[path = "../param.rs"]
mod param;

use notemap::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static WATCH: Cell<bool> = const { Cell::new(false) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
}
struct Probe;
fn count() {
    if WATCH.try_with(Cell::get).unwrap_or(false) {
        let _ = CALLS.try_with(|n| n.set(n.get() + 1));
    }
}
// SAFETY: every operation forwards the caller's contract unchanged to System.
unsafe impl GlobalAlloc for Probe {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count();
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        count();
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Probe = Probe;

struct Watch;
impl Watch {
    fn start() -> Self {
        CALLS.with(|n| n.set(0));
        WATCH.with(|n| n.set(true));
        Self
    }
}
impl Drop for Watch {
    fn drop(&mut self) {
        WATCH.with(|n| n.set(false));
    }
}

#[test]
fn notemap_transform_never_allocates_or_frees() {
    use param::ParamSet;
    fn exercise<T: NoteTransform>() {
        let mut processor = T::default();
        let mut params = T::Params::default();
        let mut notes = [MappedNote {
            key: 60,
            velocity: 0.75,
            channel: 0,
            color: 0,
        }; 128];
        let watch = Watch::start();
        for pass in 0..256 {
            for (index, info) in T::Params::descriptors().iter().enumerate() {
                params.set(index, if pass % 2 == 0 { info.min } else { info.max });
            }
            processor.set_params(&params);
            notes[0].velocity = f32::NAN;
            std::hint::black_box(processor.transform(std::hint::black_box(&mut notes), 128));
            std::hint::black_box(processor.transform(&mut [], usize::MAX));
        }
        drop(watch);
        assert_eq!(
            CALLS.with(Cell::get),
            0,
            "{} heap activity",
            T::Params::NAME
        );
    }
    exercise::<ColorMap>();
    exercise::<LevelScale>();
    exercise::<KeyMap>();
    exercise::<KeySplit>();
    exercise::<StepGrid>();
    let mut grid = StepGrid::default();
    let mut notes = [MappedNote::default(); 16];
    let watch = Watch::start();
    for _ in 0..256 {
        grid.set_tempo(120.0);
        grid.advance_samples(6000, 48_000.0);
        std::hint::black_box(grid.transform(&mut notes, 16));
        grid.reset();
    }
    drop(watch);
    assert_eq!(CALLS.with(Cell::get), 0);
}

#[test]
fn notemap_probe_observes_heap_activity() {
    let layout = Layout::from_size_align(64, 8).unwrap();
    let watch = Watch::start();
    // SAFETY: use a valid layout; free the same non-null pointer with that layout.
    unsafe {
        let ptr = std::alloc::alloc_zeroed(layout);
        assert!(!ptr.is_null());
        std::alloc::dealloc(ptr, layout);
    }
    drop(watch);
    assert_eq!(CALLS.with(Cell::get), 2);
}
