//! Channel voice tools verified through native Processor audio, not browser simulation.
#[allow(dead_code)]
#[path = "engine/support.rs"]
mod support;

use support::{Rig, impulse, level, run};
use windfall_core::AudioBuffer;
use windfall_project::{ArpeggiatorMode, ChannelId, Envelope, NoteDivision};

const RATE: u32 = 48_000;

fn chord(mode: ArpeggiatorMode, gate: f32) -> (Rig, ChannelId) {
    let mut rig = Rig::new();
    rig.project.settings.tempo_bpm = 120.0;
    let channel = rig.channel(impulse(RATE));
    for (key, velocity) in [(60, 0.25), (64, 0.5), (67, 0.75)] {
        let note = rig.note(channel, 0, 1920);
        note.key = key;
        note.velocity = velocity;
    }
    for lane in &mut rig.project.patterns[0].lanes {
        lane.notes.sort_by_key(windfall_project::Note::sort_key);
    }
    let settings = &mut rig.channel_mut(channel).voice.arpeggiator;
    settings.mode = mode;
    settings.rate = NoteDivision::Sixteenth;
    settings.gate = gate;
    (rig, channel)
}

#[test]
fn qa_native_arp_modes_order_real_audio_without_editing_source_notes() {
    for (mode, velocities) in [
        (ArpeggiatorMode::Up, [0.25, 0.5, 0.75, 0.25, 0.5]),
        (ArpeggiatorMode::Down, [0.75, 0.5, 0.25, 0.75, 0.5]),
        (ArpeggiatorMode::UpDown, [0.25, 0.5, 0.75, 0.5, 0.25]),
        (ArpeggiatorMode::AsPlayed, [0.25, 0.5, 0.75, 0.25, 0.5]),
    ] {
        let (rig, _) = chord(mode, 0.5);
        let before = rig.project.clone();
        let expected_frames = [0, 6000, 12000, 18000, 24000];
        let baseline = rig.play(RATE, 30000, 127);
        for (frame, velocity) in expected_frames.into_iter().zip(velocities) {
            assert!(
                (baseline[frame * 2] - velocity).abs() < 1e-6,
                "{mode:?} frame {frame}"
            );
        }
        for block in [7, 64, 1000] {
            assert_eq!(
                baseline,
                rig.play(RATE, 30000, block),
                "{mode:?} block {block}"
            );
        }
        assert_eq!(rig.project, before);
    }
    let (silent, _) = chord(ArpeggiatorMode::Up, 0.0);
    assert!(
        silent
            .play(RATE, 15000, 127)
            .iter()
            .all(|sample| *sample == 0.0)
    );
}

#[test]
fn qa_native_live_as_played_arp_and_release_stop_future_steps() {
    let mut rig = Rig::new();
    let channel = rig.channel(impulse(RATE));
    let settings = &mut rig.channel_mut(channel).voice.arpeggiator;
    settings.mode = ArpeggiatorMode::AsPlayed;
    settings.rate = NoteDivision::Sixteenth;
    settings.gate = 0.5;
    let (mut processor, controller) = rig.processor(RATE);
    for (key, velocity) in [(67, 0.75), (60, 0.25), (64, 0.5)] {
        controller.note_on(channel, key, velocity);
    }
    let output = run(&mut processor, 18000, 127);
    for (frame, velocity) in [(0, 0.75), (6000, 0.25), (12000, 0.5)] {
        assert!((output[frame * 2] - velocity).abs() < 1e-6);
    }
    for key in [67, 60, 64] {
        controller.note_off(channel, key);
    }
    assert!(
        run(&mut processor, 12000, 64)
            .iter()
            .all(|sample| *sample == 0.0)
    );
}

#[test]
fn qa_native_seek_preserves_live_sampler_owner_and_key_up_releases_its_envelope() {
    for block in [7, 64, 127, 1000] {
        let mut rig = Rig::new();
        let channel = rig.channel(level(RATE, 0.2, 3.0));
        rig.sampler_mut(channel).envelope = Some(Envelope {
            attack_ms: 0.0,
            decay_ms: 1.0,
            sustain: 1.0,
            release_ms: 1.0,
        });
        let (mut processor, controller) = rig.processor(RATE);
        controller.play();
        controller.note_on(channel, 60, 1.0);
        assert!(
            run(&mut processor, 1000, block)
                .iter()
                .all(|sample| *sample == 0.2)
        );
        controller.seek(2000.0);
        assert!(
            run(&mut processor, 1000, block)
                .iter()
                .all(|sample| *sample == 0.2)
        );
        assert_eq!(controller.frame().voices, 1);
        controller.note_off(channel, 60);
        let released = run(&mut processor, 1000, block);
        assert!(released[0] > 0.19);
        assert!(released[200..].iter().all(|sample| *sample == 0.0));
        assert_eq!(controller.frame().voices, 0);
    }
}

