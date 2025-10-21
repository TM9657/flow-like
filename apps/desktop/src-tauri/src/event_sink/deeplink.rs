use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use super::manager::DbConnection;
use super::{EventRegistration, EventSink};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeeplinkSink {
    pub path: String,
}

impl DeeplinkSink {
    fn init_tables(db: &DbConnection) -> Result<()> {
        let conn = db.lock().unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS deeplink_routes (
                event_id TEXT PRIMARY KEY,
                app_id TEXT NOT NULL,
                path TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                UNIQUE(app_id, path)
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_deeplink_app_path ON deeplink_routes(app_id, path)",
            [],
        )?;

        Ok(())
    }

    fn add_route(
        db: &DbConnection,
        registration: &EventRegistration,
        config: &DeeplinkSink,
    ) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        let conn = db.lock().unwrap();

        let existing = conn
            .query_row(
                "SELECT event_id FROM deeplink_routes WHERE app_id = ? AND path = ?",
                params![&registration.app_id, &config.path],
                |row| row.get::<_, String>(0),
            )
            .ok();

        if let Some(existing_event_id) = existing {
            if existing_event_id != registration.event_id {
                anyhow::bail!(
                    "Deeplink path '{}' for app '{}' is already registered to event '{}'",
                    config.path,
                    registration.app_id,
                    existing_event_id
                );
            } else {
                tracing::info!(
                    "Deeplink route already registered for event '{}', updating timestamp",
                    registration.event_id
                );
            }
        }

        conn.execute(
            "INSERT OR REPLACE INTO deeplink_routes (event_id, app_id, path, created_at)
             VALUES (?, ?, ?, ?)",
            params![
                &registration.event_id,
                &registration.app_id,
                &config.path,
                now
            ],
        )?;

        Ok(())
    }

    fn remove_route(db: &DbConnection, event_id: &str) -> Result<()> {
        let conn = db.lock().unwrap();
        conn.execute(
            "DELETE FROM deeplink_routes WHERE event_id = ?",
            params![event_id],
        )?;
        Ok(())
    }

    pub fn handle_trigger(
        app_handle: &AppHandle,
        app_id: &str,
        path: &str,
        query_params: serde_json::Value,
    ) -> Result<bool> {
        use crate::state::TauriEventSinkManagerState;

        let manager_state = app_handle
            .try_state::<TauriEventSinkManagerState>()
            .ok_or_else(|| anyhow::anyhow!("EventSinkManager state not available"))?;

        let manager = manager_state.0.clone();
        let db = {
            let manager_guard =
                flow_like_types::tokio::task::block_in_place(|| manager.blocking_lock());
            manager_guard.db()
        };

        let conn = db.lock().unwrap();

        let event_id: String = conn
            .query_row(
                "SELECT event_id FROM deeplink_routes WHERE app_id = ? AND path = ?",
                params![app_id, path],
                |row| row.get(0),
            )
            .map_err(|_| {
                anyhow::anyhow!(
                    "No deeplink route found for app_id='{}' path='{}'",
                    app_id,
                    path
                )
            })?;

        drop(conn);

        tracing::info!(
            "Triggering deeplink event '{}' for app '{}' path '{}' with params: {:?}",
            event_id,
            app_id,
            path,
            query_params
        );

        let payload = if query_params.is_null()
            || (query_params.is_object() && query_params.as_object().unwrap().is_empty())
        {
            None
        } else {
            Some(query_params)
        };

        let manager_guard =
            flow_like_types::tokio::task::block_in_place(|| manager.blocking_lock());

        manager_guard.fire_event(app_handle, &event_id, payload, None)
    }
}

#[async_trait::async_trait]
impl EventSink for DeeplinkSink {
    async fn start(&self, _app_handle: &AppHandle, db: DbConnection) -> Result<()> {
        Self::init_tables(&db)?;
        tracing::info!("🔗 Deeplink event sink initialized");
        Ok(())
    }

    async fn stop(&self, _app_handle: &AppHandle, _db: DbConnection) -> Result<()> {
        tracing::info!("🔗 Deeplink event sink stopped");
        Ok(())
    }

    async fn on_register(
        &self,
        _app_handle: &AppHandle,
        registration: &EventRegistration,
        db: DbConnection,
    ) -> Result<()> {
        tracing::info!(
            "Registering deeplink route for event '{}': path '{}'",
            registration.event_id,
            self.path
        );

        Self::add_route(&db, registration, self)?;

        tracing::info!(
            "✅ Deeplink route registered: flow-like://trigger/{}/{}",
            registration.app_id,
            self.path
        );

        Ok(())
    }

    async fn on_unregister(
        &self,
        _app_handle: &AppHandle,
        registration: &EventRegistration,
        db: DbConnection,
    ) -> Result<()> {
        tracing::info!(
            "Unregistering deeplink route for event '{}'",
            registration.event_id
        );

        Self::remove_route(&db, &registration.event_id)?;
        Ok(())
    }
}
