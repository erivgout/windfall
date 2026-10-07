//! Nothing that runs on the audio thread may touch the allocator.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use windfall_stretch::{Quality, Stretcher};

use crate::support::{RATE, Rng, noise};

thread_local! {
    static WATCHING: Cell<bool> = const { Cell::new(false) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
}

/// The system allocator, counting every call made on a thread while that
/// thread is inside [`allocator_calls`].
pub struct CountingAllocator;

fn count() {
    // The thread-locals hold plain values with no destructor, so reading
    // them here cannot allocate. `try_with` covers a thread being torn down.
    if WATCHING.try_with(Cell::get).unwrap_or(false) {
        let _ = CALLS.try_with(|calls| calls.set(calls.get() + 1));
    }
}

// SAFETY: every method forwards to the system allocator unchanged. Counting
// reads and writes thread-local cells and nothing else.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: the caller upholds `GlobalAlloc::alloc`'s contract.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: the caller upholds `GlobalAlloc::alloc_zeroed`'s contract.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        count();
        // SAFETY: the caller upholds `GlobalAlloc::dealloc`'s contract.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count();
        // SAFETY: the caller upholds `GlobalAlloc::realloc`'s contract.
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

/// Runs `work` and returns how many times it allocated, reallocated or
/// freed memory.
fn allocator_calls(work: impl FnOnce()) -> usize {
    CALLS.set(0);
    WATCHING.set(true);
    work();
    WATCHING.set(false);
    CALLS.get()
}

#[test]
fn the_counter_sees_allocations_and_frees() {
    let mut kept = None;
    assert_eq!(allocator_calls(|| kept = Some(vec![1_u8; 64])), 1);
    assert_eq!(allocator_calls(|| drop(kept.take())), 1);
    assert_eq!(allocator_calls(|| assert_eq!(2 + 2, 4)), 0);
}

#[test]
fn the_stretcher_never_allocates_after_it_is_built() {
    const SIZES: [usize; 9] = [128, 1, 7, 64, 480, 512, 33, 256, 2_048];
    for quality in Quality::ALL {
        for channels in [1, 2] {
            // Everything the control side would do ahead of time: build
            // the stretcher and line up the audio it is going to be given.
            let mut stretcher = Stretcher::with_quality(channels, RATE, quality);
            let mut rng = Rng::new(channels as u32 + 40);
            let source: Vec<Vec<f32>> = (0..channels)
                .map(|channel| {
                    let mut audio = noise(channel as u32 + 1, 0.5, 64_000);
                    // A stretch of silence, which is handled on its own.
                    audio[20_000..40_000].fill(0.0);
                    audio
                })
                .collect();
            let mut output = vec![vec![0.0_f32; 2_048]; channels];
            let mut loudest = 0.0_f32;
            let mut position = 0;

            let calls = allocator_calls(|| {
                for step in 0..600_usize {
                    match step % 40 {
                        3 => stretcher.set_time_ratio(0.25 + f64::from(rng.unipolar()) * 3.75),
                        9 => stretcher.set_pitch_semitones(f64::from(rng.bipolar()) * 24.0),
                        14 => stretcher.set_formant_preservation(step % 80 < 40),
                        21 => stretcher.reset(),
                        27 => {
                            let frames = stretcher.seek_frames().min(position);
                            let pre_roll: [&[f32]; 2] = [
                                &source[0][position - frames..position],
                                &source[channels - 1][position - frames..position],
                            ];
                            stretcher.seek(&pre_roll[..channels]);
                        }
                        33 => {
                            let mut tail: [&mut [f32]; 2] = [&mut [], &mut []];
                            for (tail, output) in tail.iter_mut().zip(output.iter_mut()) {
                                *tail = &mut output[..300];
                            }
                            stretcher.flush(&mut tail[..channels]);
                        }
                        _ => {}
                    }
                    let frames = SIZES[step % SIZES.len()];
                    let needed = stretcher.input_frames_needed(frames);
                    if position + needed > source[0].len() {
                        position = 0;
                    }
                    let input: [&[f32]; 2] =
                        [&source[0][position..], &source[channels - 1][position..]];
                    let mut into: [&mut [f32]; 2] = [&mut [], &mut []];
                    for (into, output) in into.iter_mut().zip(output.iter_mut()) {
                        *into = &mut output[..frames];
                    }
                    position += stretcher.process(&input[..channels], &mut into[..channels]);
                    loudest = loudest.max(output[0][0].abs());
                    let _ = (
                        stretcher.latency(),
                        stretcher.seek_frames(),
                        stretcher.time_ratio(),
                        stretcher.pitch_semitones(),
                    );
                }
            });
            assert_eq!(
                calls, 0,
                "{quality:?} with {channels} channels touched the allocator"
            );
            assert!(loudest > 0.0);
        }
    }
}
