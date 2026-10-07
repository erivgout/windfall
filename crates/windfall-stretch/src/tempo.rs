//! Offline loop tempo helpers. Tempo has octave ambiguity; return candidates.
use windfall_core::AudioBuffer;

/// A plausible beat rate, with a normalized periodicity score in 0..=1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TempoCandidate {
    /// Beats per minute, between 60 and 200.
    pub bpm: f64,
    /// Onset-envelope correlation, not a calibrated probability.
    pub confidence: f64,
}

/// Length multiplier to fit an original tempo to a target tempo.
/// Invalid or nonpositive tempos return `None`; the result is not clamped.
pub fn tempo_ratio(original_bpm: f64, target_bpm: f64) -> Option<f64> {
    if original_bpm.is_finite() && target_bpm.is_finite() && original_bpm > 0.0 && target_bpm > 0.0
    {
        let ratio = original_bpm / target_bpm;
        ratio.is_finite().then_some(ratio)
    } else {
        None
    }
}

/// Length multiplier for a loop with a known number of quarter-note beats.
/// Invalid lengths, beat counts, sample rates or tempos return `None`.
pub fn beat_length_ratio(
    frames: usize,
    sample_rate: u32,
    beats: f64,
    target_bpm: f64,
) -> Option<f64> {
    if frames == 0 || sample_rate == 0 || !beats.is_finite() || beats <= 0.0 {
        return None;
    }
    let original = beats * 60.0 * f64::from(sample_rate) / frames as f64;
    tempo_ratio(original, target_bpm)
}

/// Estimates periodic onsets from a complete loop, allocating off the audio thread.
///
/// A 200 Hz envelope of positive log-energy differences is autocorrelated over
/// 60–200 BPM. At least three beats are required. Silence, steady tones, short
/// loops and weak periodicity return `None`. Double/half-tempo candidates can
/// both be returned: a host should let the musician choose the beat unit.
pub fn estimate_tempo(buffer: &AudioBuffer) -> Option<Vec<TempoCandidate>> {
    let hop = (buffer.sample_rate() as usize / 200).max(1);
    let channels = usize::from(buffer.channels());
    let mut onset = Vec::new();
    let mut previous = 0.0_f64;
    for chunk in buffer.samples().chunks(hop * channels) {
        let power = chunk
            .iter()
            .filter(|s| s.is_finite())
            .map(|s| f64::from(*s).powi(2))
            .sum::<f64>()
            / chunk.len() as f64;
        let level = (1.0 + power * 1000.0).ln();
        onset.push((level - previous).max(0.0));
        previous = level;
    }
    let rate = f64::from(buffer.sample_rate()) / hop as f64;
    if onset.len() < (3.0 * rate * 60.0 / 200.0) as usize {
        return None;
    }
    let mean = onset.iter().sum::<f64>() / onset.len() as f64;
    let variance = onset.iter().map(|s| (s - mean).powi(2)).sum::<f64>();
    if variance < 1e-6 || mean < 1e-5 {
        return None;
    }
    let low = (rate * 60.0 / 200.0).ceil() as usize;
    let high = ((rate * 60.0 / 60.0).floor() as usize).min(onset.len() / 3);
    if high <= low {
        return None;
    }
    let mut scores = vec![0.0; high + 2];
    for lag in low..=high {
        let (mut cross, mut left, mut right) = (0.0, 0.0, 0.0);
        for n in lag..onset.len() {
            let a = onset[n] - mean;
            let b = onset[n - lag] - mean;
            cross += a * b;
            left += a * a;
            right += b * b;
        }
        scores[lag] = (cross / (left * right).sqrt().max(1e-30)).clamp(0.0, 1.0);
    }
    let mut candidates = Vec::new();
    for lag in low..=high {
        let score = scores[lag];
        if score < 0.35 || score < scores[lag - 1] || score <= scores[lag + 1] {
            continue;
        }
        let a = scores[lag - 1];
        let b = scores[lag + 1];
        let offset = if lag > low && lag < high {
            (0.5 * (a - b) / (a - 2.0 * score + b)).clamp(-0.5, 0.5)
        } else {
            0.0
        };
        candidates.push(TempoCandidate {
            bpm: rate * 60.0 / (lag as f64 + offset),
            confidence: score,
        });
    }
    candidates.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
    candidates.truncate(5);
    (!candidates.is_empty()).then_some(candidates)
}
