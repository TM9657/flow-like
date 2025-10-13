use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use serde_json;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

use super::{EventConfig, EventRegistration, EventSink};
use super::*;

pub type DbConnection = Arc<Mutex<Connection>>;

/// Internal storage for event registrations
/// Handles database persistence of event sink configurations
struct RegistrationStorage {
    conn: DbConnection,
}

impl RegistrationStorage {
    fn new(db_path: PathBuf) -> Result<Self> {
        let conn = Connection::open(db_path).context("Failed to open database")?;
        let storage = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        storage.init_schema()?;
        Ok(storage)
    }

    fn connection(&self) -> DbConnection {
        Arc::clone(&self.conn)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Use event_id as the primary key since each event can only be attached to one sink
        conn.execute(
            "CREATE TABLE IF NOT EXISTS event_registrations (
                event_id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                type TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                config TEXT NOT NULL,
                offline INTEGER NOT NULL,
                app_id TEXT NOT NULL,
                default_payload TEXT
            )",
            [],
        )?;

        Ok(())
    }

    fn save_registration(&self, registration: &EventRegistration) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let config_json = serde_json::to_string(&registration.config)?;
        let default_payload_json = registration.default_payload.as_ref()
            .map(|p| serde_json::to_string(p))
            .transpose()?;

        let updated_at = registration.updated_at
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;
        let created_at = registration.created_at
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        conn.execute(
            "INSERT OR REPLACE INTO event_registrations
             (event_id, name, type, updated_at, created_at, config, offline, app_id, default_payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                registration.event_id,
                registration.name,
                registration.r#type,
                updated_at,
                created_at,
                config_json,
                registration.offline as i32,
                registration.app_id,
                default_payload_json,
            ],
        )?;

        Ok(())
    }

    fn get_registration(&self, event_id: &str) -> Result<Option<EventRegistration>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT event_id, name, type, updated_at, created_at, config, offline, app_id, default_payload
             FROM event_registrations WHERE event_id = ?1"
        )?;

        let result = stmt.query_row(params![event_id], |row| {
            let config_json: String = row.get(5)?;
            let config: EventConfig = serde_json::from_str(&config_json)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                    5, rusqlite::types::Type::Text, Box::new(e)
                ))?;

            let default_payload_json: Option<String> = row.get(8)?;
            let default_payload = default_payload_json
                .map(|json| serde_json::from_str(&json))
                .transpose()
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                    8, rusqlite::types::Type::Text, Box::new(e)
                ))?;

            let updated_at_secs: i64 = row.get(3)?;
            let created_at_secs: i64 = row.get(4)?;

            Ok(EventRegistration {
                event_id: row.get(0)?,
                name: row.get(1)?,
                r#type: row.get(2)?,
                updated_at: std::time::UNIX_EPOCH + std::time::Duration::from_secs(updated_at_secs as u64),
                created_at: std::time::UNIX_EPOCH + std::time::Duration::from_secs(created_at_secs as u64),
                config,
                offline: row.get::<_, i32>(6)? != 0,
                app_id: row.get(7)?,
                default_payload,
            })
        });

        match result {
            Ok(reg) => Ok(Some(reg)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn list_registrations(&self) -> Result<Vec<EventRegistration>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT event_id, name, type, updated_at, created_at, config, offline, app_id, default_payload
             FROM event_registrations ORDER BY created_at DESC"
        )?;

        let registrations = stmt.query_map([], |row| {
            let config_json: String = row.get(5)?;
            let config: EventConfig = serde_json::from_str(&config_json)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                    5, rusqlite::types::Type::Text, Box::new(e)
                ))?;

            let default_payload_json: Option<String> = row.get(8)?;
            let default_payload = default_payload_json
                .map(|json| serde_json::from_str(&json))
                .transpose()
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                    8, rusqlite::types::Type::Text, Box::new(e)
                ))?;

            let updated_at_secs: i64 = row.get(3)?;
            let created_at_secs: i64 = row.get(4)?;

            Ok(EventRegistration {
                event_id: row.get(0)?,
                name: row.get(1)?,
                r#type: row.get(2)?,
                updated_at: std::time::UNIX_EPOCH + std::time::Duration::from_secs(updated_at_secs as u64),
                created_at: std::time::UNIX_EPOCH + std::time::Duration::from_secs(created_at_secs as u64),
                config,
                offline: row.get::<_, i32>(6)? != 0,
                app_id: row.get(7)?,
                default_payload,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(registrations)
    }

    fn delete_registration(&self, event_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM event_registrations WHERE event_id = ?1", params![event_id])?;
        Ok(())
    }
}

/// Manager for all event sinks
/// Initializes database and coordinates sink lifecycle
pub struct EventSinkManager {
    db: DbConnection,
    storage: Arc<RegistrationStorage>,
    started_sinks: Arc<tokio::sync::Mutex<HashSet<String>>>,
}

impl EventSinkManager {
    /// Create a new event sink manager
    pub fn new(db_path: &str) -> Result<Self> {
        let storage = Arc::new(RegistrationStorage::new(PathBuf::from(db_path))?);
        let db = storage.connection();

        Ok(Self {
            db,
            storage,
            started_sinks: Arc::new(tokio::sync::Mutex::new(HashSet::new())),
        })
    }

    /// Check if a sink type has been started, and mark it as started if not
    async fn ensure_sink_started(&self, sink_type: &str, app_handle: &AppHandle, sink: &dyn EventSink) -> Result<()> {
        let mut started = self.started_sinks.lock().await;

        if !started.contains(sink_type) {
            tracing::info!("Starting {} sink for the first time", sink_type);
            sink.start(app_handle, self.db.clone()).await?;
            started.insert(sink_type.to_string());
        }

        Ok(())
    }

