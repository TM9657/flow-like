use anyhow::Result;
use chrono::{DateTime, Utc};
use cron::Schedule;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::Duration;
use tauri::{AppHandle, Manager};

use super::{EventRegistration, EventSink};
use super::manager::DbConnection;
use crate::state::TauriEventBusState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronSink {
    pub id: String,
    pub expression: Option<String>,
    pub scheduled_for: Option<String>,
    pub last_fired: Option<String>,
    pub timezone: Option<String>,
}

impl CronSink {
    fn init_tables(db: &DbConnection) -> Result<()> {
        let conn = db.lock().unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS cron_jobs (
                id TEXT PRIMARY KEY,
                event_id TEXT NOT NULL UNIQUE,
                expression TEXT,
                scheduled_for INTEGER,
                timezone TEXT,
                last_fired INTEGER,
                next_run INTEGER,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_cron_next_run ON cron_jobs(next_run, enabled)",
            [],
        )?;

        Ok(())
    }

    fn add_job(db: &DbConnection, registration: &EventRegistration, config: &CronSink) -> Result<()> {
        let conn = db.lock().unwrap();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        let scheduled_timestamp = config.scheduled_for.as_ref()
            .and_then(|s| s.parse::<i64>().ok());

        let last_fired_timestamp = config.last_fired.as_ref()
            .and_then(|s| s.parse::<i64>().ok());

        conn.execute(
            "INSERT OR REPLACE INTO cron_jobs
             (id, event_id, expression, scheduled_for, timezone, last_fired, next_run, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                config.id,
                registration.event_id,
                config.expression,
                scheduled_timestamp,
                config.timezone.as_ref().unwrap_or(&"UTC".to_string()),
                last_fired_timestamp,
                scheduled_timestamp,
                now,
            ],
        )?;

        Ok(())
    }

    fn remove_job(db: &DbConnection, job_id: &str) -> Result<()> {
        let conn = db.lock().unwrap();
        conn.execute("DELETE FROM cron_jobs WHERE id = ?1", params![job_id])?;
        Ok(())
    }

    async fn process_jobs(db: &DbConnection, app_handle: &AppHandle) -> Result<()> {
        let now = Utc::now().timestamp();

        let jobs = {
            let conn = db.lock().unwrap();
            let mut stmt = conn.prepare(
                "SELECT id, event_id, expression, scheduled_for, timezone, next_run
                 FROM cron_jobs
                 WHERE enabled = 1 AND (next_run IS NULL OR next_run <= ?1)"
            )?;

            let jobs: Vec<(String, String, Option<String>, Option<i64>, String, Option<i64>)> = stmt
                .query_map(params![now], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?;

            jobs
        };

        for (job_id, event_id, expression, scheduled_for, _timezone, _) in jobs {
            tracing::info!("Firing cron job: {} -> event {}", job_id, event_id);

            // Get app_id and offline flag from the registration
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
                if let Some(event_bus_state) = app_handle.try_state::<TauriEventBusState>() {
                    let event_bus = &event_bus_state.0;

                    if let Err(e) = event_bus.push_event(None, app_id, event_id.clone(), offline) {
                        tracing::error!("Failed to push cron event to EventBus: {}", e);
                        continue;
                    }

                    tracing::info!("Cron event {} triggered successfully (offline: {})", event_id, offline);
                } else {
                    tracing::error!("EventBus state not available for cron job {}", job_id);
                    continue;
                }
            } else {
                tracing::error!("Could not find registration info for cron job {}", job_id);
                continue;
            }

            let next_run = if let Some(expr) = expression {
                if let Ok(schedule) = Schedule::from_str(&expr) {
                    schedule.upcoming(Utc).next().map(|dt| dt.timestamp())
                } else {
                    None
                }
            } else if scheduled_for.is_some() {
                None
            } else {
                None
            };

            let conn = db.lock().unwrap();
            conn.execute(
                "UPDATE cron_jobs SET last_fired = ?1, next_run = ?2 WHERE id = ?3",
                params![now, next_run, job_id],
            )?;
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl EventSink for CronSink {
    async fn start(&self, app_handle: &AppHandle, db: DbConnection) -> Result<()> {
        Self::init_tables(&db)?;

        let app_handle = app_handle.clone();
        let running = Arc::new(AtomicBool::new(true));

        // Spawn worker thread
        let worker_running = running.clone();
        let worker_db = db.clone();
        tokio::spawn(async move {
            tracing::info!("Cron worker thread started");

            while worker_running.load(Ordering::Relaxed) {
                if let Err(e) = Self::process_jobs(&worker_db, &app_handle).await {
                    tracing::error!("Error processing cron jobs: {}", e);
                }

                tokio::time::sleep(Duration::from_secs(1)).await;
            }

            tracing::info!("Cron worker thread stopped");
        });

        tracing::info!("Cron sink started - worker ready");
        Ok(())
    }

    async fn stop(&self, _app_handle: &AppHandle, _db: DbConnection) -> Result<()> {
        tracing::info!("Cron sink stopped");
        Ok(())
    }

    async fn on_register(&self, _app_handle: &AppHandle, registration: &EventRegistration, db: DbConnection) -> Result<()> {
        Self::add_job(&db, registration, self)?;

        if let Some(expr) = &self.expression {
            tracing::info!("Registered cron job: {} -> event {}", expr, registration.event_id);
        } else if let Some(time) = &self.scheduled_for {
            tracing::info!("Registered scheduled job: {} -> event {}", time, registration.event_id);
        }

        Ok(())
    }

    async fn on_unregister(&self, _app_handle: &AppHandle, _registration: &EventRegistration, db: DbConnection) -> Result<()> {
        Self::remove_job(&db, &self.id)?;
        tracing::info!("Unregistered cron job: {}", self.id);
        Ok(())
    }
}