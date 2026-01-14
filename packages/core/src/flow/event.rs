use std::{collections::HashMap, time::SystemTime};

use flow_like_storage::Path;
use flow_like_types::{FromProto, ToProto, create_id, proto};
use futures::{StreamExt, TryStreamExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    app::App,
    state::FlowLikeState,
    utils::compression::{compress_to_file, from_compressed},
};

use super::{board::VersionType, variable::Variable};

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub enum ReleaseNotes {
    NOTES(String),
    URL(String),
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct CanaryEvent {
    pub weight: f32,
    pub variables: HashMap<String, Variable>,
    pub board_id: String,
    pub board_version: Option<(u32, u32, u32)>,
    pub node_id: String,
    pub created_at: std::time::SystemTime,
    pub updated_at: std::time::SystemTime,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct Event {
    pub id: String,
    pub name: String,
    pub description: String,
    pub board_id: String,
    pub board_version: Option<(u32, u32, u32)>,
    pub node_id: String,
    pub variables: HashMap<String, Variable>,
    pub config: Vec<u8>,
    pub active: bool,

    pub canary: Option<CanaryEvent>,

    pub priority: u32,
    pub event_type: String,
    pub notes: Option<ReleaseNotes>,
    pub event_version: (u32, u32, u32),
    pub created_at: std::time::SystemTime,
    pub updated_at: std::time::SystemTime,

    // A2UI: default page to render for this event
    pub default_page_id: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct ChatEventParameters {
    pub history_elements: Option<u32>,
    pub allow_file_upload: Option<bool>,
    pub allow_voice_input: Option<bool>,
    pub allow_voice_output: Option<bool>,
    pub tools: Option<Vec<String>>,
    pub default_tools: Option<Vec<String>>,
    pub example_messages: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct EmailEventParameters {
    pub mail: Option<String>,
    pub sender_name: Option<String>,
    pub smtp_server: Option<String>,
    pub smtp_port: Option<u16>,
    pub smtp_username: Option<String>,
    pub secret_smtp_password: Option<String>,
    pub imap_server: Option<String>,
    pub imap_port: Option<u16>,
    pub imap_username: Option<String>,
    pub secret_imap_password: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct ApiEventParameters {
    pub path_suffix: Option<String>,
    pub method: Option<String>,
    pub public_endpoint: Option<bool>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
#[serde(untagged)]
pub enum EventPayload {
    ChatEvent(ChatEventParameters),
    MailEvent(EmailEventParameters),
    ApiEvent(ApiEventParameters),
    AnyEvent(HashMap<String, flow_like_types::Value>),
    QuickAction,
}

pub fn canary_equal(a: &Option<CanaryEvent>, b: &Option<CanaryEvent>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => {
            a.board_id == b.board_id
                && a.board_version == b.board_version
                && a.node_id == b.node_id
                && a.weight == b.weight
                && a.variables == b.variables
        }
        (None, None) => true,
        _ => false,
    }
}

impl Event {
    pub async fn upsert(
        &mut self,
        app: &App,
        version_type: Option<VersionType>,
        enforce_id: bool,
    ) -> flow_like_types::Result<Self> {
        if self.id.is_empty() {
            self.id = create_id();
        }

        // If we set an event as deactivated, we do not have to validate the nodes and boards
        if self.active {
            self.validate_event_references(app).await?;
        }

        let old_event = Event::load(&self.id, app, None).await;
        if let Ok(mut old_event) = old_event {
            if old_event.node_id != self.node_id
                || old_event.board_id != self.board_id
                || !canary_equal(&old_event.canary, &self.canary)
                || version_type.is_some()
            {
                let version_type = version_type.unwrap_or(VersionType::Patch);
                old_event.save(app, Some(old_event.event_version)).await?;
                old_event.event_version = match version_type {
                    VersionType::Major => (old_event.event_version.0 + 1, 0, 0),
                    VersionType::Minor => {
                        (old_event.event_version.0, old_event.event_version.1 + 1, 0)
                    }
                    VersionType::Patch => (
                        old_event.event_version.0,
                        old_event.event_version.1,
                        old_event.event_version.2 + 1,
                    ),
                };
            }

            let updated_event = Event {
                id: old_event.id,
                event_version: old_event.event_version,
                created_at: old_event.created_at,
                updated_at: SystemTime::now(),
                ..self.clone()
            };

            updated_event.save(app, None).await?;
            return Ok(updated_event.clone());
        }

        if !enforce_id {
            self.id = create_id();
        }
        self.event_version = (0, 0, 0);
        self.created_at = SystemTime::now();
        self.updated_at = SystemTime::now();
        self.save(app, None).await?;
        Ok(self.clone())
    }

    pub async fn get_versions(&self, app: &App) -> flow_like_types::Result<Vec<(u32, u32, u32)>> {
        let storage_root = Path::from("apps").child(app.id.clone()).child("events");
        let app_state = app
            .app_state
            .clone()
            .ok_or(flow_like_types::anyhow!("App state not found"))?;
        let store = FlowLikeState::project_meta_store(&app_state)
            .await?
            .as_generic();

        let versions_path = storage_root.child("versions").child(self.id.clone());
        let mut list_stream = store
            .list(Some(&versions_path))
            .map_ok(|m| m.location)
            .boxed();

        let mut versions = Vec::new();
        while let Some(Ok(location)) = list_stream.next().await {
            if let Some(version_str) = location.filename() {
                let version = version_str.split('.').collect::<Vec<&str>>();
                let version = version.as_slice();
                if version.len() == 3
                    && let (Ok(major), Ok(minor), Ok(patch)) =
                        (version[0].parse(), version[1].parse(), version[2].parse())
                {
                    versions.push((major, minor, patch));
                }
            }
        }

        Ok(versions)
    }

    pub async fn validate_event_references(&self, app: &App) -> flow_like_types::Result<()> {
        let board = app
            .open_board(self.board_id.clone(), Some(false), self.board_version)
            .await?;

        board.lock().await.nodes.get(&self.node_id).ok_or_else(|| {
            flow_like_types::anyhow!(
                "Node with id {} not found in board {}",
                self.node_id,
                self.board_id
            )
        })?;

        if let Some(canary) = &self.canary {
            let canary_board = app
                .open_board(canary.board_id.clone(), Some(false), canary.board_version)
                .await?;

            canary_board
                .lock()
                .await
                .nodes
                .get(&canary.node_id)
                .ok_or_else(|| {
                    flow_like_types::anyhow!(
                        "Node with id {} not found in board {} (Canary)",
                        canary.node_id,
                        canary.board_id
                    )
                })?;
        }

        Ok(())
    }

    pub async fn load(
        id: &str,
        app: &App,
        version: Option<(u32, u32, u32)>,
    ) -> flow_like_types::Result<Event> {
        let storage_root = Path::from("apps").child(app.id.clone()).child("events");
        let app_state = app
            .app_state
            .clone()
            .ok_or(flow_like_types::anyhow!("App state not found"))?;
        let store = FlowLikeState::project_meta_store(&app_state)
            .await?
            .as_generic();

        let event_path = match version {
            Some(version) => storage_root
                .child("versions")
                .child(id)
                .child(format!("{}.{}.{}", version.0, version.1, version.2)),
            None => storage_root.child(format!("{}.event", id)),
        };

        let event_proto: proto::Event = from_compressed(store, event_path).await?;
        let event = Event::from_proto(event_proto);

        Ok(event)
    }

    pub async fn save(
        &self,
        app: &App,
        version: Option<(u32, u32, u32)>,
    ) -> flow_like_types::Result<()> {
        let storage_root = Path::from("apps").child(app.id.clone()).child("events");
        let state = app
            .app_state
            .clone()
            .ok_or(flow_like_types::anyhow!("App state not found"))?;
        let store = FlowLikeState::project_meta_store(&state)
            .await?
            .as_generic();

        let event_path = match version {
            Some(version) => storage_root
                .child("versions")
                .child(self.id.clone())
                .child(format!("{}.{}.{}", version.0, version.1, version.2)),
            None => storage_root.child(format!("{}.event", self.id)),
        };

        compress_to_file(store, event_path, &self.to_proto()).await?;
        Ok(())
    }

    pub async fn delete(&self, app: &App) -> flow_like_types::Result<()> {
        let event_dir = Path::from("apps")
            .child(app.id.clone())
            .child("events")
            .child(format!("{}.event", self.id));

        let state = app
            .app_state
            .clone()
            .ok_or(flow_like_types::anyhow!("App state not found"))?;
        let store = FlowLikeState::project_meta_store(&state)
            .await?
            .as_generic();
        store.delete(&event_dir).await?;

        // Remove all versions of the event
        let versions_path = Path::from("apps")
            .child(app.id.clone())
            .child("events")
            .child("versions")
            .child(self.id.clone());

        let locations = store
            .list(Some(&versions_path))
            .map_ok(|m| m.location)
            .boxed();

        store
            .delete_stream(locations)
            .try_collect::<Vec<Path>>()
            .await?;

        Ok(())
    }
}
