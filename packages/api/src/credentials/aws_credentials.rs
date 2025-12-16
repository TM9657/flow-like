use super::RuntimeCredentialsTrait;
#[cfg(feature = "aws")]
use crate::credentials::CredentialsAccess;
use crate::state::{AppState, State};
#[cfg(feature = "aws")]
use flow_like::credentials::{SharedCredentials, aws_credentials::AwsSharedCredentials};
use flow_like::{
    flow_like_storage::lancedb::{connect, connection::ConnectBuilder},
    state::{FlowLikeConfig, FlowLikeState},
    utils::http::HTTPClient,
};
use flow_like_storage::object_store;
use flow_like_types::{Result, anyhow, async_trait};
use serde::{Deserialize, Serialize};
use serde_json::{json, to_string};
use std::sync::Arc;

#[cfg(feature = "aws")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AwsRuntimeCredentials {
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub session_token: Option<String>,
    pub meta_bucket: String,
    pub content_bucket: String,
    pub region: String,
    pub expiration: Option<chrono::DateTime<chrono::Utc>>,
}

#[cfg(feature = "aws")]
impl From<aws_sdk_sts::types::Credentials> for AwsRuntimeCredentials {
    fn from(credentials: aws_sdk_sts::types::Credentials) -> Self {
        AwsRuntimeCredentials {
            access_key_id: Some(credentials.access_key_id),
            secret_access_key: Some(credentials.secret_access_key),
            session_token: Some(credentials.session_token),
            meta_bucket: std::env::var("META_BUCKET_NAME").unwrap_or_default(),
            content_bucket: std::env::var("CONTENT_BUCKET_NAME").unwrap_or_default(),
            region: std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
            expiration: None,
        }
    }
}

#[cfg(feature = "aws")]
impl AwsRuntimeCredentials {
    pub fn new(meta_bucket: &str, content_bucket: &str, region: &str) -> Self {
        AwsRuntimeCredentials {
            access_key_id: None,
            secret_access_key: None,
            session_token: None,
            meta_bucket: meta_bucket.to_string(),
            content_bucket: content_bucket.to_string(),
            region: region.to_string(),
            expiration: None,
        }
    }

    pub fn from_env() -> Self {
        AwsRuntimeCredentials {
            access_key_id: std::env::var("AWS_ACCESS_KEY_ID").ok(),
            secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY").ok(),
            session_token: std::env::var("AWS_SESSION_TOKEN").ok(),
            meta_bucket: std::env::var("META_BUCKET_NAME").unwrap_or_default(),
            content_bucket: std::env::var("CONTENT_BUCKET_NAME").unwrap_or_default(),
            region: std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
            expiration: None,
        }
    }

    pub async fn master_credentials(&self) -> Self {
        AwsRuntimeCredentials {
            access_key_id: std::env::var("AWS_ACCESS_KEY_ID").ok(),
            secret_access_key: std::env::var("AWS_SECRET_ACCESS_KEY").ok(),
            session_token: std::env::var("AWS_SESSION_TOKEN").ok(),
            meta_bucket: self.meta_bucket.clone(),
            content_bucket: self.content_bucket.clone(),
            region: self.region.clone(),
            expiration: None,
        }
    }

