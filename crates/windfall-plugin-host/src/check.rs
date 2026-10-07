//! Trying a plugin out in a process of its own: load it, create it, run
//! audio through it, read its parameters, save and restore its state, and
//! shut it down, reporting how far it got.
//!
//! The scanner only asks a plugin what it is. This runs it. The
//! `windfall-plugin-check` program does the work with [`check_in_process`],
//! and [`check_plugin`] starts that program and reads its report, so a
//! plugin that crashes on the way is a line in a table and not a dead app.

use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::host::PluginHost;
use crate::scan::{RunEnd, ScannerUnavailable, exit_words, run_reporting};

/// One stage of a check, in the order they are run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckStep {
    Load,
    Create,
    Activate,
    Process,
    Params,
    State,
    Deactivate,
}

impl CheckStep {
    pub const ALL: [CheckStep; 7] = [
        CheckStep::Load,
        CheckStep::Create,
        CheckStep::Activate,
        CheckStep::Process,
        CheckStep::Params,
        CheckStep::State,
        CheckStep::Deactivate,
    ];
}

/// How one stage went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepOutcome {
    /// It worked. The text says what was seen.
    Passed(String),
    /// The plugin, or the host, reported a problem.
    Failed(String),
    /// The process died during this stage.
    Crashed(String),
    /// The process stopped answering during this stage.
    TimedOut,
    /// An earlier stage ended the check.
    NotReached,
}

impl StepOutcome {
    pub fn passed(&self) -> bool {
        matches!(self, Self::Passed(_))
    }
}

/// How every stage of a check went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckReport {
    pub steps: Vec<(CheckStep, StepOutcome)>,
}

impl CheckReport {
    pub fn outcome(&self, step: CheckStep) -> &StepOutcome {
        self.steps
            .iter()
            .find(|(candidate, _)| *candidate == step)
            .map_or(&StepOutcome::NotReached, |(_, outcome)| outcome)
    }

    pub fn all_passed(&self) -> bool {
        CheckStep::ALL
            .iter()
            .all(|step| self.outcome(*step).passed())
    }
}

/// One line of the check program's output.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum CheckLine {
    Starting {
        step: CheckStep,
    },
    Finished {
        step: CheckStep,
        ok: bool,
        detail: String,
    },
    Done,
}

/// The audio settings a plugin is checked at.
const SAMPLE_RATE: f64 = 48_000.0;
const BLOCK: usize = 512;
/// Blocks of audio run through the plugin: about two thirds of a second.
const BLOCKS: usize = 64;

