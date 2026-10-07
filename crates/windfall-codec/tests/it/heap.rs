//! Measures how much heap a piece of code holds at once, for the tests that
//! bound the memory a hostile file can make the decoder take.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

/// The system allocator, with a count of the bytes held by each thread that
/// asked to be measured. Tests run side by side on their own threads, so one
/// test's allocations never show up in another's count.
pub struct Metered;

thread_local! {
    static MEASURING: Cell<bool> = const { Cell::new(false) };
    static HELD: Cell<isize> = const { Cell::new(0) };
    static PEAK: Cell<isize> = const { Cell::new(0) };
}

/// Adds `change` bytes to what this thread holds, if it is being measured.
fn record(change: isize) {
    // The cells are gone while a thread shuts down. Nothing measures then.
    let _ = MEASURING.try_with(|measuring| {
        if !measuring.get() {
            return;
        }
        let _ = HELD.try_with(|held| {
            held.set(held.get() + change);
            let _ = PEAK.try_with(|peak| peak.set(peak.get().max(held.get())));
        });
    });
}

// SAFETY: every request goes to the system allocator unchanged; the counting
// beside it touches only thread-local cells and never allocates.
unsafe impl GlobalAlloc for Metered {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's contract is passed straight on.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record(layout.size() as isize);
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's contract is passed straight on.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            record(layout.size() as isize);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: the caller's contract is passed straight on.
        unsafe { System.dealloc(pointer, layout) };
        record(-(layout.size() as isize));
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller's contract is passed straight on.
        let moved = unsafe { System.realloc(pointer, layout, new_size) };
        if !moved.is_null() {
            // Growing may copy into a new block before the old one is freed,
            // so both count for a moment.
            record(new_size as isize);
            record(-(layout.size() as isize));
        }
        moved
    }
}

/// Runs `work` and returns its result with the most bytes of heap this thread
/// held at any moment during it, over what it held when it started. Whatever
/// `work` returns is part of that count.
pub fn peak_heap<T>(work: impl FnOnce() -> T) -> (T, usize) {
    HELD.with(|held| held.set(0));
    PEAK.with(|peak| peak.set(0));
    MEASURING.with(|measuring| measuring.set(true));
    let result = work();
    MEASURING.with(|measuring| measuring.set(false));
    (result, PEAK.with(|peak| peak.get()).max(0) as usize)
}
