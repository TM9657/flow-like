use flow_like::{
    flow::{board::Board, execution::InternalRun},
    state::FlowLikeState,
    utils::http::HTTPClient,
};
use std::{collections::HashMap, sync::Arc};
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

use crate::{profile::UserProfile, settings::Settings};

pub struct TauriFlowLikeState(pub Arc<Mutex<FlowLikeState>>);
impl TauriFlowLikeState {
    pub async fn construct(app_handle: &AppHandle) -> anyhow::Result<Arc<Mutex<FlowLikeState>>> {
        app_handle
            .try_state::<TauriFlowLikeState>()
            .map(|state| state.0.clone())
            .ok_or_else(|| anyhow::anyhow!("Flow-Like State not found"))
    }

    pub async fn http_client(app_handle: &AppHandle) -> anyhow::Result<Arc<HTTPClient>> {
        let flow_like_state = TauriFlowLikeState::construct(app_handle).await?;
        let http_client = flow_like_state.lock().await.http_client.clone();
        Ok(http_client)
    }

    pub async fn board_registry(
        app_handle: &AppHandle,
    ) -> anyhow::Result<Arc<Mutex<HashMap<String, Arc<Mutex<Board>>>>>> {
        let flow_like_state = TauriFlowLikeState::construct(app_handle).await?;
        let flow_like_state = flow_like_state.lock().await.board_registry().clone();
        Ok(flow_like_state)
    }

    pub async fn get_board_and_state(
        app_handle: &AppHandle,
        board_id: &str,
    ) -> anyhow::Result<(Arc<Mutex<Board>>, Arc<Mutex<FlowLikeState>>)> {
        let flow_like_state = TauriFlowLikeState::construct(app_handle).await?;
        let board_registry = flow_like_state.lock().await.board_registry();
        let board = board_registry
            .lock()
            .await
            .get(board_id)
            .ok_or(anyhow::anyhow!("Board not found"))?
            .clone();
        Ok((board, flow_like_state))
    }

    pub async fn get_run_and_state(
        app_handle: &AppHandle,
        run_id: &str,
    ) -> anyhow::Result<(Arc<Mutex<InternalRun>>, Arc<Mutex<FlowLikeState>>)> {
        let flow_like_state = TauriFlowLikeState::construct(app_handle).await?;
        let run_registry = flow_like_state.lock().await.board_run_registry();
        let run = run_registry
            .lock()
            .await
            .get(run_id)
            .ok_or(anyhow::anyhow!("Run not found"))?
            .clone();
        Ok((run, flow_like_state))
    }
}

pub struct TauriSettingsState(pub Arc<Mutex<Settings>>);
impl TauriSettingsState {
    pub async fn construct(app_handle: &AppHandle) -> anyhow::Result<Arc<Mutex<Settings>>> {
        app_handle
            .try_state::<TauriSettingsState>()
            .map(|state| state.0.clone())
            .ok_or_else(|| anyhow::anyhow!("Settings State not found"))
    }

    pub async fn current_profile(app_handle: &AppHandle) -> anyhow::Result<UserProfile> {
        let settings = TauriSettingsState::construct(app_handle).await?;
        let settings = settings.lock().await;
        let current_profile = settings.get_current_profile()?;
        Ok(current_profile)
    }
}
