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
    use crate::inspection::lifecycle::{LedgerLocation, LedgerScope};
    use crate::inspection::published::child;
    use crate::inspection::publisher::WriteTarget;
    use flow_like::flow::execution::context::ExecutionContext;
    use flow_like_catalog_core::FlowPath;
    use flow_like_ml_runtime::{
        LEDGER_BLOB_DIR, LEDGER_DATABASE_FILE, LedgerSnapshot, LedgerSnapshotFile,
        TrainingRepository, now_ms,
    };
    use flow_like_storage::{
        files::store::FlowLikeStore,
        object_store::{self, ObjectStoreExt, WriteMultipart},
    };
    use flow_like_types::{Bytes, Result, anyhow, json};
    use futures::StreamExt;
    use serde::{Deserialize, Serialize};
    use std::{
        collections::BTreeSet,
        path::{Path, PathBuf},
    };

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

    fn scope_name(scope: LedgerScope) -> &'static str {
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
    fn snapshot_files<'a>(
        database: &'a LedgerSnapshotFile,
        blobs: &'a [LedgerSnapshotFile],
    ) -> Result<Vec<&'a LedgerSnapshotFile>> {
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
        Ok(std::iter::once(database).chain(blobs).collect())
    }

    /// Every project id must belong to this app and to the requested scope.
    fn check_project_ids<'a>(
        ids: impl IntoIterator<Item = &'a String>,
        app_id: &str,
        scope: LedgerScope,
    ) -> Result<()> {
        let prefix = format!("{app_id}:");
        for id in ids {
            if !id.starts_with(&prefix) {
                return Err(anyhow!(
                    "The ledger holds project {id}, which belongs to another app than {app_id}; Import Training Ledger only fills a ledger of the same app"
                ));
            }
            if LedgerScope::of_project(id) != scope {
                return Err(anyhow!(
                    "The ledger holds project {id}, which belongs in the {} ledger, but the import fills the {} ledger; set private to match",
                    scope_name(LedgerScope::of_project(id)),
                    scope_name(scope)
                ));
            }
        }
        Ok(())
    }

    fn total_bytes(files: &[&LedgerSnapshotFile]) -> u64 {
        files
            .iter()
            .fold(0u64, |total, file| total.saturating_add(file.bytes))
    }

    async fn remove_directory(directory: PathBuf) {
        let removed = tokio::task::spawn_blocking(move || {
            match std::fs::remove_dir_all(&directory) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(anyhow!(
                    "Removing the staging folder {} failed: {error}",
                    directory.display()
                )),
            }
        })
        .await;
        match removed {
            Ok(Ok(())) => {}
            Ok(Err(error)) => tracing::warn!(%error, "Training ledger staging was not removed"),
            Err(error) => tracing::warn!(%error, "Training ledger staging was not removed"),
        }
    }

    fn read_part(mut file: std::fs::File) -> std::io::Result<(std::fs::File, Vec<u8>)> {
        use std::io::Read;
        let mut part = Vec::with_capacity(PART_BYTES);
        file.by_ref()
            .take(PART_BYTES as u64)
            .read_to_end(&mut part)?;
        Ok((file, part))
    }

    /// Streams a local file to `path` in 8 MiB parts and confirms its size; never holds the
    /// whole file in memory unless the store cannot take multipart uploads.
    async fn upload_file(
        target: &WriteTarget,
        path: &FlowPath,
        source: PathBuf,
        size: u64,
    ) -> Result<()> {
        let location = path.object_path();
        let upload = match target.store.as_generic().put_multipart(&location).await {
            Ok(upload) => upload,
            Err(
                object_store::Error::NotSupported { .. }
                | object_store::Error::NotImplemented { .. },
            ) if size <= SINGLE_UPLOAD_BYTES => {
                let bytes = tokio::task::spawn_blocking(move || std::fs::read(&source)).await??;
                target
                    .write(path, Bytes::from(bytes), &mut Vec::new())
                    .await?;
                return Ok(());
            }
            Err(error) => {
                return Err(anyhow!(
                    "Starting the upload of {} failed: {error}",
                    path.path
                ));
            }
        };
        let mut writer = WriteMultipart::new_with_chunk_size(upload, PART_BYTES);
        let opened = source.clone();
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
                let _ = writer.abort().await;
                return Err(anyhow!(
                    "{} grew beyond its {size} bytes while it was uploaded",
                    source.display()
                ));
            }
            if let Err(error) = writer.wait_for_capacity(PARTS_IN_FLIGHT).await {
                let _ = writer.abort().await;
                return Err(anyhow!("Uploading {} failed: {error}", path.path));
            }
            writer.put(Bytes::from(part));
        }
        if sent != size {
            let _ = writer.abort().await;
            return Err(anyhow!(
                "{} holds {sent} bytes, but the snapshot recorded {size}",
                source.display()
            ));
        }
        let written = writer
            .finish()
            .await
            .map_err(|error| anyhow!("Completing the upload of {} failed: {error}", path.path))?;
        target.confirm(path, size, written.e_tag).await?;
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
        let mut stream = fetched.into_stream();
        let (sender, mut receiver) = flow_like_types::sync::mpsc::channel::<Bytes>(4);
        let target = destination.clone();
        let writer = tokio::task::spawn_blocking(move || -> Result<u64> {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)?;
            let mut written = 0u64;
            while let Some(chunk) = receiver.blocking_recv() {
                file.write_all(&chunk)?;
                written = written.saturating_add(chunk.len() as u64);
            }
            file.sync_all()?;
            Ok(written)
        });
        let mut received = 0u64;
        let mut failure = None;
        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(error) => {
                    failure = Some(anyhow!("Reading {} failed: {error}", path.path));
                    break;
                }
            };
            received = received.saturating_add(chunk.len() as u64);
            if received > expected {
                failure = Some(anyhow!(
                    "{} holds more than the {expected} bytes ledger.json records",
                    path.path
                ));
                break;
            }
            if sender.send(chunk).await.is_err() {
                break;
            }
        }
        drop(sender);
        let written = writer.await??;
        if let Some(failure) = failure {
            return Err(failure);
        }
        if written != expected {
            return Err(anyhow!(
                "{} arrived with {written} bytes, but ledger.json records {expected}",
                path.path
            ));
        }
        Ok(())
    }

    /// Writes a consistent snapshot of the scope's existing ledger to the destination folder,
    /// streaming each file and writing ledger.json last.
    pub(crate) async fn export_training_ledger(
        context: &mut ExecutionContext,
        request: ExportTrainingLedgerRequest,
    ) -> Result<TrainingLedgerTransfer> {
        let scope = scope_of(request.private);
        let app_id = app_id(context)?;
        let location = LedgerLocation::new(context, scope)?;
        let staging = location
            .directory()?
            .join(format!(".export-{}", uuid::Uuid::new_v4()));
        let created = staging.clone();
        let snapshot = tokio::task::spawn_blocking(move || -> Result<Option<LedgerSnapshot>> {
            let Some((repo, _)) = location.open_existing()? else {
                return Ok(None);
            };
            std::fs::create_dir(&created)?;
            Ok(Some(repo.snapshot_to(&created)?))
        })
        .await;
        let outcome = async {
            let snapshot = snapshot??.ok_or_else(|| {
                anyhow!(
                    "This device holds no {} training ledger for app {app_id}, so there is nothing to export",
                    scope_name(scope)
                )
            })?;
            let files = snapshot_files(&snapshot.database, &snapshot.blobs)?;
            let target = WriteTarget::new(context, request.destination.clone()).await?;
            for file in &files {
                upload_file(
                    &target,
                    &child(&request.destination, &file.file),
                    staging.join(&file.file),
                    file.bytes,
                )
                .await?;
            }
            let manifest = LedgerManifest {
                format_version: LEDGER_FORMAT_VERSION,
                scope,
                app_id: app_id.clone(),
                project_ids: snapshot.project_ids.iter().cloned().collect(),
                database: snapshot.database.clone(),
                blobs: snapshot.blobs.clone(),
                exported_at_ms: now_ms(),
            };
            target
                .write(
                    &child(&request.destination, LEDGER_MANIFEST_FILE),
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
        .await;
        remove_directory(staging).await;
        outcome
    }

    /// Reads and checks ledger.json before any ledger file is transferred.
    async fn read_manifest(
        store: &FlowLikeStore,
        path: &FlowPath,
        scope: LedgerScope,
        app_id: &str,
        maximum_bytes: u64,
    ) -> Result<LedgerManifest> {
        let location = path.object_path();
        let meta = store
            .as_generic()
            .head(&location)
            .await
            .map_err(|error| anyhow!("Reading {} failed: {error}", path.path))?;
        if meta.size > MAXIMUM_LEDGER_MANIFEST_BYTES {
            return Err(anyhow!(
                "{} is {} bytes, above its limit of {MAXIMUM_LEDGER_MANIFEST_BYTES} bytes",
                path.path,
                meta.size
            ));
        }
        let bytes = store
            .as_generic()
            .get(&location)
            .await
            .map_err(|error| anyhow!("Reading {} failed: {error}", path.path))?
            .bytes()
            .await
            .map_err(|error| anyhow!("Reading {} failed: {error}", path.path))?;
        if bytes.len() as u64 > MAXIMUM_LEDGER_MANIFEST_BYTES {
            return Err(anyhow!(
                "{} grew above its limit of {MAXIMUM_LEDGER_MANIFEST_BYTES} bytes",
                path.path
            ));
        }
        let manifest: LedgerManifest = json::from_slice(&bytes)
            .map_err(|error| anyhow!("{} is not a training ledger manifest: {error}", path.path))?;
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
                scope_name(manifest.scope),
                scope_name(scope)
            ));
        }
        check_project_ids(&manifest.project_ids, app_id, scope)?;
        let files = snapshot_files(&manifest.database, &manifest.blobs)?;
        let total = total_bytes(&files);
        if total > maximum_bytes {
            return Err(anyhow!(
                "The training ledger holds {total} bytes, above the import limit of {maximum_bytes} bytes"
            ));
        }
        Ok(manifest)
    }

    /// The installed snapshot must be the one ledger.json describes.
    fn check_snapshot(snapshot: &LedgerSnapshot, manifest: &LedgerManifest) -> Result<()> {
        let listed = manifest.blobs.iter().collect::<BTreeSet<_>>();
        let verified = snapshot.blobs.iter().collect::<BTreeSet<_>>();
        let projects = manifest.project_ids.iter().collect::<BTreeSet<_>>();
        if snapshot.database != manifest.database
            || listed != verified
            || snapshot.project_ids.iter().collect::<BTreeSet<_>>() != projects
        {
            return Err(anyhow!(
                "The transferred training ledger differs from what ledger.json describes; export it again"
            ));
        }
        Ok(())
    }

    /// Fills the scope's empty ledger from a snapshot folder of the same app. The files are
    /// streamed into a staging folder beside the ledger, verified, then installed without
    /// replacing anything.
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
        let location = LedgerLocation::new(context, scope)?;
        let destination = location.database_path()?;
        if location.exists()? {
            return Err(anyhow!(
                "The {} training ledger at {} already exists; Import Training Ledger only fills an empty ledger, so import before training on this device",
                scope_name(scope),
                destination.display()
            ));
        }
        let directory = location.directory()?;
        let staging = directory.join(format!(".import-{}", uuid::Uuid::new_v4()));
        let prepared = {
            let staging = staging.clone();
            tokio::task::spawn_blocking(move || -> Result<()> {
                std::fs::create_dir_all(&directory)?;
                std::fs::create_dir(&staging)?;
                std::fs::create_dir(staging.join(LEDGER_BLOB_DIR))?;
                Ok(())
            })
            .await
        };
        let outcome = async {
            prepared??;
            let files = snapshot_files(&manifest.database, &manifest.blobs)?;
            for file in &files {
                download_file(
                    &store,
                    &child(&request.source, &file.file),
                    staging.join(&file.file),
                    file.bytes,
                )
                .await?;
            }
            let (folder, target, maximum) =
                (staging.clone(), destination.clone(), request.maximum_bytes);
            let snapshot = tokio::task::spawn_blocking(move || -> Result<LedgerSnapshot> {
                let snapshot = TrainingRepository::verify_snapshot(&folder, maximum)?;
                install(&folder, &target)?;
                Ok(snapshot)
            });
            let snapshot = snapshot.await??;
            Ok((snapshot, total_bytes(&files), files.len()))
        }
        .await;
        remove_directory(staging).await;
        let (snapshot, bytes, count) = outcome?;
        let installed = Ok(TrainingLedgerTransfer {
            files: count,
            bytes,
            database_sha256: snapshot.database.sha256.clone(),
            project_ids: snapshot.project_ids.iter().cloned().collect(),
        });
        check_snapshot(&snapshot, &manifest).and(installed)
    }

    /// Installs the verified snapshot. A conflict means another ledger appeared meanwhile.
    fn install(folder: &Path, destination: &Path) -> Result<()> {
        TrainingRepository::install_snapshot(folder, destination).map_err(|error| match error {
            flow_like_ml_runtime::Error::Conflict(reason) => anyhow!(
                "The training ledger at {} already exists ({reason}); Import Training Ledger only fills an empty ledger",
                destination.display()
            ),
            error => error.into(),
        })
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
