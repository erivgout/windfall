//! Standalone allocator test executable, launched by lush::tests.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use windfall_dsp::lush::{LushSpace, LushSpaceParams};
use windfall_dsp::{Effect, ParamSet};

thread_local! {
    static WATCH: Cell<bool> = const { Cell::new(false) };
    static COUNTS: Cell<[usize; 3]> = const { Cell::new([0; 3]) };
}

struct Probe;

fn count(index: usize) {
    if WATCH.try_with(Cell::get).unwrap_or(false) {
        let _ = COUNTS.try_with(|counts| {
            let mut values = counts.get();
            values[index] += 1;
            counts.set(values);
        });
    }
}

unsafe impl GlobalAlloc for Probe {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count(0);
        // SAFETY: forward the unchanged allocation contract to System.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count(0);
        // SAFETY: forward the unchanged allocation contract to System.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count(1);
        // SAFETY: forward the unchanged allocation contract to System.
        unsafe { System.realloc(pointer, layout, size) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        count(2);
        // SAFETY: forward the unchanged allocation contract to System.
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Probe = Probe;

fn watched(work: impl FnOnce()) -> [usize; 3] {
    struct Stop;
    impl Drop for Stop {
        fn drop(&mut self) {
            WATCH.with(|watch| watch.set(false));
        }
    }
    COUNTS.with(|counts| counts.set([0; 3]));
    WATCH.with(|watch| watch.set(true));
    let stop = Stop;
    work();
    drop(stop);
    COUNTS.with(Cell::get)
}

#[test]
fn probe_detects_alloc_realloc_and_free() {
    let layout = Layout::from_size_align(64, 8).unwrap();
    let calls = watched(|| {
        // SAFETY: the same allocator owns the allocation and matching layouts.
        unsafe {
            let pointer = ALLOCATOR.alloc(layout);
            assert!(!pointer.is_null());
            let pointer = ALLOCATOR.realloc(pointer, layout, 128);
            assert!(!pointer.is_null());
            ALLOCATOR.dealloc(pointer, Layout::from_size_align(128, 8).unwrap());
        }
    });
    assert_eq!(calls, [1, 1, 1]);
}

#[test]
fn callbacks_have_zero_alloc_realloc_free() {
    for rate in [1_000.0, 48_000.0] {
        let mut effect = LushSpace::default();
        let mut params = LushSpaceParams::default();
        let mut left = [0.0; 257];
        let mut right = left;
        let descriptors = LushSpaceParams::descriptors();
        // Cover unprepared callback methods too.
        assert_eq!(
            watched(|| {
                effect.set_params(&params);
                effect.set_tempo(f32::NAN);
                effect.reset();
                effect.process(&mut left, &mut right);
            }),
            [0; 3]
        );
        effect.prepare(rate, 257);
        let calls = watched(|| {
            for iteration in 0..100 {
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
                left.fill(0.1);
                right.fill(-0.2);
                effect.process(&mut left[..1], &mut right[..1]);
                effect.process(&mut left[1..19], &mut right[1..19]);
                effect.process(&mut left[19..], &mut right[19..]);
                if iteration % 7 == 0 {
                    effect.reset();
                }
            }
            // Include full decay, the bounded fade and history-clear path.
            params.decay_s = 8.0;
            effect.set_params(&params);
            for _ in 0..effect.tail_samples() / 257 + 2 {
                left.fill(0.0);
                right.fill(0.0);
                effect.process(&mut left, &mut right);
            }
            effect.process(&mut [], &mut []);
            effect.reset();
            let _ = effect.tail_samples();
            let _ = effect.gap_samples();
            let _ = effect.latency_samples();
        });
        assert_eq!(calls, [0; 3], "alloc/realloc/free at {rate} Hz");
    }
}
