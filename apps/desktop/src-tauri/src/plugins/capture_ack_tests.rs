//! Private postcommit delivery tests; no native fixture or nested Cargo.
use super::*;

fn captured_request() -> (PendingUpdate, CapturedState) {
    (
        PendingUpdate {
            binding: 11,
            token: 7,
            revision: 3,
            update: Update::Capture {
                target: PluginTarget::Effect {
                    effect: windfall_project::EffectId(9),
                },
                serial: 4,
            },
        },
        CapturedState {
            bytes: vec![1, 2, 3],
            parameters: vec![(0, 0.625)],
            restart: true,
            serial: 5,
        },
    )
}

#[test]
fn accepted_capture_survives_a_stopped_bookkeeping_owner() {
    let mut runtime = Runtime::new().unwrap();
    let (stopped, receiver) = mpsc::channel();
    drop(receiver);
    runtime.jobs = stopped;
    let (request, captured) = captured_request();
    let outcome = crate::plugins::CaptureOutcome::Accepted {
        restart: captured.restart,
        acknowledgement: runtime.acknowledge_committed_capture(&request, &captured),
    };
    assert_eq!(
        outcome,
        crate::plugins::CaptureOutcome::Accepted {
            restart: true,
            acknowledgement: crate::plugins::CaptureAcknowledgement::Pending {
                ticket: crate::plugins::capture_ack::CaptureAckTicket {
                    token: 7,
                    serial: 5,
                },
                warning: "The plugin owner stopped".into(),
            },
        }
    );
    assert_eq!(captured.bytes, [1, 2, 3]);
    assert_eq!(captured.parameters, [(0, 0.625)]);
    assert!(runtime.acknowledge_capture(&request, &captured).is_err());
}

#[test]
fn retired_owner_confirms_a_committed_capture_without_native_loading() {
    let runtime = Runtime::new().unwrap();
    let (request, captured) = captured_request();
    assert_eq!(
        runtime.acknowledge_committed_capture(&request, &captured),
        crate::plugins::CaptureAcknowledgement::Confirmed
    );
    runtime
        .call(|owner| {
            assert!(owner.instances.is_empty());
            assert_eq!(owner.next, 0);
            Ok(())
        })
        .unwrap();
}

#[test]
fn acknowledgement_ticket_is_copy_and_retains_no_native_bytes() {
    use crate::plugins::capture_ack::CaptureAckTicket;
    assert_eq!(std::mem::size_of::<CaptureAckTicket>(), 16);
    assert!(!std::mem::needs_drop::<CaptureAckTicket>());
    let ticket = CaptureAckTicket {
        token: 7,
        serial: 5,
    };
    let copied = ticket;
    assert_eq!(ticket, copied);
}
