use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use super::manager::DbConnection;
use super::{EventRegistration, EventSink};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailSink {
    pub imap_server: String,
    pub imap_port: u16,
    pub username: String,
    pub password: String,
    pub folder: Option<String>,
    pub use_tls: bool,
    pub last_seen_uid: Option<u32>,
}

impl EmailSink {
    fn init_tables(db: &DbConnection) -> Result<()> {
        let conn = db.lock().unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS email_watchers (
                event_id TEXT PRIMARY KEY,
                imap_server TEXT NOT NULL,
                imap_port INTEGER NOT NULL,
                username TEXT NOT NULL,
                password TEXT NOT NULL,
                folder TEXT NOT NULL,
                use_tls INTEGER NOT NULL,
                last_seen_uid INTEGER,
                last_checked INTEGER,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        Ok(())
    }

    fn add_watcher(
        db: &DbConnection,
        registration: &EventRegistration,
        config: &EmailSink,
    ) -> Result<()> {
        let conn = db.lock().unwrap();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        conn.execute(
            "INSERT OR REPLACE INTO email_watchers
             (event_id, imap_server, imap_port, username, password, folder, use_tls, last_seen_uid, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                registration.event_id,
                config.imap_server,
                config.imap_port,
                config.username,
                config.password,
                config.folder.as_ref().unwrap_or(&"INBOX".to_string()),
                config.use_tls as i32,
                config.last_seen_uid,
                now,
            ],
        )?;

        Ok(())
    }

    fn remove_watcher(db: &DbConnection, event_id: &str) -> Result<()> {
        let conn = db.lock().unwrap();
        conn.execute(
            "DELETE FROM email_watchers WHERE event_id = ?1",
            params![event_id],
        )?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl EventSink for EmailSink {
    async fn start(&self, _app_handle: &AppHandle, db: DbConnection) -> Result<()> {
        Self::init_tables(&db)?;

        // TODO: Start email polling worker
        // Worker polls email_watchers table and:
        // 1. Connects to IMAP servers
        // 2. Checks for new messages based on last_seen_uid
        // 3. Fires events for new emails
        // 4. Updates last_seen_uid and last_checked

        tracing::info!("Email sink started - polling worker ready");
        Ok(())
    }

    async fn stop(&self, _app_handle: &AppHandle, _db: DbConnection) -> Result<()> {
        // TODO: Stop email polling worker
        tracing::info!("Email sink stopped");
        Ok(())
    }

    async fn on_register(
        &self,
        _app_handle: &AppHandle,
        registration: &EventRegistration,
        db: DbConnection,
    ) -> Result<()> {
        Self::add_watcher(&db, registration, self)?;
        tracing::info!(
            "Registered email watcher: {}@{} -> event {}",
            self.username,
            self.imap_server,
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
        Self::remove_watcher(&db, &registration.event_id)?;
        tracing::info!("Unregistered email watcher: {}", registration.event_id);
        Ok(())
    }
}
