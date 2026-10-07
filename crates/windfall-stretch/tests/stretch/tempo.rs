//! Loop fitting and honest periodic-onset estimates.
use crate::support::{RATE, drum_loop, noise, sine};
use windfall_core::AudioBuffer;
use windfall_stretch::{beat_length_ratio, estimate_tempo, tempo_ratio};

#[test]
fn known_tempos_and_beats_fit_the_target_length() {
    assert_eq!(tempo_ratio(120.0, 100.0), Some(1.2));
    assert_eq!(beat_length_ratio(96_000, RATE, 4.0, 100.0), Some(1.2));
    for value in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(tempo_ratio(value, 100.0), None);
        assert_eq!(tempo_ratio(120.0, value), None);
        assert_eq!(beat_length_ratio(96_000, RATE, value, 100.0), None);
    }
    assert_eq!(beat_length_ratio(0, RATE, 4.0, 100.0), None);
    assert_eq!(beat_length_ratio(96_000, 0, 4.0, 100.0), None);
}

#[test]
fn regular_drum_loops_have_a_candidate_at_their_tempo() {
    for bpm in [60.0, 80.0, 100.0, 120.0, 150.0, 180.0, 200.0] {
        let buffer = AudioBuffer::from_interleaved(RATE, 1, drum_loop(bpm, 16, RATE));
        let candidates = estimate_tempo(&buffer).expect("periodic drum loop");
        assert!(
            candidates
                .iter()
                .any(|c| (c.bpm - bpm).abs() < 1.0 && c.confidence > 0.35),
            "{bpm}: {candidates:?}"
        );
        assert!(
            candidates
                .iter()
                .all(|c| c.confidence >= 0.0 && c.confidence <= 1.0)
        );
    }
}

#[test]
fn uncertain_loops_are_not_assigned_a_tempo() {
    for samples in [
        vec![0.0; 192_000],
        vec![0.5; 192_000],
        sine(440.0, 0.5, 192_000, RATE),
        noise(7, 0.5, 192_000),
        drum_loop(120.0, 1, RATE),
    ] {
        assert!(estimate_tempo(&AudioBuffer::from_interleaved(RATE, 1, samples)).is_none());
    }
}
