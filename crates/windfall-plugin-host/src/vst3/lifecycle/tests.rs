use super::*;
use std::cell::RefCell;
use std::rc::Rc;

fn source(index: u64) -> Source {
    Source {
        bundle: format!("/{index}.vst3").into(),
        binary: format!("/{index}.vst3/bin").into(),
        directory_id: (1, index),
        file_id: (2, index),
        size: 20,
        modified: (1, 0),
        changed: (1, 0),
        plist: vec![index as u8],
    }
}
struct Action {
    events: Rc<RefCell<Vec<&'static str>>>,
    callback: Option<Box<dyn FnOnce()>>,
    accepted: bool,
}
impl Cleanup for Action {
    fn cleanup(&mut self) -> bool {
        self.events.borrow_mut().push("factory");
        if let Some(callback) = self.callback.take() {
            callback();
        }
        self.events.borrow_mut().extend(["exit", "bundle"]);
        self.accepted
    }
}
fn node(ticket: Ticket, events: Rc<RefCell<Vec<&'static str>>>) -> Box<Retirement> {
    Retirement::new(
        ticket,
        Box::new(Action {
            events,
            callback: None,
            accepted: true,
        }),
    )
}
fn live(
    registry: &Registry,
    index: u64,
    events: Rc<RefCell<Vec<&'static str>>>,
) -> Box<Retirement> {
    let ticket = registry.admit(source(index)).unwrap();
    let root = ticket.record();
    scope(Some(root), || node(ticket, events))
}

#[test]
fn entry_callback_drop_defers_until_native_return_and_balances_once() {
    let registry = Registry::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let old = live(&registry, 1, events.clone());
    let new = registry.admit(source(1)).unwrap();
    let root = new.record();
    let new = scope(Some(root), || {
        retire(old);
        assert!(events.borrow().is_empty());
        assert!(matches!(registry.admit(source(1)), Err(Refusal::Busy)));
        assert!(matches!(registry.admit(source(2)), Err(Refusal::Busy)));
        node(new, events.clone())
    });
    assert_eq!(*events.borrow(), ["factory", "exit", "bundle"]);
    retire(new);
    assert_eq!(
        *events.borrow(),
        ["factory", "exit", "bundle", "factory", "exit", "bundle"]
    );
    assert_eq!(registry.total.load(Ordering::Acquire), 0);
}

#[test]
fn cleanup_reentrancy_drains_existing_nodes_without_recursive_exit() {
    let registry = Rc::new(Registry::new());
    let events = Rc::new(RefCell::new(Vec::new()));
    let old = live(&registry, 1, events.clone());
    let other = live(&registry, 2, events.clone());
    let mut old = old;
    let check = registry.clone();
    let observed = events.clone();
    old.action = Box::new(Action {
        events: events.clone(),
        accepted: true,
        callback: Some(Box::new(move || {
            retire(other);
            assert_eq!(*observed.borrow(), ["factory"]);
            assert!(matches!(check.admit(source(3)), Err(Refusal::Busy)));
        })),
    });
    retire(old);
    assert_eq!(
        *events.borrow(),
        ["factory", "exit", "bundle", "factory", "exit", "bundle"]
    );
}

#[test]
fn cross_owner_and_joined_worker_loads_refuse_without_waiting() {
    let registry = Arc::new(Registry::new());
    let ticket = registry.admit(source(1)).unwrap();
    scope(Some(ticket.record()), || {
        let worker = registry.clone();
        std::thread::spawn(move || assert!(matches!(worker.admit(source(1)), Err(Refusal::Busy))))
            .join()
            .unwrap();
    });
    let worker = registry.clone();
    std::thread::spawn(move || assert!(matches!(worker.admit(source(1)), Err(Refusal::Busy))))
        .join()
        .unwrap();
    drop(ticket);
    let worker = registry.clone();
    std::thread::spawn(move || {
        drop(worker.admit(source(1)).unwrap());
    })
    .join()
    .unwrap();
}

