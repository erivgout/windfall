//! Per-binding control-side retry admission; never an audio-thread scheduler.

use super::protocol::Identity;
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

static BUDGETS: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryStatus {
    Ready,
    Active,
    Cooling(Duration),
    Blocked,
    Exhausted,
    InvalidOwner,
}

/// Opaque local admission plus exact helper owner. Never persisted or put in
/// the audio ABI; a new budget cannot accept an old budget's completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attempt {
    budget: u64,
    serial: u64,
    owner: Identity,
}
impl Attempt {
    pub fn owner(self) -> Identity {
        self.owner
    }
}

pub struct RetryBudget {
    namespace: u64,
    binding: u64,
    limit: u8,
    failures: u8,
    cooldown: Duration,
    healthy_interval: Duration,
    retry_at: Option<Instant>,
    active: Option<(Attempt, Instant)>,
    serial: u64,
    exhausted: bool,
}
impl RetryBudget {
    pub fn new(
        binding: u64,
        limit: u8,
        cooldown: Duration,
        healthy_interval: Duration,
    ) -> Option<Self> {
        if !(1..=16).contains(&limit) || cooldown.is_zero() || healthy_interval.is_zero() {
            return None;
        }
        let mut namespace = BUDGETS.load(Ordering::Acquire);
        loop {
            let next = namespace.checked_add(1)?;
            match BUDGETS.compare_exchange(namespace, next, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => break,
                Err(current) => namespace = current,
            }
        }
        Some(Self {
            namespace,
            binding,
            limit,
            failures: 0,
            cooldown,
            healthy_interval,
            retry_at: None,
            active: None,
            serial: 0,
            exhausted: false,
        })
    }
    pub fn failures(&self) -> u8 {
        self.failures
    }
    pub fn status(&self, now: Instant) -> RetryStatus {
        if self.exhausted {
            return RetryStatus::Exhausted;
        }
        if self.active.is_some() {
            return RetryStatus::Active;
        }
        if self.failures >= self.limit {
            return RetryStatus::Blocked;
        }
        if let Some(at) = self.retry_at
            && now < at
        {
            return RetryStatus::Cooling(at.duration_since(now));
        }
        RetryStatus::Ready
    }
    /// Explicit admission only. This module never spawns or retries a process.
    pub fn admit(&mut self, owner: Identity, now: Instant) -> Result<Attempt, RetryStatus> {
        if !owner.valid() || owner.binding != self.binding {
            return Err(RetryStatus::InvalidOwner);
        }
        let status = self.status(now);
        if status != RetryStatus::Ready {
            return Err(status);
        }
        let Some(serial) = self.serial.checked_add(1) else {
            self.exhausted = true;
            return Err(RetryStatus::Exhausted);
        };
        self.serial = serial;
        let attempt = Attempt {
            budget: self.namespace,
            serial,
            owner,
        };
        self.active = Some((attempt, now));
        Ok(attempt)
    }
    /// Counts one failure per admitted owner, including startup failure.
    pub fn failed(&mut self, attempt: Attempt, now: Instant) -> bool {
        let Some((current, started)) = self.active else {
            return false;
        };
        if current != attempt || now < started {
            return false;
        }
        self.active = None;
        self.failures = self.failures.saturating_add(1);
        self.retry_at = now.checked_add(self.cooldown);
        if self.retry_at.is_none() {
            self.exhausted = true;
        }
        true
    }
    /// Startup/one successful block cannot replenish the budget. The caller
    /// must also prove continued healthy native completions over this interval.
    pub fn healthy(&mut self, attempt: Attempt, now: Instant) -> bool {
        let Some((current, started)) = self.active else {
            return false;
        };
        if current != attempt || now.saturating_duration_since(started) < self.healthy_interval {
            return false;
        }
        self.failures = 0;
        self.retry_at = None;
        true
    }
    pub fn retired(&mut self, attempt: Attempt) -> bool {
        if self.active.is_none_or(|(current, _)| current != attempt) {
            return false;
        }
        self.active = None;
        true
    }
    /// Only an explicit user retry/rescan may clear the block. It invalidates
    /// old attempt completions but never wraps/reuses its serial namespace.
    pub fn explicit_reset(&mut self) {
        self.active = None;
        self.failures = 0;
        self.retry_at = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::protocol::Identity;
    use std::time::{Duration, Instant};

    fn owner(session: u64) -> Identity {
        Identity {
            session,
            token: 1,
            revision: 7,
            binding: 99,
        }
    }

    #[test]
    fn repeated_failure_is_finite_and_stale_attempts_cannot_spend_or_clear_budget() {
        let now = Instant::now();
        let mut budget =
            RetryBudget::new(99, 3, Duration::from_secs(2), Duration::from_secs(10)).unwrap();
        let first = budget.admit(owner(1), now).unwrap();
        assert!(budget.failed(first, now));
        assert!(!budget.failed(first, now));
        assert_eq!(
            budget.status(now),
            RetryStatus::Cooling(Duration::from_secs(2))
        );
        assert!(budget.admit(owner(2), now).is_err());
        let second = budget
            .admit(owner(2), now + Duration::from_secs(2))
            .unwrap();
        assert!(!budget.healthy(first, now + Duration::from_secs(60)));
        assert!(!budget.failed(first, now + Duration::from_secs(60)));
        assert!(budget.failed(second, now + Duration::from_secs(2)));
        let third = budget
            .admit(owner(3), now + Duration::from_secs(4))
            .unwrap();
        assert!(budget.failed(third, now + Duration::from_secs(4)));
        assert_eq!(
            budget.status(now + Duration::from_secs(100)),
            RetryStatus::Blocked
        );
        assert!(
            budget
                .admit(owner(4), now + Duration::from_secs(100))
                .is_err()
        );
        budget.explicit_reset();
        assert_eq!(budget.status(now), RetryStatus::Ready);
        let fourth = budget
            .admit(owner(4), now + Duration::from_secs(101))
            .unwrap();
        assert!(!budget.healthy(fourth, now + Duration::from_secs(102)));
        assert!(budget.healthy(fourth, now + Duration::from_secs(111)));
        assert_eq!(budget.failures(), 0);
    }
}
