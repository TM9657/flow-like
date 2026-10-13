//! Explicit transfer of a device-local training ledger: a snapshot folder of `ledger.json`,
//! `jobs.sqlite` and `jobs.artifacts/{sha}.blob` that another device of the same app imports
//! into an empty ledger.

use flow_like::flow::{execution::context::ExecutionContext, node::NodeLogic};
use flow_like_catalog_core::FlowPath;
use flow_like_types::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const GIB: u64 = 1024 * 1024 * 1024;

fn default_import_limit() -> u64 {
    2 * GIB
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExportTrainingLedgerRequest {
    /// Folder that receives ledger.json, jobs.sqlite and jobs.artifacts/.
    pub destination: FlowPath,
    /// Export the executing user's private ledger instead of the app's.
    #[serde(default)]
    pub private: bool,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImportTrainingLedgerRequest {
    /// Folder written by Export Training Ledger.
    pub source: FlowPath,
    /// Fill the executing user's private ledger instead of the app's.
    #[serde(default)]
    pub private: bool,
    /// Largest total size of the database and blobs, 1 byte to 16 GiB.
    #[serde(default = "default_import_limit")]
    pub maximum_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TrainingLedgerTransfer {
    /// The database and blob files transferred.
    pub files: usize,
    pub bytes: u64,
    pub database_sha256: String,
    pub project_ids: Vec<String>,
}

#[cfg(feature = "training")]
pub(crate) use transfer::{export_training_ledger, import_training_ledger};

#[cfg(feature = "training")]
mod transfer {
    use super::{
        ExportTrainingLedgerRequest, GIB, ImportTrainingLedgerRequest, TrainingLedgerTransfer,
    };
    use crate::inspection::lifecycle::{
        LedgerLocation, LedgerScope, LedgerSite, LocalStorageTarget,
    };
    use crate::inspection::published::child;
    use crate::inspection::publisher::WriteTarget;
    use flow_like::flow::execution::context::ExecutionContext;
    use flow_like_catalog_core::FlowPath;
    use flow_like_ml_runtime::{
        LEDGER_BLOB_DIR, LEDGER_DATABASE_FILE, LedgerSnapshot, LedgerSnapshotFile,
        TrainingRepository, now_ms,
    };
    use flow_like_storage::{
        files::store::{FlowLikeStore, local_store::LocalObjectStore},
        object_store::{self, ObjectStoreExt, WriteMultipart},
    };
    use flow_like_types::{Bytes, Result, anyhow, json};
    use futures::StreamExt;
    use serde::{Deserialize, Serialize};
    use std::{
        collections::BTreeSet,
        path::{Path, PathBuf},
    };
    use tempfile::TempDir;

    const LEDGER_MANIFEST_FILE: &str = "ledger.json";
    const LEDGER_FORMAT_VERSION: u32 = 1;
    const MAXIMUM_LEDGER_MANIFEST_BYTES: u64 = 1024 * 1024;
    const MAXIMUM_IMPORT_BYTES: u64 = 16 * GIB;
    const PART_BYTES: usize = 8 * 1024 * 1024;
    const PARTS_IN_FLIGHT: usize = 4;
    /// Stores without multipart uploads receive files up to this size in one request.
    const SINGLE_UPLOAD_BYTES: u64 = 64 * 1024 * 1024;

    /// `ledger.json` of a ledger snapshot folder.
    #[derive(Clone, Debug, Serialize, Deserialize)]
    struct LedgerManifest {
        format_version: u32,
        scope: LedgerScope,
        app_id: String,
        project_ids: Vec<String>,
        database: LedgerSnapshotFile,
        blobs: Vec<LedgerSnapshotFile>,
        exported_at_ms: i64,
    }

    fn scope_of(private: bool) -> LedgerScope {
        if private {
            LedgerScope::User
        } else {
            LedgerScope::App
        }
    }

    fn scope_name(scope: &LedgerScope) -> &str {
        match scope {
            LedgerScope::App => "app",
            LedgerScope::User => "private",
        }
    }

    fn app_id(context: &ExecutionContext) -> Result<String> {
        context
            .execution_cache
            .as_ref()
            .map(|cache| cache.app_id.clone())
            .ok_or_else(|| anyhow!("Execution storage context is unavailable"))
    }

    fn is_sha256(value: &str) -> bool {
        value.len() == 64
            && value
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    }

    /// The files of a snapshot, each at the only path its role allows: `jobs.sqlite` and
    /// `jobs.artifacts/{sha}.blob`. No other stored path is ever followed.
    fn snapshot_files(
        database: &LedgerSnapshotFile,
        blobs: &[LedgerSnapshotFile],
    ) -> Result<Vec<LedgerSnapshotFile>> {
        if database.file != LEDGER_DATABASE_FILE || !is_sha256(&database.sha256) {
            return Err(anyhow!(
                "The ledger database must be {LEDGER_DATABASE_FILE} with a sha256, not '{}'",
                database.file
            ));
        }
        let mut seen = BTreeSet::new();
        for blob in blobs {
            if !is_sha256(&blob.sha256)
                || blob.file != format!("{LEDGER_BLOB_DIR}/{}.blob", blob.sha256)
                || blob.bytes == 0
                || !seen.insert(blob.sha256.as_str())
            {
                return Err(anyhow!(
                    "The ledger blob '{}' must be a nonempty {LEDGER_BLOB_DIR}/{{sha256}}.blob named by its own sha256, listed once",
                    blob.file
                ));
            }
        }
        Ok(std::iter::once(database).chain(blobs).cloned().collect())
    }

    /// Every project id must belong to this app and to the requested scope.
    fn check_project_ids(
        ids: impl IntoIterator<Item = impl AsRef<str>>,
        app_id: &str,
        scope: LedgerScope,
    ) -> Result<()> {
        let prefix = format!("{app_id}:");
        for id in ids {
            let id = id.as_ref();
            if !id.starts_with(&prefix) {
                return Err(anyhow!(
                    "The ledger holds project {id}, which belongs to another app than {app_id}; Import Training Ledger only fills a ledger of the same app"
                ));
            }
            let held = LedgerScope::of_project(id);
            if held != scope {
                return Err(anyhow!(
                    "The ledger holds project {id}, which belongs in the {} ledger, but the import fills the {} ledger; set private to match",
                    scope_name(&held),
                    scope_name(&scope)
                ));
            }
        }
        Ok(())
    }

    fn total_bytes(files: &[LedgerSnapshotFile]) -> u64 {
        files
            .iter()
            .fold(0u64, |total, file| total.saturating_add(file.bytes))
    }

    /// Where the scope's ledger lives in the training-ledgers layout, never the legacy App
    /// ledger, so staging folders and installs stay out of the app storage that copies walk.
    fn site_of(scope: LedgerScope) -> LedgerSite {
        match scope {
            LedgerScope::App => LedgerSite::App,
            LedgerScope::User => LedgerSite::User,
        }
    }

    /// The App, legacy App and private ledgers of this run.
    fn reachable_ledgers(context: &ExecutionContext) -> Result<Vec<LocalStorageTarget>> {
        [LedgerSite::App, LedgerSite::LegacyApp, LedgerSite::User]
            .into_iter()
            .map(|site| LocalStorageTarget::new(context, site))
            .collect()
    }

    fn ledger_folder(database: &Path) -> Result<PathBuf> {
        let folder = database.parent().ok_or_else(|| {
            anyhow!(
                "Training ledger path {} has no parent directory",
                database.display()
            )
        })?;
        Ok(folder.to_path_buf())
    }

    /// A new empty staging folder in `directory`, which is created when missing. Dropping the
    /// guard removes the folder, so a cancelled transfer leaves no copy behind. Blocking.
    fn create_staging(directory: &Path, prefix: &str) -> Result<TempDir> {
        let created = std::fs::create_dir_all(directory)
            .and_then(|()| TempDir::with_prefix_in(prefix, directory));
        created.map_err(|error| {
            anyhow!(
                "Creating a staging folder in {} failed: {error}",
                directory.display()
            )
        })
    }

    /// Removes a staging folder on a blocking thread; a folder that stays only warns.
    async fn remove_staging(staging: TempDir) {
        let folder = staging.path().display().to_string();
        let failure = match tokio::task::spawn_blocking(move || staging.close()).await {
            Ok(Ok(())) => return,
            Ok(Err(error)) if error.kind() == std::io::ErrorKind::NotFound => return,
            Ok(Err(error)) => error.to_string(),
            Err(error) => error.to_string(),
        };
        tracing::warn!(error = %failure, %folder, "Training ledger staging was not removed");
    }

    fn read_part(mut file: std::fs::File) -> std::io::Result<(std::fs::File, Vec<u8>)> {
        use std::io::Read;
        let mut part = Vec::with_capacity(PART_BYTES);
        file.by_ref()
            .take(PART_BYTES as u64)
            .read_to_end(&mut part)?;
        Ok((file, part))
    }

    /// Starts a multipart upload. A store without multipart uploads takes a file of at most
    /// 64 MiB in one request instead; None reports that upload as complete.
    async fn start_upload(
        target: &WriteTarget,
        path: &FlowPath,
        source: &Path,
        size: u64,
    ) -> Result<Option<Box<dyn object_store::MultipartUpload>>> {
        match target
            .store
            .as_generic()
            .put_multipart(&path.object_path())
            .await
        {
            Ok(upload) => Ok(Some(upload)),
            Err(
                object_store::Error::NotSupported { .. }
                | object_store::Error::NotImplemented { .. },
            ) if size <= SINGLE_UPLOAD_BYTES => {
                let read = source.to_path_buf();
                let bytes = tokio::task::spawn_blocking(move || std::fs::read(&read)).await??;
                if bytes.len() as u64 != size {
                    return Err(anyhow!(
                        "{} holds {} bytes, but the snapshot recorded {size}",
                        source.display(),
                        bytes.len()
                    ));
                }
                target
                    .write(path, Bytes::from(bytes), &mut Vec::new())
                    .await?;
                Ok(None)
            }
            Err(error) => Err(anyhow!(
                "Starting the upload of {} failed: {error}",
                path.path
            )),
        }
    }

    /// Streams `source` into the upload in 8 MiB parts, at most four in flight, refusing any
    /// size but `size`.
    async fn send_parts(
        writer: &mut WriteMultipart,
        path: &FlowPath,
        source: &Path,
        size: u64,
    ) -> Result<()> {
        let opened = source.to_path_buf();
        let mut file = tokio::task::spawn_blocking(move || std::fs::File::open(&opened)).await??;
        let mut sent = 0u64;
        loop {
            let (returned, part) = tokio::task::spawn_blocking(move || read_part(file)).await??;
            file = returned;
            if part.is_empty() {
                break;
            }
            sent = sent.saturating_add(part.len() as u64);
            if sent > size {
                return Err(anyhow!(
                    "{} grew beyond its {size} bytes while it was uploaded",
                    source.display()
                ));
            }
            writer
                .wait_for_capacity(PARTS_IN_FLIGHT)
                .await
                .map_err(|error| anyhow!("Uploading {} failed: {error}", path.path))?;
            writer.put(Bytes::from(part));
        }
        if sent != size {
            return Err(anyhow!(
                "{} holds {sent} bytes, but the snapshot recorded {size}",
                source.display()
            ));
        }
        Ok(())
    }

    /// Streams a local file to `path` in 8 MiB parts and confirms its size; never holds the
    /// whole file in memory unless the store cannot take multipart uploads.
    async fn upload_file(
        target: &WriteTarget,
        path: &FlowPath,
        source: PathBuf,
        size: u64,
    ) -> Result<()> {
        let Some(upload) = start_upload(target, path, &source, size).await? else {
            return Ok(());
        };
        let mut writer = WriteMultipart::new_with_chunk_size(upload, PART_BYTES);
        if let Err(error) = send_parts(&mut writer, path, &source, size).await {
            let _ = writer.abort().await;
            return Err(error);
        }
        let written = writer
            .finish()
            .await
            .map_err(|error| anyhow!("Completing the upload of {} failed: {error}", path.path))?;
        target.confirm(path, size, written.e_tag).await?;
        Ok(())
    }

    /// Writes the received chunks into the new file `destination` on a blocking thread.
    fn spawn_file_writer(
        destination: PathBuf,
        mut receiver: flow_like_types::sync::mpsc::Receiver<Bytes>,
    ) -> tokio::task::JoinHandle<Result<u64>> {
        tokio::task::spawn_blocking(move || -> Result<u64> {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&destination)?;
            let mut written = 0u64;
            while let Some(chunk) = receiver.blocking_recv() {
                file.write_all(&chunk)?;
                written = written.saturating_add(chunk.len() as u64);
            }
            file.sync_all()?;
            Ok(written)
        })
    }

    /// Sends the object's chunks to the file writer until the stream ends, a read fails or
    /// more than `expected` bytes arrived. Dropping `sender` ends the writer.
    async fn send_chunks(
        mut stream: impl futures::Stream<Item = object_store::Result<Bytes>> + Unpin,
        sender: flow_like_types::sync::mpsc::Sender<Bytes>,
        path: &FlowPath,
        expected: u64,
    ) -> Result<()> {
        let mut received = 0u64;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|error| anyhow!("Reading {} failed: {error}", path.path))?;
            received = received.saturating_add(chunk.len() as u64);
            if received > expected {
                return Err(anyhow!(
                    "{} holds more than the {expected} bytes ledger.json records",
                    path.path
                ));
            }
            if sender.send(chunk).await.is_err() {
                break;
            }
        }
        Ok(())
    }

    /// Streams `path` into the new local file `destination`, refusing any size but `expected`.
    async fn download_file(
        store: &FlowLikeStore,
        path: &FlowPath,
        destination: PathBuf,
        expected: u64,
    ) -> Result<()> {
        let fetched = store
            .as_generic()
            .get(&path.object_path())
            .await
            .map_err(|error| anyhow!("Reading {} failed: {error}", path.path))?;
        if fetched.meta.size != expected {
            return Err(anyhow!(
                "{} is {} bytes, but ledger.json records {expected}",
                path.path,
                fetched.meta.size
            ));
        }
        let (sender, receiver) = flow_like_types::sync::mpsc::channel::<Bytes>(4);
        let writer = spawn_file_writer(destination, receiver);
        let sent = send_chunks(fetched.into_stream(), sender, path, expected).await;
        let written = writer.await??;
        sent?;
        if written != expected {
            return Err(anyhow!(
                "{} arrived with {written} bytes, but ledger.json records {expected}",
                path.path
            ));
        }
        Ok(())
    }

    /// Writes a consistent snapshot of the scope's existing ledger to the destination folder,
    /// streaming each file and writing ledger.json last. The snapshot is staged beside the
    /// scope's ledger in the training-ledgers layout, even while the App scope still uses the
    /// legacy ledger in app storage.
    pub(crate) async fn export_training_ledger(
        context: &mut ExecutionContext,
        request: ExportTrainingLedgerRequest,
    ) -> Result<TrainingLedgerTransfer> {
        let scope = scope_of(request.private);
        let app_id = app_id(context)?;
        let location = LedgerLocation::new(context, scope)?;
        let ledgers = reachable_ledgers(context)?;
        let target = WriteTarget::new(context, request.destination.clone()).await?;
        check_destination(&target, &request.destination, ledgers).await?;
        let staging_target = LocalStorageTarget::new(context, site_of(scope))?;
        let (staging, snapshot) = staged_snapshot(location, staging_target, &app_id).await?;
        let exported = Exported {
            snapshot,
            staging: staging.path().to_path_buf(),
            scope,
            app_id,
        };
        let outcome = upload_snapshot(&target, &request.destination, exported).await;
        remove_staging(staging).await;
        outcome
    }

    /// Snapshots the scope's existing ledger on a blocking thread, refusing when this device
    /// holds none.
    async fn staged_snapshot(
        location: LedgerLocation,
        staging_target: LocalStorageTarget,
        app_id: &str,
    ) -> Result<(TempDir, LedgerSnapshot)> {
        let scope = location.scope();
        let snapshotting = move || snapshot_ledger(location, &staging_target);
        let snapshot = tokio::task::spawn_blocking(snapshotting).await??;
        snapshot.ok_or_else(|| {
            anyhow!(
                "This device holds no {} training ledger for app {app_id}, so there is nothing to export",
                scope_name(&scope)
            )
        })
    }

    /// Snapshots the scope's existing ledger into a new staging folder in the folder of
    /// `staging_target`; None when the ledger does not exist. Blocking.
    fn snapshot_ledger(
        location: LedgerLocation,
        staging_target: &LocalStorageTarget,
    ) -> Result<Option<(TempDir, LedgerSnapshot)>> {
        let Some((repo, _)) = location.open_existing()? else {
            return Ok(None);
        };
        let staging = create_staging(&ledger_folder(&staging_target.resolve()?)?, ".export-")?;
        let snapshot = repo.snapshot_to(staging.path())?;
        Ok(Some((staging, snapshot)))
    }

    /// Export never writes into a training ledger: the destination may be neither a ledger
    /// folder of this device nor inside or around one, and may not hold a jobs.sqlite without
    /// the ledger.json of an earlier export.
    async fn check_destination(
        target: &WriteTarget,
        destination: &FlowPath,
        ledgers: Vec<LocalStorageTarget>,
    ) -> Result<()> {
        if let FlowLikeStore::Local(store) = &target.store {
            refuse_ledger_overlap(store, destination, ledgers).await?;
        }
        let database = child(destination, LEDGER_DATABASE_FILE);
        let manifest = child(destination, LEDGER_MANIFEST_FILE);
        if target.head(&database).await?.is_some() && target.head(&manifest).await?.is_none() {
            return Err(anyhow!(
                "{} already holds a {LEDGER_DATABASE_FILE} without {LEDGER_MANIFEST_FILE}, so it may be a live training ledger; export into an empty folder, or delete that folder first if an earlier export was interrupted",
                destination.path
            ));
        }
        Ok(())
    }

    /// Refuses a destination in a Local store that is, contains or lies inside a ledger folder.
    async fn refuse_ledger_overlap(
        store: &LocalObjectStore,
        destination: &FlowPath,
        ledgers: Vec<LocalStorageTarget>,
    ) -> Result<()> {
        let folder = store
            .directory_to_filesystem(&destination.object_path())
            .map_err(|error| anyhow!("Resolving {} on disk failed: {error}", destination.path))?;
        let overlap = move || overlapping_ledger(&folder, &ledgers);
        let Some(ledger) = tokio::task::spawn_blocking(overlap).await?? else {
            return Ok(());
        };
        Err(anyhow!(
            "Export Training Ledger cannot write to {}, which is, contains or lies inside the training ledger folder {}; choose a separate folder",
            destination.path,
            ledger.display()
        ))
    }

    /// The first ledger folder that equals, contains or lies inside `folder`. Blocking.
    fn overlapping_ledger(
        folder: &Path,
        ledgers: &[LocalStorageTarget],
    ) -> Result<Option<PathBuf>> {
        let folder = comparable(folder);
        for ledger in ledgers {
            let directory = ledger_folder(&ledger.resolve()?)?;
            let compared = comparable(&directory);
            if folder.starts_with(&compared) || compared.starts_with(&folder) {
                return Ok(Some(directory));
            }
        }
        Ok(None)
    }

    /// `path` with its existing part resolved through symlinks and its ASCII case folded, so
    /// folders compare the way macOS and Windows open them. Blocking.
    fn comparable(path: &Path) -> PathBuf {
        let mut existing = path;
        let mut missing = Vec::new();
        while !existing.exists() {
            let (Some(parent), Some(name)) = (existing.parent(), existing.file_name()) else {
                break;
            };
            missing.push(name);
            existing = parent;
        }
        let mut resolved = existing
            .canonicalize()
            .unwrap_or_else(|_| existing.to_path_buf());
        resolved.extend(missing.iter().rev());
        PathBuf::from(resolved.to_string_lossy().to_ascii_lowercase())
    }

    /// A snapshot written into `staging`, ready to upload.
    struct Exported {
        snapshot: LedgerSnapshot,
        staging: PathBuf,
        scope: LedgerScope,
        app_id: String,
    }

    async fn upload_snapshot(
        target: &WriteTarget,
        destination: &FlowPath,
        exported: Exported,
    ) -> Result<TrainingLedgerTransfer> {
        let Exported {
            snapshot,
            staging,
            scope,
            app_id,
        } = exported;
        let files = snapshot_files(&snapshot.database, &snapshot.blobs)?;
        for file in &files {
            upload_file(
                target,
                &child(destination, &file.file),
                staging.join(&file.file),
                file.bytes,
            )
            .await?;
        }
        let manifest = LedgerManifest {
            format_version: LEDGER_FORMAT_VERSION,
            scope,
            app_id,
            project_ids: snapshot.project_ids.iter().cloned().collect(),
            database: snapshot.database.clone(),
            blobs: snapshot.blobs.clone(),
            exported_at_ms: now_ms(),
        };
        target
            .write(
                &child(destination, LEDGER_MANIFEST_FILE),
                Bytes::from(json::to_vec_pretty(&manifest)?),
                &mut Vec::new(),
            )
            .await?;
        Ok(TrainingLedgerTransfer {
            files: files.len(),
            bytes: total_bytes(&files),
            database_sha256: snapshot.database.sha256.clone(),
            project_ids: manifest.project_ids,
        })
    }

    /// The bytes of ledger.json, refused above 1 MiB by the size its GET reports before the
    /// body is read, and again after.
    async fn manifest_bytes(store: &FlowLikeStore, path: &FlowPath) -> Result<Bytes> {
        let failed = |error: object_store::Error| anyhow!("Reading {} failed: {error}", path.path);
        let fetched = store
            .as_generic()
            .get(&path.object_path())
            .await
            .map_err(failed)?;
        if fetched.meta.size > MAXIMUM_LEDGER_MANIFEST_BYTES {
            return Err(anyhow!(
                "{} is {} bytes, above its limit of {MAXIMUM_LEDGER_MANIFEST_BYTES} bytes",
                path.path,
                fetched.meta.size
            ));
        }
        let bytes = fetched.bytes().await.map_err(failed)?;
        if bytes.len() as u64 > MAXIMUM_LEDGER_MANIFEST_BYTES {
            return Err(anyhow!(
                "{} grew above its limit of {MAXIMUM_LEDGER_MANIFEST_BYTES} bytes",
                path.path
            ));
        }
        Ok(bytes)
    }

    /// Reads and checks ledger.json before any ledger file is transferred.
    async fn read_manifest(
        store: &FlowLikeStore,
        path: &FlowPath,
        scope: LedgerScope,
        app_id: &str,
        maximum_bytes: u64,
    ) -> Result<LedgerManifest> {
        let bytes = manifest_bytes(store, path).await?;
        let manifest: LedgerManifest = json::from_slice(&bytes)
            .map_err(|error| anyhow!("{} is not a training ledger manifest: {error}", path.path))?;
        check_manifest(&manifest, path, scope, app_id, maximum_bytes)?;
        Ok(manifest)
    }

    /// ledger.json must have this runtime's format, this app and the requested scope, list
    /// only project ids and files the rules allow, and fit `maximum_bytes`.
    fn check_manifest(
        manifest: &LedgerManifest,
        path: &FlowPath,
        scope: LedgerScope,
        app_id: &str,
        maximum_bytes: u64,
    ) -> Result<()> {
        if manifest.format_version != LEDGER_FORMAT_VERSION {
            return Err(anyhow!(
                "{} has format_version {}, but this runtime reads version {LEDGER_FORMAT_VERSION}",
                path.path,
                manifest.format_version
            ));
        }
        if manifest.app_id != app_id {
            return Err(anyhow!(
                "{} holds the training ledger of app {}, not of this app {app_id}",
                path.path,
                manifest.app_id
            ));
        }
        if manifest.scope != scope {
            return Err(anyhow!(
                "{} holds the {} training ledger, but the import fills the {} ledger; set private to match",
                path.path,
                scope_name(&manifest.scope),
                scope_name(&scope)
            ));
        }
        check_project_ids(&manifest.project_ids, app_id, scope)?;
        let total = total_bytes(&snapshot_files(&manifest.database, &manifest.blobs)?);
        if total > maximum_bytes {
            return Err(anyhow!(
                "The training ledger holds {total} bytes, above the import limit of {maximum_bytes} bytes"
            ));
        }
        Ok(())
    }

    /// The verified snapshot must be the one ledger.json describes.
    fn check_snapshot(snapshot: &LedgerSnapshot, manifest: &LedgerManifest) -> Result<()> {
        let files = |blobs: &[LedgerSnapshotFile]| {
            blobs
                .iter()
                .map(|blob| (blob.file.clone(), blob.sha256.clone(), blob.bytes))
                .collect::<BTreeSet<_>>()
        };
        let projects = manifest.project_ids.iter().collect::<BTreeSet<_>>();
        if snapshot.database != manifest.database
            || files(&snapshot.blobs) != files(&manifest.blobs)
            || snapshot.project_ids.iter().collect::<BTreeSet<_>>() != projects
        {
            return Err(anyhow!(
                "The transferred training ledger differs from what its ledger.json describes; export it again"
            ));
        }
        Ok(())
    }

    /// Fills the scope's empty ledger from a snapshot folder of the same app. The files are
    /// streamed into a staging folder beside the ledger, verified, then installed without
    /// replacing anything. The ledger and its staging always use the training-ledgers layout.
    pub(crate) async fn import_training_ledger(
        context: &mut ExecutionContext,
        request: ImportTrainingLedgerRequest,
    ) -> Result<TrainingLedgerTransfer> {
        if !(1..=MAXIMUM_IMPORT_BYTES).contains(&request.maximum_bytes) {
            return Err(anyhow!(
                "Import Training Ledger's byte limit must be between 1 byte and 16 GiB, not {}",
                request.maximum_bytes
            ));
        }
        let scope = scope_of(request.private);
        let app_id = app_id(context)?;
        let store = request.source.to_store(context).await?;
        let manifest = read_manifest(
            &store,
            &child(&request.source, LEDGER_MANIFEST_FILE),
            scope,
            &app_id,
            request.maximum_bytes,
        )
        .await?;
        let destination = empty_ledger(context, scope)?;
        let staging = create_import_staging(ledger_folder(&destination)?).await?;
        let import = Import {
            manifest,
            scope,
            app_id,
            staging: staging.path().to_path_buf(),
            destination,
            maximum_bytes: request.maximum_bytes,
        };
        let outcome = transfer_files(&store, &request.source, import).await;
        remove_staging(staging).await;
        outcome
    }

    /// The database path an import fills: the scope's ledger in the training-ledgers layout,
    /// which must hold nothing yet.
    fn empty_ledger(context: &ExecutionContext, scope: LedgerScope) -> Result<PathBuf> {
        let location = LedgerLocation::new(context, scope)?;
        let target = LocalStorageTarget::new(context, site_of(scope))?;
        let destination = target.resolve()?;
        refuse_filled_ledger(&location, &destination)?;
        Ok(destination)
    }

    /// Import only installs a ledger where none exists. The scope's ledger, for App also a
    /// legacy ledger in use, and any journal or blob folder left at `destination` refuse it
    /// before a download.
    fn refuse_filled_ledger(location: &LedgerLocation, destination: &Path) -> Result<()> {
        let scope = location.scope();
        if location.exists()? {
            return Err(anyhow!(
                "The {} training ledger at {} already exists; Import Training Ledger only installs a ledger where none exists, so import before training on this device",
                scope_name(&scope),
                location.database_path()?.display()
            ));
        }
        if let Some(leftover) = ledger_leftover(destination)? {
            return Err(anyhow!(
                "The {} training ledger at {} already exists in part: {} is present, perhaps from an interrupted import; Import Training Ledger only installs a ledger where none exists, so remove it first",
                scope_name(&scope),
                destination.display(),
                leftover.display()
            ));
        }
        Ok(())
    }

    /// The first entry an earlier or interrupted ledger left at `database`: the database, its
    /// journals or its blob folder, each of which an install refuses.
    fn ledger_leftover(database: &Path) -> Result<Option<PathBuf>> {
        let files = ["", "-journal", "-wal", "-shm"].map(|suffix| {
            let mut file = database.as_os_str().to_owned();
            file.push(suffix);
            PathBuf::from(file)
        });
        let blobs = database.with_file_name(LEDGER_BLOB_DIR);
        for entry in files.into_iter().chain([blobs]) {
            match std::fs::symlink_metadata(&entry) {
                Ok(_) => return Ok(Some(entry)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(anyhow!("Checking {} failed: {error}", entry.display())),
            }
        }
        Ok(None)
    }

    /// An import whose ledger.json was checked, with its staging folder and the ledger it fills.
    struct Import {
        manifest: LedgerManifest,
        scope: LedgerScope,
        app_id: String,
        staging: PathBuf,
        destination: PathBuf,
        maximum_bytes: u64,
    }

    /// A new staging folder holding an empty blob folder in `directory`, which is created when
    /// missing.
    async fn create_import_staging(directory: PathBuf) -> Result<TempDir> {
        tokio::task::spawn_blocking(move || -> Result<TempDir> {
            let staging = create_staging(&directory, ".import-")?;
            std::fs::create_dir(staging.path().join(LEDGER_BLOB_DIR))?;
            Ok(staging)
        })
        .await?
    }

    /// Verifies the staged snapshot against ledger.json and the project rules, then installs
    /// it. Blocking.
    fn verify_and_install(import: Import) -> Result<LedgerSnapshot> {
        let snapshot = TrainingRepository::verify_snapshot(&import.staging, import.maximum_bytes)?;
        check_snapshot(&snapshot, &import.manifest)?;
        check_project_ids(&snapshot.project_ids, &import.app_id, import.scope)?;
        install(&import.staging, &import.destination)?;
        Ok(snapshot)
    }

    /// Streams the files into the staging folder, verifies them against ledger.json and the
    /// project rules, then installs the ledger.
    async fn transfer_files(
        store: &FlowLikeStore,
        source: &FlowPath,
        import: Import,
    ) -> Result<TrainingLedgerTransfer> {
        let files = snapshot_files(&import.manifest.database, &import.manifest.blobs)?;
        for file in &files {
            download_file(
                store,
                &child(source, &file.file),
                import.staging.join(&file.file),
                file.bytes,
            )
            .await?;
        }
        let snapshot = tokio::task::spawn_blocking(move || verify_and_install(import)).await??;
        Ok(TrainingLedgerTransfer {
            files: files.len(),
            bytes: total_bytes(&files),
            database_sha256: snapshot.database.sha256.clone(),
            project_ids: snapshot.project_ids.iter().cloned().collect(),
        })
    }

    /// Installs the verified snapshot. A conflict means another ledger appeared meanwhile.
    fn install(folder: &Path, destination: &Path) -> Result<()> {
        TrainingRepository::install_snapshot(folder, destination).map_err(|error| match error {
            flow_like_ml_runtime::Error::Conflict(reason) => anyhow!(
                "The training ledger at {} already exists ({reason}); Import Training Ledger only installs a ledger where none exists",
                destination.display()
            ),
            error => error.into(),
        })
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::inspection::lifecycle::scoped_repository;
        use crate::inspection::lifecycle::test_device::{Device, plain_context};
        use flow_like_types::{json::json, storage_paths::is_legacy_training_ledger_path};
        use futures::FutureExt;
        use std::{
            sync::Arc,
            time::{Duration, Instant},
        };

        const PROJECT: &str = "app:board";
        const APP_LEDGER: &str = "training-ledgers/apps/app/project/jobs.sqlite";
        const APP_LEDGER_FOLDER: &str = "training-ledgers/apps/app/project";
        const LEGACY_LEDGER: &str = "apps/app/storage/inspection/jobs.sqlite";

        fn register_source(repo: &TrainingRepository, project: &str) -> String {
            let manifest = json!({
                "engine": "burn", "format_version": 1, "config": {}, "input_shape": [1]
            });
            repo.register_pretrained_source(
                project,
                b"weights",
                manifest,
                json!({"source": "fixture"}),
                1024,
                now_ms(),
            )
            .unwrap()
            .id
        }

        fn transfer_folder(folder: &str) -> FlowPath {
            FlowPath {
                path: folder.into(),
                store_ref: "transfer".into(),
                cache_store_ref: None,
            }
        }

        /// A run on `device` that reaches the folder `folder` as the FlowPath store `transfer`.
        async fn connected(device: &Device, folder: &Path) -> ExecutionContext {
            let context = plain_context(device.stores()).await;
            let store = LocalObjectStore::new(folder.to_path_buf()).unwrap();
            context
                .set_cache("transfer", Arc::new(FlowLikeStore::Local(Arc::new(store))))
                .await;
            context
        }

        async fn export(
            context: &mut ExecutionContext,
            destination: FlowPath,
            private: bool,
        ) -> Result<TrainingLedgerTransfer> {
            let request = ExportTrainingLedgerRequest {
                destination,
                private,
            };
            export_training_ledger(context, request).await
        }

        async fn export_to(
            context: &mut ExecutionContext,
            folder: &str,
        ) -> Result<TrainingLedgerTransfer> {
            export(context, transfer_folder(folder), false).await
        }

        async fn import(
            context: &mut ExecutionContext,
            folder: &str,
        ) -> Result<TrainingLedgerTransfer> {
            let request = ImportTrainingLedgerRequest {
                source: transfer_folder(folder),
                private: false,
                maximum_bytes: GIB,
            };
            import_training_ledger(context, request).await
        }

        fn refusal(outcome: Result<TrainingLedgerTransfer>) -> String {
            outcome.err().expect("the transfer is refused").to_string()
        }

        /// The first `.export-` or `.import-` staging folder in `folder`.
        fn staging_in(folder: &Path) -> Option<PathBuf> {
            let staging = |path: &PathBuf| {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                name.starts_with(".export-") || name.starts_with(".import-")
            };
            std::fs::read_dir(folder)
                .ok()?
                .flatten()
                .map(|entry| entry.path())
                .find(staging)
        }

        /// Files below `folder`, relative to it and joined with `/`.
        fn files_below(folder: &Path) -> Vec<String> {
            let mut found = Vec::new();
            let mut pending = vec![folder.to_path_buf()];
            while let Some(directory) = pending.pop() {
                let Ok(entries) = std::fs::read_dir(&directory) else {
                    continue;
                };
                for path in entries.flatten().map(|entry| entry.path()) {
                    if path.is_dir() {
                        pending.push(path);
                    } else if let Ok(relative) = path.strip_prefix(folder) {
                        found.push(relative.to_string_lossy().replace('\\', "/"));
                    }
                }
            }
            found
        }

        /// Nothing in the app storage of `device` would travel with a copy of the app.
        fn assert_app_storage_holds_only_the_legacy_ledger(device: &Device) {
            for file in files_below(&device.root().join("apps/app")) {
                assert!(
                    is_legacy_training_ledger_path(&file),
                    "{file} would travel with copies of the app"
                );
            }
        }

        #[tokio::test]
        async fn cancelled_legacy_ledger_exports_stage_outside_app_storage_and_clean_up() {
            let device = Device::new();
            let mut context = connected(&device, &device.scratch("transfer")).await;
            let legacy = TrainingRepository::open(device.root().join(LEGACY_LEDGER)).unwrap();
            register_source(&legacy, PROJECT);
            let staging_folder = device.root().join(APP_LEDGER_FOLDER);
            let deadline = Instant::now() + Duration::from_secs(60);
            {
                let cancelled = export_to(&mut context, "ledger");
                let mut running = std::pin::pin!(cancelled);
                while staging_in(&staging_folder).is_none() {
                    if let Some(outcome) = running.as_mut().now_or_never() {
                        panic!(
                            "the export ended before its staging folder was seen: {:?}",
                            outcome.map(|_| ())
                        );
                    }
                    assert!(Instant::now() < deadline, "no staging folder appeared");
                    tokio::task::yield_now().await;
                }
                assert_app_storage_holds_only_the_legacy_ledger(&device);
            }
            while staging_in(&staging_folder).is_some() {
                assert!(
                    Instant::now() < deadline,
                    "the cancelled export left its staging folder"
                );
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(!device.root().join(APP_LEDGER).exists());
        }

        #[tokio::test]
        async fn legacy_app_ledgers_round_trip_and_leave_no_staging_on_either_device() {
            let origin = Device::new();
            let transfer = origin.scratch("transfer");
            let mut source = connected(&origin, &transfer).await;
            let legacy = TrainingRepository::open(origin.root().join(LEGACY_LEDGER)).unwrap();
            let id = register_source(&legacy, PROJECT);
            let exported = export_to(&mut source, "ledger").await.unwrap();
            assert_eq!(exported.files, 2);
            assert_eq!(exported.project_ids, [PROJECT]);
            let device = Device::new();
            let mut target = connected(&device, &transfer).await;
            assert_eq!(import(&mut target, "ledger").await.unwrap(), exported);
            let (installed, _) = scoped_repository(&target, LedgerScope::App).unwrap();
            let copied = installed.get_pretrained_source(&id).unwrap();
            assert_eq!(installed.read_blob(&copied.blob).unwrap(), b"weights");
            for root in [origin.root(), device.root()] {
                assert_eq!(staging_in(&root.join(APP_LEDGER_FOLDER)), None);
            }
            assert_app_storage_holds_only_the_legacy_ledger(&origin);
            assert!(!origin.root().join(APP_LEDGER).exists());
            assert!(!device.root().join("apps").exists());
        }

        #[tokio::test]
        async fn exports_refuse_destinations_on_inside_or_around_a_ledger_folder() {
            let device = Device::new();
            let mut context = connected(&device, &device.scratch("transfer")).await;
            let legacy = TrainingRepository::open(device.root().join(LEGACY_LEDGER)).unwrap();
            let id = register_source(&legacy, PROJECT);
            let (user_ledger, project) = scoped_repository(&context, LedgerScope::User).unwrap();
            register_source(&user_ledger, &project);
            let storage = FlowPath::from_storage_dir(&mut context, false)
                .await
                .unwrap();
            let user_folder = FlowPath {
                path: "training-ledgers/apps/app/users/user-1".into(),
                ..storage.clone()
            };
            let destinations = [
                (child(&storage, "inspection"), false),
                (child(&storage, "inspection"), true),
                (child(&storage, "Inspection/jobs.artifacts"), false),
                (storage.clone(), false),
                (user_folder, false),
            ];
            for (destination, private) in destinations {
                let shown = destination.path.clone();
                let refused = refusal(export(&mut context, destination, private).await);
                assert!(
                    refused.contains("training ledger folder"),
                    "{shown}: {refused}"
                );
            }
            assert_app_storage_holds_only_the_legacy_ledger(&device);
            let reopened = TrainingRepository::open(device.root().join(LEGACY_LEDGER)).unwrap();
            reopened.get_pretrained_source(&id).unwrap();
        }

        #[tokio::test]
        async fn exports_refuse_a_folder_holding_a_database_without_ledger_json() {
            let device = Device::new();
            let transfer = device.scratch("transfer");
            let mut context = connected(&device, &transfer).await;
            let (repo, project) = scoped_repository(&context, LedgerScope::App).unwrap();
            register_source(&repo, &project);
            let folder = transfer.join("ledger");
            std::fs::create_dir_all(&folder).unwrap();
            std::fs::write(folder.join(LEDGER_DATABASE_FILE), b"live").unwrap();
            let refused = refusal(export_to(&mut context, "ledger").await);
            assert!(refused.contains("without ledger.json"), "{refused}");
            let kept = std::fs::read(folder.join(LEDGER_DATABASE_FILE)).unwrap();
            assert_eq!(kept, b"live");
            std::fs::write(folder.join(LEDGER_MANIFEST_FILE), b"{}").unwrap();
            let exported = export_to(&mut context, "ledger").await.unwrap();
            assert_eq!(exported.files, 2);
        }

        #[tokio::test]
        async fn imports_name_a_leftover_blob_folder_or_journal_before_downloading() {
            let origin = Device::new();
            let transfer = origin.scratch("transfer");
            let mut source = connected(&origin, &transfer).await;
            let (repo, project) = scoped_repository(&source, LedgerScope::App).unwrap();
            register_source(&repo, &project);
            export_to(&mut source, "ledger").await.unwrap();
            std::fs::remove_file(transfer.join("ledger").join(LEDGER_DATABASE_FILE)).unwrap();
            for leftover in [LEDGER_BLOB_DIR, "jobs.sqlite-wal"] {
                let device = Device::new();
                let mut target = connected(&device, &transfer).await;
                let folder = device.root().join(APP_LEDGER_FOLDER);
                let path = folder.join(leftover);
                std::fs::create_dir_all(&folder).unwrap();
                let created = if leftover == LEDGER_BLOB_DIR {
                    std::fs::create_dir(&path)
                } else {
                    std::fs::write(&path, b"")
                };
                created.unwrap();
                let refused = refusal(import(&mut target, "ledger").await);
                assert!(refused.contains(&path.display().to_string()), "{refused}");
                assert!(refused.contains("already exists"), "{refused}");
                assert_eq!(staging_in(&folder), None);
            }
        }

        #[tokio::test]
        async fn imports_refuse_a_ledger_json_above_one_mebibyte() {
            let device = Device::new();
            let transfer = device.scratch("transfer");
            let mut context = connected(&device, &transfer).await;
            let size = MAXIMUM_LEDGER_MANIFEST_BYTES + 1;
            std::fs::create_dir_all(transfer.join("ledger")).unwrap();
            let manifest = transfer.join("ledger").join(LEDGER_MANIFEST_FILE);
            std::fs::write(manifest, vec![b' '; size as usize]).unwrap();
            let refused = refusal(import(&mut context, "ledger").await);
            assert!(refused.contains(&format!("is {size} bytes")), "{refused}");
        }
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct ExportTrainingLedgerNode;

#[flow_like_types::async_trait]
impl NodeLogic for ExportTrainingLedgerNode {
    fn get_node(&self) -> flow_like::flow::node::Node {
        super::operation_node::<ExportTrainingLedgerRequest, TrainingLedgerTransfer>(
            "ml_export_training_ledger",
            "Export Training Ledger",
            "Write a consistent snapshot of this device's training ledger (the app's, or your private one) to a folder: ledger.json, jobs.sqlite and every referenced blob, streamed in parts",
            "exportTrainingLedger",
            "AI/ML/Export",
        )
    }

    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "training")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request: ExportTrainingLedgerRequest = context.evaluate_pin("request").await?;
            let result = export_training_ledger(context, request).await?;
            context
                .set_pin_value("result", flow_like_types::json::json!(result))
                .await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }
        #[cfg(not(feature = "training"))]
        {
            let _ = context;
            Err(super::lifecycle::training_unavailable())
        }
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct ImportTrainingLedgerNode;

#[flow_like_types::async_trait]
impl NodeLogic for ImportTrainingLedgerNode {
    fn get_node(&self) -> flow_like::flow::node::Node {
        super::operation_node::<ImportTrainingLedgerRequest, TrainingLedgerTransfer>(
            "ml_import_training_ledger",
            "Import Training Ledger",
            "Fill this device's empty training ledger of the same app (the app's, or your private one) from an Export Training Ledger folder after checking every project id, file size and sha256; an existing ledger is never replaced",
            "importTrainingLedger",
            "AI/ML/Export",
        )
    }

    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "training")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request: ImportTrainingLedgerRequest = context.evaluate_pin("request").await?;
            let result = import_training_ledger(context, request).await?;
            context
                .set_pin_value("result", flow_like_types::json::json!(result))
                .await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }
        #[cfg(not(feature = "training"))]
        {
            let _ = context;
            Err(super::lifecycle::training_unavailable())
        }
    }
}
