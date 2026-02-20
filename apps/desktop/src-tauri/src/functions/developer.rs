use crate::{
    functions::TauriFunctionError,
    state::{TauriFlowLikeState, TauriSettingsState, TauriWasmEngineState},
};
use dashmap::DashMap;
use flow_like::flow::node::{Node, NodeLogic, NodeWasm};
use flow_like_wasm::abi::{WasmExecutionInput, WasmExecutionResult, WasmNodeDefinition};
use flow_like_wasm::manifest::PackageManifest;
use flow_like_wasm::{build_node_from_definition, WasmEngine, WasmNodeLogic, WasmSecurityConfig};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::time::SystemTime;
use tauri::{AppHandle, Emitter};

static INSPECTION_CACHE: LazyLock<DashMap<String, (SystemTime, PackageInspection)>> =
    LazyLock::new(DashMap::new);

fn wasm_file_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

pub fn clear_inspection_cache() {
    INSPECTION_CACHE.clear();
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeveloperProject {
    pub id: String,
    pub path: String,
    pub language: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeveloperProjectStore {
    pub projects: Vec<DeveloperProject>,
    #[serde(default)]
    pub preferred_editor: String,
}

impl Default for DeveloperProjectStore {
    fn default() -> Self {
        Self {
            projects: Vec::new(),
            preferred_editor: String::from("vscode"),
        }
    }
}

fn store_path(user_dir: &Path) -> PathBuf {
    user_dir.join("developer-projects.json")
}

fn detect_project_language(project_path: &Path) -> Option<String> {
    let entries = std::fs::read_dir(project_path).ok()?;
    let names: Vec<String> = entries
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    let lower: std::collections::HashSet<String> = names.iter().map(|n| n.to_lowercase()).collect();

    let src_lower = collect_dir_names(&project_path.join("src"));

    if lower.iter().any(|f| f.ends_with(".gr")) || src_lower.iter().any(|f| f.ends_with(".gr")) {
        return Some("grain".into());
    }
    if lower.iter().any(|f| f.ends_with(".nimble")) || lower.contains("nim.cfg") || lower.contains("config.nims") {
        return Some("nim".into());
    }
    if lower.iter().any(|f| f.ends_with(".lua") || f.ends_with(".rockspec"))
        || lower.contains(".luacheckrc")
        || src_lower.iter().any(|f| f.ends_with(".lua"))
    {
        return Some("lua".into());
    }
    if lower.contains("package.swift") {
        return Some("swift".into());
    }
    if lower.contains("build.zig") || lower.contains("build.zig.zon") {
        return Some("zig".into());
    }
    if lower.contains("go.mod") {
        return Some("go".into());
    }
    if lower.contains("build.gradle.kts") || lower.contains("settings.gradle.kts") {
        return Some("kotlin".into());
    }
    if lower.iter().any(|f| f.ends_with(".csproj")) {
        return Some("csharp".into());
    }
    if lower.contains("pom.xml") {
        return Some("java".into());
    }
    if lower.contains("cmakelists.txt") || lower.iter().any(|f| f.ends_with(".cpp") || f.ends_with(".cc")) {
        return Some("cpp".into());
    }
    if lower.contains("pyproject.toml") || lower.contains("requirements.txt") {
        return Some("python".into());
    }
    if lower.contains("asconfig.json") {
        return Some("assemblyscript".into());
    }
    if lower.contains("package.json") {
        return Some("typescript".into());
    }
    if lower.contains("cargo.toml") {
        return Some("rust".into());
    }
    None
}

fn collect_dir_names(dir: &Path) -> std::collections::HashSet<String> {
    std::fs::read_dir(dir)
        .ok()
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .map(|n| n.to_lowercase())
                .collect()
        })
        .unwrap_or_default()
}

fn load_store(user_dir: &Path) -> DeveloperProjectStore {
    let path = store_path(user_dir);
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        DeveloperProjectStore::default()
    }
}

