use crate::{state::TauriSettingsState, utils::UiEmitTarget};
use flow_like::app::App;
use flow_like::flow::execution::{InternalRun, LogMeta};
use flow_like::flow_like_storage::Path;
use flow_like::hub::Hub;
use flow_like::state::RunData;
use flow_like::{flow::execution::RunPayload, state::FlowLikeState};
use flow_like_types::intercom::{BufferedInterComHandler, InterComEvent};
use flow_like_types::tokio_util::sync::CancellationToken;
use flow_like_types::{Value, sync::mpsc, tokio::sync::Mutex};
use flow_like_types::{json, tokio};
use std::time::Duration;
use std::{path::PathBuf, sync::Arc};
use tauri::AppHandle;

// Maximum number of events to queue. 100,000 should be plenty for local handling.
const MAX_QUEUE_SIZE: usize = 100_000;

pub struct EventBusEvent {
    pub payload: Option<Value>,
    pub app_id: String,
    pub event_id: String,

    pub offline: bool,

    // Either Access Token or PAT
    pub token: Option<String>,

    pub callback: Option<Arc<BufferedInterComHandler>>,
}

impl EventBusEvent {
    pub async fn execute(
        &self,
        app_handle: &AppHandle,
        flow_like_state: Arc<Mutex<FlowLikeState>>,
        hub: &Hub,
    ) -> flow_like_types::Result<Option<LogMeta>> {
        let Ok(app) = App::load(self.app_id.clone(), flow_like_state.clone()).await else {
            return Err(flow_like_types::anyhow!("App not found"));
        };

        let loaded_event = app.get_event(&self.event_id, None).await?;
        let payload = RunPayload {
            id: loaded_event.node_id.clone(),
            payload: self.payload.to_owned(),
        };

        let board_version = loaded_event.board_version;
        let board_id = loaded_event.board_id.clone();

        let Ok(board) = app
            .open_board(board_id.clone(), Some(false), board_version)
            .await
        else {
            return Err(flow_like_types::anyhow!("Board not found"));
        };

        let board = Arc::new(board.lock().await.clone());
        let profile = TauriSettingsState::current_profile(app_handle).await?;

        let app_handle_clone = app_handle.clone();
        let buffered_sender = if let Some(callback) = &self.callback {
            callback.clone()
        } else {
            BufferedInterComHandler::new(
                Arc::new(move |event| {
                    let app_handle = app_handle_clone.clone();
                    Box::pin({
                        async move {
                            let first_event = event.first();

                            if let Some(first_event) = first_event {
                                crate::utils::emit_throttled(
                                    &app_handle,
                                    UiEmitTarget::All,
                                    &first_event.event_type,
                                    event.clone(),
                                    std::time::Duration::from_millis(150),
                                );
                            }

                            Ok(())
                        }
                    })
                }),
                Some(100),
                Some(400),
                Some(true),
            )
        };

        let mut credentials = None;
        if !self.offline
            && let Some(token) = &self.token
        {
            let shared_credentials = hub.shared_credentials(token, &self.app_id).await?;
            credentials = Some(shared_credentials);
        }

        let mut internal_run = InternalRun::new(
            &self.app_id,
            board,
            Some(loaded_event),
            &flow_like_state,
            &profile.hub_profile,
            &payload,
            false,
            buffered_sender.into_callback(),
            credentials,
            self.token.clone(),
        )
        .await?;

        let run_id = internal_run.run.lock().await.id.clone();

        let _send_result = buffered_sender
            .send(InterComEvent::with_type(
                "run_initiated",
                json::json!({ "run_id": run_id.clone()}),
            ))
            .await;

        let cancellation_token = CancellationToken::new();
        let run_data = RunData::new(&board_id, &payload.id, None, cancellation_token.clone());

        flow_like_state.lock().await.register_run(&run_id, run_data);

        let meta = tokio::select! {
            result = internal_run.execute(flow_like_state.clone()) => result,
            _ = cancellation_token.cancelled() => {
                println!("Board execution cancelled for run: {}", run_id);
                match tokio::time::timeout(Duration::from_secs(30), internal_run.flush_logs_cancelled()).await {
                    Ok(Ok(Some(meta))) => {
                        Some(meta)
                    },
                    Ok(Ok(None)) => {
                        println!("No meta flushing early");
                        None
                    },
                    Ok(Err(e)) => {
                        println!("Error flushing logs early for run: {}, {:?}", run_id, e);
                        None
                    },
                    Err(_) => {
                        println!("Timeout while flushing logs early for run: {}", run_id);
                        None
                    }
                }
            }
        };

        let app_id = self.app_id.clone();

        if let Err(err) = buffered_sender.flush().await {
            println!("Error flushing buffered sender: {}", err);
        }

        if let Some(meta) = &meta {
            let db = {
                let guard = flow_like_state.lock().await;
                let guard = guard.config.read().await;

                guard.callbacks.build_logs_database.clone()
            };
            let db_fn = db
                .as_ref()
                .ok_or_else(|| flow_like_types::anyhow!("No log database configured"))?;
            let base_path = Path::from("runs").child(app_id).child(board_id);
            let db = db_fn(base_path.clone()).execute().await.map_err(|e| {
                flow_like_types::anyhow!("Failed to open database: {}, {:?}", base_path, e)
            })?;
            meta.flush(db).await.map_err(|e| {
                flow_like_types::anyhow!("Failed to flush run: {}, {:?}", base_path, e)
            })?;
        }

        let _res = flow_like_state.lock().await.remove_and_cancel_run(&run_id);

        Ok(meta)
    }
}

pub struct EventBus {
    sender: mpsc::Sender<EventBusEvent>,
    app_handle: AppHandle,
}

impl EventBus {
    pub fn new(app_handle: AppHandle) -> (Arc<Self>, mpsc::Receiver<EventBusEvent>) {
        let (sender, receiver) = mpsc::channel(MAX_QUEUE_SIZE);
        let new_self = Self { sender, app_handle };
        (Arc::new(new_self), receiver)
    }

    pub fn push_event_with_token(
        &self,
        payload: Option<Value>,
        app_id: String,
        event_id: String,
        offline: bool,
        token: Option<String>,
        callback: Option<Arc<BufferedInterComHandler>>,
    ) -> Result<(), String> {
        if !offline && token.is_none() {
            return Err("No token registered, cannot send online events".to_string());
        }

        let event = EventBusEvent {
            payload,
            app_id,
            event_id,
            token,
            offline,
            callback,
        };

        self.sender
            .try_send(event)
            .map_err(|e| format!("Failed to send event: {}", e))
    }
}

fn event_bus_dir() -> PathBuf {
    if let Some(dir) = dirs_next::data_dir() {
        dir.join("flow-like").join("event-bus")
    } else if let Some(dir) = dirs_next::cache_dir() {
        dir.join("flow-like").join("event-bus")
    } else {
        PathBuf::from("flow-like").join("event-bus")
    }
}
