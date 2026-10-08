//! The same checked musical conversions and region validation as native editing.
use crate::{Reply, json, parse};
use windfall_project::{MeterChange, MeterMap, TickRange, TimeSignature};

pub fn timeline_range(_handle: u32, input: &str) -> Reply {
    let range: Option<TickRange> = parse("timeline range", input)?;
    if let Some(range) = range {
        range.check()?;
    }
    json(&range)
}

pub fn timeline_position(_handle: u32, input: &str) -> Reply {
    #[derive(serde::Deserialize)]
    struct Request {
        legacy: TimeSignature,
        meters: Vec<MeterChange>,
        tick: u32,
    }
    let request: Request = parse("meter position", input)?;
    json(&MeterMap::new(request.legacy, &request.meters)?.tick_to_position(request.tick)?)
}
