//! Soak test: plays a drum pattern through the real output device and
//! reports dropouts.
//!
//! ```text
//! windfall-soak [--seconds N] [--buffer N] [--sample-rate N] [--host NAME]
//!               [--device NAME] [--silent] [--list-devices]
//! ```
//!
//! The process exits with 0 only if the stream ran the whole time without a
//! single xrun.

use std::process::ExitCode;
use std::time::{Duration, Instant};

use windfall_core::{AudioBuffer, TICKS_PER_STEP, gain_to_db};
use windfall_engine::{Engine, SamplePool};
use windfall_ipc::AudioSettings;
use windfall_project::{
    Channel, ChannelId, ChannelSource, DEFAULT_KEY, Lane, MixerTrack, Note, NoteId, Project,
    SampleAsset, SampleId, SamplePath, SamplerSettings, TrackId, palette_color,
};

/// Rate the drum sounds are generated at. It differs from the usual 48 kHz
/// device rate on purpose, so the test also exercises the resampler.
const SOUND_RATE: u32 = 44_100;

struct Options {
    seconds: u64,
    buffer_frames: u32,
    sample_rate: Option<u32>,
    host: Option<String>,
    device: Option<String>,
    silent: bool,
    list_devices: bool,
}

fn main() -> ExitCode {
    let options = match parse_options(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("windfall-soak: {message}");
            eprintln!(
                "usage: windfall-soak [--seconds N] [--buffer N] [--sample-rate N] \
                 [--host NAME] [--device NAME] [--silent] [--list-devices]"
            );
            return ExitCode::from(2);
        }
    };
    if options.list_devices {
        list_devices();
        return ExitCode::SUCCESS;
    }
    if soak(&options) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn parse_options(mut arguments: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut options = Options {
        seconds: 600,
        buffer_frames: 128,
        sample_rate: None,
        host: None,
        device: None,
        silent: false,
        list_devices: false,
    };
    while let Some(flag) = arguments.next() {
        let mut value = || {
            arguments
                .next()
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match flag.as_str() {
            "--seconds" => options.seconds = number(&flag, &value()?)?,
            "--buffer" => options.buffer_frames = number(&flag, &value()?)?,
            "--sample-rate" => options.sample_rate = Some(number(&flag, &value()?)?),
            "--host" => options.host = Some(value()?),
            "--device" => options.device = Some(value()?),
            "--silent" => options.silent = true,
            "--list-devices" => options.list_devices = true,
            other => return Err(format!("unknown option {other}")),
        }
    }
    Ok(options)
}

fn number<T: std::str::FromStr>(flag: &str, value: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("{flag} needs a whole number, got \"{value}\""))
}

fn list_devices() {
    let hosts = Engine::devices();
    if hosts.is_empty() {
        println!("no audio hosts found");
    }
    for host in hosts {
        let default = if host.is_default { " (default)" } else { "" };
        println!("{}{default}", host.name);
        for device in host.devices {
            let default = if device.is_default { " (default)" } else { "" };
            let buffers = match (device.min_buffer_frames, device.max_buffer_frames) {
                (Some(min), Some(max)) => format!("{min}-{max} frames"),
                _ => "buffer range unknown".to_owned(),
            };
            println!("  {}{default}", device.name);
            println!("    sample rates: {:?}", device.sample_rates);
            println!("    buffers: {buffers}");
        }
    }
}

/// Runs the soak and returns whether it passed.
fn soak(options: &Options) -> bool {
    let engine = Engine::start(&AudioSettings {
        host: options.host.clone(),
        device: options.device.clone(),
        sample_rate: options.sample_rate,
        buffer_frames: Some(options.buffer_frames),
    });
    let status = engine.status();
    if !status.running {
        let reason = status.error.unwrap_or_else(|| "unknown error".to_owned());
        println!("could not open an output stream: {reason}");
        println!(
            "SOAK RESULT: fail xruns=0 max_cpu=0.0% buffer_frames=0 sample_rate={}",
            status.sample_rate
        );
        return false;
    }
    println!(
        "playing on {} / {} at {} Hz, {} frames requested, {} seconds{}",
        status.host,
        status.device.as_deref().unwrap_or("unknown device"),
        status.sample_rate,
        options.buffer_frames,
        options.seconds,
        if options.silent { ", silent" } else { "" },
    );

    let controller = engine.controller();
    let (project, pool) = drum_project();
    controller.set_project(&project, &pool);
    if options.silent {
        controller.set_output_gain(0.0);
        // The gain glides to zero over a few milliseconds. Playback waits
        // until it is there, so not even the first hit is heard.
        std::thread::sleep(Duration::from_millis(200));
    }
    controller.play();

    let started = Instant::now();
    let mut max_cpu = 0.0_f32;
    let mut frames_before = 0;
    let mut stalled = false;
    let mut stream_error = None;
    for second in 1..=options.seconds {
        let wake = started + Duration::from_secs(second);
        std::thread::sleep(wake.saturating_duration_since(Instant::now()));

        let frame = controller.frame();
        let stats = controller.stream_stats();
        max_cpu = max_cpu.max(stats.cpu_peak);
        let master_peak = frame.meters.iter().take(2).fold(0.0_f32, |a, b| a.max(*b));
        println!(
            "{second:>5}s  xruns={}  cpu avg={:.2}% max={:.2}%  voices={}  master peak={:.1} dB",
            stats.xruns,
            stats.cpu * 100.0,
            stats.cpu_peak * 100.0,
            frame.voices,
            gain_to_db(master_peak),
        );

        let status = engine.status();
        if !status.running {
            stream_error = Some(
                status
                    .error
                    .unwrap_or_else(|| "the stream stopped".to_owned()),
            );
            break;
        }
        // A stream that is open but no longer asks for audio has failed too.
        if stats.frames == frames_before {
            stalled = true;
            break;
        }
        frames_before = stats.frames;
    }

    let status = engine.status();
    let stats = controller.stream_stats();
    max_cpu = max_cpu.max(stats.cpu_peak);
    if let Some(error) = &stream_error {
        println!("the stream failed: {error}");
    }
    if stalled {
        println!("the device stopped asking for audio");
    }
    let passed = stream_error.is_none() && !stalled && stats.xruns == 0;
    println!(
        "SOAK RESULT: {} xruns={} max_cpu={:.2}% buffer_frames={} sample_rate={}",
        if passed { "pass" } else { "fail" },
        stats.xruns,
        max_cpu * 100.0,
        status.buffer_frames,
        status.sample_rate,
    );
    passed
}

