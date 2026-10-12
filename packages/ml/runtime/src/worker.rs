//! Synchronous native training worker boundary. Hosts run it on a worker thread/process.
use crate::*;
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    sync::{
        Arc, Condvar, Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicI64, Ordering},
    },
    time::{Duration, Instant},
};

/// Engines check for cancellation every few milliseconds; the ledger is read at most this often.
const STATUS_POLL_INTERVAL: Duration = Duration::from_millis(250);
/// While a job keeps publishing checkpoints, superseded ones are collected at most this often.
const BLOB_SWEEP_INTERVAL: Duration = Duration::from_secs(15);

/// The lock survives lease expiry and is released by the OS when its process exits.
pub(crate) struct LocalRunLock {
    _file: File,
}
impl LocalRunLock {
    pub(crate) fn acquire(repository: &TrainingRepository, key: &str) -> Result<Self> {
        let file = Self::open(repository, key)?;
        file.try_lock()
            .map_err(|error| Error::Conflict(format!("training operation is locked: {error}")))?;
        Ok(Self { _file: file })
    }

    /// Waits while another holder has the key exclusively.
    pub(crate) fn shared(repository: &TrainingRepository, key: &str) -> Result<Self> {
        let file = Self::open(repository, key)?;
        file.lock_shared()?;
        Ok(Self { _file: file })
    }

    fn open(repository: &TrainingRepository, key: &str) -> Result<File> {
        repository.writable()?;
        let database = repository.path().canonicalize()?;
        let directory = database
            .parent()
            .ok_or_else(|| invalid("training repository has no parent directory"))?
            .join(".training-locks");
        std::fs::create_dir_all(&directory)?;
        if !std::fs::symlink_metadata(&directory)?.is_dir() {
            return Err(invalid("training lock directory must be a real directory"));
        }
        let mut identity = database.as_os_str().as_encoded_bytes().to_vec();
        identity.push(0);
        identity.extend_from_slice(key.as_bytes());
        let path = directory.join(format!("{}.lock", digest(&identity)));
        if std::fs::symlink_metadata(&path).is_ok_and(|metadata| !metadata.is_file()) {
            return Err(invalid("training lock must be a regular file"));
        }
        Ok(OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?)
    }
}

/// Reconcile an expired lease only after confirming no worker still holds the job lock.
/// An interrupted cancellation stays cancelled; an interrupted run remains resumable.
pub fn reconcile_expired_job(repository: &TrainingRepository, job_id: &str) -> Result<TrainingJob> {
    let _lock = LocalRunLock::acquire(repository, &format!("job:{job_id}"))?;
    reconcile_locked_job(repository, job_id)
}

fn reconcile_locked_job(repository: &TrainingRepository, job_id: &str) -> Result<TrainingJob> {
    repository.reconcile_stopped_job(job_id, now_ms())
}

#[derive(Clone, Debug)]
pub struct TrainingWork {
    pub repository: TrainingRepository,
    pub job: TrainingJob,
    pub snapshot: DatasetSnapshot,
    pub resume_checkpoint: Option<Vec<u8>>,
}

#[derive(Clone, Debug)]
pub struct EngineOutput {
    pub model_bytes: Vec<u8>,
    pub manifest: Value,
}

/// Implementations perform native training and call check_cancel between batches. Hosts
/// needing a hard deadline should isolate the worker in a supervised child process.
pub trait TrainingEngine: Send + Sync {
    fn estimated_memory_bytes(&self, work: &TrainingWork) -> Result<u64>;
    /// Largest stored snapshot this engine can admit within `budget` bytes. Larger snapshots
    /// are rejected from their stored size, before they are parsed.
    fn snapshot_byte_limit(&self, budget: u64) -> u64 {
        budget
    }
    fn resource_key(&self, work: &TrainingWork) -> Result<String> {
        Ok(format!(
            "{}:{}",
            work.job.request.compute["backend"]
                .as_str()
                .unwrap_or("cpu"),
            work.job.request.compute["device_index"]
                .as_u64()
                .unwrap_or(0)
        ))
    }
    fn train(&self, work: &TrainingWork, control: &WorkerControl) -> Result<EngineOutput>;
}

