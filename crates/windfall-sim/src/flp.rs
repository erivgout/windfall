//! The browser uses the same FL reader and converter as the native shell.
use crate::{Reply, json, parse};
use serde::{Deserialize, Serialize};
use windfall_flp::{ConvertOptions, report::ImportReport};
use windfall_project::Project;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Input {
    bytes: Vec<u8>,
    options: ConvertOptions,
}
#[derive(Serialize)]
struct Output {
    project: Project,
    report: ImportReport,
}

/// Converts original file bytes without changing an open document.
pub fn convert(_handle: u32, input: &str) -> Reply {
    let input: Input = parse("FL import", input)?;
    let conversion =
        windfall_flp::import(&input.bytes, &input.options).map_err(|e| e.to_string())?;
    json(&Output {
        project: conversion.project,
        report: conversion.report,
    })
}
