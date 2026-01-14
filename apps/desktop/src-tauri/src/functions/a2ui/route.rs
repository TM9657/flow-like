use crate::{
    functions::TauriFunctionError,
    state::TauriFlowLikeState,
};
use flow_like::flow_like_storage::object_store::path::Path;
use flow_like_types::create_id;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppRoute {
    pub id: String,
    pub app_id: String,
    pub path: String,
    pub target_type: RouteTargetType,
    pub page_id: Option<String>,
    pub board_id: Option<String>,
    pub page_version: Option<String>,
    pub event_id: Option<String>,
    pub priority: i32,
    pub label: Option<String>,
    pub icon: Option<String>,
    #[serde(default = "default_timestamp")]
    pub created_at: String,
    #[serde(default = "default_timestamp")]
    pub updated_at: String,
}

fn default_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AppRoutes {
    routes: Vec<AppRoute>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RouteTargetType {
    Page,
    Event,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAppRoute {
    pub path: String,
    pub target_type: RouteTargetType,
    pub page_id: Option<String>,
    pub board_id: Option<String>,
    pub page_version: Option<String>,
    pub event_id: Option<String>,
    pub priority: Option<i32>,
    pub label: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAppRoute {
    pub path: Option<String>,
    pub target_type: Option<RouteTargetType>,
    pub page_id: Option<String>,
    pub board_id: Option<String>,
    pub page_version: Option<String>,
    pub event_id: Option<String>,
    pub priority: Option<i32>,
    pub label: Option<String>,
    pub icon: Option<String>,
}

/// Get the storage path for routes file
fn get_routes_path(app_id: &str) -> Path {
    Path::from("apps").child(app_id).child("routes.json")
}

/// Load routes from storage
async fn load_routes(handler: &AppHandle, app_id: &str) -> Result<AppRoutes, TauriFunctionError> {
    let store = TauriFlowLikeState::get_project_meta_store(handler).await?;
    let path = get_routes_path(app_id);

    match store.get(&path).await {
        Ok(data) => {
            let bytes = data.bytes().await
                .map_err(|e| TauriFunctionError::new(&format!("Failed to read routes: {}", e)))?;
            let routes: AppRoutes = serde_json::from_slice(&bytes)
                .map_err(|e| TauriFunctionError::new(&format!("Failed to parse routes: {}", e)))?;
            Ok(routes)
        }
        Err(flow_like::flow_like_storage::object_store::Error::NotFound { .. }) => {
            Ok(AppRoutes::default())
        }
        Err(e) => Err(TauriFunctionError::new(&format!("Failed to load routes: {}", e))),
    }
}

/// Save routes to storage
async fn save_routes(handler: &AppHandle, app_id: &str, routes: &AppRoutes) -> Result<(), TauriFunctionError> {
    let store = TauriFlowLikeState::get_project_meta_store(handler).await?;
    let path = get_routes_path(app_id);
    let data = serde_json::to_vec_pretty(routes)
        .map_err(|e| TauriFunctionError::new(&format!("Failed to serialize routes: {}", e)))?;

    store.put(&path, data.into()).await
        .map_err(|e| TauriFunctionError::new(&format!("Failed to save routes: {}", e)))?;

    Ok(())
}

#[tauri::command(async)]
pub async fn get_app_routes(
    handler: AppHandle,
    app_id: String,
) -> Result<Vec<AppRoute>, TauriFunctionError> {
    let app_routes = load_routes(&handler, &app_id).await?;
    let mut routes = app_routes.routes;
    routes.sort_by_key(|r| r.priority);
    Ok(routes)
}

#[tauri::command(async)]
pub async fn get_app_route_by_path(
    handler: AppHandle,
    app_id: String,
    path: String,
) -> Result<Option<AppRoute>, TauriFunctionError> {
    let app_routes = load_routes(&handler, &app_id).await?;
    Ok(app_routes.routes.into_iter().find(|r| r.path == path))
}

#[tauri::command(async)]
pub async fn get_default_app_route(
    handler: AppHandle,
    app_id: String,
) -> Result<Option<AppRoute>, TauriFunctionError> {
    let app_routes = load_routes(&handler, &app_id).await?;
    // The "/" path is always the default route
    Ok(app_routes.routes.into_iter().find(|r| r.path == "/"))
}

#[tauri::command(async)]
pub async fn create_app_route(
    handler: AppHandle,
    app_id: String,
    route: CreateAppRoute,
) -> Result<AppRoute, TauriFunctionError> {
    let mut app_routes = load_routes(&handler, &app_id).await?;

    // Check for duplicate path
    if app_routes.routes.iter().any(|r| r.path == route.path) {
        return Err(TauriFunctionError::new("Route with this path already exists"));
    }

    let now = chrono::Utc::now().to_rfc3339();

    let new_route = AppRoute {
        id: create_id(),
        app_id: app_id.clone(),
        path: route.path,
        target_type: route.target_type,
        page_id: route.page_id,
        board_id: route.board_id,
        page_version: route.page_version,
        event_id: route.event_id,
        priority: route.priority.unwrap_or(0),
        label: route.label,
        icon: route.icon,
        created_at: now.clone(),
        updated_at: now,
    };

    app_routes.routes.push(new_route.clone());
    save_routes(&handler, &app_id, &app_routes).await?;

    Ok(new_route)
}

#[tauri::command(async)]
pub async fn update_app_route(
    handler: AppHandle,
    app_id: String,
    route_id: String,
    update: UpdateAppRoute,
) -> Result<AppRoute, TauriFunctionError> {
    let mut app_routes = load_routes(&handler, &app_id).await?;

    // Find the route index
    let route_idx = app_routes.routes.iter().position(|r| r.id == route_id)
        .ok_or_else(|| TauriFunctionError::new("Route not found"))?;

    // Check for duplicate path (excluding current route) before any mutable borrows
    if let Some(ref path) = update.path {
        if app_routes.routes.iter().any(|r| r.path == *path && r.id != route_id) {
            return Err(TauriFunctionError::new("Route with this path already exists"));
        }
    }

    // Now update the route
    let route = &mut app_routes.routes[route_idx];

    if let Some(path) = update.path {
        route.path = path;
    }
    if let Some(target_type) = update.target_type {
        route.target_type = target_type;
    }
    if update.page_id.is_some() {
        route.page_id = update.page_id;
    }
    if update.board_id.is_some() {
        route.board_id = update.board_id;
    }
    if update.page_version.is_some() {
        route.page_version = update.page_version;
    }
    if update.event_id.is_some() {
        route.event_id = update.event_id;
    }
    if let Some(priority) = update.priority {
        route.priority = priority;
    }
    if update.label.is_some() {
        route.label = update.label;
    }
    if update.icon.is_some() {
        route.icon = update.icon;
    }
    route.updated_at = chrono::Utc::now().to_rfc3339();

    let updated_route = route.clone();
    save_routes(&handler, &app_id, &app_routes).await?;

    Ok(updated_route)
}

#[tauri::command(async)]
pub async fn delete_app_route(
    handler: AppHandle,
    app_id: String,
    route_id: String,
) -> Result<(), TauriFunctionError> {
    let mut app_routes = load_routes(&handler, &app_id).await?;

    let initial_len = app_routes.routes.len();
    app_routes.routes.retain(|r| r.id != route_id);

    if app_routes.routes.len() == initial_len {
        return Err(TauriFunctionError::new("Route not found"));
    }

    save_routes(&handler, &app_id, &app_routes).await?;
    Ok(())
}
