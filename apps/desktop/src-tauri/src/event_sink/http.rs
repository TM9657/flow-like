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
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_http_routes_unique
             ON http_routes(app_id, path, method)",
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

        let existing = conn
            .query_row(
                "SELECT event_id FROM http_routes
                 WHERE app_id = ?1 AND path = ?2 AND method = ?3",
                params![registration.app_id, config.path, config.method],
                |row| row.get::<_, String>(0),
            )
            .ok();

        if let Some(existing_event_id) = existing {
            if existing_event_id != registration.event_id {
                tracing::warn!(
                    "Route conflict: {} {} {} already registered to event {}. Overwriting with event {}",
                    registration.app_id,
                    config.method,
                    config.path,
                    existing_event_id,
                    registration.event_id
                );

                conn.execute(
                    "DELETE FROM http_routes WHERE event_id = ?1",
                    params![existing_event_id],
                )?;
            }
        }

        conn.execute(
            "INSERT INTO http_routes
             (event_id, app_id, path, method, auth_token, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(event_id) DO UPDATE SET
                 app_id = excluded.app_id,
                 path = excluded.path,
                 method = excluded.method,
                 auth_token = excluded.auth_token",
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

    fn list_routes(db: &DbConnection) -> Result<Vec<(String, String, String, String)>> {
        let conn = db.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT app_id, method, path, event_id FROM http_routes ORDER BY app_id, path, method",
        )?;

        let routes = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(routes)
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

        tracing::debug!(
            "HTTP request received: {} /{}/{}",
            method_str,
            app_id,
            path
        );

        let route_info = {
            let conn = state.db.lock().unwrap();
            let mut stmt = match conn.prepare(
                "SELECT event_id, auth_token FROM http_routes
                 WHERE app_id = ?1 AND path = ?2 AND method = ?3",
            ) {
                Ok(stmt) => stmt,
                Err(e) => {
                    tracing::error!("Database prepare error: {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
                }
            };

            stmt.query_row(params![app_id, full_path, method_str], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
            })
            .ok()
        };

        let Some((event_id, auth_token)) = route_info else {
            tracing::debug!(
                "Route not found: {} /{}/{}",
                method_str,
                app_id,
                path
            );
            return (StatusCode::NOT_FOUND, "Route not found").into_response();
        };

        if let Some(required_token) = auth_token {
            let provided_token = headers
                .get("Authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "));

            if provided_token != Some(&required_token) {
                tracing::warn!(
                    "Unauthorized access attempt for route: {} /{}/{}",
                    method_str,
                    app_id,
                    path
                );
                return (StatusCode::UNAUTHORIZED, "Invalid or missing authorization token")
                    .into_response();
            }
        }

        tracing::info!(
            "HTTP request matched: {} /{}/{} -> event {}",
            method_str,
            app_id,
            path,
            event_id
        );

        let (offline, reg_app_id, personal_access_token) = {
            let conn = state.db.lock().unwrap();
            let mut stmt = match conn.prepare(
                "SELECT offline, app_id, personal_access_token FROM event_registrations WHERE event_id = ?1",
            ) {
                Ok(stmt) => stmt,
                Err(e) => {
                    tracing::error!("Failed to query event registration: {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
                }
            };

            match stmt.query_row(params![event_id], |row| {
                Ok((
                    row.get::<_, bool>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            }) {
                Ok(result) => result,
                Err(e) => {
                    tracing::error!("Event registration not found for event {}: {}", event_id, e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Event registration not found")
                        .into_response();
                }
            }
        };

        if reg_app_id != app_id {
            tracing::error!(
                "App ID mismatch: route app_id={}, registration app_id={}",
                app_id,
                reg_app_id
            );
            return (StatusCode::INTERNAL_SERVER_ERROR, "Configuration error").into_response();
        }

        if let Some(event_bus_state) = state.app_handle.try_state::<TauriEventBusState>() {
            let event_bus = &event_bus_state.0;

            let payload = None;

            // Use stored personal_access_token if available, otherwise use default
            let push_result = if let Some(token) = personal_access_token {
                event_bus.push_event_with_token(payload, app_id.clone(), event_id.clone(), offline, Some(token))
            } else {
                event_bus.push_event(payload, app_id.clone(), event_id.clone(), offline)
            };

            if let Err(e) = push_result {
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

        tracing::info!("🌐 Starting HTTP event sink server...");

        let routes = Self::list_routes(&db)?;
        if !routes.is_empty() {
            tracing::info!("📋 Existing HTTP routes:");
            for (app_id, method, path, event_id) in routes {
                tracing::info!("   {} /{}{} -> {}", method, app_id, path, event_id);
            }
        }

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
                    tracing::error!("❌ Failed to bind HTTP server on 0.0.0.0:9657: {}", e);
                    return;
                }
            };

            tracing::info!("✅ HTTP server listening on http://0.0.0.0:9657");
            tracing::info!("   Example: POST http://localhost:9657/my-app/webhook");

            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!("❌ HTTP server error: {}", e);
            }
        });

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
        if !self.path.starts_with('/') {
            return Err(anyhow::anyhow!(
                "HTTP path must start with '/': {}",
                self.path
            ));
        }

        let method_upper = self.method.to_uppercase();
        if !["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
            .contains(&method_upper.as_str())
        {
            return Err(anyhow::anyhow!("Unsupported HTTP method: {}", self.method));
        }

        Self::add_route(&db, registration, self)?;
        tracing::info!(
            "✓ Registered HTTP route: {} /{}{} -> event {} (app: {})",
            self.method.to_uppercase(),
            registration.app_id,
            self.path,
            registration.event_id,
            registration.app_id
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
        tracing::info!(
            "✗ Unregistered HTTP route: {} /{}{} (event: {})",
            self.method.to_uppercase(),
            registration.app_id,
            self.path,
            registration.event_id
        );
        Ok(())
    }
}
