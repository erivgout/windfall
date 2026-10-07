//! Unit conversions and the small numeric guards the processors share.

pub use windfall_core::{db_to_gain, gain_to_db};

/// The level reported for silence, in dB. Levels are floored here so that a
/// detector never hands negative infinity to the arithmetic after it.
pub const SILENCE_DB: f32 = -120.0;

/// Converts a linear level to dB, floored at [`SILENCE_DB`].
#[inline]
pub fn level_to_db(level: f32) -> f32 {
    if level > 1.0e-6 {
        20.0 * level.log10()
    } else {
        SILENCE_DB
    }
}

/// Converts a power (a squared level) to dB, floored at [`SILENCE_DB`].
#[inline]
pub fn power_to_db(power: f32) -> f32 {
    if power > 1.0e-12 {
        10.0 * power.log10()
    } else {
        SILENCE_DB
    }
}

/// [`db_to_gain`] computed with a single `exp`, for use once per sample.
#[inline]
pub fn db_to_gain_exp(db: f32) -> f32 {
    (db * (std::f32::consts::LN_10 / 20.0)).exp()
}

/// Returns zero for values too small to hear.
///
/// A filter or feedback loop fed silence decays forever, and once its state
/// reaches the subnormal range every operation on it costs many times more
/// CPU. Flushing the state to an exact zero long before that keeps a silent
/// tail as cheap as silence.
#[inline]
pub fn flush(value: f32) -> f32 {
    if value.abs() < 1.0e-20 { 0.0 } else { value }
}

/// [`flush`] for double precision state.
#[inline]
pub fn flush64(value: f64) -> f64 {
    if value.abs() < 1.0e-20 { 0.0 } else { value }
}

/// Forces a parameter into its range. A value that is not a number becomes
/// `fallback`.
#[inline]
pub fn clean(value: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

/// The share of the remaining distance a one-pole smoother covers per
/// sample so that it has covered 63% of a step after `time_ms`. A time of
/// zero gives 1, which means no smoothing.
#[inline]
pub fn smoothing_coefficient(time_ms: f32, sample_rate: f32) -> f32 {
    let samples = time_ms * 0.001 * sample_rate;
    if samples > 1.0e-3 {
        1.0 - (-1.0 / samples).exp()
    } else {
        1.0
    }
}

/// Frequency of a key in Hz, in equal temperament with A4 (key 69) at
/// 440 Hz. Fractional keys are allowed.
#[inline]
pub fn key_to_hz(key: f32) -> f32 {
    440.0 * ((key - 69.0) / 12.0).exp2()
}

#[inline]
pub fn lerp(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
}

/// A whole number of samples for a time in milliseconds, at least 1.
#[inline]
pub fn ms_to_samples(time_ms: f32, sample_rate: f32) -> u32 {
    (time_ms * 0.001 * sample_rate).round().max(1.0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_convert_and_floor() {
        assert!((level_to_db(1.0)).abs() < 1e-6);
        assert!((level_to_db(0.5) + 6.0206).abs() < 1e-3);
        assert_eq!(level_to_db(0.0), SILENCE_DB);
        assert_eq!(level_to_db(-1.0), SILENCE_DB);
        assert!((power_to_db(0.25) + 6.0206).abs() < 1e-3);
        assert_eq!(power_to_db(0.0), SILENCE_DB);
    }

    #[test]
    fn the_fast_conversion_agrees_with_the_reference() {
        for db in [-120.0_f32, -60.0, -6.0, 0.0, 0.1, 12.0, 36.0] {
            let (fast, reference) = (db_to_gain_exp(db), db_to_gain(db));
            assert!((fast / reference - 1.0).abs() < 1e-5, "{db}");
        }
    }

    #[test]
    fn flush_removes_only_inaudible_values() {
        assert_eq!(flush(1.0e-30), 0.0);
        assert_eq!(flush(-1.0e-39), 0.0);
        assert_eq!(flush(1.0e-10), 1.0e-10);
        assert_eq!(flush64(1.0e-200), 0.0);
        assert_eq!(flush64(-1.0e-12), -1.0e-12);
    }

    #[test]
    fn clean_clamps_and_replaces_non_numbers() {
        assert_eq!(clean(5.0, 0.0, 1.0, 0.5), 1.0);
        assert_eq!(clean(-5.0, 0.0, 1.0, 0.5), 0.0);
        assert_eq!(clean(f32::NAN, 0.0, 1.0, 0.5), 0.5);
        assert_eq!(clean(f32::INFINITY, 0.0, 1.0, 0.5), 0.5);
    }

    #[test]
    fn smoothing_coefficient_reaches_63_percent_on_time() {
        let coefficient = smoothing_coefficient(10.0, 48_000.0);
        let mut value = 0.0_f32;
        for _ in 0..480 {
            value += (1.0 - value) * coefficient;
        }
        assert!((value - 0.632).abs() < 0.002, "{value}");
        assert_eq!(smoothing_coefficient(0.0, 48_000.0), 1.0);
    }

    #[test]
    fn keys_follow_equal_temperament() {
        assert!((key_to_hz(69.0) - 440.0).abs() < 1e-3);
        assert!((key_to_hz(81.0) - 880.0).abs() < 1e-2);
        assert!((key_to_hz(60.0) - 261.6256).abs() < 1e-2);
    }
}
