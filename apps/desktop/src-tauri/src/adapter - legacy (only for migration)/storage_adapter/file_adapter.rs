use async_trait::async_trait;
use futures::future::join_all;
use ignore::WalkBuilder;
use std::fs;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Weak},
    time::SystemTime,
};
use tauri::AppHandle;
use futures::stream::StreamExt;
use crate::models::embedding::local::LocalEmbeddingModel;
use crate::models::embedding::EmbeddingModelLogic;
use crate::utils::adapter::get_vault_dir;
use crate::utils::compression::read_compressed_file_to_string;
use crate::utils::file::FileMetadata;
use crate::utils::file_watcher::FileWatchEntry;
use crate::utils::file_watcher::FileWatcher;
use crate::utils::lock::LockFile;
use crate::utils::pdf::Page;
use crate::utils::rag::chunk_pages;
use crate::utils::rag::embed_chunks;
use crate::utils::rag::extract_pages;
use crate::utils::rag::preprocess_file;
use crate::vault::path_diff_vault;
use crate::db::vector::VectorStore;
use crate::vault::chunk::Chunk;
use crate::{
    adapter::{
        Adapter, AdapterConfig, AdapterConfigEntry, AdapterConfigType, AdapterMetadata,
        AdapterTrait, AdapterTraitConstructor,
    },
    history::History,
    vault::Vault,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::Emitter;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VectorDBFileEntry {
    pub hash: String,
    pub reference: String,
    pub vector : Vec<f32>,
    pub page : Option<u32>,
    pub metadata: FileMetadata,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}

pub struct FileAdapter {
    pub adapter: Weak<Mutex<Adapter>>,
    pub vault: Weak<Mutex<Vault>>,
    pub files: Vec<(String, SystemTime, u64)>,
    pub app_handle: Option<AppHandle>,
    pub vault_dir: Option<PathBuf>,
}

impl AdapterTraitConstructor for FileAdapter {
    async fn new(
        adapter: Weak<Mutex<Adapter>>,
        vault: Weak<Mutex<Vault>>,
        _params: HashMap<String, AdapterConfig>,
        app_handle: Option<AppHandle>,
    ) -> Arc<Mutex<Self>> {
        let files = vec![];
        Arc::new(Mutex::new(Self {
            adapter,
            vault,
            files,
            app_handle: app_handle,
            vault_dir: None,
        }))
    }

    fn metadata() -> AdapterMetadata {
        return AdapterMetadata {
            name: "File Adapter".to_string(),
            description: "Allows you to connect local data!".to_string(),
            thumbnail: Some("/adapter/adapter_files.svg".to_string()),
            thumbnail_darkmode: Some("/adapter/adapter_files_darkmode.svg".to_string()),
            author: "TM9657 GmbH".to_string(),
            adapter_type: "file_adapter".to_string(),
            actions: vec![(
                "Index".to_string(),
                "Ingest all selected files into the index".to_string(),
            ),
            (
                "Optimize".to_string(),
                "Optimize the Index for Search Speed".to_string(),
            )],
        };
    }
}

fn build_default_config() -> HashMap<String, AdapterConfig> {
    let mut default = HashMap::new();

    let mut variables: HashMap<String, AdapterConfigEntry> = HashMap::new();
    variables.insert(
        "Files".to_string(),
        AdapterConfigEntry {
            value: None,
            optional: true,
            array: true,
            description: "Files to index".to_string(),
            readonly: false,
            runtime_editable: false,
            secret: false,
            value_type: AdapterConfigType::FilePath,
        },
    );

    variables.insert(
        "Folders".to_string(),
        AdapterConfigEntry {
            value: None,
            optional: true,
            array: true,
            description: "Files to index".to_string(),
            readonly: false,
            runtime_editable: false,
            secret: false,
            value_type: AdapterConfigType::DirectoryPath,
        },
    );

    default.insert("Content".to_string(), AdapterConfig { values: variables });

    return default;
}

fn merge_configs(
    default: HashMap<String, AdapterConfig>,
    mut config: HashMap<String, AdapterConfig>,
) -> HashMap<String, AdapterConfig> {
    for (key, value) in default.iter() {
        if !config.contains_key(key) {
            config.insert(key.clone(), value.clone());
        }
    }
    config
}

impl FileAdapter {
    pub fn publish_update(&self, trigger: &str, message: Value, vault_id: &str) {
        if let Some(app_handle) = &self.app_handle {
            let _ = app_handle.emit(
                &format!("adapter:{}:trigger:{}", vault_id, trigger),
                message,
            );
        }
    }

    pub fn watch_entry_from_reference_relative(&self, files_dir: &PathBuf, reference: &str) -> FileWatchEntry {
        let reference = PathBuf::from(reference);
        let path = files_dir.join(reference);
        let parent = path.parent().unwrap();
        let file_name = path.file_stem().unwrap().to_str().unwrap();
        let entry_path = parent.join(format!("{}.json.gz", file_name));
        let entry_string = read_compressed_file_to_string(entry_path.to_str().unwrap()).unwrap();
        let entry: FileWatchEntry = serde_json::from_str(&entry_string).unwrap();
        return entry;
    }

    pub fn watch_entry_from_reference(&self, reference: &str) -> FileWatchEntry {
        let path = PathBuf::from(reference);
        let parent = path.parent().unwrap();
        let file_name = path.file_stem().unwrap().to_str().unwrap();
        let entry_path = parent.join(format!("{}.json.gz", file_name));
        let entry_string = read_compressed_file_to_string(entry_path.to_str().unwrap()).unwrap();
        let entry: FileWatchEntry = serde_json::from_str(&entry_string).unwrap();
        return entry;
    }

    async fn index(&self, handler: &AppHandle) {
        let vault_id = self.vault.upgrade().unwrap().lock().await.id.clone();
        self.publish_update("Index", json!({"progress": 0, "message": "Initializing... 🚀"}), &vault_id);
        let params = self.get_config().await;
        let vault_dir = get_vault_dir(&self.vault).await;
        let lock_path = vault_dir.clone().unwrap().join(".lock");
        let lock = LockFile::new(&lock_path);
        if lock.exists() {
            self.publish_update("Index", json!({"progress": 1, "message": "Indexing in Progress!"}), &vault_id);
            return;
        }

        lock.create().unwrap();

        let embedding_models = self.vault.upgrade().unwrap().lock().await.embedding_models.clone();

        if vault_dir.is_none() {
            println!("Vault not found");
            self.publish_update("Index", json!({"progress": 1, "message": "Vault not found!"}), &vault_id);
            lock.delete().unwrap();
            return;
        }

        let vault_dir = vault_dir.unwrap();

        let mut paths: Vec<PathBuf> = vec![];
        if let Some(files) = params
            .get("Content")
            .unwrap()
            .values
            .get("Files")
            .unwrap()
            .value
            .clone()
        {
            let files = match files {
                Value::Array(vec) => vec
                    .into_iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect(),
                _ => Vec::new(), // Oder eine Fehlerbehandlung implementieren
            };
            paths = files.into_iter().map(PathBuf::from).collect();
        }

        if let Some(folders) = params
            .get("Content")
            .unwrap()
            .values
            .get("Folders")
            .unwrap()
            .value
            .clone()
        {
            let folders = match folders {
                Value::Array(vec) => vec
                    .into_iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect(),
                _ => Vec::new(), // Oder eine Fehlerbehandlung implementieren
            };
            let folders = folders
                .into_iter()
                .map(PathBuf::from)
                .collect::<Vec<PathBuf>>();
            paths = vec![paths, folders].concat();
        }

        let mut file_watcher = FileWatcher::new(vault_dir.join("watcher"), &paths);
        file_watcher.build_state().await;

        self.publish_update("Index", json!({"progress": 0.2, "message": "Scanning Files... 🔎"}), &vault_id);

        let new_files = file_watcher.get_new();
        let changed_files = file_watcher.get_changed();
        let deleted_files = file_watcher.get_deleted();

        if new_files.len() == 0 && changed_files.len() == 0 && deleted_files.len() == 0 {
            self.publish_update("Index", json!({"progress": 1, "message": "No Changes Found!"}), &vault_id);
            lock.delete().unwrap();
            return;
        }

        let files_dir = vault_dir.join("references");

        if !files_dir.exists() {
            fs::create_dir_all(&files_dir).unwrap();
        }

        let max_concurrent_tasks = 10;
        let new_file_tasks = futures::stream::iter(new_files.into_iter().map(|path| {
            let files_dir = files_dir.clone();
            async move {
                preprocess_file(&path, &files_dir, &handler.clone()).await.ok()
            }
        }))
        .buffer_unordered(max_concurrent_tasks) // Limit the number of concurrent tasks
        .collect::<Vec<_>>();

        let changed_file_tasks = futures::stream::iter(changed_files.into_iter().map(|entry| {
            let files_dir = files_dir.clone();
            async move {
                self.remove_old_file(&entry, &files_dir).await;
                preprocess_file(&entry.file_path, &files_dir, &handler.clone()).await.ok()
            }
        }))
        .buffer_unordered(max_concurrent_tasks)
        .collect::<Vec<_>>();

        let deleted_file_tasks = futures::stream::iter(deleted_files.into_iter().map(|entry| {
            let files_dir = files_dir.clone();
            file_watcher.resolve(&entry, true);
            async move {
                self.remove_old_file(&entry, &files_dir).await
            }
        }))
        .buffer_unordered(max_concurrent_tasks)
        .collect::<Vec<_>>();

        let res: (Vec<Option<PathBuf>>, Vec<Option<PathBuf>>, Vec<Vec<PathBuf>>) = futures::join!(new_file_tasks, changed_file_tasks, deleted_file_tasks);
        let mut preprocessed_files : Vec<PathBuf> = res.0.clone().into_iter().filter_map(|x| x).collect();
        preprocessed_files.extend(res.1.clone().into_iter().filter_map(|x| x));

        let pages = join_all(
            preprocessed_files.into_iter().map(|path| async move {
                match extract_pages(&path).await {
                    Ok(pages) => pages,
                    Err(e) => {
                        println!("Error extracting pages: {:?}", e);
                        vec![]
                    }
                }
            })
        ).await.into_iter().flatten().collect::<Vec<Page>>();

        let pages = pages.into_iter().map(|page| {
            let mut page = page;
            page.reference = path_diff_vault(&files_dir, &PathBuf::from(page.reference)).to_str().unwrap().to_string();
            page
        }).collect::<Vec<Page>>();

        self.publish_update("Index", json!({"progress": 0.2, "message": format!("Found {} Pages! 📜", pages.len())}), &vault_id);

        let mut added_to_fts = false;

        let amount_of_models = embedding_models.len();
        let mut model_index = 0;

        for model in embedding_models {
            let model = LocalEmbeddingModel::new(&model, &handler.clone()).await;
            let model = Arc::new(Mutex::new(model));
            if model.lock().await.is_none() {
                println!("Model not found");
                continue;
            }
            let model = model.clone().lock().await.clone().unwrap();
            let chunks = match chunk_pages(&pages, model.clone(), |page_nr| {
                self.publish_update("Index", json!({"progress": 0.2 + (0.8 * model_index as f32 / amount_of_models as f32) + (0.5 * page_nr as f32 / pages.len() as f32 * 0.8 * 1.0 / amount_of_models as f32), "message": format!("[{} / {}], Chunking.. 📚", page_nr, pages.len())}), &vault_id);
            }).await {
                Ok(chunks) => chunks,
                Err(e) => {
                    println!("Error chunking pages: {:?}", e);
                    continue;
                }
            };

            let embeddings = match embed_chunks(&chunks, model.clone(), |chunk_nr| {
                self.publish_update("Index", json!({"progress": 0.2 + (0.8 * model_index as f32 / amount_of_models as f32) + (0.5 * 0.8 * 1.0 / amount_of_models as f32) + (0.5 * chunk_nr as f32 / chunks.len() as f32 * 0.8 * 1.0 / amount_of_models as f32), "message": format!("[{} / {}], Embedding... 💿",chunk_nr, chunks.len())}), &vault_id);
            }).await {
                Ok(embeddings) => embeddings,
                Err(e) => {
                    println!("Error embedding chunks: {:?}", e);
                    continue;
                }
            };

            let vault = self.vault.upgrade().unwrap().lock().await.clone();
            let vector_db_entries: Vec<VectorDBFileEntry> = chunks.clone().into_iter().zip(embeddings.into_iter()).map(|(chunk, embedding)| {
                let entry = self.watch_entry_from_reference_relative(&files_dir, &chunk.reference);
                let metadata = FileMetadata::new(&entry.file_path);
                let mut page = 0;
                if let Some(meta) = chunk.metadata.clone() {
                    if let Some(page_val) = meta.get("page") {
                        page = page_val.as_u64().unwrap() as u32;
                    }
                }
                vault.write_chunk(&chunk);
                let entry = VectorDBFileEntry {
                    hash: chunk.hash,
                    reference: chunk.reference,
                    vector: embedding,
                    page: Some(page),
                    metadata: metadata,
                    created_at: SystemTime::now(),
                    updated_at: SystemTime::now(),
                };
                entry
            }).collect();
            drop(vault);

            let model_id = model.bit.id.clone();
            let vector_db = self.vault.upgrade().unwrap().lock().await.get_vector_db(&model_id).await;

            self.publish_update("Index", json!({"progress": 0.2 + (0.8 * model_index as f32 / amount_of_models as f32) + (0.8 * 1.0 / amount_of_models as f32), "message": "Writing to Disk 💾"}), &vault_id);

            let mut vector_db = match vector_db {
                Some(vector_db) => vector_db,
                None => {
                    println!("Vector DB not found");
                    continue;
                }
            };

            vector_db.upsert(vector_db_entries).await;

            let mut deleted_chunks = vec![];

            for path in res.2.clone() {
                for file in path {
                    let relative_path = path_diff_vault(&files_dir, &file);
                    let relative_path = relative_path.to_str().unwrap();
                    deleted_chunks.extend(Chunk::chunks_from_reference(&vault_dir.join("chunks"), relative_path));
                    vector_db.delete(&format!("reference = {}", file.to_str().unwrap())).await;
                }
            }

            println!("Deleted chunks: {:?}", deleted_chunks);

            model_index += 1;
            if !added_to_fts {
                added_to_fts = true;
                let fts_input = chunks.into_iter().map(|chunk| (chunk.hash, chunk.text.unwrap_or(String::from("")))).collect::<Vec<(String, String)>>();
                let fts = self.vault.upgrade().unwrap().lock().await.get_fts("default").await;
                if let Some(fts) = fts {
                    match fts.index(&fts_input) {
                        Ok(_) => {},
                        Err(e) => {
                            println!("Error indexing FTS: {:?}", e);
                        }
                    }
                    for chunk in &deleted_chunks {
                        match fts.remove_hash(&chunk.hash) {
                            Ok(_) => {},
                            Err(e) => {
                                println!("Error removing hash from FTS: {:?}", e);
                            }
                        };
                    }
                }
            }

            for chunk in deleted_chunks {
                chunk.delete(&vault_dir.join("chunks"));
            }
        }

        res.0.into_iter().for_each(|path| {
            if let Some(path) = path {
                let entry = self.watch_entry_from_reference(path.to_str().unwrap());
                file_watcher.resolve(&entry, false);
            }
        });

        res.1.into_iter().for_each(|path| {
            if let Some(path) = path {
                let entry = self.watch_entry_from_reference(path.to_str().unwrap());
                file_watcher.resolve(&entry, false);
            }
        });

        file_watcher.save_state();

        lock.delete().unwrap();
        self.publish_update("Index", json!({"progress": 1, "message": "Done :)"}), &vault_id);

        return;
    }

    pub async fn optimize(&self, _handler: &AppHandle) {
        let lock = LockFile::new(&self.vault_dir.clone().unwrap().join(".lock_opt"));
        if lock.exists() {
            return;
        }

        lock.create().unwrap();

        let vault_id = self.vault.upgrade().unwrap().lock().await.id.clone();
        let embedding_models = self.vault.upgrade().unwrap().lock().await.embedding_models.clone();

        self.publish_update("Optimize", json!({"progress": 0.1, "message": "Initializing"}), &vault_id);

        for (i, model_id) in embedding_models.iter().enumerate() {
            let vector_db = self.vault.upgrade().unwrap().lock().await.get_vector_db(&model_id.id).await;
            vector_db.unwrap().optimize().await;
            self.publish_update("Optimize", json!({"progress": 0.1 + 0.9 * i as f32 / embedding_models.len() as f32, "message": format!("Optimizing [{}/{}]", i, embedding_models.len())}), &vault_id);
        }

        lock.delete().unwrap();
    }

    pub async fn remove_old_file(&self, entry: &FileWatchEntry, files_dir: &PathBuf) -> Vec<PathBuf> {
        let mut removed_files = vec![];
        let mut old_path = files_dir.join(format!("{}.md", entry.last_hash));
        if !old_path.exists() {
            old_path = files_dir.join(format!("{}.pdf", entry.last_hash));
        }
        let old_image_dir = files_dir.join(format!("{}", entry.last_hash));
        let info_file = files_dir.join(format!("{}.json.gz", entry.last_hash));
        if old_path.exists() {
            fs::remove_file(&old_path).unwrap();
            removed_files.push(old_path);
        }
        if info_file.exists() {
            fs::remove_file(info_file).unwrap();
        }
        if old_image_dir.exists() {
            let walker = WalkBuilder::new(old_image_dir.clone()).build();
            for entry in walker {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        if path.is_file() {
                            removed_files.push(path.to_path_buf());
                        }
                    }
                    Err(e) => {
                        println!("Error removing old image: {:?}", e);
                    }
                }
            }
            fs::remove_dir_all(old_image_dir).unwrap();
        }

        return removed_files;
    }
}

