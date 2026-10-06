//! Locking that survives a panic elsewhere.

use std::sync::{Mutex, MutexGuard};

/// Locks a mutex whether or not a thread panicked while holding it. Every
/// value the shell guards is left usable between statements, so one failed
/// call must not turn every later call into a failure too.
pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
