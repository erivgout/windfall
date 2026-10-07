//! Exporting the project to an audio file.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use windfall_codec::{WavSampleFormat, WavWriter};
use windfall_core::{AudioBuffer, samples_per_tick};
use windfall_engine::{RenderOptions, SamplePool, render_reporting};
use windfall_ipc::{BitDepth, ExportFormat, ExportOptions, ExportProgress, PlayMode};
use windfall_project::{AutomationRange, AutomationTarget, PatternId, Project};

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
        self.send(false, None, 0);
    }

    /// `dropped_clips` is for the event that ends the export: the audio
    /// clips the render had no room to play.
    fn send(&self, done: bool, error: Option<String>, dropped_clips: u32) {
        self.session.emit(Event::ExportProgress(ExportProgress {
            path: self.path.clone(),
            fraction: self.fraction,
            done,
            error,
            dropped_clips,
        }));
    }
}

impl Session {
    /// Starts exporting the project to an audio file and returns at once.
    /// The export works from a copy of the project as it is now. Progress
    /// arrives as events, and the last one has `done` set and, if the
    /// export failed, says why. It also says how many audio clips are not
    /// in the file because more overlapped than the engine plays at once.
    ///
    /// In pattern mode the pattern rendered is the one the transport has
    /// selected. Only one export runs at a time.
    ///
    /// The file starts where the song starts: whatever time the project's
    /// effects and instruments take to put their output out is left off
    /// its front. It ends `tail_secs` after the end of the song, or, with
    /// `auto_tail`, as soon as everything has rung out, if that is sooner.
    ///
    /// A file name with no extension gets the one of the format, and a
    /// name that asks for another format is refused; see [`export_path`].
    /// Progress events carry the path as it was given here, which is what
    /// the UI matches them by.
    pub fn export_audio(&self, options: ExportOptions) -> Result<(), String> {
        if options.path.trim().is_empty() {
            return Err("Choose where to save the file.".to_owned());
        }
        let path = export_path(paths::absolute(&options.path)?, options.format)?;
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

        // All three under one lock, so the pattern is one of this project's
        // and not of a project opened a moment later.
        let (project, pool, pattern) = {
            let state = self.state();
            (
                state.document.project().clone(),
                state.pool.clone(),
                self.controller().transport().pattern,
            )
        };
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
        let mut dropped_clips = 0;
        let error = match outcome {
            Ok(Ok(dropped)) => {
                progress.fraction = 1.0;
                dropped_clips = dropped;
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
        progress.send(true, error, dropped_clips);
    }
}

/// The file an export writes, from the path the user gave.
///
/// A name with no extension gets the format's, as saving a project adds
/// `.windfall`. A name that ends in another extension is refused, because
/// writing a WAV file called `beat.mp3` helps nobody. Only something that
/// looks like an extension counts as one: up to five letters and digits
/// with a letter among them. The `2` of `take.2` and the ` Intro` of
/// `01. Intro` are part of the name, and the extension is added after them.
fn export_path(path: PathBuf, format: ExportFormat) -> Result<PathBuf, String> {
    let wanted = match format {
        ExportFormat::Wav => "wav",
    };
    let given = path
        .extension()
        .and_then(|extension| extension.to_str())
        .filter(|extension| {
            extension.len() <= 5
                && extension.chars().all(|c| c.is_ascii_alphanumeric())
                && extension.chars().any(|c| c.is_ascii_alphabetic())
        });
    match given {
        Some(extension) if extension.eq_ignore_ascii_case(wanted) => Ok(path),
        Some(extension) => Err(format!(
            "Windfall exports {} files, so the file name cannot end in .{extension}. End it in .{wanted}, or leave the ending off.",
            wanted.to_uppercase()
        )),
        None => {
            let mut name = path.into_os_string();
            name.push(".");
            name.push(wanted);
            Ok(PathBuf::from(name))
        }
    }
}

/// Renders the job and writes it to its file. Returns how many audio clips
/// the render left out for lack of room to play them.
fn write_export(job: &Job, progress: &mut Progress) -> Result<u32, String> {
    let options = &job.options;
    let rendered = render_reporting(
        &job.project,
        &job.pool,
        &RenderOptions {
            sample_rate: options.sample_rate,
            mode: options.mode,
            pattern: Some(job.pattern),
            pattern_loops: options.pattern_loops,
            tail_secs: options.tail_secs,
            auto_tail: options.auto_tail,
            ..RenderOptions::default()
        },
        &mut |fraction| {
            progress.report(fraction * RENDER_SHARE);
            true
        },
    );
    let audio = &rendered.audio;
    if audio.frames() == 0 {
        return Err("there is nothing to export".to_owned());
    }

    match options.format {
        ExportFormat::Wav => write_wav(job, audio, progress)?,
    }
    Ok(rendered.dropped_clips)
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

/// How many frames an export will be at most, near enough to refuse one
/// that is far too long before any memory is set aside for it.
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
    let tempo = match options.mode {
        PlayMode::Pattern => project.settings.tempo_bpm,
        PlayMode::Song => slowest_tempo(project),
    };
    let sample_rate = f64::from(options.sample_rate);
    ticks * samples_per_tick(tempo, sample_rate) + f64::from(options.tail_secs) * sample_rate
}

/// The slowest the song's tempo gets: the stored tempo, or the lowest
/// point of an automation of the tempo if that is slower. A song is never
/// longer than it would be at this tempo throughout.
fn slowest_tempo(project: &Project) -> f64 {
    let automations = project.automations.iter();
    let points = automations
        .filter(|automation| automation.target == AutomationTarget::Tempo)
        .flat_map(|automation| &automation.points);
    points
        .map(|point| f64::from(AutomationRange::TEMPO.value(point.value)))
        .fold(project.settings.tempo_bpm, f64::min)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_extension_of_the_format_is_added_when_the_name_has_none() {
        let named = |name: &str| export_path(PathBuf::from(name), ExportFormat::Wav);
        assert_eq!(named("beat"), Ok(PathBuf::from("beat.wav")));
        assert_eq!(named("beat.wav"), Ok(PathBuf::from("beat.wav")));
        assert_eq!(named("Beat.WAV"), Ok(PathBuf::from("Beat.WAV")));
        // A dot in the name does not make an extension of what follows.
        assert_eq!(named("take.2"), Ok(PathBuf::from("take.2.wav")));
        assert_eq!(named("01. Intro"), Ok(PathBuf::from("01. Intro.wav")));
        assert_eq!(named("v1.final-mix"), Ok(PathBuf::from("v1.final-mix.wav")));
        assert_eq!(named(".hidden"), Ok(PathBuf::from(".hidden.wav")));
    }

    #[test]
    fn a_song_is_sized_by_the_slowest_its_tempo_gets() {
        use windfall_project::{
            Automation, AutomationId, AutomationPoint, Clip, ClipContent, ClipId, PlaylistTrackId,
        };

        // Four bars of a pattern at a stored 120 bpm: eight seconds.
        let mut project = Project::new("Song");
        let pattern = project.patterns[0].id;
        project.playlist.clips.push(Clip {
            id: ClipId(50),
            track: PlaylistTrackId(40),
            start: 0,
            length: 15_360,
            offset: 0,
            muted: false,
            content: ClipContent::Pattern { pattern },
        });
        let options = ExportOptions {
            path: String::new(),
            format: ExportFormat::Wav,
            bit_depth: BitDepth::Float32,
            sample_rate: 48_000,
            mode: PlayMode::Song,
            pattern_loops: 1,
            tail_secs: 1.0,
            auto_tail: false,
        };
        assert_eq!(expected_frames(&project, pattern, &options), 9.0 * 48_000.0);

        // A tempo curve that dips to 60 bpm somewhere can make the song
        // twice as long, and that is what room is checked for.
        let point = |tick: u32, bpm: f32| AutomationPoint {
            tick,
            value: (bpm - 10.0) / 512.0,
            curve: 0.0,
            hold: false,
        };
        project.automations.push(Automation {
            id: AutomationId(60),
            name: String::new(),
            color: 0,
            target: AutomationTarget::Tempo,
            points: vec![point(0, 200.0), point(960, 60.0), point(1_920, 300.0)],
        });
        assert_eq!(slowest_tempo(&project), 60.0);
        assert_eq!(
            expected_frames(&project, pattern, &options),
            17.0 * 48_000.0
        );
        // A pattern is exported at the stored tempo, which no curve moves.
        let looped = ExportOptions {
            mode: PlayMode::Pattern,
            ..options
        };
        assert_eq!(expected_frames(&project, pattern, &looped), 3.0 * 48_000.0);
    }

    #[test]
    fn a_name_that_asks_for_another_format_is_refused() {
        for (name, extension) in [
            ("beat.mp3", "mp3"),
            ("beat.FLAC", "FLAC"),
            ("beat.wav.ogg", "ogg"),
            ("beat.txt", "txt"),
        ] {
            let error = export_path(PathBuf::from(name), ExportFormat::Wav).unwrap_err();
            assert_eq!(
                error,
                format!(
                    "Windfall exports WAV files, so the file name cannot end in .{extension}. End it in .wav, or leave the ending off."
                )
            );
        }
    }
}
