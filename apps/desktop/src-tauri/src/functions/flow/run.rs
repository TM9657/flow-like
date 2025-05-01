use flow_like::flow::execution::log::LogMessage;
use flow_like::flow::execution::{LogMeta, RunPayload};
use flow_like::flow::execution::{InternalRun, Run, RunStatus, trace::Trace};
use flow_like_types::intercom::{BufferedInterComHandler, InterComEvent};
use flow_like_types::sync::Mutex;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

use crate::{
    functions::TauriFunctionError,
    state::{TauriFlowLikeState, TauriSettingsState},
};

#[tauri::command(async)]
pub async fn execute_board(
    app_handle: AppHandle,
    app_id: String,
    board_id: String,
    payload: Vec<RunPayload>,
    events: tauri::ipc::Channel<Vec<InterComEvent>>,
) -> Result<String, TauriFunctionError> {
    let (board, flow_like_state) =
        TauriFlowLikeState::get_board_and_state(&app_handle, &board_id).await?;
    let board = Arc::new(board.lock().await.clone());
    let profile = TauriSettingsState::current_profile(&app_handle).await?;

    println!("Executing board: {:?}", payload);

    let buffered_sender = Arc::new(BufferedInterComHandler::new(
        Arc::new(move |event| {
            let events_cb = events.clone();
            let app_handle = app_handle.clone();
            Box::pin({
                async move {
                    if let Err(err) = events_cb.send(event.clone()) {
                        println!("Error emitting event: {}", err);
                    }

                    let first_event = event.first();
                    if let Some(first_event) = first_event {
                        if let Err(err) = app_handle.emit(&first_event.event_type, event.clone()) {
                            println!("Error emitting event: {}", err);
                        }
                    }

                    Ok(())
                }
            })
        }),
        Some(1),
        Some(100),
        Some(true),
    ));

    let mut internal_run = InternalRun::new(
        &app_id,
        board,
        &flow_like_state,
        &profile.hub_profile,
        payload,
        None,
        buffered_sender.into_callback(),
    )
    .await?;
    let run_id = internal_run.run.lock().await.id.clone();
    internal_run.execute(flow_like_state.clone()).await;
    if let Err(err) = buffered_sender.flush().await {
        println!("Error flushing buffered sender: {}", err);
    }

    flow_like_state
        .lock()
        .await
        .register_run(&run_id, Arc::new(Mutex::new(internal_run)));

    Ok(run_id)
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
pub async fn list_runs(
    app_handle: AppHandle,
    app_id: String,
    board_id: String,
) -> Result<Vec<LogMeta>, TauriFunctionError> {
    let state = TauriFlowLikeState::construct(&app_handle).await?;
    let runs = state.lock().await.list_runs(&app_id, &board_id).await?;
    Ok(runs)
}

#[tauri::command(async)]
pub async fn get_run_meta(
    app_handle: AppHandle,
    log_meta: LogMeta
) -> Result<LogMeta, TauriFunctionError> {
    let state = TauriFlowLikeState::construct(&app_handle).await?;
    let meta = state.lock().await.get_run_meta(&log_meta).await?;
    Ok(meta)
}
#[tauri::command(async)]
pub async fn query_run(
    app_handle: AppHandle,
    log_meta: LogMeta,
    query: String,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<LogMessage>, TauriFunctionError> {
    let state = TauriFlowLikeState::construct(&app_handle).await?;
    let logs = state.lock().await.query_run(&log_meta, &query, limit, offset).await?;
    Ok(logs)
}

#[tauri::command(async)]
pub async fn finalize_run(
    app_state: AppHandle,
    app_id: String,
    run_id: String,
) -> Result<(), TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_state).await?;
    flow_like_state
        .lock()
        .await
        .remove_run(&run_id)
        .ok_or(TauriFunctionError::new("Run not found"))?;
    Ok(())
}
