//! Offline rendering: the realtime audio path run as fast as it will go.

use std::sync::Arc;

use windfall_core::{AudioBuffer, samples_per_tick};
use windfall_ipc::{PlayMode, TransportPatch};
use windfall_project::{PatternId, Project};

use crate::plan::compile;
use crate::pool::SamplePool;
use crate::processor::Processor;

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
    /// ring out instead of being cut off.
    pub tail_secs: f32,
    /// Frames processed per step. It sets how often `progress` is called
    /// and has no effect on the audio.
    pub block_frames: usize,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            mode: PlayMode::Pattern,
            pattern: None,
            pattern_loops: 1,
            tail_secs: 0.0,
            block_frames: 1024,
        }
    }
}

/// Renders a project to stereo audio with the same [`Processor`] that plays
/// it live, so the result is what playback sounds like.
///
/// The same input always gives bit-identical output. `progress` is called
/// after every block with the fraction done, 0 to 1; returning `false` from
/// it cancels the render, and the audio rendered so far is returned.
pub fn render(
    project: &Project,
    pool: &SamplePool,
    options: &RenderOptions,
    progress: &mut dyn FnMut(f32) -> bool,
) -> AudioBuffer {
    let sample_rate = options.sample_rate.max(1);
    let plan = Arc::new(compile(project, pool));
    let pattern = options
        .pattern
        .filter(|id| plan.pattern_ids.get(id.0).is_some())
        .or(plan.patterns.first().map(|pattern| pattern.id));
    let (passes, ticks) = match options.mode {
        PlayMode::Pattern => {
            let length = pattern
                .and_then(|id| plan.pattern_ids.get(id.0))
                .map_or(0, |index| plan.patterns[index].length);
            (
                options.pattern_loops,
                u64::from(length) * u64::from(options.pattern_loops),
            )
        }
        PlayMode::Song => (1, u64::from(plan.song_end)),
    };
    let frames_per_tick = samples_per_tick(plan.tempo_bpm, f64::from(sample_rate));
    let body_frames = (ticks as f64 * frames_per_tick).ceil() as usize;
    let tail_frames = if options.tail_secs.is_finite() {
        (f64::from(options.tail_secs.max(0.0)) * f64::from(sample_rate)).round() as usize
    } else {
        0
    };
    let total_frames = body_frames + tail_frames;

    let (mut processor, controller) = Processor::new(sample_rate);
    controller.set_plan(plan);
    controller.set_transport(TransportPatch {
        mode: Some(options.mode),
        pattern,
        loop_song: Some(false),
    });
    if ticks > 0 {
        controller.play_passes(passes);
    }

    let mut data = vec![0.0_f32; total_frames * 2];
    let block_samples = options.block_frames.max(1) * 2;
    let mut done = 0;
    for block in data.chunks_mut(block_samples) {
        processor.process(block);
        done += block.len();
        if !progress(done as f32 / (total_frames * 2) as f32) {
            break;
        }
    }
    data.truncate(done);
    AudioBuffer::from_interleaved(sample_rate, 2, data)
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
