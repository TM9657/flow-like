//! Execution state store abstraction
//!
//! This module provides different backends for storing execution state and events:
//! - **PostgreSQL**: Via Prisma/SeaORM - reliable, supports complex queries
//! - **Redis**: Fast, with native TTL support - good for high-throughput
//! - **DynamoDB**: Serverless, with TTL + FlowLikeStore for large payloads - good for AWS
//! - **Object Storage**: S3/R2/GCS - for large payloads and archival
//!
//! ## Backend Selection
//!
//! | Backend | Latency | Scalability | TTL | Best For |
//! |---------|---------|-------------|-----|----------|
//! | PostgreSQL | Medium | Vertical | Manual | Full-featured, complex queries |
//! | Redis | Low | Horizontal | Native | High-throughput, real-time |
//! | DynamoDB | Low | Horizontal | Native | Serverless, AWS-native |
//! | Object Storage | High | Infinite | Lifecycle | Large payloads, archival |
//!
//! ## Recommended Configuration
//!
//! | Deployment | Backend | Reason |
//! |------------|---------|--------|
//! | AWS Lambda/ECS | `dynamodb` | Native TTL, serverless, auto-scaling, FlowLikeStore for large payloads |
//! | Kubernetes | `redis` | Fast, native TTL, already deployed in cluster |
//! | Docker Compose | `redis` | Simple setup, native TTL |
//!
//! ## Configuration
//!
//! ```bash
//! # Select backend
//! EXECUTION_STATE_BACKEND=dynamodb  # postgres, redis, dynamodb, s3
//!
//! # PostgreSQL (default, requires manual TTL cleanup)
//! DATABASE_URL=postgres://...
//!
//! # Redis (Kubernetes/Docker)
//! REDIS_URL=redis://...
//! EXECUTION_STATE_TTL_SECONDS=86400  # 24 hours
//!
//! # DynamoDB (AWS - recommended, reuses cdn_bucket from AppState)
//! DYNAMODB_TABLE_PREFIX=flowlike-  # optional
//! # Reuses cdn_bucket (FlowLikeStore) from AppState for large payloads
//! # Fallback: CDN_BUCKET_NAME env var when AppState not available
//!
//! # Object Storage (for large payloads)
//! EXECUTION_PAYLOAD_BUCKET=flow-like-execution-payloads
//! ```
//!
//! ## Large Payload Handling (DynamoDB)
//!
//! DynamoDB has a 400KB item limit. Payloads larger than 100KB are automatically
//! stored via FlowLikeStore under `polling/{run_id}/{event_id}.json` and referenced in DynamoDB.
//! Uses the cdn_bucket from AppState when available, avoiding duplicate client construction.

mod postgres;
mod types;

#[cfg(feature = "redis")]
mod redis;

#[cfg(feature = "dynamodb")]
mod dynamodb;

#[cfg(feature = "s3")]
mod object_storage;

pub use postgres::PostgresStateStore;
pub use types::*;

#[cfg(feature = "redis")]
pub use redis::RedisStateStore;

#[cfg(feature = "dynamodb")]
pub use dynamodb::DynamoDbStateStore;

#[cfg(feature = "s3")]
pub use object_storage::ObjectStorageStateStore;

use std::sync::Arc;

#[cfg(feature = "aws")]
use aws_config::SdkConfig;

#[cfg(any(feature = "dynamodb", feature = "s3"))]
use flow_like_storage::files::store::FlowLikeStore;

/// Backend type for execution state storage
#[derive(Clone, Debug, Default)]
pub enum StateBackend {
    #[default]
    Postgres,
    #[cfg(feature = "redis")]
    Redis,
    #[cfg(feature = "dynamodb")]
    DynamoDB,
    #[cfg(feature = "s3")]
    ObjectStorage,
}

impl StateBackend {
    pub fn from_env() -> Self {
        match std::env::var("EXECUTION_STATE_BACKEND")
            .unwrap_or_default()
            .to_lowercase()
            .as_str()
        {
            #[cfg(feature = "redis")]
            "redis" => Self::Redis,
            #[cfg(feature = "dynamodb")]
            "dynamodb" | "dynamo" => Self::DynamoDB,
            #[cfg(feature = "s3")]
            "s3" | "object_storage" | "objectstorage" => Self::ObjectStorage,
            _ => Self::Postgres,
        }
    }
}

/// Configuration for creating state stores
#[derive(Default)]
pub struct StateStoreConfig {
    pub db: Option<Arc<sea_orm::DatabaseConnection>>,
    #[cfg(feature = "aws")]
    pub aws_config: Option<Arc<SdkConfig>>,
    #[cfg(feature = "dynamodb")]
    pub content_store: Option<Arc<FlowLikeStore>>,
    #[cfg(feature = "s3")]
    pub meta_store: Option<Arc<FlowLikeStore>>,
}

impl StateStoreConfig {
    pub fn with_db(mut self, db: Arc<sea_orm::DatabaseConnection>) -> Self {
        self.db = Some(db);
        self
    }

    #[cfg(feature = "aws")]
    pub fn with_aws_config(mut self, config: Arc<SdkConfig>) -> Self {
        self.aws_config = Some(config);
        self
    }

    #[cfg(feature = "dynamodb")]
    pub fn with_content_store(mut self, store: Arc<FlowLikeStore>) -> Self {
        self.content_store = Some(store);
        self
    }

    #[cfg(feature = "s3")]
    pub fn with_meta_store(mut self, store: Arc<FlowLikeStore>) -> Self {
        self.meta_store = Some(store);
        self
    }
}

/// Create a state store based on environment configuration
pub async fn create_state_store(
    config: StateStoreConfig,
) -> Result<Arc<dyn ExecutionStateStore>, types::StateStoreError> {
    let backend = StateBackend::from_env();

    match backend {
        StateBackend::Postgres => {
            let db = config.db.ok_or_else(|| {
                types::StateStoreError::Configuration(
                    "Database connection required for Postgres backend".into(),
                )
            })?;
            Ok(Arc::new(PostgresStateStore::new(db)))
        }

        #[cfg(feature = "redis")]
        StateBackend::Redis => {
            let store = RedisStateStore::from_env().await?;
            Ok(Arc::new(store))
        }

        #[cfg(feature = "dynamodb")]
        StateBackend::DynamoDB => {
            // Prefer using provided AWS config and content store from AppState
            match (config.aws_config, config.content_store) {
                (Some(aws_cfg), Some(store)) => {
                    Ok(Arc::new(DynamoDbStateStore::new(&aws_cfg, store)))
                }
                _ => {
                    // Fallback to environment configuration
                    let store = DynamoDbStateStore::from_env().await?;
                    Ok(Arc::new(store))
                }
            }
        }

        #[cfg(feature = "s3")]
        StateBackend::ObjectStorage => match config.meta_store {
            Some(store) => Ok(Arc::new(ObjectStorageStateStore::new(store))),
            None => {
                let store = ObjectStorageStateStore::from_env().await?;
                Ok(Arc::new(store))
            }
        },
    }
}
