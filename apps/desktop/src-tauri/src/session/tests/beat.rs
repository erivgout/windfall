//! The Phase 1 exit check from WINDFALL_PLAN.md, end to end: build a drum
//! loop, save it, reopen it and export it.

use windfall_core::AudioBuffer;
use windfall_ipc::{BitDepth, ExportFormat, ExportOptions, PlayMode};
use windfall_project::{ChannelId, Command, SamplePath, SettingsPatch};

use super::{Rig, SAMPLE_RATE, factory_file, rms};
use crate::events::Event;

/// Steps the kick plays on.
const KICK_STEPS: [u32; 4] = [0, 4, 8, 12];

/// At 150 bpm a sixteenth-note step is a tenth of a second.
const TEMPO: f64 = 150.0;
const FRAMES_PER_STEP: usize = 4_800;
const FRAMES_PER_BAR: usize = FRAMES_PER_STEP * 16;

const LOOPS: u32 = 2;
const TAIL_SECS: f32 = 0.5;

fn toggle(rig: &Rig, channel: ChannelId, steps: &[u32]) {
    for &step in steps {
        rig.session
            .dispatch(
                Command::ToggleStep {
                    pattern: rig.pattern(),
                    channel,
                    step,
                },
                None,
            )
            .unwrap();
    }
}

fn set_tempo(rig: &Rig, tempo_bpm: f64) {
    rig.session
        .dispatch(
            Command::UpdateSettings {
                patch: SettingsPatch {
                    tempo_bpm: Some(tempo_bpm),
                    ..SettingsPatch::default()
                },
            },
            None,
        )
        .unwrap();
}

/// Checks that the audio is loud right where each kick starts and was
/// quiet just before it.
fn assert_kicks_are_where_the_steps_are(audio: &AudioBuffer, label: &str) {
    // 20 ms on either side of the onset.
    let window = SAMPLE_RATE as usize / 50;
    let samples = audio.samples();
    let span = |from: usize, to: usize| rms(&samples[from * 2..to * 2]);
    for pass in 0..LOOPS as usize {
        for step in KICK_STEPS {
            let onset = pass * FRAMES_PER_BAR + step as usize * FRAMES_PER_STEP;
            let hit = span(onset, onset + window);
            assert!(
                hit > 0.1,
                "{label}: no kick at step {step} of pass {pass}: rms {hit}"
            );
            if onset > 0 {
                let before = span(onset - window, onset);
                assert!(
                    hit > before * 4.0,
                    "{label}: step {step} of pass {pass} is no louder than what came before: {hit} after {before}"
                );
            }
        }
    }
}

