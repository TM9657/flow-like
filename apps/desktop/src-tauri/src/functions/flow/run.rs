use flow_like::flow::execution::log::LogMessage;
use flow_like::flow::execution::{InternalRun, Run, RunStatus, trace::Trace};
use flow_like::flow::execution::{LogLevel, LogMeta, RunPayload};
use flow_like::flow_like_storage::async_duckdb::ClientBuilder;
use flow_like::flow_like_storage::async_duckdb::duckdb::params;
use flow_like::flow_like_storage::lancedb::arrow::SendableRecordBatchStreamExt;
use flow_like::flow_like_storage::lancedb::query::{ExecutableQuery, QueryBase};
use flow_like::flow_like_storage::{Path, arrow_schema, serde_arrow};
use flow_like_types::intercom::{BufferedInterComHandler, InterComEvent};
use flow_like_types::sync::Mutex;
use flow_like_types::tokio;
use flow_like_types::tokio::task::spawn_blocking;
use futures::{StreamExt, TryStreamExt};
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
    payload: RunPayload,
    events: tauri::ipc::Channel<Vec<InterComEvent>>,
) -> Result<Option<LogMeta>, TauriFunctionError> {
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
    let meta = internal_run.execute(flow_like_state.clone()).await;
    if let Err(err) = buffered_sender.flush().await {
        println!("Error flushing buffered sender: {}", err);
    }

    if let Some(meta) = &meta {
        let db = {
            let guard = flow_like_state.lock().await;
            let guard = guard.config.read().await;
            let db = guard.callbacks.build_logs_database.clone();
            db
        };
        let db_fn = db
            .as_ref()
            .ok_or_else(|| flow_like_types::anyhow!("No log database configured"))?;
        let base_path = Path::from("runs").child(app_id).child(board_id);
        let db = db_fn(base_path.clone())
            .execute()
            .await
            .map_err(|_| flow_like_types::anyhow!("Failed to open database: {}", base_path))?;
        meta.flush(db)
            .await
            .map_err(|_| flow_like_types::anyhow!("Failed to flush run: {}", base_path))?;
    }

    flow_like_state
        .lock()
        .await
        .register_run(&run_id, Arc::new(Mutex::new(internal_run)));

    Ok(meta)
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
    node_id: Option<String>,
    from: Option<u64>,
    to: Option<u64>,
    status: Option<LogLevel>,
    limit: Option<usize>,
    offset: Option<usize>,
    last_meta: Option<LogMeta>,
) -> Result<Vec<LogMeta>, TauriFunctionError> {
    let limit = limit.unwrap_or(100);
    let offset = offset.unwrap_or(0);
    let state = TauriFlowLikeState::construct(&app_handle).await?;
    let db = {
        let guard = state.lock().await;
        let guard = guard.config.read().await;
        let db = guard.callbacks.build_logs_database.clone();
        db
    };
    let db_fn = db
        .as_ref()
        .ok_or_else(|| flow_like_types::anyhow!("No log database configured"))?;
    let base_path = Path::from("runs").child(app_id).child(board_id);
    let db = db_fn(base_path.clone())
        .execute()
        .await
        .map_err(|_| flow_like_types::anyhow!("Failed to open database: {}", base_path))?;

    let db = db
        .open_table("runs")
        .execute()
        .await
        .map_err(|_| flow_like_types::anyhow!("Failed to open table: runs"))?;

    let mut query_string = String::from("");

    if let Some(node_id) = node_id {
        query_string.push_str(&format!("node_id = '{}'", node_id));
    }

    if let Some(from) = from {
        if !query_string.is_empty() {
            query_string.push_str(" AND ");
        }
        query_string.push_str(&format!("start >= {}", from));
    }

    if let Some(to) = to {
        if !query_string.is_empty() {
            query_string.push_str(" AND ");
        }
        query_string.push_str(&format!("start <= {}", to));
    }

    if let Some(status) = status {
        if !query_string.is_empty() {
            query_string.push_str(" AND ");
        }

        let status = status.to_u8();
        if status == 0 {
            query_string.push_str("log_level <= 1");
        } else {
            query_string.push_str(&format!("log_level = {}", status));
        }
    }

    let mut query = db.query();

    if !query_string.is_empty() {
        query = query.only_if(&query_string);
    }

    let runs = query
        .limit(limit)
        .offset(offset)
        .execute()
        .await
        .map_err(|_| flow_like_types::anyhow!("Failed to execute query"))?;
    let results = runs
        .try_collect::<Vec<_>>()
        .await
        .map_err(|_| flow_like_types::anyhow!("Failed to collect results"))?;
    let mut log_meta = Vec::with_capacity(results.len() * 10);
    for result in results {
        let result = serde_arrow::from_record_batch::<Vec<LogMeta>>(&result).unwrap_or(vec![]);
        log_meta.extend(result);
    }
    Ok(log_meta)

    // let mut stream = db
    //     .query()
    //     .execute()
    //     .await
    //     .map_err(|_| flow_like_types::anyhow!("Failed to execute query on table: runs"))?;

    // let client = ClientBuilder::new().open().await?;
    // let out = client.conn(move |conn| {
    //     conn.execute_batch("
    //         CREATE TABLE runs (
    //             start     UBIGINT,
    //             run_id    VARCHAR,
    //             log_level UTINYINT,
    //             node_id   VARCHAR
    //         )
    //     ")?;
    //     let mut appender = conn.appender("runs").unwrap();
    //     let (tx, rx) = std::sync::mpsc::channel();
    //     tokio::spawn(async move {
    //         while let Some(item_res) = stream.next().await {
    //             if let Ok(item) = item_res {
    //                 let _ = tx.send(item);
    //             }
    //         }
    //     });

    //     for meta in rx {
    //         appender.append_record_batch(meta)?;
    //     }
    //     appender.flush()?;
    //     let mut stmt = conn.prepare("SELECT start, run_id, log_level, node_id FROM runs ORDER BY start DESC LIMIT ?, ?")?;
    //     let mut rows = stmt.query(params![offset as i64, limit as i64])?;
    //     let mut out = Vec::new();
    //     while let Some(r) = rows.next()? {
    //         let start: u64 = r.get(0)?;
    //         println!("Row: {:?}", start);
    //     }
    //     Ok(out)
    // }).await?;

    // return Ok(out);
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
    let logs = state
        .lock()
        .await
        .query_run(&log_meta, &query, limit, offset)
        .await?;
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