#[derive(Clone, Copy)]
enum Stop {
    Cancelled,
    LeaseLost,
}
impl Stop {
    fn error(self) -> Error {
        match self {
            Self::Cancelled => Error::Cancelled,
            Self::LeaseLost => Error::LeaseLost,
        }
    }
}

#[derive(Default)]
struct StatusPoll {
    checked: Option<Instant>,
    stopped: Option<Stop>,
}

/// Lease state shared by the engine thread, its cancellation monitor and the heartbeat.
struct LeaseKeeper {
    repository: TrainingRepository,
    lease: JobLease,
    duration_ms: i64,
    deadline_ms: AtomicI64,
    lost: AtomicBool,
    /// Sequence number and value of the latest reported progress.
    progress: Mutex<(u64, Option<Value>)>,
    /// Sequence number of the stored progress, held while a renewal writes.
    persisted: Mutex<u64>,
    status: Mutex<StatusPoll>,
    swept: Mutex<Option<Instant>>,
}
impl LeaseKeeper {
    fn new(
        repository: TrainingRepository,
        lease: JobLease,
        duration_ms: i64,
        claimed_at_ms: i64,
    ) -> Self {
        Self {
            repository,
            lease,
            duration_ms,
            deadline_ms: AtomicI64::new(claimed_at_ms.saturating_add(duration_ms)),
            lost: AtomicBool::new(false),
            progress: Mutex::new((0, None)),
            persisted: Mutex::new(0),
            status: Mutex::new(StatusPoll::default()),
            swept: Mutex::new(None),
        }
    }

    fn ensure_current(&self) -> Result<()> {
        if self.lost.load(Ordering::Acquire)
            || now_ms() >= self.deadline_ms.load(Ordering::Acquire)
        {
            return Err(Error::LeaseLost);
        }
        Ok(())
    }

    /// Reads only the status column. A stop is final for this lease.
    fn poll_status(&self) -> Result<()> {
        let mut poll = lock(&self.status);
        if let Some(stop) = poll.stopped {
            return Err(stop.error());
        }
        if poll
            .checked
            .is_some_and(|checked| checked.elapsed() < STATUS_POLL_INTERVAL)
        {
            return Ok(());
        }
        poll.checked = Some(Instant::now());
        let stop = match self.repository.job_status(&self.lease.job_id) {
            Ok(JobStatus::Running) => return Ok(()),
            Ok(JobStatus::CancelRequested) => Stop::Cancelled,
            Ok(_) => Stop::LeaseLost,
            Err(error) if transient(&error) => return Ok(()),
            Err(error) => return Err(error),
        };
        poll.stopped = Some(stop);
        Err(stop.error())
    }

    fn record(&self, progress: Value) {
        let mut latest = lock(&self.progress);
        latest.0 += 1;
        latest.1 = Some(progress);
    }

    /// Renewals are serialized and always carry the newest progress, so stored progress never
    /// moves backwards; without newer progress only the lease is extended.
    fn renew(&self) -> Result<()> {
        let mut persisted = lock(&self.persisted);
        let (sequence, progress) = {
            let latest = lock(&self.progress);
            if latest.0 > *persisted {
                (latest.0, latest.1.clone())
            } else {
                (*persisted, None)
            }
        };
        let at_ms = now_ms();
        match self
            .repository
            .renew_lease(&self.lease, self.duration_ms, progress, at_ms)
        {
            Ok(()) => {
                *persisted = sequence;
                self.deadline_ms
                    .store(at_ms.saturating_add(self.duration_ms), Ordering::Release);
                Ok(())
            }
            Err(error) => {
                if !transient(&error) {
                    self.lost.store(true, Ordering::Release);
                }
                Err(error)
            }
        }
    }

