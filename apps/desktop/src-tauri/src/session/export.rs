//! Exporting the project to an audio file.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use windfall_codec::{WavSampleFormat, WavWriter};
use windfall_core::{AudioBuffer, samples_per_tick};
use windfall_engine::{RenderOptions, SamplePool, render};
use windfall_ipc::{BitDepth, ExportFormat, ExportOptions, ExportProgress, PlayMode};
use windfall_project::{PatternId, Project};

use super::Session;
use crate::events::Event;
use crate::paths;

/// Sample rates an export may ask for.
const SAMPLE_RATES: std::ops::RangeInclusive<u32> = 8_000..=384_000;

/// Longest export, in frames. The render is held in memory as 32-bit float
/// stereo before it is written, so this is 2 GB of it, and it also keeps a
/// WAV file of any bit depth under the format's 4 GB limit.
const MAX_FRAMES: f64 = (2_u64 * 1024 * 1024 * 1024 / 8) as f64;

/// Shortest time between two progress events.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(50);

/// Share of the progress bar given to rendering. Writing the file takes the
/// rest.
const RENDER_SHARE: f32 = 0.9;

/// Samples handed to the file writer at a time.
const WRITE_CHUNK: usize = 1 << 16;

/// Everything an export needs, copied out of the session so the project can
/// be edited while the export runs.
struct Job {
    path: PathBuf,
    project: Project,
    pool: SamplePool,
    pattern: PatternId,
    options: ExportOptions,
}

/// Sends progress events, no more often than [`PROGRESS_INTERVAL`].
struct Progress<'a> {
    session: &'a Session,
    path: String,
    sent_at: Option<Instant>,
    fraction: f32,
}

impl Progress<'_> {
    fn report(&mut self, fraction: f32) {
        self.fraction = fraction.clamp(self.fraction, 1.0);
        if self
            .sent_at
            .is_some_and(|at| at.elapsed() < PROGRESS_INTERVAL)
        {
            return;
        }
        self.sent_at = Some(Instant::now());
        self.send(false, None);
    }

    fn send(&self, done: bool, error: Option<String>) {
        self.session.emit(Event::ExportProgress(ExportProgress {
            path: self.path.clone(),
            fraction: self.fraction,
            done,
            error,
        }));
    }
}

impl Session {
    /// Starts exporting the project to an audio file and returns at once.
    /// The export works from a copy of the project as it is now. Progress
    /// arrives as events, and the last one has `done` set and, if the
    /// export failed, says why.
    ///
    /// In pattern mode the pattern rendered is the one the transport has
    /// selected. Only one export runs at a time.
    pub fn export_audio(&self, options: ExportOptions) -> Result<(), String> {
        if options.path.trim().is_empty() {
            return Err("Choose where to save the file.".to_owned());
        }
        let path = paths::absolute(&options.path)?;
        if !SAMPLE_RATES.contains(&options.sample_rate) {
            return Err(format!(
                "{} Hz is not a sample rate Windfall can export.",
                options.sample_rate
            ));
        }
        if !options.tail_secs.is_finite() || options.tail_secs < 0.0 {
            return Err("The tail must be zero seconds or longer.".to_owned());
        }
        if options.mode == PlayMode::Pattern && options.pattern_loops < 1 {
            return Err("Render the pattern at least once.".to_owned());
        }

        let (project, pool) = {
            let state = self.state();
            (state.document.project().clone(), state.pool.clone())
        };
        let pattern = self.controller().transport().pattern;
        if options.mode == PlayMode::Song && project.playlist.clips.is_empty() {
            return Err("The playlist is empty, so there is no song to export.".to_owned());
        }
        let frames = expected_frames(&project, pattern, &options);
        if frames > MAX_FRAMES {
            let minutes = |frames: f64| frames / f64::from(options.sample_rate) / 60.0;
            return Err(format!(
                "The export would be {:.0} minutes long. The longest Windfall can export at {} Hz is {:.0} minutes.",
                minutes(frames),
                options.sample_rate,
                minutes(MAX_FRAMES).floor(),
            ));
        }

        if self.inner.exporting.swap(true, Ordering::SeqCst) {
            return Err("An export is already running.".to_owned());
        }
        let job = Job {
            path,
            project,
            pool,
            pattern,
            options,
        };
        let session = self.clone();
        let spawned = std::thread::Builder::new()
            .name("windfall-export".to_owned())
            .spawn(move || session.run_export(&job));
        spawned.map(drop).map_err(|error| {
            self.inner.exporting.store(false, Ordering::SeqCst);
            format!("Could not start the export: {error}")
        })
    }

