//! Real native worker/file lifecycle tests. The only adapter is explicitly
//! test-only authored CPU copying, and makes no inference/ML quality claim.
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Cursor, Read};
use std::path::Path;
use std::sync::{
    Arc, Barrier,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;
use tempfile::TempDir;
use windfall_analysis::*;
use windfall_core::AudioBuffer;

const MODEL: &[u8] = include_bytes!("fixtures/lifecycle-cpu-v1.model");
fn work() -> Work {
    Work::new(CancelToken::default(), 10_000_000, Duration::from_secs(30)).unwrap()
}
fn manifest() -> ModelManifest {
    ModelManifest {
        id: "test-only-lifecycle-copy".into(),
        version: "1".into(),
        revision: 1,
        sha256: Sha256::digest(MODEL).into(),
        bytes: MODEL.len() as u64,
        max_bytes: 4096,
        provenance: ModelProvenance {
            origin: "authored:windfall/lifecycle-cpu-v1".into(),
            source_revision: "fixture-v1".into(),
            author: "Windfall contributors".into(),
            license_spdx: "CC0-1.0".into(),
            license_reference: "tests/fixtures/README.md".into(),
            adapter_id: "test-only-copy".into(),
            adapter_version: "1".into(),
            device: "cpu".into(),
        },
        sample_rate: 48_000,
        input_channels: 2,
        max_input_frames: 48_000,
        outputs: vec![OutputRole {
            role: "lifecycle-copy".into(),
            channels: 2,
        }],
    }
}
fn source() -> AudioBuffer {
    AudioBuffer::from_interleaved(
        48_000,
        2,
        vec![0.25, -0.25, 0.5, -0.5, 0.75, -0.75, 1.0, -1.0],
    )
}
fn stamp(audio: &AudioBuffer) -> CaptureStamp {
    CaptureStamp {
        generation: 7,
        edit_revision: 11,
        source_key: 5,
        binding: [42; 32],
        source_identity: audio.identity(),
        source_fingerprint: ContentFingerprint::audio(audio, &mut work()).unwrap(),
        input_shape: AudioShape::of(audio),
        selection: FrameRange { start: 1, end: 3 },
    }
}
fn request(audio: &AudioBuffer) -> JobRequest {
    JobRequest {
        input: CapturedInput::capture(audio, stamp(audio), InputLimits::default(), &mut work())
            .unwrap(),
        model: manifest(),
        budget: JobBudget {
            scratch_bytes: 4096,
            output_bytes: 16 * 1024,
            work_units: 1_000_000,
            timeout: Duration::from_secs(30),
        },
    }
}

#[derive(Default)]
struct CopyAdapter {
    gate: Option<Arc<Gate>>,
    mode: Mode,
    active: AtomicUsize,
    maximum_active: AtomicUsize,
    starts: AtomicUsize,
}
#[derive(Default, Clone, Copy)]
enum Mode {
    #[default]
    Copy,
    Nonfinite,
    TooLong,
    TooShort,
    DecodeError,
    Panic,
    Missing,
    WrongRole,
    HugeChunk,
    TooMany,
    EncodedTooLarge,
    BadPath,
}
struct Gate {
    entered: Barrier,
    release: Barrier,
}
impl Gate {
    fn new(parties: usize) -> Arc<Self> {
        Arc::new(Self {
            entered: Barrier::new(parties),
            release: Barrier::new(parties),
        })
    }
    fn hold(self: &Arc<Self>) -> Release {
        self.entered.wait();
        Release(Some(self.clone()))
    }
}
// Unwind releases the worker before the manager's joining destructor.
struct Release(Option<Arc<Gate>>);
impl Release {
    fn release(mut self) {
        self.0.take().unwrap().release.wait();
    }
}
impl Drop for Release {
    fn drop(&mut self) {
        if let Some(gate) = self.0.take() {
            gate.release.wait();
        }
    }
}
struct Active<'a>(&'a AtomicUsize);
impl Drop for Active<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
impl AnalysisAdapter for CopyAdapter {
    fn id(&self) -> &str {
        "test-only-copy"
    }
    fn version(&self) -> &str {
        "1"
    }
    fn run(
        &self,
        input: &CapturedInput,
        model: &VerifiedModel,
        context: &mut WorkerContext,
        writer: &mut ArtifactWriter,
    ) -> Result<()> {
        assert_eq!(model.bytes(), MODEL);
        assert_eq!(context.scratch().len(), 1024);
        self.starts.fetch_add(1, Ordering::SeqCst);
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.maximum_active.fetch_max(active, Ordering::SeqCst);
        let _active = Active(&self.active);
        if let Some(gate) = &self.gate {
            gate.entered.wait();
            gate.release.wait();
        }
        context.checkpoint(1)?;
        match self.mode {
            Mode::DecodeError => {
                return writer.import_audio(
                    RelativeName::new("copy.wav")?,
                    "lifecycle-copy",
                    Cursor::new(b"not an audio file"),
                    context.work(),
                );
            }
            Mode::Panic => panic!("authored adapter panic seam"),
            Mode::Missing => return Ok(()),
            Mode::EncodedTooLarge => {
                return writer.import_audio(
                    RelativeName::new("copy.wav")?,
                    "lifecycle-copy",
                    io::repeat(0).take(32 * 1024),
                    context.work(),
                );
            }
            Mode::BadPath => {
                RelativeName::new("../competitor.wav")?;
            }
            _ => {}
        }
        let role = if matches!(self.mode, Mode::WrongRole) {
            "not-declared"
        } else {
            "lifecycle-copy"
        };
        writer.start_audio(RelativeName::new("copy.wav")?, role, context.work())?;
        match self.mode {
            Mode::Nonfinite => writer.write_audio(&[f32::NAN, 0.0], context.work())?,
            Mode::TooLong => writer.write_audio(&[0.0; 6], context.work())?,
            Mode::TooShort => writer.write_audio(&[0.0; 2], context.work())?,
            Mode::HugeChunk => writer.write_audio(&[0.0; 4098], context.work())?,
            _ => {
                for chunk in input.selected_samples().chunks(4096) {
                    writer.write_audio(chunk, context.work())?;
                }
            }
        }
        writer.finish_audio(context.work())?;
        if matches!(self.mode, Mode::TooMany) {
            writer.start_audio(
                RelativeName::new("second.wav")?,
                "lifecycle-copy",
                context.work(),
            )?;
        }
        Ok(())
    }
}

