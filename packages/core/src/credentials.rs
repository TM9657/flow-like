use aws_credentials::AwsSharedCredentials;
use azure_credentials::AzureSharedCredentials;
use flow_like_storage::files::store::FlowLikeStore;
use flow_like_storage::lancedb::connection::ConnectBuilder;
use flow_like_types::Result;
use flow_like_types::async_trait;
use gcp_credentials::GcpSharedCredentials;
use serde::{Deserialize, Serialize};

pub mod aws_credentials;
pub mod azure_credentials;
pub mod gcp_credentials;

#[async_trait]
pub trait SharedCredentialsTrait {
    async fn to_store(&self, meta: bool) -> Result<FlowLikeStore>;
    async fn to_db(&self, app_id: &str) -> Result<ConnectBuilder>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SharedCredentials {
    Aws(AwsSharedCredentials),
    Azure(AzureSharedCredentials),
    Gcp(GcpSharedCredentials),
}

impl SharedCredentials {
    pub async fn to_store(&self, meta: bool) -> Result<FlowLikeStore> {
        match self {
            SharedCredentials::Aws(aws) => aws.to_store(meta).await,
            SharedCredentials::Azure(azure) => azure.to_store(meta).await,
            SharedCredentials::Gcp(gcp) => gcp.to_store(meta).await,
        }
    }

    pub async fn to_db(&self, app_id: &str) -> Result<ConnectBuilder> {
        match self {
            SharedCredentials::Aws(aws) => aws.to_db(app_id).await,
            SharedCredentials::Azure(azure) => azure.to_db(app_id).await,
            SharedCredentials::Gcp(gcp) => gcp.to_db(app_id).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_like_types::json::{from_str, to_string};

    fn sample_aws() -> AwsSharedCredentials {
        AwsSharedCredentials {
            access_key_id: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
            secret_access_key: Some("secret".to_string()),
            session_token: Some("token".to_string()),
            meta_bucket: "aws-meta".to_string(),
            content_bucket: "aws-content".to_string(),
            region: "us-west-2".to_string(),
            expiration: None,
        }
    }

    fn sample_azure() -> AzureSharedCredentials {
        AzureSharedCredentials {
            sas_token: "?sv=2022-11-02&ss=b&sig=test".to_string(),
            meta_container: "azure-meta".to_string(),
            content_container: "azure-content".to_string(),
            account_name: "mystorageaccount".to_string(),
            account_key: None,
            expiration: None,
        }
    }

    fn sample_gcp() -> GcpSharedCredentials {
        GcpSharedCredentials {
            service_account_key: r#"{"type":"service_account","project_id":"test"}"#.to_string(),
            access_token: None,
            meta_bucket: "gcp-meta".to_string(),
            content_bucket: "gcp-content".to_string(),
            allowed_prefixes: Vec::new(),
            write_access: true,
            expiration: None,
        }
    }

    #[test]
    fn test_shared_credentials_aws_serialization() {
        let creds = SharedCredentials::Aws(sample_aws());
        let json = to_string(&creds).expect("Failed to serialize");

        assert!(json.contains("Aws"));
        assert!(json.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(json.contains("aws-meta"));
    }

    #[test]
    fn test_shared_credentials_azure_serialization() {
        let creds = SharedCredentials::Azure(sample_azure());
        let json = to_string(&creds).expect("Failed to serialize");

        assert!(json.contains("Azure"));
        assert!(json.contains("mystorageaccount"));
        assert!(json.contains("azure-meta"));
    }

    #[test]
    fn test_shared_credentials_gcp_serialization() {
        let creds = SharedCredentials::Gcp(sample_gcp());
        let json = to_string(&creds).expect("Failed to serialize");

        assert!(json.contains("Gcp"));
        assert!(json.contains("gcp-meta"));
    }

    #[test]
    fn test_shared_credentials_aws_deserialization() {
        let json = r#"{"Aws":{"access_key_id":"AKIA123","secret_access_key":"secret","session_token":"token","meta_bucket":"meta","content_bucket":"content","region":"us-east-1","expiration":null}}"#;
        let creds: SharedCredentials = from_str(json).expect("Failed to deserialize");

        match creds {
            SharedCredentials::Aws(aws) => {
                assert_eq!(aws.access_key_id, Some("AKIA123".to_string()));
                assert_eq!(aws.region, "us-east-1");
            }
            _ => panic!("Expected AWS credentials"),
        }
    }

    #[test]
    fn test_shared_credentials_azure_deserialization() {
        let json = r#"{"Azure":{"sas_token":"?sv=test","meta_container":"meta","content_container":"content","account_name":"storage","expiration":null}}"#;
        let creds: SharedCredentials = from_str(json).expect("Failed to deserialize");

        match creds {
            SharedCredentials::Azure(azure) => {
                assert_eq!(azure.account_name, "storage");
                assert_eq!(azure.meta_container, "meta");
            }
            _ => panic!("Expected Azure credentials"),
        }
    }

    #[test]
    fn test_shared_credentials_gcp_deserialization() {
        let json = r#"{"Gcp":{"service_account_key":"{\"type\":\"service_account\"}","meta_bucket":"meta","content_bucket":"content","expiration":null}}"#;
        let creds: SharedCredentials = from_str(json).expect("Failed to deserialize");

        match creds {
            SharedCredentials::Gcp(gcp) => {
                assert_eq!(gcp.meta_bucket, "meta");
                assert!(gcp.service_account_key.contains("service_account"));
            }
            _ => panic!("Expected GCP credentials"),
        }
    }

    #[test]
    fn test_shared_credentials_roundtrip_all_variants() {
        let variants: Vec<SharedCredentials> = vec![
            SharedCredentials::Aws(sample_aws()),
            SharedCredentials::Azure(sample_azure()),
            SharedCredentials::Gcp(sample_gcp()),
        ];

        for creds in variants {
            let json = to_string(&creds).expect("Failed to serialize");
            let deserialized: SharedCredentials = from_str(&json).expect("Failed to deserialize");

            match (&creds, &deserialized) {
                (SharedCredentials::Aws(a), SharedCredentials::Aws(b)) => {
                    assert_eq!(a.access_key_id, b.access_key_id);
                    assert_eq!(a.meta_bucket, b.meta_bucket);
                }
                (SharedCredentials::Azure(a), SharedCredentials::Azure(b)) => {
                    assert_eq!(a.account_name, b.account_name);
                    assert_eq!(a.meta_container, b.meta_container);
                }
                (SharedCredentials::Gcp(a), SharedCredentials::Gcp(b)) => {
                    assert_eq!(a.meta_bucket, b.meta_bucket);
                }
                _ => panic!("Variant mismatch after roundtrip"),
            }
        }
    }

    #[test]
    fn test_shared_credentials_debug_impl() {
        let creds = SharedCredentials::Aws(sample_aws());
        let debug_str = format!("{:?}", creds);
        assert!(debug_str.contains("Aws"));
    }

    #[test]
    fn test_shared_credentials_clone() {
        let original = SharedCredentials::Azure(sample_azure());
        let cloned = original.clone();

        match (original, cloned) {
            (SharedCredentials::Azure(a), SharedCredentials::Azure(b)) => {
                assert_eq!(a.account_name, b.account_name);
                assert_eq!(a.sas_token, b.sas_token);
            }
            _ => panic!("Clone should preserve variant"),
        }
    }
}