/// Checks a plugin in this process, printing one line per stage to `out`.
/// A plugin that crashes takes the process with it, which is why only the
/// `windfall-plugin-check` program calls this.
pub fn check_in_process(path: &Path, id: &str, out: &mut dyn Write) {
    let mut emit = |line: CheckLine| {
        let json = serde_json::to_string(&line).expect("check lines are plain data");
        let _ = writeln!(out, "\n{json}");
        let _ = out.flush();
    };
    // Runs one stage. `Err` ends the check.
    macro_rules! stage {
        ($step:expr, $work:expr) => {{
            emit(CheckLine::Starting { step: $step });
            let result: Result<_, String> = $work;
            match result {
                Ok((value, detail)) => {
                    emit(CheckLine::Finished {
                        step: $step,
                        ok: true,
                        detail,
                    });
                    value
                }
                Err(detail) => {
                    emit(CheckLine::Finished {
                        step: $step,
                        ok: false,
                        detail,
                    });
                    emit(CheckLine::Done);
                    return;
                }
            }
        }};
    }

    let host = PluginHost::windfall();
    let module = stage!(CheckStep::Load, {
        host.load(path)
            .map(|module| {
                let count = module.descriptors().len();
                (module, format!("{count} plugins in the file"))
            })
            .map_err(|error| error.to_string())
    });
    let mut instance = stage!(CheckStep::Create, {
        module
            .create(id)
            .map(|instance| {
                let layout = instance.layout();
                let detail = format!(
                    "{} audio inputs, {} audio outputs, {} note inputs",
                    layout.audio_inputs.len(),
                    layout.audio_outputs.len(),
                    layout.note_inputs
                );
                (instance, detail)
            })
            .map_err(|error| error.to_string())
    });
    let mut processor = stage!(CheckStep::Activate, {
        instance
            .activate(SAMPLE_RATE, BLOCK)
            .map(|processor| {
                let detail = format!("latency {} samples", processor.latency_samples());
                (processor, detail)
            })
            .map_err(|error| error.to_string())
    });

    stage!(CheckStep::Process, {
        let (mut left, mut right) = (vec![0.0_f32; BLOCK], vec![0.0_f32; BLOCK]);
        let plays_notes = instance.layout().note_inputs > 0;
        let mut peak = 0.0_f32;
        for block in 0..BLOCKS {
            // Exercise an effect with a signal, not just a silence path.
            for (frame, (left, right)) in left.iter_mut().zip(&mut right).enumerate() {
                let value = if plays_notes {
                    0.0
                } else {
                    ((block * BLOCK + frame) as f32 * std::f32::consts::TAU * 440.0
                        / SAMPLE_RATE as f32)
                        .sin()
                        * 0.1
                };
                *left = value;
                *right = value * 0.75;
            }
            if plays_notes && block == 2 {
                processor.note_on(0, 60, 0.8);
            }
            if plays_notes && block == BLOCKS / 2 {
                processor.note_off(0, 60);
            }
            processor.process(&mut left, &mut right);
            for sample in left.iter().chain(&right) {
                peak = peak.max(sample.abs());
            }
        }
        let health = processor.health();
        if health.failed {
            Err("the plugin reported an error while processing".to_owned())
        } else if health.scrubbed_samples > 0 {
            Err(format!(
                "{} output samples were not numbers",
                health.scrubbed_samples
            ))
        } else {
            let tail = processor
                .tail_samples()
                .map_or_else(|| "endless".to_owned(), |tail| tail.to_string());
            Ok((
                (),
                format!("{BLOCKS} blocks, output peak {peak:.4}, tail {tail}"),
            ))
        }
    });

    stage!(CheckStep::Params, {
        let ids: Vec<u32> = instance.params().iter().map(|param| param.id).collect();
        let mut readable = 0;
        let mut texts = 0;
        for &id in &ids {
            if let Some(value) = instance.param_value(id) {
                readable += 1;
                if instance.param_text(id, value).is_some() {
                    texts += 1;
                }
            }
        }
        if readable == ids.len() {
            Ok(((), format!("{} parameters, {texts} with text", ids.len())))
        } else {
            Err(format!(
                "{} of {} parameters could not be read",
                ids.len() - readable,
                ids.len()
            ))
        }
    });

    let mut processor = stage!(CheckStep::State, {
        // VST3 component state is a main-thread operation that must not
        // race process. Return ownership before saving/restoring either format.
        processor.stop();
        instance.deactivate(processor);
        instance
            .save_state()
            .and_then(|saved| {
                instance.load_state(&saved)?;
                let again = instance.save_state()?;
                let same = if again == saved {
                    "the same bytes after a restore"
                } else {
                    "other bytes after a restore"
                };
                let kind = if instance.layout().has_state {
                    "plugin state"
                } else {
                    "parameter values"
                };
                Ok((
                    instance.activate(SAMPLE_RATE, BLOCK)?,
                    format!("{} bytes of {kind}, {same}", saved.as_bytes().len()),
                ))
            })
            .map_err(|error| error.to_string())
    });

    stage!(CheckStep::Deactivate, {
        processor.stop();
        instance.deactivate(processor);
        let inactive = !instance.is_active();
        drop(instance);
        if inactive {
            Ok(((), "deactivated and destroyed".to_owned()))
        } else {
            Err("the plugin stayed active".to_owned())
        }
    });
    emit(CheckLine::Done);
}

/// Checks a plugin by running the check program at `program` on it. Each
/// stage gets `timeout` to finish.
pub fn check_plugin(
    program: &Path,
    path: &Path,
    id: &str,
    timeout: Duration,
) -> Result<CheckReport, ScannerUnavailable> {
    let mut command = Command::new(program);
    command.arg(path).arg(id);
    let mut steps: Vec<(CheckStep, StepOutcome)> = Vec::new();
    let mut running = None;
    let mut done = false;
    let end = run_reporting(command, timeout, &mut |line| {
        let line = line.trim();
        if !line.starts_with('{') {
            return None;
        }
        match serde_json::from_str::<CheckLine>(line) {
            Ok(CheckLine::Starting { step }) => running = Some(step),
            Ok(CheckLine::Finished { step, ok, detail }) => {
                running = None;
                let outcome = if ok {
                    StepOutcome::Passed(detail)
                } else {
                    StepOutcome::Failed(detail)
                };
                steps.push((step, outcome));
            }
            Ok(CheckLine::Done) => done = true,
            Err(_) => return None,
        }
        Some(done)
    })?;

    if !done {
        // The process ended without saying so: the stage it was in is the
        // one to blame, or the one after the last it finished.
        let step = running.unwrap_or_else(|| {
            CheckStep::ALL
                .into_iter()
                .find(|step| !steps.iter().any(|(finished, _)| finished == step))
                .unwrap_or(CheckStep::Deactivate)
        });
        let outcome = match end {
            RunEnd::TimedOut => StepOutcome::TimedOut,
            RunEnd::Exited(code) => StepOutcome::Crashed(exit_words(code)),
        };
        steps.push((step, outcome));
    }
    for step in CheckStep::ALL {
        if !steps.iter().any(|(reached, _)| *reached == step) {
            steps.push((step, StepOutcome::NotReached));
        }
    }
    Ok(CheckReport { steps })
}
