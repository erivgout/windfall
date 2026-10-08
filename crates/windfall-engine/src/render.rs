//! Offline rendering: the realtime audio path run as fast as it will go.

use windfall_core::{AudioBuffer, db_to_gain, samples_per_tick};
use windfall_ipc::{PlayMode, TransportPatch};
use windfall_project::{PatternId, Project};

use crate::controller::Controller;
use crate::plan::compile_render as compile;
use crate::pool::SamplePool;

/// What [`render`] produces.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderOptions {
    pub sample_rate: u32,
    /// Pattern mode renders one pattern `pattern_loops` times. Song mode
    /// renders the playlist from the start to the end of its last clip.
    pub mode: PlayMode,
    /// The pattern to render in pattern mode. `None`, or an id the project
    /// does not have, means the project's first pattern.
    pub pattern: Option<PatternId>,
    pub pattern_loops: u32,
    /// Extra time rendered after the end, so notes that are still sounding
    /// ring out instead of being cut off. With `auto_tail` it is the
    /// longest the tail may get.
    pub tail_secs: f32,
    /// Ends the tail as soon as everything has rung out, instead of always
    /// rendering all of `tail_secs`: once no note, instrument or effect can
    /// sound any more, the audio ends right after its last frame louder
    /// than [`TAIL_SILENCE_DB`].
    pub auto_tail: bool,
    /// Frames processed per step. It sets how often `progress` is called
    /// and has no effect on the audio.
    pub block_frames: usize,
}

/// The level below which an automatic tail counts as over, in decibels
/// relative to full scale.
pub const TAIL_SILENCE_DB: f32 = -90.0;

/// How long the output has to stay that quiet, once nothing can sound any
/// more, before an automatic tail is taken to be over.
pub(crate) const TAIL_HOLD_SECONDS: f64 = 0.1;

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            mode: PlayMode::Pattern,
            pattern: None,
            pattern_loops: 1,
            tail_secs: 0.0,
            auto_tail: false,
            block_frames: 1024,
        }
    }
}

/// What [`render_reporting`] returns: the audio, and what the listener
/// should be told about it.
#[derive(Debug, Clone)]
pub struct Rendered {
    pub audio: AudioBuffer,
    /// Audio clips of the playlist that are not in the audio, because
    /// [`MAX_AUDIO_CLIPS`](crate::MAX_AUDIO_CLIPS) were playing already
    /// when each was to start. They are the ones that start last and, of
    /// those that start on one tick, the ones with the highest ids.
    pub dropped_clips: u32,
    pub sampler_error: Option<crate::sampler_processing::SamplerPreparationError>,
}

/// Renders a project to stereo audio. See [`render_reporting`], which this
/// is without the report.
pub fn render(
    project: &Project,
    pool: &SamplePool,
    options: &RenderOptions,
    progress: &mut dyn FnMut(f32) -> bool,
) -> AudioBuffer {
    render_reporting(project, pool, options, progress).audio
}

