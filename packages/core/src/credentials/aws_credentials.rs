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

#[cfg(test)]
mod tests {
    use super::*;
    use flow_like_types::json::{from_str, to_string};

    fn sample_credentials() -> AwsSharedCredentials {
        AwsSharedCredentials {
            access_key_id: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
            secret_access_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
            session_token: Some("FwoGZXIvYXdzEBYaDJ...".to_string()),
            meta_bucket: "my-meta-bucket--usw2-az1--x-s3".to_string(),
            content_bucket: "my-content-bucket".to_string(),
            region: "us-west-2".to_string(),
            expiration: None,
        }
    }

    #[test]
    fn test_aws_credentials_serialization() {
        let creds = sample_credentials();
        let json = to_string(&creds).expect("Failed to serialize");

        assert!(json.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(json.contains("my-meta-bucket--usw2-az1--x-s3"));
        assert!(json.contains("my-content-bucket"));
        assert!(json.contains("us-west-2"));
    }

    #[test]
    fn test_aws_credentials_deserialization() {
        let json = r#"{
            "access_key_id": "AKIAIOSFODNN7EXAMPLE",
            "secret_access_key": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            "session_token": "FwoGZXIvYXdzEBYaDJ...",
            "meta_bucket": "test-meta",
            "content_bucket": "test-content",
            "region": "eu-west-1",
            "expiration": null
        }"#;

        let creds: AwsSharedCredentials = from_str(json).expect("Failed to deserialize");

        assert_eq!(
            creds.access_key_id,
            Some("AKIAIOSFODNN7EXAMPLE".to_string())
        );
        assert_eq!(creds.meta_bucket, "test-meta");
        assert_eq!(creds.content_bucket, "test-content");
        assert_eq!(creds.region, "eu-west-1");
        assert!(creds.expiration.is_none());
    }

    #[test]
    fn test_aws_credentials_roundtrip() {
        let original = sample_credentials();
        let json = to_string(&original).expect("Failed to serialize");
        let deserialized: AwsSharedCredentials = from_str(&json).expect("Failed to deserialize");

        assert_eq!(original.access_key_id, deserialized.access_key_id);
        assert_eq!(original.secret_access_key, deserialized.secret_access_key);
        assert_eq!(original.session_token, deserialized.session_token);
        assert_eq!(original.meta_bucket, deserialized.meta_bucket);
        assert_eq!(original.content_bucket, deserialized.content_bucket);
        assert_eq!(original.region, deserialized.region);
    }

    #[test]
    fn test_aws_credentials_with_expiration() {
        let json = r#"{
            "access_key_id": "AKIAIOSFODNN7EXAMPLE",
            "secret_access_key": "secret",
            "session_token": "token",
            "meta_bucket": "meta",
            "content_bucket": "content",
            "region": "us-east-1",
            "expiration": "2025-01-15T12:00:00Z"
        }"#;

        let creds: AwsSharedCredentials = from_str(json).expect("Failed to deserialize");
        assert!(creds.expiration.is_some());
    }

    #[test]
    fn test_aws_credentials_optional_fields() {
        let json = r#"{
            "access_key_id": null,
            "secret_access_key": null,
            "session_token": null,
            "meta_bucket": "meta",
            "content_bucket": "content",
            "region": "us-east-1",
            "expiration": null
        }"#;

        let creds: AwsSharedCredentials = from_str(json).expect("Failed to deserialize");
        assert!(creds.access_key_id.is_none());
        assert!(creds.secret_access_key.is_none());
        assert!(creds.session_token.is_none());
    }
}
