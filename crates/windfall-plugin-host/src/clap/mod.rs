//! Hosting CLAP plugins, on top of clack.
//!
//! Nothing outside this module names a clack or `clap-sys` type. The rest
//! of the crate sees a CLAP plugin through the two backend traits, which is
//! what would let another library, or another process, take clack's place.

mod editor;
mod events;
mod handlers;
mod instance;
mod processor;

use std::ffi::CStr;
use std::path::Path;

use clack_host::prelude::*;

pub(crate) use handlers::silent_log;

use crate::descriptor::{PluginDescriptor, PluginFormat, PluginKind};
use crate::error::PluginError;
use crate::host::LogSink;
use crate::instance::InstanceBackend;

/// A loaded `.clap` file.
#[derive(Clone)]
pub(crate) struct ClapModule {
    entry: PluginEntry,
}

fn text(value: Option<&CStr>) -> String {
    value.map_or_else(String::new, |value| value.to_string_lossy().into_owned())
}

impl ClapModule {
    /// Loads the file and runs its entry point. This runs the plugin's
    /// code, which is why the app only does it for files the scanner has
    /// already been through.
    pub fn load(path: &Path) -> Result<Self, PluginError> {
        // SAFETY: loading a library runs code the host cannot check. That
        // is what hosting a plugin is, and the scanner process has loaded
        // this file before and survived.
        let entry = unsafe { PluginEntry::load(path) }
            .map_err(|error| PluginError::Load(error.to_string()))?;
        Ok(Self { entry })
    }

    /// The plugins the file lists.
    pub fn descriptors(&self) -> Vec<PluginDescriptor> {
        let Some(factory) = self.entry.get_plugin_factory() else {
            return Vec::new();
        };
        factory
            .plugin_descriptors()
            .filter_map(|descriptor| {
                let id = text(descriptor.id());
                if id.is_empty() {
                    return None;
                }
                let features: Vec<String> = descriptor
                    .features()
                    .map(|feature| feature.to_string_lossy().into_owned())
                    .collect();
                Some(PluginDescriptor {
                    format: PluginFormat::Clap,
                    kind: PluginKind::from_features(features.iter().map(String::as_str)),
                    id,
                    name: text(descriptor.name()),
                    vendor: text(descriptor.vendor()),
                    version: text(descriptor.version()),
                    features,
                })
            })
            .collect()
    }

    /// Creates the plugin with the given id.
    pub fn create(
        &self,
        id: &str,
        host: &HostInfo,
        log: LogSink,
    ) -> Result<Box<dyn InstanceBackend>, PluginError> {
        let instance = instance::ClapInstance::create(&self.entry, id, host, log)?;
        Ok(Box::new(instance))
    }
}
