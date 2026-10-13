use crate::{
    functions::{TauriFunctionError, app::TRAINING_DRAIN_TIMEOUT},
    settings::{LogRetentionSettings, Settings},
    state::{TauriFlowLikeState, TauriSettingsState},
};
use flow_like::app::App;
use flow_like_catalog::ml::inspection::lifecycle::{TrainingDrain, drain_app_training};
use flow_like_types::storage_paths::{
    TRAINING_LEDGER_ROOT, is_legacy_training_ledger_path, storage_path_segment,
    training_ledger_app_dir,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

/// The per-board Lance summary table earlier releases wrote next to the run
/// logs. Nothing reads it any more; it is listed so it can be deleted.
const LEGACY_RUN_INDEX: &str = "runs";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageItem {
    pub id: String,
    pub name: String,
    pub detail: String,
    pub size_bytes: u64,
    pub file_count: u64,
    pub updated_at_ms: Option<u64>,
    pub deletable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageCategory {
    pub key: String,
    pub label: String,
    pub description: String,
    pub size_bytes: u64,
    pub item_count: usize,
    pub items: Vec<StorageItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageOverview {
    pub total_bytes: u64,
    pub generated_at_ms: u64,
    pub log_retention: LogRetentionSettings,
    pub categories: Vec<StorageCategory>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageDeleteResult {
    pub deleted_items: usize,
    pub freed_bytes: u64,
    pub skipped_items: Vec<String>,
}

#[derive(Debug, Clone)]
struct StoragePaths {
    project_dir: PathBuf,
    bit_dir: PathBuf,
    logs_dir: PathBuf,
    user_dir: PathBuf,
    temporary_dir: PathBuf,
}

impl StoragePaths {
    fn from_settings(settings: &Settings) -> Self {
        Self {
            project_dir: settings.project_dir.clone(),
            bit_dir: settings.bit_dir.clone(),
            logs_dir: settings.logs_dir.clone(),
            user_dir: settings.user_dir.clone(),
            temporary_dir: settings.temporary_dir.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogRetentionInput {
    pub enabled: bool,
    pub days: u32,
}

#[derive(Default, Debug, Clone, Copy)]
struct PathStats {
    size_bytes: u64,
    file_count: u64,
    updated_at: Option<SystemTime>,
}

impl PathStats {
    fn merge(&mut self, other: Self) {
        self.size_bytes = self.size_bytes.saturating_add(other.size_bytes);
        self.file_count = self.file_count.saturating_add(other.file_count);
        self.updated_at = match (self.updated_at, other.updated_at) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (None, value) | (value, None) => value,
        };
    }
}

fn path_stats(path: &Path) -> PathStats {
    tree_stats(path, &mut HashSet::new())
}

/// Stats of the tree at `path`, without following symlinks. A file with several hard links
/// counts once across the walks that share `seen`.
fn tree_stats(path: &Path, seen: &mut HashSet<(u64, u64)>) -> PathStats {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return PathStats::default();
    };
    let counted = metadata.is_file() && linked_file(&metadata).is_none_or(|id| seen.insert(id));
    let mut stats = PathStats {
        size_bytes: if counted { metadata.len() } else { 0 },
        file_count: u64::from(counted),
        updated_at: metadata.modified().ok(),
    };
    if !metadata.is_dir() {
        return stats;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return stats;
    };
    for entry in entries.flatten() {
        stats.merge(tree_stats(&entry.path(), seen));
    }
    stats
}

/// The device and inode of a file with more than one hard link.
#[cfg(unix)]
fn linked_file(metadata: &fs::Metadata) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    (metadata.nlink() > 1).then(|| (metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
fn linked_file(_metadata: &fs::Metadata) -> Option<(u64, u64)> {
    None
}

fn to_ms(time: Option<SystemTime>) -> Option<u64> {
    time.and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
}

/// The app's device-local training ledgers, which live beside `apps/` in the project directory.
fn training_ledger_path(project_dir: &Path, app_id: &str) -> PathBuf {
    training_ledger_app_dir(app_id)
        .split('/')
        .fold(project_dir.to_path_buf(), |path, part| path.join(part))
}

fn direct_items(
    root: &Path,
    detail: &str,
    excluded_names: &HashSet<String>,
    deletable: bool,
) -> Vec<StorageItem> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut items = entries
        .flatten()
        .filter_map(|entry| {
            let id = entry.file_name().to_string_lossy().to_string();
            if id.is_empty() || excluded_names.contains(&id) {
                return None;
            }
            let stats = path_stats(&entry.path());
            Some(StorageItem {
                name: id.clone(),
                id,
                detail: detail.to_string(),
                size_bytes: stats.size_bytes,
                file_count: stats.file_count,
                updated_at_ms: to_ms(stats.updated_at),
                deletable,
            })
        })
        .collect::<Vec<_>>();
    items.sort_by_key(|item| std::cmp::Reverse(item.size_bytes));
    items
}

/// Each app's files with its training ledgers, then the ledgers no app folder claims, which an
/// app's deletion keeps while its training is still running. A file hard-linked into an app's
/// folder and its ledgers counts once.
fn app_items(project_dir: &Path) -> Vec<StorageItem> {
    let entries = |root: PathBuf| {
        fs::read_dir(root)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| {
                let id = entry.file_name().to_string_lossy().to_string();
                (id, entry.path())
            })
    };
    let mut claimed = HashSet::new();
    let mut items = Vec::new();
    for (id, path) in entries(project_dir.join("apps")) {
        let ledgers = training_ledger_path(project_dir, &id);
        let mut seen = HashSet::new();
        let mut stats = tree_stats(&path, &mut seen);
        stats.merge(tree_stats(&ledgers, &mut seen));
        claimed.insert(ledgers);
        let detail = format!("Local app · {} files", stats.file_count);
        items.push(deletable_item(id.clone(), id, detail, stats));
    }
    for (id, path) in entries(project_dir.join(TRAINING_LEDGER_ROOT).join("apps")) {
        if claimed.contains(&path) || storage_path_segment(&id, "app") != id || !path.is_dir() {
            continue;
        }
        let stats = path_stats(&path);
        let name = format!("Training ledgers {}", short_id(&id));
        let detail = format!("Left by a deleted app · {} files", stats.file_count);
        items.push(deletable_item(id, name, detail, stats));
    }
    items.sort_by_key(|item| std::cmp::Reverse(item.size_bytes));
    items
}

fn deletable_item(id: String, name: String, detail: String, stats: PathStats) -> StorageItem {
    StorageItem {
        id,
        name,
        detail,
        size_bytes: stats.size_bytes,
        file_count: stats.file_count,
        updated_at_ms: to_ms(stats.updated_at),
        deletable: true,
    }
}

fn log_items(logs_dir: &Path, active_runs: &HashSet<String>) -> Vec<StorageItem> {
    let runs_root = logs_dir.join("runs");
    let Ok(apps) = fs::read_dir(&runs_root) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    for app in apps
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
    {
        let app_id = app.file_name().to_string_lossy().to_string();
        let Ok(boards) = fs::read_dir(app.path()) else {
            continue;
        };
        for board in boards
            .flatten()
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        {
            let board_id = board.file_name().to_string_lossy().to_string();
            let Ok(runs) = fs::read_dir(board.path()) else {
                continue;
            };
            // A run is one `{run_id}.lance` table plus an optional
            // `{run_id}.payload` sidecar; the sidecar folds into the run's item.
            let mut payloads: HashMap<String, PathStats> = HashMap::new();
            let mut tables: Vec<(String, String, PathStats)> = Vec::new();
            for run in runs.flatten() {
                let file_name = run.file_name().to_string_lossy().to_string();
                if let Some(run_id) = file_name.strip_suffix(".payload") {
                    payloads.insert(run_id.to_string(), path_stats(&run.path()));
                    continue;
                }
                let Some(run_id) = file_name.strip_suffix(".lance") else {
                    continue;
                };
                if run_id.is_empty() {
                    continue;
                }
                let stats = path_stats(&run.path());
                tables.push((file_name.clone(), run_id.to_string(), stats));
            }
            for (file_name, run_id, mut stats) in tables {
                if let Some(payload) = payloads.remove(&run_id) {
                    stats.merge(payload);
                }
                let legacy = run_id == LEGACY_RUN_INDEX;
                let scope = format!("App {} · Board {}", short_id(&app_id), short_id(&board_id));
                items.push(StorageItem {
                    id: format!("{app_id}/{board_id}/{file_name}"),
                    name: run_id.clone(),
                    detail: if legacy {
                        format!("Legacy run index · {scope}")
                    } else {
                        scope
                    },
                    size_bytes: stats.size_bytes,
                    file_count: stats.file_count,
                    updated_at_ms: to_ms(stats.updated_at),
                    deletable: legacy || !active_runs.contains(&run_id),
                });
            }
        }
    }
    items.sort_by_key(|item| std::cmp::Reverse(item.updated_at_ms));
    items
}

fn offloaded_blob_items(blob_dir: &Path) -> Vec<StorageItem> {
    let ref_counts = fs::read(blob_dir.join("_refcounts.json"))
        .ok()
        .and_then(|data| {
            serde_json::from_slice::<std::collections::HashMap<String, u64>>(&data).ok()
        })
        .unwrap_or_default();
    let Ok(prefixes) = fs::read_dir(blob_dir) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    for prefix in prefixes
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
    {
        let Ok(blobs) = fs::read_dir(prefix.path()) else {
            continue;
        };
        for blob in blobs
            .flatten()
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        {
            let hash = blob.file_name().to_string_lossy().to_string();
            if !valid_component(&hash) {
                continue;
            }
            let stats = path_stats(&blob.path());
            let references = ref_counts.get(&hash).copied().unwrap_or_default();
            items.push(StorageItem {
                id: hash.clone(),
                name: format!("Browser payload {}", short_id(&hash)),
                detail: if references == 0 {
                    "Offloaded from browser storage · reference pending or untracked".to_string()
                } else {
                    format!(
                        "Offloaded from browser storage · {references} {}",
                        if references == 1 {
                            "reference"
                        } else {
                            "references"
                        }
                    )
                },
                size_bytes: stats.size_bytes,
                file_count: 1,
                updated_at_ms: to_ms(stats.updated_at),
                deletable: false,
            });
        }
    }
    items.sort_by_key(|item| std::cmp::Reverse(item.size_bytes));
    items
}

fn short_id(value: &str) -> &str {
    value.get(..8).unwrap_or(value)
}

fn category(key: &str, label: &str, description: &str, items: Vec<StorageItem>) -> StorageCategory {
    StorageCategory {
        key: key.to_string(),
        label: label.to_string(),
        description: description.to_string(),
        size_bytes: items.iter().map(|item| item.size_bytes).sum(),
        item_count: items.len(),
        items,
    }
}

fn build_overview(
    paths: &StoragePaths,
    log_retention: &LogRetentionSettings,
    active_runs: &HashSet<String>,
) -> StorageOverview {
    let apps = app_items(&paths.project_dir);

    let mut bits = direct_items(
        &paths.bit_dir,
        "Downloaded model or runtime artifact",
        &HashSet::from(["deps-cache".to_string()]),
        true,
    );
    for bit in &mut bits {
        bit.name = format!("Artifact {}", short_id(&bit.id));
    }

    let logs = log_items(&paths.logs_dir, active_runs);
    let offloaded = offloaded_blob_items(&paths.user_dir.join("blob_store"));
    let temporary = direct_items(
        &paths.temporary_dir,
        "Temporary execution file",
        &HashSet::new(),
        true,
    );

    let mut cache_exclusions = HashSet::new();
    cache_exclusions.insert("blob_store".to_string());
    for known in [
        &paths.bit_dir,
        &paths.project_dir,
        &paths.logs_dir,
        &paths.temporary_dir,
    ] {
        if known.parent() == Some(paths.user_dir.as_path())
            && let Some(name) = known.file_name()
        {
            cache_exclusions.insert(name.to_string_lossy().to_string());
        }
    }
    let cache = direct_items(
        &paths.user_dir,
        "Cache, local database, or supporting data",
        &cache_exclusions,
        false,
    );

    let categories = vec![
        category(
            "apps",
            "Apps & projects",
            "Boards, media, databases, and files owned by local apps.",
            apps,
        ),
        category(
            "bits",
            "Downloaded bits",
            "Local model weights and other reusable runtime artifacts.",
            bits,
        ),
        category(
            "logs",
            "Run logs",
            "Debug and execution history stored for individual local runs.",
            logs,
        ),
        category(
            "offloaded",
            "Offloaded browser files",
            "Large IndexedDB values moved onto disk by Studio to keep the WebView responsive.",
            offloaded,
        ),
        category(
            "cache",
            "Cache & support data",
            "Rebuildable caches plus local supporting databases.",
            cache,
        ),
        category(
            "temporary",
            "Temporary files",
            "Intermediate files created while workflows are running.",
            temporary,
        ),
    ];
    StorageOverview {
        total_bytes: categories.iter().map(|entry| entry.size_bytes).sum(),
        generated_at_ms: to_ms(Some(SystemTime::now())).unwrap_or_default(),
        log_retention: log_retention.clone(),
        categories,
    }
}

fn valid_component(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && !value.contains('/')
        && !value.contains('\\')
}

fn remove_path(path: &Path) -> std::io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// Deletes an app folder and returns the bytes it kept: while the app's training still runs,
/// the legacy App ledger inside the folder (`storage/inspection/jobs.sqlite` and its
/// companions) stays where the running job expects it.
fn remove_app_folder(path: &Path, keep_legacy_ledger: bool) -> std::io::Result<u64> {
    if !keep_legacy_ledger {
        return remove_path(path).map(|()| 0);
    }
    let kept = remove_tree_except(path, path, &is_legacy_training_ledger_path)?;
    if fs::read_dir(path)?.next().is_none() {
        fs::remove_dir(path)?;
    }
    Ok(kept)
}

/// Removes everything below `directory` whose path relative to `root` `keep` rejects, then
/// every directory left empty. Returns the bytes kept.
fn remove_tree_except(
    root: &Path,
    directory: &Path,
    keep: &dyn Fn(&str) -> bool,
) -> std::io::Result<u64> {
    let mut kept = 0;
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        let metadata = fs::symlink_metadata(&path)?;
        if keep(&relative) {
            kept += path_stats(&path).size_bytes;
        } else if metadata.is_dir() {
            let below = remove_tree_except(root, &path, keep)?;
            if below == 0 && fs::read_dir(&path)?.next().is_none() {
                fs::remove_dir(&path)?;
            }
            kept += below;
        } else {
            fs::remove_file(&path)?;
        }
    }
    Ok(kept)
}

/// Remove now-empty parent directories left behind after deleting a leaf entry,
/// walking upward until (but never touching) `stop_at`.
fn prune_empty_parents(deleted: &Path, stop_at: &Path) {
    let mut current = deleted.parent();
    while let Some(dir) = current {
        if dir == stop_at || !dir.starts_with(stop_at) {
            break;
        }
        let is_empty = fs::read_dir(dir)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false);
        if !is_empty || fs::remove_dir(dir).is_err() {
            break;
        }
        current = dir.parent();
    }
}

fn item_path(paths: &StoragePaths, category: &str, id: &str) -> Option<PathBuf> {
    match category {
        "apps" if valid_component(id) => Some(paths.project_dir.join("apps").join(id)),
        "bits" if valid_component(id) && id != "deps-cache" => Some(paths.bit_dir.join(id)),
        "cache" if valid_component(id) => Some(paths.user_dir.join(id)),
        "temporary" if valid_component(id) => Some(paths.temporary_dir.join(id)),
        "logs" => {
            let parts = id.split('/').collect::<Vec<_>>();
            if parts.len() == 3 && parts.iter().all(|part| valid_component(part)) {
                Some(
                    paths
                        .logs_dir
                        .join("runs")
                        .join(parts[0])
                        .join(parts[1])
                        .join(parts[2]),
                )
            } else {
                None
            }
        }
        _ => None,
    }
}

fn run_id_from_item_id(id: &str) -> Option<&str> {
    id.rsplit('/')
        .next()
        .map(|value| value.strip_suffix(".lance").unwrap_or(value))
}

/// `(app_id, run_id)` of a run log item — the row the run index must forget
/// once the table is gone. `None` for the legacy per-board index table.
fn indexed_run_from_item_id(id: &str) -> Option<(String, String)> {
    let app_id = id.split('/').next()?;
    let run_id = run_id_from_item_id(id)?;
    if run_id == LEGACY_RUN_INDEX {
        return None;
    }
    Some((app_id.to_string(), run_id.to_string()))
}

/// Removes a run's log table and its payload sidecar, then prunes emptied
/// board and app directories. Returns the index row to forget.
fn delete_log_item(
    paths: &StoragePaths,
    item: &StorageItem,
    result: &mut StorageDeleteResult,
) -> Option<(String, String)> {
    let Some(path) = item_path(paths, "logs", &item.id) else {
        result.skipped_items.push(item.id.clone());
        return None;
    };
    if remove_path(&path).is_err() {
        result.skipped_items.push(item.id.clone());
        return None;
    }
    if let Err(error) = fs::remove_file(path.with_extension("payload"))
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(%error, item = %item.id, "Failed to remove a run's payload sidecar");
    }
    result.deleted_items += 1;
    result.freed_bytes = result.freed_bytes.saturating_add(item.size_bytes);
    prune_empty_parents(&path, &paths.logs_dir.join("runs"));
    indexed_run_from_item_id(&item.id)
}

/// Drops the index rows of runs whose tables were just deleted. A stale row
/// only means a listing entry whose logs are gone, so failures are logged.
async fn forget_indexed_runs(app_handle: &AppHandle, runs: &[(String, String)]) {
    if runs.is_empty() {
        return;
    }
    let index = match TauriFlowLikeState::construct(app_handle).await {
        Ok(state) => crate::run_index::registered_run_index(&state).await,
        Err(error) => Err(error),
    };
    let index = match index {
        Ok(index) => index,
        Err(error) => {
            tracing::warn!(%error, "Run index unavailable; deleted runs keep their index rows");
            return;
        }
    };
    for (app_id, run_id) in runs {
        if let Err(error) = index.delete(app_id, run_id).await {
            tracing::warn!(%error, app_id, run_id, "Failed to remove a deleted run from the run index");
        }
    }
}

fn active_run_ids(app_handle: &AppHandle) -> HashSet<String> {
    app_handle
        .try_state::<TauriFlowLikeState>()
        .map(|state| {
            state
                .0
                .board_run_registry
                .iter()
                .map(|entry| entry.key().clone())
                .collect()
        })
        .unwrap_or_default()
}

fn cleanup_logs(
    paths: &StoragePaths,
    active_runs: &HashSet<String>,
    days: u32,
) -> (StorageDeleteResult, Vec<(String, String)>) {
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(u64::from(days.max(1)) * 86_400))
        .unwrap_or(UNIX_EPOCH);
    let mut result = StorageDeleteResult::default();
    let mut deleted_runs = Vec::new();
    for item in log_items(&paths.logs_dir, active_runs) {
        let is_expired = item
            .updated_at_ms
            .and_then(|millis| UNIX_EPOCH.checked_add(Duration::from_millis(millis)))
            .is_some_and(|modified| modified < cutoff);
        if !is_expired || !item.deletable {
            continue;
        }
        deleted_runs.extend(delete_log_item(paths, &item, &mut result));
    }
    (result, deleted_runs)
}

pub async fn run_configured_log_cleanup(
    app_handle: &AppHandle,
) -> Result<StorageDeleteResult, TauriFunctionError> {
    let settings_state = TauriSettingsState::construct(app_handle)
        .await
        .map_err(|error| TauriFunctionError::new(&error.to_string()))?;
    let (paths, days, enabled) = {
        let settings = settings_state.lock().await;
        (
            StoragePaths::from_settings(&settings),
            settings.log_retention.days,
            settings.log_retention.enabled,
        )
    };
    if !enabled {
        return Ok(StorageDeleteResult::default());
    }
    let active_runs = active_run_ids(app_handle);
    let (result, deleted_runs) = flow_like_types::tokio::task::spawn_blocking(move || {
        cleanup_logs(&paths, &active_runs, days)
    })
    .await
    .map_err(|error| TauriFunctionError::new(&error.to_string()))?;
    forget_indexed_runs(app_handle, &deleted_runs).await;

    let mut settings = settings_state.lock().await;
    settings.log_retention.last_cleanup_ms = to_ms(Some(SystemTime::now()));
    Settings::serialize(&mut settings);
    Ok(result)
}

#[tauri::command(async)]
pub async fn get_local_storage_overview(
    app_handle: AppHandle,
) -> Result<StorageOverview, TauriFunctionError> {
    let settings_state = TauriSettingsState::construct(&app_handle)
        .await
        .map_err(|error| TauriFunctionError::new(&error.to_string()))?;
    let (paths, log_retention) = {
        let settings = settings_state.lock().await;
        (
            StoragePaths::from_settings(&settings),
            settings.log_retention.clone(),
        )
    };
    let active_runs = active_run_ids(&app_handle);
    let mut overview = flow_like_types::tokio::task::spawn_blocking(move || {
        build_overview(&paths, &log_retention, &active_runs)
    })
    .await
    .map_err(|error| TauriFunctionError::new(&error.to_string()))?;

    if let Ok(state) = TauriFlowLikeState::construct(&app_handle).await
        && let Some(apps) = overview
            .categories
            .iter_mut()
            .find(|entry| entry.key == "apps")
    {
        for item in &mut apps.items {
            if let Ok(meta) = App::get_meta(item.id.clone(), state.clone(), None, None).await {
                item.name = meta.name;
                item.detail = format!("Local app · {} files", item.file_count);
            }
        }
    }
    Ok(overview)
}

#[tauri::command(async)]
pub async fn set_log_retention_policy(
    app_handle: AppHandle,
    policy: LogRetentionInput,
) -> Result<StorageDeleteResult, TauriFunctionError> {
    if !(1..=3650).contains(&policy.days) {
        return Err(TauriFunctionError::new(
            "Log retention must be between 1 and 3650 days",
        ));
    }
    let settings_state = TauriSettingsState::construct(&app_handle)
        .await
        .map_err(|error| TauriFunctionError::new(&error.to_string()))?;
    {
        let mut settings = settings_state.lock().await;
        settings.log_retention.enabled = policy.enabled;
        settings.log_retention.days = policy.days;
        Settings::serialize(&mut settings);
    }
    run_configured_log_cleanup(&app_handle).await
}

#[tauri::command(async)]
pub async fn run_log_cleanup(
    app_handle: AppHandle,
) -> Result<StorageDeleteResult, TauriFunctionError> {
    run_configured_log_cleanup(&app_handle).await
}

#[derive(Default)]
struct DeleteOutcome {
    result: StorageDeleteResult,
    deleted_apps: Vec<String>,
    deleted_runs: Vec<(String, String)>,
}

/// Stops the background training of the apps about to be deleted. Returns the apps whose
/// training was still running when the drain gave up, and the drains, which keep new training
/// of these apps refused until they drop.
async fn drain_training(
    project_dir: &Path,
    app_ids: &[String],
) -> (HashSet<String>, Vec<TrainingDrain>) {
    let drains = app_ids
        .iter()
        .filter(|id| valid_component(id))
        .map(|id| async move {
            let drain = drain_app_training(project_dir, id, TRAINING_DRAIN_TIMEOUT).await;
            (id, drain)
        });
    let mut still_running = HashSet::new();
    let mut held = Vec::new();
    for (id, drain) in futures::future::join_all(drains).await {
        if drain.report.still_running > 0 {
            still_running.insert(id.clone());
        }
        held.push(drain);
    }
    (still_running, held)
}

/// Removes a deleted app's training ledgers under `project_dir`, the whole directory tree,
/// unless its training is still running. Returns the bytes left on disk; a ledger that cannot
/// be removed is a warning, not a failure.
pub(crate) fn remove_training_ledgers(
    project_dir: &Path,
    app_id: &str,
    still_running: bool,
) -> u64 {
    let ledgers = training_ledger_path(project_dir, app_id);
    if still_running {
        tracing::warn!(
            app_id,
            ledgers = %ledgers.display(),
            "Training of the deleted app did not stop in time; keeping its training ledgers"
        );
        return path_stats(&ledgers).size_bytes;
    }
    match remove_path(&ledgers) {
        Ok(()) => 0,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
        Err(error) => {
            tracing::warn!(
                app_id,
                ledgers = %ledgers.display(),
                %error,
                "Failed to delete the training ledgers of the deleted app"
            );
            path_stats(&ledgers).size_bytes
        }
    }
}

fn delete_items_blocking(
    paths: &StoragePaths,
    log_retention: &LogRetentionSettings,
    category: &str,
    ids: Vec<String>,
    active_runs: &HashSet<String>,
    still_training: &HashSet<String>,
) -> Result<DeleteOutcome, TauriFunctionError> {
    let overview = build_overview(paths, log_retention, active_runs);
    let Some(inventory) = overview
        .categories
        .iter()
        .find(|entry| entry.key == category)
    else {
        return Err(TauriFunctionError::new("Unknown storage category"));
    };
    let known = inventory
        .items
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect::<HashMap<_, _>>();
    let mut outcome = DeleteOutcome::default();

    for id in ids {
        let Some(item) = known.get(id.as_str()) else {
            outcome.result.skipped_items.push(id);
            continue;
        };
        if !item.deletable
            || (category == "logs"
                && run_id_from_item_id(&id).is_some_and(|run| active_runs.contains(run)))
        {
            outcome.result.skipped_items.push(id);
            continue;
        }
        if category == "logs" {
            outcome
                .deleted_runs
                .extend(delete_log_item(paths, item, &mut outcome.result));
            continue;
        }
        if category == "apps" {
            delete_app_item(paths, item, still_training, &mut outcome);
            continue;
        }
        let Some(path) = item_path(paths, category, &id) else {
            outcome.result.skipped_items.push(id);
            continue;
        };
        match remove_path(&path) {
            Ok(()) => {
                outcome.result.deleted_items += 1;
                outcome.result.freed_bytes =
                    outcome.result.freed_bytes.saturating_add(item.size_bytes);
            }
            Err(_) => outcome.result.skipped_items.push(id),
        }
    }
    Ok(outcome)
}

/// Deletes an app's folder, then its training ledgers unless its training is still running.
/// An item without a folder is the ledgers a deleted app left behind, deleted once they are
/// gone.
fn delete_app_item(
    paths: &StoragePaths,
    item: &StorageItem,
    still_training: &HashSet<String>,
    outcome: &mut DeleteOutcome,
) {
    let Some(path) = item_path(paths, "apps", &item.id) else {
        outcome.result.skipped_items.push(item.id.clone());
        return;
    };
    let still_running = still_training.contains(&item.id);
    let (removed, legacy_kept) = match remove_app_folder(&path, still_running) {
        Ok(kept) => (true, kept),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (false, 0),
        Err(_) => {
            outcome.result.skipped_items.push(item.id.clone());
            return;
        }
    };
    let kept = legacy_kept.saturating_add(remove_training_ledgers(
        &paths.project_dir,
        &item.id,
        still_running,
    ));
    if !removed && (still_running || kept > 0) {
        outcome.result.skipped_items.push(item.id.clone());
        return;
    }
    outcome.result.deleted_items += 1;
    outcome.result.freed_bytes = outcome
        .result
        .freed_bytes
        .saturating_add(item.size_bytes.saturating_sub(kept));
    if removed {
        outcome.deleted_apps.push(item.id.clone());
    }
}

#[tauri::command(async)]
pub async fn delete_local_storage_items(
    app_handle: AppHandle,
    category: String,
    ids: Vec<String>,
) -> Result<StorageDeleteResult, TauriFunctionError> {
    if ids.is_empty() || ids.len() > 500 {
        return Err(TauriFunctionError::new("Select between 1 and 500 items"));
    }
    let active_runs = active_run_ids(&app_handle);
    let settings_state = TauriSettingsState::construct(&app_handle)
        .await
        .map_err(|error| TauriFunctionError::new(&error.to_string()))?;
    let (paths, log_retention) = {
        let settings = settings_state.lock().await;
        (
            StoragePaths::from_settings(&settings),
            settings.log_retention.clone(),
        )
    };
    let (still_training, drains) = if category == "apps" {
        drain_training(&paths.project_dir, &ids).await
    } else {
        Default::default()
    };
    let category_for_task = category.clone();
    let outcome = flow_like_types::tokio::task::spawn_blocking(move || {
        let _drains = drains;
        delete_items_blocking(
            &paths,
            &log_retention,
            &category_for_task,
            ids,
            &active_runs,
            &still_training,
        )
    })
    .await
    .map_err(|error| TauriFunctionError::new(&error.to_string()))??;
    forget_indexed_runs(&app_handle, &outcome.deleted_runs).await;

    if !outcome.deleted_apps.is_empty() {
        let mut settings = settings_state.lock().await;
        for profile in settings.profiles.values_mut() {
            if let Some(apps) = &mut profile.hub_profile.apps {
                apps.retain(|app| !outcome.deleted_apps.contains(&app.app_id));
            }
        }
        Settings::serialize(&mut settings);
    }
    Ok(outcome.result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_traversal_components() {
        assert!(valid_component("safe-id"));
        assert!(!valid_component("../unsafe"));
        assert!(!valid_component("nested/path"));
        assert!(!valid_component("nested\\path"));
    }

    #[test]
    fn recursive_stats_do_not_follow_symlinks() {
        let root = std::env::temp_dir().join(format!(
            "flow-like-storage-stats-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("nested/file.bin"), [1_u8, 2, 3, 4]).unwrap();
        let stats = path_stats(&root);
        assert_eq!(stats.size_bytes, 4);
        assert_eq!(stats.file_count, 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn prune_empty_parents_stops_at_root_and_keeps_populated_dirs() {
        let root = std::env::temp_dir().join(format!(
            "flow-like-storage-prune-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let runs = root.join("runs");
        let leaf = runs.join("app").join("board").join("run.lance");
        fs::create_dir_all(&leaf).unwrap();
        // A sibling run keeps `board` (and its ancestors) populated.
        let sibling = runs.join("app").join("board").join("other.lance");
        fs::create_dir_all(&sibling).unwrap();

        fs::remove_dir_all(&leaf).unwrap();
        prune_empty_parents(&leaf, &runs);
        assert!(sibling.exists());
        assert!(runs.join("app").join("board").exists());

        fs::remove_dir_all(&sibling).unwrap();
        prune_empty_parents(&sibling, &runs);
        // Empty `board` and `app` are pruned, but the `runs` root survives.
        assert!(!runs.join("app").exists());
        assert!(runs.exists());

        fs::remove_dir_all(&root).unwrap();
    }

    fn scratch_logs_dir(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "flow-like-storage-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let board = root.join("runs").join("app").join("board");
        fs::create_dir_all(board.join("run1.lance")).unwrap();
        fs::write(board.join("run1.lance").join("data.bin"), [0_u8; 10]).unwrap();
        fs::write(board.join("run1.payload"), b"{\"a\":1}").unwrap();
        fs::create_dir_all(board.join("runs.lance")).unwrap();
        fs::write(board.join("runs.lance").join("data.bin"), [0_u8; 3]).unwrap();
        fs::write(board.join("orphan.payload"), b"{}").unwrap();
        root
    }

    #[test]
    fn log_items_fold_sidecars_and_expose_the_legacy_index() {
        let logs_dir = scratch_logs_dir("list");
        let items = log_items(&logs_dir, &HashSet::new());
        assert_eq!(items.len(), 2);

        let run = items.iter().find(|item| item.name == "run1").unwrap();
        assert_eq!(run.id, "app/board/run1.lance");
        assert_eq!(run.size_bytes, 10 + 7);
        assert_eq!(run.file_count, 2);
        assert!(run.deletable);

        let legacy = items.iter().find(|item| item.name == "runs").unwrap();
        assert_eq!(legacy.id, "app/board/runs.lance");
        assert!(legacy.detail.starts_with("Legacy run index"));
        assert!(legacy.deletable);

        assert!(!items.iter().any(|item| item.name == "orphan"));
        assert_eq!(
            indexed_run_from_item_id(&run.id),
            Some(("app".to_string(), "run1".to_string()))
        );
        assert_eq!(indexed_run_from_item_id(&legacy.id), None);

        fs::remove_dir_all(logs_dir).unwrap();
    }

    #[test]
    fn deleting_a_run_removes_its_table_and_sidecar() {
        let logs_dir = scratch_logs_dir("delete");
        let paths = StoragePaths {
            project_dir: logs_dir.join("apps"),
            bit_dir: logs_dir.join("bits"),
            logs_dir: logs_dir.clone(),
            user_dir: logs_dir.join("user"),
            temporary_dir: logs_dir.join("tmp"),
        };
        let items = log_items(&logs_dir, &HashSet::new());
        let run = items.iter().find(|item| item.name == "run1").unwrap();
        let mut result = StorageDeleteResult::default();

        let forgotten = delete_log_item(&paths, run, &mut result);
        assert_eq!(forgotten, Some(("app".to_string(), "run1".to_string())));
        assert_eq!(result.deleted_items, 1);
        assert_eq!(result.freed_bytes, 17);
        let board = logs_dir.join("runs").join("app").join("board");
        assert!(!board.join("run1.lance").exists());
        assert!(!board.join("run1.payload").exists());
        assert!(board.join("runs.lance").exists());

        let legacy = items.iter().find(|item| item.name == "runs").unwrap();
        assert_eq!(delete_log_item(&paths, legacy, &mut result), None);
        assert!(!board.join("runs.lance").exists());
        assert_eq!(result.deleted_items, 2);
        assert!(result.skipped_items.is_empty());

        fs::remove_dir_all(logs_dir).unwrap();
    }

    fn scratch_paths(label: &str) -> StoragePaths {
        let root = std::env::temp_dir().join(format!(
            "flow-like-storage-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        StoragePaths {
            project_dir: root.join("projects"),
            bit_dir: root.join("bits"),
            logs_dir: root.join("logs"),
            temporary_dir: root.join("tmp"),
            user_dir: root,
        }
    }

    fn write_bytes(path: &Path, size: usize) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, vec![0_u8; size]).unwrap();
    }

    /// Writes an app file of `app_bytes` and an App ledger of `ledger_bytes`; returns the
    /// app's ledger directory.
    fn app_with_ledger(
        paths: &StoragePaths,
        app_id: &str,
        app_bytes: usize,
        ledger_bytes: usize,
    ) -> PathBuf {
        write_bytes(
            &paths
                .project_dir
                .join("apps")
                .join(app_id)
                .join("manifest.app"),
            app_bytes,
        );
        let ledgers = training_ledger_path(&paths.project_dir, app_id);
        write_bytes(&ledgers.join("project").join("jobs.sqlite"), ledger_bytes);
        ledgers
    }

    #[test]
    fn app_items_count_their_training_ledgers_and_deleting_an_app_removes_them() {
        let paths = scratch_paths("ledgers");
        let ledgers = app_with_ledger(&paths, "doomed", 10, 5);
        write_bytes(&ledgers.join("users").join("user").join("jobs.sqlite"), 3);
        let kept = app_with_ledger(&paths, "kept", 20, 2);
        let retention = LogRetentionSettings::default();

        let overview = build_overview(&paths, &retention, &HashSet::new());
        let apps = overview
            .categories
            .iter()
            .find(|category| category.key == "apps")
            .unwrap();
        let doomed = apps.items.iter().find(|item| item.id == "doomed").unwrap();
        assert_eq!((doomed.size_bytes, doomed.file_count), (18, 3));
        assert_eq!(doomed.detail, "Local app · 3 files");
        let order = apps
            .items
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(order, ["kept", "doomed"]);
        assert_eq!(apps.size_bytes, 40);

        let outcome = delete_items_blocking(
            &paths,
            &retention,
            "apps",
            vec!["doomed".to_string()],
            &HashSet::new(),
            &HashSet::new(),
        )
        .unwrap();
        assert_eq!(outcome.deleted_apps, ["doomed"]);
        assert_eq!(outcome.result.freed_bytes, 18);
        assert!(!paths.project_dir.join("apps").join("doomed").exists());
        assert!(!ledgers.exists());
        assert!(kept.join("project").join("jobs.sqlite").is_file());
        assert!(
            paths
                .project_dir
                .join("apps")
                .join("kept")
                .join("manifest.app")
                .is_file()
        );

        fs::remove_dir_all(&paths.user_dir).unwrap();
    }

    #[test]
    fn deleting_an_app_whose_training_kept_running_keeps_its_ledgers() {
        let paths = scratch_paths("still-training");
        let ledgers = app_with_ledger(&paths, "busy", 10, 5);
        let retention = LogRetentionSettings::default();

        let outcome = delete_items_blocking(
            &paths,
            &retention,
            "apps",
            vec!["busy".to_string()],
            &HashSet::new(),
            &HashSet::from(["busy".to_string()]),
        )
        .unwrap();
        assert_eq!(outcome.deleted_apps, ["busy"]);
        assert_eq!(outcome.result.deleted_items, 1);
        assert_eq!(outcome.result.freed_bytes, 10);
        assert!(!paths.project_dir.join("apps").join("busy").exists());
        assert!(ledgers.join("project").join("jobs.sqlite").is_file());

        fs::remove_dir_all(&paths.user_dir).unwrap();
    }

    #[test]
    fn deleting_an_app_whose_training_kept_running_keeps_its_legacy_ledger_in_place() {
        let paths = scratch_paths("still-training-legacy");
        app_with_ledger(&paths, "busy", 10, 5);
        let inspection = paths.project_dir.join("apps/busy/storage/inspection");
        write_bytes(&inspection.join("jobs.sqlite"), 7);
        write_bytes(&inspection.join("jobs.artifacts/source.blob"), 3);
        write_bytes(&inspection.join("pki/device.pem"), 4);

        let outcome = delete_items_blocking(
            &paths,
            &LogRetentionSettings::default(),
            "apps",
            vec!["busy".to_string()],
            &HashSet::new(),
            &HashSet::from(["busy".to_string()]),
        )
        .unwrap();
        assert_eq!(outcome.deleted_apps, ["busy"]);
        assert_eq!(outcome.result.freed_bytes, 14);
        assert!(inspection.join("jobs.sqlite").is_file());
        assert!(inspection.join("jobs.artifacts/source.blob").is_file());
        assert!(!inspection.join("pki").exists());
        assert!(!paths.project_dir.join("apps/busy/manifest.app").exists());

        fs::remove_dir_all(&paths.user_dir).unwrap();
    }

    fn apps_category(paths: &StoragePaths) -> StorageCategory {
        build_overview(paths, &LogRetentionSettings::default(), &HashSet::new())
            .categories
            .into_iter()
            .find(|category| category.key == "apps")
            .unwrap()
    }

    #[cfg(unix)]
    #[test]
    fn hard_linked_ledger_blobs_count_once_for_their_app() {
        let paths = scratch_paths("hard-links");
        let ledgers = app_with_ledger(&paths, "linked", 10, 5);
        let legacy = paths
            .project_dir
            .join("apps/linked/storage/inspection/jobs.artifacts/source.blob");
        write_bytes(&legacy, 100);
        let pretrained = ledgers.join("project/jobs.artifacts/pretrained.blob");
        write_bytes(&pretrained, 40);
        let adopted = ledgers.join("users/user/jobs.artifacts");
        fs::create_dir_all(&adopted).unwrap();
        fs::hard_link(&legacy, adopted.join("source.blob")).unwrap();
        fs::hard_link(&pretrained, adopted.join("pretrained.blob")).unwrap();

        let apps = apps_category(&paths);
        let linked = &apps.items[0];
        assert_eq!(linked.id, "linked");
        assert_eq!((linked.size_bytes, linked.file_count), (155, 4));
        assert_eq!(path_stats(&ledgers).size_bytes, 145);

        fs::remove_dir_all(&paths.user_dir).unwrap();
    }

    #[test]
    fn ledgers_left_by_a_deleted_app_are_listed_and_can_be_deleted() {
        let paths = scratch_paths("unclaimed-ledgers");
        let kept = app_with_ledger(&paths, "kept", 10, 5);
        let left = training_ledger_path(&paths.project_dir, "deleted");
        write_bytes(&left.join("project").join("jobs.sqlite"), 7);
        write_bytes(&left.join("users").join("user").join("jobs.sqlite"), 3);
        fs::create_dir_all(left.join("project").join("jobs.artifacts")).unwrap();
        let foreign = paths
            .project_dir
            .join(TRAINING_LEDGER_ROOT)
            .join("apps")
            .join("not a segment");
        write_bytes(&foreign.join("jobs.sqlite"), 1);
        let retention = LogRetentionSettings::default();

        let apps = apps_category(&paths);
        let ids = apps
            .items
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, ["kept", "deleted"]);
        let deleted = &apps.items[1];
        assert_eq!((deleted.size_bytes, deleted.file_count), (10, 2));
        assert_eq!(deleted.detail, "Left by a deleted app · 2 files");
        assert!(deleted.deletable);

        let delete = |still_training: HashSet<String>| {
            delete_items_blocking(
                &paths,
                &retention,
                "apps",
                vec!["deleted".to_string()],
                &HashSet::new(),
                &still_training,
            )
            .unwrap()
        };
        let running = delete(HashSet::from(["deleted".to_string()]));
        assert_eq!(running.result.skipped_items, ["deleted"]);
        assert!(left.join("project").join("jobs.sqlite").is_file());
        let outcome = delete(HashSet::new());
        assert!(outcome.deleted_apps.is_empty());
        assert_eq!(
            (outcome.result.deleted_items, outcome.result.freed_bytes),
            (1, 10)
        );
        assert!(!left.exists());
        assert!(kept.join("project").join("jobs.sqlite").is_file());
        assert!(foreign.is_dir());

        fs::remove_dir_all(&paths.user_dir).unwrap();
    }
}
