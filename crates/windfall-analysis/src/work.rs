use crate::{AnalysisError, Result, add};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

#[derive(Clone, Default, Debug)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

/// Cooperative control-side work accounting. A checkpoint charges at least
/// one unit; callers must checkpoint between bounded chunks. This is not a
/// process sandbox or a way to preempt an uncooperative native adapter/IO call.
pub struct Work {
    cancel: CancelToken,
    deadline: Instant,
    maximum: u64,
    completed: u64,
    pub(crate) progress: Option<Arc<dyn Fn(u64) -> Result<()> + Send + Sync>>,
}

impl Work {
    pub fn new(cancel: CancelToken, maximum: u64, timeout: Duration) -> Result<Self> {
        if maximum == 0 || maximum > u64::MAX - 32 || timeout.is_zero() {
            return Err(AnalysisError::Invalid("work/deadline bounds"));
        }
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or(AnalysisError::Exhausted)?;
        Ok(Self {
            cancel,
            deadline,
            maximum,
            completed: 0,
            progress: None,
        })
    }
    pub(crate) fn with_deadline(cancel: CancelToken, maximum: u64, deadline: Instant) -> Self {
        Self {
            cancel,
            deadline,
            maximum,
            completed: 0,
            progress: None,
        }
    }
    pub fn check(&self) -> Result<()> {
        if self.cancel.is_cancelled() {
            return Err(AnalysisError::Cancelled);
        }
        if Instant::now() >= self.deadline {
            return Err(AnalysisError::Deadline);
        }
        Ok(())
    }
    pub fn checkpoint(&mut self, units: u64) -> Result<()> {
        self.check()?;
        let next = add(self.completed, units.max(1))?;
        if next > self.maximum {
            return Err(AnalysisError::Budget("work units"));
        }
        self.completed = next;
        if let Some(progress) = &self.progress {
            progress(next)?;
        }
        Ok(())
    }
    pub fn completed(&self) -> u64 {
        self.completed
    }
    pub(crate) fn set_completed(&mut self, completed: u64) {
        self.completed = completed;
    }
}