    #[tracing::instrument(
        name = "AwsRuntimeCredentials::scoped_credentials",
        skip(self, state),
        level = "debug"
    )]
    pub async fn scoped_credentials(
        &self,
        sub: &str,
        app_id: &str,
        state: &State,
        mode: CredentialsAccess,
    ) -> Result<Self> {
        if sub.is_empty() || app_id.is_empty() {
            return Err(flow_like_types::anyhow!("Sub or App ID cannot be empty"));
        }

        let role = std::env::var("RUNTIME_ROLE_ARN").map_err(|_| {
            flow_like_types::anyhow!("RUNTIME_ROLE_ARN environment variable not set")
        })?;

        let client = aws_sdk_sts::Client::new(&state.aws_client);

        let apps_prefix = format!("apps/{}", app_id);
        let user_prefix = format!("users/{}/apps/{}", sub, app_id);
        let log_prefix = format!("logs/runs/{}", app_id);
        let temporary_user_prefix = format!("tmp/user/{}/apps/{}", sub, app_id);
        let temporary_global_prefix = format!("tmp/global/apps/{}", app_id);

        let policy = match mode {
            CredentialsAccess::EditApp => edit_app_policy(self, &apps_prefix),
            CredentialsAccess::ReadApp => read_app_policy(self, &apps_prefix),
            CredentialsAccess::InvokeNone => invoke_none_policy(
                self,
                &apps_prefix,
                &user_prefix,
                &log_prefix,
                &temporary_user_prefix,
            ),
            CredentialsAccess::InvokeRead => invoke_read_policy(
                self,
                &apps_prefix,
                &user_prefix,
                &log_prefix,
                &temporary_user_prefix,
                &temporary_global_prefix,
            ),
            CredentialsAccess::InvokeWrite => invoke_read_write_policy(
                self,
                &apps_prefix,
                &user_prefix,
                &log_prefix,
                &temporary_user_prefix,
                &temporary_global_prefix,
            ),
            CredentialsAccess::ReadLogs => read_logs_policy(self, &log_prefix),
        };

        let policy = to_string(&policy)
            .map_err(|e| flow_like_types::anyhow!("Failed to serialize policy: {}", e))?;

        let credentials = client
            .assume_role()
            .role_arn(role)
            .role_session_name(format!("{}-{}", sub, app_id))
            .policy(policy)
            .duration_seconds(3600) // 1 hour
            .send()
            .await?;

        let chrono_expiration = chrono::Utc::now() + chrono::Duration::hours(1);

        Ok(Self {
            access_key_id: credentials
                .credentials()
                .map(|c| c.access_key_id().to_string()),
            secret_access_key: credentials
                .credentials()
                .map(|c| c.secret_access_key().to_string()),
            session_token: credentials
                .credentials()
                .map(|c| c.session_token().to_string()),
            meta_bucket: self.meta_bucket.clone(),
            content_bucket: self.content_bucket.clone(),
            region: self.region.clone(),
            expiration: Some(chrono_expiration),
        })
    }
}

#[cfg(feature = "aws")]
#[async_trait]
impl RuntimeCredentialsTrait for AwsRuntimeCredentials {
    fn into_shared_credentials(&self) -> SharedCredentials {
        SharedCredentials::Aws(AwsSharedCredentials {
            access_key_id: self.access_key_id.clone(),
            secret_access_key: self.secret_access_key.clone(),
            session_token: self.session_token.clone(),
            meta_bucket: self.meta_bucket.clone(),
            content_bucket: self.content_bucket.clone(),
            region: self.region.clone(),
            expiration: self.expiration,
        })
    }

    async fn to_db(&self, app_id: &str) -> Result<ConnectBuilder> {
        self.into_shared_credentials().to_db(app_id).await
    }

    #[tracing::instrument(
        name = "AwsRuntimeCredentials::to_state",
        skip(self, state),
        level = "debug"
    )]
    async fn to_state(&self, state: AppState) -> Result<FlowLikeState> {
        let (meta_store, content_store, (http_client, _refetch_rx)) = {
            use flow_like_types::tokio;

            tokio::join!(
                async { self.into_shared_credentials().to_store(true).await },
                async { self.into_shared_credentials().to_store(false).await },
                async { HTTPClient::new() }
            )
        };

        let meta_store = meta_store?;
        let content_store = content_store?;

        let mut config = {
            let mut cfg = FlowLikeConfig::with_default_store(content_store);
            cfg.register_app_meta_store(meta_store.clone());
            cfg
        };

        let (bkt, key, secret, token) = (
            self.content_bucket.clone(),
            self.access_key_id
                .clone()
                .ok_or(anyhow!("AWS_ACCESS_KEY_ID is not set"))?,
            self.secret_access_key
                .clone()
                .ok_or(anyhow!("AWS_SECRET_ACCESS_KEY is not set"))?,
            self.session_token
                .clone()
                .ok_or(anyhow!("SESSION_TOKEN is not set"))?,
        );

        config.register_build_logs_database(Arc::new(make_s3_builder(
            bkt.clone(),
            key.clone(),
            secret.clone(),
            token.clone(),
        )));
        config.register_build_project_database(Arc::new(make_s3_builder(bkt, key, secret, token)));

        let mut flow_like_state = FlowLikeState::new(config, http_client);

        flow_like_state.model_provider_config = state.provider.clone();
        flow_like_state.node_registry.write().await.node_registry = state.registry.clone();

        Ok(flow_like_state)
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
        connect(&url)
            .storage_option("aws_access_key_id".to_string(), access_key.clone())
            .storage_option("aws_secret_access_key".to_string(), secret_key.clone())
            .storage_option("aws_session_token".to_string(), session_token.clone())
    }
}

