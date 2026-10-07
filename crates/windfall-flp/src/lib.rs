//! Reads FL Studio project files.

pub mod error;
pub mod event;
pub mod model;
pub mod names;
pub mod parse;
pub mod plugin;
pub mod target;
mod text;

pub use error::{EventError, FlpError};
pub use event::{Container, Event, EventReader, EventValue, FileFormat, Header, open};
pub use model::*;
pub use parse::parse;
pub use plugin::{HostedPlugin, PluginFormat};
pub use target::{ChannelParam, ControlTarget, InsertParam, MainParam, SlotParam};

pub mod convert;
pub mod paths;
pub mod report;
pub mod units;
pub use convert::{Conversion, ConvertOptions, PluginPlace, PluginPlaceholder, convert, import};
pub use report::{ImportReport, Outcome, ReportSection};
