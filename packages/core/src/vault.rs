use std::{sync::Arc, time::SystemTime, vec};

use cuid2;
use futures::{StreamExt, TryStreamExt};
use object_store::path::Path;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{
    flow::board::Board,
    state::FlowLikeState,
    utils::compression::{compress_to_file, from_compressed},
};

pub mod graph;
pub mod vector;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Vault {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub tags: Vec<String>,
    pub boards: Vec<String>,

    pub bits: Vec<String>,

    pub updated_at: SystemTime,
    pub created_at: SystemTime,

    #[serde(skip)]
    pub app_state: Option<Arc<Mutex<FlowLikeState>>>,
}

impl Clone for Vault {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            author: self.author.clone(),
            tags: self.tags.clone(),
            bits: self.bits.clone(),
            updated_at: self.updated_at,
            created_at: self.created_at,
            app_state: self.app_state.clone(),
            boards: self.boards.clone(),
        }
    }
}

impl Vault {
    pub async fn new(
        name: String,
        description: String,
        author: String,
        bits: Vec<String>,
        app_state: Arc<Mutex<FlowLikeState>>,
    ) -> anyhow::Result<Self> {
        let id = cuid2::create_id();

        let item = Self {
            id,
            name,
            description,
            boards: vec![],
            author,
            bits,
            tags: vec![],
            updated_at: SystemTime::now(),
            created_at: SystemTime::now(),

            app_state: Some(app_state.clone()),
        };

        Ok(item)
    }

    pub async fn load(id: String, app_state: Arc<Mutex<FlowLikeState>>) -> anyhow::Result<Self> {
        let storage_root = Path::from("vaults").child(id.clone());

        let store = app_state
            .lock()
            .await
            .config
            .read()
            .await
            .project_store
            .clone();

        let mut vault: Vault = from_compressed(store, storage_root.child("manifest.vault")).await?;
        vault.app_state = Some(app_state.clone());

        Ok(vault)
    }

    pub async fn create_board(&mut self) -> anyhow::Result<String> {
        let state = self
            .app_state
            .clone()
            .ok_or(anyhow::anyhow!("App state not found"))?;
        let board = Board::new(
            Path::from("vaults").child(self.id.clone()).child("boards"),
            state,
        );
        board.save(None).await?;
        self.boards.push(board.id.clone());
        self.updated_at = SystemTime::now();
        Ok(board.id)
    }

    pub async fn boards_configured(&self) -> bool {
        for board_id in &self.boards {
            let board = self.open_board(board_id.clone(), Some(false)).await;
            if let Ok(board) = board {
                let board = board.lock().await;
                for var in board.variables.values() {
                    if var.default_value.is_none() {
                        return false;
                    }
                }
            }
        }

        true
    }

    pub async fn open_board(
        &self,
        board_id: String,
        register: Option<bool>,
    ) -> anyhow::Result<Arc<Mutex<Board>>> {
        if let Some(app_state) = &self.app_state {
            let board = app_state
                .lock()
                .await
                .board_registry()
                .lock()
                .await
                .get(&board_id)
                .cloned();

            if let Some(board) = board {
                return Ok(board);
            }
        }

        let board_dir = Path::from("vaults")
            .child(self.id.clone())
            .child("boards")
            .child(board_id.clone());
        let state = self
            .app_state
            .clone()
            .ok_or(anyhow::anyhow!("App state not found"))?;

        let board = Board::load(board_dir, state).await?;
        let board_ref = Arc::new(Mutex::new(board));
        let register = register.unwrap_or(false);
        if register {
            if let Some(app_state) = &self.app_state {
                app_state
                    .lock()
                    .await
                    .board_registry()
                    .lock()
                    .await
                    .insert(board_id.clone(), board_ref.clone());
            }
        }

        Ok(board_ref)
    }

    pub async fn delete_board(&mut self, board_id: &str) -> anyhow::Result<()> {
        let board_index = self
            .boards
            .iter()
            .position(|x| x == board_id)
            .ok_or(anyhow::anyhow!("Board not found"))?;
        let board_id = self.boards.remove(board_index);
        let board_dir = Path::from("vaults")
            .child(self.id.clone())
            .child("boards")
            .child(board_id.clone());

        let store = self
            .app_state
            .clone()
            .ok_or(anyhow::anyhow!("App state not found"))?;
        let store = store.lock().await.config.read().await.project_store.clone();
        let locations = store.list(Some(&board_dir)).map_ok(|m| m.location).boxed();
        store
            .delete_stream(locations)
            .try_collect::<Vec<Path>>()
            .await?;

        if let Some(app_state) = &self.app_state {
            app_state
                .lock()
                .await
                .board_registry()
                .lock()
                .await
                .remove(&board_id);
        }

        self.updated_at = SystemTime::now();
        Ok(())
    }

    pub async fn save(&self) -> anyhow::Result<()> {
        if let Some(app_state) = &self.app_state {
            let store = app_state
                .lock()
                .await
                .config
                .read()
                .await
                .project_store
                .clone();
            let registry_guard = app_state.lock().await;
            let registry = registry_guard.board_registry.lock().await;

            for board_id in &self.boards {
                let board = registry.get(board_id).cloned();
                if let Some(board) = board {
                    board.lock().await.save(Some(store.clone())).await?;
                }
            }
        }

        let store = self
            .app_state
            .clone()
            .ok_or(anyhow::anyhow!("App state not found"))?;
        let store = store.lock().await.config.read().await.project_store.clone();

        let manifest_path = Path::from("vaults")
            .child(self.id.clone())
            .child("manifest.vault");

        compress_to_file(store, manifest_path, self).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{state::FlowLikeConfig, utils::http::HTTPClient};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    async fn flow_state() -> Arc<Mutex<crate::state::FlowLikeState>> {
        let config: FlowLikeConfig = FlowLikeConfig::new(
            None,
            Arc::new(object_store::memory::InMemory::new()),
            Arc::new(object_store::memory::InMemory::new()),
            Arc::new(object_store::memory::InMemory::new()),
        );
        let (http_client, _refetch_rx) = HTTPClient::new();
        let (flow_like_state, _) = crate::state::FlowLikeState::new(config, http_client);
        Arc::new(Mutex::new(flow_like_state))
    }

    #[tokio::test]
    async fn serialize_vault() {
        let vault = crate::vault::Vault {
            id: "id".to_string(),
            name: "name".to_string(),
            description: "description".to_string(),
            author: "author".to_string(),
            tags: vec!["tag1".to_string(), "tag2".to_string()],
            boards: vec!["board1".to_string(), "board2".to_string()],
            bits: vec!["bit1".to_string(), "bit2".to_string()],
            updated_at: std::time::SystemTime::now(),
            created_at: std::time::SystemTime::now(),
            app_state: Some(flow_state().await),
        };

        let ser = bitcode::serialize(&vault).unwrap();
        let deser: crate::vault::Vault = bitcode::deserialize(&ser).unwrap();

        assert_eq!(vault.id, deser.id);
    }
}
