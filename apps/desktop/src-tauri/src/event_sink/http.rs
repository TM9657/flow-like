use anyhow::Result;
use axum::{
    Router,
    body::Body,
    extract::{Path as AxumPath, State},
    http::{HeaderMap, Method, Request, StatusCode},
    response::IntoResponse,
    routing::any,
};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tower::ServiceBuilder;

use super::manager::DbConnection;
use super::{EventRegistration, EventSink};
use crate::state::TauriEventBusState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpSink {
    pub path: String,
    pub method: String,
    pub auth_token: Option<String>,
}

impl HttpSink {
    fn init_tables(db: &DbConnection) -> Result<()> {
        let conn = db.lock().unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS http_routes (
                event_id TEXT PRIMARY KEY,
                app_id TEXT NOT NULL,
                path TEXT NOT NULL,
                method TEXT NOT NULL,
                auth_token TEXT,
                created_at INTEGER NOT NULL,
                UNIQUE(app_id, path, method)
            )",
            [],
        )?;

        Ok(())
    }

    fn add_route(
        db: &DbConnection,
        registration: &EventRegistration,
        config: &HttpSink,
    ) -> Result<()> {
        let conn = db.lock().unwrap();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        conn.execute(
            "INSERT OR REPLACE INTO http_routes
             (event_id, app_id, path, method, auth_token, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                registration.event_id,
                registration.app_id,
                config.path,
                config.method,
                config.auth_token,
                now,
            ],
        )?;

        Ok(())
    }

    fn remove_route(db: &DbConnection, event_id: &str) -> Result<()> {
        let conn = db.lock().unwrap();
        conn.execute(
            "DELETE FROM http_routes WHERE event_id = ?1",
            params![event_id],
        )?;
        Ok(())
    }

    async fn health_check() -> impl IntoResponse {
        (StatusCode::OK, "OK")
    }

    async fn handle_request(
        State(state): State<Arc<HttpServerState>>,
        AxumPath((app_id, path)): AxumPath<(String, String)>,
        method: Method,
        headers: HeaderMap,
        _request: Request<Body>,
    ) -> impl IntoResponse {
        let method_str = method.as_str();
        let full_path = format!("/{}", path);

        let route_info = {
            let conn = state.db.lock().unwrap();
            let mut stmt = match conn.prepare(
                "SELECT event_id, auth_token FROM http_routes
                 WHERE app_id = ?1 AND path = ?2 AND method = ?3",
            ) {
                Ok(stmt) => stmt,
                Err(_) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
                }
            };

            stmt.query_row(params![app_id, full_path, method_str], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
            })
            .ok()
        };

        let Some((event_id, auth_token)) = route_info else {
            return (StatusCode::NOT_FOUND, "Route not found").into_response();
        };

        if let Some(required_token) = auth_token {
            let provided_token = headers
                .get("Authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "));

            if provided_token != Some(&required_token) {
                return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
            }
        }

        tracing::info!(
            "HTTP request matched: {} {} -> event {}",
            method_str,
            full_path,
            event_id
        );

        // Get the offline flag from the event registration
        let offline = {
            let conn = state.db.lock().unwrap();
            let mut stmt = match conn
                .prepare("SELECT offline FROM event_registrations WHERE event_id = ?1")
            {
                Ok(stmt) => stmt,
                Err(_) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
                }
            };

            stmt.query_row(params![event_id], |row| row.get::<_, bool>(0))
                .unwrap_or(false)
        };

        // Fire event through EventBus
        if let Some(event_bus_state) = state.app_handle.try_state::<TauriEventBusState>() {
            let event_bus = &event_bus_state.0;

            // TODO: Parse request body as payload
            let payload = None;

            if let Err(e) = event_bus.push_event(payload, app_id.clone(), event_id.clone(), offline)
            {
                tracing::error!("Failed to push event to EventBus: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to trigger event")
                    .into_response();
            }

            tracing::info!(
                "Event {} triggered successfully (offline: {})",
                event_id,
                offline
            );
        } else {
            tracing::error!("EventBus state not available");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Event system not available",
            )
                .into_response();
        }

        (StatusCode::OK, "Event triggered").into_response()
    }
}

#[derive(Clone)]
struct HttpServerState {
    db: DbConnection,
    app_handle: AppHandle,
}

#[async_trait::async_trait]
impl EventSink for HttpSink {
    async fn start(&self, app_handle: &AppHandle, db: DbConnection) -> Result<()> {
        Self::init_tables(&db)?;

        let state = Arc::new(HttpServerState {
            db: db.clone(),
            app_handle: app_handle.clone(),
        });

        let app = Router::new()
            .route("/:app_id/*path", any(Self::handle_request))
            .route("/health", axum::routing::get(Self::health_check))
            .with_state(state)
            .layer(ServiceBuilder::new());

        tokio::spawn(async move {
            let listener = match tokio::net::TcpListener::bind("0.0.0.0:9657").await {
                Ok(l) => l,
                Err(e) => {
                    tracing::error!("Failed to bind HTTP server: {}", e);
                    return;
                }
            };

            tracing::info!("HTTP server listening on 0.0.0.0:9657");

            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!("HTTP server error: {}", e);
            }
        });

        tracing::info!("HTTP sink started - server ready on 0.0.0.0:9657");
        Ok(())
    }

    async fn stop(&self, _app_handle: &AppHandle, _db: DbConnection) -> Result<()> {
        // TODO: Shutdown Axum server if no more routes registered
        tracing::info!("HTTP sink stopped");
        Ok(())
    }

    async fn on_register(
        &self,
        _app_handle: &AppHandle,
        registration: &EventRegistration,
        db: DbConnection,
    ) -> Result<()> {
        Self::add_route(&db, registration, self)?;
        tracing::info!(
            "Registered HTTP route: {} {} -> event {}",
            self.method,
            self.path,
            registration.event_id
        );
        Ok(())
    }

    async fn on_unregister(
        &self,
        _app_handle: &AppHandle,
        registration: &EventRegistration,
        db: DbConnection,
    ) -> Result<()> {
        Self::remove_route(&db, &registration.event_id)?;
        tracing::info!("Unregistered HTTP route: {} {}", self.method, self.path);
        Ok(())
    }
}
