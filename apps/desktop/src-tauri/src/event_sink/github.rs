use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubSink {
    pub id: String,
    pub personal_access_token: String,
    pub repository: String,
    pub events: Vec<GitHubEventType>,
    pub branch: Option<String>,
    pub last_event_id: Option<String>,
    pub poll_interval: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GitHubEventType {
    Push,
    PullRequest,
    Issues,
    IssueComment,
    Release,
    Star,
    Fork,
    Watch,
}

// Implementation in stubs.rs