#[test]
fn a_beat_can_be_built_played_saved_reopened_and_exported() {
    let mut rig = Rig::new();
    let session = rig.session.clone();
    let (kick, clap, hat) = (rig.channel(0), rig.channel(1), rig.channel(2));

    // Build the loop on the default kit.
    toggle(&rig, kick, &KICK_STEPS);
    toggle(&rig, clap, &[4, 12]);
    toggle(&rig, hat, &[2, 6, 10, 14]);

    // Bring in one more sound from the browser.
    let added = session
        .add_channel_from_file(&factory_file("Drums/Percussion/Rim Click.wav"), None)
        .unwrap();
    let rim = ChannelId(added.created[1]);
    toggle(&rig, rim, &[7, 15]);
    assert_eq!(rig.project().channels.len(), 5);
    assert_eq!(rig.project().channels[4].name, "Rim Click");

    // Change the tempo, think better of it, and change it back again.
    set_tempo(&rig, TEMPO);
    assert_eq!(rig.project().settings.tempo_bpm, TEMPO);
    session.undo().unwrap();
    assert_eq!(rig.project().settings.tempo_bpm, 120.0);
    session.undo().unwrap();
    let rim_notes = |rig: &Rig| {
        rig.project().patterns[0]
            .lane(rim)
            .map_or(0, |lane| lane.notes.len())
    };
    assert_eq!(rim_notes(&rig), 1);
    session.redo().unwrap();
    session.redo().unwrap();
    assert_eq!(session.redo(), None);
    assert_eq!(rim_notes(&rig), 2);
    assert_eq!(rig.project().settings.tempo_bpm, TEMPO);

    // It plays: the playhead moves and the meters show the kick.
    rig.events.take();
    assert!(session.transport_play().unwrap().playing);
    rig.run(SAMPLE_RATE as usize / 4);
    let first = session.realtime_tick();
    assert!(first.playing);
    // A quarter of a second at 150 bpm is 600 ticks.
    assert!((first.tick - 600.0).abs() < 1.0, "tick {}", first.tick);
    assert_eq!(first.meters.len(), rig.project().mixer.tracks.len() * 2);
    assert!(
        first.meters[0] > 0.1 && first.meters[1] > 0.1,
        "{:?}",
        first.meters
    );
    assert!(first.voices > 0);
    rig.run(SAMPLE_RATE as usize / 4);
    let second = session.realtime_tick();
    assert!(second.tick > first.tick);
    assert!(!session.transport_stop().playing);
    rig.run(960);
    let stopped = session.realtime_tick();
    assert!(!stopped.playing);
    assert_eq!(stopped.tick, 0.0);

    // Save it.
    let path = session.project_save(Some(&rig.file("My Beat"))).unwrap();
    assert!(path.ends_with("My Beat.windfall"), "{path}");
    let saved = rig.project();
    assert!(!session.document_snapshot().dirty);

    // Start over, then open what was saved.
    session.project_new().unwrap();
    assert_ne!(rig.project(), saved);
    rig.events.take();
    let opened = session.project_open(&path).unwrap();
    assert_eq!(opened.project, saved);
    assert_eq!(opened.path.as_deref(), Some(path.as_str()));
    assert_eq!(opened.revision, 0);
    assert!(!opened.dirty);
    assert!(opened.history.entries.is_empty());
    assert!(
        !rig.events
            .take()
            .iter()
            .any(|event| matches!(event, Event::ProjectWarnings(_)))
    );
    for sample in &saved.samples {
        assert!(rig.has_audio(sample.id), "{sample:?}");
    }
    assert_eq!(
        saved.samples[4].path,
        SamplePath::Factory("Drums/Percussion/Rim Click.wav".to_owned())
    );

    // Export it in every bit depth.
    let mut exports = Vec::new();
    for (bit_depth, name) in [
        (BitDepth::Float32, "beat float.wav"),
        (BitDepth::Int24, "beat 24.wav"),
        (BitDepth::Int16, "beat 16.wav"),
    ] {
        let out = rig.file(name);
        session
            .export_audio(ExportOptions {
                path: out.clone(),
                format: ExportFormat::Wav,
                bit_depth,
                sample_rate: SAMPLE_RATE,
                mode: PlayMode::Pattern,
                pattern_loops: LOOPS,
                tail_secs: TAIL_SECS,
                auto_tail: false,
                ..ExportOptions::default()
            })
            .unwrap();
        let done = rig.events.wait_for_export();
        assert_eq!(done.error, None);
        assert_eq!(done.path, out);
        assert_eq!(done.fraction, 1.0);
        let fractions: Vec<f32> = rig
            .events
            .take()
            .iter()
            .filter_map(|event| match event {
                Event::ExportProgress(progress) => Some(progress.fraction),
                _ => None,
            })
            .collect();
        assert!(fractions.is_sorted(), "{fractions:?}");
        assert!(fractions.iter().all(|f| (0.0..=1.0).contains(f)));

        let audio = windfall_codec::decode_file(&out).unwrap();
        assert_eq!(audio.sample_rate(), SAMPLE_RATE);
        assert_eq!(audio.channels(), 2);
        // Two bars at 150 bpm are 3.2 seconds, and the tail adds half of one.
        let tail = (TAIL_SECS * SAMPLE_RATE as f32) as usize;
        assert_eq!(audio.frames(), FRAMES_PER_BAR * LOOPS as usize + tail);
        assert!(rms(audio.samples()) > 0.02, "{name} is silent");
        assert_kicks_are_where_the_steps_are(&audio, name);
        exports.push(audio);
    }

    // The three files hold the same audio, each to the precision of its
    // format. Integer files clip at full scale; the float file does not.
    let reference = exports[0].samples();
    for (audio, step, name) in [
        (&exports[1], 1.0 / 8_388_608.0, "24-bit"),
        (&exports[2], 1.0 / 32_768.0, "16-bit"),
    ] {
        let worst = audio
            .samples()
            .iter()
            .zip(reference)
            .map(|(got, want)| (got - want.clamp(-1.0, 1.0)).abs())
            .fold(0.0_f32, f32::max);
        // Dither moves a sample by up to one step and rounding by half.
        assert!(worst <= step * 2.0, "{name} is off by {worst}");
    }
}
