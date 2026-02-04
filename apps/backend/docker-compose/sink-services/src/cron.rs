use crate::api_client::{ApiClient, CronScheduleInfo};
use crate::storage::{CronScheduleState, RedisStorage};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub struct CronScheduler {
    api_client: Arc<ApiClient>,
    storage: Option<Arc<RedisStorage>>,
    scheduler: JobScheduler,
    active_jobs: Arc<RwLock<HashMap<String, Uuid>>>,
}

impl CronScheduler {
    pub async fn new(
        api_client: Arc<ApiClient>,
        storage: Option<Arc<RedisStorage>>,
    ) -> Result<Self, CronError> {
        let scheduler = JobScheduler::new()
            .await
            .map_err(|e| CronError::Scheduler(e.to_string()))?;

        Ok(Self {
            api_client,
            storage,
            scheduler,
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn start(&self) -> Result<(), CronError> {
        self.scheduler
            .start()
            .await
            .map_err(|e| CronError::Scheduler(e.to_string()))?;

        info!("Cron scheduler started");
        Ok(())
    }

    pub async fn run_sync_loop(&self) {
        let poll_interval = std::time::Duration::from_secs(30);

        loop {
            match self.sync_schedules().await {
                Ok((added, removed)) => {
                    if added > 0 || removed > 0 {
                        info!(added, removed, "Synced cron schedules");
                    }
                }
                Err(e) => {
                    error!("Failed to sync cron schedules: {}", e);
                }
            }

            tokio::time::sleep(poll_interval).await;
        }
    }

    async fn sync_schedules(&self) -> Result<(usize, usize), CronError> {
        let schedules = self
            .api_client
            .get_cron_schedules()
            .await
            .map_err(|e| CronError::Api(e.to_string()))?;

        let enabled_schedules: HashMap<String, CronScheduleInfo> = schedules
            .into_iter()
            .filter(|s| s.enabled)
            .map(|s| (s.id.clone(), s))
            .collect();

        // Sync to Redis storage if available
        if let Some(ref storage) = self.storage {
            let states: Vec<CronScheduleState> = enabled_schedules
                .values()
                .map(|s| CronScheduleState {
                    event_id: s.event_id.clone(),
                    cron_expression: s.cron_expression.clone(),
                    enabled: s.enabled,
                    last_triggered: s.last_triggered.map(|dt| dt.timestamp()),
                    next_trigger: s.next_trigger.map(|dt| dt.timestamp()),
                })
                .collect();

            if let Err(e) = storage.sync_cron_schedules(states).await {
                warn!("Failed to sync cron schedules to Redis: {}", e);
            }
        }

        let mut added = 0;
        let mut removed = 0;

        let mut active_jobs = self.active_jobs.write().await;

        let current_ids: Vec<String> = active_jobs.keys().cloned().collect();
        for id in current_ids {
            if !enabled_schedules.contains_key(&id) {
                if let Some(job_id) = active_jobs.remove(&id) {
                    if let Err(e) = self.scheduler.remove(&job_id).await {
                        warn!("Failed to remove job {}: {}", id, e);
                    } else {
                        debug!("Removed cron job: {}", id);
                        removed += 1;
                    }
                }
            }
        }

        for (id, schedule) in enabled_schedules {
            if !active_jobs.contains_key(&id) {
                match self.add_job(&schedule).await {
                    Ok(job_id) => {
                        active_jobs.insert(id.clone(), job_id);
                        debug!("Added cron job: {} ({})", id, schedule.cron_expression);
                        added += 1;
                    }
                    Err(e) => {
                        error!("Failed to add cron job {}: {}", id, e);
                    }
                }
            }
        }

        Ok((added, removed))
    }

    async fn add_job(&self, schedule: &CronScheduleInfo) -> Result<Uuid, CronError> {
        let event_id = schedule.event_id.clone();
        let api_client = Arc::clone(&self.api_client);
        let storage = self.storage.clone();
        let schedule_id = schedule.id.clone();

        let job = Job::new_async(schedule.cron_expression.as_str(), move |_uuid, _lock| {
            let event_id = event_id.clone();
            let api_client = Arc::clone(&api_client);
            let storage = storage.clone();
            let schedule_id = schedule_id.clone();

            Box::pin(async move {
                info!(
                    "Triggering cron event: {} (schedule: {})",
                    event_id, schedule_id
                );

                match api_client.trigger_event(&event_id, "cron", None).await {
                    Ok(()) => {
                        info!("Successfully triggered cron event: {}", event_id);

                        // Update last_triggered in Redis
                        if let Some(ref storage) = storage {
                            let now = chrono::Utc::now().timestamp();
                            if let Err(e) =
                                storage.update_cron_last_triggered(&schedule_id, now).await
                            {
                                warn!("Failed to update last_triggered in Redis: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to trigger cron event {}: {}", event_id, e);
                    }
                }
            })
        })
        .map_err(|e| CronError::Job(e.to_string()))?;

        let job_id = self
            .scheduler
            .add(job)
            .await
            .map_err(|e| CronError::Scheduler(e.to_string()))?;

        Ok(job_id)
    }
}

#[derive(Debug)]
pub enum CronError {
    Scheduler(String),
    Api(String),
    Job(String),
}

impl std::fmt::Display for CronError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CronError::Scheduler(e) => write!(f, "Scheduler error: {}", e),
            CronError::Api(e) => write!(f, "API error: {}", e),
            CronError::Job(e) => write!(f, "Job error: {}", e),
        }
    }
}

impl std::error::Error for CronError {}
