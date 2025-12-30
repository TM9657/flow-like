use crate::credentials::{LogsDbBuilder, SharedCredentialsTrait, StoreType};
use flow_like_storage::lancedb::connection::ConnectBuilder;
use flow_like_storage::object_store::azure::MicrosoftAzureBuilder;
use flow_like_storage::{Path, object_store};
use flow_like_storage::{files::store::FlowLikeStore, lancedb};
use flow_like_types::{Result, anyhow, async_trait};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AzureSharedCredentials {
    /// SAS token for meta container
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta_sas_token: Option<String>,
    /// SAS token for content container
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_sas_token: Option<String>,
    /// SAS token for logs container
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logs_sas_token: Option<String>,
    pub meta_container: String,
    pub content_container: String,
    pub logs_container: String,
    pub account_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_key: Option<String>,
    pub expiration: Option<chrono::DateTime<chrono::Utc>>,
}

impl AzureSharedCredentials {
    fn parse_sas_token(sas: &str) -> Vec<(String, String)> {
        let sas = sas.trim_start_matches('?');
        sas.split('&')
            .filter_map(|pair| {
                let mut parts = pair.splitn(2, '=');
                match (parts.next(), parts.next()) {
                    (Some(k), Some(v)) => {
                        // URL-decode the value since object_store will re-encode it
                        let decoded_v = percent_encoding::percent_decode_str(v)
                            .decode_utf8()
                            .map(|cow| cow.into_owned())
                            .unwrap_or_else(|_| v.to_string());
                        Some((k.to_string(), decoded_v))
                    }
                    _ => None,
                }
            })
            .collect()
    }
}

#[async_trait]
impl SharedCredentialsTrait for AzureSharedCredentials {
    #[tracing::instrument(name = "AzureSharedCredentials::to_store", skip(self, meta), fields(meta = meta), level="debug")]
    async fn to_store(&self, meta: bool) -> Result<FlowLikeStore> {
        self.to_store_type(if meta {
            StoreType::Meta
        } else {
            StoreType::Content
        })
        .await
    }

    #[tracing::instrument(name = "AzureSharedCredentials::to_store_type", skip(self), fields(store_type = ?store_type), level="debug")]
    async fn to_store_type(&self, store_type: StoreType) -> Result<FlowLikeStore> {
        use flow_like_types::tokio;

        let (container, sas_token) = match store_type {
            StoreType::Meta => (&self.meta_container, &self.meta_sas_token),
            StoreType::Content => (&self.content_container, &self.content_sas_token),
            StoreType::Logs => (&self.logs_container, &self.logs_sas_token),
        };

        let account = self.account_name.clone();
        let container = container.clone();
        let account_key = self.account_key.clone();
        let sas_token = sas_token.clone();

        let store = tokio::task::spawn_blocking(move || {
            let builder = MicrosoftAzureBuilder::new()
                .with_account(account)
                .with_container_name(container);

            // Use account key for master credentials, SAS for scoped credentials
            if let Some(key) = account_key {
                builder.with_access_key(key).build()
            } else if let Some(sas) = sas_token {
                let sas_pairs = Self::parse_sas_token(&sas);
                builder.with_sas_authorization(sas_pairs).build()
            } else {
                Err(object_store::Error::Generic {
                    store: "MicrosoftAzure",
                    source: "No account key or SAS token provided".into(),
                })
            }
        })
        .await
        .map_err(|e| anyhow!("Failed to spawn blocking task: {}", e))??;

        Ok(FlowLikeStore::Azure(Arc::new(store)))
    }

    #[tracing::instrument(name = "AzureSharedCredentials::to_db", skip(self), level = "debug")]
    async fn to_db(&self, app_id: &str) -> Result<ConnectBuilder> {
        let path = Path::from("apps")
            .child(app_id)
            .child("storage")
            .child("db");
        let sas_token = self.content_sas_token.clone().unwrap_or_default();
        let connection = make_azure_builder(
            self.account_name.clone(),
            self.content_container.clone(),
            sas_token,
        );
        let connection = connection(path.clone());
        Ok(connection)
    }

    fn to_logs_db_builder(&self) -> Result<LogsDbBuilder> {
        if self.logs_container.is_empty() {
            return Err(anyhow!(
                "logs_container is empty - cannot create logs database builder"
            ));
        }
        let sas_token = self.logs_sas_token.clone().unwrap_or_default();
        let builder = make_azure_builder(
            self.account_name.clone(),
            self.logs_container.clone(),
            sas_token,
        );
        Ok(Arc::new(builder))
    }
}

