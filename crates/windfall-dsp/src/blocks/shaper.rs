//! Soft clippers: curves that squash a signal smoothly as it gets loud
//! instead of cutting it off flat.

/// A close, cheap stand-in for `tanh`. Small signals pass almost unchanged,
/// loud ones are squeezed toward 1, and nothing ever comes out above 1.
///
/// It is the rational approximation `x (27 + x^2) / (27 + 9 x^2)`, which
/// reaches exactly 1 at an input of 3 and is held there beyond it. Its
/// slope never exceeds 1, so it cannot raise the gain of a feedback loop.
#[inline]
pub fn soft_clip(input: f32) -> f32 {
    let x = input.clamp(-3.0, 3.0);
    let x2 = x * x;
    // Rounding can land a hair above 1 just short of the limit.
    (x * (27.0 + x2) / (27.0 + 9.0 * x2)).clamp(-1.0, 1.0)
}

/// A gentler curve, `1.5 x - 0.5 x^3`, that reaches 1 at an input of 1 and
/// is held there beyond it. It adds mostly third-harmonic warmth.
#[inline]
pub fn cubic_clip(input: f32) -> f32 {
    let x = input.clamp(-1.0, 1.0);
    1.5 * x - 0.5 * x * x * x
}

/// Cuts the signal off flat at plus and minus `limit`.
#[inline]
pub fn hard_clip(input: f32, limit: f32) -> f32 {
    input.clamp(-limit, limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soft_clip_tracks_tanh_and_never_exceeds_one() {
        for step in -400..=400 {
            let x = step as f32 / 100.0;
            let clipped = soft_clip(x);
            assert!(clipped.abs() <= 1.0);
            if x.abs() <= 3.0 {
                assert!((clipped - x.tanh()).abs() < 0.03, "{x}: {clipped}");
            }
        }
        assert_eq!(soft_clip(3.0), 1.0);
        assert_eq!(soft_clip(1.0e9), 1.0);
        assert_eq!(soft_clip(-50.0), -1.0);
    }

    #[test]
    fn soft_clip_is_transparent_for_quiet_signals() {
        for x in [1.0e-4_f32, -1.0e-3, 0.01] {
            assert!((soft_clip(x) / x - 1.0).abs() < 1.0e-4);
        }
        assert_eq!(soft_clip(0.0), 0.0);
    }

    #[test]
    fn curves_are_odd_and_monotonic_with_slope_at_most_one() {
        // The cubic's slope at zero is 1.5 by design, so only the soft clip
        // is held to a slope of one.
        for (curve, steepest) in [(soft_clip as fn(f32) -> f32, 1.0), (cubic_clip, 1.5)] {
            let mut previous = curve(-5.0);
            for step in -499..=500 {
                let x = step as f32 / 100.0;
                let value = curve(x);
                assert!(value >= previous - 1e-6);
                assert!(value - previous <= 0.01 * steepest + 1e-5, "slope at {x}");
                assert!((value + curve(-x)).abs() < 1e-6);
                previous = value;
            }
        }
    }

    #[test]
    fn cubic_and_hard_clip_hit_their_limits() {
        assert_eq!(cubic_clip(1.0), 1.0);
        assert_eq!(cubic_clip(7.0), 1.0);
        assert!((cubic_clip(0.5) - 0.6875).abs() < 1e-6);
        assert_eq!(hard_clip(2.0, 0.5), 0.5);
        assert_eq!(hard_clip(-2.0, 0.5), -0.5);
        assert_eq!(hard_clip(0.25, 0.5), 0.25);
    }
}
