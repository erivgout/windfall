//! How MIDI's 7-bit values and a project's levels map onto each other.
//! Import and export both go through these, so a value that leaves comes
//! back the same.

use windfall_project::{DEFAULT_CHANNEL_VOLUME, MAX_GAIN};

/// The channel volume a MIDI device starts with. It maps onto
/// [`DEFAULT_CHANNEL_VOLUME`], so a file that never sets a volume and a
/// file that sets the usual one sound alike.
const DEFAULT_MIDI_VOLUME: f32 = 100.0;

/// A note's velocity, 0 to 1, from a MIDI velocity.
pub(crate) fn velocity_from_midi(velocity: u8) -> f32 {
    f32::from(velocity.min(127)) / 127.0
}

/// The MIDI velocity nearest a note's velocity. A note that sounds at all
/// has at least 1, because 0 would end it.
pub(crate) fn velocity_to_midi(velocity: f32) -> u8 {
    scaled(velocity * 127.0).max(1)
}

/// A channel's volume from a MIDI channel volume (controller 7). General
/// MIDI makes loudness follow the square of the value, and so does this:
/// `0.8 * (value / 100)²`, which puts MIDI's default of 100 on Windfall's
/// default of 0.8 and its top of 127 at 1.29, about +2 dB.
pub(crate) fn volume_from_midi(value: u16) -> f32 {
    let part = f32::from(value.min(127)) / DEFAULT_MIDI_VOLUME;
    (DEFAULT_CHANNEL_VOLUME * part * part).min(MAX_GAIN)
}

/// The MIDI channel volume nearest a channel's volume. Volumes above 1.29
/// are more than MIDI can say and give 127.
pub(crate) fn volume_to_midi(volume: f32) -> u8 {
    scaled((volume.max(0.0) / DEFAULT_CHANNEL_VOLUME).sqrt() * DEFAULT_MIDI_VOLUME)
}

/// A channel's pan from a MIDI pan (controller 10), where 0 is hard left,
/// 64 the center and 127 hard right. The left half has 64 steps and the
/// right half 63, so each side is scaled by its own count.
pub(crate) fn pan_from_midi(value: u16) -> f32 {
    let offset = f32::from(value.min(127)) - 64.0;
    if offset < 0.0 {
        offset / 64.0
    } else {
        offset / 63.0
    }
}

/// The MIDI pan nearest a channel's pan.
pub(crate) fn pan_to_midi(pan: f32) -> u8 {
    let steps = if pan < 0.0 { 64.0 } else { 63.0 };
    scaled(64.0 + pan.clamp(-1.0, 1.0) * steps)
}

/// The nearest 7-bit value. Anything that is not a number is 0.
fn scaled(value: f32) -> u8 {
    // The cast saturates and turns NaN into 0.
    (value.round() as u8).min(127)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_midi_value_comes_back_from_the_project_unchanged() {
        for value in 0..=127_u8 {
            let wide = u16::from(value);
            assert_eq!(volume_to_midi(volume_from_midi(wide)), value);
            assert_eq!(pan_to_midi(pan_from_midi(wide)), value);
            if value > 0 {
                assert_eq!(velocity_to_midi(velocity_from_midi(value)), value);
            }
        }
    }

    #[test]
    fn the_usual_values_land_on_the_usual_levels() {
        assert_eq!(volume_from_midi(100), DEFAULT_CHANNEL_VOLUME);
        assert_eq!(volume_from_midi(0), 0.0);
        assert!((volume_from_midi(127) - 1.290_32).abs() < 1e-4);
        assert_eq!(volume_from_midi(9_999), volume_from_midi(127));
        assert_eq!(pan_from_midi(64), 0.0);
        assert_eq!(pan_from_midi(0), -1.0);
        assert_eq!(pan_from_midi(127), 1.0);
        assert_eq!(velocity_from_midi(127), 1.0);
    }

    #[test]
    fn levels_midi_cannot_say_go_to_the_nearest_it_can() {
        assert_eq!(volume_to_midi(MAX_GAIN), 127);
        assert_eq!(volume_to_midi(-1.0), 0);
        assert_eq!(volume_to_midi(f32::NAN), 0);
        assert_eq!(pan_to_midi(-7.0), 0);
        assert_eq!(pan_to_midi(7.0), 127);
        assert_eq!(velocity_to_midi(0.0), 1);
        assert_eq!(velocity_to_midi(0.001), 1);
        assert_eq!(velocity_to_midi(2.0), 127);
        assert_eq!(velocity_to_midi(0.8), 102);
    }
}