struct Fixture {
    root: TempDir,
    cache: ModelCache,
    config: ManagerConfig,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let cache = ModelCache::new(
            root.path().join("models"),
            ModelCacheLimits {
                max_model_bytes: 4096,
                max_cache_bytes: 64 * 1024,
                max_entries: 16,
                max_imports: 2,
            },
        )
        .unwrap();
        cache
            .import_reader(&manifest(), Cursor::new(MODEL), &mut work())
            .unwrap();
        let mut config =
            ManagerConfig::bounded(root.path().join("staging"), root.path().join("published"));
        config.workers = 1;
        config.max_timeout = Duration::from_secs(30);
        Self {
            root,
            cache,
            config,
        }
    }
    fn manager(&self, adapter: Arc<CopyAdapter>) -> JobManager {
        JobManager::new(self.config.clone(), self.cache.clone(), adapter).unwrap()
    }
    fn staging_empty(&self) {
        assert_eq!(fs::read_dir(&self.config.staging_root).unwrap().count(), 0);
    }
}
fn name(text: &str) -> Vec<RelativeName> {
    vec![RelativeName::new(text).unwrap()]
}
fn ready(manager: &JobManager, audio: &AudioBuffer) -> Review {
    let id = manager.submit(request(audio)).unwrap();
    let snapshot = manager.wait(id).unwrap();
    assert_eq!(snapshot.status, JobStatus::Ready, "{:?}", snapshot.failure);
    manager.review(snapshot.ticket).unwrap()
}

