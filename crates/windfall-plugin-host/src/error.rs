//! What can go wrong between the host and a plugin.

use crate::state::InvalidState;

/// A plugin could not be loaded, created, activated or restored. The text
/// is for the user.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PluginError {
    /// The file could not be loaded as a plugin of its format.
    #[error("the plugin file could not be loaded: {0}")]
    Load(String),
    /// The file holds no plugin with the id asked for.
    #[error("the file has no plugin with the id {0}")]
    NotFound(String),
    /// The plugin refused to be created.
    #[error("the plugin could not be created: {0}")]
    Create(String),
    /// The plugin describes ports the host cannot feed.
    #[error("the plugin's ports cannot be used: {0}")]
    Layout(String),
    /// The plugin refused to be activated.
    #[error("the plugin could not be activated: {0}")]
    Activate(String),
    /// The plugin is active already and has a processor out.
    #[error("the plugin is already active")]
    AlreadyActive,
    /// The plugin could not save or load its state.
    #[error("the plugin's state could not be {0}")]
    State(&'static str),
    /// The saved state is not one this host wrote.
    #[error(transparent)]
    InvalidState(#[from] InvalidState),
    /// The host cannot run plugins of this format in this build.
    #[error("{0} plugins cannot be run by this build")]
    Unsupported(&'static str),
}
