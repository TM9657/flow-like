use anyhow::Result;
use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use cron::Schedule;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::Duration;
use tauri::{AppHandle, Manager};

use super::{EventRegistration, EventSink, manager::DbConnection};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledLocal {
    pub date: String, // "YYYY-MM-DD"
    pub time: String, // "HH:mm"
}

impl ScheduledLocal {
    fn to_utc_timestamp(&self, tz: Tz) -> Option<i64> {
        let date = NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").ok()?;
        let time = NaiveTime::parse_from_str(&self.time, "%H:%M").ok()?;
        let naive_dt = date.and_time(time);
        let tz_dt = tz.from_local_datetime(&naive_dt).single()?;
        let utc_dt = tz_dt.with_timezone(&Utc);
        Some(utc_dt.timestamp())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronSink {
    pub expression: Option<String>,
    pub scheduled_for: Option<ScheduledLocal>,
    pub last_fired: Option<String>,
    pub timezone: Option<String>,
}

impl CronSink {
    fn init_tables(db: &DbConnection) -> Result<()> {
        let conn = db.lock().unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS cron_jobs (
                event_id      TEXT PRIMARY KEY,
                expression    TEXT,
                scheduled_for INTEGER,
                timezone      TEXT NOT NULL,
                last_fired    INTEGER,
                next_run      INTEGER,
                created_at    INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_cron_next_run ON cron_jobs(next_run)",
            [],
        )?;

        Ok(())
    }

    fn parse_tz(tz: Option<&str>) -> Tz {
        tz.and_then(|s| s.parse::<Tz>().ok())
            .unwrap_or(chrono_tz::UTC)
    }

    fn compute_next_from_cron(expr: &str, tz: Tz) -> Option<i64> {
        let expr_with_seconds = if expr.split_whitespace().count() == 5 {
            format!("0 {}", expr)
        } else {
            expr.to_string()
        };

        match Schedule::from_str(&expr_with_seconds) {
            Ok(schedule) => {
                let mut upcoming = schedule.upcoming(tz);
                let next = upcoming.next();

                match next {
                    Some(dt) => {
                        let utc_dt = dt.with_timezone(&Utc);
                        let timestamp = utc_dt.timestamp();
                        Some(timestamp)
                    }
                    None => None,
                }
            }
            Err(e) => {
                tracing::error!(
                    "Failed to parse cron expression '{}': {}",
                    expr_with_seconds,
                    e
                );
                None
            }
        }
    }

    fn add_job(
        db: &DbConnection,
        registration: &EventRegistration,
        config: &CronSink,
    ) -> Result<()> {
        println!("🔧 [CRON] Adding cron job for event_id: {}", registration.event_id);
        tracing::info!("Adding cron job for event_id: {}", registration.event_id);
        println!("🔧 [CRON] Config: expression={:?}, scheduled_for={:?}, timezone={:?}",
            config.expression,
            config.scheduled_for,
            config.timezone
        );
        tracing::debug!(
            "Config: expression={:?}, scheduled_for={:?}, timezone={:?}",
            config.expression,
            config.scheduled_for,
            config.timezone
        );

        let conn = db.lock().unwrap();
        let now = Utc::now().timestamp();
        let tz = Self::parse_tz(config.timezone.as_deref());
        println!("🔧 [CRON] Current timestamp: {}, timezone: {:?}", now, tz);

        let expression = config
            .expression
            .as_ref()
            .map(|e| e.trim())
            .filter(|e| !e.is_empty())
            .map(|e| e.to_string());

        let scheduled_for_ts = if expression.is_none() {
            config
                .scheduled_for
                .as_ref()
                .and_then(|sl| sl.to_utc_timestamp(tz))
        } else {
            None
        };

        let last_fired_ts = config
            .last_fired
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.timestamp());

        let next_run = if let Some(ref expr) = expression {
            let computed = Self::compute_next_from_cron(expr.trim(), tz);
            println!("🔧 [CRON] Computed next_run from expression '{}': {:?}", expr, computed);
            computed
        } else {
            println!("🔧 [CRON] Using scheduled_for_ts: {:?}", scheduled_for_ts);
            scheduled_for_ts
        };

        println!("🔧 [CRON] Final next_run: {:?} for event_id: {}", next_run, registration.event_id);
        tracing::info!(
            "Calculated next_run: {:?} for event_id: {}",
            next_run,
            registration.event_id
        );

        conn.execute(
            "INSERT OR REPLACE INTO cron_jobs
             (event_id, expression, scheduled_for, timezone, last_fired, next_run, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                registration.event_id,
                expression,
                scheduled_for_ts,
                config.timezone.as_deref().unwrap_or("UTC"),
                last_fired_ts,
                next_run,
                now,
            ],
        )?;

        println!("✅ [CRON] Successfully inserted cron job for event_id: {}", registration.event_id);
        tracing::info!(
            "Successfully inserted cron job for event_id: {}",
            registration.event_id
        );
        Ok(())
    }

    fn remove_job(db: &DbConnection, event_id: &str) -> Result<()> {
        let conn = db.lock().unwrap();
        conn.execute(
            "DELETE FROM cron_jobs WHERE event_id = ?1",
            params![event_id],
        )?;
        Ok(())
    }

    fn calculate_missing_next_runs(db: &DbConnection) -> Result<()> {
        let conn = db.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT event_id, expression, scheduled_for, timezone
               FROM cron_jobs
              WHERE next_run IS NULL",
        )?;

        let jobs: Vec<(String, Option<String>, Option<i64>, String)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        drop(stmt);

        tracing::debug!("Found {} jobs with NULL next_run", jobs.len());

        for (event_id, expression, scheduled_for, tz_str) in jobs {
            let tz = Self::parse_tz(Some(&tz_str));

            let next_run = if let Some(expr) = expression.as_ref().filter(|e| !e.trim().is_empty())
            {
                Self::compute_next_from_cron(expr.trim(), tz)
            } else {
                scheduled_for
            };

            if let Some(ts) = next_run {
                tracing::debug!("Updating event_id {} with next_run: {}", event_id, ts);
                conn.execute(
                    "UPDATE cron_jobs SET next_run = ?1 WHERE event_id = ?2",
                    params![ts, event_id],
                )?;
            } else {
                tracing::warn!(
                    "Deleting event_id {} - no valid next_run could be calculated",
                    event_id
                );
                conn.execute(
                    "DELETE FROM cron_jobs WHERE event_id = ?1",
                    params![event_id],
                )?;
            }
        }

        Ok(())
    }