fn invoke_read_write_policy(
    credentials: &AwsRuntimeCredentials,
    apps_prefix: &str,
    user_prefix: &str,
    log_prefix: &str,
    temporary_user_prefix: &str,
    temporary_global_prefix: &str,
) -> flow_like_types::Value {
    let policy = json!({
        "Version": "2012-10-17",
        "Statement": [
          {
            "Effect": "Allow",
            "Action": [
                "s3:ListBucket"
            ],
            "Resource": [
                format!("arn:aws:s3:::{}", credentials.meta_bucket),
                format!("arn:aws:s3:::{}", credentials.content_bucket)
            ],
            "Condition": {
                "StringLike": {
                    "s3:prefix": [
                        format!("{}/*", apps_prefix),
                        format!("{}/*", user_prefix),
                        format!("{}/*", log_prefix),
                        format!("{}/*", temporary_user_prefix),
                        format!("{}/*", temporary_global_prefix)
                    ]
                }
            }
          },
          {
            "Effect": "Allow",
            "Action": [
                "s3:GetObject",
                "s3:PutObject",
                "s3:DeleteObject"
            ],
            "Resource": [
                format!("arn:aws:s3:::{}/{}/*", credentials.content_bucket, apps_prefix),
                format!("arn:aws:s3:::{}/{}/*", credentials.content_bucket, user_prefix),
                format!("arn:aws:s3:::{}/{}/*", credentials.content_bucket, temporary_user_prefix),
                format!("arn:aws:s3:::{}/{}/*", credentials.content_bucket, temporary_global_prefix),
            ],
          },
          {
            "Effect": "Allow",
            "Action": [
                "s3:GetObject",
            ],
            "Resource": [
                format!("arn:aws:s3express:::{}/{}/*", credentials.meta_bucket, apps_prefix),
            ],
          },
          {
            "Effect": "Allow",
            "Action": [
                "s3:GetObject",
                "s3:PutObject",
            ],
            "Resource": [
                format!("arn:aws:s3:::{}/{}/*", credentials.content_bucket, log_prefix),
            ],
          },
          {
            "Effect": "Allow",
            "Action": [
                "s3express:CreateSession",
            ],
            "Resource": [
                "*"
            ]
          }
        ],
    });

    policy
}

fn invoke_read_policy(
    credentials: &AwsRuntimeCredentials,
    apps_prefix: &str,
    user_prefix: &str,
    log_prefix: &str,
    temporary_user_prefix: &str,
    temporary_global_prefix: &str,
) -> flow_like_types::Value {
    let policy = json!({
        "Version": "2012-10-17",
        "Statement": [
          {
            "Effect": "Allow",
            "Action": [
                "s3:ListBucket"
            ],
            "Resource": [
                format!("arn:aws:s3:::{}", credentials.meta_bucket),
                format!("arn:aws:s3:::{}", credentials.content_bucket)
            ],
            "Condition": {
                "StringLike": {
                    "s3:prefix": [
                        format!("{}/*", apps_prefix),
                        format!("{}/*", user_prefix),
                        format!("{}/*", log_prefix),
                        format!("{}/*", temporary_user_prefix),
                        format!("{}/*", temporary_global_prefix)
                    ]
                }
            }
          },
          {
            "Effect": "Allow",
            "Action": [
                "s3:GetObject",
            ],
            "Resource": [
                format!("arn:aws:s3:::{}/{}/*", credentials.content_bucket, apps_prefix),
                format!("arn:aws:s3:::{}/{}/*", credentials.content_bucket, temporary_global_prefix),
                format!("arn:aws:s3express:::{}/{}/*", credentials.meta_bucket, apps_prefix),
            ],
          },
          {
            "Effect": "Allow",
            "Action": [
                "s3:GetObject",
                "s3:PutObject",
                "s3:DeleteObject"
            ],
            "Resource": [
                format!("arn:aws:s3:::{}/{}/*", credentials.content_bucket, user_prefix),
                format!("arn:aws:s3:::{}/{}/*", credentials.content_bucket, temporary_user_prefix),
            ],
          },
          {
            "Effect": "Allow",
            "Action": [
                "s3:GetObject",
                "s3:PutObject",
            ],
            "Resource": [
                format!("arn:aws:s3:::{}/{}/*", credentials.content_bucket, log_prefix),
            ],
          },
          {
            "Effect": "Allow",
            "Action": [
                "s3express:CreateSession",
            ],
            "Resource": [
                "*"
            ]
          }
        ],
    });

    policy
}

