use crate::{functions::TauriFunctionError, state::TauriFlowLikeState};
use flow_like::app::App;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

/// Simple route mapping entry for frontend consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteMapping {
    pub path: String,
    pub event_id: String,
}

/// Get all route mappings for an app (derived from events with routes)
#[tauri::command(async)]
pub async fn get_app_routes(
    handler: AppHandle,
    app_id: String,
) -> Result<Vec<RouteMapping>, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
    let app = App::load(app_id.clone(), flow_like_state)
        .await
        .map_err(|e| TauriFunctionError::new(&format!("Failed to load app: {}", e)))?;

    let mut routes = Vec::new();
    for event_id in &app.events {
        if let Ok(event) = app.get_event(event_id, None).await {
            if let Some(route) = &event.route {
                routes.push(RouteMapping {
                    path: route.clone(),
                    event_id: event.id.clone(),
                });
            }
        }
    }

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

    for event_id in &app.events {
        if let Ok(event) = app.get_event(event_id, None).await {
            if event.route.as_deref() == Some(&path) {
                return Ok(Some(RouteMapping {
                    path,
                    event_id: event.id,
                }));
            }
        }
    }

    Ok(None)
}

/// Get the default route (event with is_default=true or route="/")
#[tauri::command(async)]
pub async fn get_default_app_route(
    handler: AppHandle,
    app_id: String,
) -> Result<Option<RouteMapping>, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
    let app = App::load(app_id.clone(), flow_like_state)
        .await
        .map_err(|e| TauriFunctionError::new(&format!("Failed to load app: {}", e)))?;

    // First try to find event with is_default=true
    for event_id in &app.events {
        if let Ok(event) = app.get_event(event_id, None).await {
            if event.is_default {
                if let Some(route) = &event.route {
                    return Ok(Some(RouteMapping {
                        path: route.clone(),
                        event_id: event.id,
                    }));
                }
            }
        }
    }

    // Fall back to route="/"
    get_app_route_by_path(handler, app_id, "/".to_string()).await
}

/// Set a route on an event
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

    // Check if another event already has this route
    for eid in &app.events {
        if eid != &event_id {
            if let Ok(e) = app.get_event(eid, None).await {
                if e.route.as_deref() == Some(&path) {
                    return Err(TauriFunctionError::new(&format!(
                        "Route path already in use by event {}: {}",
                        eid, path
                    )));
                }
            }
        }
    }

    // Update the event's route
    let mut event = app
        .get_event(&event_id, None)
        .await
        .map_err(|e| TauriFunctionError::new(&format!("Event not found: {}", e)))?;

    event.route = Some(path.clone());
    app.upsert_event(event, None, None)
        .await
        .map_err(|e| TauriFunctionError::new(&format!("Failed to save event: {}", e)))?;

    Ok(RouteMapping { path, event_id })
}

/// Delete a route mapping by path (clears route from the event)
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

    // Find and clear the route from the event
    for event_id in app.events.clone() {
        if let Ok(mut event) = app.get_event(&event_id, None).await {
            if event.route.as_deref() == Some(&path) {
                event.route = None;
                app.upsert_event(event, None, None).await.map_err(|e| {
                    TauriFunctionError::new(&format!("Failed to save event: {}", e))
                })?;
                return Ok(());
            }
        }
    }

    Err(TauriFunctionError::new("Route not found"))
}

/// Delete a route mapping by event ID (clears route from the event)
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

    let mut event = app
        .get_event(&event_id, None)
        .await
        .map_err(|e| TauriFunctionError::new(&format!("Event not found: {}", e)))?;

    if event.route.is_some() {
        event.route = None;
        app.upsert_event(event, None, None)
            .await
            .map_err(|e| TauriFunctionError::new(&format!("Failed to save event: {}", e)))?;
    }

    Ok(())
}

/// Update routes in bulk - sets routes on events
#[tauri::command(async)]
pub async fn set_app_routes(
    handler: AppHandle,
    app_id: String,
    routes: Vec<RouteMapping>,
) -> Result<Vec<RouteMapping>, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
    let mut app = App::load(app_id.clone(), flow_like_state)
        .await
        .map_err(|e| TauriFunctionError::new(&format!("Failed to load app: {}", e)))?;

    // First clear all existing routes
    for event_id in app.events.clone() {
        if let Ok(mut event) = app.get_event(&event_id, None).await {
            if event.route.is_some() {
                event.route = None;
                app.upsert_event(event, None, None).await.ok();
            }
        }
    }

    // Set new routes
    let mut result = Vec::new();
    for mapping in routes {
        if let Ok(mut event) = app.get_event(&mapping.event_id, None).await {
            event.route = Some(mapping.path.clone());
            if app.upsert_event(event, None, None).await.is_ok() {
                result.push(mapping);
            }
        }
    }

    Ok(result)
}