#[test]
fn actual_worker_review_commit_retirement_and_history_source_lifetime() {
    assert_eq!(
        format!("{:x}", Sha256::digest(MODEL)),
        "07f6812eb7ac451adc4b010bb18b2fe098ce235498880c80ead3fad519e47d33"
    );
    let fixture = Fixture::new();
    let manager = fixture.manager(Arc::new(CopyAdapter::default()));
    let audio = source();
    let review = ready(&manager, &audio);
    let before = manager.snapshot(review.job).unwrap();
    assert_eq!(
        before.transitions,
        [JobStatus::Queued, JobStatus::Running, JobStatus::Ready]
    );
    assert!(before.sequence > 3 && before.completed_work > MODEL.len() as u64);
    assert_eq!(review.capture.source_identity, audio.identity());
    assert_eq!(review.artifacts[0].frame_origin, 1);
    assert_eq!(
        review.artifacts[0].shape,
        AudioShape {
            frames: 2,
            channels: 2,
            sample_rate: 48_000
        }
    );
    let lease = manager
        .claim(review.ticket, &stamp(&audio), &manifest())
        .unwrap();
    assert!(matches!(
        manager.cancel(review.job),
        Err(AnalysisError::Claimed)
    ));
    assert!(matches!(
        manager.claim(review.ticket, &stamp(&audio), &manifest()),
        Err(AnalysisError::Claimed)
    ));
    let prepared = lease.prepare(name("history.wav")).unwrap();
    prepared
        .check_eligibility(&stamp(&audio), &manifest())
        .unwrap();
    let path = prepared.review().artifacts[0]
        .published_path
        .clone()
        .unwrap();
    let bytes = fs::read(&path).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        "4dbb9f64ee9794914f7a9bf01ed95c3775858a4bb9413877a99218b88077528b"
    );
    assert_eq!(bytes.len(), 74);
    assert_eq!(
        windfall_codec::decode_file(&path).unwrap().samples(),
        &[0.5, -0.5, 0.75, -0.75]
    );
    // This test simulates caller acceptance; it does NOT prove a Session batch.
    let applied = prepared.acknowledge_commit();
    assert_eq!(
        manager.snapshot(review.job).unwrap().status,
        JobStatus::Consumed
    );
    assert!(manager.usage().memory_bytes > 64 * 1024);
    assert!(matches!(
        manager.forget(review.job),
        Err(AnalysisError::Claimed)
    ));
    let assets = applied.retire();
    fixture.staging_empty();
    assert_eq!(manager.usage().memory_bytes, 64 * 1024);
    assert_eq!(manager.cancel(review.job).unwrap(), JobStatus::Consumed);
    assert!(matches!(
        manager.claim(review.ticket, &stamp(&audio), &manifest()),
        Err(AnalysisError::NotReady)
    ));
    manager.forget(review.job).unwrap();
    assert_eq!(manager.usage().memory_bytes, 0);
    manager.shutdown();
    drop(manager);
    // Undo removes a reference; redo/save/archive still need identical bytes.
    assert_eq!(
        fs::read(assets[0].published_path.as_ref().unwrap()).unwrap(),
        bytes
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn barrier_queue_concurrency_cancel_and_count_retention() {
    let mut fixture = Fixture::new();
    fixture.config.max_queued = 1;
    fixture.config.max_retained_jobs = 2;
    let gate = Gate::new(2);
    let adapter = Arc::new(CopyAdapter {
        gate: Some(gate.clone()),
        ..CopyAdapter::default()
    });
    let manager = fixture.manager(adapter.clone());
    let audio = source();
    let running = manager.submit(request(&audio)).unwrap();
    let release = gate.hold();
    assert_eq!(
        manager.snapshot(running).unwrap().status,
        JobStatus::Running
    );
    let queued = manager.submit(request(&audio)).unwrap();
    assert_eq!(manager.snapshot(queued).unwrap().status, JobStatus::Queued);
    assert!(matches!(
        manager.submit(request(&audio)),
        Err(AnalysisError::Budget(_))
    ));
    assert!(matches!(
        manager.forget(running),
        Err(AnalysisError::NotReady)
    ));
    assert_eq!(manager.cancel(queued).unwrap(), JobStatus::Cancelled);
    assert_eq!(
        manager.wait(queued).unwrap().transitions,
        [
            JobStatus::Queued,
            JobStatus::Cancelling,
            JobStatus::Cancelled
        ]
    );
    assert_eq!(manager.cancel(running).unwrap(), JobStatus::Cancelling);
    assert_eq!(
        manager.snapshot(running).unwrap().status,
        JobStatus::Cancelling
    );
    release.release();
    assert_eq!(manager.wait(running).unwrap().status, JobStatus::Cancelled);
    assert_eq!(adapter.starts.load(Ordering::SeqCst), 1);
    assert_eq!(adapter.maximum_active.load(Ordering::SeqCst), 1);
    fixture.staging_empty();
    assert_eq!(manager.usage().active, 0);
    assert_eq!(manager.usage().queued, 0);
    assert_eq!(manager.usage().reserved_disk_bytes, 0);
    assert!(matches!(
        manager.submit(request(&audio)),
        Err(AnalysisError::Budget(_))
    ));
    manager.forget(queued).unwrap();
    manager.forget(running).unwrap();
    let later = manager.submit(request(&audio)).unwrap();
    assert!(later.0 > queued.0);
    gate.hold().release();
    let result = manager.wait(later).unwrap();
    assert!(result.request > manager.usage().retained_jobs as u64);
    manager.cancel(later).unwrap();
}

#[test]
fn two_real_workers_stop_at_barrier_and_outputs_do_not_share_staging() {
    let mut fixture = Fixture::new();
    fixture.config.workers = 2;
    let gate = Gate::new(3);
    let adapter = Arc::new(CopyAdapter {
        gate: Some(gate.clone()),
        ..CopyAdapter::default()
    });
    let manager = fixture.manager(adapter.clone());
    let audio = source();
    let first = manager.submit(request(&audio)).unwrap();
    let second = manager.submit(request(&audio)).unwrap();
    let release = gate.hold();
    assert_eq!(manager.usage().active, 2);
    assert_eq!(adapter.maximum_active.load(Ordering::SeqCst), 2);
    release.release();
    let one = manager.wait(first).unwrap();
    let two = manager.wait(second).unwrap();
    assert_eq!(one.status, JobStatus::Ready);
    assert_eq!(two.status, JobStatus::Ready);
    assert_eq!(
        fs::read_dir(&fixture.config.staging_root).unwrap().count(),
        2
    );
    manager.cancel(first).unwrap();
    assert_eq!(
        fs::read_dir(&fixture.config.staging_root).unwrap().count(),
        1
    );
    assert_eq!(
        manager.review(two.ticket).unwrap().artifacts[0]
            .shape
            .frames,
        2
    );
    manager.cancel(second).unwrap();
    fixture.staging_empty();
}

#[test]
fn compact_inputs_shape_finite_range_and_memory_admission() {
    let audio = source();
    let mut vector = Vec::with_capacity(1_000_000);
    vector.extend_from_slice(audio.samples());
    let oversized_capacity = AudioBuffer::from_interleaved(48_000, 2, vector);
    let input = CapturedInput::capture(
        &oversized_capacity,
        stamp(&oversized_capacity),
        InputLimits::default(),
        &mut work(),
    )
    .unwrap();
    assert_eq!(input.retained_bytes(), 32);
    assert_ne!(input.audio().identity(), oversized_capacity.identity());
    for range in [
        FrameRange { start: 3, end: 1 },
        FrameRange { start: 1, end: 1 },
        FrameRange { start: 0, end: 5 },
    ] {
        let mut capture = stamp(&audio);
        capture.selection = range;
        assert!(
            CapturedInput::capture(&audio, capture, InputLimits::default(), &mut work()).is_err()
        );
    }
    let limits = InputLimits {
        max_bytes: 31,
        ..InputLimits::default()
    };
    assert!(matches!(
        CapturedInput::capture(&audio, stamp(&audio), limits, &mut work()),
        Err(AnalysisError::Budget(_))
    ));
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let invalid = AudioBuffer::from_interleaved(48_000, 2, vec![value, 0.0, 0.0, 0.0]);
        let mut capture = stamp(&audio);
        capture.input_shape = AudioShape::of(&invalid);
        capture.selection = FrameRange { start: 0, end: 2 };
        assert!(
            CapturedInput::capture(&invalid, capture, InputLimits::default(), &mut work()).is_err()
        );
    }
    let mut fixture = Fixture::new();
    fixture.config.memory_bytes = 2 * 1024 * 1024 + 64 * 1024;
    let manager = fixture.manager(Arc::new(CopyAdapter::default()));
    assert!(matches!(
        manager.submit(request(&audio)),
        Err(AnalysisError::Budget("retained job memory"))
    ));
    assert_eq!(manager.usage().retained_jobs, 0);
}