#[test]
fn qa_native_arp_gate_and_octave_range_change_audible_notes() {
    let mut rig = Rig::new();
    let channel = rig.channel(level(RATE, 0.2, 3.0));
    rig.sampler_mut(channel).envelope = Some(Envelope {
        attack_ms: 0.0,
        decay_ms: 1.0,
        sustain: 1.0,
        release_ms: 1.0,
    });
    rig.note(channel, 0, 1920).key = 60;
    let settings = &mut rig.channel_mut(channel).voice.arpeggiator;
    settings.mode = ArpeggiatorMode::Up;
    settings.rate = NoteDivision::Sixteenth;
    settings.gate = 0.25;
    settings.range_octaves = 2;
    let audio = rig.play(RATE, 13000, 127);
    assert!(audio[1000 * 2] > 0.19);
    assert_eq!(audio[2000 * 2], 0.0);
    assert!(audio[7000 * 2] > 0.19);
    assert_eq!(audio[8000 * 2], 0.0);
    // A linear source makes octave expansion observable as doubled playback
    // slope, rather than merely inferring the key from generated metadata.
    let sample = rig.sample(AudioBuffer::from_interleaved(
        RATE,
        1,
        (0..1000).map(|n| n as f32 / 1000.0).collect(),
    ));
    rig.sampler_mut(channel).sample = Some(sample);
    rig.sampler_mut(channel).envelope = None;
    let audio = rig.play(RATE, 13000, 127);
    assert!((audio[2] - 0.001).abs() < 1e-6);
    assert!((audio[6001 * 2] - 0.002).abs() < 1e-6);
    assert_eq!(audio, rig.play(RATE, 13000, 64));
}

#[test]
fn qa_native_polyphony_caps_audible_survivors_and_is_partition_independent() {
    for limit in [1, 2, 3] {
        let mut rig = Rig::new();
        let channel = rig.channel(level(RATE, 0.2, 3.0));
        rig.channel_mut(channel).voice.polyphony.max_voices = limit;
        for (key, velocity) in [(60, 0.25), (64, 0.5), (67, 0.75)] {
            let note = rig.note(channel, 0, 1920);
            note.key = key;
            note.velocity = velocity;
        }
        for lane in &mut rig.project.patterns[0].lanes {
            lane.notes.sort_by_key(windfall_project::Note::sort_key);
        }
        let (mut processor, controller) = rig.processor(RATE);
        controller.play();
        let audio = run(&mut processor, 1000, 127);
        assert_eq!(controller.frame().voices, u32::from(limit));
        let expected = match limit {
            1 => 0.15,
            2 => 0.25,
            _ => 0.3,
        };
        assert!((audio[999 * 2] - expected).abs() < 1e-6);
        for block in [7, 64, 1000] {
            assert_eq!(audio, rig.play(RATE, 1000, block));
        }
    }
}

fn mono_audio(portamento_ms: f32, block: usize) -> Vec<f32> {
    let mut rig = Rig::new();
    let sample =
        AudioBuffer::from_interleaved(RATE, 1, (0..100000).map(|n| n as f32 / 1000000.0).collect());
    let channel = rig.channel(sample);
    rig.channel_mut(channel).voice.polyphony.mono_legato = true;
    rig.channel_mut(channel).voice.polyphony.portamento_ms = portamento_ms;
    rig.note(channel, 0, 1920).key = 60;
    rig.note(channel, 240, 1920).key = 72;
    let (mut processor, controller) = rig.processor(RATE);
    controller.play();
    let output = run(&mut processor, 7500, block);
    assert_eq!(controller.frame().voices, 1);
    output
}

#[test]
fn qa_native_mono_preserves_sample_position_and_glide_time_changes_pitch_slope() {
    let snap = mono_audio(0.0, 127);
    let glide = mono_audio(10.0, 127);
    assert!(
        (snap[6000 * 2] - 0.006).abs() < 1e-6,
        "mono transfer must retain sample position"
    );
    let slope = |audio: &[f32], at: usize| (audio[(at + 1) * 2] - audio[at * 2]) * 1000000.0;
    assert!((slope(&snap, 6000) - 2.0).abs() < 0.01);
    assert!(slope(&glide, 6000) > 1.0 && slope(&glide, 6000) < 1.02);
    assert!(slope(&glide, 6480) > 1.5 && slope(&glide, 6480) < 1.6);
    assert_eq!(snap, mono_audio(0.0, 64));
    assert_eq!(glide, mono_audio(10.0, 1000));
}
