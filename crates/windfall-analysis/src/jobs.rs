use crate::artifacts::{Artifact, publish};
use crate::model::inventory;
use crate::{
    AnalysisError, ArtifactMetadata, ArtifactWriter, CancelToken, CaptureStamp, CapturedInput,
    InputLimits, ModelCache, ModelCacheLimits, ModelManifest, RelativeName, Result, VerifiedModel,
    Work, add, lock,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tempfile::TempDir;

const RECORD_BYTES: u64 = 64 * 1024;
const CODEC_ALLOWANCE: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct JobId(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TicketId(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Queued,
    Running,
    Cancelling,
    Cancelled,
    Failed,
    Ready,
    Consumed,
}
impl JobStatus {
    fn settled(self) -> bool {
        matches!(
            self,
            Self::Cancelled | Self::Failed | Self::Ready | Self::Consumed
        )
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSnapshot {
    pub id: JobId,
    pub request: u64,
    pub ticket: TicketId,
    pub status: JobStatus,
    pub sequence: u64,
    pub completed_work: u64,
    pub maximum_work: u64,
    pub failure: Option<String>,
    /// Only transitions, at most eight; progress is a coalesced latest value.
    pub transitions: Vec<JobStatus>,
}
#[derive(Debug, Clone, Copy)]
pub struct JobBudget {
    pub scratch_bytes: u64,
    pub output_bytes: u64,
    pub work_units: u64,
    pub timeout: Duration,
}

pub struct JobRequest {
    pub input: CapturedInput,
    pub model: ModelManifest,
    pub budget: JobBudget,
}

#[derive(Debug, Clone)]
pub struct ManagerConfig {
    pub staging_root: PathBuf,
    pub published_root: PathBuf,
    pub workers: usize,
    pub max_queued: usize,
    pub max_retained_jobs: usize,
    pub memory_bytes: u64,
    /// Includes staging + in-progress publication + every retained final file.
    pub disk_bytes: u64,
    pub max_published_files: usize,
    pub max_scratch_bytes: u64,
    pub max_work_units: u64,
    pub max_timeout: Duration,
    pub input_limits: InputLimits,
}
impl ManagerConfig {
    pub fn bounded(staging_root: PathBuf, published_root: PathBuf) -> Self {
        Self {
            staging_root,
            published_root,
            workers: 2,
            max_queued: 4,
            max_retained_jobs: 16,
            memory_bytes: 256 * 1024 * 1024,
            disk_bytes: 1024 * 1024 * 1024,
            max_published_files: 256,
            max_scratch_bytes: 64 * 1024 * 1024,
            max_work_units: 4_000_000_000,
            max_timeout: Duration::from_secs(600),
            input_limits: InputLimits::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    pub queued: usize,
    pub active: usize,
    pub retained_jobs: usize,
    pub memory_bytes: u64,
    pub reserved_disk_bytes: u64,
    pub published_disk_bytes: u64,
    pub reserved_output_files: usize,
    pub published_files: usize,
}

/// Trusted native adapter, installed once per manager. An adapter's own fixed
/// model/runtime memory must be charged in scratch_bytes; it must use the
/// provided bounded scratch and checkpoint between bounded CPU chunks. This
/// cooperative contract does not sandbox arbitrary native allocations/code.
/// No production inference adapter ships with this foundation.
pub trait AnalysisAdapter: Send + Sync + 'static {
    fn id(&self) -> &str;
    fn version(&self) -> &str;
    fn run(
        &self,
        input: &CapturedInput,
        model: &VerifiedModel,
        context: &mut WorkerContext,
        artifacts: &mut ArtifactWriter,
    ) -> Result<()>;
}
pub struct WorkerContext {
    work: Work,
    scratch: Vec<f32>,
}
impl WorkerContext {
    pub fn work(&mut self) -> &mut Work {
        &mut self.work
    }
    pub fn scratch(&mut self) -> &mut [f32] {
        &mut self.scratch
    }
    pub fn checkpoint(&mut self, units: u64) -> Result<()> {
        self.work.checkpoint(units)
    }
}

#[derive(Debug, Clone)]
pub struct Review {
    pub job: JobId,
    pub ticket: TicketId,
    pub capture: CaptureStamp,
    pub model: ModelManifest,
    pub artifacts: Vec<ArtifactMetadata>,
}
impl Review {
    /// Pure eligibility only. The caller must rehash/recover the canonical
    /// current source off State, then atomically guard recording-before-State
    /// and dispatch one checked document batch. This proves no installation.
    pub fn check_eligibility(&self, current: &CaptureStamp, model: &ModelManifest) -> Result<()> {
        if self.capture != *current || self.model != *model || !current.source_identity.is_live() {
            return Err(AnalysisError::Stale);
        }
        Ok(())
    }
}

struct Proposal {
    input: CapturedInput,
    model: ModelManifest,
    _directory: TempDir,
    artifacts: Vec<Artifact>,
    destinations: Option<Vec<RelativeName>>,
}
impl Proposal {
    fn review(&self, id: JobId, ticket: TicketId) -> Review {
        Review {
            job: id,
            ticket,
            capture: self.input.stamp().clone(),
            model: self.model.clone(),
            artifacts: self.artifacts.iter().map(|a| a.metadata.clone()).collect(),
        }
    }
}
struct Task {
    request: JobRequest,
    deadline: Instant,
}
struct Record {
    snapshot: JobSnapshot,
    task: Option<Task>,
    proposal: Option<Proposal>,
    cancel: CancelToken,
    claimed: bool,
    retiring: bool,
    memory: u64,
    disk: u64,
    files: usize,
    budget: JobBudget,
    cleanup_path: Option<PathBuf>,
}
impl Record {
    fn bump(&mut self) -> Result<()> {
        if self.snapshot.sequence >= u64::MAX - 16 {
            return Err(AnalysisError::Exhausted);
        }
        self.snapshot.sequence += 1;
        Ok(())
    }
    fn transition(&mut self, status: JobStatus) {
        // All fallible operations reserve terminal-transition headroom.
        self.snapshot.sequence += 1;
        self.snapshot.status = status;
        self.snapshot.transitions.push(status);
        debug_assert!(self.snapshot.transitions.len() <= 8);
    }
}
#[derive(Clone)]
struct Counters {
    job: u64,
    ticket: u64,
    request: u64,
}
fn next(counter: &mut u64) -> Result<u64> {
    if *counter == u64::MAX {
        return Err(AnalysisError::Exhausted);
    }
    let value = *counter;
    *counter += 1;
    Ok(value)
}
struct State {
    records: BTreeMap<JobId, Record>,
    queue: VecDeque<JobId>,
    usage: Usage,
    counters: Counters,
    shutdown: bool,
}
struct Shared {
    state: Mutex<State>,
    changed: Condvar,
    config: ManagerConfig,
    cache: ModelCache,
    adapter: Arc<dyn AnalysisAdapter>,
}

/// Control-side owner. Drop/shutdown joins real worker threads and can do IO;
/// it must never run under State, controller or audio locks.
pub struct JobManager {
    shared: Arc<Shared>,
    workers: Mutex<Vec<JoinHandle<()>>>,
    lifecycle: Mutex<()>,
}
impl JobManager {
    pub fn new(
        mut config: ManagerConfig,
        cache: ModelCache,
        adapter: Arc<dyn AnalysisAdapter>,
    ) -> Result<Self> {
        if config.workers == 0
            || config.workers > 16
            || config.max_queued == 0
            || config.max_retained_jobs == 0
            || config.max_retained_jobs > 256
            || config.memory_bytes < RECORD_BYTES + CODEC_ALLOWANCE
            || config.disk_bytes == 0
            || config.max_published_files == 0
            || config.max_work_units == 0
            || config.max_work_units > u64::MAX - 32
            || config.max_timeout.is_zero()
        {
            return Err(AnalysisError::Invalid("manager bounds"));
        }
        fs::create_dir_all(&config.staging_root)?;
        fs::create_dir_all(&config.published_root)?;
        config.staging_root = fs::canonicalize(&config.staging_root)?;
        config.published_root = fs::canonicalize(&config.published_root)?;
        let roots = [
            &config.staging_root as &Path,
            &config.published_root,
            cache.root(),
        ];
        for (i, root) in roots.iter().enumerate() {
            for other in &roots[..i] {
                if root.starts_with(other) || other.starts_with(root) {
                    return Err(AnalysisError::Invalid("overlapping ownership directories"));
                }
            }
        }
        let (published_disk_bytes, published_files) = inventory(
            &config.published_root,
            ModelCacheLimits {
                max_model_bytes: config.disk_bytes,
                max_cache_bytes: config.disk_bytes,
                max_entries: config.max_published_files,
                max_imports: 1,
            },
        )?;
        // Stale directories from earlier processes are never deleted here.
        // Refuse a nonempty staging root: a session must give us fresh storage.
        if fs::read_dir(&config.staging_root)?.next().is_some() {
            return Err(AnalysisError::Invalid(
                "staging root must be empty and exclusively owned",
            ));
        }
        let shared = Arc::new(Shared {
            config,
            cache,
            adapter,
            changed: Condvar::new(),
            state: Mutex::new(State {
                records: BTreeMap::new(),
                queue: VecDeque::new(),
                usage: Usage {
                    published_disk_bytes,
                    published_files,
                    ..Usage::default()
                },
                counters: Counters {
                    job: 1,
                    ticket: 1,
                    request: 1,
                },
                shutdown: false,
            }),
        });
        let manager = Self {
            shared,
            workers: Mutex::new(Vec::new()),
            lifecycle: Mutex::new(()),
        };
        for number in 0..manager.shared.config.workers {
            let shared = manager.shared.clone();
            let handle = thread::Builder::new()
                .name(format!("windfall-analysis-{number}"))
                .spawn(move || worker(shared))?;
            lock(&manager.workers).push(handle);
        }
        Ok(manager)
    }
    pub fn submit(&self, request: JobRequest) -> Result<JobId> {
        let config = &self.shared.config;
        let shape = request.input.stamp().input_shape;
        let frames = request.input.stamp().selection.frames()?;
        request.model.validate_input(shape, frames)?;
        if request.model.provenance.adapter_id != self.shared.adapter.id()
            || request.model.provenance.adapter_version != self.shared.adapter.version()
        {
            return Err(AnalysisError::Invalid("adapter/model revision mismatch"));
        }
        let b = request.budget;
        if b.scratch_bytes > config.max_scratch_bytes
            || !b.scratch_bytes.is_multiple_of(4)
            || b.scratch_bytes > usize::MAX as u64
            || b.output_bytes == 0
            || b.work_units == 0
            || b.work_units > config.max_work_units
            || b.timeout.is_zero()
            || b.timeout > config.max_timeout
        {
            return Err(AnalysisError::Invalid("job budget bounds"));
        }
        let limits = config.input_limits;
        if request.input.retained_bytes() > limits.max_bytes
            || shape.frames > limits.max_frames
            || shape.channels > limits.max_channels
            || shape.sample_rate < limits.min_sample_rate
            || shape.sample_rate > limits.max_sample_rate
        {
            return Err(AnalysisError::Budget("manager input bounds"));
        }
        let mut decode_bytes = 0;
        let mut minimum_output = 0;
        for output in &request.model.outputs {
            let bytes = frames
                .checked_mul(u64::from(output.channels))
                .and_then(|n| n.checked_mul(4))
                .ok_or(AnalysisError::Exhausted)?;
            decode_bytes = decode_bytes.max(bytes);
            minimum_output = add(minimum_output, add(bytes, 256)?)?;
        }
        if minimum_output > b.output_bytes {
            return Err(AnalysisError::Budget("aligned output reservation"));
        }
        let memory = add(
            add(
                add(
                    add(request.input.retained_bytes(), request.model.bytes)?,
                    b.scratch_bytes,
                )?,
                decode_bytes,
            )?,
            CODEC_ALLOWANCE,
        )?;
        let disk = b
            .output_bytes
            .checked_mul(2)
            .ok_or(AnalysisError::Exhausted)?;
        let deadline = Instant::now()
            .checked_add(b.timeout)
            .ok_or(AnalysisError::Exhausted)?;
        let files = request.model.outputs.len();
        let mut state = lock(&self.shared.state);
        if state.shutdown {
            return Err(AnalysisError::Shutdown);
        }
        if state.queue.len() >= config.max_queued || state.records.len() >= config.max_retained_jobs
        {
            return Err(AnalysisError::Budget("queue/retained job count"));
        }
        if add(state.usage.memory_bytes, add(memory, RECORD_BYTES)?)? > config.memory_bytes {
            return Err(AnalysisError::Budget("retained job memory"));
        }
        if add(
            add(
                state.usage.reserved_disk_bytes,
                state.usage.published_disk_bytes,
            )?,
            disk,
        )? > config.disk_bytes
        {
            return Err(AnalysisError::Budget("job/publication disk"));
        }
        if state
            .usage
            .published_files
            .checked_add(state.usage.reserved_output_files)
            .and_then(|n| n.checked_add(files))
            .is_none_or(|n| n > config.max_published_files)
        {
            return Err(AnalysisError::Budget("persistent output count"));
        }
        let mut counters = state.counters.clone();
        let id = JobId(next(&mut counters.job)?);
        let ticket = TicketId(next(&mut counters.ticket)?);
        let request_id = next(&mut counters.request)?;
        state.counters = counters;
        state.usage.memory_bytes += memory + RECORD_BYTES;
        state.usage.reserved_disk_bytes += disk;
        state.usage.reserved_output_files += files;
        state.usage.queued += 1;
        state.usage.retained_jobs += 1;
        state.queue.push_back(id);
        state.records.insert(
            id,
            Record {
                snapshot: JobSnapshot {
                    id,
                    request: request_id,
                    ticket,
                    status: JobStatus::Queued,
                    sequence: 1,
                    completed_work: 0,
                    maximum_work: b.work_units,
                    failure: None,
                    transitions: vec![JobStatus::Queued],
                },
                task: Some(Task { request, deadline }),
                proposal: None,
                cancel: CancelToken::default(),
                claimed: false,
                retiring: false,
                memory,
                disk,
                files,
                budget: b,
                cleanup_path: None,
            },
        );
        self.shared.changed.notify_all();
        Ok(id)
    }
    pub fn usage(&self) -> Usage {
        lock(&self.shared.state).usage
    }
    pub fn snapshot(&self, id: JobId) -> Result<JobSnapshot> {
        Ok(lock(&self.shared.state)
            .records
            .get(&id)
            .ok_or(AnalysisError::Unknown)?
            .snapshot
            .clone())
    }
    /// Wait off locks for ready/failed/cancelled/consumed. Condition-variable
    /// predicates include record removal and shutdown; there is no polling.
    pub fn wait(&self, id: JobId) -> Result<JobSnapshot> {
        let mut state = lock(&self.shared.state);
        loop {
            let record = state.records.get(&id).ok_or(AnalysisError::Unknown)?;
            if record.snapshot.status.settled() && !record.retiring {
                return Ok(record.snapshot.clone());
            }
            state = self
                .shared
                .changed
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }
    pub fn wait_for_change(&self, id: JobId, after_sequence: u64) -> Result<JobSnapshot> {
        let mut state = lock(&self.shared.state);
        loop {
            let record = state.records.get(&id).ok_or(AnalysisError::Unknown)?;
            if after_sequence > record.snapshot.sequence {
                return Err(AnalysisError::Invalid("future progress sequence"));
            }
            if record.snapshot.sequence > after_sequence || record.snapshot.status.settled() {
                return Ok(record.snapshot.clone());
            }
            state = self
                .shared
                .changed
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }
    pub fn review(&self, ticket: TicketId) -> Result<Review> {
        let state = lock(&self.shared.state);
        let (id, record) = find_ticket(&state, ticket)?;
        if record.claimed {
            return Err(AnalysisError::Claimed);
        }
        let proposal = record.proposal.as_ref().ok_or(AnalysisError::NotReady)?;
        Ok(proposal.review(id, ticket))
    }
    pub fn cancel(&self, id: JobId) -> Result<JobStatus> {
        let cleanup = {
            let mut state = lock(&self.shared.state);
            let record = state.records.get_mut(&id).ok_or(AnalysisError::Unknown)?;
            if record.claimed {
                return Err(AnalysisError::Claimed);
            }
            match record.snapshot.status {
                JobStatus::Consumed
                | JobStatus::Cancelled
                | JobStatus::Failed
                | JobStatus::Cancelling => return Ok(record.snapshot.status),
                JobStatus::Running => {
                    record.cancel.cancel();
                    record.transition(JobStatus::Cancelling);
                    self.shared.changed.notify_all();
                    return Ok(JobStatus::Cancelling);
                }
                JobStatus::Queued | JobStatus::Ready => {
                    record.cancel.cancel();
                    record.transition(JobStatus::Cancelling);
                    record.retiring = true;
                    let task = record.task.take();
                    let proposal = record.proposal.take();
                    if task.is_some() {
                        state.queue.retain(|queued| *queued != id);
                        state.usage.queued -= 1;
                    }
                    Cleanup {
                        shared: self.shared.clone(),
                        id,
                        task,
                        proposal,
                        terminal: JobStatus::Cancelled,
                        failure: None,
                    }
                }
            }
        };
        drop(cleanup);
        Ok(JobStatus::Cancelled)
    }
    /// Explicit retention policy: only settled, unclaimed, fully retired jobs
    /// can be forgotten. Ready work must first be cancelled or consumed.
    pub fn forget(&self, id: JobId) -> Result<()> {
        let mut state = lock(&self.shared.state);
        let record = state.records.get(&id).ok_or(AnalysisError::Unknown)?;
        if record.claimed || record.retiring {
            return Err(AnalysisError::Claimed);
        }
        if !matches!(
            record.snapshot.status,
            JobStatus::Consumed | JobStatus::Cancelled | JobStatus::Failed
        ) {
            return Err(AnalysisError::NotReady);
        }
        if record.cleanup_path.is_some() {
            return Err(AnalysisError::Budget("staging cleanup remains reserved"));
        }
        debug_assert_eq!(record.memory, 0);
        state.records.remove(&id);
        state.usage.memory_bytes -= RECORD_BYTES;
        state.usage.retained_jobs -= 1;
        self.shared.changed.notify_all();
        Ok(())
    }
    pub fn claim(
        &self,
        ticket: TicketId,
        current: &CaptureStamp,
        model: &ModelManifest,
    ) -> Result<ApplyLease> {
        let mut state = lock(&self.shared.state);
        if state.shutdown {
            return Err(AnalysisError::Shutdown);
        }
        let (id, record) = find_ticket(&state, ticket)?;
        if record.claimed {
            return Err(AnalysisError::Claimed);
        }
        let proposal = record.proposal.as_ref().ok_or(AnalysisError::NotReady)?;
        proposal
            .review(id, ticket)
            .check_eligibility(current, model)?;
        if record.snapshot.completed_work >= record.budget.work_units {
            return Err(AnalysisError::Budget("apply work units"));
        }
        let timeout = record.budget.timeout;
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or(AnalysisError::Exhausted)?;
        let request = next(&mut state.counters.request)?;
        let record = state.records.get_mut(&id).expect("ticket found");
        record.bump()?;
        record.claimed = true;
        let mut work = make_work(
            &self.shared,
            id,
            CancelToken::default(),
            record.budget.work_units,
            deadline,
        );
        work.set_completed(record.snapshot.completed_work);
        let proposal = record.proposal.take();
        Ok(ApplyLease {
            shared: self.shared.clone(),
            id,
            ticket,
            request,
            proposal,
            work,
        })
    }
    pub fn shutdown(&self) {
        let _lifecycle = lock(&self.lifecycle);
        let ids = {
            let mut state = lock(&self.shared.state);
            state.shutdown = true;
            self.shared.changed.notify_all();
            state.records.keys().copied().collect::<Vec<_>>()
        };
        for id in ids {
            let _ = self.cancel(id);
        }
        let handles = std::mem::take(&mut *lock(&self.workers));
        for handle in handles {
            let _ = handle.join();
        }
    }
    /// Retry only this job's private refused staging, never a published path.
    /// A cleanup failure retains its disk/count reservation and blocks forget.
    pub fn retry_cleanup(&self, id: JobId) -> Result<()> {
        let path = {
            let mut state = lock(&self.shared.state);
            let record = state.records.get_mut(&id).ok_or(AnalysisError::Unknown)?;
            if record.claimed || record.retiring {
                return Err(AnalysisError::Claimed);
            }
            if !matches!(
                record.snapshot.status,
                JobStatus::Cancelled | JobStatus::Failed | JobStatus::Consumed
            ) {
                return Err(AnalysisError::NotReady);
            }
            let Some(path) = record.cleanup_path.clone() else {
                return Ok(());
            };
            record.retiring = true;
            path
        };
        let result = remove_owned_stage(&path, &self.shared.config.staging_root);
        let mut state = lock(&self.shared.state);
        let record = state.records.get_mut(&id).expect("cleanup retry retained");
        record.retiring = false;
        if result.is_ok() {
            record.cleanup_path = None;
            let (disk, files) = (record.disk, record.files);
            record.disk = 0;
            record.files = 0;
            state.usage.reserved_disk_bytes -= disk;
            state.usage.reserved_output_files -= files;
        }
        self.shared.changed.notify_all();
        result
    }
}
impl Drop for JobManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn find_ticket(state: &State, ticket: TicketId) -> Result<(JobId, &Record)> {
    state
        .records
        .iter()
        .find(|(_, record)| record.snapshot.ticket == ticket)
        .map(|(id, record)| (*id, record))
        .ok_or(AnalysisError::Unknown)
}
fn make_work(
    shared: &Arc<Shared>,
    id: JobId,
    cancel: CancelToken,
    maximum: u64,
    deadline: Instant,
) -> Work {
    let mut work = Work::with_deadline(cancel, maximum, deadline);
    let progress_shared = shared.clone();
    work.progress = Some(Arc::new(move |completed| {
        let mut state = lock(&progress_shared.state);
        let record = state.records.get_mut(&id).ok_or(AnalysisError::Unknown)?;
        record.bump()?;
        if completed < record.snapshot.completed_work {
            return Err(AnalysisError::Invalid("progress regressed"));
        }
        record.snapshot.completed_work = completed;
        progress_shared.changed.notify_all();
        Ok(())
    }));
    work
}

fn worker(shared: Arc<Shared>) {
    loop {
        let (id, task, cancel) = {
            let mut state = lock(&shared.state);
            loop {
                if state.shutdown {
                    return;
                }
                if let Some(id) = state.queue.pop_front() {
                    state.usage.queued -= 1;
                    state.usage.active += 1;
                    let record = state.records.get_mut(&id).expect("queued record");
                    record.transition(JobStatus::Running);
                    let task = record.task.take().expect("queued task");
                    let cancel = record.cancel.clone();
                    shared.changed.notify_all();
                    break (id, task, cancel);
                }
                state = shared
                    .changed
                    .wait(state)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        };
        let deadline = task.deadline;
        let token = cancel.clone();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            execute(&shared, id, task, token)
        }))
        .unwrap_or_else(|_| Err(AnalysisError::Adapter("native adapter panicked".into())));
        let cleanup = {
            let mut state = lock(&shared.state);
            state.usage.active -= 1;
            let shutting_down = state.shutdown;
            let record = state.records.get_mut(&id).expect("active record retained");
            match result {
                Ok(proposal)
                    if !cancel.is_cancelled() && !shutting_down && Instant::now() < deadline =>
                {
                    record.proposal = Some(proposal);
                    record.transition(JobStatus::Ready);
                    shared.changed.notify_all();
                    None
                }
                result => {
                    let (proposal, failure, terminal) = match result {
                        Ok(proposal) if cancel.is_cancelled() || shutting_down => {
                            (Some(proposal), None, JobStatus::Cancelled)
                        }
                        Ok(proposal) => (
                            Some(proposal),
                            Some(AnalysisError::Deadline.to_string()),
                            JobStatus::Failed,
                        ),
                        Err(_) if cancel.is_cancelled() || shutting_down => {
                            (None, None, JobStatus::Cancelled)
                        }
                        Err(error) => (
                            None,
                            Some(error.to_string().chars().take(1024).collect()),
                            JobStatus::Failed,
                        ),
                    };
                    record.retiring = true;
                    Some(Cleanup {
                        shared: shared.clone(),
                        id,
                        task: None,
                        proposal,
                        terminal,
                        failure,
                    })
                }
            }
        };
        drop(cleanup);
    }
}

fn execute(shared: &Arc<Shared>, id: JobId, task: Task, cancel: CancelToken) -> Result<Proposal> {
    let request = task.request;
    let work = make_work(shared, id, cancel, request.budget.work_units, task.deadline);
    let mut context = WorkerContext {
        work,
        scratch: Vec::new(),
    };
    context.work.check()?;
    context
        .scratch
        .try_reserve_exact((request.budget.scratch_bytes / 4) as usize)
        .map_err(|_| AnalysisError::Budget("worker scratch allocation"))?;
    if context.scratch.capacity() as u64 * 4 > request.budget.scratch_bytes {
        return Err(AnalysisError::Budget("worker scratch capacity"));
    }
    context
        .scratch
        .resize((request.budget.scratch_bytes / 4) as usize, 0.0);
    let model = shared.cache.load(&request.model, &mut context.work)?;
    let mut writer = ArtifactWriter::new(
        &shared.config.staging_root,
        &request.model,
        request.input.stamp().selection.frames()?,
        request.input.stamp().selection.start,
        request.budget.output_bytes,
    )?;
    lock(&shared.state)
        .records
        .get_mut(&id)
        .expect("running record")
        .cleanup_path = Some(writer.path().to_path_buf());
    shared
        .adapter
        .run(&request.input, &model, &mut context, &mut writer)?;
    let (directory, artifacts) = writer.complete(&mut context.work)?;
    Ok(Proposal {
        input: request.input,
        model: request.model,
        _directory: directory,
        artifacts,
        destinations: None,
    })
}

/// Drops all uncommitted staging and buffers before releasing reservations.
/// Published outputs are never owned by this deletion path.
struct Cleanup {
    shared: Arc<Shared>,
    id: JobId,
    task: Option<Task>,
    proposal: Option<Proposal>,
    terminal: JobStatus,
    failure: Option<String>,
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        drop(self.task.take());
        drop(self.proposal.take());
        let path = lock(&self.shared.state)
            .records
            .get(&self.id)
            .expect("cleanup record retained")
            .cleanup_path
            .clone();
        let removal = path
            .as_ref()
            .map(|p| remove_owned_stage(p, &self.shared.config.staging_root))
            .transpose();
        let mut state = lock(&self.shared.state);
        let record = state
            .records
            .get_mut(&self.id)
            .expect("cleanup record retained");
        let (memory, disk, files) = (record.memory, record.disk, record.files);
        record.memory = 0;
        record.retiring = false;
        record.snapshot.failure = self.failure.take();
        if let Err(error) = &removal {
            record.snapshot.failure = Some(format!(
                "Owned staging cleanup failed and remains reserved: {error}"
            ));
        } else {
            record.disk = 0;
            record.files = 0;
            record.cleanup_path = None;
        }
        if record.snapshot.status != self.terminal {
            record.transition(self.terminal);
        }
        state.usage.memory_bytes -= memory;
        if removal.is_ok() {
            state.usage.reserved_disk_bytes -= disk;
            state.usage.reserved_output_files -= files;
        }
        self.shared.changed.notify_all();
    }
}

fn remove_owned_stage(path: &Path, root: &Path) -> Result<()> {
    if path.parent() != Some(root) {
        return Err(AnalysisError::Invalid("staging ownership path"));
    }
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            fs::remove_dir_all(path)?;
            Ok(())
        }
        Ok(_) => Err(AnalysisError::Invalid(
            "staging directory was replaced; manual recovery required",
        )),
    }
}

/// Exclusive ready-ticket ownership; cancellation/forget refuse while claimed.
/// Dropping a failed/refused apply restores Ready (or cancels during shutdown).
/// One bounded set of destinations is pinned on the first publication attempt;
/// retries reuse already published files and cannot grow orphan output sets.
pub struct ApplyLease {
    shared: Arc<Shared>,
    id: JobId,
    ticket: TicketId,
    request: u64,
    proposal: Option<Proposal>,
    work: Work,
}
impl ApplyLease {
    pub fn request(&self) -> u64 {
        self.request
    }
    pub fn input(&self) -> &CapturedInput {
        &self.proposal.as_ref().expect("live apply lease").input
    }
    pub fn review(&self) -> Review {
        self.proposal
            .as_ref()
            .expect("live apply lease")
            .review(self.id, self.ticket)
    }
    /// File copy, checksum, no-clobber publication and sync all happen here,
    /// off State/audio. A collision leaves its final bytes intact. Even partial
    /// successful final publications are retained and charged, never unlinked.
    pub fn prepare(mut self, names: Vec<RelativeName>) -> Result<PreparedApply> {
        let proposal = self.proposal.as_mut().expect("live lease");
        if names.len() != proposal.artifacts.len()
            || names
                .iter()
                .enumerate()
                .any(|(i, n)| names[..i].contains(n))
        {
            return Err(AnalysisError::Invalid("publication name count/duplicates"));
        }
        if proposal
            .destinations
            .as_ref()
            .is_some_and(|old| *old != names)
        {
            return Err(AnalysisError::Invalid("publication destinations changed"));
        }
        proposal.destinations = Some(names.clone());
        for (artifact, name) in proposal.artifacts.iter_mut().zip(names) {
            let target = self.shared.config.published_root.join(name.as_str());
            if let Some(path) = &artifact.metadata.published_path {
                if *path != target {
                    return Err(AnalysisError::Invalid("published target changed"));
                }
                let fingerprint = crate::ContentFingerprint::reader(
                    fs::File::open(path)?,
                    artifact.metadata.bytes,
                    &mut self.work,
                )?;
                if fingerprint.bytes != artifact.metadata.bytes
                    || fingerprint.sha256 != artifact.metadata.sha256
                {
                    return Err(AnalysisError::Checksum);
                }
                continue;
            }
            publish(artifact, &target, &mut self.work)?;
            artifact.metadata.published_path = Some(target);
            let mut state = lock(&self.shared.state);
            let record = state
                .records
                .get_mut(&self.id)
                .expect("claimed record retained");
            record.disk -= artifact.metadata.bytes;
            record.files -= 1;
            state.usage.reserved_disk_bytes -= artifact.metadata.bytes;
            state.usage.published_disk_bytes += artifact.metadata.bytes;
            state.usage.reserved_output_files -= 1;
            state.usage.published_files += 1;
        }
        Ok(PreparedApply { lease: self })
    }
}
impl Drop for ApplyLease {
    fn drop(&mut self) {
        let Some(proposal) = self.proposal.take() else {
            return;
        };
        let cleanup = {
            let mut state = lock(&self.shared.state);
            let shutdown = state.shutdown;
            let record = state
                .records
                .get_mut(&self.id)
                .expect("claimed record retained");
            record.claimed = false;
            if shutdown {
                record.retiring = true;
                Some(Cleanup {
                    shared: self.shared.clone(),
                    id: self.id,
                    task: None,
                    proposal: Some(proposal),
                    terminal: JobStatus::Cancelled,
                    failure: None,
                })
            } else {
                record.proposal = Some(proposal);
                self.shared.changed.notify_all();
                None
            }
        };
        drop(cleanup);
    }
}

pub struct PreparedApply {
    lease: ApplyLease,
}
impl PreparedApply {
    pub fn review(&self) -> Review {
        self.lease.review()
    }
    pub fn check_eligibility(&self, current: &CaptureStamp, model: &ModelManifest) -> Result<()> {
        self.review().check_eligibility(current, model)
    }
    /// Call ONLY after the caller's real atomic document commit. Infallible,
    /// with no disk/codec/worker operations; returns deferred retirement, which
    /// must be dropped/retired after every Session/recording guard is released.
    /// If the caller commit failed, drop PreparedApply instead to restore Ready.
    pub fn acknowledge_commit(mut self) -> AppliedAssets {
        let proposal = self.lease.proposal.take().expect("prepared proposal");
        {
            let mut state = lock(&self.lease.shared.state);
            let record = state
                .records
                .get_mut(&self.lease.id)
                .expect("claimed record retained");
            record.claimed = false;
            record.retiring = true;
            record.transition(JobStatus::Consumed);
            self.lease.shared.changed.notify_all();
        }
        // Move metadata, retain input/staging reservations until retirement.
        let artifacts = proposal
            .artifacts
            .iter()
            .map(|a| a.metadata.clone())
            .collect();
        AppliedAssets {
            artifacts,
            cleanup: Some(Cleanup {
                shared: self.lease.shared.clone(),
                id: self.lease.id,
                task: None,
                proposal: Some(proposal),
                terminal: JobStatus::Consumed,
                failure: None,
            }),
        }
    }
}

/// Persistent source metadata plus control-side retirement. The persistent
/// files outlive this value, job cancellation, history changes and shutdown.
pub struct AppliedAssets {
    artifacts: Vec<ArtifactMetadata>,
    cleanup: Option<Cleanup>,
}
impl AppliedAssets {
    pub fn artifacts(&self) -> &[ArtifactMetadata] {
        &self.artifacts
    }
    pub fn retire(mut self) -> Vec<ArtifactMetadata> {
        drop(self.cleanup.take());
        std::mem::take(&mut self.artifacts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counters_fail_before_rollover_without_reusing_ids() {
        let mut counter = u64::MAX - 1;
        assert_eq!(next(&mut counter).unwrap(), u64::MAX - 1);
        assert!(matches!(next(&mut counter), Err(AnalysisError::Exhausted)));
        assert_eq!(counter, u64::MAX);
    }
}
