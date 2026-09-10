//! Kubernetes configuration for job execution

use serde::{Deserialize, Serialize};

const DEFAULT_EXECUTOR_IMAGE: &str = "ghcr.io/rheosoph/flow-like-kubernetes-executor:dev";

/// Configuration for Kubernetes job execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KubernetesConfig {
    /// Kubernetes namespace for executor jobs
    pub namespace: String,
    /// Docker image for the executor
    pub executor_image: String,
    /// Names of `kubernetes.io/dockerconfigjson` secrets the executor pods pull with
    #[serde(default)]
    pub image_pull_secrets: Vec<String>,
    /// Runtime class for isolation (e.g., "kata" for Kata Containers)
    pub runtime_class: Option<String>,
    /// Job timeout in seconds
    pub job_timeout_seconds: u64,
    /// Maximum retries for failed jobs
    pub job_max_retries: u32,
    /// URL for warm executor pool (optional)
    pub executor_pool_url: Option<String>,
    /// Memory request for executor pods
    pub memory_request: String,
    /// Memory limit for executor pods
    pub memory_limit: String,
    /// CPU request for executor pods
    pub cpu_request: String,
    /// CPU limit for executor pods
    pub cpu_limit: String,
}

impl Default for KubernetesConfig {
    fn default() -> Self {
        Self {
            namespace: "flow-like".to_string(),
            executor_image: DEFAULT_EXECUTOR_IMAGE.to_string(),
            image_pull_secrets: Vec::new(),
            runtime_class: None,
            job_timeout_seconds: 3600,
            job_max_retries: 3,
            executor_pool_url: None,
            memory_request: "256Mi".to_string(),
            memory_limit: "2Gi".to_string(),
            cpu_request: "100m".to_string(),
            cpu_limit: "2".to_string(),
        }
    }
}

/// Split a comma-separated list of secret names, dropping blanks.
pub fn parse_image_pull_secrets(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
}

impl KubernetesConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            namespace: std::env::var("K8S_NAMESPACE").unwrap_or_else(|_| "flow-like".to_string()),
            executor_image: std::env::var("K8S_EXECUTOR_IMAGE")
                .unwrap_or_else(|_| DEFAULT_EXECUTOR_IMAGE.to_string()),
            image_pull_secrets: std::env::var("K8S_IMAGE_PULL_SECRETS")
                .map(|raw| parse_image_pull_secrets(&raw))
                .unwrap_or_default(),
            runtime_class: std::env::var("K8S_RUNTIME_CLASS").ok(),
            job_timeout_seconds: std::env::var("K8S_JOB_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600),
            job_max_retries: std::env::var("K8S_JOB_MAX_RETRIES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
            executor_pool_url: std::env::var("K8S_EXECUTOR_POOL_URL").ok(),
            memory_request: std::env::var("K8S_MEMORY_REQUEST")
                .unwrap_or_else(|_| "256Mi".to_string()),
            memory_limit: std::env::var("K8S_MEMORY_LIMIT").unwrap_or_else(|_| "2Gi".to_string()),
            cpu_request: std::env::var("K8S_CPU_REQUEST").unwrap_or_else(|_| "100m".to_string()),
            cpu_limit: std::env::var("K8S_CPU_LIMIT").unwrap_or_else(|_| "2".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_executor_image_is_the_published_dev_tag() {
        assert_eq!(
            KubernetesConfig::default().executor_image,
            DEFAULT_EXECUTOR_IMAGE
        );
        assert_eq!(
            DEFAULT_EXECUTOR_IMAGE,
            "ghcr.io/rheosoph/flow-like-kubernetes-executor:dev"
        );
    }

    #[test]
    fn image_pull_secrets_are_trimmed_and_blank_entries_dropped() {
        assert_eq!(
            parse_image_pull_secrets(" ghcr-pull , , mirror-pull ,"),
            vec!["ghcr-pull".to_string(), "mirror-pull".to_string()]
        );
        assert!(parse_image_pull_secrets("").is_empty());
        assert!(parse_image_pull_secrets(" , ").is_empty());
    }
}