fn save_store(user_dir: &Path, store: &DeveloperProjectStore) -> Result<(), TauriFunctionError> {
    let path = store_path(user_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    }
    let json =
        serde_json::to_string_pretty(store).map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    std::fs::write(&path, json).map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub async fn developer_list_projects(
    app_handle: AppHandle,
) -> Result<Vec<DeveloperProject>, TauriFunctionError> {
    let settings = TauriSettingsState::construct(&app_handle)
        .await
        .map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    let settings_guard = settings.lock().await;
    let mut store = load_store(&settings_guard.user_dir);

    let mut changed = false;
    for project in &mut store.projects {
        let path = Path::new(&project.path);
        if let Some(detected) = detect_project_language(path) {
            if project.language != detected {
                project.language = detected;
                changed = true;
            }
        }
    }
    if changed {
        let _ = save_store(&settings_guard.user_dir, &store);
    }

    Ok(store.projects)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddProjectInput {
    pub path: String,
    pub language: String,
    pub name: String,
}

#[tauri::command]
pub async fn developer_add_project(
    app_handle: AppHandle,
    input: AddProjectInput,
) -> Result<DeveloperProject, TauriFunctionError> {
    let settings = TauriSettingsState::construct(&app_handle)
        .await
        .map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    let settings_guard = settings.lock().await;
    let mut store = load_store(&settings_guard.user_dir);

    if store.projects.iter().any(|p| p.path == input.path) {
        return Err(TauriFunctionError::new("Project already registered"));
    }

    let project = DeveloperProject {
        id: format!(
            "{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        ),
        path: input.path.clone(),
        language: detect_project_language(Path::new(&input.path))
            .unwrap_or(input.language),
        name: input.name,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    store.projects.push(project.clone());
    save_store(&settings_guard.user_dir, &store)?;
    Ok(project)
}

#[tauri::command]
pub async fn developer_remove_project(
    app_handle: AppHandle,
    project_id: String,
) -> Result<(), TauriFunctionError> {
    let settings = TauriSettingsState::construct(&app_handle)
        .await
        .map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    let settings_guard = settings.lock().await;
    let mut store = load_store(&settings_guard.user_dir);
    store.projects.retain(|p| p.id != project_id);
    save_store(&settings_guard.user_dir, &store)?;
    Ok(())
}

#[tauri::command]
pub async fn developer_list_local_files(
    _app_handle: AppHandle,
    path: String,
) -> Result<Vec<String>, TauriFunctionError> {
    let dir = PathBuf::from(&path);
    if !dir.is_dir() {
        return Err(TauriFunctionError::new("Path is not a directory"));
    }
    let entries = std::fs::read_dir(&dir)
        .map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    let names: Vec<String> = entries
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    Ok(names)
}

#[tauri::command]
pub async fn developer_get_manifest(
    _app_handle: AppHandle,
    project_path: String,
) -> Result<serde_json::Value, TauriFunctionError> {
    let manifest_path = Path::new(&project_path).join("flow-like.toml");
    if !manifest_path.exists() {
        return Err(TauriFunctionError::new(
            "No flow-like.toml found in project",
        ));
    }
    let content = std::fs::read_to_string(&manifest_path)
        .map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    let value: toml::Value =
        toml::from_str(&content).map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    let json = serde_json::to_value(value).map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    Ok(json)
}

#[tauri::command]
pub async fn developer_save_manifest(
    _app_handle: AppHandle,
    project_path: String,
    manifest: serde_json::Value,
) -> Result<(), TauriFunctionError> {
    let manifest_path = Path::new(&project_path).join("flow-like.toml");
    let toml_value: toml::Value = serde_json::from_value(manifest)
        .map_err(|e| TauriFunctionError::new(&format!("Invalid manifest: {}", e)))?;
    let toml_str =
        toml::to_string_pretty(&toml_value).map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    std::fs::write(&manifest_path, toml_str)
        .map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub async fn developer_open_in_editor(
    app_handle: AppHandle,
    project_path: String,
) -> Result<(), TauriFunctionError> {
    let settings = TauriSettingsState::construct(&app_handle)
        .await
        .map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    let settings_guard = settings.lock().await;
    let store = load_store(&settings_guard.user_dir);
    let editor = &store.preferred_editor;
    drop(settings_guard);

    let cmd = match editor.as_str() {
        "vscode" => "code",
        "cursor" => "cursor",
        "zed" => "zed",
        "idea" | "jetbrains" => "idea",
        "fleet" => "fleet",
        "sublime" => "subl",
        "vim" | "nvim" => "nvim",
        other => {
            return Err(TauriFunctionError::new(&format!(
                "Unknown editor '{}'. Please select a supported editor in settings.",
                other
            )));
        }
    };

    std::process::Command::new(cmd)
        .arg(&project_path)
        .spawn()
        .map_err(|e| {
            TauriFunctionError::new(&format!(
                "Failed to open editor '{}': {}. Make sure it is installed and available in PATH.",
                cmd, e
            ))
        })?;

    Ok(())
}

#[tauri::command]
pub async fn developer_get_settings(
    app_handle: AppHandle,
) -> Result<DeveloperSettings, TauriFunctionError> {
    let settings = TauriSettingsState::construct(&app_handle)
        .await
        .map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    let settings_guard = settings.lock().await;
    let store = load_store(&settings_guard.user_dir);
    Ok(DeveloperSettings {
        preferred_editor: store.preferred_editor,
        dev_mode: settings_guard.dev_mode,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeveloperSettings {
    pub preferred_editor: String,
    pub dev_mode: bool,
}

#[tauri::command]
pub async fn developer_save_settings(
    app_handle: AppHandle,
    dev_settings: DeveloperSettings,
) -> Result<(), TauriFunctionError> {
    let settings = TauriSettingsState::construct(&app_handle)
        .await
        .map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    let mut settings_guard = settings.lock().await;
    settings_guard.dev_mode = dev_settings.dev_mode;

    let mut store = load_store(&settings_guard.user_dir);
    store.preferred_editor = dev_settings.preferred_editor;
    save_store(&settings_guard.user_dir, &store)?;

    crate::settings::Settings::serialize(&mut settings_guard);
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScaffoldInput {
    pub target_dir: String,
    pub language: String,
    pub project_name: String,
}

#[tauri::command]
pub async fn developer_scaffold_project(
    app_handle: AppHandle,
    input: ScaffoldInput,
) -> Result<DeveloperProject, TauriFunctionError> {
    let template_dir = match input.language.as_str() {
        "rust" => "wasm-node-rust",
        "python" => "wasm-node-python",
        "assemblyscript" | "as" => "wasm-node-assemblyscript",
        "go" => "wasm-node-go",
        "cpp" | "c" | "c++" => "wasm-node-cpp",
        "csharp" | "c#" => "wasm-node-csharp",
        "kotlin" | "kt" => "wasm-node-kotlin",
        "zig" => "wasm-node-zig",
        "nim" => "wasm-node-nim",
        "lua" => "wasm-node-lua",
        "swift" => "wasm-node-swift",
        "java" => "wasm-node-java",
        "grain" => "wasm-node-grain",
        "moonbit" => "wasm-node-moonbit",
        other => {
            return Err(TauriFunctionError::new(&format!(
                "Unsupported language: {}",
                other
            )));
        }
    };

    let api_url = format!(
        "https://api.github.com/repos/TM9657/flow-like/contents/templates/{}?ref=dev",
        template_dir
    );

    let target = PathBuf::from(&input.target_dir);
    if target.exists()
        && std::fs::read_dir(&target)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false)
    {
        return Err(TauriFunctionError::new("Target directory is not empty"));
    }
    std::fs::create_dir_all(&target).map_err(|e| TauriFunctionError::new(&e.to_string()))?;

    download_github_dir(&api_url, &target).await?;

    patch_manifest(&target, &input.project_name)?;

    let add_input = AddProjectInput {
        path: target.to_string_lossy().to_string(),
        language: input.language,
        name: input.project_name,
    };
    developer_add_project(app_handle, add_input).await
}

async fn download_github_dir(api_url: &str, target: &Path) -> Result<(), TauriFunctionError> {
    let client = flow_like_types::reqwest::Client::builder()
        .user_agent("flow-like-desktop")
        .build()
        .map_err(|e| TauriFunctionError::new(&e.to_string()))?;

    let resp = client
        .get(api_url)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| TauriFunctionError::new(&format!("GitHub API request failed: {}", e)))?;

    if !resp.status().is_success() {
        return Err(TauriFunctionError::new(&format!(
            "GitHub API returned {}",
            resp.status()
        )));
    }

    let entries: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| TauriFunctionError::new(&format!("Failed to parse GitHub response: {}", e)))?;

    for entry in &entries {
        let entry_type = entry["type"].as_str().unwrap_or("");
        let name = entry["name"].as_str().unwrap_or("");

        if name.starts_with('.') || name == "build" || name == "__pycache__" {
            continue;
        }

        let safe_name = match std::path::Path::new(name).file_name() {
            Some(n) => n.to_owned(),
            None => continue,
        };

        match entry_type {
            "file" => {
                let download_url = entry["download_url"]
                    .as_str()
                    .ok_or_else(|| TauriFunctionError::new("Missing download_url"))?;

                let content = client
                    .get(download_url)
                    .send()
                    .await
                    .map_err(|e| TauriFunctionError::new(&e.to_string()))?
                    .bytes()
                    .await
                    .map_err(|e| TauriFunctionError::new(&e.to_string()))?;

                let file_path = target.join(&safe_name);
                if let Some(parent) = file_path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| TauriFunctionError::new(&e.to_string()))?;
                }
                std::fs::write(&file_path, &content)
                    .map_err(|e| TauriFunctionError::new(&e.to_string()))?;
            }
            "dir" => {
                let sub_url = entry["url"]
                    .as_str()
                    .ok_or_else(|| TauriFunctionError::new("Missing dir url"))?;
                let sub_dir = target.join(&safe_name);
                std::fs::create_dir_all(&sub_dir)
                    .map_err(|e| TauriFunctionError::new(&e.to_string()))?;
                Box::pin(download_github_dir(sub_url, &sub_dir)).await?;
            }
            _ => {}
        }
    }

    Ok(())
}

fn patch_manifest(target: &Path, project_name: &str) -> Result<(), TauriFunctionError> {
    let manifest_path = target.join("flow-like.toml");
    if !manifest_path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(&manifest_path)
        .map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    let mut doc = content
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| TauriFunctionError::new(&e.to_string()))?;

    if let Some(pkg) = doc.get_mut("package").and_then(|v| v.as_table_mut()) {
        let slug = project_name
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect::<String>();
        pkg["name"] = toml_edit::value(project_name);
        pkg["id"] = toml_edit::value(format!("com.custom.{}", slug));
    }

    std::fs::write(&manifest_path, doc.to_string())
        .map_err(|e| TauriFunctionError::new(&e.to_string()))?;

    Ok(())
}

#[tauri::command]
pub async fn developer_inspect_node(
    app_handle: AppHandle,
    wasm_path: String,
) -> Result<Vec<WasmNodeDefinition>, TauriFunctionError> {
    let engine = TauriWasmEngineState::construct(&app_handle)
        .map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    tokio::spawn(async move {
        let path = PathBuf::from(&wasm_path);
        if !path.exists() {
            return Err(TauriFunctionError::new("WASM file not found"));
        }

        let loaded = engine
            .load_auto_from_file(&path)
            .await
            .map_err(|e| TauriFunctionError::new(&format!("Failed to load WASM module: {}", e)))?;

        let security = WasmSecurityConfig::permissive();
        let mut instance = loaded
            .instantiate(&engine, security)
            .await
            .map_err(|e| {
                TauriFunctionError::new(&format!("Failed to instantiate module: {}", e))
            })?;

        instance.call_get_nodes().await.map_err(|e| {
            TauriFunctionError::new(&format!("Failed to get node definitions: {}", e))
        })
    })
    .await
    .map_err(|e| TauriFunctionError::new(&format!("Task panicked: {}", e)))?
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageInspection {
    pub nodes: Vec<WasmNodeDefinition>,
    pub manifest: Option<PackageManifest>,
    pub is_package: bool,
    pub wasm_path: String,
}

#[tauri::command]
pub async fn developer_inspect_package(
    app_handle: AppHandle,
    project_path: String,
) -> Result<PackageInspection, TauriFunctionError> {
    let engine = TauriWasmEngineState::construct(&app_handle)
        .map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    tokio::spawn(async move {
        let project = PathBuf::from(&project_path);
        let wasm_path = find_wasm_file(&project)?;

        if let Some(mtime) = wasm_file_mtime(&wasm_path) {
            let cache_key = wasm_path.to_string_lossy().to_string();
            if let Some(entry) = INSPECTION_CACHE.get(&cache_key) {
                let (cached_mtime, cached_result) = entry.value();
                if *cached_mtime == mtime {
                    return Ok(cached_result.clone());
                }
            }
        }

        let manifest = load_manifest_typed(&project).ok();
        let loaded = engine
            .load_auto_from_file(&wasm_path)
            .await
            .map_err(|e| {
                TauriFunctionError::new(&format!("Failed to load WASM module: {}", e))
            })?;

        let security = WasmSecurityConfig::permissive();
        let mut instance = loaded
            .instantiate(&engine, security)
            .await
            .map_err(|e| {
                TauriFunctionError::new(&format!("Failed to instantiate module: {}", e))
            })?;

        let is_package = instance.is_package();
        let nodes = instance.call_get_nodes().await.map_err(|e| {
            TauriFunctionError::new(&format!("Failed to get node definitions: {}", e))
        })?;

        let result = PackageInspection {
            nodes,
            manifest,
            is_package,
            wasm_path: wasm_path.to_string_lossy().to_string(),
        };

        if let Some(mtime) = wasm_file_mtime(&wasm_path) {
            let cache_key = wasm_path.to_string_lossy().to_string();
            INSPECTION_CACHE.insert(cache_key, (mtime, result.clone()));
        }

        Ok(result)
    })
    .await
    .map_err(|e| TauriFunctionError::new(&format!("Task panicked: {}", e)))?
}

fn load_manifest_typed(project_path: &Path) -> Result<PackageManifest, TauriFunctionError> {
    let manifest_path = project_path.join("flow-like.toml");
    let content = std::fs::read_to_string(&manifest_path)
        .map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    PackageManifest::from_toml(&content)
        .map_err(|e| TauriFunctionError::new(&format!("Invalid manifest: {}", e)))
}

fn find_wasm_file(project_path: &Path) -> Result<PathBuf, TauriFunctionError> {
    // 1. Check manifest wasm_path first
    if let Ok(manifest) = load_manifest_typed(project_path) {
        if let Some(wasm_path) = &manifest.wasm_path {
            let p = project_path.join(wasm_path);
            if p.exists() {
                return Ok(p);
            }
        }
    }

    // 2. Check well-known output paths
    let candidates = [
        "build/node.wasm",
        "build/release.wasm",
        "build/debug.wasm",
        "node.wasm",
        ".build/release/Node.wasm",
        "target/wasm/classes.wasm",
    ];
    for candidate in &candidates {
        let p = project_path.join(candidate);
        if p.exists() {
            return Ok(p);
        }
    }

    // 3. Recursively search for .wasm files, skipping build tooling dirs
    let skip_dirs: &[&str] = &[
        "node_modules", ".venv", "__pycache__", ".git", "deps",
        "examples", ".zig-cache", "gradle", ".gradle", "obj",
        "wasm-sdk-rust", "wasm-sdk-go", "wasm-sdk-cpp", "wasm-sdk-kotlin",
        "wasm-sdk-zig", "wasm-sdk-assemblyscript", "wasm-sdk-typescript",
        "wasm-sdk-python", "wasm-sdk-csharp", "wasm-sdk-nim", "wasm-sdk-grain",
        "wasm-sdk-moonbit", "wasm-sdk-lua", "wasm-sdk-swift", "wasm-sdk-java",
    ];
    if let Some(wasm) = find_wasm_recursive(project_path, project_path, skip_dirs, 0) {
        return Ok(wasm);
    }

    Err(TauriFunctionError::new(
        "No built .wasm file found. Build your project first.",
    ))
}

fn find_wasm_recursive(
    dir: &Path,
    project_root: &Path,
    skip_dirs: &[&str],
    depth: u32,
) -> Option<PathBuf> {
    if depth > 8 {
        return None;
    }
    let entries = std::fs::read_dir(dir).ok()?;
    let mut best: Option<PathBuf> = None;
    let mut subdirs = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "wasm") {
            let path_str = path.strip_prefix(project_root).unwrap_or(&path).to_string_lossy();
            if path_str.contains("/deps/") {
                continue;
            }
            // .NET publish/ outputs are not self-contained (need external ICU data etc.)
            if path_str.contains("/publish/") && path.file_name().is_some_and(|n| n == "dotnet.wasm") {
                continue;
            }
            best = pick_better(best, path, project_root);
        } else if path.is_dir() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if !skip_dirs.contains(&name_str.as_ref()) && !name_str.starts_with('.') {
                subdirs.push(path);
            }
        }
    }

    // Check all subdirectories and pick the overall best match
    for sub in subdirs {
        if let Some(found) = find_wasm_recursive(&sub, project_root, skip_dirs, depth + 1) {
            best = pick_better(best, found, project_root);
        }
    }
    best
}

fn pick_better(current: Option<PathBuf>, candidate: PathBuf, project_root: &Path) -> Option<PathBuf> {
    let cand_str = candidate.strip_prefix(project_root).unwrap_or(&candidate).to_string_lossy();
    let Some(cur) = current else { return Some(candidate) };
    let cur_str = cur.strip_prefix(project_root).unwrap_or(&cur).to_string_lossy();

    // AppBundle (single-file bundle) is always preferred
    if cur_str.contains("AppBundle") && !cand_str.contains("AppBundle") {
        return Some(cur);
    }
    if cand_str.contains("AppBundle")
        || cand_str.contains("release")
        || cand_str.contains("production")
    {
        return Some(candidate);
    }
    Some(cur)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunNodeInput {
    pub wasm_path: String,
    pub inputs: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub node_name: String,
}

#[tauri::command]
pub async fn developer_run_node(
    app_handle: AppHandle,
    input: RunNodeInput,
) -> Result<WasmExecutionResult, TauriFunctionError> {
    let engine = TauriWasmEngineState::construct(&app_handle)
        .map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    tokio::spawn(async move {
        let path = PathBuf::from(&input.wasm_path);
        if !path.exists() {
            return Err(TauriFunctionError::new("WASM file not found"));
        }

        let loaded = engine
            .load_auto_from_file(&path)
            .await
            .map_err(|e| {
                TauriFunctionError::new(&format!("Failed to load WASM module: {}", e))
            })?;

        let security = WasmSecurityConfig::permissive();
        let mut instance = loaded
            .instantiate(&engine, security)
            .await
            .map_err(|e| {
                TauriFunctionError::new(&format!("Failed to instantiate module: {}", e))
            })?;

        let exec_input = WasmExecutionInput {
            inputs: input.inputs,
            node_id: "debug".to_string(),
            run_id: "debug".to_string(),
            app_id: "debug".to_string(),
            board_id: "debug".to_string(),
            user_id: "debug".to_string(),
            stream_state: false,
            log_level: 0,
            node_name: input.node_name,
        };

        instance.call_run(&exec_input).await.map_err(|e| {
            TauriFunctionError::new(&format!("Node execution failed: {}", e))
        })
    })
    .await
    .map_err(|e| TauriFunctionError::new(&format!("Task panicked: {}", e)))?
}

async fn load_wasm_nodes_from_path(
    wasm_path: &Path,
    engine: Arc<WasmEngine>,
) -> Result<Vec<(Node, Arc<dyn NodeLogic>)>, TauriFunctionError> {

    let loaded = engine.load_auto_from_file(wasm_path).await.map_err(|e| {
        TauriFunctionError::new(&format!("Failed to load WASM module: {}", e))
    })?;

    let security = WasmSecurityConfig::permissive();
    let mut instance = loaded
        .instantiate(&engine, security.clone())
        .await
        .map_err(|e| TauriFunctionError::new(&format!("Failed to instantiate module: {}", e)))?;

    let definitions = instance.call_get_nodes().await.map_err(|e| {
        TauriFunctionError::new(&format!("Failed to get node definitions: {}", e))
    })?;

    Ok(definitions
        .into_iter()
        .map(|def| {
            let package_id = format!("local::{}", def.name);
            let mut node = build_node_from_definition(&def);
            let permissions = node.wasm.as_ref().map(|w| w.permissions.clone()).unwrap_or_default();
            node.wasm = Some(NodeWasm {
                package_id: package_id.clone(),
                permissions,
            });
            // Include package_id in hash so local nodes get a stable, unique identity
            {
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                node.hash.hash(&mut hasher);
                package_id.hash(&mut hasher);
                node.hash = Some(hasher.finish());
            }
            let logic = Arc::new(
                WasmNodeLogic::from_loaded_with_target(
                    loaded.clone(),
                    engine.clone(),
                    security.clone(),
                    def,
                )
                .with_package_id(package_id),
            ) as Arc<dyn NodeLogic>;
            (node, logic)
        })
        .collect())
}

#[tauri::command]
pub async fn developer_load_into_catalog(
    app_handle: AppHandle,
    project_path: String,
) -> Result<usize, TauriFunctionError> {
    let _ = app_handle.emit(
        "package-status",
        serde_json::json!({ "packageId": format!("dev:{}", project_path), "status": "compiling" }),
    );

    let engine = TauriWasmEngineState::construct(&app_handle)
        .map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    let project = PathBuf::from(&project_path);
    let wasm_path = find_wasm_file(&project).map_err(|e| {
        let _ = app_handle.emit(
            "package-status",
            serde_json::json!({ "packageId": format!("dev:{}", project_path), "status": "error" }),
        );
        e
    })?;
    let node_pairs = match load_wasm_nodes_from_path(&wasm_path, engine).await {
        Ok(pairs) => pairs,
        Err(e) => {
            let _ = app_handle.emit(
                "package-status",
                serde_json::json!({ "packageId": format!("dev:{}", project_path), "status": "error" }),
            );
            return Err(e);
        }
    };
    let count = node_pairs.len();

    if count > 0 {
        let flow_state = TauriFlowLikeState::construct(&app_handle)
            .await
            .map_err(|e| TauriFunctionError::new(&e.to_string()))?;
        let registry_guard = flow_state.node_registry.clone();
        let mut registry = registry_guard.write().await;
        let mut inner = flow_like::state::FlowNodeRegistryInner {
            registry: registry.node_registry.registry.clone(),
        };
        for (node, logic) in node_pairs {
            inner.insert(node, logic);
        }
        registry.node_registry = Arc::new(inner);
        drop(registry);
        let _ = app_handle.emit("catalog-updated", ());
    }

    let _ = app_handle.emit(
        "package-status",
        serde_json::json!({ "packageId": format!("dev:{}", project_path), "status": "ready" }),
    );

    Ok(count)
}

pub async fn load_all_developer_nodes(app_handle: &AppHandle) {
    let engine = match TauriWasmEngineState::construct(app_handle) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("Failed to get WasmEngine for developer node loading: {}", e);
            return;
        }
    };

    let user_dir = match TauriSettingsState::construct(app_handle).await {
        Ok(settings) => {
            let guard = settings.lock().await;
            guard.user_dir.clone()
        }
        Err(e) => {
            tracing::warn!("Failed to get settings for developer node loading: {}", e);
            return;
        }
    };

    let store = load_store(&user_dir);
    if store.projects.is_empty() {
        return;
    }

    let flow_state = match TauriFlowLikeState::construct(app_handle).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Failed to get flow state for developer node loading: {}", e);
            return;
        }
    };

    let mut all_node_pairs: Vec<(Node, Arc<dyn NodeLogic>)> = Vec::new();

    for project in &store.projects {
        let project_path = PathBuf::from(&project.path);
        match find_wasm_file(&project_path) {
            Ok(wasm_path) => match load_wasm_nodes_from_path(&wasm_path, engine.clone()).await {
                Ok(pairs) => {
                    tracing::info!(
                        "Loaded {} developer node(s) from '{}'",
                        pairs.len(),
                        project.name
                    );
                    all_node_pairs.extend(pairs);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to load developer nodes from '{}': {:?}",
                        project.name,
                        e
                    );
                }
            },
            Err(e) => {
                tracing::debug!(
                    "No WASM file found for developer project '{}': {:?}",
                    project.name,
                    e
                );
            }
        }
    }

    if !all_node_pairs.is_empty() {
        let registry_guard = flow_state.node_registry.clone();
        let mut registry = registry_guard.write().await;
        let mut inner = flow_like::state::FlowNodeRegistryInner {
            registry: registry.node_registry.registry.clone(),
        };
        for (node, logic) in all_node_pairs {
            inner.insert(node, logic);
        }
        registry.node_registry = Arc::new(inner);
        drop(registry);
        let _ = app_handle.emit("catalog-updated", ());
        tracing::info!("Developer nodes loaded into catalog");
    }
}