    fn run_export(&self, job: &Job) {
        let mut progress = Progress {
            session: self,
            path: job.options.path.clone(),
            sent_at: None,
            fraction: 0.0,
        };
        // A panic must not leave the session refusing every later export.
        let outcome = catch_unwind(AssertUnwindSafe(|| write_export(job, &mut progress)));
        let error = match outcome {
            Ok(Ok(())) => {
                progress.fraction = 1.0;
                None
            }
            Ok(Err(reason)) => Some(format!(
                "Could not export to \"{}\": {reason}",
                paths::display(&job.path)
            )),
            Err(_) => Some("The export stopped unexpectedly.".to_owned()),
        };
        if let Some(error) = &error {
            log::warn!("{error}");
        }
        // Cleared first, so a UI that starts the next export on hearing
        // `done` is not refused.
        self.inner.exporting.store(false, Ordering::SeqCst);
        progress.send(true, error);
    }
}

/// Renders the job and writes it to its file.
fn write_export(job: &Job, progress: &mut Progress) -> Result<(), String> {
    let options = &job.options;
    let audio = render(
        &job.project,
        &job.pool,
        &RenderOptions {
            sample_rate: options.sample_rate,
            mode: options.mode,
            pattern: Some(job.pattern),
            pattern_loops: options.pattern_loops,
            tail_secs: options.tail_secs,
            ..RenderOptions::default()
        },
        &mut |fraction| {
            progress.report(fraction * RENDER_SHARE);
            true
        },
    );
    if audio.frames() == 0 {
        return Err("there is nothing to export".to_owned());
    }

    match options.format {
        ExportFormat::Wav => write_wav(job, &audio, progress),
    }
}

fn write_wav(job: &Job, audio: &AudioBuffer, progress: &mut Progress) -> Result<(), String> {
    let format = match job.options.bit_depth {
        BitDepth::Int16 => WavSampleFormat::Int16,
        BitDepth::Int24 => WavSampleFormat::Int24,
        BitDepth::Float32 => WavSampleFormat::Float32,
    };
    let mut writer = WavWriter::create(&job.path, audio.sample_rate(), audio.channels(), format)
        .map_err(|error| error.to_string())?;
    let samples = audio.samples();
    let mut written = 0;
    for chunk in samples.chunks(WRITE_CHUNK) {
        writer.write(chunk).map_err(|error| error.to_string())?;
        written += chunk.len();
        let share = written as f32 / samples.len() as f32;
        progress.report(RENDER_SHARE + (1.0 - RENDER_SHARE) * share);
    }
    writer.finalize().map_err(|error| error.to_string())
}

/// How many frames an export will be, near enough to refuse one that is far
/// too long before any memory is set aside for it.
fn expected_frames(project: &Project, pattern: PatternId, options: &ExportOptions) -> f64 {
    let ticks = match options.mode {
        PlayMode::Pattern => {
            let length = project
                .pattern(pattern)
                .or(project.patterns.first())
                .map_or(0, |pattern| pattern.length_ticks());
            f64::from(length) * f64::from(options.pattern_loops)
        }
        PlayMode::Song => project
            .playlist
            .clips
            .iter()
            .map(|clip| f64::from(clip.start) + f64::from(clip.length))
            .fold(0.0, f64::max),
    };
    let sample_rate = f64::from(options.sample_rate);
    ticks * samples_per_tick(project.settings.tempo_bpm, sample_rate)
        + f64::from(options.tail_secs) * sample_rate
}
