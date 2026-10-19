use flow_like::flow_like_storage::contracts::column_metadata::ColumnReader;
use flow_like_catalog_ml::inspection::forecasting::studio::saved::{
    self, CreateStudioForecast, SavedStudioForecast, SavedStudioForecastSummary,
};
use tauri::AppHandle;

use super::graph::graph_connection;
use crate::functions::{TauriFunctionError, flow::storage::current_user_sub};

#[tauri::command(async)]
pub async fn studio_forecast_list(
    app_handle: AppHandle,
    app_id: String,
    user_scoped: Option<bool>,
    offset: Option<usize>,
    limit: Option<usize>,
) -> Result<Vec<SavedStudioForecastSummary>, TauriFunctionError> {
    let limit = limit.unwrap_or(100);
    if !(1..=100).contains(&limit) {
        return Err(TauriFunctionError::new("limit must be between 1 and 100"));
    }
    if offset.is_some_and(|offset| offset > 1000) {
        return Err(TauriFunctionError::new("offset must not exceed 1000"));
    }
    let connection = graph_connection(&app_handle, &app_id, user_scoped.unwrap_or(false)).await?;
    Ok(saved::list(
        &connection,
        &app_id,
        &ColumnReader::Unrestricted,
        offset.unwrap_or(0),
        limit,
    )
    .await?)
}

#[tauri::command(async)]
pub async fn studio_forecast_get(
    app_handle: AppHandle,
    app_id: String,
    forecast_id: String,
    user_scoped: Option<bool>,
) -> Result<SavedStudioForecast, TauriFunctionError> {
    let connection = graph_connection(&app_handle, &app_id, user_scoped.unwrap_or(false)).await?;
    saved::get(
        &connection,
        &app_id,
        &forecast_id,
        &ColumnReader::Unrestricted,
    )
    .await?
    .ok_or_else(|| TauriFunctionError::new("Saved forecast not found"))
}

#[tauri::command(async)]
pub async fn studio_forecast_create(
    app_handle: AppHandle,
    app_id: String,
    payload: CreateStudioForecast,
    user_scoped: Option<bool>,
) -> Result<SavedStudioForecast, TauriFunctionError> {
    let connection = graph_connection(&app_handle, &app_id, user_scoped.unwrap_or(false)).await?;
    let created_by = current_user_sub(&app_handle).await?;
    Ok(saved::create(
        &connection,
        &app_id,
        &created_by,
        &ColumnReader::Unrestricted,
        payload,
    )
    .await?)
}

#[tauri::command(async)]
pub async fn studio_forecast_delete(
    app_handle: AppHandle,
    app_id: String,
    forecast_id: String,
    user_scoped: Option<bool>,
) -> Result<(), TauriFunctionError> {
    let connection = graph_connection(&app_handle, &app_id, user_scoped.unwrap_or(false)).await?;
    if !saved::delete(
        &connection,
        &app_id,
        &forecast_id,
        &ColumnReader::Unrestricted,
    )
    .await?
    {
        return Err(TauriFunctionError::new("Saved forecast not found"));
    }
    Ok(())
}