    /// A storage error is left for the heartbeat to retry.
    fn persist(&self) -> Result<()> {
        match self.renew() {
            Err(error) if !transient(&error) => Err(error),
            _ => Ok(()),
        }
    }

    fn sweep_superseded(&self) {
        {
            let mut swept = lock(&self.swept);
            if swept.is_some_and(|at| at.elapsed() < BLOB_SWEEP_INTERVAL) {
                return;
            }
            *swept = Some(Instant::now());
        }
        let _ = self.repository.collect_unreferenced_blobs();
    }
}

#[derive(Clone)]
pub struct WorkerControl {
    keeper: Arc<LeaseKeeper>,
    limits: WorkerLimits,
    started: Instant,
    experiment_deadline_ms: Option<i64>,
}
impl WorkerControl {
    fn new(keeper: Arc<LeaseKeeper>, limits: WorkerLimits, request: &TrainingRequest) -> Self {
        Self {
            keeper,
            limits,
            started: Instant::now(),
            experiment_deadline_ms: request
                .recipe
                .get("experiment_deadline_ms")
                .and_then(Value::as_i64),
        }
    }

    pub fn limits(&self) -> &WorkerLimits {
        &self.limits
    }

    /// A stopped engine may persist the completed batch after cancellation was requested.
    /// The lease still has to be current; this never changes cancellation into success.
    pub fn checkpoint_after_stop(
        &self,
        bytes: &[u8],
        step: u64,
        metadata: Value,
    ) -> Result<CheckpointRef> {
        if bytes.len() as u64 > self.limits.maximum_checkpoint_bytes {
            return Err(invalid("checkpoint exceeds worker size budget"));
        }
        self.publish(bytes, step, metadata)
    }

    /// Lease loss and deadlines are checked in memory; the job status is read from the ledger
    /// at most every 250 ms.
    pub fn check_cancel(&self) -> Result<()> {
        self.keeper.ensure_current()?;
        self.keeper.poll_status()?;
        if self
            .experiment_deadline_ms
            .is_some_and(|deadline| now_ms() > deadline)
        {
            return Err(Error::Engine(
                "experiment wall time budget exhausted".into(),
            ));
        }
        if self.started.elapsed().as_millis() > self.limits.maximum_duration_ms as u128 {
            return Err(Error::Engine("training duration budget exhausted".into()));
        }
        Ok(())
    }

    /// The latest value stays in memory for the heartbeat to store; an epoch end, which
    /// carries a validation loss, is stored immediately.
    pub fn progress(&self, progress: Value) -> Result<()> {
        self.check_cancel()?;
        let epoch_end = progress
            .get("validation_loss")
            .is_some_and(|loss| !loss.is_null());
        self.keeper.record(progress);
        if epoch_end {
            self.keeper.persist()?;
        }
        Ok(())
    }

    pub fn checkpoint(&self, bytes: &[u8], step: u64, metadata: Value) -> Result<CheckpointRef> {
        self.check_cancel()?;
        if bytes.len() as u64 > self.limits.maximum_checkpoint_bytes {
            return Err(invalid("checkpoint exceeds worker size budget"));
        }
        self.publish(bytes, step, metadata)
    }

    fn publish(&self, bytes: &[u8], step: u64, metadata: Value) -> Result<CheckpointRef> {
        let checkpoint = self.keeper.repository.publish_checkpoint(
            &self.keeper.lease,
            bytes,
            step,
            metadata,
            false,
            now_ms(),
        )?;
        self.keeper.sweep_superseded();
        Ok(checkpoint)
    }
}

struct Admitted {
    engine: Arc<dyn TrainingEngine>,
    work: TrainingWork,
    lease: JobLease,
    claimed_at_ms: i64,
}

/// One invocation trains one durable job. The hosting scheduler controls concurrency and
/// device admission; returning from the triggering workflow does not cancel this worker.
pub struct TrainingWorker {
    repository: TrainingRepository,
    limits: WorkerLimits,
    engines: HashMap<String, Arc<dyn TrainingEngine>>,
}
impl TrainingWorker {
    pub fn limits(&self) -> &WorkerLimits {
        &self.limits
    }

