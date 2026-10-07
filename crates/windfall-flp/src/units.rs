//! From FL Studio's numbers to Windfall's units.
//!
//! Every function here is pure, and each says where its rule comes from.
//! Some rules are exact: time, pan, the tempo. Others are a curve through a
//! few known points, and say so; the import report counts what went
//! through those as approximated.

use windfall_core::PPQ;

/// A length or position in Windfall's ticks, and whether the conversion
/// was exact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rescaled {
    pub ticks: u64,
    /// False when the source tick fell between two of Windfall's ticks and
    /// was moved to the nearer one, by less than a tick.
    pub exact: bool,
}

/// Converts ticks at `ppq` per quarter note to Windfall's 960.
///
/// 960 is a multiple of 24, 48, 96, 120, 192, 480 and 960, so files at
/// those time bases convert without rounding. At the others FL Studio
/// offers (72, 144, 168, 384, 768) a tick can fall between two of
/// Windfall's; it goes to the nearer, and half way goes up.
///
/// A `ppq` of 0 is taken as 1, which no file has: the header is refused
/// first.
pub fn rescale(ticks: u32, ppq: u16) -> Rescaled {
    let ppq = u64::from(ppq.max(1));
    let scaled = u64::from(ticks) * u64::from(PPQ);
    Rescaled {
        ticks: (scaled + ppq / 2) / ppq,
        exact: scaled.is_multiple_of(ppq),
    }
}

/// The exponent of a mixer fader's curve: see [`fader_gain`].
const FADER_EXPONENT: f64 = 2.889_28;

/// The gain of a mixer fader, a send level or the main volume, from FL
/// Studio's number, where 12800 is 100%.
///
/// PyFLP documents three points of the fader: 0 is silence, 12800 is 0 dB
/// and 16000 (125%) is +5.6 dB. This is the power curve through them,
/// `(raw / 12800) ^ 2.889`. At 0 and at 12800 it is exact. Anywhere else
/// it is this crate's interpolation: FL Studio's own curve is not
/// published.
pub fn fader_gain(raw: i32) -> f32 {
    let position = f64::from(raw.max(0)) / 12_800.0;
    position.powf(FADER_EXPONENT).min(2.0) as f32
}

/// True when [`fader_gain`] is exact for this number.
pub fn fader_is_exact(raw: i32) -> bool {
    raw <= 0 || raw == 12_800
}

/// The gain of a channel's volume knob, from FL Studio's number, where
/// 12800 is the top.
///
/// One source: DawVert takes the knob to the power of 1.5. The top is
/// unity, so a channel never comes out louder than its sample.
pub fn channel_gain(raw: u32) -> f32 {
    let position = (f64::from(raw) / 12_800.0).min(1.0);
    position.powf(1.5) as f32
}

/// True when [`channel_gain`] is exact for this number.
pub fn channel_gain_is_exact(raw: u32) -> bool {
    raw == 0 || raw >= 12_800
}

/// A channel's pan, from 0 (left) through 6400 to 12800 (right), as -1 to
/// 1. Agreed (PyFLP, DawVert).
pub fn channel_pan(raw: u32) -> f32 {
    ((f64::from(raw) / 6_400.0) - 1.0).clamp(-1.0, 1.0) as f32
}

/// A mixer insert's pan, from -6400 (left) to 6400 (right), as -1 to 1.
/// Agreed (PyFLP, DawVert).
pub fn insert_pan(raw: i32) -> f32 {
    (f64::from(raw) / 6_400.0).clamp(-1.0, 1.0) as f32
}

/// A note's velocity, 0 to 128, as 0 to 1. FL Studio shows the same
/// number as a percentage of 128, so the 100 a drawn note has is 78%.
/// One source for the range (PyFLP).
pub fn note_velocity(raw: u8) -> f32 {
    f32::from(raw.min(128)) / 128.0
}

