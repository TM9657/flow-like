use crate::{functions::TauriFunctionError, state::TauriFlowLikeState};
use flow_like::app::App;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::AppHandle;

/// Simple route mapping entry for frontend consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteMapping {
    pub path: String,
    pub event_id: String,
}

/// Get all route mappings for an app
#[tauri::command(async)]
pub async fn get_app_routes(
    handler: AppHandle,
    app_id: String,
) -> Result<Vec<RouteMapping>, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
    let app = App::load(app_id.clone(), flow_like_state)
        .await
        .map_err(|e| TauriFunctionError::new(&format!("Failed to load app: {}", e)))?;

    let mut routes: Vec<RouteMapping> = app
        .route_mappings
        .into_iter()
        .map(|(path, event_id)| RouteMapping { path, event_id })
        .collect();

    // Sort by path for consistency
    routes.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(routes)
}

/// Get the route mapping for a specific path
#[tauri::command(async)]
pub async fn get_app_route_by_path(
    handler: AppHandle,
    app_id: String,
    path: String,
) -> Result<Option<RouteMapping>, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
    let app = App::load(app_id.clone(), flow_like_state)
        .await
        .map_err(|e| TauriFunctionError::new(&format!("Failed to load app: {}", e)))?;

    Ok(app.route_mappings.get(&path).map(|event_id| RouteMapping {
        path: path.clone(),
        event_id: event_id.clone(),
    }))
}

/// Get the default route (path = "/")
#[tauri::command(async)]
pub async fn get_default_app_route(
    handler: AppHandle,
    app_id: String,
) -> Result<Option<RouteMapping>, TauriFunctionError> {
    get_app_route_by_path(handler, app_id, "/".to_string()).await
}

/// Set a route mapping (path -> event_id)
#[tauri::command(async)]
pub async fn set_app_route(
    handler: AppHandle,
    app_id: String,
    path: String,
    event_id: String,
) -> Result<RouteMapping, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
    let mut app = App::load(app_id.clone(), flow_like_state)
        .await
        .map_err(|e| TauriFunctionError::new(&format!("Failed to load app: {}", e)))?;

    if let Some(existing_event_id) = app.route_mappings.get(&path)
        && existing_event_id != &event_id
    {
        return Err(TauriFunctionError::new(&format!(
            "Route path already in use: {}",
            path
        )));
    }

    app.route_mappings.insert(path.clone(), event_id.clone());
    app.save()
        .await
        .map_err(|e| TauriFunctionError::new(&format!("Failed to save app: {}", e)))?;

    Ok(RouteMapping { path, event_id })
}

/// Delete a route mapping by path
#[tauri::command(async)]
pub async fn delete_app_route_by_path(
    handler: AppHandle,
    app_id: String,
    path: String,
) -> Result<(), TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
    let mut app = App::load(app_id.clone(), flow_like_state)
        .await
        .map_err(|e| TauriFunctionError::new(&format!("Failed to load app: {}", e)))?;

    if app.route_mappings.remove(&path).is_none() {
        return Err(TauriFunctionError::new("Route not found"));
    }

    app.save()
        .await
        .map_err(|e| TauriFunctionError::new(&format!("Failed to save app: {}", e)))?;

    Ok(())
}

/// Delete a route mapping by event ID
#[tauri::command(async)]
pub async fn delete_app_route_by_event(
    handler: AppHandle,
    app_id: String,
    event_id: String,
) -> Result<(), TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
    let mut app = App::load(app_id.clone(), flow_like_state)
        .await
        .map_err(|e| TauriFunctionError::new(&format!("Failed to load app: {}", e)))?;

    let path_to_remove: Option<String> = app
        .route_mappings
        .iter()
        .find(|(_, eid)| *eid == &event_id)
        .map(|(path, _)| path.clone());

    if let Some(path) = path_to_remove {
        app.route_mappings.remove(&path);
        app.save()
            .await
            .map_err(|e| TauriFunctionError::new(&format!("Failed to save app: {}", e)))?;
    }

    Ok(())
}

/// Update route mappings in bulk (replaces all existing mappings)
#[tauri::command(async)]
pub async fn set_app_routes(
    handler: AppHandle,
    app_id: String,
    routes: HashMap<String, String>,
) -> Result<Vec<RouteMapping>, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
    let mut app = App::load(app_id.clone(), flow_like_state)
        .await
        .map_err(|e| TauriFunctionError::new(&format!("Failed to load app: {}", e)))?;

    app.route_mappings = routes;
    app.save()
        .await
        .map_err(|e| TauriFunctionError::new(&format!("Failed to save app: {}", e)))?;

    let result: Vec<RouteMapping> = app
        .route_mappings
        .into_iter()
        .map(|(path, event_id)| RouteMapping { path, event_id })
        .collect();

    Ok(result)
}
