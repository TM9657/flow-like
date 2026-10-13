use crate::worker::LocalRunLock;
use crate::*;
use rusqlite::{
    Connection, OptionalExtension, Transaction, TransactionBehavior, config::DbConfig, params,
};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

/// Writers share it from staging a blob until its reference commits; the collector takes
/// it exclusively, so a staged blob is never mistaken for an unreferenced one.
const BLOB_STORE_LOCK: &str = "blob-store";

/// Checkpoints that a resume or a successive-halving continuation can still read.
const LIVE_CHECKPOINTS: &str = "SELECT json_extract(j.body,'$.checkpoint.blob.path'),NULL FROM jobs j LEFT JOIN experiment_trials t ON t.job_id=j.id WHERE CASE WHEN t.experiment_id IS NULL THEN j.status<>'succeeded' ELSE j.status IN ('queued','running','cancel_requested','paused','interrupted') OR t.experiment_id IN (SELECT id FROM experiments WHERE json_extract(body,'$.selected_trial_id') IS NULL AND json_extract(body,'$.status') IN ('running','cancel_requested','cancelled')) END";

/// How many `eligible_samples` (?1 scope, ?2 time) lie outside `triggered_samples`. Consumed
/// samples are dropped first; the annotation_label_availability index answers the label-time
/// lookup, so no annotation body is read.
const NEW_SAMPLE_COUNT: &str = "WITH pending AS MATERIALIZED (SELECT a.sample_id FROM accepted a WHERE a.scope=?1 AND NOT EXISTS(SELECT 1 FROM triggered_samples t WHERE t.scope=?1 AND t.sample_id=a.sample_id)) SELECT count(*) FROM pending p WHERE (SELECT n.accepted FROM annotations n WHERE n.scope=?1 AND n.sample_id=p.sample_id AND json_extract(n.body,'$.label_available_at_ms')<=?2 ORDER BY n.revision DESC LIMIT 1)=1";

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS ml_meta(version INTEGER NOT NULL);
INSERT INTO ml_meta(version) SELECT 1 WHERE NOT EXISTS(SELECT 1 FROM ml_meta);
CREATE TABLE IF NOT EXISTS streams(scope TEXT PRIMARY KEY, next_sequence INTEGER NOT NULL DEFAULT 0, last_trigger_sequence INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS annotations(scope TEXT NOT NULL, sample_id TEXT NOT NULL, revision INTEGER NOT NULL, accepted INTEGER NOT NULL, body TEXT NOT NULL, PRIMARY KEY(scope,sample_id,revision));
CREATE INDEX IF NOT EXISTS annotation_label_availability ON annotations(scope,sample_id,revision,accepted,json_extract(body,'$.label_available_at_ms'));
CREATE TABLE IF NOT EXISTS accepted(scope TEXT NOT NULL, sample_id TEXT NOT NULL, sequence INTEGER NOT NULL, PRIMARY KEY(scope,sample_id), UNIQUE(scope,sequence));
CREATE TABLE IF NOT EXISTS snapshots(id TEXT PRIMARY KEY, scope TEXT NOT NULL, body TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS triggered_samples(scope TEXT NOT NULL, sample_id TEXT NOT NULL, snapshot_id TEXT NOT NULL, PRIMARY KEY(scope,sample_id));
CREATE TABLE IF NOT EXISTS jobs(id TEXT PRIMARY KEY, scope TEXT NOT NULL, status TEXT NOT NULL, body TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS resource_reservations(job_id TEXT PRIMARY KEY, resource_key TEXT NOT NULL, bytes INTEGER NOT NULL);
CREATE UNIQUE INDEX IF NOT EXISTS one_active_training ON jobs(scope) WHERE status IN ('queued','running','cancel_requested','paused','interrupted');
CREATE TABLE IF NOT EXISTS artifacts(id TEXT PRIMARY KEY, scope TEXT NOT NULL, body TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS pretrained_sources(id TEXT PRIMARY KEY, project_id TEXT NOT NULL, body TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS predictions(artifact_id TEXT NOT NULL, sample_id TEXT NOT NULL, body TEXT NOT NULL, PRIMARY KEY(artifact_id,sample_id));
CREATE TABLE IF NOT EXISTS evaluations(id TEXT PRIMARY KEY, artifact_id TEXT NOT NULL, body TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS deployments(id TEXT PRIMARY KEY, scope TEXT NOT NULL, generation INTEGER NOT NULL, body TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS deployment_history(id TEXT NOT NULL, generation INTEGER NOT NULL, body TEXT NOT NULL, PRIMARY KEY(id,generation));
";

/// Cloneable handle. Each operation opens its own connection, so transaction tests cover
/// competing processes as well as threads. Artifacts live beside the SQLite ledger.
#[derive(Clone, Debug)]
pub struct TrainingRepository {
    path: PathBuf,
    blob_dir: PathBuf,
    /// Canonical directory of the ledger, as resolved when it was opened.
    root: PathBuf,
    shadow: bool,
}

impl TrainingRepository {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_owned();
        if path.as_os_str().is_empty() || path == Path::new(":memory:") {
            return Err(invalid("a persistent SQLite file is required"));
        }
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent)?;
        let root = parent.canonicalize()?;
        let blob_dir = path.with_extension("artifacts");
        std::fs::create_dir_all(&blob_dir)?;
        let repo = Self {
            path,
            blob_dir,
            root,
            shadow: false,
        };
        repo.verify_storage()?;
        let mut connection = repo.connection()?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute_batch(SCHEMA)?;
        crate::experiment_repository::initialize_experiments(&tx)?;
        crate::learning_repository::initialize_learning(&tx)?;
        let version: i64 = tx.query_row("SELECT version FROM ml_meta", [], |r| r.get(0))?;
        if version != 1 {
            return Err(invalid(format!(
                "unsupported training ledger version {version}"
            )));
        }
        // After migrations, so they may drop obsolete objects; a rejection rolls them back.
        verify_schema_objects(&tx)?;
        tx.commit()?;
        if let Ok(exclusive) = LocalRunLock::acquire(&repo, BLOB_STORE_LOCK) {
            repo.sweep_blob_store(&exclusive, None);
        }
        Ok(repo)
    }

    /// A shadow handle can read and record predictions. All lifecycle/data mutations fail.
    pub fn shadow_telemetry(&self) -> Self {
        Self {
            shadow: true,
            ..self.clone()
        }
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Checks that the ledger, its blob and lock directories and its journal files are still
    /// real entries of the ledger's directory. Hosts that reuse an open handle can call it
    /// instead of reopening; blob writes and sweeps check their own directory every time.
    pub fn verify_storage(&self) -> Result<()> {
        verify_storage_entries(&self.root, &self.path, &self.blob_dir)
    }
    pub(crate) fn writable(&self) -> Result<()> {
        if self.shadow {
            Err(Error::ShadowWrite)
        } else {
            Ok(())
        }
    }
    pub(crate) fn connection(&self) -> Result<Connection> {
        let connection = Connection::open(&self.path)?;
        connection.set_db_config(DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true)?;
        connection.set_db_config(DbConfig::SQLITE_DBCONFIG_TRUSTED_SCHEMA, false)?;
        connection.busy_timeout(Duration::from_secs(15))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        Ok(connection)
    }

    pub fn record_sample(
        &self,
        stream: &StreamKey,
        sample: &TrainingSample,
    ) -> Result<SampleReceipt> {
        self.writable()?;
        validate_sample(sample)?;
        let payload = sample.payload.get("sample").unwrap_or(&sample.payload);
        if payload
            .get("stream_id")
            .is_some_and(|value| value.as_str() != Some(&stream.stream_id))
        {
            return Err(invalid(
                "sample payload stream differs from repository scope",
            ));
        }
        let scope = scope(stream)?;
        let body = serde_json::to_string(sample)?;
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute("INSERT OR IGNORE INTO streams(scope) VALUES (?)", [&scope])?;
        let existing: Option<String> = tx
            .query_row(
                "SELECT body FROM annotations WHERE scope=? AND sample_id=? AND revision=?",
                params![scope, sample.id, sample.annotation_revision],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(existing) = existing {
            if existing != body {
                return Err(Error::Conflict(
                    "an annotation revision is immutable".into(),
                ));
            }
            let sequence = accepted_sequence(&tx, &scope, &sample.id)?;
            return Ok(SampleReceipt {
                inserted: false,
                accepted_sequence: sequence,
            });
        }
        let prior: Option<String> = tx.query_row(
            "SELECT body FROM annotations WHERE scope=? AND sample_id=? ORDER BY revision DESC LIMIT 1",
            params![scope, sample.id], |r| r.get(0)).optional()?;
        if let Some(prior) = prior {
            let prior: TrainingSample = serde_json::from_str(&prior)?;
            if sample.annotation_revision <= prior.annotation_revision
                || sample.content_digest != prior.content_digest
                || sample.group_id != prior.group_id
                || sample.captured_at_ms != prior.captured_at_ms
            {
                return Err(Error::Conflict(
                    "revisions must increase and preserve sample content, group and capture time"
                        .into(),
                ));
            }
        }
        tx.execute(
            "INSERT INTO annotations(scope,sample_id,revision,accepted,body) VALUES (?,?,?,?,?)",
            params![
                scope,
                sample.id,
                sample.annotation_revision,
                sample.accepted,
                body
            ],
        )?;
        let mut sequence = accepted_sequence(&tx, &scope, &sample.id)?;
        if sample.accepted && sequence.is_none() {
            tx.execute(
                "UPDATE streams SET next_sequence=next_sequence+1 WHERE scope=?",
                [&scope],
            )?;
            let next: i64 = tx.query_row(
                "SELECT next_sequence FROM streams WHERE scope=?",
                [&scope],
                |r| r.get(0),
            )?;
            tx.execute(
                "INSERT INTO accepted(scope,sample_id,sequence) VALUES (?,?,?)",
                params![scope, sample.id, next],
            )?;
            sequence = Some(next);
        }
        tx.commit()?;
        Ok(SampleReceipt {
            inserted: true,
            accepted_sequence: sequence,
        })
    }

    pub fn snapshot(
        &self,
        stream: &StreamKey,
        policy: SplitPolicy,
        as_of_ms: i64,
    ) -> Result<DatasetSnapshot> {
        self.writable()?;
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let snapshot = create_snapshot(&tx, stream, policy, as_of_ms)?;
        tx.commit()?;
        Ok(snapshot)
    }

    pub fn get_snapshot(&self, snapshot_id: &str) -> Result<DatasetSnapshot> {
        self.get_snapshot_limited(snapshot_id, u64::MAX)
    }

    /// SQLite checks UTF-8 byte length before returning the JSON for deserialization.
    pub fn get_snapshot_limited(
        &self,
        snapshot_id: &str,
        maximum_bytes: u64,
    ) -> Result<DatasetSnapshot> {
        let limit = maximum_bytes.min(i64::MAX as u64) as i64;
        let result: Option<Option<String>> = self.connection()?.query_row(
            "SELECT CASE WHEN length(CAST(body AS BLOB)) <= ? THEN body ELSE NULL END FROM snapshots WHERE id=?",
            params![limit, snapshot_id],
            |row| row.get(0),
        ).optional()?;
        let body = result
            .ok_or_else(|| Error::NotFound(snapshot_id.into()))?
            .ok_or_else(|| {
                invalid(format!(
                    "serialized snapshot exceeds the {maximum_bytes} byte budget"
                ))
            })?;
        Ok(serde_json::from_str(&body)?)
    }

    /// Samples Train After N has not consumed yet: accepted, with an accepted latest revision
    /// whose label is available at `as_of_ms`, and outside every earlier trigger. Label times
    /// come from an index, so no sample body is read.
    pub fn count_new_training_samples(&self, stream: &StreamKey, as_of_ms: i64) -> Result<usize> {
        count_new_samples(&self.connection()?, &scope(stream)?, as_of_ms)
    }

    /// Counts distinct accepted samples since the last trigger. Reserving the watermark,
    /// immutable snapshot and queued job is one transaction. Failure is retried by resume_job.
    pub fn trigger_after_n(
        &self,
        stream: &StreamKey,
        n: usize,
        request: TrainingRequest,
        policy: SplitPolicy,
        at_ms: i64,
    ) -> Result<Option<TrainingJob>> {
        self.writable()?;
        if n == 0 {
            return Err(invalid("N must be positive"));
        }
        if request.engine.trim().is_empty()
            || !request.recipe.is_object()
            || !request.compute.is_object()
        {
            return Err(invalid(
                "engine and object-valued recipe/compute are required",
            ));
        }
        let scope = scope(stream)?;
        let mut connection = self.connection()?;
        // A stream short of N new samples returns before taking the write lock.
        if count_new_samples(&connection, &scope, at_ms)? < n {
            return Ok(None);
        }
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let active: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM jobs WHERE scope=? AND status IN ('queued','running','cancel_requested','paused','interrupted'))", [&scope], |r| r.get(0))?;
        if active || count_new_samples(&tx, &scope, at_ms)? < n {
            return Ok(None);
        }
        let samples = eligible_samples(&tx, &scope, at_ms)?;
        if !partitions_ready(&samples, &policy)? {
            return Ok(None);
        }
        let consumed: Vec<String> = samples
            .iter()
            .map(|(_, sample)| sample.id.clone())
            .collect();
        let mut snapshot = snapshot_from_samples(&tx, stream, policy, at_ms, samples)?;
        if normal_only(&request.recipe) {
            for partition in [&mut snapshot.train, &mut snapshot.validation] {
                partition.retain(|sample| {
                    if is_normal(sample) {
                        true
                    } else {
                        snapshot.excluded.push(sample.id.clone());
                        false
                    }
                });
            }
            if snapshot.train.is_empty() || snapshot.validation.is_empty() {
                return Ok(None);
            }
            snapshot.excluded.sort();
            snapshot.digest = snapshot_digest(&snapshot)?;
            tx.execute(
                "UPDATE snapshots SET body=? WHERE id=?",
                params![serde_json::to_string(&snapshot)?, snapshot.id],
            )?;
        }
        if snapshot.train.is_empty() {
            return Err(invalid("training partition is empty"));
        }
        if !training_ready(&snapshot, &request.recipe)? {
            return Ok(None);
        }
        let job = TrainingJob {
            id: id(),
            stream: stream.clone(),
            snapshot_id: snapshot.id,
            request,
            status: JobStatus::Queued,
            generation: 0,
            lease_owner: None,
            lease_expires_at_ms: None,
            checkpoint: None,
            artifact_id: None,
            error: None,
            progress: Value::Null,
            created_at_ms: at_ms,
            updated_at_ms: at_ms,
        };
        tx.execute(
            "INSERT INTO jobs(id,scope,status,body) VALUES (?,?,?,?)",
            params![
                job.id,
                scope,
                status_name(job.status),
                serde_json::to_string(&job)?
            ],
        )?;
        tx.execute(
            "UPDATE streams SET last_trigger_sequence=? WHERE scope=?",
            params![snapshot.cutoff_sequence, scope],
        )?;
        for sample_id in &consumed {
            tx.execute("INSERT OR IGNORE INTO triggered_samples(scope,sample_id,snapshot_id) VALUES (?,?,?)",params![scope,sample_id,job.snapshot_id])?;
        }
        tx.commit()?;
        Ok(Some(job))
    }

    pub fn get_job(&self, job_id: &str) -> Result<TrainingJob> {
        read_json(
            &self.connection()?,
            "SELECT body FROM jobs WHERE id=?",
            job_id,
        )
    }

    pub fn list_jobs(&self, stream: &StreamKey) -> Result<Vec<TrainingJob>> {
        let conn = self.connection()?;
        let mut stmt = conn.prepare("SELECT body FROM jobs WHERE scope=? ORDER BY rowid")?;
        let rows = stmt.query_map([scope(stream)?], |row| row.get::<_, String>(0))?;
        rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
    }

    pub fn claim_job(
        &self,
        job_id: &str,
        owner: &str,
        lease_ms: i64,
        at_ms: i64,
    ) -> Result<JobLease> {
        self.claim_job_with_resources(job_id, owner, lease_ms, at_ms, None)
    }

    /// Resource reservations are shared by every worker using this ledger. Interrupted
    /// jobs retain reservations until their supervisor reconciles and resumes/cancels them.
    /// A claim that finds the budget occupied first settles holders whose lease expired and
    /// whose job lock is free, because their worker process is gone.
    pub fn claim_job_with_resources(
        &self,
        job_id: &str,
        owner: &str,
        lease_ms: i64,
        at_ms: i64,
        resources: Option<ResourceRequest>,
    ) -> Result<JobLease> {
        self.claim_job_state(job_id, owner, lease_ms, at_ms, resources)
            .map(|(lease, _)| lease)
    }

    /// Like `claim_job_with_resources`, also returning the job as the claim committed it.
    pub(crate) fn claim_job_state(
        &self,
        job_id: &str,
        owner: &str,
        lease_ms: i64,
        at_ms: i64,
        resources: Option<ResourceRequest>,
    ) -> Result<(JobLease, TrainingJob)> {
        self.writable()?;
        if owner.is_empty() || lease_ms <= 0 {
            return Err(invalid("owner and positive lease duration are required"));
        }
        if resources.as_ref().is_some_and(|resources| {
            resources.key.is_empty()
                || resources.budget_bytes == 0
                || resources.estimated_bytes > i64::MAX as u64
        }) {
            return Err(invalid(
                "resource reservation needs a key and a positive budget",
            ));
        }
        let mut reclaimed = false;
        loop {
            match self.claim_once(job_id, owner, lease_ms, at_ms, resources.as_ref())? {
                Ok(claimed) => return Ok(claimed),
                Err(occupied)
                    if !reclaimed && self.reclaim_stopped_holders(&occupied.holders, at_ms) =>
                {
                    reclaimed = true;
                }
                Err(occupied) => return Err(Error::Conflict(occupied.message)),
            }
        }
    }

    fn claim_once(
        &self,
        job_id: &str,
        owner: &str,
        lease_ms: i64,
        at_ms: i64,
        resources: Option<&ResourceRequest>,
    ) -> Result<std::result::Result<(JobLease, TrainingJob), Occupied>> {
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut job: TrainingJob = read_json(&tx, "SELECT body FROM jobs WHERE id=?", job_id)?;
        if job.status != JobStatus::Queued {
            return Err(Error::Conflict(format!(
                "job is {:?}, expected queued",
                job.status
            )));
        }
        if let Some(resources) = resources {
            let mut query = tx.prepare(
                "SELECT job_id,bytes FROM resource_reservations WHERE resource_key=? AND job_id<>? ORDER BY job_id",
            )?;
            let holders = query
                .query_map(params![resources.key, job_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            drop(query);
            let used = holders
                .iter()
                .fold(0u64, |used, (_, bytes)| used.saturating_add(*bytes));
            if used.saturating_add(resources.estimated_bytes) > resources.budget_bytes {
                let holders: Vec<String> = holders.into_iter().map(|(holder, _)| holder).collect();
                return Ok(Err(Occupied {
                    message: format!(
                        "worker resource budget {} is occupied: job(s) {} reserve {used} of {} bytes and this job needs {}",
                        resources.key,
                        holders.join(", "),
                        resources.budget_bytes,
                        resources.estimated_bytes
                    ),
                    holders,
                }));
            }
            tx.execute("INSERT INTO resource_reservations(job_id,resource_key,bytes) VALUES (?,?,?) ON CONFLICT(job_id) DO UPDATE SET resource_key=excluded.resource_key,bytes=excluded.bytes",params![job_id,resources.key,resources.estimated_bytes])?;
        }
        job.status = JobStatus::Running;
        job.generation += 1;
        job.lease_owner = Some(owner.into());
        job.lease_expires_at_ms = Some(
            at_ms
                .checked_add(lease_ms)
                .ok_or_else(|| invalid("lease overflow"))?,
        );
        job.error = None;
        job.updated_at_ms = at_ms;
        save_job(&tx, &job)?;
        tx.commit()?;
        let lease = JobLease {
            job_id: job.id.clone(),
            owner: owner.into(),
            generation: job.generation,
        };
        Ok(Ok((lease, job)))
    }

    fn reclaim_stopped_holders(&self, holders: &[String], at_ms: i64) -> bool {
        let mut reclaimed = false;
        for holder in holders {
            let Ok(job) = self.get_job(holder) else {
                continue;
            };
            if !lease_expired(&job, at_ms) {
                continue;
            }
            let Some(_stopped) = self.stopped_worker_lock(holder) else {
                continue;
            };
            reclaimed |= self
                .reconcile_stopped_job(holder, at_ms)
                .is_ok_and(|settled| settled.generation != job.generation);
        }
        reclaimed
    }

    /// Acquiring a job's lock proves that no worker process still runs it.
    fn stopped_worker_lock(&self, job_id: &str) -> Option<LocalRunLock> {
        LocalRunLock::acquire(self, &format!("job:{job_id}")).ok()
    }

    pub fn pending_jobs(&self) -> Result<Vec<TrainingJob>> {
        let connection = self.connection()?;
        let mut query =
            connection.prepare("SELECT body FROM jobs WHERE status='queued' ORDER BY rowid")?;
        query
            .query_map([], |r| r.get::<_, String>(0))?
            .map(|row| Ok(serde_json::from_str(&row?)?))
            .collect()
    }

    /// Does not launch or resume expired work. A supervisor can enumerate this list after
    /// restart and confirm the old process stopped before mark_interrupted/resume_job.
    pub fn expired_jobs(&self, at_ms: i64) -> Result<Vec<TrainingJob>> {
        let connection = self.connection()?;
        let mut query = connection.prepare(
            "SELECT body FROM jobs WHERE status IN ('running','cancel_requested') ORDER BY rowid",
        )?;
        let jobs: Vec<TrainingJob> = query
            .query_map([], |r| r.get::<_, String>(0))?
            .map(|row| Ok(serde_json::from_str(&row?)?))
            .collect::<Result<_>>()?;
        Ok(jobs
            .into_iter()
            .filter(|job| {
                job.lease_expires_at_ms
                    .is_some_and(|deadline| deadline <= at_ms)
            })
            .collect())
    }

    pub fn heartbeat(
        &self,
        lease: &JobLease,
        lease_ms: i64,
        progress: Value,
        at_ms: i64,
    ) -> Result<()> {
        self.renew_lease(lease, lease_ms, Some(progress), at_ms)
    }

    /// Extends the lease and replaces progress only when a value is given, so a renewal
    /// never writes back progress it read earlier.
    pub(crate) fn renew_lease(
        &self,
        lease: &JobLease,
        lease_ms: i64,
        progress: Option<Value>,
        at_ms: i64,
    ) -> Result<()> {
        self.writable()?;
        if lease_ms <= 0 {
            return Err(invalid("lease duration must be positive"));
        }
        self.update_leased(lease, at_ms, |job| {
            job.lease_expires_at_ms = Some(
                at_ms
                    .checked_add(lease_ms)
                    .ok_or_else(|| invalid("lease overflow"))?,
            );
            if let Some(progress) = progress {
                job.progress = progress;
            }
            Ok(())
        })
    }

    /// Reads the status column without parsing the job body.
    pub(crate) fn job_status(&self, job_id: &str) -> Result<JobStatus> {
        let status: String = self
            .connection()?
            .query_row("SELECT status FROM jobs WHERE id=?", [job_id], |row| {
                row.get(0)
            })
            .optional()?
            .ok_or_else(|| Error::NotFound(job_id.into()))?;
        Ok(serde_json::from_value(Value::String(status))?)
    }

    /// Keeps a failure that happened before the claim visible on a job that is still waiting
    /// to run. The status is unchanged and the next successful claim clears the error.
    pub fn record_job_attempt_error(&self, job_id: &str, error: &str, at_ms: i64) -> Result<()> {
        self.writable()?;
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut job: TrainingJob = read_json(&tx, "SELECT body FROM jobs WHERE id=?", job_id)?;
        if !matches!(
            job.status,
            JobStatus::Queued | JobStatus::Paused | JobStatus::Interrupted
        ) || job.error.as_deref() == Some(error)
        {
            return Ok(());
        }
        job.error = Some(error.into());
        job.updated_at_ms = at_ms;
        save_job(&tx, &job)?;
        tx.commit()?;
        Ok(())
    }

    /// Settles this stream's jobs whose lease expired with their worker: a requested
    /// cancellation becomes Cancelled and a run becomes Interrupted, releasing reservations.
    /// Jobs whose worker still holds its lock are left alone. Returns the settled jobs.
    pub fn recover_expired_jobs(&self, stream: &StreamKey, at_ms: i64) -> Result<Vec<TrainingJob>> {
        self.writable()?;
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT body FROM jobs WHERE scope=? AND status IN ('running','cancel_requested') ORDER BY rowid",
        )?;
        let expired: Vec<TrainingJob> = statement
            .query_map([scope(stream)?], |row| row.get::<_, String>(0))?
            .map(|row| Ok(serde_json::from_str(&row?)?))
            .collect::<Result<_>>()?;
        let mut recovered = Vec::new();
        for job in expired {
            if !lease_expired(&job, at_ms) {
                continue;
            }
            let Some(_stopped) = self.stopped_worker_lock(&job.id) else {
                continue;
            };
            let settled = self.reconcile_stopped_job(&job.id, at_ms)?;
            if settled.generation != job.generation {
                recovered.push(settled);
            }
        }
        Ok(recovered)
    }

    /// A job whose lease expired with its worker is settled as Cancelled directly; a live
    /// worker sees CancelRequested and stops itself.
    pub fn request_cancel(&self, job_id: &str, at_ms: i64) -> Result<TrainingJob> {
        self.writable()?;
        let stopped = lease_expired(&self.get_job(job_id)?, at_ms)
            .then(|| self.stopped_worker_lock(job_id))
            .flatten();
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut job: TrainingJob = read_json(&tx, "SELECT body FROM jobs WHERE id=?", job_id)?;
        if stopped.is_some() && lease_expired(&job, at_ms) {
            settle_stopped(&mut job, JobStatus::Cancelled, at_ms);
        } else {
            job.status = match job.status {
                JobStatus::Running | JobStatus::CancelRequested => JobStatus::CancelRequested,
                JobStatus::Queued | JobStatus::Paused | JobStatus::Interrupted => {
                    JobStatus::Cancelled
                }
                status => status,
            };
            job.updated_at_ms = at_ms;
        }
        save_job(&tx, &job)?;
        tx.commit()?;
        Ok(job)
    }

    pub fn acknowledge_cancel(&self, lease: &JobLease, at_ms: i64) -> Result<()> {
        self.writable()?;
        self.update_leased(lease, at_ms, |job| {
            if job.status != JobStatus::CancelRequested {
                return Err(Error::Conflict("cancellation was not requested".into()));
            }
            job.status = JobStatus::Cancelled;
            job.lease_expires_at_ms = None;
            Ok(())
        })
    }

    /// Fences an expired worker. The supervisor must stop/reconcile that worker before resuming.
    pub fn mark_interrupted(
        &self,
        job_id: &str,
        expected_generation: i64,
        at_ms: i64,
    ) -> Result<()> {
        self.writable()?;
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut job: TrainingJob = read_json(&tx, "SELECT body FROM jobs WHERE id=?", job_id)?;
        if job.generation != expected_generation
            || !matches!(job.status, JobStatus::Running | JobStatus::CancelRequested)
            || job
                .lease_expires_at_ms
                .is_none_or(|deadline| deadline > at_ms)
        {
            return Err(Error::Conflict(
                "worker lease is current or generation changed".into(),
            ));
        }
        job.status = JobStatus::Interrupted;
        job.generation += 1;
        job.lease_owner = None;
        job.lease_expires_at_ms = None;
        job.updated_at_ms = at_ms;
        save_job(&tx, &job)?;
        tx.commit()?;
        Ok(())
    }

    /// A job whose lease expired with its worker is reconciled first, so it resumes too.
    pub fn resume_job(&self, job_id: &str, at_ms: i64) -> Result<TrainingJob> {
        self.writable()?;
        if lease_expired(&self.get_job(job_id)?, at_ms)
            && let Some(_stopped) = self.stopped_worker_lock(job_id)
        {
            self.reconcile_stopped_job(job_id, at_ms)?;
        }
        self.resume_job_with_policy(job_id, at_ms, false)
    }

    /// The caller holds the job's operating-system lock, proving that no old worker runs.
    pub(crate) fn reconcile_stopped_job(&self, job_id: &str, at_ms: i64) -> Result<TrainingJob> {
        self.writable()?;
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut job: TrainingJob = read_json(&tx, "SELECT body FROM jobs WHERE id=?", job_id)?;
        if lease_expired(&job, at_ms) {
            let status = if job.status == JobStatus::CancelRequested {
                JobStatus::Cancelled
            } else {
                JobStatus::Interrupted
            };
            settle_stopped(&mut job, status, at_ms);
            save_job(&tx, &job)?;
            tx.execute("DELETE FROM resource_reservations WHERE job_id=?", [job_id])?;
        }
        tx.commit()?;
        Ok(job)
    }

    pub(crate) fn resume_stopped_job(&self, job_id: &str, at_ms: i64) -> Result<TrainingJob> {
        self.resume_job_with_policy(job_id, at_ms, true)
    }

    fn resume_job_with_policy(
        &self,
        job_id: &str,
        at_ms: i64,
        stopped_only: bool,
    ) -> Result<TrainingJob> {
        self.writable()?;
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut job: TrainingJob = read_json(&tx, "SELECT body FROM jobs WHERE id=?", job_id)?;
        let stopped = matches!(job.status, JobStatus::Paused | JobStatus::Interrupted);
        if !stopped
            && (stopped_only || !matches!(job.status, JobStatus::Cancelled | JobStatus::Failed))
        {
            return Err(Error::Conflict(
                "only paused/interrupted/cancelled/failed jobs can resume".into(),
            ));
        }
        if let Some(reason) =
            crate::experiment_repository::reserve_recovered_trial(&tx, &job, at_ms)?
        {
            job.status = JobStatus::Failed;
            job.generation += 1;
            job.lease_owner = None;
            job.lease_expires_at_ms = None;
            job.error = Some(reason.clone());
            job.updated_at_ms = at_ms;
            save_job(&tx, &job)?;
            tx.commit()?;
            return Err(invalid(reason));
        }
        job.status = JobStatus::Queued;
        job.generation += 1;
        job.lease_owner = None;
        job.lease_expires_at_ms = None;
        job.error = None;
        job.updated_at_ms = at_ms;
        save_job(&tx, &job)?;
        tx.commit()?;
        Ok(job)
    }

    /// Replaces the compute settings of a lifecycle job that waits or stopped, so its next
    /// admission uses them. Its status is kept, and a checkpoint written for the previous
    /// request stays resumable.
    pub fn reconfigure_job_compute(
        &self,
        job_id: &str,
        compute: Value,
        at_ms: i64,
    ) -> Result<TrainingJob> {
        self.writable()?;
        if !compute.is_object() {
            return Err(invalid("job compute settings must be a JSON object"));
        }
        // A worker admitting a queued job holds its lock and would run the request it read.
        let Some(_idle) = self.stopped_worker_lock(job_id) else {
            return Err(Error::Conflict(format!(
                "a worker is starting or running job {job_id}; change its compute after it stops"
            )));
        };
        let mut connection = self.connection()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut job: TrainingJob = read_json(&tx, "SELECT body FROM jobs WHERE id=?", job_id)?;
        if experiment_trial(&tx, job_id)? {
            return Err(Error::Conflict(format!(
                "job {job_id} is an experiment trial; its experiment sets its compute"
            )));
        }
        if !matches!(
            job.status,
            JobStatus::Queued | JobStatus::Paused | JobStatus::Interrupted
        ) {
            return Err(Error::Conflict(format!(
                "job {job_id} is {:?}; only queued, paused or interrupted jobs can change compute",
                job.status
            )));
        }
        rewrite_job_request(&mut job, |request| {
            request.compute = compute;
            Ok(())
        })?;
        job.generation += 1;
        job.error = None;
        job.updated_at_ms = at_ms;
        save_job(&tx, &job)?;
        tx.commit()?;
        Ok(job)
    }

    pub fn fail_job(&self, lease: &JobLease, error: impl Into<String>, at_ms: i64) -> Result<()> {
        self.writable()?;
        self.update_leased(lease, at_ms, |job| {
            settle_failed(job, error.into());
            Ok(())
        })
    }

    /// A run that exhausted device memory after publishing a checkpoint is Interrupted with
    /// its reservation released, so it resumes from that checkpoint. Runs without a checkpoint
    /// fail like `fail_job`, as do experiment trials: their controller fails the trial anyway
    /// and would charge an Interrupted attempt its whole reserved training slice.
    pub(crate) fn settle_out_of_memory(
        &self,
        lease: &JobLease,
        error: &str,
        at_ms: i64,
    ) -> Result<()> {
        self.writable()?;
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut job: TrainingJob =
            read_json(&tx, "SELECT body FROM jobs WHERE id=?", &lease.job_id)?;
        validate_lease(&job, lease, at_ms)?;
        if job.status == JobStatus::Running
            && job.checkpoint.is_some()
            && !experiment_trial(&tx, &job.id)?
        {
            settle_stopped(&mut job, JobStatus::Interrupted, at_ms);
            job.error = Some(error.into());
            tx.execute(
                "DELETE FROM resource_reservations WHERE job_id=?",
                [&job.id],
            )?;
        } else {
            settle_failed(&mut job, error.into());
            job.updated_at_ms = at_ms;
        }
        save_job(&tx, &job)?;
        tx.commit()?;
        Ok(())
    }

    fn update_leased(
        &self,
        lease: &JobLease,
        at_ms: i64,
        update: impl FnOnce(&mut TrainingJob) -> Result<()>,
    ) -> Result<()> {
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut job: TrainingJob =
            read_json(&tx, "SELECT body FROM jobs WHERE id=?", &lease.job_id)?;
        validate_lease(&job, lease, at_ms)?;
        update(&mut job)?;
        job.updated_at_ms = at_ms;
        save_job(&tx, &job)?;
        tx.commit()?;
        Ok(())
    }

    /// Writes bytes before publishing their reference, so readers never discover a partially
    /// written checkpoint through the ledger. The replaced checkpoint is left to the collector.
    pub fn publish_checkpoint(
        &self,
        lease: &JobLease,
        bytes: &[u8],
        step: u64,
        metadata: Value,
        pause: bool,
        at_ms: i64,
    ) -> Result<CheckpointRef> {
        self.writable()?;
        if !metadata.is_object() {
            return Err(invalid(
                "checkpoint metadata must describe the engine state",
            ));
        }
        let job = self.get_job(&lease.job_id)?;
        validate_lease(&job, lease, at_ms)?;
        let writer = self.blob_writer()?;
        let blob = self.put_blob(&writer, bytes)?;
        let checkpoint = CheckpointRef {
            blob,
            step,
            metadata,
            request_digest: digest(&serde_json::to_vec(&job.request)?),
        };
        self.update_leased(lease, at_ms, |job| {
            if job.checkpoint.as_ref().is_some_and(|old| old.step > step) {
                return Err(Error::Conflict(
                    "checkpoint steps cannot go backwards".into(),
                ));
            }
            if pause && job.status == JobStatus::CancelRequested {
                return Err(Error::Cancelled);
            }
            job.checkpoint = Some(checkpoint.clone());
            if pause {
                job.status = JobStatus::Paused;
                job.lease_expires_at_ms = None;
            }
            Ok(())
        })?;
        Ok(checkpoint)
    }

    pub fn finish_training(
        &self,
        lease: &JobLease,
        bytes: &[u8],
        manifest: Value,
        at_ms: i64,
    ) -> Result<ModelArtifact> {
        self.writable()?;
        if !manifest.is_object() {
            return Err(invalid("model manifest must be an object"));
        }
        let job = self.get_job(&lease.job_id)?;
        validate_lease(&job, lease, at_ms)?;
        if job.status == JobStatus::CancelRequested {
            return Err(Error::Cancelled);
        }
        let dataset_digest = self.stored_snapshot_digest(&job.snapshot_id)?;
        self.check_experiment_artifact_budget(&job.id, bytes.len() as u64)?;
        let writer = self.blob_writer()?;
        let blob = self.put_blob(&writer, bytes)?;
        let artifact = ModelArtifact {
            id: id(),
            job_id: job.id.clone(),
            stream: job.stream.clone(),
            snapshot_id: job.snapshot_id.clone(),
            dataset_digest,
            blob,
            manifest,
            created_at_ms: at_ms,
        };
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut job: TrainingJob =
            read_json(&tx, "SELECT body FROM jobs WHERE id=?", &lease.job_id)?;
        validate_lease(&job, lease, at_ms)?;
        if job.status == JobStatus::CancelRequested {
            return Err(Error::Cancelled);
        }
        tx.execute(
            "INSERT INTO artifacts(id,scope,body) VALUES (?,?,?)",
            params![
                artifact.id,
                scope(&artifact.stream)?,
                serde_json::to_string(&artifact)?
            ],
        )?;
        job.artifact_id = Some(artifact.id.clone());
        job.status = JobStatus::Succeeded;
        job.lease_expires_at_ms = None;
        job.updated_at_ms = at_ms;
        save_job(&tx, &job)?;
        tx.commit()?;
        Ok(artifact)
    }

    pub fn get_artifact(&self, artifact_id: &str) -> Result<ModelArtifact> {
        read_json(
            &self.connection()?,
            "SELECT body FROM artifacts WHERE id=?",
            artifact_id,
        )
    }

    pub fn register_pretrained_source(
        &self,
        project_id: &str,
        bytes: &[u8],
        manifest: Value,
        origin: Value,
        maximum_bytes: u64,
        at_ms: i64,
    ) -> Result<PretrainedSource> {
        self.writable()?;
        if project_id.trim().is_empty()
            || bytes.is_empty()
            || bytes.len() as u64 > maximum_bytes
            || !matches!(manifest["engine"].as_str(), Some("burn" | "sam2"))
            || manifest["format_version"].as_u64() != Some(1)
            || !manifest["config"].is_object()
            || !manifest["input_shape"].is_array()
            || !origin.is_object()
            || serde_json::to_vec(&(&manifest, &origin))?.len() > 1024 * 1024
        {
            return Err(invalid("invalid pretrained source metadata or byte budget"));
        }
        let writer = self.blob_writer()?;
        let source = PretrainedSource {
            id: format!("pretrained:{}", id()),
            project_id: project_id.into(),
            source_kind: PretrainedSourceKind::Imported,
            blob: self.put_blob(&writer, bytes)?,
            manifest,
            origin,
            created_at_ms: at_ms,
        };
        self.connection()?.execute(
            "INSERT INTO pretrained_sources(id,project_id,body) VALUES(?,?,?)",
            params![
                source.id,
                source.project_id,
                serde_json::to_string(&source)?
            ],
        )?;
        Ok(source)
    }

    pub fn get_pretrained_source(&self, source_id: &str) -> Result<PretrainedSource> {
        if source_id.starts_with("pretrained:") {
            return read_json(
                &self.connection()?,
                "SELECT body FROM pretrained_sources WHERE id=?",
                source_id,
            );
        }
        Ok(pretrained_artifact(self.get_artifact(source_id)?))
    }

    pub fn list_pretrained_sources(
        &self,
        project_id: &str,
        maximum: usize,
    ) -> Result<Vec<PretrainedSource>> {
        self.list_pretrained_sources_filtered(project_id, maximum, None)
    }

    /// Filter before limiting so other model families cannot hide compatible sources.
    pub fn list_pretrained_sources_for_engine(
        &self,
        project_id: &str,
        maximum: usize,
        engine: &str,
    ) -> Result<Vec<PretrainedSource>> {
        if !matches!(engine, "burn" | "sam2") {
            return Err(invalid("unsupported pretrained source engine"));
        }
        self.list_pretrained_sources_filtered(project_id, maximum, Some(engine))
    }

    fn list_pretrained_sources_filtered(
        &self,
        project_id: &str,
        maximum: usize,
        engine: Option<&str>,
    ) -> Result<Vec<PretrainedSource>> {
        if !(1..=128).contains(&maximum) {
            return Err(invalid("pretrained source list limit must be 1..128"));
        }
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT body FROM pretrained_sources WHERE project_id=?1 AND (?2 IS NULL OR json_extract(body,'$.manifest.engine')=?2) ORDER BY json_extract(body,'$.created_at_ms') DESC,id ASC LIMIT ?3",
        )?;
        let mut sources = statement
            .query_map(params![project_id, engine, maximum], |row| {
                row.get::<_, String>(0)
            })?
            .map(|row| Ok(serde_json::from_str::<PretrainedSource>(&row?)?))
            .collect::<Result<Vec<_>>>()?;
        if engine != Some("sam2") {
            let mut statement = connection.prepare(
                "SELECT body FROM artifacts WHERE json_extract(body,'$.stream.project_id')=? AND json_extract(body,'$.manifest.engine')='burn' ORDER BY json_extract(body,'$.created_at_ms') DESC,id ASC LIMIT ?",
            )?;
            for row in
                statement.query_map(params![project_id, maximum], |row| row.get::<_, String>(0))?
            {
                sources.push(pretrained_artifact(serde_json::from_str(&row?)?));
            }
        }
        sources.sort_by(|a, b| b.created_at_ms.cmp(&a.created_at_ms).then(a.id.cmp(&b.id)));
        sources.truncate(maximum);
        Ok(sources)
    }

    pub fn read_blob(&self, blob: &BlobRef) -> Result<Vec<u8>> {
        self.read_blob_limited(blob, u64::MAX)
    }

    pub fn read_blob_limited(&self, blob: &BlobRef, maximum_bytes: u64) -> Result<Vec<u8>> {
        use std::io::Read;
        if blob.path != format!("{}.blob", blob.sha256)
            || blob.sha256.len() != 64
            || !blob.sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(invalid("invalid repository blob reference"));
        }
        if blob.bytes == 0 || blob.bytes > maximum_bytes || blob.bytes >= usize::MAX as u64 {
            return Err(invalid("artifact exceeds the configured byte limit"));
        }
        let path = self.blob_dir.join(&blob.path);
        if !std::fs::symlink_metadata(&path)?.file_type().is_file() {
            return Err(invalid("artifact must be a regular repository file"));
        }
        let file = std::fs::File::open(path)?;
        if file.metadata()?.len() != blob.bytes {
            return Err(invalid("artifact digest or size mismatch"));
        }
        let mut bytes = Vec::with_capacity((blob.bytes as usize).min(16 * 1024 * 1024));
        file.take(blob.bytes + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 != blob.bytes || digest(&bytes) != blob.sha256 {
            return Err(invalid("artifact digest or size mismatch"));
        }
        Ok(bytes)
    }

    /// The caller keeps `_writer` until the returned reference is committed. The blob
    /// directory is checked on every write, because a handle outlives the checks of `open`.
    fn put_blob(&self, _writer: &LocalRunLock, bytes: &[u8]) -> Result<BlobRef> {
        if bytes.is_empty() {
            return Err(invalid("empty checkpoint/model artifact"));
        }
        verify_storage_entry(&self.root, &self.blob_dir, true)?;
        let hash = digest(bytes);
        let blob = BlobRef {
            path: format!("{hash}.blob"),
            sha256: hash,
            bytes: bytes.len() as u64,
        };
        let destination = self.blob_dir.join(&blob.path);
        if destination.exists() {
            self.read_blob(&blob)?;
            return Ok(blob);
        }
        let staging = StagingFile(self.blob_dir.join(format!("{}.part", id())));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staging.0)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        if let Err(error) = std::fs::rename(&staging.0, &destination) {
            if !destination.exists() {
                return Err(error.into());
            }
            self.read_blob(&blob)?;
        }
        #[cfg(unix)]
        std::fs::File::open(&self.blob_dir)?.sync_all()?;
        Ok(blob)
    }

    fn blob_writer(&self) -> Result<LocalRunLock> {
        LocalRunLock::shared(self, BLOB_STORE_LOCK)
    }

    fn stored_snapshot_digest(&self, snapshot_id: &str) -> Result<String> {
        self.connection()?
            .query_row(
                "SELECT json_extract(body,'$.digest') FROM snapshots WHERE id=?",
                [snapshot_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten()
            .ok_or_else(|| Error::NotFound(snapshot_id.into()))
    }

    /// Deletes blobs that no artifact, pretrained source or resumable checkpoint references,
    /// plus abandoned staging files, and returns the bytes freed. Skipped while any writer
    /// is between staging a blob and committing its reference.
    pub(crate) fn collect_unreferenced_blobs(&self) -> Result<u64> {
        let exclusive = match LocalRunLock::acquire(self, BLOB_STORE_LOCK) {
            Ok(lock) => lock,
            Err(Error::Conflict(_)) => return Ok(0),
            Err(error) => return Err(error),
        };
        let mut connection = self.connection()?;
        let tx = connection.transaction()?;
        let referenced = referenced_blobs(&tx)?;
        drop(tx);
        Ok(self.sweep_blob_store(&exclusive, Some(&referenced)))
    }

    /// Without `referenced` only staging files are removed. A blob directory that is no
    /// longer a real directory of the ledger is left alone.
    fn sweep_blob_store(
        &self,
        _exclusive: &LocalRunLock,
        referenced: Option<&HashSet<String>>,
    ) -> u64 {
        if verify_storage_entry(&self.root, &self.blob_dir, true).is_err() {
            return 0;
        }
        let Ok(entries) = std::fs::read_dir(&self.blob_dir) else {
            return 0;
        };
        let mut freed = 0u64;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let stale = name.ends_with(".part")
                || referenced.is_some_and(|live| is_blob_name(name) && !live.contains(name));
            if !stale {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.is_file() && std::fs::remove_file(entry.path()).is_ok() {
                freed = freed.saturating_add(metadata.len());
            }
        }
        freed
    }

    /// Hosts may call this through the controller-owned telemetry channel for shadow runs.
    /// The restricted handle cannot append samples, checkpoints, evaluations or deployments.
    pub fn record_prediction(&self, prediction: &PredictionRecord) -> Result<bool> {
        if prediction.sample_id.is_empty()
            || !prediction.latency_ms.is_finite()
            || prediction.latency_ms < 0.
        {
            return Err(invalid(
                "prediction requires a sample ID and finite nonnegative latency",
            ));
        }
        if prediction.actual_label.is_some() && prediction.actual_source.is_none() {
            return Err(invalid("ground truth requires its source"));
        }
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let artifact: ModelArtifact = read_json(
            &tx,
            "SELECT body FROM artifacts WHERE id=?",
            &prediction.artifact_id,
        )?;
        if prediction.actual_source.is_some() {
            prediction_truth(&tx, &artifact.stream, prediction, true)?;
        }
        let body = serde_json::to_string(prediction)?;
        let existing: Option<String> = tx
            .query_row(
                "SELECT body FROM predictions WHERE artifact_id=? AND sample_id=?",
                params![prediction.artifact_id, prediction.sample_id],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(existing) = existing {
            if existing != body {
                return Err(Error::Conflict(
                    "prediction pairs are immutable; use a new sample evaluation ID".into(),
                ));
            }
            return Ok(false);
        }
        tx.execute(
            "INSERT INTO predictions(artifact_id,sample_id,body) VALUES (?,?,?)",
            params![prediction.artifact_id, prediction.sample_id, body],
        )?;
        tx.commit()?;
        Ok(true)
    }

    pub fn predictions(&self, artifact_id: &str) -> Result<Vec<PredictionRecord>> {
        let conn = self.connection()?;
        let mut statement =
            conn.prepare("SELECT body FROM predictions WHERE artifact_id=? ORDER BY sample_id")?;
        statement
            .query_map([artifact_id], |r| r.get::<_, String>(0))?
            .map(|row| Ok(serde_json::from_str(&row?)?))
            .collect()
    }

    /// Computes the proof from immutable prediction pairs, rather than accepting supplied
    /// accuracy numbers. Training/validation groups cannot supply audited evidence.
    pub fn evaluate_classification(
        &self,
        artifact_id: &str,
        at_ms: i64,
    ) -> Result<EvaluationReport> {
        self.evaluate_classification_cohort(artifact_id, None, at_ms)
    }

    pub(crate) fn evaluate_classification_cohort(
        &self,
        artifact_id: &str,
        cohort: Option<&std::collections::BTreeSet<String>>,
        at_ms: i64,
    ) -> Result<EvaluationReport> {
        self.writable()?;
        let mut conn = self.connection()?;
        // Artifacts, snapshots and their lineage are immutable: read them before the write lock.
        let artifact: ModelArtifact =
            read_json(&conn, "SELECT body FROM artifacts WHERE id=?", artifact_id)?;
        let snapshot: DatasetSnapshot = read_json(
            &conn,
            "SELECT body FROM snapshots WHERE id=?",
            &artifact.snapshot_id,
        )?;
        #[cfg(feature = "native")]
        let consumed = crate::engines::ConsumedGroups::load(&conn, &artifact, &snapshot)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut statement =
            tx.prepare("SELECT body FROM predictions WHERE artifact_id=? ORDER BY sample_id")?;
        let mut predictions: Vec<PredictionRecord> = statement
            .query_map([artifact_id], |r| r.get::<_, String>(0))?
            .map(|row| Ok(serde_json::from_str(&row?)?))
            .collect::<Result<_>>()?;
        drop(statement);
        if let Some(cohort) = cohort {
            predictions.retain(|p| cohort.contains(&p.sample_id));
            if predictions.len() != cohort.len() {
                return Err(invalid(
                    "model predictions do not cover the complete audit cohort",
                ));
            }
        }
        if predictions.is_empty() {
            return Err(invalid("no prediction pairs to evaluate"));
        }
        if predictions
            .iter()
            .any(|prediction| prediction.recorded_at_ms > at_ms)
        {
            return Err(invalid(
                "prediction evidence was not available at evaluation time",
            ));
        }
        let mut teacher_count = 0usize;
        let mut teacher_correct = 0usize;
        let mut truths = Vec::new();
        for prediction in &predictions {
            if let Some(teacher) = &prediction.teacher_label {
                teacher_count += 1;
                teacher_correct += usize::from(
                    prediction.predicted_label.as_ref() == Some(teacher) && !prediction.failed,
                );
            }
            if !matches!(
                prediction.actual_source,
                Some(LabelSource::Reviewed | LabelSource::ObservedOutcome)
            ) {
                continue;
            }
            // A withdrawn annotation no longer provides audited truth.
            let Some(mut sample) = current_prediction_truth(&tx, &artifact.stream, prediction)?
            else {
                continue;
            };
            // Only the label is scored, and payloads can hold whole image tensors.
            let actual = truth_label(&sample).map(str::to_owned);
            sample.payload = Value::Null;
            truths.push((prediction, sample, actual));
        }
        #[cfg(feature = "native")]
        consumed.ensure_independent(truths.iter().map(|(_, sample, _)| sample))?;
        #[cfg(not(feature = "native"))]
        {
            let training_groups: std::collections::HashSet<_> = snapshot
                .train
                .iter()
                .chain(&snapshot.validation)
                .map(|s| s.group_id.as_str())
                .collect();
            if let Some((_, sample, _)) = truths
                .iter()
                .find(|(_, sample, _)| training_groups.contains(sample.group_id.as_str()))
            {
                return Err(invalid(format!(
                    "sample {} shares a training/validation group",
                    sample.id
                )));
            }
        }
        let mut audited = 0usize;
        let mut correct = 0usize;
        let mut supports: BTreeMap<String, (usize, usize)> = BTreeMap::new();
        #[cfg(feature = "native")]
        let mut scored = Vec::new();
        for (prediction, sample, actual) in truths {
            if sample.label_available_at_ms > at_ms {
                return Err(invalid("evaluation truth was unavailable when recorded"));
            }
            let Some(actual) = actual else {
                continue;
            };
            let hit = !prediction.failed
                && prediction.predicted_label.as_deref() == Some(actual.as_str());
            audited += 1;
            correct += usize::from(hit);
            #[cfg(feature = "native")]
            scored.push((actual.clone(), prediction));
            let counts = supports.entry(actual).or_default();
            counts.0 += 1;
            counts.1 += usize::from(hit);
        }
        let mut metrics = BTreeMap::new();
        metrics.insert("accuracy".into(), ratio(correct, audited));
        metrics.insert("artifact_bytes".into(), artifact.blob.bytes as f64);
        #[cfg(feature = "native")]
        if let Some(labels) = artifact.manifest.get("labels").and_then(Value::as_array) {
            metrics.extend(class_order_metrics(labels, &scored));
        }
        metrics.insert(
            "accuracy_wilson_lower_95".into(),
            wilson_lower(correct, audited),
        );
        metrics.insert(
            "teacher_agreement".into(),
            ratio(teacher_correct, teacher_count),
        );
        metrics.insert(
            "failure_rate".into(),
            ratio(
                predictions.iter().filter(|p| p.failed).count(),
                predictions.len(),
            ),
        );
        metrics.insert(
            "mean_latency_ms".into(),
            predictions.iter().map(|p| p.latency_ms).sum::<f64>() / predictions.len() as f64,
        );
        for (class, (support, _)) in &supports {
            metrics.insert(format!("support/{class}"), *support as f64);
        }
        for (class, (support, hits)) in &supports {
            metrics.insert(format!("recall/{class}"), ratio(*hits, *support));
        }
        if let Some(labels) = artifact.manifest.get("labels").and_then(Value::as_array) {
            for (index, label) in labels.iter().enumerate() {
                if let Some(label) = label.as_str() {
                    let (support, hits) = supports.get(label).copied().unwrap_or_default();
                    metrics.insert(format!("support/{label}"), support as f64);
                    metrics.insert(format!("recall/{label}"), ratio(hits, support));
                    metrics.insert(format!("class/{index}/support"), support as f64);
                    metrics.insert(format!("class/{index}/recall"), ratio(hits, support));
                }
            }
        }
        let report = EvaluationReport {
            id: id(),
            artifact_id: artifact.id,
            dataset_digest: artifact.dataset_digest,
            evidence_digest: digest(&serde_json::to_vec(&predictions)?),
            truth_digest: Some(evaluation_truth_digest(
                &tx,
                &artifact.stream,
                &predictions,
            )?),
            audited_samples: audited,
            teacher_samples: teacher_count,
            metrics,
            per_class_recall: supports
                .into_iter()
                .map(|(class, (support, hits))| (class, ratio(hits, support)))
                .collect(),
            created_at_ms: at_ms,
            task: EvaluationTask::Classification,
        };
        tx.execute(
            "INSERT INTO evaluations(id,artifact_id,body) VALUES (?,?,?)",
            params![
                report.id,
                report.artifact_id,
                serde_json::to_string(&report)?
            ],
        )?;
        tx.commit()?;
        Ok(report)
    }

    pub fn get_evaluation(&self, evaluation_id: &str) -> Result<EvaluationReport> {
        read_json(
            &self.connection()?,
            "SELECT body FROM evaluations WHERE id=?",
            evaluation_id,
        )
    }

    pub fn promote(
        &self,
        deployment_id: &str,
        expected_generation: i64,
        evaluation_id: &str,
        policy: PromotionPolicy,
        at_ms: i64,
    ) -> Result<Deployment> {
        self.writable()?;
        validate_policy(&policy)?;
        self.promote_checked(
            deployment_id,
            expected_generation,
            evaluation_id,
            Some(policy),
            None,
            at_ms,
        )
    }

    pub fn promote_metrics(
        &self,
        deployment_id: &str,
        expected_generation: i64,
        evaluation_id: &str,
        policy: MetricPromotionPolicy,
        at_ms: i64,
    ) -> Result<Deployment> {
        self.writable()?;
        validate_metric_policy(&policy)?;
        self.promote_checked(
            deployment_id,
            expected_generation,
            evaluation_id,
            None,
            Some(policy),
            at_ms,
        )
    }

    fn promote_checked(
        &self,
        deployment_id: &str,
        expected_generation: i64,
        evaluation_id: &str,
        policy: Option<PromotionPolicy>,
        metric_policy: Option<MetricPromotionPolicy>,
        at_ms: i64,
    ) -> Result<Deployment> {
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let deployment = self.promote_in_transaction(
            &tx,
            deployment_id,
            expected_generation,
            evaluation_id,
            policy,
            metric_policy,
            at_ms,
        )?;
        tx.commit()?;
        Ok(deployment)
    }

    pub(crate) fn promote_in_transaction(
        &self,
        tx: &Connection,
        deployment_id: &str,
        expected_generation: i64,
        evaluation_id: &str,
        policy: Option<PromotionPolicy>,
        metric_policy: Option<MetricPromotionPolicy>,
        at_ms: i64,
    ) -> Result<Deployment> {
        if deployment_id.is_empty() || expected_generation < 0 {
            return Err(invalid("deployment ID and nonnegative generation required"));
        }
        let report: EvaluationReport = read_json(
            &tx,
            "SELECT body FROM evaluations WHERE id=?",
            evaluation_id,
        )?;
        let artifact: ModelArtifact = read_json(
            &tx,
            "SELECT body FROM artifacts WHERE id=?",
            &report.artifact_id,
        )?;
        if report.created_at_ms > at_ms {
            return Err(invalid(
                "evaluation report was not available at promotion time",
            ));
        }
        self.read_blob(&artifact.blob)?;
        if report.dataset_digest != artifact.dataset_digest {
            return Err(invalid("evaluation dataset does not match model"));
        }
        let mut evidence_query =
            tx.prepare("SELECT body FROM predictions WHERE artifact_id=? ORDER BY sample_id")?;
        let evidence: Vec<PredictionRecord> = evidence_query
            .query_map([&artifact.id], |r| r.get::<_, String>(0))?
            .map(|row| Ok(serde_json::from_str(&row?)?))
            .collect::<Result<_>>()?;
        drop(evidence_query);
        if digest(&serde_json::to_vec(&evidence)?) != report.evidence_digest {
            return Err(Error::Conflict(
                "evaluation evidence changed; evaluate again before promotion".into(),
            ));
        }
        if !truth_digest_matches(
            tx,
            &artifact.stream,
            &evidence,
            report.truth_digest.as_deref(),
        )? {
            return Err(Error::Conflict(
                "accepted audit annotations changed; evaluate again before promotion".into(),
            ));
        }
        if let Some(policy) = &policy {
            if report.task != EvaluationTask::Classification {
                return Err(invalid(
                    "classification policy requires classification evidence",
                ));
            }
            check_promotion(&report, policy)?;
        }
        if let Some(policy) = &metric_policy {
            check_metric_promotion(&report, policy)?;
        }
        let old: Option<String> = tx
            .query_row(
                "SELECT body FROM deployments WHERE id=?",
                [deployment_id],
                |r| r.get(0),
            )
            .optional()?;
        let old: Option<Deployment> = old.map(|s| serde_json::from_str(&s)).transpose()?;
        if old.as_ref().map_or(0, |d| d.generation) != expected_generation {
            return Err(Error::Conflict("deployment generation changed".into()));
        }
        if old
            .as_ref()
            .is_some_and(|old| old.stream != artifact.stream)
        {
            return Err(Error::Conflict(
                "deployment belongs to a different inspection stream/version".into(),
            ));
        }
        let deployment = Deployment {
            id: deployment_id.into(),
            stream: artifact.stream,
            generation: expected_generation + 1,
            active_artifact_id: artifact.id,
            paused: false,
            previous_artifact_id: old.map(|d| d.active_artifact_id),
            evaluation_id: report.id,
            policy,
            metric_policy,
            updated_at_ms: at_ms,
        };
        save_deployment(&tx, &deployment)?;
        Ok(deployment)
    }

    pub fn get_deployment(&self, deployment_id: &str) -> Result<Deployment> {
        read_json(
            &self.connection()?,
            "SELECT body FROM deployments WHERE id=?",
            deployment_id,
        )
    }

    pub fn set_deployment_paused(
        &self,
        deployment_id: &str,
        expected_generation: i64,
        paused: bool,
        at_ms: i64,
    ) -> Result<Deployment> {
        self.writable()?;
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut deployment: Deployment = read_json(
            &tx,
            "SELECT body FROM deployments WHERE id=?",
            deployment_id,
        )?;
        if deployment.generation != expected_generation {
            return Err(Error::Conflict("deployment generation changed".into()));
        }
        deployment.paused = paused;
        deployment.generation += 1;
        deployment.updated_at_ms = at_ms;
        save_deployment(&tx, &deployment)?;
        tx.commit()?;
        Ok(deployment)
    }

    pub fn rollback(
        &self,
        deployment_id: &str,
        expected_generation: i64,
        at_ms: i64,
    ) -> Result<Deployment> {
        self.writable()?;
        let mut conn = self.connection()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let deployment =
            self.rollback_in_transaction(&tx, deployment_id, expected_generation, at_ms)?;
        tx.commit()?;
        Ok(deployment)
    }

    pub(crate) fn rollback_in_transaction(
        &self,
        tx: &Connection,
        deployment_id: &str,
        expected_generation: i64,
        at_ms: i64,
    ) -> Result<Deployment> {
        let mut deployment: Deployment = read_json(
            &tx,
            "SELECT body FROM deployments WHERE id=?",
            deployment_id,
        )?;
        if deployment.generation != expected_generation {
            return Err(Error::Conflict("deployment generation changed".into()));
        }
        let previous = deployment
            .previous_artifact_id
            .clone()
            .ok_or_else(|| Error::Conflict("deployment has no previous model".into()))?;
        let previous_state:String=tx.query_row("SELECT body FROM deployment_history WHERE id=? AND generation<? AND json_extract(body,'$.active_artifact_id')=? ORDER BY generation DESC LIMIT 1",params![deployment_id,expected_generation,previous],|r|r.get(0))?;
        let previous_state: Deployment = serde_json::from_str(&previous_state)?;
        if previous_state.active_artifact_id != previous {
            return Err(Error::Conflict(
                "deployment history does not identify the previous model".into(),
            ));
        }
        let artifact: ModelArtifact =
            read_json(&tx, "SELECT body FROM artifacts WHERE id=?", &previous)?;
        self.read_blob(&artifact.blob)?;
        deployment.previous_artifact_id = Some(deployment.active_artifact_id);
        deployment.active_artifact_id = previous;
        deployment.evaluation_id = previous_state.evaluation_id;
        deployment.policy = previous_state.policy;
        deployment.metric_policy = previous_state.metric_policy;
        deployment.paused = previous_state.paused;
        deployment.generation += 1;
        deployment.updated_at_ms = at_ms;
        save_deployment(&tx, &deployment)?;
        Ok(deployment)
    }
}

/// Digests the identity, provenance, target and acceptance of each prediction's current truth,
/// so raw-row (`feature_row`) and transformed-input edits leave sealed evidence valid.
/// Withdrawn truth digests as null.
pub(crate) fn evaluation_truth_digest(
    conn: &Connection,
    stream: &StreamKey,
    predictions: &[PredictionRecord],
) -> Result<String> {
    immutable_truth_digest(&evaluation_truths(conn, stream, predictions)?)
}

/// Whether a sealed truth digest still describes the predictions' truth. Reports sealed before
/// truth digests left out mutable sample parts carry a digest of the full annotation bodies.
pub(crate) fn truth_digest_matches(
    conn: &Connection,
    stream: &StreamKey,
    predictions: &[PredictionRecord],
    sealed: Option<&str>,
) -> Result<bool> {
    let Some(sealed) = sealed else {
        return Ok(false);
    };
    let truths = evaluation_truths(conn, stream, predictions)?;
    Ok(immutable_truth_digest(&truths)? == sealed
        || digest(&serde_json::to_vec(&truths)?) == sealed)
}

fn evaluation_truths(
    conn: &Connection,
    stream: &StreamKey,
    predictions: &[PredictionRecord],
) -> Result<Vec<Option<TrainingSample>>> {
    predictions
        .iter()
        .filter(|prediction| prediction.actual_source.is_some())
        .map(|prediction| current_prediction_truth(conn, stream, prediction))
        .collect()
}

fn immutable_truth_digest(truths: &[Option<TrainingSample>]) -> Result<String> {
    let immutable = truths
        .iter()
        .map(|truth| {
            truth
                .as_ref()
                .map(crate::experiment_repository::immutable_sample)
                .transpose()
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(digest(&serde_json::to_vec(&immutable)?))
}

/// The latest accepted annotation of the prediction's sample, which also replaces the truth
/// of a prediction recorded against an earlier revision. Fails when the latest annotation
/// is withdrawn.
pub(crate) fn validate_prediction_truth(
    conn: &Connection,
    stream: &StreamKey,
    prediction: &PredictionRecord,
) -> Result<TrainingSample> {
    current_prediction_truth(conn, stream, prediction)?
        .ok_or_else(|| invalid("the sample's latest annotation is no longer accepted"))
}

/// Like `validate_prediction_truth`, but a withdrawn latest annotation yields None so the
/// prediction can be left out of audited evidence.
pub(crate) fn current_prediction_truth(
    conn: &Connection,
    stream: &StreamKey,
    prediction: &PredictionRecord,
) -> Result<Option<TrainingSample>> {
    prediction_truth(conn, stream, prediction, false)
}

/// The prediction's ground truth must match an accepted revision of its sample. When it is
/// recorded, it must match the latest one unless it predates that revision's availability.
fn prediction_truth(
    conn: &Connection,
    stream: &StreamKey,
    prediction: &PredictionRecord,
    recording: bool,
) -> Result<Option<TrainingSample>> {
    let mut statement = conn.prepare(
        "SELECT body FROM annotations WHERE scope=? AND sample_id=? ORDER BY revision DESC",
    )?;
    let mut rows = statement.query(params![scope(stream)?, prediction.sample_id])?;
    let Some(row) = rows.next()? else {
        return Err(invalid("actual labels must reference a recorded sample"));
    };
    let latest: TrainingSample = serde_json::from_str(&row.get::<_, String>(0)?)?;
    if latest.accepted && truth_matches(&latest, prediction) {
        return Ok(Some(latest));
    }
    if !recording || prediction.recorded_at_ms < latest.label_available_at_ms {
        while let Some(row) = rows.next()? {
            let revision: TrainingSample = serde_json::from_str(&row.get::<_, String>(0)?)?;
            if revision.accepted && truth_matches(&revision, prediction) {
                let current = latest.accepted;
                return Ok(current.then_some(latest));
            }
        }
    }
    Err(invalid(
        "prediction ground truth differs from the accepted annotation",
    ))
}

fn truth_matches(sample: &TrainingSample, prediction: &PredictionRecord) -> bool {
    Some(sample.source) == prediction.actual_source
        && prediction
            .actual_label
            .as_deref()
            .is_none_or(|actual| truth_label(sample) == Some(actual))
}

fn truth_label(sample: &TrainingSample) -> Option<&str> {
    sample
        .payload
        .get("label")
        .or_else(|| {
            sample
                .payload
                .get("annotation")
                .and_then(|annotation| annotation.get("label"))
        })
        .and_then(Value::as_str)
}

/// A failed or out-of-schema prediction is a miss for its actual class: it lowers that
/// class's recall without becoming another class's false positive, and it takes the
/// largest rank error.
#[cfg(feature = "native")]
fn class_order_metrics(
    labels: &[Value],
    truths: &[(String, &PredictionRecord)],
) -> BTreeMap<String, f64> {
    let mut metrics = BTreeMap::new();
    let classes = labels.len();
    let position = |label: &str| {
        labels
            .iter()
            .position(|candidate| candidate.as_str() == Some(label))
    };
    let Some(pairs) = truths
        .iter()
        .map(|(actual, prediction)| {
            let predicted = prediction
                .predicted_label
                .as_deref()
                .filter(|_| !prediction.failed)
                .and_then(position);
            Some((position(actual.as_str())?, predicted))
        })
        .collect::<Option<Vec<_>>>()
        .filter(|pairs| !pairs.is_empty())
    else {
        return metrics;
    };
    let actual: Vec<u32> = pairs.iter().map(|&(actual, _)| actual as u32).collect();
    let missed = pairs.iter().any(|&(_, predicted)| predicted.is_none());
    let predicted: Vec<u32> = pairs
        .iter()
        .map(|&(_, predicted)| predicted.unwrap_or(classes) as u32)
        .collect();
    if let Ok(classification) = flow_like_ml_native::evaluation::classification_metrics(
        &actual,
        &predicted,
        classes + usize::from(missed),
    ) {
        let macro_f1 = if missed {
            let f1: Vec<f64> = classification
                .per_class
                .iter()
                .filter(|class| (class.class_id as usize) < classes)
                .filter_map(|class| class.f1)
                .collect();
            (!f1.is_empty()).then(|| f1.iter().sum::<f64>() / f1.len() as f64)
        } else {
            Some(classification.macro_f1)
        };
        if let Some(macro_f1) = macro_f1 {
            metrics.insert("macro_f1".into(), macro_f1);
        }
        metrics.insert("balanced_accuracy".into(), classification.balanced_accuracy);
    }
    let ranked: Vec<u32> = pairs
        .iter()
        .map(|&(actual, predicted)| {
            predicted.unwrap_or(if actual * 2 < classes { classes - 1 } else { 0 }) as u32
        })
        .collect();
    if let Ok(ordinal) = flow_like_ml_native::evaluation::ordinal_metrics(&actual, &ranked, classes)
    {
        metrics.insert("mean_rank_error".into(), ordinal.mean_rank_error);
        metrics.insert("macro_rank_error".into(), ordinal.macro_rank_error);
        metrics.insert("quadratic_kappa".into(), ordinal.quadratic_kappa);
    }
    metrics
}
fn ratio(a: usize, b: usize) -> f64 {
    if b == 0 { 0. } else { a as f64 / b as f64 }
}
fn wilson_lower(hits: usize, total: usize) -> f64 {
    if total == 0 {
        return 0.;
    }
    let n = total as f64;
    let p = hits as f64 / n;
    let z = 1.959963984540054;
    (p + z * z / (2. * n) - z * (p * (1. - p) / n + z * z / (4. * n * n)).sqrt()) / (1. + z * z / n)
}
fn validate_policy(policy: &PromotionPolicy) -> Result<()> {
    if policy.minimum_audited_samples == 0
        || !policy.maximum_mean_latency_ms.is_finite()
        || policy.maximum_mean_latency_ms < 0.
        || [&policy.minimum_accuracy, &policy.maximum_failure_rate]
            .into_iter()
            .chain(policy.minimum_class_recall.values())
            .any(|x| !x.is_finite() || !(0. ..=1.).contains(x))
    {
        return Err(invalid(
            "promotion needs audited samples, finite latency and thresholds in [0,1]",
        ));
    }
    Ok(())
}
fn check_promotion(report: &EvaluationReport, policy: &PromotionPolicy) -> Result<()> {
    if report.audited_samples < policy.minimum_audited_samples
        || report.metrics.get("accuracy").copied().unwrap_or(0.) < policy.minimum_accuracy
        || report
            .metrics
            .get("mean_latency_ms")
            .copied()
            .unwrap_or(f64::INFINITY)
            > policy.maximum_mean_latency_ms
        || report.metrics.get("failure_rate").copied().unwrap_or(1.) > policy.maximum_failure_rate
        || policy
            .minimum_class_recall
            .iter()
            .any(|(class, threshold)| {
                report
                    .per_class_recall
                    .get(class)
                    .is_none_or(|recall| recall < threshold)
            })
    {
        return Err(Error::Conflict(
            "candidate does not satisfy promotion policy".into(),
        ));
    }
    Ok(())
}
fn validate_metric_policy(policy: &MetricPromotionPolicy) -> Result<()> {
    let mut names = std::collections::HashSet::new();
    if policy.minimum_audited_samples == 0 || policy.bounds.is_empty() {
        return Err(invalid(
            "metric policy requires audited samples and at least one bound",
        ));
    }
    for bound in &policy.bounds {
        if bound.name.trim().is_empty()
            || !names.insert(&bound.name)
            || (bound.minimum.is_none() && bound.maximum.is_none())
            || bound
                .minimum
                .into_iter()
                .chain(bound.maximum)
                .any(|value| !value.is_finite())
            || matches!((bound.minimum,bound.maximum),(Some(min),Some(max)) if min>max)
        {
            return Err(invalid(
                "metric names must be unique and bounds finite and ordered",
            ));
        }
    }
    Ok(())
}
fn check_metric_promotion(report: &EvaluationReport, policy: &MetricPromotionPolicy) -> Result<()> {
    if report.audited_samples < policy.minimum_audited_samples {
        return Err(Error::Conflict(
            "too few independently audited samples".into(),
        ));
    }
    for bound in &policy.bounds {
        let value = report.metrics.get(&bound.name).copied().ok_or_else(|| {
            invalid(format!(
                "metric {} is undefined for this evidence",
                bound.name
            ))
        })?;
        if !value.is_finite()
            || bound.minimum.is_some_and(|min| value < min)
            || bound.maximum.is_some_and(|max| value > max)
        {
            return Err(Error::Conflict(format!(
                "metric {} does not satisfy promotion bounds",
                bound.name
            )));
        }
    }
    Ok(())
}
fn save_deployment(tx: &Connection, deployment: &Deployment) -> Result<()> {
    let body = serde_json::to_string(deployment)?;
    tx.execute("INSERT INTO deployments(id,scope,generation,body) VALUES (?,?,?,?) ON CONFLICT(id) DO UPDATE SET generation=excluded.generation,body=excluded.body",params![deployment.id,scope(&deployment.stream)?,deployment.generation,body])?;
    tx.execute(
        "INSERT INTO deployment_history(id,generation,body) VALUES (?,?,?)",
        params![deployment.id, deployment.generation, body],
    )?;
    Ok(())
}

fn scope(stream: &StreamKey) -> Result<String> {
    if [
        &stream.project_id,
        &stream.stream_id,
        &stream.inspection_version,
    ]
    .iter()
    .any(|s| s.trim().is_empty())
    {
        return Err(invalid(
            "project, stream and inspection version must be nonempty",
        ));
    }
    Ok(serde_json::to_string(stream)?)
}
fn validate_sample(sample: &TrainingSample) -> Result<()> {
    if sample.id.is_empty()
        || sample.group_id.is_empty()
        || sample.content_digest.len() != 64
        || !sample.content_digest.bytes().all(|b| b.is_ascii_hexdigit())
        || sample.annotation_revision > i64::MAX as u64
    {
        return Err(invalid(
            "sample requires ID, group, SHA256 content digest and valid revision",
        ));
    }
    if sample.label_available_at_ms < sample.captured_at_ms {
        return Err(invalid("label cannot precede sample capture"));
    }
    let payload = sample.payload.get("sample").unwrap_or(&sample.payload);
    for (field, expected) in [
        ("id", sample.id.as_str()),
        ("group_id", sample.group_id.as_str()),
    ] {
        if payload
            .get(field)
            .is_some_and(|value| value.as_str() != Some(expected))
        {
            return Err(invalid(format!(
                "payload {field} differs from ledger identity"
            )));
        }
    }
    if payload
        .pointer("/outcome/available_at_ms")
        .and_then(Value::as_i64)
        .is_some_and(|time| time > sample.label_available_at_ms)
    {
        return Err(invalid("outcome is unavailable at the recorded label time"));
    }
    sample_interval(sample)?;
    Ok(())
}

fn sample_interval(sample: &TrainingSample) -> Result<(i64, i64)> {
    let payload = sample.payload.get("sample").unwrap_or(&sample.payload);
    let start = payload
        .get("window_start_ms")
        .and_then(Value::as_i64)
        .unwrap_or(sample.captured_at_ms);
    let end = payload
        .get("window_end_ms")
        .and_then(Value::as_i64)
        .unwrap_or(sample.captured_at_ms);
    if start > end {
        return Err(invalid("feature window starts after it ends"));
    }
    let target_end = payload
        .pointer("/outcome/target_end_ms")
        .and_then(Value::as_i64)
        .unwrap_or(end);
    Ok((start, end.max(target_end)))
}
fn accepted_sequence(tx: &Connection, scope: &str, sample_id: &str) -> Result<Option<i64>> {
    Ok(tx
        .query_row(
            "SELECT sequence FROM accepted WHERE scope=? AND sample_id=?",
            params![scope, sample_id],
            |r| r.get(0),
        )
        .optional()?)
}
fn read_json<T: serde::de::DeserializeOwned>(conn: &Connection, sql: &str, key: &str) -> Result<T> {
    let body: String = conn
        .query_row(sql, [key], |r| r.get(0))
        .optional()?
        .ok_or_else(|| Error::NotFound(key.into()))?;
    Ok(serde_json::from_str(&body)?)
}
fn status_name(status: JobStatus) -> &'static str {
    match status {
        JobStatus::Queued => "queued",
        JobStatus::Running => "running",
        JobStatus::CancelRequested => "cancel_requested",
        JobStatus::Paused => "paused",
        JobStatus::Interrupted => "interrupted",
        JobStatus::Cancelled => "cancelled",
        JobStatus::Failed => "failed",
        JobStatus::Succeeded => "succeeded",
    }
}

fn pretrained_artifact(artifact: ModelArtifact) -> PretrainedSource {
    PretrainedSource {
        id: artifact.id,
        project_id: artifact.stream.project_id.clone(),
        source_kind: PretrainedSourceKind::TrainingArtifact,
        blob: artifact.blob,
        manifest: artifact.manifest,
        origin: serde_json::json!({
            "stream": artifact.stream,
            "job_id": artifact.job_id,
            "snapshot_id": artifact.snapshot_id,
            "dataset_digest": artifact.dataset_digest,
        }),
        created_at_ms: artifact.created_at_ms,
    }
}
fn save_job(tx: &Connection, job: &TrainingJob) -> Result<()> {
    tx.execute(
        "UPDATE jobs SET status=?,body=? WHERE id=?",
        params![status_name(job.status), serde_json::to_string(job)?, job.id],
    )?;
    if job.status.is_terminal() || matches!(job.status, JobStatus::Paused | JobStatus::Queued) {
        tx.execute(
            "DELETE FROM resource_reservations WHERE job_id=?",
            [&job.id],
        )?;
    }
    Ok(())
}
fn experiment_trial(connection: &Connection, job_id: &str) -> Result<bool> {
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM experiment_trials WHERE job_id=?)",
        [job_id],
        |row| row.get(0),
    )?)
}
/// A checkpoint written for the previous request stays resumable under the rewritten one; a
/// checkpoint that already differed from it stays rejected at admission.
pub(crate) fn rewrite_job_request(
    job: &mut TrainingJob,
    rewrite: impl FnOnce(&mut TrainingRequest) -> Result<()>,
) -> Result<()> {
    let previous = digest(&serde_json::to_vec(&job.request)?);
    rewrite(&mut job.request)?;
    if let Some(checkpoint) = job
        .checkpoint
        .as_mut()
        .filter(|checkpoint| checkpoint.request_digest == previous)
    {
        checkpoint.request_digest = digest(&serde_json::to_vec(&job.request)?);
    }
    Ok(())
}
fn validate_lease(job: &TrainingJob, lease: &JobLease, at_ms: i64) -> Result<()> {
    if job.id != lease.job_id
        || job.generation != lease.generation
        || job.lease_owner.as_deref() != Some(&lease.owner)
        || job
            .lease_expires_at_ms
            .is_none_or(|deadline| deadline <= at_ms)
        || !matches!(job.status, JobStatus::Running | JobStatus::CancelRequested)
    {
        return Err(Error::LeaseLost);
    }
    Ok(())
}

fn lease_expired(job: &TrainingJob, at_ms: i64) -> bool {
    matches!(job.status, JobStatus::Running | JobStatus::CancelRequested)
        && job
            .lease_expires_at_ms
            .is_some_and(|deadline| deadline <= at_ms)
}

fn settle_stopped(job: &mut TrainingJob, status: JobStatus, at_ms: i64) {
    job.status = status;
    job.generation += 1;
    job.lease_owner = None;
    job.lease_expires_at_ms = None;
    job.updated_at_ms = at_ms;
}

/// A failure during a requested cancellation still ends as Cancelled.
fn settle_failed(job: &mut TrainingJob, error: String) {
    job.status = if job.status == JobStatus::CancelRequested {
        JobStatus::Cancelled
    } else {
        JobStatus::Failed
    };
    job.error = Some(error);
    job.lease_expires_at_ms = None;
}

/// Holders of the reservations that left a claim over budget.
struct Occupied {
    holders: Vec<String>,
    message: String,
}

/// Removes a staged blob on every exit path; after the rename there is nothing left to remove.
struct StagingFile(PathBuf);
impl Drop for StagingFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn is_blob_name(name: &str) -> bool {
    name.strip_suffix(".blob")
        .is_some_and(|hash| hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn referenced_blobs(conn: &Connection) -> Result<HashSet<String>> {
    let mut referenced = HashSet::new();
    for sql in [
        "SELECT json_extract(body,'$.blob.path'),json_extract(body,'$.manifest.source_blob.path') FROM artifacts",
        "SELECT json_extract(body,'$.blob.path'),NULL FROM pretrained_sources",
        LIVE_CHECKPOINTS,
    ] {
        let mut statement = conn.prepare(sql)?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            for column in 0..2 {
                if let Some(path) = row.get::<_, Option<String>>(column)? {
                    referenced.insert(path);
                }
            }
        }
    }
    Ok(referenced)
}

/// The ledger, its blob and lock directories and its journal files must be real entries of
/// the ledger's own directory, so a planted symlink cannot redirect writes elsewhere.
fn verify_storage_entries(root: &Path, ledger: &Path, blob_dir: &Path) -> Result<()> {
    let mut entries = vec![
        (ledger.to_owned(), false),
        (blob_dir.to_owned(), true),
        (root.join(".training-locks"), true),
    ];
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut journal = ledger.as_os_str().to_owned();
        journal.push(suffix);
        entries.push((PathBuf::from(journal), false));
    }
    for (entry, directory) in entries {
        verify_storage_entry(root, &entry, directory)?;
    }
    Ok(())
}

/// An absent entry passes.
fn verify_storage_entry(root: &Path, entry: &Path, directory: bool) -> Result<()> {
    match present(std::fs::symlink_metadata(entry))? {
        Some(metadata) => verify_present_entry(root, entry, directory, &metadata),
        None => Ok(()),
    }
}

/// SQLite deletes its -wal and -shm files when its last connection closes, so an entry can
/// vanish after its metadata was read; it then counts as absent.
fn verify_present_entry(
    root: &Path,
    entry: &Path,
    directory: bool,
    metadata: &std::fs::Metadata,
) -> Result<()> {
    let real = if directory {
        metadata.is_dir()
    } else {
        metadata.is_file()
    };
    if real
        && present(entry.canonicalize())?
            .is_none_or(|canonical| canonical.parent() == Some(root))
    {
        return Ok(());
    }
    Err(invalid(format!(
        "training storage entry {} must be a real {} inside {}",
        entry.display(),
        if directory { "directory" } else { "file" },
        root.display()
    )))
}

/// `None` when the path does not exist.
fn present<T>(result: std::io::Result<T>) -> Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// A received ledger must not carry triggers, views or other objects this runtime does not
/// create: they would run on, or stand in for, the user's own training records.
fn verify_schema_objects(conn: &Connection) -> Result<()> {
    let expected = expected_schema_objects()?;
    let mut statement = conn.prepare("SELECT type,name FROM sqlite_master")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let object = (row.get::<_, String>(0)?, row.get::<_, String>(1)?);
        let internal =
            object.1.starts_with("sqlite_") && matches!(object.0.as_str(), "table" | "index");
        if !internal && !expected.contains(&object) {
            return Err(invalid(format!(
                "training ledger contains an unexpected {} {}",
                object.0, object.1
            )));
        }
    }
    Ok(())
}

fn expected_schema_objects() -> Result<&'static BTreeSet<(String, String)>> {
    static OBJECTS: std::sync::OnceLock<BTreeSet<(String, String)>> = std::sync::OnceLock::new();
    if let Some(objects) = OBJECTS.get() {
        return Ok(objects);
    }
    let connection = Connection::open_in_memory()?;
    connection.execute_batch(SCHEMA)?;
    crate::experiment_repository::initialize_experiments(&connection)?;
    crate::learning_repository::initialize_learning(&connection)?;
    let mut statement = connection
        .prepare("SELECT type,name FROM sqlite_master WHERE type IN ('table','index')")?;
    let objects = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<BTreeSet<_>>>()?;
    Ok(OBJECTS.get_or_init(|| objects))
}

fn count_new_samples(conn: &Connection, scope: &str, at_ms: i64) -> Result<usize> {
    Ok(conn.query_row(NEW_SAMPLE_COUNT, params![scope, at_ms], |row| row.get(0))?)
}

fn eligible_samples(
    tx: &Connection,
    scope: &str,
    at_ms: i64,
) -> Result<Vec<(i64, TrainingSample)>> {
    let mut stmt = tx.prepare("SELECT a.sequence,n.body FROM accepted a JOIN annotations n ON n.scope=a.scope AND n.sample_id=a.sample_id WHERE a.scope=? ORDER BY a.sequence,n.revision DESC")?;
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for row in stmt.query_map([scope], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
    })? {
        let (sequence, body) = row?;
        let sample: TrainingSample = serde_json::from_str(&body)?;
        if sample.label_available_at_ms > at_ms || !seen.insert(sample.id.clone()) {
            continue;
        }
        if sample.accepted {
            result.push((sequence, sample));
        }
    }
    Ok(result)
}

fn partitions_ready(samples: &[(i64, TrainingSample)], policy: &SplitPolicy) -> Result<bool> {
    let mut groups: BTreeMap<&str, (i64, i64)> = BTreeMap::new();
    for (_, sample) in samples {
        let (start, end) = sample_interval(sample)?;
        groups
            .entry(sample.group_id.as_str())
            .and_modify(|range| {
                range.0 = range.0.min(start);
                range.1 = range.1.max(end);
            })
            .or_insert((start, end));
    }
    match *policy {
        SplitPolicy::Group {
            train_fraction,
            validation_fraction,
            ..
        } => {
            if !train_fraction.is_finite()
                || !validation_fraction.is_finite()
                || train_fraction <= 0.
                || validation_fraction <= 0.
                || train_fraction + validation_fraction >= 1.
            {
                return Err(invalid(
                    "group split needs positive train/validation/test fractions",
                ));
            }
            Ok(groups.len() >= 3)
        }
        SplitPolicy::Time {
            train_end_ms,
            validation_end_ms,
            embargo_ms,
        } => {
            if embargo_ms < 0 || train_end_ms >= validation_end_ms {
                return Err(invalid(
                    "time split requires ordered boundaries and nonnegative embargo",
                ));
            }
            let validation_start = train_end_ms
                .checked_add(embargo_ms)
                .ok_or_else(|| invalid("embargo overflow"))?;
            let test_start = validation_end_ms
                .checked_add(embargo_ms)
                .ok_or_else(|| invalid("embargo overflow"))?;
            let mut present = [false; 3];
            for (_, (start, end)) in groups {
                if end <= train_end_ms {
                    present[0] = true;
                } else if start > validation_start && end <= validation_end_ms {
                    present[1] = true;
                } else if start > test_start {
                    present[2] = true;
                }
            }
            Ok(present.into_iter().all(|exists| exists))
        }
    }
}

fn create_snapshot(
    tx: &Transaction<'_>,
    stream: &StreamKey,
    policy: SplitPolicy,
    at_ms: i64,
) -> Result<DatasetSnapshot> {
    let samples = eligible_samples(tx, &scope(stream)?, at_ms)?;
    snapshot_from_samples(tx, stream, policy, at_ms, samples)
}

/// `samples` are the stream's `eligible_samples` at `at_ms`, read in the same transaction.
fn snapshot_from_samples(
    tx: &Transaction<'_>,
    stream: &StreamKey,
    policy: SplitPolicy,
    at_ms: i64,
    samples: Vec<(i64, TrainingSample)>,
) -> Result<DatasetSnapshot> {
    let scope = scope(stream)?;
    if samples.is_empty() {
        return Err(invalid("no accepted labels are available at snapshot time"));
    }
    let cutoff = samples
        .iter()
        .map(|(sequence, _)| *sequence)
        .max()
        .unwrap_or(0);
    let mut groups: BTreeMap<String, Vec<TrainingSample>> = BTreeMap::new();
    let mut content_groups: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for (_, sample) in samples {
        if content_groups
            .insert(sample.content_digest.clone(), sample.group_id.clone())
            .is_some_and(|group| group != sample.group_id)
        {
            return Err(invalid(
                "identical source content appears in different split groups",
            ));
        }
        groups
            .entry(sample.group_id.clone())
            .or_default()
            .push(sample);
    }
    let mut snapshot = DatasetSnapshot {
        id: id(),
        digest: String::new(),
        stream: stream.clone(),
        as_of_ms: at_ms,
        cutoff_sequence: cutoff,
        policy: policy.clone(),
        train: vec![],
        validation: vec![],
        test: vec![],
        excluded: vec![],
    };
    match policy {
        SplitPolicy::Group {
            train_fraction,
            validation_fraction,
            seed,
        } => {
            if !train_fraction.is_finite()
                || !validation_fraction.is_finite()
                || train_fraction <= 0.
                || validation_fraction <= 0.
                || train_fraction + validation_fraction >= 1.
            {
                return Err(invalid(
                    "group split needs positive train/validation/test fractions",
                ));
            }
            if groups.len() < 3 {
                return Err(invalid(
                    "at least three independent groups are required for train/validation/test",
                ));
            }
            let mut groups: Vec<_> = groups.into_iter().collect();
            groups.sort_by_key(|(group, _)| digest(format!("{seed}:{scope}:{group}").as_bytes()));
            let train_count = ((groups.len() as f64 * train_fraction).floor() as usize)
                .clamp(1, groups.len() - 2);
            let validation_count = ((groups.len() as f64 * validation_fraction).floor() as usize)
                .clamp(1, groups.len() - train_count - 1);
            for (i, (_, samples)) in groups.into_iter().enumerate() {
                if i < train_count {
                    snapshot.train.extend(samples);
                } else if i < train_count + validation_count {
                    snapshot.validation.extend(samples);
                } else {
                    snapshot.test.extend(samples);
                }
            }
        }
        SplitPolicy::Time {
            train_end_ms,
            validation_end_ms,
            embargo_ms,
        } => {
            if embargo_ms < 0 || train_end_ms >= validation_end_ms {
                return Err(invalid(
                    "time split requires ordered boundaries and nonnegative embargo",
                ));
            }
            let train_start_validation = train_end_ms
                .checked_add(embargo_ms)
                .ok_or_else(|| invalid("embargo overflow"))?;
            let validation_start_test = validation_end_ms
                .checked_add(embargo_ms)
                .ok_or_else(|| invalid("embargo overflow"))?;
            for (_, samples) in groups {
                let intervals = samples
                    .iter()
                    .map(sample_interval)
                    .collect::<Result<Vec<_>>>()?;
                let first = intervals.iter().map(|(start, _)| *start).min().unwrap();
                let last = intervals.iter().map(|(_, end)| *end).max().unwrap();
                if last <= train_end_ms {
                    snapshot.train.extend(samples);
                } else if first > train_start_validation && last <= validation_end_ms {
                    snapshot.validation.extend(samples);
                } else if first > validation_start_test {
                    snapshot.test.extend(samples);
                } else {
                    snapshot.excluded.extend(samples.into_iter().map(|s| s.id));
                }
            }
            if snapshot.train.is_empty()
                || snapshot.validation.is_empty()
                || snapshot.test.is_empty()
            {
                return Err(invalid(
                    "time split leaves an empty train/validation/test partition",
                ));
            }
        }
    }
    for part in [
        &mut snapshot.train,
        &mut snapshot.validation,
        &mut snapshot.test,
    ] {
        part.sort_by(|a, b| a.id.cmp(&b.id));
    }
    // Identity and creation time do not change the content digest.
    snapshot.digest = snapshot_digest(&snapshot)?;
    tx.execute(
        "INSERT INTO snapshots(id,scope,body) VALUES (?,?,?)",
        params![snapshot.id, scope, serde_json::to_string(&snapshot)?],
    )?;
    Ok(snapshot)
}

fn snapshot_digest(snapshot: &DatasetSnapshot) -> Result<String> {
    Ok(digest(&serde_json::to_vec(
        &serde_json::json!({"stream":snapshot.stream,"policy":snapshot.policy,"train":snapshot.train,"validation":snapshot.validation,"test":snapshot.test,"excluded":snapshot.excluded}),
    )?))
}

fn normal_only(recipe: &Value) -> bool {
    matches!(
        recipe["inspection_task"].as_str(),
        Some("visual_anomaly" | "sensor_anomaly" | "sequence_autoencoder")
    )
}

fn is_normal(sample: &TrainingSample) -> bool {
    let payload = sample.payload.get("sample").unwrap_or(&sample.payload);
    payload["annotation"]["kind"].as_str() == Some("anomaly")
        && payload["annotation"]["is_anomaly"].as_bool() == Some(false)
}

pub(crate) fn training_ready(snapshot: &DatasetSnapshot, recipe: &Value) -> Result<bool> {
    let total = recipe
        .get("minimum_examples")
        .and_then(Value::as_u64)
        .unwrap_or(1);
    if (snapshot.train.len() as u64) < total {
        return Ok(false);
    }
    let per_class = recipe
        .get("minimum_examples_per_class")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if normal_only(recipe) {
        return Ok(snapshot
            .train
            .iter()
            .filter(|sample| is_normal(sample))
            .count() as u64
            >= total.max(per_class));
    }
    if matches!(
        recipe["inspection_task"].as_str(),
        Some("sensor_regression" | "sequence_forecast")
    ) {
        return Ok(true);
    }
    let Some(labels) = recipe.get("labels").and_then(Value::as_array) else {
        return Ok(true);
    };
    if labels.is_empty() {
        return Ok(true);
    }
    let mut support = vec![0u64; labels.len()];
    for sample in &snapshot.train {
        let payload = sample.payload.get("sample").unwrap_or(&sample.payload);
        let annotation = &payload["annotation"];
        let mut classes = std::collections::HashSet::new();
        if let Some(class) = annotation["class_id"].as_u64() {
            classes.insert(class as usize);
        }
        if let Some(boxes) = annotation["boxes"].as_array() {
            for box_ in boxes {
                if let Some(class) = box_["class_id"].as_u64() {
                    classes.insert(class as usize);
                }
            }
        }
        if let Some(instances) = annotation["instances"].as_array() {
            for instance in instances {
                if let Some(class) = instance["bounds"]["class_id"].as_u64() {
                    classes.insert(class as usize);
                }
            }
        }
        if let Some(mask) = annotation["classes"].as_array() {
            for class in mask {
                if let Some(class) = class.as_u64() {
                    classes.insert(class as usize);
                }
            }
        }
        if classes.is_empty() {
            if let Some(label) = sample.payload["label"].as_str() {
                let class = labels
                    .iter()
                    .position(|candidate| candidate.as_str() == Some(label))
                    .ok_or_else(|| invalid("accepted label is absent from recipe label order"))?;
                classes.insert(class);
            }
        }
        for class in classes {
            if class >= labels.len() {
                return Err(invalid("accepted class ID exceeds recipe label order"));
            }
            if annotation["kind"].as_str() == Some("class")
                && sample
                    .payload
                    .get("label")
                    .is_some_and(|label| label != &labels[class])
            {
                return Err(invalid(
                    "accepted class ID and named label disagree with recipe label order",
                ));
            }
            support[class] += 1;
        }
    }
    Ok(support.into_iter().all(|count| count >= per_class))
}

#[cfg(test)]
mod blob_store_tests {
    use super::*;
    use serde_json::json;

    fn queued_job(repo: &TrainingRepository, stream_id: &str) -> TrainingJob {
        let stream = StreamKey {
            project_id: "blobs".into(),
            stream_id: stream_id.into(),
            inspection_version: "v1".into(),
        };
        for i in 0..6 {
            repo.record_sample(
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
        repo.trigger_after_n(
            &stream,
            6,
            TrainingRequest {
                engine: "fixture".into(),
                recipe: json!({}),
                compute: json!({}),
            },
            SplitPolicy::Group {
                train_fraction: 0.5,
                validation_fraction: 0.25,
                seed: 7,
            },
            now_ms(),
        )
        .unwrap()
        .unwrap()
    }

    fn stored(repo: &TrainingRepository, blob: &BlobRef) -> bool {
        repo.blob_dir.join(&blob.path).exists()
    }

    #[test]
    fn superseded_and_finished_checkpoints_are_collected_while_resumable_ones_stay() {
        let directory = tempfile::tempdir().unwrap();
        let repo = TrainingRepository::open(directory.path().join("jobs.sqlite")).unwrap();
        let job = queued_job(&repo, "line");
        let lease = repo.claim_job(&job.id, "worker", 60_000, now_ms()).unwrap();
        let first = repo
            .publish_checkpoint(&lease, b"epoch-1", 1, json!({}), false, now_ms())
            .unwrap();
        let second = repo
            .publish_checkpoint(&lease, b"epoch-2", 2, json!({}), false, now_ms())
            .unwrap();
        let orphan_bytes = b"written before a crash lost its reference";
        let orphan = repo.blob_dir.join(format!("{}.blob", "f".repeat(64)));
        std::fs::write(&orphan, orphan_bytes).unwrap();
        assert_eq!(
            repo.collect_unreferenced_blobs().unwrap(),
            first.blob.bytes + orphan_bytes.len() as u64
        );
        assert!(!stored(&repo, &first.blob));
        assert!(!orphan.exists());
        assert!(stored(&repo, &second.blob), "a running job resumes from it");
        let artifact = repo
            .finish_training(&lease, b"model", json!({}), now_ms())
            .unwrap();
        repo.collect_unreferenced_blobs().unwrap();
        assert!(!stored(&repo, &second.blob), "a finished job never resumes");
        assert!(stored(&repo, &artifact.blob));

        let cancelled = queued_job(&repo, "cancelled");
        let lease = repo
            .claim_job(&cancelled.id, "worker", 60_000, now_ms())
            .unwrap();
        let kept = repo
            .publish_checkpoint(&lease, b"cancelled-epoch", 1, json!({}), false, now_ms())
            .unwrap();
        repo.request_cancel(&cancelled.id, now_ms()).unwrap();
        repo.acknowledge_cancel(&lease, now_ms()).unwrap();
        repo.collect_unreferenced_blobs().unwrap();
        assert!(stored(&repo, &kept.blob), "a cancelled job resumes from it");
    }

    #[test]
    fn succeeded_trial_checkpoints_stay_while_their_experiment_can_continue() {
        let directory = tempfile::tempdir().unwrap();
        let repo = TrainingRepository::open(directory.path().join("jobs.sqlite")).unwrap();
        let job = queued_job(&repo, "trial");
        let lease = repo.claim_job(&job.id, "worker", 60_000, now_ms()).unwrap();
        let checkpoint = repo
            .publish_checkpoint(&lease, b"halving-parent", 3, json!({}), false, now_ms())
            .unwrap();
        repo.finish_training(&lease, b"trial-model", json!({}), now_ms())
            .unwrap();
        let connection = repo.connection().unwrap();
        connection
            .execute(
                "INSERT INTO experiments(id,scope,body) VALUES ('experiment','scope',?)",
                [json!({"status":"running","selected_trial_id":null}).to_string()],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO experiment_trials(id,experiment_id,job_id,idempotency_key,candidate_index,body) VALUES ('trial','experiment',?,'key',0,'{}')",
                [&job.id],
            )
            .unwrap();
        repo.collect_unreferenced_blobs().unwrap();
        assert!(stored(&repo, &checkpoint.blob));
        connection
            .execute(
                "UPDATE experiments SET body=? WHERE id='experiment'",
                [json!({"status":"evaluating","selected_trial_id":"trial"}).to_string()],
            )
            .unwrap();
        repo.collect_unreferenced_blobs().unwrap();
        assert!(!stored(&repo, &checkpoint.blob));
    }

    #[test]
    fn writers_in_flight_defer_collection_and_staging_sweeps() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("jobs.sqlite");
        let repo = TrainingRepository::open(&path).unwrap();
        let staged = repo.blob_dir.join("interrupted.part");
        let orphan = repo.blob_dir.join(format!("{}.blob", "e".repeat(64)));
        std::fs::write(&staged, b"partial").unwrap();
        std::fs::write(&orphan, b"orphan").unwrap();
        let writer = repo.blob_writer().unwrap();
        assert_eq!(repo.collect_unreferenced_blobs().unwrap(), 0);
        TrainingRepository::open(&path).unwrap();
        assert!(staged.exists() && orphan.exists());
        drop(writer);
        TrainingRepository::open(&path).unwrap();
        assert!(!staged.exists());
        assert!(orphan.exists(), "opening only sweeps staging files");
        assert_eq!(
            repo.collect_unreferenced_blobs().unwrap(),
            b"orphan".len() as u64
        );
    }

    #[test]
    fn journal_files_that_vanish_during_the_storage_check_count_as_absent() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let wal = root.join("jobs.sqlite-wal");
        std::fs::write(&wal, b"").unwrap();
        let metadata = std::fs::symlink_metadata(&wal).unwrap();
        // The last connection closes between the two lookups.
        std::fs::remove_file(&wal).unwrap();
        verify_present_entry(&root, &wal, false, &metadata).unwrap();
        std::fs::create_dir(&wal).unwrap();
        let metadata = std::fs::symlink_metadata(&wal).unwrap();
        assert!(matches!(
            verify_present_entry(&root, &wal, false, &metadata),
            Err(Error::Invalid(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn a_blob_directory_replaced_by_a_symlink_after_open_is_neither_written_nor_swept() {
        let directory = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let repo = TrainingRepository::open(directory.path().join("jobs.sqlite")).unwrap();
        std::fs::remove_dir(&repo.blob_dir).unwrap();
        std::os::unix::fs::symlink(outside.path(), &repo.blob_dir).unwrap();
        let staged = outside.path().join("unrelated.part");
        std::fs::write(&staged, b"not the ledger's").unwrap();
        let written = repo.register_pretrained_source(
            "project",
            b"weights",
            json!({"engine":"burn","format_version":1,"config":{},"input_shape":[1]}),
            json!({}),
            1024,
            now_ms(),
        );
        assert!(matches!(written, Err(Error::Invalid(_))), "{written:?}");
        assert_eq!(repo.collect_unreferenced_blobs().unwrap(), 0);
        let names: Vec<_> = std::fs::read_dir(outside.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, vec![std::ffi::OsString::from("unrelated.part")]);
        assert!(matches!(repo.verify_storage(), Err(Error::Invalid(_))));
    }

    #[test]
    fn ledger_connections_are_defensive_and_distrust_schema_functions() {
        let directory = tempfile::tempdir().unwrap();
        let repo = TrainingRepository::open(directory.path().join("jobs.sqlite")).unwrap();
        let connection = repo.connection().unwrap();
        assert!(
            connection
                .db_config(DbConfig::SQLITE_DBCONFIG_DEFENSIVE)
                .unwrap()
        );
        assert!(
            !connection
                .db_config(DbConfig::SQLITE_DBCONFIG_TRUSTED_SCHEMA)
                .unwrap()
        );
    }
}

#[cfg(test)]
mod sample_count_tests {
    use super::*;
    use serde_json::json;

    fn annotation(index: usize, revision: u64, accepted: bool, label_at_ms: i64) -> TrainingSample {
        TrainingSample {
            id: format!("sample-{index}"),
            annotation_revision: revision,
            group_id: format!("group-{index}"),
            captured_at_ms: 1,
            label_available_at_ms: label_at_ms,
            content_digest: format!("{index:064x}"),
            source: LabelSource::Reviewed,
            accepted,
            payload: json!({"label":"normal","input":{"shape":[2],"values":[0.5,1.5]}}),
        }
    }

    /// Train After N's former count: parse every eligible sample, then look up its trigger.
    fn parsed_count(connection: &Connection, scope: &str, at_ms: i64) -> usize {
        eligible_samples(connection, scope, at_ms)
            .unwrap()
            .iter()
            .filter(|(_, sample)| {
                !connection
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM triggered_samples WHERE scope=? AND sample_id=?)",
                        rusqlite::params![scope, sample.id],
                        |row| row.get::<_, bool>(0),
                    )
                    .unwrap()
            })
            .count()
    }

    #[test]
    fn new_sample_count_matches_the_parsed_count_and_reads_label_times_from_the_index() {
        let directory = tempfile::tempdir().unwrap();
        let repo = TrainingRepository::open(directory.path().join("jobs.sqlite")).unwrap();
        let stream = StreamKey {
            project_id: "counts".into(),
            stream_id: "line".into(),
            inspection_version: "v1".into(),
        };
        for sample in [
            annotation(0, 1, true, 10),
            // Withdrawn once its second revision is available.
            annotation(1, 1, true, 10),
            annotation(1, 2, false, 20),
            // Never accepted.
            annotation(2, 1, false, 10),
            // Accepted by a later revision.
            annotation(3, 1, false, 10),
            annotation(3, 2, true, 30),
            // Labelled only at 50.
            annotation(4, 1, true, 50),
            // Consumed by an earlier trigger.
            annotation(5, 1, true, 10),
            // Relabelled, accepted throughout.
            annotation(6, 1, true, 10),
            annotation(6, 2, true, 40),
        ] {
            repo.record_sample(&stream, &sample).unwrap();
        }
        let stream_scope = scope(&stream).unwrap();
        let connection = repo.connection().unwrap();
        connection
            .execute(
                "INSERT INTO triggered_samples(scope,sample_id,snapshot_id) VALUES (?,'sample-5','earlier')",
                [&stream_scope],
            )
            .unwrap();
        for (at_ms, expected) in [(5, 0), (15, 3), (25, 2), (35, 3), (45, 3), (55, 4)] {
            assert_eq!(
                repo.count_new_training_samples(&stream, at_ms).unwrap(),
                expected,
                "at {at_ms}"
            );
            assert_eq!(
                parsed_count(&connection, &stream_scope, at_ms),
                expected,
                "at {at_ms}"
            );
        }
        let plan = connection
            .prepare(&format!("EXPLAIN QUERY PLAN {NEW_SAMPLE_COUNT}"))
            .unwrap()
            .query_map(rusqlite::params![stream_scope, 0], |row| {
                row.get::<_, String>(3)
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert!(
            plan.iter()
                .any(|step| step.contains("annotation_label_availability")),
            "{plan:?}"
        );
    }
}

#[cfg(test)]
mod truth_digest_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn truth_digests_ignore_raw_row_edits_and_still_accept_full_body_seals() {
        let directory = tempfile::tempdir().unwrap();
        let repo = TrainingRepository::open(directory.path().join("jobs.sqlite")).unwrap();
        let stream = StreamKey {
            project_id: "truth".into(),
            stream_id: "line".into(),
            inspection_version: "v1".into(),
        };
        let recorded = TrainingSample {
            id: "audited".into(),
            annotation_revision: 1,
            group_id: "lot".into(),
            captured_at_ms: 1,
            label_available_at_ms: 1,
            content_digest: "a".repeat(64),
            source: LabelSource::Reviewed,
            accepted: true,
            payload: json!({"label":"good","feature_row":{"temperature":20}}),
        };
        repo.record_sample(&stream, &recorded).unwrap();
        let predictions = [PredictionRecord {
            sample_id: recorded.id.clone(),
            artifact_id: "model".into(),
            predicted_label: Some("good".into()),
            teacher_label: None,
            actual_label: Some("good".into()),
            actual_source: Some(LabelSource::Reviewed),
            latency_ms: 1.,
            failed: false,
            recorded_at_ms: 2,
            details: json!({}),
        }];
        let connection = repo.connection().unwrap();
        let verifies = |sealed: Option<&str>| {
            truth_digest_matches(&connection, &stream, &predictions, sealed).unwrap()
        };
        let sealed = evaluation_truth_digest(&connection, &stream, &predictions).unwrap();
        let legacy = digest(&serde_json::to_vec(&[Some(&recorded)]).unwrap());
        assert_ne!(sealed, legacy);
        assert!(verifies(Some(sealed.as_str())) && verifies(Some(legacy.as_str())));
        assert!(!verifies(None) && !verifies(Some("0".repeat(64).as_str())));

        let mut edited = recorded.clone();
        edited.payload["feature_row"] = json!({"temperature":21});
        repo.record_prepared_sample(&stream, &edited).unwrap();
        assert_eq!(
            evaluation_truth_digest(&connection, &stream, &predictions).unwrap(),
            sealed
        );
        assert!(verifies(Some(sealed.as_str())));
        assert!(
            !verifies(Some(legacy.as_str())),
            "a full-body digest covers the raw row"
        );

        let mut relabelled = edited;
        relabelled.annotation_revision = 2;
        relabelled.payload["label"] = json!("defect");
        repo.record_sample(&stream, &relabelled).unwrap();
        assert!(!verifies(Some(sealed.as_str())));
    }
}