/// Renders a project to stereo audio with the same
/// [`Processor`](crate::Processor) that plays it live, so the result is what
/// playback sounds like.
///
/// The same input always gives bit-identical output. `progress` is called
/// after every block with the fraction done, 0 to 1; returning `false` from
/// it cancels the render, and the audio rendered so far is returned. With
/// `auto_tail` the fraction is of the longest the render can get, so it
/// may end well short of 1.
///
/// Instruments and effects can put their output out late, a limiter by its
/// look-ahead for one. The engine lines every path up with the slowest and
/// the render leaves that much off its front, so a note on the first tick
/// is heard on the first frame, as it would be with no latency anywhere.
///
/// Automation stays where the song's end left it for the whole of the
/// tail: nobody is there to play on, so nothing returns to its stored
/// value.
pub fn render_reporting(
    project: &Project,
    pool: &SamplePool,
    options: &RenderOptions,
    progress: &mut dyn FnMut(f32) -> bool,
) -> Rendered {
    let sample_rate = options.sample_rate.max(1);
    let prepared_pool = match pool.prepare_samplers(project, &mut || true, &mut |_, _, _| {}) {
        Ok(pool) => pool,
        Err(error) => {
            return Rendered {
                audio: AudioBuffer::from_interleaved(sample_rate, 2, Vec::new()),
                dropped_clips: 0,
                sampler_error: Some(error),
            };
        }
    };
    let plan = compile(project, &prepared_pool);
    let pattern = options
        .pattern
        .filter(|id| plan.pattern_ids.get(id.0).is_some())
        .or(plan.patterns.first().map(|pattern| pattern.id));
    // The length in ticks of the stored tempo. A song whose tempo is
    // automated takes as long as its tempo map says.
    let (passes, ticks) = match options.mode {
        PlayMode::Pattern => {
            let length = pattern
                .and_then(|id| plan.pattern_ids.get(id.0))
                .map_or(0, |index| plan.patterns[index].length);
            let ticks = u64::from(length) * u64::from(options.pattern_loops);
            (options.pattern_loops, ticks as f64)
        }
        PlayMode::Song => (1, plan.warp(f64::from(plan.song_end))),
    };
    let frames_per_tick = samples_per_tick(plan.tempo_bpm, f64::from(sample_rate));
    let body_frames = (ticks * frames_per_tick).ceil() as usize;
    let tail_frames = if options.tail_secs.is_finite() {
        (f64::from(options.tail_secs.max(0.0)) * f64::from(sample_rate)).round() as usize
    } else {
        0
    };
    // The processor is made after the controller has the plan, so it starts
    // out on the project instead of changing over to it.
    let controller = Controller::new();
    controller.set_plan(plan);
    let mut processor = controller.attach(sample_rate);
    controller.set_transport(TransportPatch {
        mode: Some(options.mode),
        pattern,
        loop_song: Some(false),
    });
    if ticks > 0.0 {
        controller.play_passes(passes);
    }

    // Everything comes out of the processor this many frames late.
    let latency = controller.latency_frames() as usize;
    let most = latency + body_frames + tail_frames;
    let body_end = latency + body_frames;
    let hold = (TAIL_HOLD_SECONDS * f64::from(sample_rate)).round() as usize;
    let silence = db_to_gain(TAIL_SILENCE_DB);

    // Once nothing can sound any more the output only dies away, so quiet
    // for this long is quiet for good.
    let rung_out = |processor: &crate::Processor, done: usize, loud_end: usize| {
        done >= body_end && done >= loud_end + hold && processor.settled()
    };

    let block = options.block_frames.max(1);
    let certain = if options.auto_tail { body_end } else { most };
    let mut data = Vec::with_capacity(certain * 2);
    let mut done = 0;
    // One past the last frame that was not silent.
    let mut loud_end = 0;
    let mut cancelled = false;
    while done < most {
        let frames = block.min(most - done);
        data.resize((done + frames) * 2, 0.0);
        let fresh = &mut data[done * 2..];
        processor.process(fresh);
        let (fresh, _) = fresh.as_chunks::<2>();
        let loud = |frame: &[f32; 2]| frame[0].abs() >= silence || frame[1].abs() >= silence;
        if let Some(last) = fresh.iter().rposition(loud) {
            loud_end = done + last + 1;
        }
        done += frames;
        if !progress(done as f32 / most as f32) {
            cancelled = true;
            break;
        }
        if options.auto_tail && rung_out(&processor, done, loud_end) {
            break;
        }
    }
    if options.auto_tail && !cancelled && rung_out(&processor, done, loud_end) {
        // The same length whatever the block size: it only depends on
        // where the sound ends. A tail that was still sounding when it ran
        // out of time is kept whole.
        data.truncate(loud_end.max(body_end) * 2);
    }
    data.drain(..(latency * 2).min(data.len()));
    Rendered {
        audio: AudioBuffer::from_interleaved(sample_rate, 2, data),
        dropped_clips: processor.clips_left_out(),
        sampler_error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_pattern_renders_silence_of_the_right_length() {
        let project = Project::new("empty");
        let options = RenderOptions {
            pattern_loops: 2,
            tail_secs: 0.5,
            ..RenderOptions::default()
        };
        let mut calls = 0;
        let audio = render(&project, &SamplePool::new(), &options, &mut |fraction| {
            calls += 1;
            assert!(fraction > 0.0 && fraction <= 1.0);
            true
        });
        // Two bars at 120 bpm are four seconds, and the tail adds half of one.
        assert_eq!(audio.frames(), 192_000 + 24_000);
        assert_eq!(audio.channels(), 2);
        assert!(audio.samples().iter().all(|sample| *sample == 0.0));
        assert_eq!(calls, (192_000 + 24_000_usize).div_ceil(1024));
    }

    #[test]
    fn returning_false_from_progress_cancels() {
        let project = Project::new("empty");
        let options = RenderOptions {
            block_frames: 100,
            ..RenderOptions::default()
        };
        let audio = render(&project, &SamplePool::new(), &options, &mut |_| false);
        assert_eq!(audio.frames(), 100);
    }
}
