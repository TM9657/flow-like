use crate::credentials::SharedCredentialsTrait;
use flow_like_storage::lancedb::connection::ConnectBuilder;
use flow_like_storage::object_store::aws::AmazonS3Builder;
use flow_like_storage::{Path, object_store};
use flow_like_storage::{files::store::FlowLikeStore, lancedb};
use flow_like_types::{Result, anyhow, async_trait};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AwsSharedCredentials {
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub session_token: Option<String>,
    pub meta_bucket: String,
    pub content_bucket: String,
    pub region: String,
    pub expiration: Option<chrono::DateTime<chrono::Utc>>,
}

#[async_trait]
impl SharedCredentialsTrait for AwsSharedCredentials {
    #[tracing::instrument(name = "AwsSharedCredentials::to_store", skip(self, meta), fields(meta = meta), level="debug")]
    async fn to_store(&self, meta: bool) -> Result<FlowLikeStore> {
        use flow_like_types::tokio;

        let builder = {
            let mut builder = AmazonS3Builder::new()
                .with_access_key_id(
                    self.access_key_id
                        .clone()
                        .ok_or(anyhow!("AWS_ACCESS_KEY_ID is not set"))?,
                )
                .with_secret_access_key(
                    self.secret_access_key
                        .clone()
                        .ok_or(anyhow!("AWS_SECRET_ACCESS_KEY is not set"))?,
                )
                .with_token(
                    self.session_token
                        .clone()
                        .ok_or(anyhow!("SESSION TOKEN is not set"))?,
                )
                .with_bucket_name(if meta {
                    &self.meta_bucket
                } else {
                    &self.content_bucket
                })
                .with_region(&self.region);

            if meta {
                builder = builder.with_s3_express(true);
            }
            builder
        };

        let store = tokio::task::spawn_blocking(move || builder.build())
            .await
            .map_err(|e| anyhow!("Failed to spawn blocking task: {}", e))??;
        Ok(FlowLikeStore::AWS(Arc::new(store)))
    }

    #[tracing::instrument(name = "AwsSharedCredentials::to_db", skip(self), level = "debug")]
    async fn to_db(&self, app_id: &str) -> Result<ConnectBuilder> {
        let path = Path::from("apps")
            .child(app_id)
            .child("storage")
            .child("db");
        let connection = make_s3_builder(
            self.content_bucket.clone(),
            self.access_key_id
                .clone()
                .ok_or(anyhow!("AWS_ACCESS_KEY_ID is not set"))?,
            self.secret_access_key
                .clone()
                .ok_or(anyhow!("AWS_SECRET_ACCESS_KEY is not set"))?,
            self.session_token
                .clone()
                .ok_or(anyhow!("SESSION TOKEN is not set"))?,
        );
        let connection = connection(path.clone());
        Ok(connection)
    }
}

fn make_s3_builder(
    bucket: String,
    access_key: String,
    secret_key: String,
    session_token: String,
) -> impl Fn(object_store::path::Path) -> ConnectBuilder {
    move |path| {
        let url = format!("s3://{}/{}", bucket, path);
        lancedb::connect(&url)
            .storage_option("aws_access_key_id".to_string(), access_key.clone())
            .storage_option("aws_secret_access_key".to_string(), secret_key.clone())
            .storage_option("aws_session_token".to_string(), session_token.clone())
    }
}
