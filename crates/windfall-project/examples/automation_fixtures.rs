//! Prints reference values for automation as JSON: curves sampled at a
//! spread of ticks, and each kind of range mapped both ways.
//!
//! `scripts/gen-bindings.sh` writes the output to
//! `apps/desktop/src/bindings/automation-fixtures.json`. The UI draws
//! automation curves and shows values in real units with its own code, and
//! its tests compare that code against these numbers, which come from the
//! functions the engine itself uses.

use serde_json::{Value, json};
use windfall_dsp::{EffectKind, InstrumentKind};
use windfall_project::{AutomationPoint, AutomationRange, curve_shape, curve_value};

fn main() {
    let output = json!({
        "shapes": shapes(),
        "curves": curves(),
        "ranges": ranges(),
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output).expect("fixtures serialize")
    );
}

/// The bend of one segment at a spread of positions, for a spread of
/// tensions.
fn shapes() -> Vec<Value> {
    let parts: Vec<f64> = (0..=10).map(|step| step as f64 / 10.0).collect();
    [-1.0_f32, -0.5, -0.1, 0.0, 0.25, 0.75, 1.0]
        .iter()
        .map(|&curve| {
            let values: Vec<f64> = parts.iter().map(|&part| curve_shape(part, curve)).collect();
            json!({ "curve": curve, "parts": parts, "values": values })
        })
        .collect()
}

fn point(tick: u32, value: f32, curve: f32, hold: bool) -> AutomationPoint {
    AutomationPoint {
        tick,
        value,
        curve,
        hold,
    }
}

/// Whole curves read at ticks before, on, between and after their points.
fn curves() -> Vec<Value> {
    let cases = [
        vec![point(0, 0.25, 0.0, false)],
        vec![point(0, 0.0, 0.0, false), point(3840, 1.0, 0.0, false)],
        vec![
            point(960, 0.2, 0.6, false),
            point(2880, 0.9, -0.8, false),
            point(7680, 0.1, 0.0, false),
        ],
        vec![
            point(0, 0.5, 0.0, true),
            point(1920, 1.0, 0.0, false),
            point(3840, 0.0, 0.3, true),
            point(5760, 0.75, 0.0, false),
        ],
        // Two points on one tick make a jump.
        vec![
            point(0, 0.1, 0.0, false),
            point(1920, 0.4, 0.0, false),
            point(1920, 0.9, 0.0, false),
            point(3840, 0.6, 1.0, false),
        ],
    ];

    cases
        .iter()
        .map(|points| {
            let last = points.last().map_or(0, |point| point.tick);
            let ticks: Vec<f64> = (0..=32)
                .map(|step| step as f64 * (last as f64 + 960.0) / 32.0)
                .chain(points.iter().map(|point| point.tick as f64))
                .collect();
            let values: Vec<f32> = ticks
                .iter()
                .map(|&tick| curve_value(points, tick))
                .collect();
            json!({ "points": points, "ticks": ticks, "values": values })
        })
        .collect()
}

/// Each range with a spread of normalized positions mapped to real values
/// and back.
fn ranges() -> Vec<Value> {
    let mut ranges: Vec<(String, AutomationRange)> = vec![
        ("gain".into(), AutomationRange::GAIN),
        ("pan".into(), AutomationRange::PAN),
        ("mix".into(), AutomationRange::MIX),
        ("tempo".into(), AutomationRange::TEMPO),
    ];
    // One parameter of every kind and scale the processors have.
    for kind in EffectKind::ALL {
        for info in kind.descriptors() {
            ranges.push((
                format!("{}: {}", kind.name(), info.id),
                AutomationRange::of_param(info),
            ));
        }
    }
    for kind in InstrumentKind::ALL {
        for info in kind.descriptors() {
            ranges.push((
                format!("{}: {}", kind.name(), info.id),
                AutomationRange::of_param(info),
            ));
        }
    }

    let positions: Vec<f32> = (0..=8).map(|step| step as f32 / 8.0).collect();
    ranges
        .into_iter()
        .map(|(name, range)| {
            let values: Vec<f32> = positions.iter().map(|&n| range.value(n)).collect();
            let back: Vec<f32> = values
                .iter()
                .map(|&value| range.normalized(value))
                .collect();
            json!({
                "name": name,
                "range": range,
                "normalized": positions,
                "values": values,
                "normalizedBack": back,
            })
        })
        .collect()
}