fn invoke_none_policy(
    credentials: &AwsRuntimeCredentials,
    apps_prefix: &str,
    user_prefix: &str,
    log_prefix: &str,
    temporary_user_prefix: &str,
) -> flow_like_types::Value {
    let policy = json!({
        "Version": "2012-10-17",
        "Statement": [
          {
            "Effect": "Allow",
            "Action": [
                "s3:ListBucket"
            ],
            "Resource": [
                format!("arn:aws:s3:::{}", credentials.meta_bucket),
            ],
            "Condition": {
                "StringLike": {
                    "s3:prefix": [
                        format!("{}/*", user_prefix),
                        format!("{}/*", temporary_user_prefix),
                    ]
                }
            }
          },
          {
            "Effect": "Allow",
            "Action": [
                "s3:GetObject",
                "s3:PutObject",
                "s3:DeleteObject"
            ],
            "Resource": [
                format!("arn:aws:s3:::{}/{}/*", credentials.content_bucket, user_prefix),
                format!("arn:aws:s3:::{}/{}/*", credentials.content_bucket, temporary_user_prefix),
            ],
          },
          {
            "Effect": "Allow",
            "Action": [
                "s3:GetObject",
            ],
            "Resource": [
                format!("arn:aws:s3express:::{}/{}/*", credentials.meta_bucket, apps_prefix),
            ],
          },
          {
            "Effect": "Allow",
            "Action": [
                "s3:GetObject",
                "s3:PutObject",
            ],
            "Resource": [
                format!("arn:aws:s3:::{}/{}/*", credentials.content_bucket, log_prefix),
            ],
          },
          {
            "Effect": "Allow",
            "Action": [
                "s3express:CreateSession",
            ],
            "Resource": [
                "*"
            ]
          }
        ],
    });

    policy
}

fn edit_app_policy(
    credentials: &AwsRuntimeCredentials,
    apps_prefix: &str,
) -> flow_like_types::Value {
    let policy = json!({
        "Version": "2012-10-17",
        "Statement": [
          {
            "Effect": "Allow",
            "Action": [
                "s3:ListBucket"
            ],
            "Resource": [
                format!("arn:aws:s3:::{}", credentials.meta_bucket),
                format!("arn:aws:s3:::{}", credentials.content_bucket)
            ],
            "Condition": {
                "StringLike": {
                    "s3:prefix": [
                        format!("{}/*", apps_prefix),
                    ]
                }
            }
          },
          {
            "Effect": "Allow",
            "Action": [
                "s3:GetObject",
                "s3:PutObject",
                "s3:DeleteObject"
            ],
            "Resource": [
                format!("arn:aws:s3:::{}/{}/*", credentials.content_bucket, apps_prefix),
                format!("arn:aws:s3express:::{}/{}/*", credentials.meta_bucket, apps_prefix),
            ],
          },
          {
            "Effect": "Allow",
            "Action": [
                "s3express:CreateSession",
            ],
            "Resource": [
                "*"
            ]
          }
        ],
    });

    policy
}

fn read_app_policy(
    credentials: &AwsRuntimeCredentials,
    apps_prefix: &str,
) -> flow_like_types::Value {
    let policy = json!({
        "Version": "2012-10-17",
        "Statement": [
          {
            "Effect": "Allow",
            "Action": [
                "s3:ListBucket"
            ],
            "Resource": [
                format!("arn:aws:s3:::{}", credentials.meta_bucket),
                format!("arn:aws:s3:::{}", credentials.content_bucket)
            ],
            "Condition": {
                "StringLike": {
                    "s3:prefix": [
                        format!("{}/*", apps_prefix),
                    ]
                }
            }
          },
          {
            "Effect": "Allow",
            "Action": [
                "s3:GetObject"
            ],
            "Resource": [
                format!("arn:aws:s3:::{}/{}/*", credentials.content_bucket, apps_prefix),
                format!("arn:aws:s3express:::{}/{}/*", credentials.meta_bucket, apps_prefix),
            ],
          },
          {
            "Effect": "Allow",
            "Action": [
                "s3express:CreateSession",
            ],
            "Resource": [
                "*"
            ]
          }
        ],
    });

    policy
}

