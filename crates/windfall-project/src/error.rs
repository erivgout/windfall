use thiserror::Error;

/// Why a command could not be applied. The project is unchanged.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum CommandError {
    #[error("{kind} {id} does not exist")]
    NotFound { kind: &'static str, id: u32 },
    #[error("{0}")]
    Invalid(String),
}

impl CommandError {
    pub fn not_found(kind: &'static str, id: impl Into<u32>) -> Self {
        Self::NotFound {
            kind,
            id: id.into(),
        }
    }

    pub fn invalid(reason: impl Into<String>) -> Self {
        Self::Invalid(reason.into())
    }
}
