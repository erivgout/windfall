//! Deterministic acceptance checks for observational audio and click scheduling.
use crate::metronome::{CountIn, Metronome};
use crate::shared::Shared;
use crate::waveform_meter::{WaveAccumulator, WaveSlot};
use std::sync::atomic::Ordering;
use windfall_ipc::MetronomeSettings;
use windfall_project::TrackId;

fn click(gain: f32, accent: bool, setting_accent: bool) -> Vec<[f32; 2]> {
    let mut metro = Metronome::new();
    metro.configure(MetronomeSettings {
        enabled: true,
        gain,
        accent: setting_accent,
    });
    metro.trigger(accent, 8_000);
    metro.render(0, 256);
    let mut out = vec![[0.0; 2]; 256];
    metro.mix(&mut out);
    out
}

#[test]
fn qa_click_gain_stereo_accent_and_disable() {
    let quiet = click(0.25, false, true);
    let loud = click(0.5, false, true);
    assert!(quiet.iter().any(|frame| frame[0].abs() > 0.01));
    for (a, b) in quiet.iter().zip(&loud) {
        assert_eq!(a[0], a[1]);
        assert_eq!(a[0] * 2.0, b[0]);
    }
    assert_ne!(click(0.5, true, true), loud);
    assert_eq!(click(0.5, true, false), loud);
    assert!(loud[200..].iter().all(|frame| *frame == [0.0; 2]));
    let mut metro = Metronome::new();
    metro.configure(MetronomeSettings {
        enabled: true,
        gain: 1.0,
        accent: true,
    });
    metro.trigger(true, 8_000);
    metro.configure(MetronomeSettings {
        enabled: false,
        gain: 1.0,
        accent: true,
    });
    metro.render(0, 256);
    let mut out = [[0.0; 2]; 256];
    metro.mix(&mut out);
    assert!(out.iter().all(|frame| *frame == [0.0; 2]));
}

fn count_in_audio(block: usize) -> (Vec<[f32; 2]>, u32, u64) {
    let mut metro = Metronome::new();
    metro.configure(MetronomeSettings {
        enabled: false,
        gain: 0.5,
        accent: true,
    });
    let shared = Shared::new();
    let mut count = CountIn {
        start: 37,
        frames_per_beat: 250.25,
        beats: 3,
        beats_per_bar: 3,
        emitted: 0,
    };
    let mut out = Vec::new();
    for base in (0..1024).step_by(block) {
        let len = block.min(1024 - base);
        metro.clear_block();
        metro.render_count_in(&mut count, &shared, base as u64, 0, len, 8_000);
        let mut frames = vec![[0.0; 2]; len];
        metro.mix(&mut frames);
        out.extend(frames);
    }
    assert_eq!(shared.count_in_remaining.load(Ordering::Acquire), 1);
    (out, count.emitted, count.end())
}

#[test]
fn qa_count_in_fractional_deadlines_and_block_independent_audio() {
    let (audio, emitted, end) = count_in_audio(64);
    assert_eq!(emitted, 3);
    assert_eq!(end, 788);
    assert_eq!(audio, count_in_audio(127).0);
    assert!(audio[..38].iter().all(|frame| *frame == [0.0; 2]));
    for onset in [38, 289, 539] {
        assert!(audio[onset][0].abs() > 0.0, "missing click at {onset}");
    }
    let mut count = CountIn {
        start: u64::MAX - 2,
        frames_per_beat: 2.5,
        beats: 4,
        beats_per_bar: 4,
        emitted: 1,
    };
    assert_eq!(count.next(), u64::MAX);
    assert_eq!(count.end(), u64::MAX);
    count.emitted = 0;
    assert_eq!(count.next(), u64::MAX - 2);
}

#[test]
fn qa_waveform_stereo_extrema_partial_bucket_and_sanitization() {
    let slot = WaveSlot::new();
    let id = TrackId(7);
    let mut accumulator = WaveAccumulator::new();
    accumulator.capture(&slot, id, 1, 0, 1_000, &[[1.0, -1.0]; 5]);
    assert!(slot.snapshot(id, 1).is_none());
    slot.requested.store(id.0, Ordering::Release);
    accumulator.capture(&slot, id, 1, 0, 1_000, &[[1.0, -1.0], [-2.0, 3.0]]);
    assert!(slot.snapshot(id, 1).is_none());
    accumulator.capture(
        &slot,
        id,
        1,
        2,
        1_000,
        &[[f32::NAN, f32::INFINITY], [9.0, -9.0], [0.5, 0.25]],
    );
    let wave = slot.snapshot(id, 1).unwrap();
    assert_eq!(wave.points, vec![[-2.0, 8.0, -8.0, 3.0]]);
    assert_eq!(wave.bucket_frames, 5);
    assert_eq!(wave.sample_rate, 1_000);
    assert_eq!(wave.serial, 1);
}

#[test]
fn qa_waveform_ring_order_and_identity_epoch_rate_discontinuity_reset() {
    let slot = WaveSlot::new();
    let id = TrackId(7);
    slot.requested.store(id.0, Ordering::Release);
    let mut accumulator = WaveAccumulator::new();
    let frames: Vec<_> = (0..70)
        .map(|n| [n as f32 / 10.0, -(n as f32) / 10.0])
        .collect();
    accumulator.capture(&slot, id, 1, 0, 200, &frames);
    let wave = slot.snapshot(id, 1).unwrap();
    assert_eq!(wave.points.len(), 64);
    assert_eq!(wave.serial, 70);
    assert_eq!(wave.points[0], [0.6, 0.6, -0.6, -0.6]);
    assert_eq!(wave.points[63], [6.9, 6.9, -6.9, -6.9]);
    assert!(slot.snapshot(id, 2).is_none());
    for (epoch, base, rate) in [(2, 70, 200), (2, 99, 200), (2, 100, 400)] {
        accumulator.capture(&slot, id, epoch, base, rate, &[[0.25, -0.5]; 2]);
        let wave = slot.snapshot(id, epoch).unwrap();
        assert!(
            wave.points
                .iter()
                .all(|point| *point == [0.25, 0.25, -0.5, -0.5])
        );
        assert!(wave.serial <= 2);
    }
    let replacement = TrackId(8);
    slot.requested.store(replacement.0, Ordering::Release);
    assert!(slot.snapshot(id, 2).is_none());
    assert!(slot.snapshot(replacement, 2).is_none());
    accumulator.capture(&slot, replacement, 2, 102, 200, &[[0.75, 0.75]]);
    assert_eq!(
        slot.snapshot(replacement, 2).unwrap().points,
        vec![[0.75; 4]]
    );
}
