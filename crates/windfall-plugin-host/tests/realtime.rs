//! The audio path must not touch the allocator.
//!
//! The host's own code is what is being measured. The test plugins are
//! written not to allocate while they process either, so every call counted
//! here would be one of the host's.

mod common;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use common::{GAIN, MIDI_SINE, MONO, SIDECHAIN, SINE, SWAP, create, gain, signal};
use windfall_plugin_host::{HostEvent, Transport};

thread_local! {
    static WATCHING: Cell<bool> = const { Cell::new(false) };
    static CALLS: Cell<usize> = const { Cell::new(0) };
}

/// The system allocator, counting every call made on a thread while that
/// thread is inside [`allocator_calls`].
struct CountingAllocator;

fn count() {
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

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// Runs `work` and returns how many times it allocated, reallocated or
/// freed memory.
fn allocator_calls(work: impl FnOnce()) -> usize {
    CALLS.set(0);
    WATCHING.set(true);
    work();
    WATCHING.set(false);
    CALLS.get()
}

const RATE: f64 = 48_000.0;

#[test]
fn the_counter_counts() {
    assert_eq!(
        allocator_calls(|| drop(std::hint::black_box(Box::new(1_u8)))),
        2
    );
}

#[test]
fn an_effect_processes_without_allocating() {
    let (_module, mut instance) = create(GAIN);
    let mut processor = instance.activate(RATE, 256).unwrap();
    let (mut left, mut right) = signal(1000);
    // A parameter change that crosses from the main thread.
    instance.set_param(gain::MODE, 1.0);

    let calls = allocator_calls(|| {
        for block in 0..50 {
            // Events of every kind, a new transport, and a block longer
            // than the plugin's maximum, which is split.
            processor.set_param(block, gain::GAIN, 0.5 + f64::from(block) * 0.01);
            processor.set_param(700, gain::GAIN, 1.0);
            // The key that makes the plugin send a gesture back.
            processor.note_on(10, 127, 0.25);
            processor.push_event(HostEvent::AllNotesOff { time: 20 });
            processor.set_transport(Transport {
                playing: true,
                tempo_bpm: 128.0,
                position_beats: f64::from(block),
                ..Transport::default()
            });
            processor.process(&mut left, &mut right);
            let _ = processor.latency_samples();
            let _ = processor.tail_samples();
            let _ = processor.health();
        }
        processor.reset();
        processor.stop();
        processor.process(&mut left, &mut right);
    });
    assert_eq!(calls, 0);
    assert_eq!(processor.health().dropped_events, 0);
    instance.deactivate(processor);
}

#[test]
fn an_instrument_processes_without_allocating() {
    for id in [SINE, MIDI_SINE] {
        let (_module, mut instance) = create(id);
        let mut processor = instance.activate(RATE, 512).unwrap();
        let mut left = vec![0.0_f32; 512];
        let mut right = vec![0.0_f32; 512];
        let calls = allocator_calls(|| {
            for block in 0..50_u8 {
                processor.note_on(3, 40 + block, 0.5);
                processor.note_off(400, 40 + block);
                processor.set_param(100, common::SINE_LEVEL, 0.25);
                processor.process(&mut left, &mut right);
            }
            processor.all_notes_off(0);
            processor.process(&mut left, &mut right);
        });
        assert_eq!(calls, 0, "{id}");
        instance.deactivate(processor);
    }
}

#[test]
fn unusual_port_layouts_process_without_allocating() {
    for id in [SWAP, SIDECHAIN, MONO] {
        let (_module, mut instance) = create(id);
        let mut processor = instance.activate(RATE, 128).unwrap();
        let (mut left, mut right) = signal(300);
        let calls = allocator_calls(|| {
            for _ in 0..20 {
                processor.process(&mut left, &mut right);
            }
        });
        assert_eq!(calls, 0, "{id}");
        instance.deactivate(processor);
    }
}

#[test]
fn a_full_event_queue_drops_events_without_allocating() {
    let (_module, mut instance) = create(GAIN);
    let mut processor = instance.activate(RATE, 256).unwrap();
    let (mut left, mut right) = signal(256);
    let calls = allocator_calls(|| {
        for index in 0..3_000_u32 {
            processor.set_param(index % 256, gain::GAIN, 1.0);
        }
        processor.process(&mut left, &mut right);
    });
    assert_eq!(calls, 0);
    assert_eq!(
        processor.health().dropped_events as usize,
        3_000 - windfall_plugin_host::EVENT_CAPACITY
    );
    instance.deactivate(processor);
}

#[test]
fn the_adapters_process_without_allocating() {
    let (_module, mut instance) = create(GAIN);
    let mut effect = instance.prepare_effect(48_000.0, 256).unwrap();
    let (mut left, mut right) = signal(256);
    let calls = allocator_calls(|| {
        for block in 0..20 {
            effect.set_param(0, 0.5);
            effect.set_tempo(100.0 + block as f32);
            effect.process(&mut left, &mut right);
            let _ = (
                effect.latency_samples(),
                effect.tail_samples(),
                effect.gap_samples(),
            );
        }
        effect.reset();
    });
    assert_eq!(calls, 0);
    instance.release_effect(effect);

    let (_sine_module, mut sine) = create(SINE);
    let mut instrument = sine.prepare_instrument(48_000.0, 256).unwrap();
    let calls = allocator_calls(|| {
        for block in 0..20_u8 {
            instrument.note_on(60 + block, 0.8);
            instrument.process(&mut left, &mut right);
            instrument.note_off(60 + block);
            instrument.process(&mut left, &mut right);
            let _ = (instrument.active_voices(), instrument.tail_samples());
        }
        instrument.all_notes_off();
        instrument.process(&mut left, &mut right);
    });
    assert_eq!(calls, 0);
    sine.release_instrument(instrument);
}

#[test]
fn translated_midi_overflow_is_bounded_and_reported_without_allocating() {
    let (_module, mut instance) = create(MIDI_SINE);
    let mut processor = instance.activate(RATE, 64).unwrap();
    let mut left = [0.0; 64];
    let mut right = [0.0; 64];
    let calls = allocator_calls(|| {
        for _ in 0..windfall_plugin_host::EVENT_CAPACITY {
            assert!(processor.all_notes_off(0));
        }
        processor.process(&mut left, &mut right);
    });
    assert_eq!(calls, 0);
    assert_eq!(
        processor.health().dropped_events,
        (windfall_plugin_host::EVENT_CAPACITY * 32 - (windfall_plugin_host::EVENT_CAPACITY + 32))
            as u32
    );
    instance.deactivate(processor);
}