    fn get_due_jobs(
        db: &DbConnection,
        now: i64,
    ) -> Result<Vec<(String, Option<String>, Option<i64>, String)>> {
        let conn = db.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT event_id, expression, scheduled_for, timezone
               FROM cron_jobs
              WHERE next_run IS NOT NULL AND next_run <= ?1
           ORDER BY next_run ASC
              LIMIT 64",
        )?;

        stmt.query_map(params![now], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(Into::into)
    }

    fn fire_event(app_handle: &AppHandle, event_id: &str) -> Result<bool> {
        use crate::state::TauriEventSinkManagerState;

        println!("🔥 [CRON] fire_event called for: {}", event_id);
        if let Some(manager_state) = app_handle.try_state::<TauriEventSinkManagerState>() {
            println!("🔥 [CRON] EventSinkManager state found, attempting to acquire lock...");

            // Use try_lock first to avoid potential deadlock
            match manager_state.0.try_lock() {
                Ok(manager) => {
                    println!("✅ [CRON] Lock acquired successfully");
                    let result = manager.fire_event(app_handle, event_id, None);
                    println!("🔥 [CRON] fire_event result: {:?}", result);
                    result
                }
                Err(_) => {
                    println!("⚠️ [CRON] Could not acquire lock immediately, using blocking_lock...");
                    let manager = manager_state.0.blocking_lock();
                    println!("✅ [CRON] blocking_lock acquired");
                    let result = manager.fire_event(app_handle, event_id, None);
                    println!("🔥 [CRON] fire_event result: {:?}", result);
                    result
                }
            }
        } else {
            println!("❌ [CRON] EventSinkManager state not available for {}", event_id);
            tracing::error!("EventSinkManager state not available for {}", event_id);
            Ok(false)
        }
    }

    fn handle_executed_job(
        db: &DbConnection,
        event_id: &str,
        expression: Option<String>,
        tz: Tz,
        now: i64,
    ) -> Result<()> {
        println!("🔄 [CRON] handle_executed_job for: {} (expression: {:?})", event_id, expression);
        let conn = db.lock().unwrap();

        if let Some(expr) = expression.filter(|e| !e.trim().is_empty()) {
            if let Some(next_ts) = Self::compute_next_from_cron(expr.trim(), tz) {
                println!("🔄 [CRON] Updating job {} with next_run: {}", event_id, next_ts);
                conn.execute(
                    "UPDATE cron_jobs SET last_fired = ?1, next_run = ?2 WHERE event_id = ?3",
                    params![now, next_ts, event_id],
                )?;
            } else {
                println!("🗑️ [CRON] No next run found, deleting job: {}", event_id);
                conn.execute(
                    "DELETE FROM cron_jobs WHERE event_id = ?1",
                    params![event_id],
                )?;
            }
        } else {
            println!("🗑️ [CRON] One-time job completed, deleting: {}", event_id);
            conn.execute(
                "DELETE FROM cron_jobs WHERE event_id = ?1",
                params![event_id],
            )?;
        }

        Ok(())
    }

