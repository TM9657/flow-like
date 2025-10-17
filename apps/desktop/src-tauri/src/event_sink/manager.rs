use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use serde_json;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

use super::cron::CronSink;
use super::*;
use super::{EventConfig, EventRegistration, EventSink};

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
                default_payload TEXT,
                personal_access_token TEXT
            )",
            [],
        )?;

        Ok(())
    }

    fn save_registration(&self, registration: &EventRegistration) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let config_json = serde_json::to_string(&registration.config)?;
        let default_payload_json = registration
            .default_payload
            .as_ref()
            .map(|p| serde_json::to_string(p))
            .transpose()?;

        let updated_at = registration
            .updated_at
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;
        let created_at = registration
            .created_at
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        // Debug: Log PAT before saving
        println!("Saving registration for event {} with PAT present: {}", registration.event_id, registration.personal_access_token.is_some());
        tracing::info!("Saving registration for event {} with PAT present: {}", registration.event_id, registration.personal_access_token.is_some());

        conn.execute(
            "INSERT OR REPLACE INTO event_registrations
             (event_id, name, type, updated_at, created_at, config, offline, app_id, default_payload, personal_access_token)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
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
                registration.personal_access_token,
            ],
        )?;

        println!("Successfully saved registration for event {}", registration.event_id);
        tracing::info!("Successfully saved registration for event {}", registration.event_id);

        Ok(())
    }

    fn get_registration(&self, event_id: &str) -> Result<Option<EventRegistration>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT event_id, name, type, updated_at, created_at, config, offline, app_id, default_payload, personal_access_token
             FROM event_registrations WHERE event_id = ?1"
        )?;

        let result = stmt.query_row(params![event_id], |row| {
            let config_json: String = row.get(5)?;
            let config: EventConfig = serde_json::from_str(&config_json).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;

            let default_payload_json: Option<String> = row.get(8)?;
            let default_payload = default_payload_json
                .map(|json| serde_json::from_str(&json))
                .transpose()
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        8,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

            let updated_at_secs: i64 = row.get(3)?;
            let created_at_secs: i64 = row.get(4)?;

            Ok(EventRegistration {
                event_id: row.get(0)?,
                name: row.get(1)?,
                r#type: row.get(2)?,
                updated_at: std::time::UNIX_EPOCH
                    + std::time::Duration::from_secs(updated_at_secs as u64),
                created_at: std::time::UNIX_EPOCH
                    + std::time::Duration::from_secs(created_at_secs as u64),
                config,
                offline: row.get::<_, i32>(6)? != 0,
                app_id: row.get(7)?,
                default_payload,
                personal_access_token: row.get(9)?,
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
            "SELECT event_id, name, type, updated_at, created_at, config, offline, app_id, default_payload, personal_access_token
             FROM event_registrations ORDER BY created_at DESC"
        )?;

        let registrations = stmt
            .query_map([], |row| {
                let config_json: String = row.get(5)?;
                let config: EventConfig = serde_json::from_str(&config_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        5,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

                let default_payload_json: Option<String> = row.get(8)?;
                let default_payload = default_payload_json
                    .map(|json| serde_json::from_str(&json))
                    .transpose()
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            8,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;

                let updated_at_secs: i64 = row.get(3)?;
                let created_at_secs: i64 = row.get(4)?;

                Ok(EventRegistration {
                    event_id: row.get(0)?,
                    name: row.get(1)?,
                    r#type: row.get(2)?,
                    updated_at: std::time::UNIX_EPOCH
                        + std::time::Duration::from_secs(updated_at_secs as u64),
                    created_at: std::time::UNIX_EPOCH
                        + std::time::Duration::from_secs(created_at_secs as u64),
                    config,
                    offline: row.get::<_, i32>(6)? != 0,
                    app_id: row.get(7)?,
                    default_payload,
                    personal_access_token: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(registrations)
    }

    fn delete_registration(&self, event_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM event_registrations WHERE event_id = ?1",
            params![event_id],
        )?;
        Ok(())
    }
}

/// Manager for all event sinks
/// Initializes database and coordinates sink lifecycle
pub struct EventSinkManager {
    db: DbConnection,
    storage: Arc<RegistrationStorage>,
    started_sinks: Arc<flow_like_types::tokio::sync::Mutex<HashSet<String>>>,
}

impl EventSinkManager {
    /// Create a new event sink manager
    pub fn new(db_path: &str) -> Result<Self> {
        let storage = Arc::new(RegistrationStorage::new(PathBuf::from(db_path))?);
        let db = storage.connection();

        Ok(Self {
            db,
            storage,
            started_sinks: Arc::new(flow_like_types::tokio::sync::Mutex::new(HashSet::new())),
        })
    }

    /// Check if a sink type has been started, and mark it as started if not
    async fn ensure_sink_started(
        &self,
        sink_type: &str,
        app_handle: &AppHandle,
        sink: &dyn EventSink,
    ) -> Result<()> {
        let mut started = self.started_sinks.lock().await;

        if !started.contains(sink_type) {
            tracing::info!("🚀 Starting {} sink for the first time", sink_type);
            println!("🚀 Starting {} sink for the first time", sink_type);
            sink.start(app_handle, self.db.clone()).await?;
            started.insert(sink_type.to_string());
            tracing::info!("✅ {} sink started and marked as active", sink_type);
            println!("✅ {} sink started and marked as active", sink_type);
        } else {
            tracing::debug!("Sink {} already started, skipping", sink_type);
        }

        Ok(())
    }

    /// Get database connection
    pub fn db(&self) -> DbConnection {
        self.db.clone()
    }

    /// Fire an event by retrieving its configuration and pushing it to the event bus
    /// This is a centralized method that handles offline status, personal_access_token, etc.
    pub fn fire_event(
        &self,
        app_handle: &AppHandle,
        event_id: &str,
        payload: Option<flow_like_types::Value>,
    ) -> Result<bool> {
        println!("🔥 [FIRE_EVENT] Starting fire_event for: {}", event_id);
        tracing::info!("🔥 [FIRE_EVENT] Starting fire_event for: {}", event_id);

        println!("🔥 [FIRE_EVENT] Attempting to lock database connection...");
        let conn = self.db.lock().unwrap();
        println!("✅ [FIRE_EVENT] Database connection locked");

        let mut stmt = conn.prepare(
            "SELECT app_id, offline, personal_access_token FROM event_registrations WHERE event_id = ?1",
        )?;
        println!("✅ [FIRE_EVENT] SQL statement prepared");

        let query_result = stmt.query_row(params![event_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, bool>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        });

        let (app_id, offline, personal_access_token) = match query_result {
            Ok(result) => {
                println!("✅ [FIRE_EVENT] Query successful for event: {}", event_id);
                result
            }
            Err(e) => {
                println!("❌ [FIRE_EVENT] Query failed for event {}: {:?}", event_id, e);
                drop(stmt);
                drop(conn);
                return Err(e.into());
            }
        };

        println!("🔥 [FIRE_EVENT] Retrieved config - app_id: {}, offline: {}, has_token: {}",
                 app_id, offline, personal_access_token.is_some());

        println!("🔥 [FIRE_EVENT] Dropping statement and connection...");
        drop(stmt);
        drop(conn);
        println!("✅ [FIRE_EVENT] Database resources released");

        println!("🔥 [FIRE_EVENT] Attempting to get EventBus state...");
        if let Some(event_bus_state) = app_handle.try_state::<crate::state::TauriEventBusState>()
        {
            println!("✅ [FIRE_EVENT] EventBus state found");
            let event_bus = &event_bus_state.0;

            // Use stored personal_access_token if available, otherwise use default
            let push_result = if let Some(token) = personal_access_token {
                println!("🔥 [FIRE_EVENT] Pushing event WITH token");
                event_bus.push_event_with_token(
                    payload,
                    app_id.clone(),
                    event_id.to_string(),
                    offline,
                    Some(token),
                )
            } else {
                println!("🔥 [FIRE_EVENT] Pushing event WITHOUT token");
                event_bus.push_event_with_token(payload, app_id.clone(), event_id.to_string(), offline, personal_access_token)
            };

            match push_result {
                Ok(_) => {
                    println!("✅ [FIRE_EVENT] Event {} pushed successfully", event_id);
                    Ok(true)
                }
                Err(e) => {
                    println!("❌ [FIRE_EVENT] Failed to push event {}: {:?}", event_id, e);
                    tracing::error!("Failed to push event {}: {:?}", event_id, e);
                    Ok(false)
                }
            }
        } else {
            println!("❌ [FIRE_EVENT] EventBus state not available for {}", event_id);
            tracing::error!("EventBus state not available for {}", event_id);
            #[cfg(debug_assertions)]
            println!("❌ EventBus state not available for {}", event_id);
            Ok(false)
        }
    }

    /// Register a new event with its sink configuration
    pub async fn register_event(
        &self,
        app_handle: &AppHandle,
        registration: EventRegistration,
    ) -> Result<()> {
        tracing::info!(
            "Registering event {} with type {}",
            registration.event_id,
            registration.r#type
        );

        // Save registration to database
        self.storage.save_registration(&registration)?;

        // Get the appropriate sink and call on_register
        match &registration.config {
            EventConfig::Cron(sink) => {
                self.ensure_sink_started("cron", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::Discord(sink) => {
                self.ensure_sink_started("discord", app_handle, sink)
                    .await?;
                sink.on_register(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::Email(sink) => {
                self.ensure_sink_started("email", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::Http(sink) => {
                self.ensure_sink_started("http", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::Rss(sink) => {
                self.ensure_sink_started("rss", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::Slack(sink) => {
                self.ensure_sink_started("slack", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::Telegram(sink) => {
                self.ensure_sink_started("telegram", app_handle, sink)
                    .await?;
                sink.on_register(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::WebWatcher(sink) => {
                self.ensure_sink_started("web_watcher", app_handle, sink)
                    .await?;
                sink.on_register(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::File(sink) => {
                self.ensure_sink_started("file", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::Webhook(sink) => {
                self.ensure_sink_started("webhook", app_handle, sink)
                    .await?;
                sink.on_register(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::GitHub(sink) => {
                self.ensure_sink_started("github", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::Mqtt(sink) => {
                self.ensure_sink_started("mqtt", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::Notion(sink) => {
                self.ensure_sink_started("notion", app_handle, sink).await?;
                sink.on_register(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::GeoLocation(sink) => {
                self.ensure_sink_started("geolocation", app_handle, sink)
                    .await?;
                sink.on_register(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::Deeplink(sink) => {
                tracing::warn!("Deeplink sink not yet implemented");
                // TODO: Implement DeeplinkSink
            }
            EventConfig::Nfc(sink) => {
                tracing::warn!("NFC sink not yet implemented");
                // TODO: Implement NFCSink
            }
            EventConfig::Shortcut(sink) => {
                tracing::warn!("Shortcut sink not yet implemented");
                // TODO: Implement ShortcutSink
            }
            EventConfig::Mcp(sink) => {
                tracing::warn!("MCP sink not yet implemented");
                // TODO: Implement MCPSink
            }
        }

        tracing::info!(
            "Registered event: {} with config: {:?}",
            registration.event_id,
            registration.config
        );
        Ok(())
    }

    /// Unregister an event
    pub async fn unregister_event(&self, app_handle: &AppHandle, event_id: &str) -> Result<()> {
        // Get registration from database
        let registration = self
            .storage
            .get_registration(event_id)?
            .ok_or_else(|| anyhow::anyhow!("Registration not found: {}", event_id))?;

        // Call on_unregister for the sink
        match &registration.config {
            EventConfig::Cron(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::Discord(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::Email(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::Http(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::Rss(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::Slack(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::Telegram(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::WebWatcher(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::File(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::Webhook(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::GitHub(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::Mqtt(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::Notion(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone())
                    .await?;
            }
            EventConfig::GeoLocation(sink) => {
                sink.on_unregister(app_handle, &registration, self.db.clone())
                    .await?;
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

    /// Automatically register an event from a flow_like Event struct
    /// This parses the event.config bytes and event_type to determine which sink to use
    pub async fn register_from_flow_event(
        &self,
        app_handle: &AppHandle,
        app_id: &str,
        event: &flow_like::flow::event::Event,
        offline: Option<bool>,
        personal_access_token: Option<String>,
    ) -> Result<()> {
        tracing::info!("=== register_from_flow_event DEBUG ===");
        tracing::info!("Event ID: {}", event.id);
        tracing::info!("Event Name: {}", event.name);
        tracing::info!("Event Type: {}", event.event_type);
        tracing::info!("Event Active: {}", event.active);
        tracing::info!("Config bytes length: {}", event.config.len());
        println!("=== register_from_flow_event DEBUG ===");
        println!("Event ID: {}", event.id);
        println!("Event Name: {}", event.name);
        println!("Event Type: {}", event.event_type);
        println!("Event Active: {}", event.active);
        println!("Config bytes length: {}", event.config.len());

        if !event.config.is_empty() {
            if let Ok(config_str) = String::from_utf8(event.config.clone()) {
                tracing::info!("Config as string: {}", config_str);
                println!("Config as string: {}", config_str);
            }
        }

        // Check if this event type supports sink registration
        if !Self::supports_sink_registration(&event.event_type) {
            tracing::debug!(
                "Event type '{}' does not require sink registration, skipping",
                event.event_type
            );
            println!(
                "Event type '{}' does not require sink registration, skipping",
                event.event_type
            );
            // Clean up if it was previously registered (e.g., type changed)
            if self.storage.get_registration(&event.id)?.is_some() {
                self.unregister_event(app_handle, &event.id).await?;
            }
            return Ok(());
        }

        // Only register active events
        if !event.active {
            tracing::info!("Skipping registration for inactive event: {}", event.id);
            println!("Skipping registration for inactive event: {}", event.id);
            // If it was previously registered, unregister it
            if self.storage.get_registration(&event.id)?.is_some() {
                self.unregister_event(app_handle, &event.id).await?;
            }
            return Ok(());
        }

        // Determine which PAT to use based on existing registration
        let final_pat = if let Some(existing_reg) = self.storage.get_registration(&event.id)? {
            match (&existing_reg.personal_access_token, &personal_access_token) {
                (Some(existing), None) => {
                    // Keep existing PAT if new one is None
                    tracing::info!("Keeping existing PAT for event {}", event.id);
                    Some(existing.clone())
                }
                (None, Some(new_pat)) => {
                    // Use new PAT if existing is None
                    tracing::info!("Using new PAT for event {}", event.id);
                    Some(new_pat.clone())
                }
                (Some(_), Some(new_pat)) => {
                    // Both exist, use new one
                    tracing::info!("Updating PAT for event {}", event.id);
                    Some(new_pat.clone())
                }
                (None, None) => {
                    // Neither exists
                    tracing::info!("No PAT for event {}", event.id);
                    None
                }
            }
        } else {
            // No existing registration, use whatever was provided
            personal_access_token
        };

        // Parse config bytes to determine sink type and configuration
        let config_result = self.parse_event_config(&event.event_type, &event.config);

        match config_result {
            Ok(event_config) => {
                let registration = EventRegistration {
                    event_id: event.id.clone(),
                    name: event.name.clone(),
                    r#type: event.event_type.clone(),
                    updated_at: event.updated_at,
                    created_at: event.created_at,
                    config: event_config,
                    offline: offline.unwrap_or(true),
                    app_id: app_id.to_string(),
                    default_payload: None, // TODO: Parse from event if needed
                    personal_access_token: final_pat.clone(),
                };

                // Debug: Log PAT in registration
                println!("Registering event {} with PAT present: {}", event.id, registration.personal_access_token.is_some());
                tracing::info!("Registering event {} with PAT present: {}", event.id, registration.personal_access_token.is_some());

                self.register_event(app_handle, registration).await?;
                tracing::info!(
                    "Auto-registered event {} with type {}",
                    event.id,
                    event.event_type
                );
            }
            Err(e) => {
                tracing::warn!(
                    "Could not parse config for event {} (type: {}): {}",
                    event.id,
                    event.event_type,
                    e
                );
                tracing::warn!("Event will not have an active sink");
                println!(
                    "Could not parse config for event {} (type: {}): {}",
                    event.id, event.event_type, e
                );
                println!("Event will not have an active sink");
                // If it was previously registered, unregister it
                if self.storage.get_registration(&event.id)?.is_some() {
                    self.unregister_event(app_handle, &event.id).await?;
                }
            }
        }

        Ok(())
    }

    /// Parse event config bytes based on event_type
    fn parse_event_config(&self, event_type: &str, config_bytes: &[u8]) -> Result<EventConfig> {
        // If config is empty, try to create default config based on type
        if config_bytes.is_empty() {
            return Err(anyhow::anyhow!(
                "Empty config for event type: {}",
                event_type
            ));
        }

        // Deserialize the config JSON
        let config_json: serde_json::Value =
            serde_json::from_slice(config_bytes).context("Failed to parse config as JSON")?;

        // Match event_type to sink type and parse appropriate config
        match event_type {
            "cron" => {
                let cron_config: super::cron::CronSink =
                    serde_json::from_value(config_json).context("Failed to parse cron config")?;
                Ok(EventConfig::Cron(cron_config))
            }
            "api" | "http" => {
                let http_config: super::http::HttpSink =
                    serde_json::from_value(config_json).context("Failed to parse HTTP config")?;
                Ok(EventConfig::Http(http_config))
            }
            "mail" | "email" => {
                let email_config: super::email::EmailSink =
                    serde_json::from_value(config_json).context("Failed to parse email config")?;
                Ok(EventConfig::Email(email_config))
            }
            "rss" => {
                let rss_config: super::rss::RSSSink =
                    serde_json::from_value(config_json).context("Failed to parse RSS config")?;
                Ok(EventConfig::Rss(rss_config))
            }
            "discord" => {
                let discord_config: super::discord::DiscordSink =
                    serde_json::from_value(config_json)
                        .context("Failed to parse Discord config")?;
                Ok(EventConfig::Discord(discord_config))
            }
            // Add more sink types as needed
            _ => Err(anyhow::anyhow!(
                "Unsupported event type for sink registration: {}",
                event_type
            )),
        }
    }

    /// Add or update an event sink registration
    /// If the event already exists with a different type, it will be unregistered first
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
                    event_id,
                    old_type,
                    new_type
                );

                // Unregister from the old sink
                self.unregister_event(app_handle, &event_id).await?;

                // Now register with the new sink
                tracing::info!(
                    "Registering event {} with new sink type {}",
                    event_id,
                    new_type
                );
                self.register_event(app_handle, registration).await?;
            } else {
                // Same sink type - unregister and re-register to update configuration
                tracing::info!(
                    "Updating event {} configuration for {} sink",
                    event_id,
                    new_type
                );

                // Unregister to clean up old configuration
                self.unregister_event(app_handle, &event_id).await?;

                // Register with new configuration
                self.register_event(app_handle, registration).await?;
            }
        } else {
            // New event - just register it
            tracing::info!(
                "Adding new event {} with sink type {}",
                event_id,
                registration.r#type
            );
            self.register_event(app_handle, registration).await?;
        }

        Ok(())
    }

    /// Remove an event sink
    /// This is an alias for unregister_event with clearer naming for external API
    pub async fn remove_event_sink(&self, app_handle: &AppHandle, event_id: &str) -> Result<()> {
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

    /// Check if a sink is active for an event
    /// Returns true if the event is registered and has an active sink
    pub fn is_event_active(&self, event_id: &str) -> bool {
        let registration = self.storage
            .get_registration(event_id)
            .ok()
            .flatten();

        if let Some(reg) = registration {
            // An event is considered active if it is registered and not offline
            (!reg.offline && reg.personal_access_token.is_some()) || reg.offline
        } else {
            false
        }
    }

    /// Check if an event type supports sink registration
    /// Some event types (like quick_action, chat) don't need sink infrastructure
    pub fn supports_sink_registration(event_type: &str) -> bool {
        matches!(
            event_type,
            "cron"
                | "api"
                | "http"
                | "mail"
                | "email"
                | "rss"
                | "discord"
                | "webhook"
                | "slack"
                | "telegram"
                | "mqtt"
                | "github"
                | "notion"
                | "web_watcher"
                | "file"
                | "geolocation"
                | "deeplink"
                | "nfc"
                | "shortcut"
                | "mcp"
        )
    }

    /// Initialize all sinks on app startup
    /// This loads all registrations from the database and starts their infrastructure
    /// NOTE: We only need to start the sink workers, not re-register events
    /// (the database already has the registrations)
    pub async fn init_from_storage(&self, app_handle: &AppHandle) -> Result<()> {
        let registrations = self.list_registrations()?;

        tracing::info!(
            "🔄 Loading {} event registrations from database",
            registrations.len()
        );
        println!(
            "🔄 Loading {} event registrations from database",
            registrations.len()
        );

        // Group registrations by sink type to start each sink once
        let mut sink_types = std::collections::HashSet::new();
        for registration in &registrations {
            tracing::debug!(
                "Found registration: event_id={}, type={}",
                registration.event_id,
                registration.r#type
            );
            // Extract sink type from config
            let sink_type = match &registration.config {
                EventConfig::Cron(_) => "cron",
                EventConfig::Discord(_) => "discord",
                EventConfig::Email(_) => "email",
                EventConfig::Http(_) => "http",
                EventConfig::Rss(_) => "rss",
                EventConfig::Slack(_) => "slack",
                EventConfig::Telegram(_) => "telegram",
                EventConfig::WebWatcher(_) => "web_watcher",
                EventConfig::File(_) => "file",
                EventConfig::Webhook(_) => "webhook",
                EventConfig::GitHub(_) => "github",
                EventConfig::Mqtt(_) => "mqtt",
                EventConfig::Notion(_) => "notion",
                EventConfig::GeoLocation(_) => "geolocation",
                _ => continue, // Skip unimplemented sinks
            };
            sink_types.insert(sink_type);
        }

        // Start each unique sink type
        tracing::info!("📋 Unique sink types to start: {:?}", sink_types);
        println!("📋 Unique sink types to start: {:?}", sink_types);

        for sink_type in sink_types {
            tracing::info!("⚙️ Starting {} sink during initialization", sink_type);
            println!("⚙️ Starting {} sink during initialization", sink_type);

            // Start the sink by ensuring it's initialized
            // The sink's start() method will spawn the background worker
            match sink_type {
                "cron" => {
                    let cron_sink = CronSink {
                        expression: None,
                        scheduled_for: None,
                        last_fired: None,
                        timezone: None,
                    };
                    if let Err(e) = self
                        .ensure_sink_started("cron", app_handle, &cron_sink)
                        .await
                    {
                        tracing::error!("❌ Failed to start cron sink: {}", e);
                        println!("❌ Failed to start cron sink: {}", e);
                    }
                }
                // Add other sink types as needed
                _ => {
                    tracing::debug!(
                        "Sink type {} will be started on first registration",
                        sink_type
                    );
                }
            }
        }

        tracing::info!(
            "✅ Sink initialization complete. {} event registrations ready.",
            registrations.len()
        );
        println!(
            "✅ Sink initialization complete. {} event registrations ready.",
            registrations.len()
        );
        Ok(())
    }
}
