//! Bounded echo through the prepared project and the public audio path.
use windfall_core::AudioBuffer;
use windfall_engine::{Processor, SamplePool};
use windfall_project::*;

#[test]
fn channel_voice_prepared_bounded_echo_reaches_audio_and_stops() {
    let mut document = Document::new(Project::new("Echo"));
    let sample = SampleId(
        document
            .dispatch(
                Command::AddSample {
                    name: "Impulse".into(),
                    path: SamplePath::Project("impulse.wav".into()),
                },
                None,
            )
            .unwrap()
            .created[0],
    );
    let channel = ChannelId(
        document
            .dispatch(
                Command::AddChannel {
                    name: None,
                    sample: Some(sample),
                    instrument: None,
                    index: None,
                    mixer_track: None,
                },
                None,
            )
            .unwrap()
            .created[0],
    );
    let settings = ChannelVoiceSettings {
        echo: NoteEchoSettings {
            enabled: true,
            time: EchoTime::Milliseconds { ms: 10.0 },
            feedback: 0.5,
            repeats: 2,
            pitch_semitones: 0,
        },
        ..Default::default()
    };
    document
        .dispatch(
            Command::SetChannelVoiceSettings {
                id: channel,
                settings,
            },
            None,
        )
        .unwrap();
    let pattern = document.project().patterns[0].id;
    document
        .dispatch(
            Command::AddNotes {
                pattern,
                channel,
                notes: vec![NoteInit {
                    start: 0,
                    length: 1,
                    key: DEFAULT_KEY,
                    velocity: Some(1.0),
                    pan: Some(0.0),
                    expression: None,
                }],
            },
            None,
        )
        .unwrap();
    let mut pool = SamplePool::new();
    pool.insert(sample, AudioBuffer::from_interleaved(48_000, 1, vec![1.0]));
    let (mut processor, controller) = Processor::new(48_000);
    controller.set_project(document.project(), &pool);
    controller.play();
    let mut output = [0.0; 2400];
    for block in output.chunks_mut(254) {
        processor.process(block);
    }
    let peaks: Vec<_> = output
        .as_chunks::<2>()
        .0
        .iter()
        .enumerate()
        .filter(|(_, f)| f[0].abs() > 0.001)
        .map(|(i, f)| (i, f[0]))
        .collect();
    assert_eq!(
        peaks.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
        [0, 480, 960]
    );
    assert!((peaks[1].1 / peaks[0].1 - 0.5).abs() < 1e-5);
    assert!((peaks[2].1 / peaks[0].1 - 0.25).abs() < 1e-5);
}
