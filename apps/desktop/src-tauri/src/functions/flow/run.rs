use flow_like::flow::execution::{trace::Trace, InternalRun, LogLevel, Run, RunStatus};
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::Mutex;

use crate::{
    functions::TauriFunctionError,
    state::{TauriFlowLikeState, TauriSettingsState},
};

#[tauri::command(async)]
pub async fn create_run(
    app_handle: AppHandle,
    board_id: String,
    start_ids: Vec<String>,
    log_level: LogLevel,
) -> Result<String, TauriFunctionError> {
    let (board, flow_like_state) =
        TauriFlowLikeState::get_board_and_state(&app_handle, &board_id).await?;
    let board = board.lock().await;
    let profile = TauriSettingsState::current_profile(&app_handle).await?;

    let internal_run = InternalRun::new(
        &board,
        &flow_like_state,
        &profile.hub_profile,
        start_ids,
        log_level,
    )
    .await?;
    let run_id = internal_run.run.lock().await.id.clone();
    flow_like_state
        .lock()
        .await
        .board_run_registry
        .lock()
        .await
        .insert(run_id.clone(), Arc::new(Mutex::new(internal_run)));
    Ok(run_id)
}

#[tauri::command(async)]
pub async fn execute_run(app_handle: AppHandle, id: String) -> Result<(), TauriFunctionError> {
    let (run, flow_like_state) = TauriFlowLikeState::get_run_and_state(&app_handle, &id).await?;
    let mut run = run.lock().await;
    run.execute(flow_like_state).await;
    Ok(())
}

#[tauri::command(async)]
pub async fn debug_step_run(app_handle: AppHandle, id: String) -> Result<(), TauriFunctionError> {
    let (run, flow_like_state) = TauriFlowLikeState::get_run_and_state(&app_handle, &id).await?;
    let mut run = run.lock().await;
    run.debug_step(flow_like_state).await;
    Ok(())
}

#[tauri::command(async)]
pub async fn get_run_status(
    app_handle: AppHandle,
    id: String,
) -> Result<RunStatus, TauriFunctionError> {
    let (run, _) = TauriFlowLikeState::get_run_and_state(&app_handle, &id).await?;
    let run = run.lock().await;

    let status = run.get_status().await;
    Ok(status)
}

#[tauri::command(async)]
pub async fn get_run(app_handle: AppHandle, id: String) -> Result<Run, TauriFunctionError> {
    let (run, _) = TauriFlowLikeState::get_run_and_state(&app_handle, &id).await?;
    let run = run.lock().await;

    let run = run.get_run().await;
    Ok(run)
}

#[tauri::command(async)]
pub async fn get_run_traces(
    app_handle: AppHandle,
    id: String,
) -> Result<Vec<Trace>, TauriFunctionError> {
    let (run, _) = TauriFlowLikeState::get_run_and_state(&app_handle, &id).await?;
    let run = run.lock().await;
    let traces = run.get_traces().await;
    Ok(traces)
}

#[tauri::command(async)]
pub async fn finalize_run(app_state: AppHandle, id: String) -> Result<(), TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_state).await?;
    let flow_like_state = flow_like_state.lock().await;
    let mut run_state = flow_like_state.board_run_registry.lock().await;
    run_state.remove(&id);
    Ok(())
}