/// A one-bar four-on-the-floor beat: kick on every beat, snare on beats two
/// and four, and closed hats on every eighth note.
fn drum_project() -> (Project, SamplePool) {
    let mut project = Project::new("Soak");
    // Off the round 120, so the once-a-second report does not always catch
    // the beat at the same point.
    project.settings.tempo_bpm = 128.0;
    let mut pool = SamplePool::new();
    let pattern = &mut project.patterns[0];
    let rows: [(&str, AudioBuffer, &[u32], f32); 3] = [
        ("Kick", kick(), &[0, 4, 8, 12], 0.5),
        ("Snare", snare(), &[4, 12], 0.4),
        ("Hat", hat(), &[0, 2, 4, 6, 8, 10, 12, 14], 0.25),
    ];
    for (index, (name, sound, steps, volume)) in rows.into_iter().enumerate() {
        let mut next_id = || {
            project.next_id += 1;
            project.next_id - 1
        };
        let sample = SampleId(next_id());
        let track = TrackId(next_id());
        let channel = ChannelId(next_id());
        pool.insert(sample, sound);
        project.samples.push(SampleAsset {
            id: sample,
            name: name.to_owned(),
            path: SamplePath::Factory(format!("soak/{}.wav", name.to_lowercase())),
        });
        project.mixer.tracks.push(MixerTrack {
            id: track,
            name: name.to_owned(),
            color: palette_color(index),
            volume: 1.0,
            pan: 0.0,
            muted: false,
            solo: false,
            output: Some(TrackId::MASTER),
            sends: Vec::new(),
        });
        project.channels.push(Channel {
            id: channel,
            name: name.to_owned(),
            color: palette_color(index),
            volume,
            pan: 0.0,
            muted: false,
            solo: false,
            mixer_track: track,
            source: ChannelSource::Sampler(SamplerSettings {
                sample: Some(sample),
                // A new hat chokes the one before it, as on a real kit.
                cut_self: name == "Hat",
                ..SamplerSettings::default()
            }),
        });
        pattern.lanes.push(Lane {
            channel,
            notes: steps
                .iter()
                .map(|step| Note {
                    id: NoteId(next_id()),
                    start: step * TICKS_PER_STEP,
                    length: TICKS_PER_STEP,
                    key: DEFAULT_KEY,
                    velocity: 1.0,
                    pan: 0.0,
                })
                .collect(),
        });
    }
    (project, pool)
}

/// A sine that falls from 160 Hz to 45 Hz while it dies away.
fn kick() -> AudioBuffer {
    let mut phase = 0.0_f32;
    synthesize(0.35, |time| {
        let frequency = 45.0 + 115.0 * (-time * 28.0).exp();
        phase += frequency / SOUND_RATE as f32;
        (phase * std::f32::consts::TAU).sin() * (-time * 9.0).exp() * 0.9
    })
}

/// A burst of noise over a short 190 Hz body.
fn snare() -> AudioBuffer {
    let mut noise = Noise(0x1234_5678);
    synthesize(0.22, |time| {
        let body = (time * 190.0 * std::f32::consts::TAU).sin() * (-time * 30.0).exp();
        (noise.next() * (-time * 22.0).exp() * 0.7 + body * 0.4) * 0.8
    })
}

/// A very short burst of noise with the lows taken out.
fn hat() -> AudioBuffer {
    let mut noise = Noise(0x0BAD_5EED);
    let mut previous = 0.0;
    synthesize(0.06, |time| {
        let sample = noise.next();
        // The difference of neighbouring samples is a crude high-pass.
        let bright = (sample - previous) * 0.5;
        previous = sample;
        bright * (-time * 70.0).exp() * 0.6
    })
}

/// Builds a mono sound from a function of time in seconds, faded out over
/// its last few milliseconds so it ends on silence.
fn synthesize(seconds: f32, mut wave: impl FnMut(f32) -> f32) -> AudioBuffer {
    let frames = (seconds * SOUND_RATE as f32) as usize;
    let fade_frames = (0.005 * SOUND_RATE as f32) as usize;
    let data = (0..frames)
        .map(|frame| {
            let fade = ((frames - frame) as f32 / fade_frames as f32).min(1.0);
            wave(frame as f32 / SOUND_RATE as f32) * fade
        })
        .collect();
    AudioBuffer::from_interleaved(SOUND_RATE, 1, data)
}

/// White noise from a linear congruential generator, so every run of the
/// soak plays exactly the same sounds.
struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 8) as f32 / (1 << 23) as f32 - 1.0
    }
}
