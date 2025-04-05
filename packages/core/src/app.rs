use crate::{
    bit::BitMeta,
    flow::board::Board,
    state::FlowLikeState,
    utils::compression::{compress_to_file, from_compressed},
};
use flow_like_storage::Path;
use flow_like_types::{create_id, sync::Mutex, FromProto, ToProto};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::SystemTime, vec};

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub enum StandardInterfaces {
    Chat,
    Search,
    Form,
    List,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct FrontendConfiguration {
    pub landing_page: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct App {
    pub id: String,
    pub meta: std::collections::HashMap<String, BitMeta>,
    pub authors: Vec<String>,

    pub bits: Vec<String>,
    pub boards: Vec<String>,

    pub updated_at: SystemTime,
    pub created_at: SystemTime,

    pub frontend: Option<FrontendConfiguration>,

    #[serde(skip)]
    pub app_state: Option<Arc<Mutex<FlowLikeState>>>,
}

impl Clone for App {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            meta: self.meta.clone(),
            authors: self.authors.clone(),
            boards: self.boards.clone(),
            bits: self.bits.clone(),
            updated_at: self.updated_at,
            created_at: self.created_at,
            app_state: self.app_state.clone(),
            frontend: self.frontend.clone(),
        }
    }
}

impl App {
    pub async fn new(
        id: Option<String>,
        meta: BitMeta,
        bits: Vec<String>,
        app_state: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<Self> {
        let id = id.unwrap_or(create_id());

        let mut meta_map = std::collections::HashMap::new();
        meta_map.insert("en".to_string(), meta);

        let item = Self {
            id,
            meta: meta_map,
            authors: vec![],
            bits,
            boards: vec![],
            updated_at: SystemTime::now(),
            created_at: SystemTime::now(),
            frontend: None,

            app_state: Some(app_state.clone()),
        };

        Ok(item)
    }

    pub async fn load(id: String, app_state: Arc<Mutex<FlowLikeState>>) -> flow_like_types::Result<Self> {
        let storage_root = Path::from("apps").child(id.clone());

        let store = FlowLikeState::project_store(&app_state).await?.as_generic();

        let vault: flow_like_types::proto::App =
            from_compressed(store, storage_root.child("manifest.app")).await?;
        let mut vault = App::from_proto(vault);
        vault.app_state = Some(app_state.clone());

        Ok(vault)
    }

    pub async fn create_board(&mut self, id: Option<String>) -> flow_like_types::Result<String> {
        let storage_root = Path::from("apps").child(self.id.clone());
        let state = self
            .app_state
            .clone()
            .ok_or(flow_like_types::anyhow!("App state not found"))?;
        let board = Board::new(id, storage_root, state);
        board.save(None).await?;
        self.boards.push(board.id.clone());
        self.updated_at = SystemTime::now();
        Ok(board.id)
    }

    pub async fn boards_configured(&self) -> bool {
        for board_id in &self.boards {
            let board = self.open_board(board_id.clone(), Some(false)).await;
            if let Ok(board) = board {
                let vars = board
                    .lock()
                    .await
                    .variables
                    .values()
                    .cloned()
                    .collect::<Vec<_>>();
                for var in vars {
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
    ) -> flow_like_types::Result<Arc<Mutex<Board>>> {
        let storage_root = Path::from("apps").child(self.id.clone());
        if let Some(app_state) = &self.app_state {
            let board = app_state.lock().await.get_board(&board_id);

            if let Ok(board) = board {
                return Ok(board);
            }
        }

        let state = self
            .app_state
            .clone()
            .ok_or(flow_like_types::anyhow!("App state not found"))?;

        let board = Board::load(storage_root, &board_id, state).await?;
        let board_ref = Arc::new(Mutex::new(board));
        let register = register.unwrap_or(false);
        if register {
            if let Some(app_state) = &self.app_state {
                app_state
                    .lock()
                    .await
                    .register_board(&board_id, board_ref.clone())?;
            }
        }

        Ok(board_ref)
    }

    pub async fn delete_board(&mut self, board_id: &str) -> flow_like_types::Result<()> {
        let board_index = self
            .boards
            .iter()
            .position(|x| x == board_id)
            .ok_or(flow_like_types::anyhow!("Board not found"))?;
        let board_id = self.boards.remove(board_index);
        let board_dir = Path::from("apps")
            .child(self.id.clone())
            .child(format!("{}.board", board_id.clone()));

        let state = self
            .app_state
            .clone()
            .ok_or(flow_like_types::anyhow!("App state not found"))?;
        let store = FlowLikeState::project_store(&state).await?.as_generic();
        store.delete(&board_dir).await?;

        if let Some(app_state) = &self.app_state {
            app_state.lock().await.remove_board(&board_id)?;
        }

        self.updated_at = SystemTime::now();
        Ok(())
    }

    pub async fn save(&self) -> flow_like_types::Result<()> {
        if let Some(app_state) = &self.app_state {
            let store = FlowLikeState::project_store(app_state).await?.as_generic();

            let board_refs = {
                let guard = app_state.lock().await;
                let mut refs = Vec::with_capacity(self.boards.len());

                for board_id in &self.boards {
                    if let Ok(board) = guard.get_board(board_id) {
                        refs.push(board.clone());
                    }
                }
                refs
            };

            for board in board_refs {
                let tmp = board.lock().await.clone();
                tmp.save(Some(store.clone())).await?;
            }
        }

        let store = self
            .app_state
            .clone()
            .ok_or(flow_like_types::anyhow!("App state not found"))?;
        let store = FlowLikeState::project_store(&store).await?.as_generic();

        let manifest_path = Path::from("apps")
            .child(self.id.clone())
            .child("manifest.app");

        let proto_app = self.to_proto();
        compress_to_file(store, manifest_path, &proto_app).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{state::FlowLikeConfig, utils::http::HTTPClient};
    use flow_like_storage::files::store::FlowLikeStore;
    use flow_like_types::sync::Mutex;
    use flow_like_types::{tokio, Message};
    use flow_like_types::{FromProto, ToProto};
    use std::sync::Arc;

    async fn flow_state() -> Arc<Mutex<crate::state::FlowLikeState>> {
        let mut config: FlowLikeConfig = FlowLikeConfig::new();
        config.register_project_store(FlowLikeStore::Other(Arc::new(
            flow_like_storage::object_store::memory::InMemory::new(),
        )));
        let (http_client, _refetch_rx) = HTTPClient::new();
        let (flow_like_state, _) = crate::state::FlowLikeState::new(config, http_client);
        Arc::new(Mutex::new(flow_like_state))
    }

    #[tokio::test]
    async fn serialize_app() {
        let app = crate::app::App {
            id: "id".to_string(),
            meta: std::collections::HashMap::new(),
            authors: vec!["author1".to_string(), "author2".to_string()],
            boards: vec!["board1".to_string(), "board2".to_string()],
            bits: vec!["bit1".to_string(), "bit2".to_string()],
            updated_at: std::time::SystemTime::now(),
            created_at: std::time::SystemTime::now(),
            app_state: Some(flow_state().await),
            frontend: None,
        };

        let mut buf = Vec::new();
        app.to_proto().encode(&mut buf).unwrap();
        let deser = super::App::from_proto(flow_like_types::proto::App::decode(&buf[..]).unwrap());

        assert_eq!(app.id, deser.id);
    }
}
