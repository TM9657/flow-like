use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serenity::all::GatewayIntents;
use tauri::AppHandle;

use super::manager::DbConnection;
use super::{EventRegistration, EventSink};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordSink {
    pub token: String,
    pub channel_id: Option<String>,
    pub intents: Option<Vec<GatewayIntents>>,
}

impl DiscordSink {
    fn init_tables(db: &DbConnection) -> Result<()> {
        let conn = db.lock().unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS discord_bots (
                token TEXT PRIMARY KEY,
                intents TEXT,
                connected INTEGER NOT NULL DEFAULT 0,
                last_message_id TEXT,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS discord_handlers (
                event_id TEXT PRIMARY KEY,
                bot_token TEXT NOT NULL,
                channel_id TEXT,
                created_at INTEGER NOT NULL,
                FOREIGN KEY(bot_token) REFERENCES discord_bots(token) ON DELETE CASCADE
            )",
            [],
        )?;

        Ok(())
    }

    fn add_bot_and_handler(
        db: &DbConnection,
        registration: &EventRegistration,
        config: &DiscordSink,
    ) -> Result<()> {
        let conn = db.lock().unwrap();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        let intents_json = config
            .intents
            .as_ref()
            .map(|i| serde_json::to_string(i))
            .transpose()?;

        conn.execute(
            "INSERT OR IGNORE INTO discord_bots (token, intents, created_at)
             VALUES (?1, ?2, ?3)",
            params![config.token, intents_json, now],
        )?;

        conn.execute(
            "INSERT OR REPLACE INTO discord_handlers
             (event_id, bot_token, channel_id, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![registration.event_id, config.token, config.channel_id, now,],
        )?;

        Ok(())
    }

    fn remove_handler(db: &DbConnection, event_id: &str) -> Result<()> {
        let conn = db.lock().unwrap();
        conn.execute(
            "DELETE FROM discord_handlers WHERE event_id = ?1",
            params![event_id],
        )?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl EventSink for DiscordSink {
    async fn start(&self, _app_handle: &AppHandle, db: DbConnection) -> Result<()> {
        Self::init_tables(&db)?;

        // TODO: Initialize Discord client manager
        // The manager should:
        // 1. Create Discord clients for each unique bot token in discord_bots table
        // 2. Connect to Discord gateway with configured intents
        // 3. Set up message handlers that query discord_handlers table
        // 4. Route messages to appropriate events based on channel_id filters

        tracing::info!("Discord sink started - client manager ready");
        Ok(())
    }

    async fn stop(&self, _app_handle: &AppHandle, _db: DbConnection) -> Result<()> {
        // TODO: Disconnect all Discord clients
        tracing::info!("Discord sink stopped");
        Ok(())
    }

    async fn on_register(
        &self,
        _app_handle: &AppHandle,
        registration: &EventRegistration,
        db: DbConnection,
    ) -> Result<()> {
        Self::add_bot_and_handler(&db, registration, self)?;

        if let Some(channel) = &self.channel_id {
            tracing::info!(
                "Registered Discord handler for channel {} -> event {}",
                channel,
                registration.event_id
            );
        } else {
            tracing::info!(
                "Registered Discord handler for all channels -> event {}",
                registration.event_id
            );
        }

        // TODO: If this is a new bot token, create and connect the Discord client

        Ok(())
    }

    async fn on_unregister(
        &self,
        _app_handle: &AppHandle,
        registration: &EventRegistration,
        db: DbConnection,
    ) -> Result<()> {
        Self::remove_handler(&db, &registration.event_id)?;
        tracing::info!("Unregistered Discord handler: {}", registration.event_id);

        // TODO: If no more handlers for this bot, disconnect the client

        Ok(())
    }
}
