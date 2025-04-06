use flow_like::flow::execution::{trace::Trace, InternalRun, Run, RunStatus};
use flow_like_types::intercom::{BufferedInterComHandler, InterComEvent};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use crate::{
    functions::TauriFunctionError,
    state::{TauriFlowLikeState, TauriSettingsState},
};

#[tauri::command(async)]
pub async fn execute_board(
    app_handle: AppHandle,
    app_id: String,
    board_id: String,
    start_ids: Vec<String>,
    events: tauri::ipc::Channel<Vec<InterComEvent>>,
) -> Result<String, TauriFunctionError> {
    let (board, flow_like_state) =
        TauriFlowLikeState::get_board_and_state(&app_handle, &board_id).await?;
    let board = Arc::new(board.lock().await.clone());
    let profile = TauriSettingsState::current_profile(&app_handle).await?;

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
    ));

    let mut internal_run = InternalRun::new(
        board,
        &flow_like_state,
        &profile.hub_profile,
        start_ids,
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
pub async fn get_run(
    app_handle: AppHandle,
    app_id: String,
    run_id: String,
) -> Result<Run, TauriFunctionError> {
    let (run, _) = TauriFlowLikeState::get_run_and_state(&app_handle, &run_id).await?;
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
