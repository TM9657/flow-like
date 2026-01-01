use crate::config::Config;
use flow_like_storage::files::store::FlowLikeStore;
use flow_like_storage::object_store::{
    aws::AmazonS3Builder,
    azure::{AzureConfigKey, MicrosoftAzureBuilder},
    gcp::GoogleCloudStorageBuilder,
};
use std::sync::Arc;

pub fn create_content_store(config: &Config) -> Result<FlowLikeStore, StorageError> {
    match &config.storage_config {
        crate::config::StorageConfig::Aws(cfg) => build_aws_store(cfg),
        crate::config::StorageConfig::Azure(cfg) => build_azure_store(cfg),
        crate::config::StorageConfig::Gcp(cfg) => build_gcp_store(cfg),
        crate::config::StorageConfig::R2(cfg) => build_r2_store(cfg),
        crate::config::StorageConfig::S3(cfg) => build_s3_store(cfg),
    }
}

fn build_aws_store(cfg: &crate::config::AwsStorageConfig) -> Result<FlowLikeStore, StorageError> {
    use flow_like_storage::object_store::aws::AmazonS3ConfigKey;

    let mut builder = AmazonS3Builder::new()
        .with_region(&cfg.region)
        .with_bucket_name(&cfg.content_bucket);

    if let Some(endpoint) = &cfg.endpoint {
        builder = builder.with_endpoint(endpoint);
    }

    if let (Some(access_key), Some(secret_key)) = (&cfg.access_key_id, &cfg.secret_access_key) {
        builder = builder
            .with_access_key_id(access_key)
            .with_secret_access_key(secret_key);
    }

    if cfg.use_path_style {
        builder = builder.with_config(AmazonS3ConfigKey::VirtualHostedStyleRequest, "false");
    }

    let store = builder
        .build()
        .map_err(|e| StorageError::Build(format!("AWS S3: {}", e)))?;
    Ok(FlowLikeStore::AWS(Arc::new(store)))
}

fn build_azure_store(cfg: &crate::config::AzureStorageConfig) -> Result<FlowLikeStore, StorageError> {
    let mut builder = MicrosoftAzureBuilder::new()
        .with_account(&cfg.account_name)
        .with_container_name(&cfg.content_container);

    if let Some(key) = &cfg.account_key {
        builder = builder.with_config(AzureConfigKey::AccessKey, key);
    }

    let store = builder
        .build()
        .map_err(|e| StorageError::Build(format!("Azure: {}", e)))?;
    Ok(FlowLikeStore::Azure(Arc::new(store)))
}

fn build_gcp_store(cfg: &crate::config::GcpStorageConfig) -> Result<FlowLikeStore, StorageError> {
    let mut builder = GoogleCloudStorageBuilder::new().with_bucket_name(&cfg.content_bucket);

    if let Some(key) = &cfg.service_account_key {
        builder = builder.with_service_account_key(key);
    }

    let store = builder
        .build()
        .map_err(|e| StorageError::Build(format!("GCP: {}", e)))?;
    Ok(FlowLikeStore::Google(Arc::new(store)))
}

fn build_r2_store(cfg: &crate::config::R2StorageConfig) -> Result<FlowLikeStore, StorageError> {
    use flow_like_storage::object_store::aws::AmazonS3ConfigKey;

    let endpoint = format!("https://{}.r2.cloudflarestorage.com", cfg.account_id);

    let builder = AmazonS3Builder::new()
        .with_bucket_name(&cfg.content_bucket)
        .with_endpoint(&endpoint)
        .with_region("auto")
        .with_access_key_id(&cfg.access_key_id)
        .with_secret_access_key(&cfg.secret_access_key)
        .with_config(AmazonS3ConfigKey::VirtualHostedStyleRequest, "false");

    let store = builder
        .build()
        .map_err(|e| StorageError::Build(format!("R2: {}", e)))?;
    Ok(FlowLikeStore::AWS(Arc::new(store)))
}

fn build_s3_store(cfg: &crate::config::S3StorageConfig) -> Result<FlowLikeStore, StorageError> {
    use flow_like_storage::object_store::aws::AmazonS3ConfigKey;

    let mut builder = AmazonS3Builder::new()
        .with_bucket_name(&cfg.content_bucket)
        .with_endpoint(&cfg.endpoint)
        .with_region(&cfg.region)
        .with_access_key_id(&cfg.access_key_id)
        .with_secret_access_key(&cfg.secret_access_key);

    if cfg.use_path_style {
        builder = builder.with_config(AmazonS3ConfigKey::VirtualHostedStyleRequest, "false");
    }

    let store = builder
        .build()
        .map_err(|e| StorageError::Build(format!("S3: {}", e)))?;
    Ok(FlowLikeStore::AWS(Arc::new(store)))
}

#[derive(Debug)]
pub enum StorageError {
    Build(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::Build(msg) => write!(f, "Failed to build storage: {}", msg),
        }
    }
}

impl std::error::Error for StorageError {}
