//! Bounded macOS lifecycle admission. Native ownership never enters the registry.
//!
//! Also compiled in unit tests on other platforms: the exact admission and
//! deferred-retirement implementation is exercised without pretending to load CF.
use std::cell::Cell;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, TryLockError};
use std::thread::ThreadId;

pub(super) const MAX_RECORDS: usize = 256;
pub(super) const MAX_BYTES: usize = 8 * 1024 * 1024;
pub(super) const MAX_TICKETS: usize = 512;
pub(super) const MAX_PER_IMAGE: usize = 32;
pub(super) const MAX_PREFLIGHT: usize = 16;
pub(super) const MAX_PLIST: usize = 256 * 1024;
pub(super) const MAX_PATH: usize = 16 * 1024;
const MAX_ALIASES: usize = 16;
const STALE: u8 = 1;
const REFUSED: u8 = 2;
const POISONED: u8 = 4;
const NATIVE_REFUSED: u8 = 8;
fn acquire(counter: &AtomicUsize, limit: usize) -> Result<(), Refusal> {
    let mut count = counter.load(Ordering::Acquire);
    loop {
        if count >= limit {
            return Err(Refusal::Capacity);
        }
        match counter.compare_exchange_weak(count, count + 1, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return Ok(()),
            Err(current) => count = current,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Source {
    pub bundle: PathBuf,
    pub binary: PathBuf,
    pub directory_id: (u64, u64),
    pub file_id: (u64, u64),
    pub size: u64,
    pub modified: (i64, i64),
    pub changed: (i64, i64),
    pub plist: Vec<u8>,
}
impl Source {
    fn charge(&self) -> Option<usize> {
        if self.bundle.as_os_str().as_encoded_bytes().len() > MAX_PATH
            || self.binary.as_os_str().as_encoded_bytes().len() > MAX_PATH
            || self.plist.len() > MAX_PLIST
        {
            return None;
        }
        self.bundle
            .capacity()
            .checked_add(self.binary.capacity())?
            .checked_add(self.plist.capacity())
    }
    fn intersects(&self, other: &Self) -> bool {
        self.bundle == other.bundle
            || self.binary == other.binary
            || self.directory_id == other.directory_id
            || self.file_id == other.file_id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Refusal {
    Busy,
    Capacity,
    StaleSource,
    Poisoned,
    ExitRefused,
    NativeRefused,
}

pub(super) struct Record {
    busy: AtomicBool,
    tickets: AtomicUsize,
    status: AtomicU8,
    loaded: AtomicBool,
    disabled: Arc<AtomicBool>,
}
impl Record {
    pub fn has_loaded(&self) -> bool {
        self.loaded.load(Ordering::Acquire)
    }
    pub fn mark_loaded(&self) {
        self.loaded.store(true, Ordering::Release);
    }
    pub fn refusal(&self) -> Option<Refusal> {
        let status = self.status.load(Ordering::Acquire);
        if status & POISONED != 0 {
            Some(Refusal::Poisoned)
        } else if status & STALE != 0 {
            Some(Refusal::StaleSource)
        } else if status & REFUSED != 0 {
            Some(Refusal::ExitRefused)
        } else if status & NATIVE_REFUSED != 0 {
            Some(Refusal::NativeRefused)
        } else {
            None
        }
    }
    pub fn refuse_preparation(&self) {
        self.status.fetch_or(NATIVE_REFUSED, Ordering::Release);
    }
    pub fn stale(&self) {
        self.status.fetch_or(STALE, Ordering::Release);
    }
    fn poison(&self) {
        self.disabled.store(true, Ordering::Release);
        self.status.fetch_or(POISONED, Ordering::Release);
    }
}
struct Known {
    source: Source,
    record: Arc<Record>,
    owner: ThreadId,
    aliases: [Option<PathBuf>; MAX_ALIASES],
}
struct History {
    known: Vec<Known>,
    bytes: usize,
}
pub(super) struct Registry {
    history: Mutex<History>,
    disabled: Arc<AtomicBool>,
    total: Arc<AtomicUsize>,
    preflight: AtomicUsize,
    records_limit: usize,
    tickets_limit: usize,
}
impl Registry {
    pub fn new() -> Self {
        Self::bounded(MAX_RECORDS, MAX_TICKETS)
    }
    fn bounded(records_limit: usize, tickets_limit: usize) -> Self {
        let mut known = Vec::new();
        let records_limit = if known.try_reserve_exact(records_limit).is_ok() {
            records_limit
        } else {
            0
        };
        Self {
            history: Mutex::new(History { known, bytes: 0 }),
            disabled: Arc::new(AtomicBool::new(false)),
            total: Arc::new(AtomicUsize::new(0)),
            preflight: AtomicUsize::new(0),
            records_limit,
            tickets_limit,
        }
    }
    pub fn preflight(&self) -> Result<Preflight<'_>, Refusal> {
        if in_native_scope() {
            return Err(Refusal::Busy);
        }
        if self.disabled.load(Ordering::Acquire) {
            return Err(Refusal::Poisoned);
        }
        acquire(&self.preflight, MAX_PREFLIGHT)?;
        Ok(Preflight(self))
    }
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, History>, Refusal> {
        if self.disabled.load(Ordering::Acquire) {
            return Err(Refusal::Poisoned);
        }
        match self.history.try_lock() {
            Ok(guard) => Ok(guard),
            Err(TryLockError::WouldBlock) => Err(Refusal::Busy),
            Err(TryLockError::Poisoned(_)) => {
                self.disabled.store(true, Ordering::Release);
                Err(Refusal::Poisoned)
            }
        }
    }
    pub fn known_bundle(&self, path: &std::path::Path) -> Result<Option<Arc<Record>>, Refusal> {
        Ok(self
            .lock()?
            .known
            .iter()
            .find(|k| {
                k.source.bundle == path || k.aliases.iter().flatten().any(|alias| alias == path)
            })
            .map(|k| k.record.clone()))
    }
    /// The source record is established before the caller may create a bundle.
    /// History is never evicted, including after a failed native preparation.
    #[cfg(test)]
    pub fn admit(&self, source: Source) -> Result<Ticket, Refusal> {
        let requested = source.bundle.clone();
        self.admit_at(source, requested)
    }
    pub fn admit_at(&self, source: Source, requested: PathBuf) -> Result<Ticket, Refusal> {
        if in_native_scope() {
            return Err(Refusal::Busy);
        }
        let charge = source.charge().ok_or(Refusal::Capacity)?;
        if requested.as_os_str().as_encoded_bytes().len() > MAX_PATH {
            return Err(Refusal::Capacity);
        }
        let owner = std::thread::current().id();
        let candidate = Arc::new(Record {
            busy: AtomicBool::new(false),
            tickets: AtomicUsize::new(0),
            status: AtomicU8::new(0),
            loaded: AtomicBool::new(false),
            disabled: self.disabled.clone(),
        });
        let mut history = self.lock()?;
        let index = history.known.iter().position(|k| {
            k.source.intersects(&source)
                || k.aliases.iter().flatten().any(|alias| alias == &requested)
        });
        let needs_alias = requested != source.bundle
            && index.is_none_or(|index| {
                !history.known[index]
                    .aliases
                    .iter()
                    .flatten()
                    .any(|alias| alias == &requested)
            });
        let alias_charge = if needs_alias { requested.capacity() } else { 0 };
        if alias_charge > MAX_BYTES.saturating_sub(history.bytes) {
            return Err(Refusal::Capacity);
        }
        let record = if let Some(index) = index {
            let known = &mut history.known[index];
            if known.source != source {
                known.record.stale();
                return Err(Refusal::StaleSource);
            }
            let status = known.record.status.load(Ordering::Acquire);
            if status != 0 {
                return Err(if status & POISONED != 0 {
                    Refusal::Poisoned
                } else if status & STALE != 0 {
                    Refusal::StaleSource
                } else if status & REFUSED != 0 {
                    Refusal::ExitRefused
                } else {
                    Refusal::NativeRefused
                });
            }
            let count = known.record.tickets.load(Ordering::Acquire);
            if known.record.busy.load(Ordering::Acquire) || (count != 0 && known.owner != owner) {
                return Err(Refusal::Busy);
            }
            if count >= MAX_PER_IMAGE {
                return Err(Refusal::Capacity);
            }
            if needs_alias {
                let Some(slot) = known.aliases.iter_mut().find(|alias| alias.is_none()) else {
                    return Err(Refusal::Capacity);
                };
                *slot = Some(requested);
            }
            if count == 0 {
                known.owner = owner;
            }
            known.record.clone()
        } else {
            if history.known.len() >= self.records_limit
                || charge.saturating_add(alias_charge) > MAX_BYTES.saturating_sub(history.bytes)
            {
                return Err(Refusal::Capacity);
            }
            // The vector and candidate were allocated before this transaction.
            let record = candidate;
            history.bytes += charge;
            let mut aliases = [const { None }; MAX_ALIASES];
            if needs_alias {
                aliases[0] = Some(requested);
            }
            history.known.push(Known {
                source,
                record: record.clone(),
                owner,
                aliases,
            });
            record
        };
        history.bytes += alias_charge;
        acquire(&self.total, self.tickets_limit)?;
        record.tickets.fetch_add(1, Ordering::AcqRel);
        record.busy.store(true, Ordering::Release);
        Ok(Ticket {
            record,
            total: self.total.clone(),
            owner,
        })
    }
}
pub(super) struct Preflight<'a>(&'a Registry);
impl Drop for Preflight<'_> {
    fn drop(&mut self) {
        self.0.preflight.fetch_sub(1, Ordering::AcqRel);
    }
}
pub(super) struct Ticket {
    record: Arc<Record>,
    total: Arc<AtomicUsize>,
    owner: ThreadId,
}
impl Ticket {
    pub fn record(&self) -> Arc<Record> {
        self.record.clone()
    }
}
impl Drop for Ticket {
    fn drop(&mut self) {
        self.record.tickets.fetch_sub(1, Ordering::AcqRel);
        self.total.fetch_sub(1, Ordering::AcqRel);
        if !in_native_scope() {
            if std::thread::panicking() {
                self.record.poison();
            }
            // Cancellation before a native scope was installed owns no native
            // state. The reservation must not survive that host-side unwind.
            self.record.busy.store(false, Ordering::Release);
        }
    }
}

/// Implementations take/clear each owned stage before calling foreign code.
/// They must not panic; false marks persistent exit refusal after cleanup.
pub(super) trait Cleanup {
    fn cleanup(&mut self) -> bool;
}
pub(super) struct Retirement {
    action: Box<dyn Cleanup>,
    ticket: Ticket,
    next: Option<Box<Retirement>>,
}
impl Retirement {
    /// Allocate the complete retirement node before native creation/loading.
    pub fn new(ticket: Ticket, action: Box<dyn Cleanup>) -> Box<Self> {
        Box::new(Self {
            action,
            ticket,
            next: None,
        })
    }
    pub fn borrow_scope<R>(&self, operation: impl FnOnce() -> R) -> R {
        if self.ticket.owner != std::thread::current().id() {
            self.ticket.record.poison();
            std::process::abort();
        }
        if in_native_scope() {
            return operation();
        }
        if self.ticket.record.busy.swap(true, Ordering::AcqRel) {
            std::process::abort();
        }
        scope(Some(self.ticket.record.clone()), operation)
    }
}

thread_local! {
    // Borrowed pointer valid only during scope(). No TLS-owned native handle.
    static ACTIVE: Cell<*const Frame> = const { Cell::new(std::ptr::null()) };
}
fn in_native_scope() -> bool {
    match ACTIVE.try_with(|p| !p.get().is_null()) {
        Ok(active) => active,
        Err(_) => std::process::abort(),
    }
}
struct Frame {
    head: Cell<Option<Box<Retirement>>>,
    root: Option<Arc<Record>>,
}
impl Frame {
    fn push(&self, mut node: Box<Retirement>) {
        node.next = self.head.take();
        self.head.set(Some(node));
    }
    fn finish(&self) {
        let unwinding = std::thread::panicking();
        if unwinding && let Some(root) = &self.root {
            root.poison();
        }
        while let Some(mut node) = self.head.take() {
            self.head.set(node.next.take());
            let record = node.ticket.record.clone();
            let uses_root = self.root.as_ref().is_some_and(|r| Arc::ptr_eq(r, &record));
            if !uses_root && record.busy.swap(true, Ordering::AcqRel) {
                // Legal admission keeps all live tickets on this owner. A
                // competing cleanup is an unsafe owner-contract violation.
                std::process::abort();
            }
            if unwinding {
                record.poison();
            }
            if !node.action.cleanup() {
                record.status.fetch_or(REFUSED, Ordering::Release);
            }
            drop(node); // Empty native state, then return this ticket's capacity.
            if !uses_root {
                record.busy.store(false, Ordering::Release);
            }
        }
        if let Some(root) = &self.root {
            root.busy.store(false, Ordering::Release);
        }
        if ACTIVE.try_with(|p| p.set(std::ptr::null())).is_err() {
            std::process::abort();
        }
    }
}
struct ScopeGuard<'a>(&'a Frame);
impl Drop for ScopeGuard<'_> {
    fn drop(&mut self) {
        self.0.finish();
    }
}
/// Root admission already owns its reservation. Nested factory borrows share
/// the outer scope. Reentrant loads are refused by preflight/admit, never wait.
pub(super) fn scope<R>(root: Option<Arc<Record>>, operation: impl FnOnce() -> R) -> R {
    if in_native_scope() {
        return operation();
    }
    let frame = Frame {
        head: Cell::new(None),
        root,
    };
    let _guard = ScopeGuard(&frame);
    if ACTIVE.try_with(|p| p.set(&raw const frame)).is_err() {
        std::process::abort();
    }
    operation() // frame drains before the result returns or host Rust unwinds.
}
pub(super) fn retire(node: Box<Retirement>) {
    if node.ticket.owner != std::thread::current().id() {
        node.ticket.record.poison();
        std::process::abort();
    }
    if in_native_scope() {
        if ACTIVE
            .try_with(|p| {
                // SAFETY: scope keeps this stack Frame immobile and alive through
                // all callbacks and cleanup. Owner affinity forbids foreign access.
                unsafe { (&*p.get()).push(node) };
            })
            .is_err()
        {
            std::process::abort();
        }
    } else {
        scope(None, || retire(node));
    }
}

#[cfg(test)]
mod tests;
