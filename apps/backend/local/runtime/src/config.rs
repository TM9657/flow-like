use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub max_concurrent_executions: usize,
    /// Enable queue worker mode (polls Redis for jobs)
    pub queue_worker_enabled: bool,
    /// Redis URL for queue worker
    pub redis_url: Option<String>,
    /// Redis queue name
    pub redis_queue_name: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Config {
            port: env::var("RUNTIME_PORT")
                .or_else(|_| env::var("PORT"))
                .unwrap_or_else(|_| "9000".to_string())
                .parse()
                .map_err(|_| ConfigError::InvalidValue("RUNTIME_PORT".to_string()))?,
            max_concurrent_executions: env::var("MAX_CONCURRENT_EXECUTIONS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .map_err(|_| ConfigError::InvalidValue("MAX_CONCURRENT_EXECUTIONS".to_string()))?,
            queue_worker_enabled: env::var("QUEUE_WORKER_ENABLED")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            redis_url: env::var("REDIS_URL").ok(),
            redis_queue_name: env::var("REDIS_EXECUTION_QUEUE")
                .unwrap_or_else(|_| "exec:jobs".to_string()),
        })
    }
}

#[derive(Debug)]
pub enum ConfigError {
    InvalidValue(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::InvalidValue(var) => write!(f, "Invalid value for: {}", var),
        }
    }
}

impl std::error::Error for ConfigError {}