#[test]
fn worker_output_failures_release_only_owned_staging() {
    for mode in [
        Mode::Nonfinite,
        Mode::TooLong,
        Mode::TooShort,
        Mode::DecodeError,
        Mode::Panic,
        Mode::Missing,
        Mode::WrongRole,
        Mode::HugeChunk,
        Mode::TooMany,
        Mode::EncodedTooLarge,
        Mode::BadPath,
    ] {
        let fixture = Fixture::new();
        let manager = fixture.manager(Arc::new(CopyAdapter {
            mode,
            ..CopyAdapter::default()
        }));
        let competitor = fixture.config.published_root.join("competitor.wav");
        fs::write(&competitor, b"competitor bytes").unwrap();
        let id = manager.submit(request(&source())).unwrap();
        let result = manager.wait(id).unwrap();
        assert_eq!(result.status, JobStatus::Failed);
        assert!(result.failure.is_some());
        fixture.staging_empty();
        assert_eq!(fs::read(&competitor).unwrap(), b"competitor bytes");
        assert_eq!(manager.usage().reserved_disk_bytes, 0);
        assert_eq!(manager.usage().memory_bytes, 64 * 1024);
    }
}

#[test]
fn stale_canonical_checks_leave_review_and_publication_unconsumed() {
    let fixture = Fixture::new();
    let manager = fixture.manager(Arc::new(CopyAdapter::default()));
    let audio = source();
    let review = ready(&manager, &audio);
    let mut changes = Vec::new();
    let original = stamp(&audio);
    let mut x = original.clone();
    x.generation += 1;
    changes.push(x);
    let mut x = original.clone();
    x.edit_revision += 1;
    changes.push(x);
    let mut x = original.clone();
    x.source_key += 1;
    changes.push(x);
    let mut x = original.clone();
    x.binding[0] ^= 1;
    changes.push(x);
    let mut x = original.clone();
    x.source_fingerprint.sha256[0] ^= 1;
    changes.push(x);
    let other = source();
    let mut x = original.clone();
    x.source_identity = other.identity();
    changes.push(x);
    let mut x = original.clone();
    x.selection = FrameRange { start: 0, end: 2 };
    changes.push(x);
    let mut x = original.clone();
    x.input_shape.sample_rate = 44_100;
    changes.push(x);
    for current in changes {
        assert!(matches!(
            manager.claim(review.ticket, &current, &manifest()),
            Err(AnalysisError::Stale)
        ));
    }
    let mut changed_model = manifest();
    changed_model.revision += 1;
    assert!(matches!(
        manager.claim(review.ticket, &original, &changed_model),
        Err(AnalysisError::Stale)
    ));
    assert_eq!(
        manager.snapshot(review.job).unwrap().status,
        JobStatus::Ready
    );
    let prepared = manager
        .claim(review.ticket, &original, &manifest())
        .unwrap()
        .prepare(name("stale.wav"))
        .unwrap();
    let bytes = fs::read(fixture.config.published_root.join("stale.wav")).unwrap();
    let mut changed = original;
    changed.edit_revision += 1;
    assert!(matches!(
        prepared.check_eligibility(&changed, &manifest()),
        Err(AnalysisError::Stale)
    ));
    drop(prepared); // actual caller commit refused
    assert_eq!(
        manager.snapshot(review.job).unwrap().status,
        JobStatus::Ready
    );
    manager.cancel(review.job).unwrap();
    fixture.staging_empty();
    assert_eq!(
        fs::read(fixture.config.published_root.join("stale.wav")).unwrap(),
        bytes
    );
    assert_eq!(manager.usage().published_disk_bytes, bytes.len() as u64);
}