    pub fn new(repository: TrainingRepository, limits: WorkerLimits) -> Result<Self> {
        if limits.memory_budget_bytes == 0
            || limits.maximum_artifact_bytes == 0
            || limits.maximum_checkpoint_bytes == 0
            || limits.maximum_duration_ms == 0
            || limits.lease_duration_ms < 300
        {
            return Err(invalid(
                "worker budgets must be positive; lease must be at least 300ms",
            ));
        }
        Ok(Self {
            repository,
            limits,
            engines: HashMap::new(),
        })
    }
    pub fn register(
        &mut self,
        name: impl Into<String>,
        engine: Arc<dyn TrainingEngine>,
    ) -> Result<()> {
        let name = name.into();
        if name.trim().is_empty() || self.engines.contains_key(&name) {
            return Err(invalid("engine name is empty or already registered"));
        }
        self.engines.insert(name, engine);
        Ok(())
    }
    pub fn run(&self, job_id: &str) -> Result<ModelArtifact> {
        let _lock = LocalRunLock::acquire(&self.repository, &format!("job:{job_id}"))?;
        let mut job = reconcile_locked_job(&self.repository, job_id)?;
        if matches!(job.status, JobStatus::Paused | JobStatus::Interrupted) {
            job = self.repository.resume_stopped_job(job_id, now_ms())?;
        }
        if matches!(
            job.status,
            JobStatus::Cancelled | JobStatus::CancelRequested
        ) {
            return Err(Error::Cancelled);
        }
        if job.status != JobStatus::Queued {
            return Err(Error::Conflict(
                "training job is not queued or stopped".into(),
            ));
        }
        let Admitted {
            engine,
            work,
            lease,
            claimed_at_ms,
        } = self.admit(job_id, job).inspect_err(|error| {
            let _ = self
                .repository
                .record_job_attempt_error(job_id, &error.to_string(), now_ms());
        })?;
        let keeper = Arc::new(LeaseKeeper::new(
            self.repository.clone(),
            lease.clone(),
            self.limits.lease_duration_ms,
            claimed_at_ms,
        ));
        let control = WorkerControl::new(keeper.clone(), self.limits.clone(), &work.job.request);
        let heartbeat = Heartbeat::start(keeper.clone());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut output = engine.train(&work, &control)?;
            control.check_cancel()?;
            for key in ["preprocessing_manifest", "inspection_spec"] {
                if let Some(value) = work.job.request.recipe.get(key) {
                    let manifest = output
                        .manifest
                        .as_object_mut()
                        .ok_or_else(|| invalid("model manifest must be an object"))?;
                    manifest.insert(key.into(), value.clone());
                }
            }
            if output.model_bytes.len() as u64 > self.limits.maximum_artifact_bytes {
                return Err(invalid("model exceeds worker artifact budget"));
            }
            keeper.persist()?;
            self.repository
                .finish_training(&lease, &output.model_bytes, output.manifest, now_ms())
        }))
        .unwrap_or_else(|payload| Err(panic_error(&*payload)))
        .map_err(name_device_memory_exhaustion);
        // Stop the heartbeat before settling so it cannot renew a terminal job.
        drop(heartbeat);
        if let Err(error) = &result {
            if matches!(error, Error::Cancelled) {
                let _ = self.repository.acknowledge_cancel(&lease, now_ms());
            } else {
                let _ = self
                    .repository
                    .fail_job(&lease, error.to_string(), now_ms());
            }
        }
        let _ = self.repository.collect_unreferenced_blobs();
        result
    }

    /// Everything that can fail before the claim. The caller records such failures on the
    /// still-queued job.
    fn admit(&self, job_id: &str, job: TrainingJob) -> Result<Admitted> {
        let engine = self
            .engines
            .get(&job.request.engine)
            .cloned()
            .ok_or_else(|| {
                Error::Engine(format!("engine {} is not registered", job.request.engine))
            })?;
        let requested = job
            .request
            .compute
            .get("memory_limit_bytes")
            .and_then(Value::as_u64)
            .unwrap_or(self.limits.memory_budget_bytes);
        let budget = requested.min(self.limits.memory_budget_bytes);
        let checkpoint_bytes = job
            .checkpoint
            .as_ref()
            .map_or(0, |checkpoint| checkpoint.blob.bytes);
        let snapshot_budget = budget
            .checked_sub(checkpoint_bytes)
            .ok_or_else(|| invalid("resume checkpoint exceeds the worker memory budget"))?;
        let snapshot = self.repository.get_snapshot_limited(
            &job.snapshot_id,
            engine
                .snapshot_byte_limit(snapshot_budget)
                .min(snapshot_budget),
        )?;
        let resume_checkpoint = job
            .checkpoint
            .as_ref()
            .map(|checkpoint| {
                if checkpoint.request_digest != digest(&serde_json::to_vec(&job.request)?) {
                    return Err(invalid("checkpoint recipe/compute contract changed"));
                }
                self.repository.read_blob_limited(
                    &checkpoint.blob,
                    self.limits.maximum_checkpoint_bytes.min(budget),
                )
            })
            .transpose()?;
        let work = TrainingWork {
            repository: self.repository.clone(),
            snapshot,
            job,
            resume_checkpoint,
        };
        let memory = engine.estimated_memory_bytes(&work)?;
        if memory > budget {
            return Err(Error::Engine(format!(
                "training needs an estimated {memory} bytes; budget is {budget}"
            )));
        }
        let claimed_at_ms = now_ms();
        let lease = self.repository.claim_job_with_resources(
            job_id,
            &format!("{}:{}", std::process::id(), id()),
            self.limits.lease_duration_ms,
            claimed_at_ms,
            Some(ResourceRequest {
                key: engine.resource_key(&work)?,
                estimated_bytes: memory,
                budget_bytes: self.limits.memory_budget_bytes,
            }),
        )?;
        Ok(Admitted {
            engine,
            work,
            lease,
            claimed_at_ms,
        })
    }
}