    /// Get database connection
    pub fn db(&self) -> DbConnection {
        self.db.clone()
    }

    /// Register a new event with its sink configuration
    pub async fn register_event(
        &self,
        app_handle: &AppHandle,
        registration: EventRegistration,
    ) -> Result<()> {
        // Save registration to database
        self.storage.save_registration(&registration)?;

        // Get the appropriate sink and call on_register
        match &registration.config {
            EventConfig::Cron(sink) => {
                self.ensure_sink_started("cron", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::Discord(sink) => {
                self.ensure_sink_started("discord", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::Email(sink) => {
                self.ensure_sink_started("email", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::Http(sink) => {
                self.ensure_sink_started("http", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::RSS(sink) => {
                self.ensure_sink_started("rss", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::Slack(sink) => {
                self.ensure_sink_started("slack", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::Telegram(sink) => {
                self.ensure_sink_started("telegram", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::WebWatcher(sink) => {
                self.ensure_sink_started("web_watcher", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::File(sink) => {
                self.ensure_sink_started("file", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::Webhook(sink) => {
                self.ensure_sink_started("webhook", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::GitHub(sink) => {
                self.ensure_sink_started("github", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::MQTT(sink) => {
                self.ensure_sink_started("mqtt", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::Notion(sink) => {
                self.ensure_sink_started("notion", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::GeoLocation(sink) => {
                self.ensure_sink_started("geolocation", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::Deeplink(sink) => {
                tracing::warn!("Deeplink sink not yet implemented");
                // TODO: Implement DeeplinkSink
            }
            EventConfig::NFC(sink) => {
                tracing::warn!("NFC sink not yet implemented");
                // TODO: Implement NFCSink
            }
            EventConfig::Shortcut(sink) => {
                tracing::warn!("Shortcut sink not yet implemented");
                // TODO: Implement ShortcutSink
            }
            EventConfig::MCP(sink) => {
                tracing::warn!("MCP sink not yet implemented");
                // TODO: Implement MCPSink
            }
        }

        tracing::info!("Registered event: {} with config: {:?}", registration.event_id, registration.config);
        Ok(())
    }

    /// Unregister an event
    pub async fn unregister_event(
        &self,
        app_handle: &AppHandle,
        event_id: &str,
    ) -> Result<()> {
        // Get registration from database
        let registration = self.storage.get_registration(event_id)?
            .ok_or_else(|| anyhow::anyhow!("Registration not found: {}", event_id))?;

        // Call on_unregister for the sink
        match &registration.config {
            EventConfig::Cron(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::Discord(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::Email(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::Http(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::RSS(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::Slack(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::Telegram(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::WebWatcher(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::File(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::Webhook(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::GitHub(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::MQTT(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::Notion(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone()).await?;
            }
            EventConfig::GeoLocation(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone()).await?;
            }
            _ => {
                tracing::warn!("Unregister called for unimplemented sink type");
            }
        }

        // Delete registration from database
        self.storage.delete_registration(event_id)?;

        tracing::info!("Unregistered event: {}", event_id);
        Ok(())
    }

    /// Add or update an event sink
    /// If the event already exists with a different sink type, it will be unregistered first
    pub async fn add_event_sink(
        &self,
        app_handle: &AppHandle,
        registration: EventRegistration,
    ) -> Result<()> {
        let event_id = registration.event_id.clone();

        // Check if event already exists
        if let Some(existing_registration) = self.storage.get_registration(&event_id)? {
            let old_type = existing_registration.r#type.clone();
            let new_type = registration.r#type.clone();

            // If the sink type changed, unregister the old one first
            if old_type != new_type {
                tracing::info!(
                    "Event {} is switching from {} to {}, unregistering old sink",
                    event_id, old_type, new_type
                );

                // Unregister from the old sink
                self.unregister_event(app_handle, &event_id).await?;

                // Now register with the new sink
                tracing::info!("Registering event {} with new sink type {}", event_id, new_type);
                self.register_event(app_handle, registration).await?;
            } else {
                // Same sink type - unregister and re-register to update configuration
                tracing::info!("Updating event {} configuration for {} sink", event_id, new_type);

                // Unregister to clean up old configuration
                self.unregister_event(app_handle, &event_id).await?;

                // Register with new configuration
                self.register_event(app_handle, registration).await?;
            }
        } else {
            // New event - just register it
            tracing::info!("Adding new event {} with sink type {}", event_id, registration.r#type);
            self.register_event(app_handle, registration).await?;
        }

        Ok(())
    }

    /// Remove an event sink
    /// This is an alias for unregister_event with clearer naming for external API
    pub async fn remove_event_sink(
        &self,
        app_handle: &AppHandle,
        event_id: &str,
    ) -> Result<()> {
        tracing::info!("Removing event sink for event: {}", event_id);
        self.unregister_event(app_handle, event_id).await
    }

    /// Get all registrations
    pub fn list_registrations(&self) -> Result<Vec<EventRegistration>> {
        self.storage.list_registrations()
    }

    /// Get a specific registration by event_id
    pub fn get_registration(&self, event_id: &str) -> Result<Option<EventRegistration>> {
        self.storage.get_registration(event_id)
    }

    /// Initialize all sinks on app startup
    /// This loads all registrations from the database and starts their infrastructure
    pub async fn init_from_storage(&self, app_handle: &AppHandle) -> Result<()> {
        let registrations = self.list_registrations()?;

        tracing::info!("Loading {} event registrations from database", registrations.len());

        for registration in registrations {
            if let Err(e) = self.register_event(app_handle, registration.clone()).await {
                tracing::error!("Failed to restore registration {}: {}", registration.event_id, e);
            }
        }

        Ok(())
    }
}
