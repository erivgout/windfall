//! Actual sample onsets prove the native plan uses channel timing.
#[allow(dead_code)]
#[path = "engine/support.rs"]
mod support;

use support::{Rig, impulse, sounding_frames};
use windfall_project::ChannelTiming;

#[test]
fn native_sequencer_channel_swing_mix_and_signed_shift_match_exact_frames() {
    // At 48 kHz and 120 bpm one project tick is exactly 25 frames.
    // Half swing on step 1: 240 + 40 = 280 ticks. Step 2: 480 ticks.
    for (mix, shift, ticks) in [
        (0.5, 19, [299, 499]),
        (0.0, 19, [259, 499]),
        (1.0, -19, [301, 461]),
        (0.5, -300, [0, 180]),
    ] {
        let mut rig = Rig::new();
        rig.project.settings.tempo_bpm = 120.0;
        rig.project.settings.swing = 1.0;
        let channel = rig.channel(impulse(48_000));
        rig.steps(channel, &[1, 2]);
        rig.channel_mut(channel).timing = ChannelTiming {
            swing_mix: mix,
            gate_ticks: 75,
            shift_ticks: shift,
        };
        let before = rig.project.clone();
        for block in [7, 64, 1000] {
            let pcm = rig.play(48_000, 24_000, block);
            assert_eq!(
                sounding_frames(&pcm),
                ticks.map(|tick| tick * 25),
                "mix {mix}, shift {shift}, block {block}"
            );
        }
        assert_eq!(rig.project, before);
    }
}
