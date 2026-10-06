//! Sine, exponential and tanh built from IEEE 754 addition, multiplication
//! and division only.
//!
//! The standard library forwards these functions to the platform's C math
//! library, and those libraries disagree in the last bit. Every sample of the
//! factory sounds has to come out the same on every machine, so the
//! synthesis code calls these instead.

use std::f64::consts::{LN_2, LN_10, TAU};

/// Sine of an angle given in turns: `sin_turns(0.25)` is 1.
pub fn sin_turns(turns: f64) -> f64 {
    // Fold the angle into a quarter turn either side of zero, where a short
    // Taylor series is exact to the last bit or two.
    let y = turns - turns.floor() - 0.5;
    let y = if y > 0.25 {
        0.5 - y
    } else if y < -0.25 {
        -0.5 - y
    } else {
        y
    };
    let x = TAU * y;
    let x2 = x * x;
    let mut sum = SIN_TERMS[SIN_TERMS.len() - 1];
    for term in SIN_TERMS.iter().rev().skip(1) {
        sum = sum * x2 + term;
    }
    // The fold measured the angle from half a turn, which flips the sign.
    -(x * sum)
}

/// Cosine of an angle given in turns.
pub fn cos_turns(turns: f64) -> f64 {
    sin_turns(turns + 0.25)
}

/// Tangent of an angle given in turns.
pub fn tan_turns(turns: f64) -> f64 {
    sin_turns(turns) / cos_turns(turns)
}

/// The exponential function. Inputs below -700 return zero; inputs above 700
/// are outside what the synthesis code needs and are clamped.
pub fn exp(x: f64) -> f64 {
    if x < -700.0 {
        return 0.0;
    }
    let x = x.min(700.0);
    // exp(x) = 2^k * exp(r) with r no larger than half of ln 2.
    let k = (x / LN_2).round();
    let r = x - k * LN_2;
    let mut sum = EXP_TERMS[EXP_TERMS.len() - 1];
    for term in EXP_TERMS.iter().rev().skip(1) {
        sum = sum * r + term;
    }
    sum * f64::from_bits(((1023 + k as i64) as u64) << 52)
}

/// The hyperbolic tangent, used as a soft clipper.
pub fn tanh(x: f64) -> f64 {
    if x > 20.0 {
        1.0
    } else if x < -20.0 {
        -1.0
    } else {
        let e = exp(2.0 * x);
        (e - 1.0) / (e + 1.0)
    }
}

/// Converts decibels to a linear gain factor.
pub fn db_to_gain(db: f64) -> f64 {
    exp(db * (LN_10 / 20.0))
}

/// Frequency ratio of an interval in cents (hundredths of a semitone).
pub fn cents(cents: f64) -> f64 {
    exp(cents * (LN_2 / 1200.0))
}

/// Taylor coefficients of sin(x) / x in powers of x squared, through x^20.
const SIN_TERMS: [f64; 11] = [
    1.0,
    -1.0 / 6.0,
    1.0 / 120.0,
    -1.0 / 5_040.0,
    1.0 / 362_880.0,
    -1.0 / 39_916_800.0,
    1.0 / 6_227_020_800.0,
    -1.0 / 1_307_674_368_000.0,
    1.0 / 355_687_428_096_000.0,
    -1.0 / 121_645_100_408_832_000.0,
    1.0 / 51_090_942_171_709_440_000.0,
];

/// Taylor coefficients of exp(r), through r^13.
const EXP_TERMS: [f64; 14] = [
    1.0,
    1.0,
    1.0 / 2.0,
    1.0 / 6.0,
    1.0 / 24.0,
    1.0 / 120.0,
    1.0 / 720.0,
    1.0 / 5_040.0,
    1.0 / 40_320.0,
    1.0 / 362_880.0,
    1.0 / 3_628_800.0,
    1.0 / 39_916_800.0,
    1.0 / 479_001_600.0,
    1.0 / 6_227_020_800.0,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_matches_the_standard_library() {
        for i in -4_000..4_000 {
            let turns = i as f64 / 1_000.0 + 0.000_123;
            let expected = (TAU * turns).sin();
            assert!(
                (sin_turns(turns) - expected).abs() < 1e-13,
                "sin_turns({turns})"
            );
            assert!(
                (cos_turns(turns) - (TAU * turns).cos()).abs() < 1e-13,
                "cos_turns({turns})"
            );
        }
    }

    #[test]
    fn sine_hits_its_landmarks_exactly() {
        assert_eq!(sin_turns(0.0), 0.0);
        assert_eq!(sin_turns(0.25), 1.0);
        assert_eq!(sin_turns(0.75), -1.0);
        assert!(sin_turns(0.5).abs() < 1e-15);
    }

    #[test]
    fn exponential_matches_the_standard_library() {
        for i in -7_000..7_000 {
            let x = i as f64 / 10.0 + 0.012_3;
            let expected = x.exp();
            assert!(
                (exp(x) - expected).abs() <= expected * 1e-13,
                "exp({x}) = {} but std says {expected}",
                exp(x)
            );
        }
        assert_eq!(exp(0.0), 1.0);
        assert_eq!(exp(-1_000.0), 0.0);
    }

    #[test]
    fn tanh_matches_the_standard_library() {
        for i in -3_000..3_000 {
            let x = i as f64 / 100.0;
            assert!((tanh(x) - x.tanh()).abs() < 1e-13, "tanh({x})");
        }
    }

    #[test]
    fn unit_conversions_are_right() {
        assert!((db_to_gain(-6.0) - 0.501_187_233_627_272_2).abs() < 1e-13);
        assert_eq!(db_to_gain(0.0), 1.0);
        assert!((cents(1_200.0) - 2.0).abs() < 1e-13);
        assert!((tan_turns(0.125) - 1.0).abs() < 1e-13);
    }
}
