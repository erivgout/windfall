//! Standalone test root for drums while other DSP families are under construction.
//! Compile with rustc --test, using the workspace's built dependency rlibs.
//! This imports the real Instrument/ParamSet/blocks, without altering the registry.
#![allow(dead_code)]

#[path = "../blocks/mod.rs"]
mod blocks;
#[path = "mod.rs"]
mod drums;
#[path = "../instrument.rs"]
mod instrument;
#[path = "../note_expression.rs"]
mod note_expression;
#[path = "../param.rs"]
mod param;
#[path = "../synth.rs"]
mod synth;

pub use note_expression::{NoteExpression, NoteInstanceId};

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static WATCH: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
    static FREES: Cell<usize> = const { Cell::new(0) };
}

struct CountingAllocator;
fn count(counter: &'static std::thread::LocalKey<Cell<usize>>) {
    if WATCH.try_with(Cell::get).unwrap_or(false) {
        let _ = counter.try_with(|n| n.set(n.get() + 1));
    }
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count(&ALLOCATIONS);
        // SAFETY: forward the caller's allocator contract unchanged.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count(&ALLOCATIONS);
        // SAFETY: forward the caller's allocator contract unchanged.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count(&ALLOCATIONS);
        count(&FREES);
        // SAFETY: forward the caller's allocator contract unchanged.
        unsafe { System.realloc(pointer, layout, size) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        count(&FREES);
        // SAFETY: forward the caller's allocator contract unchanged.
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn drums_realtime_methods_never_allocate_or_free() {
    use instrument::Instrument;
    use param::ParamSet;
    fn exercise<I: Instrument + Default>() {
        let mut drum = I::default();
        drum.prepare(48000.0, 128);
        let mut params = I::Params::default();
        let mut left = [0.0; 128];
        let mut right = [0.0; 128];
        ALLOCATIONS.with(|n| n.set(0));
        FREES.with(|n| n.set(0));
        WATCH.with(|n| n.set(true));
        for index in 0..300 {
            params.set(index % I::Params::descriptors().len(), f32::MAX);
            drum.set_params(&params);
            drum.note_on((index % 128) as u8, 1.0);
            drum.process(&mut left, &mut right);
            drum.note_off((index % 128) as u8);
            drum.note_on(36, f32::NAN);
            if index % 17 == 0 {
                drum.all_notes_off();
            }
            if index % 31 == 0 {
                drum.reset();
            }
        }
        drum.reset();
        drum.set_tempo(f32::NAN);
        WATCH.with(|n| n.set(false));
        assert_eq!(ALLOCATIONS.with(Cell::get), 0);
        assert_eq!(FREES.with(Cell::get), 0);
    }
    exercise::<drums::Membrane>();
    exercise::<drums::DrumRack>();
    exercise::<drums::Kick>();
    exercise::<drums::DrumVoice>();
}