#[test]
fn final_collision_competitor_bytes_and_failed_commit_retry_are_bounded() {
    let fixture = Fixture::new();
    let manager = fixture.manager(Arc::new(CopyAdapter::default()));
    let audio = source();
    let review = ready(&manager, &audio);
    let target = fixture.config.published_root.join("collision.wav");
    fs::write(&target, b"competitor").unwrap();
    assert!(matches!(
        manager
            .claim(review.ticket, &stamp(&audio), &manifest())
            .unwrap()
            .prepare(name("collision.wav")),
        Err(AnalysisError::Collision)
    ));
    assert_eq!(fs::read(&target).unwrap(), b"competitor");
    assert!(matches!(
        manager
            .claim(review.ticket, &stamp(&audio), &manifest())
            .unwrap()
            .prepare(name("different.wav")),
        Err(AnalysisError::Invalid(_))
    ));
    assert!(!fixture.config.published_root.join("different.wav").exists());
    manager.cancel(review.job).unwrap();
    fixture.staging_empty();
    let review = ready(&manager, &audio);
    let prepared = manager
        .claim(review.ticket, &stamp(&audio), &manifest())
        .unwrap()
        .prepare(name("retry.wav"))
        .unwrap();
    let before = manager.usage();
    let path = prepared.review().artifacts[0]
        .published_path
        .clone()
        .unwrap();
    let bytes = fs::read(&path).unwrap();
    drop(prepared);
    let retry = manager
        .claim(review.ticket, &stamp(&audio), &manifest())
        .unwrap()
        .prepare(name("retry.wav"))
        .unwrap();
    assert_eq!(
        manager.usage().published_disk_bytes,
        before.published_disk_bytes
    );
    assert_eq!(manager.usage().published_files, before.published_files);
    retry.acknowledge_commit().retire();
    assert_eq!(fs::read(path).unwrap(), bytes);
    fixture.staging_empty();
}

