//! Windows per-instance audio/state crash containment. Native code in helpers
//! has the user's permissions; this is not a security sandbox.

pub mod adapter;
#[cfg(windows)]
pub mod auth;
pub mod control;
#[cfg(windows)]
pub mod helper;
#[cfg(windows)]
pub mod mapping;
pub mod protocol;
pub mod retry;
pub mod slots;
#[cfg(windows)]
pub mod supervisor;
