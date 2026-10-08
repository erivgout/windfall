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