#[async_trait]
impl AdapterTrait for FileAdapter {
    async fn save(&self) {
        unimplemented!()
    }

    async fn trigger_action(&mut self, app_handle: &AppHandle, action: String, id: String) {
        println!("Triggering action: {} with id {}", action, id);
        self.app_handle = Some(app_handle.clone());
        self.vault_dir = get_vault_dir(&self.vault).await;
        match action.as_str() {
            "Index" => {
                self.index(app_handle).await;
                return;
            },
            "Optimize" => {
                self.optimize(app_handle).await;
                return;
            },
            _ => {}
        }
    }

    async fn get_config(&self) -> HashMap<String, AdapterConfig> {
        let default_config = build_default_config();

        if let Some(adapter) = self.adapter.upgrade() {
            let default_config = merge_configs(default_config, adapter.lock().await.params.clone());
            return default_config;
        }

        return default_config;
    }

    async fn set_config(&mut self, config: HashMap<String, AdapterConfig>, runtime: bool) {
        if let Some(adapter) = self.adapter.upgrade() {
            adapter.lock().await.params = config;
            return;
        }

        println!("FAILED TO SET CONFIG; ADAPTER NOT FOUND")
    }

    async fn create(&mut self) {
        println!("Creating File Adapter");
    }

    async fn load(&mut self) {
        println!("Loaded File Adapter");
    }

    async fn is_ready(&mut self) -> bool {
        let mut configured = true;
        let adapter = self.adapter.upgrade().unwrap().lock().await.params.clone();
        adapter.into_iter().for_each(|(_key, value)| {
            value.values.into_iter().for_each(|(_key, value)| {
                if value.value.is_none() && !value.optional {
                    configured = false;
                }
            });
        });

        return configured;
    }

    async fn push(&self, _message: Value) {
        unimplemented!()
    }

    async fn search(&self, _query: History) {
        unimplemented!()
    }

    fn meta(&self) -> AdapterMetadata {
        return Self::metadata();
    }
}

impl Drop for FileAdapter {
    fn drop(&mut self) {
        let vault_dir = self.vault_dir.clone();
        if vault_dir.is_none() {
            return;
        }
        let lock = LockFile::new(&vault_dir.clone().unwrap().join(".lock"));
        if lock.exists() {
            lock.delete().unwrap();
        }
        let lock = LockFile::new(&vault_dir.clone().unwrap().join(".lock_opt"));
        if lock.exists() {
            lock.delete().unwrap();
        }
        println!("Removing Locks");
    }
}