fn panic_error(payload: &(dyn std::any::Any + Send)) -> Error {
    let message = payload
        .downcast_ref::<&str>()
        .map(|message| (*message).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned());
    Error::Engine(match message {
        Some(message) => format!("training engine panicked: {}", first_line(&message)),
        None => "training engine panicked".into(),
    })
}

/// Device allocators report exhaustion in several spellings; the stored reason names the fix.
fn name_device_memory_exhaustion(error: Error) -> Error {
    match error {
        Error::Engine(message) if out_of_device_memory(&message) => Error::Engine(format!(
            "out of device memory; reduce the batch size or input resolution ({})",
            first_line(&message)
        )),
        error => error,
    }
}

fn out_of_device_memory(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    [
        "out of memory",
        "out-of-memory",
        "out of device memory",
        "out_of_memory",
        "outofmemory",
        "can't allocate buffer",
    ]
    .iter()
    .any(|marker| message.contains(marker))
}

fn first_line(message: &str) -> String {
    message
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .chars()
        .take(512)
        .collect()
}

fn transient(error: &Error) -> bool {
    matches!(error, Error::Sql(_) | Error::Io(_))
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

struct Heartbeat {
    stop: Arc<(Mutex<bool>, Condvar)>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Heartbeat {
    /// Renews every third of the lease. Storage errors are retried while the known lease is
    /// still valid; only a lost lease or an expired deadline stops the worker.
    fn start(keeper: Arc<LeaseKeeper>) -> Self {
        let stop = Arc::new((Mutex::new(false), Condvar::new()));
        let signal = stop.clone();
        let thread = std::thread::spawn(move || {
            let interval = Duration::from_millis((keeper.duration_ms / 3) as u64);
            let retry = Duration::from_millis((keeper.duration_ms / 10).clamp(10, 1000) as u64);
            let mut delay = interval;
            loop {
                let (guard, _) = signal
                    .1
                    .wait_timeout_while(lock(&signal.0), delay, |stop| !*stop)
                    .unwrap_or_else(|e| e.into_inner());
                if *guard {
                    break;
                }
                drop(guard);
                delay = match keeper.renew() {
                    Ok(()) => interval,
                    Err(error) if transient(&error) && keeper.ensure_current().is_ok() => retry,
                    Err(_) => {
                        keeper.lost.store(true, Ordering::Release);
                        break;
                    }
                };
            }
        });
        Self {
            stop,
            thread: Some(thread),
        }
    }
}
impl Drop for Heartbeat {
    fn drop(&mut self) {
        *lock(&self.stop.0) = true;
        self.stop.1.notify_all();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod lock_tests {
    use super::*;
    use serde_json::json;

    struct FixtureEngine;
    impl TrainingEngine for FixtureEngine {
        fn estimated_memory_bytes(&self, _: &TrainingWork) -> Result<u64> {
            Ok(1024)
        }
        fn train(&self, _: &TrainingWork, control: &WorkerControl) -> Result<EngineOutput> {
            control.check_cancel()?;
            Ok(EngineOutput {
                model_bytes: b"fixture".to_vec(),
                manifest: json!({}),
            })
        }
    }

    fn job(repository: &TrainingRepository) -> TrainingJob {
        job_in(repository, "locks")
    }

    fn job_in(repository: &TrainingRepository, stream_id: &str) -> TrainingJob {
        let stream = StreamKey {
            project_id: "test".into(),
            stream_id: stream_id.into(),
            inspection_version: "v1".into(),
        };
        for i in 0..12 {
            repository
                .record_sample(
                    &stream,
                    &TrainingSample {
                        id: format!("sample-{i}"),
                        annotation_revision: 1,
                        group_id: format!("group-{i}"),
                        captured_at_ms: 1,
                        label_available_at_ms: 1,
                        content_digest: format!("{i:064x}"),
                        source: LabelSource::Reviewed,
                        accepted: true,
                        payload: json!({"label":"normal"}),
                    },
                )
                .unwrap();
        }
        repository
            .trigger_after_n(
                &stream,
                12,
                TrainingRequest {
                    engine: "fixture".into(),
                    recipe: json!({}),
                    compute: json!({}),
                },
                SplitPolicy::Group {
                    train_fraction: 0.5,
                    validation_fraction: 0.25,
                    seed: 42,
                },
                now_ms(),
            )
            .unwrap()
            .unwrap()
    }

    #[cfg(feature = "burn")]
    fn monitor_control(repository: &TrainingRepository, job_id: &str) -> WorkerControl {
        let claimed_at_ms = now_ms();
        let lease = repository
            .claim_job(job_id, "monitor-test", 30_000, claimed_at_ms)
            .unwrap();
        let request = repository.get_job(job_id).unwrap().request;
        WorkerControl::new(
            Arc::new(LeaseKeeper::new(
                repository.clone(),
                lease,
                30_000,
                claimed_at_ms,
            )),
            WorkerLimits::default(),
            &request,
        )
    }

    #[cfg(feature = "burn")]
    #[test]
    fn cancellation_monitor_stops_backend_wait_without_progress_and_preserves_reason() {
        use crate::engines::BurnCancellationMonitor;
        for cause in ["requested", "deadline", "lease"] {
            let directory = tempfile::tempdir().unwrap();
            let repository =
                TrainingRepository::open(directory.path().join("training.sqlite")).unwrap();
            let job = job(&repository);
            if cause == "deadline" {
                let mut queued = repository.get_job(&job.id).unwrap();
                queued.request.recipe["experiment_deadline_ms"] = json!(now_ms() + 500);
                repository
                    .connection()
                    .unwrap()
                    .execute(
                        "UPDATE jobs SET body=? WHERE id=?",
                        rusqlite::params![serde_json::to_string(&queued).unwrap(), job.id],
                    )
                    .unwrap();
            }
            let control = monitor_control(&repository, &job.id);
            let cancellation = flow_like_ml_burn::CancellationToken::new();
            let monitor = BurnCancellationMonitor::start(&control, &cancellation).unwrap();
            match cause {
                "requested" => {
                    repository.request_cancel(&job.id, now_ms()).unwrap();
                }
                "lease" => control.keeper.lost.store(true, Ordering::Release),
                _ => {}
            }
            // The backend emits no callback while waiting for another job's RNG phase.
            let timeout = Instant::now() + Duration::from_secs(2);
            while !cancellation.is_cancelled() && Instant::now() < timeout {
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(
                cancellation.is_cancelled(),
                "monitor did not propagate {cause}"
            );
            match (cause, monitor.finish().unwrap()) {
                ("requested", Error::Cancelled) | ("lease", Error::LeaseLost) => {}
                ("deadline", Error::Engine(message)) => {
                    assert_eq!(message, "experiment wall time budget exhausted")
                }
                (_, error) => panic!("Monitor lost original {cause} reason: {error}"),
            }
        }
    }

    #[cfg(feature = "burn")]
    #[test]
    fn cancellation_monitor_drop_joins_before_later_job_cancellation() {
        let directory = tempfile::tempdir().unwrap();
        let repository =
            TrainingRepository::open(directory.path().join("training.sqlite")).unwrap();
        let job = job(&repository);
        let control = monitor_control(&repository, &job.id);
        let cancellation = flow_like_ml_burn::CancellationToken::new();
        let monitor =
            crate::engines::BurnCancellationMonitor::start(&control, &cancellation).unwrap();
        let started = Instant::now();
        drop(monitor);
        assert!(started.elapsed() < Duration::from_secs(1));
        repository.request_cancel(&job.id, now_ms()).unwrap();
        std::thread::sleep(Duration::from_millis(150));
        assert!(
            !cancellation.is_cancelled(),
            "dropped monitor remained active"
        );
    }

    #[test]
    fn active_os_lock_prevents_expired_worker_recovery_and_releasing_resources() {
        let directory = tempfile::tempdir().unwrap();
        let repository =
            TrainingRepository::open(directory.path().join("training.sqlite")).unwrap();
        let job = job(&repository);
        let lease = repository
            .claim_job_with_resources(
                &job.id,
                "old-worker",
                1000,
                now_ms() - 2000,
                Some(ResourceRequest {
                    key: "cpu:0".into(),
                    estimated_bytes: 1024,
                    budget_bytes: 1024,
                }),
            )
            .unwrap();
        let lock = LocalRunLock::acquire(&repository, &format!("job:{}", job.id)).unwrap();
        let mut worker = TrainingWorker::new(repository.clone(), WorkerLimits::default()).unwrap();
        worker.register("fixture", Arc::new(FixtureEngine)).unwrap();
        assert!(matches!(worker.run(&job.id), Err(Error::Conflict(_))));
        assert!(matches!(
            reconcile_expired_job(&repository, &job.id),
            Err(Error::Conflict(_))
        ));
        let unchanged = repository.get_job(&job.id).unwrap();
        assert_eq!(unchanged.status, JobStatus::Running);
        assert_eq!(unchanged.generation, lease.generation);
        let reserved: u64 = repository
            .connection()
            .unwrap()
            .query_row(
                "SELECT bytes FROM resource_reservations WHERE job_id=?",
                [&job.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(reserved, 1024);
        drop(lock);
        let reopened = TrainingRepository::open(repository.path()).unwrap();
        let mut recovered = TrainingWorker::new(reopened.clone(), WorkerLimits::default()).unwrap();
        recovered
            .register("fixture", Arc::new(FixtureEngine))
            .unwrap();
        let artifact = recovered.run(&job.id).unwrap();
        let completed = reopened.get_job(&job.id).unwrap();
        assert_eq!(completed.status, JobStatus::Succeeded);
        assert_eq!(completed.artifact_id.as_deref(), Some(artifact.id.as_str()));
        assert!(completed.generation > lease.generation);
        assert!(
            repository
                .heartbeat(&lease, 1000, json!({}), now_ms())
                .is_err()
        );
    }

    #[test]
    fn expired_cancelled_worker_reconciliation_never_restarts_training() {
        let directory = tempfile::tempdir().unwrap();
        let repository =
            TrainingRepository::open(directory.path().join("training.sqlite")).unwrap();
        let job = job(&repository);
        repository
            .claim_job(&job.id, "old-worker", 1000, now_ms() - 2000)
            .unwrap();
        repository.request_cancel(&job.id, now_ms()).unwrap();
        let reconciled = reconcile_expired_job(&repository, &job.id).unwrap();
        assert_eq!(reconciled.status, JobStatus::Cancelled);
        let mut worker = TrainingWorker::new(repository.clone(), WorkerLimits::default()).unwrap();
        worker.register("fixture", Arc::new(FixtureEngine)).unwrap();
        assert!(matches!(worker.run(&job.id), Err(Error::Cancelled)));
        assert_eq!(
            repository.get_job(&job.id).unwrap().generation,
            reconciled.generation
        );
    }

    #[test]
    fn a_live_worker_lock_keeps_its_expired_lease_and_reservation_until_it_exits() {
        let directory = tempfile::tempdir().unwrap();
        let repository =
            TrainingRepository::open(directory.path().join("training.sqlite")).unwrap();
        let stalled = job(&repository);
        let next = job_in(&repository, "next");
        let resource = ResourceRequest {
            key: "cpu:0".into(),
            estimated_bytes: 600,
            budget_bytes: 1000,
        };
        repository
            .claim_job_with_resources(
                &stalled.id,
                "stalled-worker",
                1000,
                now_ms() - 2000,
                Some(resource.clone()),
            )
            .unwrap();
        let held = LocalRunLock::acquire(&repository, &format!("job:{}", stalled.id)).unwrap();
        assert_eq!(
            repository.request_cancel(&stalled.id, now_ms()).unwrap().status,
            JobStatus::CancelRequested
        );
        assert!(
            repository
                .recover_expired_jobs(&stalled.stream, now_ms())
                .unwrap()
                .is_empty()
        );
        assert!(matches!(
            repository.claim_job_with_resources(&next.id, "next-worker", 1000, now_ms(), Some(resource.clone())),
            Err(Error::Conflict(message)) if message.contains(&stalled.id)
        ));
        drop(held);
        repository
            .claim_job_with_resources(&next.id, "next-worker", 1000, now_ms(), Some(resource))
            .unwrap();
        let settled = repository.get_job(&stalled.id).unwrap();
        assert_eq!(settled.status, JobStatus::Cancelled);
        assert!(settled.lease_expires_at_ms.is_none());
    }

    #[test]
    fn device_memory_exhaustion_names_the_remedy() {
        for message in [
            "CUDA_ERROR_OUT_OF_MEMORY",
            "called `Result::unwrap()` on an `Err` value: OutOfMemory { size: 1073741824 }",
            "An out-of-memory error happened during launch\nCaused by:\n  out of memory",
            "can't allocate buffer of size: 4294967296",
        ] {
            let Error::Engine(reason) =
                name_device_memory_exhaustion(Error::Engine(message.into()))
            else {
                panic!("engine error expected");
            };
            assert!(
                reason.starts_with("out of device memory; reduce the batch size"),
                "{reason}"
            );
            assert!(!reason.contains('\n'));
        }
        assert!(matches!(
            name_device_memory_exhaustion(Error::Engine("training duration budget exhausted".into())),
            Error::Engine(message) if message == "training duration budget exhausted"
        ));
        assert!(matches!(
            name_device_memory_exhaustion(Error::LeaseLost),
            Error::LeaseLost
        ));
    }
}
