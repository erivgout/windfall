//! Standalone spatial test entry point when sibling families are incomplete.
//! Link the actual built windfall-dsp rlib (see the seam doc); never duplicate
//! processor source. The allocator belongs only to this test executable.
pub use windfall_dsp::{Effect, ParamSet, echo_bank, spatial::*};

#[path = "tests.rs"]
mod tests;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
thread_local! {
    static WATCH: Cell<bool> = const { Cell::new(false) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
}
struct WatchedAllocator;
fn count() {
    if WATCH.try_with(Cell::get).unwrap_or(false) {
        let _ = CALLS.try_with(|n| n.set(n.get() + 1));
    }
}
unsafe impl GlobalAlloc for WatchedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: forward the caller's unchanged allocator contract.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: forward the caller's unchanged allocator contract.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count();
        // SAFETY: forward the caller's unchanged allocator contract.
        unsafe { System.realloc(p, layout, size) }
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        count();
        // SAFETY: forward the caller's unchanged allocator contract.
        unsafe { System.dealloc(p, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: WatchedAllocator = WatchedAllocator;

fn calls(work: impl FnOnce()) -> usize {
    struct Stop;
    impl Drop for Stop {
        fn drop(&mut self) {
            WATCH.with(|w| w.set(false));
        }
    }
    CALLS.with(|c| c.set(0));
    WATCH.with(|w| w.set(true));
    let stop = Stop;
    work();
    drop(stop);
    CALLS.with(Cell::get)
}
#[test]
fn spatial_callbacks_never_allocate_reallocate_or_free() {
    fn check<E: Effect + Default>() {
        let mut fx = E::default();
        fx.prepare(1000.0, 31);
        let mut params = E::Params::default();
        let (mut l, mut r) = ([0.0; 31], [0.0; 31]);
        let n = calls(|| {
            for block in 0..300 {
                let i = block % E::Params::descriptors().len();
                params.set(i, if block % 2 == 0 { f32::MAX } else { f32::NAN });
                fx.set_params(&params);
                fx.set_tempo(f32::NAN);
                l.fill(0.0);
                r.fill(0.0);
                l[0] = 0.1;
                fx.process(&mut l, &mut r);
                if block % 37 == 0 {
                    fx.reset();
                }
            }
            // Include the expiry path that clears every recursive history.
            for _ in 0..(fx.tail_samples() / 31 + 2) {
                l.fill(0.0);
                r.fill(0.0);
                fx.process(&mut l, &mut r);
            }
            fx.reset();
            fx.process(&mut [], &mut []);
        });
        assert_eq!(n, 0, "{} allocator calls", E::Params::NAME);
    }
    check::<VintageChorus>();
    check::<HyperChorus>();
    check::<VintagePhaser>();
    check::<StackedFlanger>();
    check::<BandDelay>();
    check::<Room>();
    check::<Spreader>();
    check::<StereoEnhancer>();
}
#[test]
fn spatial_allocator_guard_positive_controls() {
    let mut held = None;
    assert!(calls(|| held = Some(vec![1_u8; 64])) > 0);
    assert!(calls(|| drop(held.take())) > 0);
}