fn make_azure_builder(
    account_name: String,
    container: String,
    sas_token: String,
) -> impl Fn(object_store::path::Path) -> ConnectBuilder + Send + Sync + 'static {
    move |path| {
        let url = format!("az://{}/{}", container, path);
        lancedb::connect(&url)
            .storage_option(
                "azure_storage_account_name".to_string(),
                account_name.clone(),
            )
            .storage_option("azure_storage_sas_token".to_string(), sas_token.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_like_types::json::{from_str, to_string};

    fn sample_credentials() -> AzureSharedCredentials {
        AzureSharedCredentials {
            meta_sas_token: Some("?sv=2022-11-02&ss=b&srt=sco&sp=rl&se=2025-01-15T20:00:00Z&st=2025-01-15T12:00:00Z&spr=https&sig=meta123".to_string()),
            content_sas_token: Some("?sv=2022-11-02&ss=b&srt=sco&sp=rwdlacyx&se=2025-01-15T20:00:00Z&st=2025-01-15T12:00:00Z&spr=https&sig=content456".to_string()),
            logs_sas_token: Some("?sv=2022-11-02&ss=b&srt=sco&sp=rl&se=2025-01-15T20:00:00Z&st=2025-01-15T12:00:00Z&spr=https&sig=logs789".to_string()),
            meta_container: "meta-container".to_string(),
            content_container: "content-container".to_string(),
            logs_container: "logs-container".to_string(),
            account_name: "mystorageaccount".to_string(),
            account_key: None,
            expiration: None,
        }
    }

    #[test]
    fn test_azure_credentials_serialization() {
        let creds = sample_credentials();
        let json = to_string(&creds).expect("Failed to serialize");

        assert!(json.contains("mystorageaccount"));
        assert!(json.contains("meta-container"));
        assert!(json.contains("content-container"));
        assert!(json.contains("sv=2022-11-02"));
    }

    #[test]
    fn test_azure_credentials_deserialization() {
        let json = r#"{
            "sas_token": "?sv=2022-11-02&ss=b&srt=sco&sp=r&se=2025-01-15T20:00:00Z&sig=test",
            "meta_container": "test-meta",
            "content_container": "test-content",
            "logs_container": "test-logs",
            "account_name": "teststorage",
            "expiration": null
        }"#;

        let creds: AzureSharedCredentials = from_str(json).expect("Failed to deserialize");

        assert_eq!(creds.account_name, "teststorage");
        assert_eq!(creds.meta_container, "test-meta");
        assert_eq!(creds.content_container, "test-content");
        assert!(creds.expiration.is_none());
    }

    #[test]
    fn test_azure_credentials_roundtrip() {
        let original = sample_credentials();
        let json = to_string(&original).expect("Failed to serialize");
        let deserialized: AzureSharedCredentials = from_str(&json).expect("Failed to deserialize");

        assert_eq!(original.meta_container, deserialized.meta_container);
        assert_eq!(original.content_container, deserialized.content_container);
        assert_eq!(original.account_name, deserialized.account_name);
    }

    #[test]
    fn test_azure_credentials_with_expiration() {
        let json = r#"{
            "sas_token": "?sv=2022-11-02&ss=b&sig=test",
            "meta_container": "meta",
            "content_container": "content",
            "logs_container": "logs",
            "account_name": "storage",
            "expiration": "2025-01-15T12:00:00Z"
        }"#;

        let creds: AzureSharedCredentials = from_str(json).expect("Failed to deserialize");
        assert!(creds.expiration.is_some());
    }

    #[test]
    fn test_parse_sas_token_with_question_mark() {
        let sas = "?sv=2022-11-02&ss=b&srt=sco&sp=rwdlacyx&se=2025-01-15T20:00:00Z";
        let pairs = AzureSharedCredentials::parse_sas_token(sas);

        assert!(pairs.iter().any(|(k, _)| k == "sv"));
        assert!(pairs.iter().any(|(k, _)| k == "ss"));
        assert!(pairs.iter().any(|(k, _)| k == "srt"));
        assert!(pairs.iter().any(|(k, _)| k == "sp"));
        assert!(pairs.iter().any(|(k, _)| k == "se"));
    }

    #[test]
    fn test_parse_sas_token_without_question_mark() {
        let sas = "sv=2022-11-02&ss=b&srt=sco";
        let pairs = AzureSharedCredentials::parse_sas_token(sas);

        assert_eq!(pairs.len(), 3);
        assert!(pairs.iter().any(|(k, v)| k == "sv" && v == "2022-11-02"));
        assert!(pairs.iter().any(|(k, v)| k == "ss" && v == "b"));
        assert!(pairs.iter().any(|(k, v)| k == "srt" && v == "sco"));
    }

    #[test]
    fn test_parse_sas_token_decodes_url_encoded_values() {
        let sas = "sv=2022-11-02&se=2025-01-15T20%3A00%3A00Z&sig=abc%2Fdef%3D";
        let pairs = AzureSharedCredentials::parse_sas_token(sas);

        // Values should be URL-decoded since object_store will re-encode them
        let se = pairs
            .iter()
            .find(|(k, _)| k == "se")
            .map(|(_, v)| v.as_str());
        assert_eq!(se, Some("2025-01-15T20:00:00Z"));

        let sig = pairs
            .iter()
            .find(|(k, _)| k == "sig")
            .map(|(_, v)| v.as_str());
        assert_eq!(sig, Some("abc/def="));
    }

    #[test]
    fn test_parse_sas_token_empty() {
        let pairs = AzureSharedCredentials::parse_sas_token("");
        assert!(pairs.is_empty());
    }
}
