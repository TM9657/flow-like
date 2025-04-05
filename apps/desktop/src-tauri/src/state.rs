use flow_like::{
    flow::{board::Board, execution::InternalRun}, flow_like_storage::object_store::ObjectStore, state::FlowLikeState, utils::http::HTTPClient
};
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

use crate::{profile::UserProfile, settings::Settings};

pub struct TauriFlowLikeState(pub Arc<Mutex<FlowLikeState>>);
impl TauriFlowLikeState {
    #[inline]
    pub async fn construct(app_handle: &AppHandle) -> anyhow::Result<Arc<Mutex<FlowLikeState>>> {
        app_handle
            .try_state::<TauriFlowLikeState>()
            .map(|state| state.0.clone())
            .ok_or_else(|| anyhow::anyhow!("Flow-Like State not found"))
    }

    #[inline]
    pub async fn http_client(app_handle: &AppHandle) -> anyhow::Result<Arc<HTTPClient>> {
        let flow_like_state = TauriFlowLikeState::construct(app_handle).await?;
        let http_client = flow_like_state.lock().await.http_client.clone();
        Ok(http_client)
    }

    #[inline]
    pub async fn get_board_and_state(
        app_handle: &AppHandle,
        board_id: &str,
    ) -> anyhow::Result<(Arc<Mutex<Board>>, Arc<Mutex<FlowLikeState>>)> {
        let flow_like_state = TauriFlowLikeState::construct(app_handle).await?;
        let board = flow_like_state.lock().await.get_board(board_id)?;
        Ok((board, flow_like_state))
    }

    #[inline]
    pub async fn get_run_and_state(
        app_handle: &AppHandle,
        run_id: &str,
    ) -> anyhow::Result<(Arc<Mutex<InternalRun>>, Arc<Mutex<FlowLikeState>>)> {
        let flow_like_state = TauriFlowLikeState::construct(app_handle).await?;
        let run = flow_like_state.lock().await.get_run(run_id)?;
        Ok((run, flow_like_state))
    }

    #[inline]
    pub async fn get_project_store(app_handle: &AppHandle) -> anyhow::Result<Arc<dyn ObjectStore>> {
        let flow_like_state = TauriFlowLikeState::construct(app_handle).await?;
        let project_store = flow_like_state
            .lock()
            .await
            .config
            .read()
            .await
            .stores
            .project_store
            .clone()
            .ok_or(anyhow::anyhow!("Project store not found"))?
            .as_generic();
        Ok(project_store)
    }
}

pub struct TauriSettingsState(pub Arc<Mutex<Settings>>);
impl TauriSettingsState {
    #[inline]
    pub async fn construct(app_handle: &AppHandle) -> anyhow::Result<Arc<Mutex<Settings>>> {
        app_handle
            .try_state::<TauriSettingsState>()
            .map(|state| state.0.clone())
            .ok_or_else(|| anyhow::anyhow!("Settings State not found"))
    }

    #[inline]
    pub async fn current_profile(app_handle: &AppHandle) -> anyhow::Result<UserProfile> {
        let settings = TauriSettingsState::construct(app_handle).await?;
        let settings = settings.lock().await;
        let current_profile = settings.get_current_profile()?;
        Ok(current_profile)
    }
}
