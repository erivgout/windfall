//! Standalone test harness, using the compiled library's actual Instrument/ParamSet.
//! See docs/integration/seams/zones.md for the rustc invocation before registry wiring.
#![allow(dead_code)]

pub use windfall_dsp::{NoteExpression, NoteInstanceId};
mod instrument {
    pub use windfall_dsp::Instrument;
}
mod param {
    pub use windfall_dsp::{ParamChoice, ParamInfo, ParamKind, ParamScale, ParamSet, ParamUnit};
}
#[path = "mod.rs"]
mod zones;

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
// SAFETY: every operation is forwarded to System with the same pointer/layout.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count();
        unsafe { System.realloc(pointer, layout, size) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        count();
        unsafe { System.dealloc(pointer, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn allocator_guard_detects_both_allocations_and_frees() {
    let layout = Layout::from_size_align(64, 8).unwrap();
    CALLS.with(|calls| calls.set(0));
    WATCH.with(|watch| watch.set(true));
    // SAFETY: System pointer is returned to the same allocator/layout.
    unsafe {
        let pointer = ALLOCATOR.alloc(layout);
        assert!(!pointer.is_null());
        ALLOCATOR.dealloc(pointer, layout);
    }
    WATCH.with(|watch| watch.set(false));
    assert_eq!(CALLS.with(Cell::get), 2);
}

#[test]
fn every_instruments_callbacks_allocate_and_free_nothing() {
    use instrument::Instrument;
    use param::ParamSet;
    use zones::*;
    let z = Zone {
        sample: Sample::from_slice(&[0.8; 1024], 8000.0).unwrap(),
        loop_mode: LoopMode::Continuous,
        loop_start: 0,
        loop_end: 1024,
        ..Default::default()
    };
    let table = ZoneTable::from_slice(&[z; MAX_ZONES]).unwrap();
    macro_rules! exercise {
        ($instrument:ident, $params:ident) => {{
            let mut params = $params {
                zones: table,
                ..Default::default()
            };
            let mut instrument = $instrument::new(&params);
            instrument.prepare(8000.0, 64);
            let (mut left, mut right) = ([0.0; 64], [0.0; 64]);
            CALLS.with(|calls| calls.set(0));
            WATCH.with(|watch| watch.set(true));
            for edit in 0..64 {
                for (index, info) in $params::descriptors().iter().enumerate() {
                    params.set(index, if edit % 2 == 0 { info.min } else { info.max });
                }
                instrument.set_params(&params);
                instrument.set_tempo(120.0);
                let id = NoteInstanceId(edit);
                instrument.note_on_instance(
                    id,
                    36 + (edit % 16) as u8,
                    0.8,
                    0.0,
                    NoteExpression::default(),
                );
                instrument.note_on_expression(36, 0.7, 0.2, NoteExpression::default());
                instrument.note_on(37, 0.9);
                instrument.set_note_expression(id, -0.2, NoteExpression::default());
                instrument.set_note_pitch(id, 48.5);
                instrument.process(&mut left, &mut right);
                instrument.note_off_instance(id, 36);
                instrument.note_off(37);
                instrument.all_notes_off();
                instrument.process(&mut left, &mut right);
                let _ = (
                    instrument.active_voices(),
                    instrument.latency_samples(),
                    instrument.tail_samples(),
                );
                instrument.reset();
            }
            instrument.process(&mut [], &mut []);
            WATCH.with(|watch| watch.set(false));
            assert_eq!(
                CALLS.with(Cell::get),
                0,
                "{} callback heap activity",
                $params::NAME
            );
        }};
    }
    exercise!(ZoneSampler, ZoneSamplerParams);
    exercise!(ZonePlayer, ZonePlayerParams);
    exercise!(PadSampler, PadSamplerParams);
    exercise!(KeyBed, KeyBedParams);
}
