//! Private capture acceptance and retryable owner bookkeeping, off all guards.
//!
//! Acceptance is independent of acknowledgement delivery. A ticket keeps only
//! the unique native owner and the serial covered by the accepted capture; it
//! never retains opaque state or authorizes parameter/DSP acknowledgement.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CaptureAckTicket {
    pub(super) token: u64,
    pub(super) serial: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CaptureAcknowledgement {
    /// The bookkeeping owner confirmed completion, including a retired token.
    Confirmed,
    /// Delivery is unconfirmed. Its closure may already have taken effect, so
    /// retry this idempotent ticket rather than recapturing accepted state.
    Pending {
        ticket: CaptureAckTicket,
        warning: String,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CaptureOutcome {
    Obsolete,
    Accepted {
        restart: bool,
        acknowledgement: CaptureAcknowledgement,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CaptureAckCompletion {
    Acknowledged,
    Retired,
}

/// One-shot receipt loss after real owner bookkeeping succeeds. Test-only and
/// scoped to a target; it never interrupts capture, construction or native work.
#[cfg(all(test, windows))]
pub(super) struct CaptureAckTestFault {
    key: std::sync::Arc<()>,
    target: windfall_project::PluginTarget,
    armed: bool,
}

#[cfg(all(test, windows))]
pub(crate) struct CaptureAckTestScope {
    faults: std::sync::Arc<std::sync::Mutex<Option<CaptureAckTestFault>>>,
    key: std::sync::Arc<()>,
}

#[cfg(all(test, windows))]
impl CaptureAckTestScope {
    pub(super) fn arm(
        faults: std::sync::Arc<std::sync::Mutex<Option<CaptureAckTestFault>>>,
        target: windfall_project::PluginTarget,
    ) -> Self {
        let key = std::sync::Arc::new(());
        {
            let mut fault = faults.lock().unwrap_or_else(|error| error.into_inner());
            assert!(
                fault.is_none(),
                "capture acknowledgement scopes must not overlap"
            );
            *fault = Some(CaptureAckTestFault {
                key: key.clone(),
                target,
                armed: true,
            });
        }
        Self { faults, key }
    }
}

#[cfg(all(test, windows))]
impl CaptureAckTestFault {
    pub(super) fn consume(&mut self, target: windfall_project::PluginTarget) -> bool {
        if self.armed && self.target == target {
            self.armed = false;
            true
        } else {
            false
        }
    }
}

#[cfg(all(test, windows))]
impl Drop for CaptureAckTestScope {
    fn drop(&mut self) {
        let mut fault = self
            .faults
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if fault
            .as_ref()
            .is_some_and(|fault| std::sync::Arc::ptr_eq(&fault.key, &self.key))
        {
            *fault = None;
        }
    }
}

struct PendingAcknowledgement {
    ticket: CaptureAckTicket,
    warning: String,
}

/// Manager-owned intent. Capture refusal and postcommit delivery uncertainty
/// have separate queues, so retrying acknowledgement cannot replay native bytes.
#[derive(Default)]
pub(super) struct CaptureUpdates {
    captures: Vec<super::PendingUpdate>,
    acknowledgements: Vec<PendingAcknowledgement>,
}

#[derive(Default)]
pub(super) struct CaptureProgress {
    pub(super) restart: bool,
    pub(super) error: Option<String>,
}

impl CaptureUpdates {
    pub(super) fn push(&mut self, request: super::PendingUpdate) {
        self.captures.retain(|before| before.token != request.token);
        self.captures.push(request);
    }

    pub(super) fn capture(
        &mut self,
        mut capture: impl FnMut(super::PendingUpdate) -> Result<CaptureOutcome, String>,
    ) -> CaptureProgress {
        let mut progress = CaptureProgress::default();
        let Self {
            captures,
            acknowledgements,
        } = self;
        captures.retain(|request| match capture(request.clone()) {
            Ok(CaptureOutcome::Obsolete) => false,
            Ok(CaptureOutcome::Accepted {
                restart,
                acknowledgement,
            }) => {
                progress.restart |= restart;
                if let CaptureAcknowledgement::Pending { ticket, warning } = acknowledgement {
                    // A later accepted serial subsumes the older ticket for this
                    // unique owner. Keep at most one compact ticket per owner.
                    if let Some(before) = acknowledgements
                        .iter_mut()
                        .find(|before| before.ticket.token == ticket.token)
                    {
                        if ticket.serial >= before.ticket.serial {
                            *before = PendingAcknowledgement { ticket, warning };
                        }
                    } else {
                        acknowledgements.push(PendingAcknowledgement { ticket, warning });
                    }
                }
                // Acceptance is final even when owner bookkeeping is uncertain.
                // The request leaves this queue and can never be recaptured here.
                false
            }
            Err(error) => {
                progress.error = Some(error);
                true
            }
        });
        progress
    }

    pub(super) fn retry_ack(
        &mut self,
        mut deliver: impl FnMut(CaptureAckTicket) -> Result<CaptureAckCompletion, String>,
    ) {
        self.acknowledgements
            .retain_mut(|pending| match deliver(pending.ticket) {
                Ok(CaptureAckCompletion::Acknowledged | CaptureAckCompletion::Retired) => false,
                Err(warning) => {
                    pending.warning = warning;
                    true
                }
            });
    }

    pub(super) fn has_pending_ack(&self) -> bool {
        !self.acknowledgements.is_empty()
    }

    pub(super) fn warning(&self) -> Option<&str> {
        self.acknowledgements
            .first()
            .map(|pending| pending.warning.as_str())
    }

    pub(super) fn is_empty(&self) -> bool {
        self.captures.is_empty() && self.acknowledgements.is_empty()
    }
}

#[cfg(test)]
#[path = "capture_ack_manager_tests.rs"]
mod manager_tests;