#[test]
fn explicit_model_import_checks_cache_content_size_cancel_io_and_absent() {
    let fixture = Fixture::new();
    let model = manifest();
    let path = fixture
        .cache
        .import_reader(&model, Cursor::new(MODEL), &mut work())
        .unwrap();
    let original = fs::read(&path).unwrap();
    fs::write(&path, vec![b'x'; MODEL.len()]).unwrap();
    assert!(matches!(
        fixture.cache.load(&model, &mut work()),
        Err(AnalysisError::Checksum)
    ));
    assert!(matches!(
        fixture
            .cache
            .import_reader(&model, Cursor::new(MODEL), &mut work()),
        Err(AnalysisError::Checksum)
    ));
    assert_eq!(fs::read(&path).unwrap(), vec![b'x'; MODEL.len()]);
    fs::write(&path, original).unwrap();
    let absent = ModelManifest {
        sha256: [1; 32],
        ..model.clone()
    };
    assert!(matches!(
        fixture.cache.load(&absent, &mut work()),
        Err(AnalysisError::ModelAbsent)
    ));
    for data in [&MODEL[..MODEL.len() - 1], &[0; 512][..]] {
        assert!(matches!(
            fixture
                .cache
                .import_reader(&absent, Cursor::new(data), &mut work()),
            Err(AnalysisError::ModelSize)
        ));
    }
    assert!(matches!(
        fixture
            .cache
            .import_reader(&absent, Cursor::new(MODEL), &mut work()),
        Err(AnalysisError::Checksum)
    ));
    let token = CancelToken::default();
    token.cancel();
    let mut cancelled = Work::new(token, 10_000, Duration::from_secs(30)).unwrap();
    assert!(matches!(
        fixture
            .cache
            .import_reader(&absent, Cursor::new(MODEL), &mut cancelled),
        Err(AnalysisError::Cancelled)
    ));
    struct BadReader;
    impl Read for BadReader {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("authored read failure"))
        }
    }
    assert!(matches!(
        fixture.cache.import_reader(&absent, BadReader, &mut work()),
        Err(AnalysisError::Io(_))
    ));
    assert_eq!(fs::read_dir(fixture.cache.root()).unwrap().count(), 1);
    assert_eq!(
        fixture.cache.load(&model, &mut work()).unwrap().bytes(),
        MODEL
    );
}

#[test]
fn file_fingerprint_tracks_actual_content_and_ignores_path_aliases() {
    let fixture = Fixture::new();
    let a = fixture.root.path().join("source.bin");
    let alias = fixture.root.path().join("alias.bin");
    fs::write(&a, b"same-size-A").unwrap();
    fs::hard_link(&a, &alias).unwrap();
    let fingerprint = |path: &Path| {
        ContentFingerprint::reader(fs::File::open(path).unwrap(), 1024, &mut work()).unwrap()
    };
    let before = fingerprint(&a);
    assert_eq!(fingerprint(&alias), before);
    fs::write(&a, b"same-size-B").unwrap();
    let after = fingerprint(&alias);
    assert_eq!(before.bytes, after.bytes);
    assert_ne!(before.sha256, after.sha256);
}

