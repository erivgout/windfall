//! Pure shared slicer operations, run in a dedicated Web Worker by the browser.
use super::{Reply, json, parse};
use serde::Deserialize;
use windfall_project::{
    Clip,
    slicer::{self, SliceOptions},
};

pub fn slice_analyze(_: u32, input: &str) -> Reply {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Request {
        clip: Clip,
        tempo: f64,
        swing: f32,
        options: SliceOptions,
        sample_rate: u32,
        channels: u16,
        samples: Vec<f32>,
    }
    let request: Request = parse("slice analysis", input)?;
    if request.sample_rate == 0
        || !(1..=2).contains(&request.channels)
        || !request
            .samples
            .len()
            .is_multiple_of(usize::from(request.channels))
        || request.samples.len() > 64_000_000
    {
        return Err("Invalid or oversized slicer audio buffer.".into());
    }
    let audio = windfall_core::AudioBuffer::from_interleaved(
        request.sample_rate,
        request.channels,
        request.samples,
    );
    json(&slicer::analyze(
        &audio,
        &request.clip,
        request.tempo,
        request.swing,
        request.options,
    )?)
}
pub fn slice_command(_: u32, input: &str) -> Reply {
    #[derive(Deserialize)]
    struct Request {
        clip: Clip,
        markers: Vec<u32>,
        tempo: f64,
        swing: f32,
    }
    let request: Request = parse("slice boundaries", input)?;
    json(&slicer::split_command(
        &request.clip,
        &request.markers,
        request.tempo,
        request.swing,
    )?)
}