    fn get_next_upcoming(db: &DbConnection) -> Option<i64> {
        let conn = db.lock().unwrap();
        conn.query_row(
            "SELECT MIN(next_run) FROM cron_jobs WHERE next_run IS NOT NULL",
            [],
            |row| row.get::<_, Option<i64>>(0),
        )
        .unwrap_or(None)
    }

    async fn process_jobs(db: &DbConnection, app_handle: &AppHandle) -> Result<Option<i64>> {
        println!("⏰ [CRON] Processing jobs cycle started");
        Self::calculate_missing_next_runs(db)?;

        let now = Utc::now().timestamp();
        println!("⏰ [CRON] Current timestamp: {}", now);
        let due_jobs = Self::get_due_jobs(db, now)?;

        println!("⏰ [CRON] Found {} due jobs at timestamp {}", due_jobs.len(), now);
        tracing::debug!("Found {} due jobs at timestamp {}", due_jobs.len(), now);

        for (event_id, expression, _scheduled_for, tz_str) in due_jobs {
            let tz = Self::parse_tz(Some(&tz_str));

            println!("🔥 [CRON] Firing event: {} (expression: {:?})", event_id, expression);
            tracing::info!("Firing event: {}", event_id);

            match Self::fire_event(app_handle, &event_id) {
                Ok(true) => {
                    println!("✅ [CRON] Event {} fired successfully", event_id);
                    tracing::info!("Event {} fired successfully", event_id);
                    Self::handle_executed_job(db, &event_id, expression, tz, now)?;
                }
                Ok(false) => {
                    println!("⚠️ [CRON] Event {} failed to fire, will retry in next cycle", event_id);
                    tracing::warn!(
                        "Event {} failed to fire, will retry in next cycle",
                        event_id
                    );
                }
                Err(e) => {
                    println!("❌ [CRON] Error firing event {}: {}", event_id, e);
                    tracing::error!("Error firing event {}: {}", event_id, e);
                }
            }
        }

        let next = Self::get_next_upcoming(db);
        println!("⏰ [CRON] Next upcoming job at: {:?}", next);
        tracing::debug!("Next upcoming job at: {:?}", next);
        Ok(next)
    }
}

#[async_trait::async_trait]
impl EventSink for CronSink {
    async fn start(&self, app_handle: &AppHandle, db: DbConnection) -> Result<()> {
        Self::init_tables(&db)?;

        let app_handle = app_handle.clone();
        let worker_db = db.clone();

        flow_like_types::tokio::spawn(async move {
            println!("🚀 [CRON] Cron worker started");
            tracing::info!("🚀 Cron worker started");

            const MIN_TICK: Duration = Duration::from_millis(250);
            const MAX_TICK: Duration = Duration::from_secs(10);

            loop {
                let next_upcoming = match Self::process_jobs(&worker_db, &app_handle).await {
                    Ok(ts) => ts,
                    Err(e) => {
                        println!("❌ [CRON] Cron processing error: {}", e);
                        tracing::error!("Cron processing error: {}", e);
                        None
                    }
                };

                let now = Utc::now().timestamp();
                let sleep_dur = if let Some(ts) = next_upcoming {
                    if ts <= now {
                        println!("⏰ [CRON] Job is overdue, using MIN_TICK");
                        MIN_TICK
                    } else {
                        let d = Duration::from_secs((ts - now) as u64);
                        let final_dur = d.min(MAX_TICK).max(MIN_TICK);
                        println!("💤 [CRON] Sleeping for {:?} until next job at timestamp {}", final_dur, ts);
                        final_dur
                    }
                } else {
                    println!("💤 [CRON] No upcoming jobs, sleeping for MAX_TICK");
                    MAX_TICK
                };

                flow_like_types::tokio::time::sleep(sleep_dur).await;
            }
        });

        Ok(())
    }

    async fn stop(&self, _app_handle: &AppHandle, _db: DbConnection) -> Result<()> {
        tracing::info!("Cron sink stopped");
        Ok(())
    }

    async fn on_register(
        &self,
        _app_handle: &AppHandle,
        registration: &EventRegistration,
        db: DbConnection,
    ) -> Result<()> {
        println!("📝 [CRON] CronSink::on_register called for event_id: {}", registration.event_id);
        tracing::info!(
            "CronSink::on_register called for event_id: {}",
            registration.event_id
        );

        Self::add_job(&db, registration, self)?;

        println!("✅ [CRON] CronSink::on_register completed for event_id: {}", registration.event_id);
        tracing::info!(
            "CronSink::on_register completed for event_id: {}",
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
        Self::remove_job(&db, &registration.event_id)?;
        Ok(())
    }
}