#[test]
fn disk_file_work_and_integer_bounds_fail_observably() {
    let audio = source();
    let mut fixture = Fixture::new();
    fixture.config.disk_bytes = 16 * 1024;
    let manager = fixture.manager(Arc::new(CopyAdapter::default()));
    assert!(matches!(
        manager.submit(request(&audio)),
        Err(AnalysisError::Budget("job/publication disk"))
    ));
    let fixture = Fixture::new();
    let manager = fixture.manager(Arc::new(CopyAdapter::default()));
    let mut oversized = request(&audio);
    oversized.budget.output_bytes = u64::MAX;
    assert!(matches!(
        manager.submit(oversized),
        Err(AnalysisError::Exhausted)
    ));
    let mut too_little_output = request(&audio);
    too_little_output.budget.output_bytes = 1;
    assert!(matches!(
        manager.submit(too_little_output),
        Err(AnalysisError::Budget("aligned output reservation"))
    ));
    let mut little_work = request(&audio);
    little_work.budget.work_units = 1;
    let id = manager.submit(little_work).unwrap();
    let failed = manager.wait(id).unwrap();
    assert_eq!(failed.status, JobStatus::Failed);
    assert!(failed.failure.unwrap().contains("work units"));
    assert!(matches!(
        AudioShape {
            frames: u64::MAX,
            channels: 2,
            sample_rate: 48_000
        }
        .pcm_bytes(),
        Err(AnalysisError::Exhausted)
    ));
    assert!(Work::new(CancelToken::default(), u64::MAX, Duration::from_secs(1)).is_err());
    for bad in [
        "../a.wav", "a/b.wav", "C:a.wav", "a\\b.wav", "CON.wav", "LPT1.wav", ".a.wav", "a..wav",
        "a.wav.", "a.wav ",
    ] {
        assert!(RelativeName::new(bad).is_err(), "{bad}");
    }
}

#[test]
fn shutdown_cancels_queue_and_running_worker_then_joins_without_sleeps() {
    let fixture = Fixture::new();
    let gate = Gate::new(2);
    let adapter = Arc::new(CopyAdapter {
        gate: Some(gate.clone()),
        ..CopyAdapter::default()
    });
    let manager = Arc::new(fixture.manager(adapter));
    let audio = source();
    let running = manager.submit(request(&audio)).unwrap();
    let release = gate.hold();
    let queued = manager.submit(request(&audio)).unwrap();
    let sequence = manager.snapshot(running).unwrap().sequence;
    let shutdown_manager = manager.clone();
    let shutdown = std::thread::spawn(move || shutdown_manager.shutdown());
    assert_eq!(
        manager.wait_for_change(running, sequence).unwrap().status,
        JobStatus::Cancelling
    );
    assert_eq!(manager.wait(queued).unwrap().status, JobStatus::Cancelled);
    release.release();
    shutdown.join().unwrap();
    assert_eq!(manager.wait(running).unwrap().status, JobStatus::Cancelled);
    assert_eq!(manager.wait(queued).unwrap().status, JobStatus::Cancelled);
    assert!(matches!(
        manager.submit(request(&audio)),
        Err(AnalysisError::Shutdown)
    ));
    assert_eq!(manager.usage().active, 0);
    fixture.staging_empty();
}

#[test]
fn actual_output_io_refusal_preserves_replacement_and_reserves_failed_cleanup() {
    let fixture = Fixture::new();
    let gate = Gate::new(2);
    let manager = fixture.manager(Arc::new(CopyAdapter {
        gate: Some(gate.clone()),
        ..CopyAdapter::default()
    }));
    let audio = source();
    let id = manager.submit(request(&audio)).unwrap();
    let release = gate.hold();
    let staging = fs::read_dir(&fixture.config.staging_root)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(
        staging.parent(),
        Some(fixture.config.staging_root.as_path())
    );
    fs::remove_dir(&staging).unwrap();
    fs::write(&staging, b"replacement file must survive refusal").unwrap();
    release.release();
    let result = manager.wait(id).unwrap();
    assert_eq!(result.status, JobStatus::Failed);
    assert!(result.failure.unwrap().contains("cleanup failed"));
    assert_eq!(
        fs::read(&staging).unwrap(),
        b"replacement file must survive refusal"
    );
    assert!(manager.usage().reserved_disk_bytes > 0);
    assert!(manager.forget(id).is_err());
    assert!(manager.retry_cleanup(id).is_err());
    // External owner removes its replacement, then only our reservation retires.
    fs::remove_file(&staging).unwrap();
    manager.retry_cleanup(id).unwrap();
    assert_eq!(manager.usage().reserved_disk_bytes, 0);
    manager.forget(id).unwrap();
    fixture.staging_empty();
}