#[test]
fn cross_key_callbacks_have_no_logical_wait_cycle() {
    let registry = Arc::new(Registry::new());
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let mut workers = Vec::new();
    for index in [1, 2] {
        let registry = registry.clone();
        let barrier = barrier.clone();
        workers.push(std::thread::spawn(move || {
            // Retry only host metadata contention before either native scope.
            let ticket = loop {
                match registry.admit(source(index)) {
                    Ok(ticket) => break ticket,
                    Err(Refusal::Busy) => std::thread::yield_now(),
                    Err(error) => panic!("unexpected {error:?}"),
                }
            };
            scope(Some(ticket.record()), || {
                barrier.wait();
                assert!(matches!(
                    registry.admit(source(3 - index)),
                    Err(Refusal::Busy)
                ));
            });
            drop(ticket);
        }));
    }
    for worker in workers {
        worker.join().unwrap();
    }
}

#[test]
fn changed_retired_source_context_and_restoration_never_readmit() {
    for change in [0, 1, 2] {
        let registry = Registry::new();
        drop(registry.admit(source(1)).unwrap());
        let mut replacement = source(1);
        match change {
            0 => {
                replacement.file_id = (2, 99);
                replacement.size += 1;
            }
            1 => {
                replacement.plist.push(9);
                replacement.binary = "/new-binary".into();
            }
            _ => {
                replacement.bundle = "/different-context.vst3".into();
                replacement.directory_id = (1, 99);
            }
        }
        assert!(matches!(
            registry.admit(replacement),
            Err(Refusal::StaleSource)
        ));
        assert!(matches!(
            registry.admit(source(1)),
            Err(Refusal::StaleSource)
        ));
    }
}

#[test]
fn capacity_and_refused_history_do_not_consume_existing_cleanup_storage() {
    let registry = Registry::bounded(1, 2);
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut a = live(&registry, 1, events.clone());
    let b = live(&registry, 1, events.clone());
    assert!(matches!(registry.admit(source(1)), Err(Refusal::Capacity)));
    assert!(matches!(registry.admit(source(2)), Err(Refusal::Capacity)));
    a.action = Box::new(Action {
        events: events.clone(),
        callback: None,
        accepted: false,
    });
    retire(a);
    retire(b);
    assert_eq!(events.borrow().len(), 6);
    assert!(matches!(
        registry.admit(source(1)),
        Err(Refusal::ExitRefused)
    ));
    assert!(matches!(registry.admit(source(2)), Err(Refusal::Capacity)));
}

#[test]
fn host_unwind_and_poison_cannot_abandon_owned_cleanup() {
    let registry = Registry::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let ticket = registry.admit(source(1)).unwrap();
    let root = ticket.record();
    let queued = node(ticket, events.clone());
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        scope(Some(root), || {
            retire(queued);
            panic!("host after factory acquisition");
        });
    }));
    assert!(result.is_err());
    assert_eq!(*events.borrow(), ["factory", "exit", "bundle"]);
    assert!(matches!(registry.admit(source(1)), Err(Refusal::Poisoned)));

    let registry = Registry::new();
    let held = live(&registry, 1, events.clone());
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _metadata = registry.history.lock().unwrap();
        panic!("host metadata failure");
    }));
    assert!(matches!(registry.admit(source(1)), Err(Refusal::Poisoned)));
    retire(held);
    assert_eq!(registry.total.load(Ordering::Acquire), 0);
    assert_eq!(events.borrow().len(), 6);
}

#[test]
fn preflight_and_source_byte_limits_refuse_before_native_preparation() {
    let registry = Registry::new();
    let permits: Vec<_> = (0..MAX_PREFLIGHT)
        .map(|_| registry.preflight().unwrap())
        .collect();
    assert!(matches!(registry.preflight(), Err(Refusal::Capacity)));
    drop(permits);
    assert!(registry.preflight().is_ok());
    let mut huge = source(1);
    huge.plist.resize(MAX_PLIST + 1, 0);
    assert!(matches!(registry.admit(huge), Err(Refusal::Capacity)));
    assert_eq!(registry.history.lock().unwrap().known.len(), 0);
}