/// A note's pan, 0 (left) through 64 to 128 (right), as -1 to 1. Agreed
/// (PyFLP, DawVert).
pub fn note_pan(raw: u8) -> f32 {
    ((f32::from(raw.min(128)) - 64.0) / 64.0).clamp(-1.0, 1.0)
}

/// A time of a sampler envelope in milliseconds, from FL Studio's 100 to
/// 65536.
///
/// One source: DawVert's fit, `((raw / 65535 * 3.66) ^ 4.5 / 8) ^ 0.6`
/// seconds. It is a fit to what FL Studio shows, not FL Studio's formula.
pub fn envelope_ms(raw: u32) -> f32 {
    let position = f64::from(raw.min(65_536)) / 65_535.0;
    let seconds = ((position * 3.66).powf(4.5) / 8.0).powf(0.6);
    (seconds * 1000.0).clamp(0.0, 60_000.0) as f32
}

/// The automation value, 0 to 1, that gives a linear gain on Windfall's
/// gain targets, where the gain is `2 * value * value`.
pub fn gain_to_automation(gain: f32) -> f32 {
    (gain.max(0.0) / 2.0).sqrt().min(1.0)
}

/// The bend of an automation segment, from FL Studio's tension.
///
/// Both run from -1 to 1 with 0 a straight line, and the tension is
/// passed through. Which way a positive tension bends, and how sharply,
/// is not in any source, so a bent segment is counted as approximated.
pub fn automation_curve(tension: f32) -> f32 {
    if tension.is_finite() {
        tension.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_bases_that_divide_960_convert_exactly() {
        for (ppq, factor) in [
            (24, 40),
            (48, 20),
            (96, 10),
            (120, 8),
            (192, 5),
            (480, 2),
            (960, 1),
        ] {
            for ticks in [0, 1, 7, 95, 96, 1_000_003, u32::MAX] {
                let rescaled = rescale(ticks, ppq);
                assert_eq!(
                    rescaled.ticks,
                    u64::from(ticks) * factor,
                    "{ticks} at {ppq}"
                );
                assert!(rescaled.exact, "{ticks} at {ppq}");
            }
        }
    }

    #[test]
    fn at_384_every_second_tick_falls_between_two() {
        assert_eq!(
            rescale(0, 384),
            Rescaled {
                ticks: 0,
                exact: true
            }
        );
        assert_eq!(
            rescale(2, 384),
            Rescaled {
                ticks: 5,
                exact: true
            }
        );
        assert_eq!(
            rescale(384, 384),
            Rescaled {
                ticks: 960,
                exact: true
            }
        );
        // 1 tick is 2.5 of Windfall's: half way goes up.
        assert_eq!(
            rescale(1, 384),
            Rescaled {
                ticks: 3,
                exact: false
            }
        );
        assert_eq!(
            rescale(3, 384),
            Rescaled {
                ticks: 8,
                exact: false
            }
        );
    }

    #[test]
    fn odd_time_bases_round_to_the_nearer_tick() {
        // 960 / 7 is 137.14: down. 2 * 960 / 7 is 274.29: down.
        assert_eq!(
            rescale(1, 7),
            Rescaled {
                ticks: 137,
                exact: false
            }
        );
        assert_eq!(
            rescale(4, 7),
            Rescaled {
                ticks: 549,
                exact: false
            }
        );
        assert_eq!(
            rescale(7, 7),
            Rescaled {
                ticks: 960,
                exact: true
            }
        );
        // 5 * 960 / 768 is 6.25: down. 3 * 960 / 768 is 3.75: up.
        assert_eq!(
            rescale(5, 768),
            Rescaled {
                ticks: 6,
                exact: false
            }
        );
        assert_eq!(
            rescale(3, 768),
            Rescaled {
                ticks: 4,
                exact: false
            }
        );
        assert_eq!(
            rescale(4, 768),
            Rescaled {
                ticks: 5,
                exact: true
            }
        );
        // A quarter note is 960 ticks at any time base.
        for ppq in [1, 7, 72, 100, 144, 168, 1000, u16::MAX] {
            assert_eq!(
                rescale(u32::from(ppq), ppq),
                Rescaled {
                    ticks: 960,
                    exact: true
                }
            );
        }
    }

    #[test]
    fn rounding_never_moves_a_tick_by_a_whole_tick_or_out_of_order() {
        for ppq in [5_u16, 72, 144, 168, 384, 768, 1000] {
            let mut last = 0;
            for ticks in 0..2_000_u32 {
                let rescaled = rescale(ticks, ppq);
                let true_ticks = f64::from(ticks) * 960.0 / f64::from(ppq);
                assert!((rescaled.ticks as f64 - true_ticks).abs() <= 0.5);
                assert!(rescaled.ticks >= last);
                last = rescaled.ticks;
            }
        }
    }

    #[test]
    fn the_fader_passes_through_its_three_known_points() {
        assert_eq!(fader_gain(0), 0.0);
        assert_eq!(fader_gain(-5), 0.0);
        assert_eq!(fader_gain(12_800), 1.0);
        let top_db = 20.0 * fader_gain(16_000).log10();
        assert!((top_db - 5.6).abs() < 0.01, "{top_db}");
        assert!(fader_is_exact(12_800) && fader_is_exact(0) && !fader_is_exact(6_400));
        // Nothing gets past Windfall's own top.
        assert_eq!(fader_gain(i32::MAX), 2.0);
    }

    #[test]
    fn channel_volume_tops_out_at_unity() {
        assert_eq!(channel_gain(0), 0.0);
        assert_eq!(channel_gain(12_800), 1.0);
        assert_eq!(channel_gain(u32::MAX), 1.0);
        assert!((channel_gain(10_000) - 0.6905).abs() < 1e-3);
    }

    #[test]
    fn pans_are_linear_and_stay_in_range() {
        assert_eq!(channel_pan(0), -1.0);
        assert_eq!(channel_pan(6_400), 0.0);
        assert_eq!(channel_pan(12_800), 1.0);
        assert_eq!(channel_pan(u32::MAX), 1.0);
        assert_eq!(insert_pan(-6_400), -1.0);
        assert_eq!(insert_pan(3_200), 0.5);
        assert_eq!(insert_pan(i32::MIN), -1.0);
        assert_eq!(note_pan(0), -1.0);
        assert_eq!(note_pan(64), 0.0);
        assert_eq!(note_pan(255), 1.0);
    }

    #[test]
    fn a_drawn_note_has_the_velocity_fl_studio_shows() {
        assert_eq!(note_velocity(100), 0.78125);
        assert_eq!(note_velocity(128), 1.0);
        assert_eq!(note_velocity(255), 1.0);
        assert_eq!(note_velocity(0), 0.0);
    }

    #[test]
    fn envelope_times_grow_with_the_knob_and_stay_finite() {
        let times: Vec<f32> = [100, 20_000, 30_000, 65_536, u32::MAX]
            .into_iter()
            .map(envelope_ms)
            .collect();
        assert!(times.windows(2).all(|pair| pair[0] <= pair[1]), "{times:?}");
        assert!(times[0] < 1.0);
        assert!(times[3] > 1_000.0 && times[3] <= 60_000.0);
    }

    #[test]
    fn a_gain_becomes_the_automation_value_that_gives_it_back() {
        for gain in [0.0_f32, 0.25, 1.0, 1.9, 2.0] {
            let value = gain_to_automation(gain);
            assert!((2.0 * value * value - gain).abs() < 1e-6);
        }
        assert_eq!(gain_to_automation(5.0), 1.0);
        assert_eq!(gain_to_automation(-1.0), 0.0);
    }

    #[test]
    fn tension_passes_through_inside_its_range() {
        assert_eq!(automation_curve(0.5), 0.5);
        assert_eq!(automation_curve(-3.0), -1.0);
        assert_eq!(automation_curve(f32::NAN), 0.0);
    }
}
