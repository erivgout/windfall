//! Separate executable: the library's existing global allocator stays intact.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use crate::instrument::Instrument;
use crate::param::ParamSet;
use crate::speech::{Phoneme, PhonemeBuffer, SpeechVoice, SpeechVoiceParams};

thread_local! {
    static WATCH: Cell<bool> = const { Cell::new(false) };
    static COUNTS: Cell<[usize; 3]> = const { Cell::new([0; 3]) };
}

struct ProbeAllocator;

fn count(index: usize) {
    if WATCH.try_with(Cell::get).unwrap_or(false) {
        let _ = COUNTS.try_with(|counts| {
            let mut value = counts.get();
            value[index] += 1;
            counts.set(value);
        });
    }
}

unsafe impl GlobalAlloc for ProbeAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count(0);
        // SAFETY: forward the caller's layout contract unchanged.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count(0);
        // SAFETY: forward the caller's layout contract unchanged.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count(1);
        // SAFETY: forward the caller's live pointer and layout unchanged.
        unsafe { System.realloc(pointer, layout, size) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        count(2);
        // SAFETY: forward the caller's live pointer and layout unchanged.
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: ProbeAllocator = ProbeAllocator;

fn start() {
    COUNTS.with(|counts| counts.set([0; 3]));
    WATCH.with(|watch| watch.set(true));
}

fn stop() -> [usize; 3] {
    WATCH.with(|watch| watch.set(false));
    COUNTS.with(Cell::get)
}

pub fn run() {
    // Prove that the observer sees all three operations before trusting zero.
    let layout = Layout::from_size_align(16, 8).unwrap();
    let resized = Layout::from_size_align(32, 8).unwrap();
    start();
    // SAFETY: matching valid layouts, with each live pointer checked and freed.
    unsafe {
        let pointer = std::alloc::alloc(layout);
        if pointer.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        std::hint::black_box(pointer);
        let next = std::alloc::realloc(pointer, layout, 32);
        if next.is_null() {
            std::alloc::dealloc(pointer, layout);
            std::alloc::handle_alloc_error(resized);
        }
        std::hint::black_box(next);
        std::alloc::dealloc(next, resized);
    }
    assert_eq!(stop(), [1, 1, 1]);

    let mut voice = SpeechVoice::default();
    voice.prepare(48_000.0, 128);
    let mut params = SpeechVoiceParams::default();
    let mut left = [0.0; 128];
    let mut right = [0.0; 128];
    start();
    for index in 0..400 {
        params.phrase = PhonemeBuffer::from_slice(&[
            Phoneme::Ah,
            Phoneme::Ee,
            Phoneme::Oo,
            Phoneme::Eh,
            Phoneme::Oh,
            Phoneme::NoiseBurst,
            Phoneme::Nasal,
            Phoneme::Silence,
        ]);
        params.set(index % SpeechVoiceParams::descriptors().len(), f32::MAX);
        voice.set_params(&params);
        voice.note_on((index % 128) as u8, 0.8);
        voice.process(&mut left, &mut right);
        voice.note_off((index % 128) as u8);
        voice.process(&mut left, &mut right);
        voice.all_notes_off();
        voice.set_tempo(f32::NAN);
        voice.note_on(60, f32::NAN);
        voice.reset();
        std::hint::black_box((&left, &right));
    }
    // Exercise phoneme boundaries, phrase completion and release termination.
    voice.set_params(&SpeechVoiceParams {
        rate: 30.0,
        ..Default::default()
    });
    voice.note_on(60, 1.0);
    for _ in 0..300 {
        voice.process(&mut left, &mut right);
    }
    voice.reset();
    assert_eq!(stop(), [0, 0, 0], "callback alloc/realloc/free");
}
