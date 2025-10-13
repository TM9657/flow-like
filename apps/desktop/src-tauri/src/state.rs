use flow_like::{
    flow_like_storage::object_store::ObjectStore, state::FlowLikeState, utils::http::HTTPClient,
};
use flow_like_types::sync::Mutex;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

use crate::{event_bus::EventBus, profile::UserProfile, settings::Settings};

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
    pub async fn get_project_storage_store(
        app_handle: &AppHandle,
    ) -> anyhow::Result<Arc<dyn ObjectStore>> {
        let flow_like_state = TauriFlowLikeState::construct(app_handle).await?;
        let project_store = flow_like_state
            .lock()
            .await
            .config
            .read()
            .await
            .stores
            .app_storage_store
            .clone()
            .ok_or(anyhow::anyhow!("Project store not found"))?
            .as_generic();
        Ok(project_store)
    }

    #[inline]
    pub async fn get_project_meta_store(
        app_handle: &AppHandle,
    ) -> anyhow::Result<Arc<dyn ObjectStore>> {
        let flow_like_state = TauriFlowLikeState::construct(app_handle).await?;
        let project_store = flow_like_state
            .lock()
            .await
            .config
            .read()
            .await
            .stores
            .app_meta_store
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

pub struct TauriEventBusState(pub Arc<EventBus>);
impl TauriEventBusState {
    #[inline]
    pub fn construct(app_handle: &AppHandle) -> anyhow::Result<Arc<EventBus>> {
        app_handle
            .try_state::<TauriEventBusState>()
            .map(|state| state.0.clone())
            .ok_or_else(|| anyhow::anyhow!("EventBus State not found"))
    }
}

pub struct TauriEventSinkManagerState(pub Arc<Mutex<crate::event_sink::EventSinkManager>>);
impl TauriEventSinkManagerState {
    #[inline]
    pub async fn construct(app_handle: &AppHandle) -> anyhow::Result<Arc<Mutex<crate::event_sink::EventSinkManager>>> {
        app_handle
            .try_state::<TauriEventSinkManagerState>()
            .map(|state| state.0.clone())
            .ok_or_else(|| anyhow::anyhow!("EventSinkManager State not found"))
    }
}
