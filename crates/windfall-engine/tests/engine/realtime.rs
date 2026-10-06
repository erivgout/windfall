//! The audio path must never touch the allocator.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use windfall_core::TICKS_PER_STEP;
use windfall_engine::SamplePool;
use windfall_ipc::{PlayMode, TransportPatch};
use windfall_project::{Envelope, Send};

use crate::support::{Rig, impulse, level, peak, sine};

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

/// A project that keeps every part of the engine busy: resampled and
/// pitched voices, envelopes, cut groups, sends, swing and a playlist.
fn busy_rig() -> Rig {
    let mut rig = Rig::new();
    rig.project.settings.tempo_bpm = 174.0;
    rig.project.settings.swing = 0.4;
    let drums = rig.track();
    let effects = rig.track();
    rig.track_mut(drums).sends.push(Send {
        target: effects,
        gain: 0.4,
    });

    let kick = rig.channel_on(sine(44_100, 60.0, 0.3), drums);
    rig.steps(kick, &[0, 4, 8, 12]);
    let hat = rig.channel_on(sine(48_000, 7_000.0, 0.2), drums);
    rig.sampler_mut(hat).cut_self = true;
    rig.sampler_mut(hat).cut_group = 1;
    rig.steps(hat, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
    let open_hat = rig.channel_on(sine(32_000, 5_000.0, 0.6), drums);
    rig.sampler_mut(open_hat).cut_group = 1;
    rig.steps(open_hat, &[2, 10]);
    let pad = rig.channel_on(sine(44_100, 220.0, 2.0), effects);
    rig.sampler_mut(pad).envelope = Some(Envelope {
        attack_ms: 20.0,
        decay_ms: 100.0,
        sustain: 0.5,
        release_ms: 200.0,
    });
    for (index, key) in [57, 60, 64, 67].into_iter().enumerate() {
        rig.note(pad, index as u32 * 960, 900).key = key;
    }
    let click = rig.channel(impulse(48_000));
    rig.steps(click, &[0, 8]);

    let second = rig.pattern(8);
    rig.note_in(second, kick, 0, TICKS_PER_STEP);
    rig.note_in(second, pad, 480, 2_000).key = 72;
    let lane = rig.playlist_track();
    let first = rig.first_pattern();
    rig.clip(lane, first, 0, 3_840 * 2);
    rig.clip(lane, second, 1_920, 5_000).offset = 300;
    rig
}

#[test]
fn processing_never_allocates_or_frees() {
    let mut rig = busy_rig();
    let hat = rig.project.channels[1].id;
    let pad = rig.project.channels[3].id;
    let (mut processor, controller) = rig.processor(48_000);
    let preview = sine(22_050, 880.0, 0.5);

    let mut out = vec![0.0_f32; 4_096 * 2];
    let sizes = [128, 1, 7, 64, 480, 1_024, 4_096, 33];
    let mut calls = 0;
    let mut loudest = 0.0_f32;
    controller.play();

    for step in 0..600_u32 {
        // Everything the control side does between buffers may allocate.
        // What the audio thread does with it may not.
        match step % 120 {
            5 => controller.note_on(pad, 64, 0.9),
            9 => controller.note_on(hat, 60, 0.7),
            15 => controller.note_off(pad, 64),
            20 => controller.preview(preview.clone()),
            24 => controller.preview(level(48_000, 0.1, 0.05)),
            30 => controller.stop_preview(),
            35 => controller.seek(f64::from(step) * 7.5),
            40 => controller.set_transport(TransportPatch {
                mode: Some(PlayMode::Song),
                loop_song: Some(step % 240 < 120),
                ..TransportPatch::default()
            }),
            44 => controller.play(),
            60 => controller.stop(),
            62 => controller.play(),
            80 => controller.set_transport(TransportPatch {
                mode: Some(PlayMode::Pattern),
                pattern: Some(rig.project.patterns[(step as usize / 120) % 2].id),
                ..TransportPatch::default()
            }),
            82 => controller.play(),
            100 => controller.set_output_gain(if step % 240 < 120 { 0.5 } else { 1.0 }),
            _ => {}
        }
        if step % 3 == 0 {
            // An edit and a new plan: a fader, the tempo, a toggled step.
            rig.project.channels[0].volume = 0.5 + (step % 7) as f32 * 0.05;
            rig.project.mixer.tracks[1].pan = (step % 5) as f32 * 0.2 - 0.4;
            rig.project.settings.tempo_bpm = 120.0 + f64::from(step % 50);
            let muted = &mut rig.project.channels[2].muted;
            *muted = !*muted;
            controller.set_project(&rig.project, &rig.pool);
        }
        if step == 250 {
            // The sample of a sounding channel is swapped for another, and
            // the pool lets go of the old one, so a voice ends up as its
            // last owner.
            let replacement = rig.sample(sine(48_000, 330.0, 1.0));
            let old = rig.sampler_mut(pad).sample.replace(replacement);
            rig.pool.remove(old.expect("the pad had a sample"));
            controller.set_project(&rig.project, &rig.pool);
        }
        if step == 400 {
            // A channel and a mixer track disappear while they sound.
            rig.project.channels.retain(|channel| channel.id != hat);
            rig.project.mixer.tracks.remove(2);
            controller.set_project(&rig.project, &rig.pool);
        }

        let frames = sizes[step as usize % sizes.len()];
        let block = &mut out[..frames * 2];
        calls += allocator_calls(|| processor.process(block));
        loudest = loudest.max(peak(block));
    }

    assert_eq!(calls, 0, "the audio path used the allocator");
    assert!(loudest > 0.1, "the test played nothing");
    assert!(controller.frame().voices <= 320);
}

#[test]
fn a_voice_that_outlives_every_other_owner_of_its_sample_frees_nothing() {
    let mut rig = Rig::new();
    let channel = rig.channel(level(48_000, 0.5, 0.05));
    rig.steps(channel, &[0]);
    let (mut processor, controller) = rig.processor(48_000);
    controller.play();
    // 64 frames a call, so the 4 ms fade below spans several calls.
    let mut out = vec![0.0_f32; 128];
    assert_eq!(allocator_calls(|| processor.process(&mut out)), 0);
    assert_eq!(controller.frame().voices, 1);

    // The project moves on without the sample, and every handle to it on
    // this side is dropped.
    let empty = Rig::new();
    controller.set_project(&empty.project, &SamplePool::new());
    drop(rig);
    assert_eq!(allocator_calls(|| processor.process(&mut out)), 0);
    // This call drops the plan the audio thread just handed back. Now only
    // the fading voice holds the audio.
    assert_eq!(controller.frame().voices, 1);

    let mut calls = 0;
    for _ in 0..8 {
        calls += allocator_calls(|| processor.process(&mut out));
    }
    assert_eq!(
        calls, 0,
        "ending the voice freed its sample on the audio path"
    );
    // The audio went back to this side instead, and this call frees it.
    assert_eq!(controller.frame().voices, 0);
}