fn read_logs_policy(
    credentials: &AwsRuntimeCredentials,
    log_prefix: &str,
) -> flow_like_types::Value {
    let policy = json!({
        "Version": "2012-10-17",
        "Statement": [
          {
            "Effect": "Allow",
            "Action": [
                "s3:ListBucket"
            ],
            "Resource": [
                format!("arn:aws:s3:::{}", credentials.content_bucket),
            ],
            "Condition": {
                "StringLike": {
                    "s3:prefix": [
                        format!("{}/*", log_prefix),
                    ]
                }
            }
          },
          {
            "Effect": "Allow",
            "Action": [
                "s3:GetObject",
            ],
            "Resource": [
                format!("arn:aws:s3:::{}/{}/*", credentials.content_bucket, log_prefix),
            ],
          }
        ],
    });

    policy
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(all(test, feature = "aws"))]
mod tests {
    use super::*;
    use crate::credentials::RuntimeCredentialsTrait;
    use flow_like::credentials::SharedCredentialsTrait;
    use flow_like_storage::Path;
    use flow_like_storage::object_store::ObjectStore;
    use flow_like_types::json::{from_str, to_string};
    use flow_like_types::tokio;

    #[tokio::test]
    #[ignore]
    async fn test_aws_master_credentials_setup() {
        let creds = AwsRuntimeCredentials::from_env();
        assert!(
            creds.access_key_id.is_some(),
            "AWS_ACCESS_KEY_ID must be set"
        );
        assert!(
            creds.secret_access_key.is_some(),
            "AWS_SECRET_ACCESS_KEY must be set"
        );
        assert!(
            !creds.meta_bucket.is_empty(),
            "META_BUCKET_NAME must be set"
        );
        assert!(
            !creds.content_bucket.is_empty(),
            "CONTENT_BUCKET_NAME must be set"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_aws_master_credentials_can_write() {
        let creds = AwsRuntimeCredentials::from_env();
        let shared = creds.into_shared_credentials();
        let store = shared
            .to_store(false)
            .await
            .expect("Failed to create store from master credentials");

        let test_path = format!(
            "test/master-write-test-{}.txt",
            flow_like_types::create_id()
        );
        let path = Path::from(test_path.as_str());

        match &store {
            flow_like::flow_like_storage::files::store::FlowLikeStore::AWS(s) => {
                s.put(&path, b"test content".to_vec().into())
                    .await
                    .expect("Master credentials should be able to write");
                s.delete(&path).await.ok();
            }
            _ => panic!("Expected AWS store"),
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_aws_master_credentials_can_read() {
        let creds = AwsRuntimeCredentials::from_env();
        let shared = creds.into_shared_credentials();
        let store = shared
            .to_store(false)
            .await
            .expect("Failed to create store from master credentials");

        let test_path = format!("test/master-read-test-{}.txt", flow_like_types::create_id());
        let path = Path::from(test_path.as_str());
        let content = b"read test content";

        match &store {
            flow_like::flow_like_storage::files::store::FlowLikeStore::AWS(s) => {
                s.put(&path, content.to_vec().into())
                    .await
                    .expect("Setup: write should succeed");

                let result = s.get(&path).await.expect("Read should succeed");
                let bytes = result.bytes().await.expect("Should get bytes");
                assert_eq!(bytes.as_ref(), content);

                s.delete(&path).await.ok();
            }
            _ => panic!("Expected AWS store"),
        }
    }

    #[test]
    fn test_aws_runtime_credentials_serialization() {
        let creds = AwsRuntimeCredentials {
            access_key_id: Some("AKIATEST".to_string()),
            secret_access_key: Some("secret".to_string()),
            session_token: Some("token".to_string()),
            meta_bucket: "meta".to_string(),
            content_bucket: "content".to_string(),
            region: "us-east-1".to_string(),
            expiration: None,
        };

        let json = to_string(&creds).expect("Failed to serialize");
        let deserialized: AwsRuntimeCredentials = from_str(&json).expect("Failed to deserialize");

        assert_eq!(creds.access_key_id, deserialized.access_key_id);
        assert_eq!(creds.region, deserialized.region);
    }

    #[test]
    fn test_credentials_access_display() {
        use crate::credentials::CredentialsAccess;

        assert_eq!(format!("{}", CredentialsAccess::EditApp), "edit_app");
        assert_eq!(format!("{}", CredentialsAccess::ReadApp), "read_app");
        assert_eq!(format!("{}", CredentialsAccess::InvokeNone), "invoke_none");
        assert_eq!(format!("{}", CredentialsAccess::InvokeRead), "invoke_read");
        assert_eq!(
            format!("{}", CredentialsAccess::InvokeWrite),
            "invoke_write"
        );
        assert_eq!(format!("{}", CredentialsAccess::ReadLogs), "read_logs");
    }
}
