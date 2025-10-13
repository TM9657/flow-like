use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::Duration;
use tauri::{AppHandle, Manager};

use super::{EventRegistration, EventSink};
use super::manager::DbConnection;
use crate::state::TauriEventBusState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RSSSink {
    pub id: String,
    pub feed_url: String,
    pub poll_interval: u64,
    pub headers: Option<Vec<(String, String)>>,
    pub filter_keywords: Option<Vec<String>>,
}

impl RSSSink {
    fn init_tables(db: &DbConnection) -> Result<()> {
        let conn = db.lock().unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS rss_feeds (
                id TEXT PRIMARY KEY,
                event_id TEXT NOT NULL UNIQUE,
                feed_url TEXT NOT NULL,
                poll_interval INTEGER NOT NULL,
                headers TEXT,
                filter_keywords TEXT,
                last_item_guid TEXT,
                last_pub_date TEXT,
                last_checked INTEGER,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        Ok(())
    }

    fn add_feed(db: &DbConnection, registration: &EventRegistration, config: &RSSSink) -> Result<()> {
        let conn = db.lock().unwrap();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        let headers_json = config.headers.as_ref()
            .map(|h| serde_json::to_string(h).ok())
            .flatten();

        let keywords_json = config.filter_keywords.as_ref()
            .map(|k| serde_json::to_string(k).ok())
            .flatten();

        conn.execute(
            "INSERT OR REPLACE INTO rss_feeds
             (id, event_id, feed_url, poll_interval, headers, filter_keywords, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                config.id,
                registration.event_id,
                config.feed_url,
                config.poll_interval,
                headers_json,
                keywords_json,
                now,
            ],
        )?;

        Ok(())
    }

    fn remove_feed(db: &DbConnection, feed_id: &str) -> Result<()> {
        let conn = db.lock().unwrap();
        conn.execute("DELETE FROM rss_feeds WHERE id = ?1", params![feed_id])?;
        Ok(())
    }

    async fn process_feeds(db: &DbConnection, _app_handle: &AppHandle) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        let feeds = {
            let conn = db.lock().unwrap();
            let mut stmt = conn.prepare(
                "SELECT id, event_id, feed_url, poll_interval, last_checked, last_item_guid
                 FROM rss_feeds
                 WHERE last_checked IS NULL OR last_checked + poll_interval <= ?1"
            )?;

            stmt.query_map(params![now], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?
        };

        for (feed_id, event_id, feed_url, _poll_interval, _last_checked, last_guid) in feeds {
            tracing::info!("Checking RSS feed: {} -> event {}", feed_url, event_id);

            // TODO: Fetch and parse RSS feed
            // TODO: Compare with last_item_guid
            // For now, we'll trigger an event on each check

            // Get app_id and offline flag from registration
            let registration_info = {
                let conn = db.lock().unwrap();
                let mut stmt = conn.prepare(
                    "SELECT app_id, offline FROM event_registrations WHERE event_id = ?1"
                )?;
                stmt.query_row(params![event_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, bool>(1)?))
                }).ok()
            };

            if let Some((app_id, offline)) = registration_info {
                if let Some(event_bus_state) = _app_handle.try_state::<TauriEventBusState>() {
                    let event_bus = &event_bus_state.0;

                    if let Err(e) = event_bus.push_event(None, app_id, event_id.clone(), offline) {
                        tracing::error!("Failed to push RSS event to EventBus: {}", e);
                    } else {
                        tracing::info!("RSS event {} triggered successfully (offline: {})", event_id, offline);
                    }
                } else {
                    tracing::error!("EventBus state not available for RSS feed {}", feed_id);
                }
            } else {
                tracing::error!("Could not find registration info for RSS feed {}", feed_id);
            }

            let conn = db.lock().unwrap();
            conn.execute(
                "UPDATE rss_feeds SET last_checked = ?1 WHERE id = ?2",
                params![now, feed_id],
            )?;
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl EventSink for RSSSink {
    async fn start(&self, app_handle: &AppHandle, db: DbConnection) -> Result<()> {
        Self::init_tables(&db)?;

        let app_handle = app_handle.clone();
        let running = Arc::new(AtomicBool::new(true));
        let worker_running = running.clone();
        let worker_db = db.clone();

        tokio::spawn(async move {
            tracing::info!("RSS worker thread started");

            while worker_running.load(Ordering::Relaxed) {
                if let Err(e) = Self::process_feeds(&worker_db, &app_handle).await {
                    tracing::error!("Error processing RSS feeds: {}", e);
                }

                tokio::time::sleep(Duration::from_secs(10)).await;
            }

            tracing::info!("RSS worker thread stopped");
        });

        tracing::info!("RSS sink started - worker ready");
        Ok(())
    }

    async fn stop(&self, _app_handle: &AppHandle, _db: DbConnection) -> Result<()> {
        tracing::info!("RSS sink stopped");
        Ok(())
    }

    async fn on_register(&self, _app_handle: &AppHandle, registration: &EventRegistration, db: DbConnection) -> Result<()> {
        Self::add_feed(&db, registration, self)?;
        tracing::info!("Registered RSS feed: {} -> event {}", self.feed_url, registration.event_id);
        Ok(())
    }

    async fn on_unregister(&self, _app_handle: &AppHandle, _registration: &EventRegistration, db: DbConnection) -> Result<()> {
        Self::remove_feed(&db, &self.id)?;
        tracing::info!("Unregistered RSS feed: {}", self.id);
        Ok(())
    }
}