#[test]
fn known_record_retains_native_generation_and_terminal_failure() {
    let registry = Registry::new();
    drop(registry.admit(source(1)).unwrap());
    let record = registry
        .known_bundle(std::path::Path::new("/1.vst3"))
        .unwrap()
        .unwrap();
    assert!(!record.has_loaded());
    record.mark_loaded();
    assert!(record.has_loaded());
    assert_eq!(record.refusal(), None);
    record.stale();
    assert_eq!(record.refusal(), Some(Refusal::StaleSource));
    let other = Registry::new();
    drop(other.admit(source(2)).unwrap());
    let record = other
        .known_bundle(std::path::Path::new("/2.vst3"))
        .unwrap()
        .unwrap();
    record.refuse_preparation();
    assert_eq!(record.refusal(), Some(Refusal::NativeRefused));
    assert!(matches!(
        other.admit(source(2)),
        Err(Refusal::NativeRefused)
    ));
    assert!(matches!(
        registry.admit(source(1)),
        Err(Refusal::StaleSource)
    ));
}

#[test]
fn observed_alias_retarget_is_refused_even_if_new_target_is_unrelated() {
    let registry = Registry::new();
    drop(registry.admit_at(source(1), "/alias.vst3".into()).unwrap());
    assert!(
        registry
            .known_bundle(std::path::Path::new("/alias.vst3"))
            .unwrap()
            .is_some()
    );
    assert!(matches!(
        registry.admit_at(source(2), "/alias.vst3".into()),
        Err(Refusal::StaleSource)
    ));
    assert!(matches!(
        registry.admit(source(1)),
        Err(Refusal::StaleSource)
    ));
}

#[test]
fn scoped_factory_borrow_unwind_poison_preserves_existing_retirement() {
    let registry = Registry::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let held = live(&registry, 1, events.clone());
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        held.borrow_scope(|| panic!("host factory query"));
    }));
    assert!(result.is_err());
    assert!(matches!(registry.admit(source(1)), Err(Refusal::Poisoned)));
    assert!(events.borrow().is_empty());
    retire(held);
    assert_eq!(*events.borrow(), ["factory", "exit", "bundle"]);
}

#[test]
fn full_reserved_ticket_set_drains_without_extra_capacity() {
    let registry = Registry::new();
    let events = Rc::new(RefCell::new(Vec::with_capacity(MAX_TICKETS * 3)));
    let mut held = Vec::new();
    for index in 1..=16 {
        for _ in 0..MAX_PER_IMAGE {
            held.push(live(&registry, index, events.clone()));
        }
        assert!(matches!(
            registry.admit(source(index)),
            Err(Refusal::Capacity)
        ));
    }
    assert!(matches!(registry.admit(source(17)), Err(Refusal::Capacity)));
    scope(None, || {
        for node in held {
            retire(node);
        }
        assert!(events.borrow().is_empty());
    });
    assert_eq!(events.borrow().len(), MAX_TICKETS * 3);
    assert_eq!(registry.total.load(Ordering::Acquire), 0);
}

#[test]
fn established_record_and_byte_budgets_never_evict_retired_generations() {
    let registry = Registry::new();
    for index in 1..=MAX_RECORDS {
        drop(registry.admit(source(index as u64)).unwrap());
    }
    assert!(matches!(
        registry.admit(source(999)),
        Err(Refusal::Capacity)
    ));
    let mut changed = source(1);
    changed.size += 1;
    assert!(matches!(registry.admit(changed), Err(Refusal::StaleSource)));
    let registry = Registry::new();
    let mut accepted = 0;
    for index in 1..=MAX_RECORDS {
        let mut full = source(index as u64);
        full.plist = vec![1; MAX_PLIST];
        match registry.admit(full) {
            Ok(ticket) => {
                accepted += 1;
                drop(ticket);
            }
            Err(Refusal::Capacity) => break,
            Err(error) => panic!("unexpected {error:?}"),
        }
    }
    assert_eq!(accepted, MAX_BYTES / MAX_PLIST - 1);
    assert!(registry.history.lock().unwrap().bytes <= MAX_BYTES);
}
