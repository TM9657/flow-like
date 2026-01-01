use std::fmt::Display;

#[cfg(feature = "aws")]
use aws_credentials::AwsRuntimeCredentials;
#[cfg(feature = "azure")]
use azure_credentials::AzureRuntimeCredentials;
use flow_like::credentials::SharedCredentials;
use flow_like::flow_like_storage::files::store::FlowLikeStore;
use flow_like::state::FlowLikeState;
use flow_like_storage::lancedb::connection::ConnectBuilder;
use flow_like_types::Result;
use flow_like_types::async_trait;
#[cfg(feature = "gcp")]
use gcp_credentials::GcpRuntimeCredentials;
#[cfg(feature = "r2")]
use r2_credentials::R2RuntimeCredentials;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::state::AppState;
use crate::state::State;

#[cfg(feature = "aws")]
pub mod aws_credentials;
#[cfg(feature = "azure")]
pub mod azure_credentials;
#[cfg(feature = "gcp")]
pub mod gcp_credentials;
pub mod local_credentials;
#[cfg(feature = "r2")]
pub mod r2_credentials;

#[async_trait]
pub trait RuntimeCredentialsTrait {
    async fn to_state(&self, state: AppState) -> Result<FlowLikeState>;
    async fn to_db(&self, app_id: &str) -> Result<ConnectBuilder>;
    fn into_shared_credentials(&self) -> SharedCredentials;
}

#[derive(Clone, Debug)]
pub enum CredentialsAccess {
    EditApp,
    ReadApp,
    InvokeNone,
    InvokeRead,
    InvokeWrite,
    ReadLogs,
}

impl Display for CredentialsAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CredentialsAccess::EditApp => write!(f, "edit_app"),
            CredentialsAccess::ReadApp => write!(f, "read_app"),
            CredentialsAccess::InvokeNone => write!(f, "invoke_none"),
            CredentialsAccess::InvokeRead => write!(f, "invoke_read"),
            CredentialsAccess::InvokeWrite => write!(f, "invoke_write"),
            CredentialsAccess::ReadLogs => write!(f, "read_logs"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RuntimeCredentials {
    #[cfg(feature = "aws")]
    Aws(AwsRuntimeCredentials),
    #[cfg(feature = "azure")]
    Azure(AzureRuntimeCredentials),
    #[cfg(feature = "gcp")]
    Gcp(GcpRuntimeCredentials),
    #[cfg(feature = "r2")]
    R2(R2RuntimeCredentials),
}

impl RuntimeCredentials {
    pub async fn scoped(
        sub: &str,
        app_id: &str,
        state: &State,
        mode: CredentialsAccess,
    ) -> Result<Self> {
        #[cfg(feature = "r2")]
        return Ok(RuntimeCredentials::R2(
            R2RuntimeCredentials::from_env()
                .scoped_credentials(sub, app_id, state, mode)
                .await?,
        ));

        #[cfg(all(feature = "aws", not(feature = "r2")))]
        return Ok(RuntimeCredentials::Aws(
            AwsRuntimeCredentials::from_env()
                .scoped_credentials(sub, app_id, state, mode)
                .await?,
        ));

        #[cfg(all(feature = "azure", not(feature = "aws"), not(feature = "r2")))]
        return Ok(RuntimeCredentials::Azure(
            AzureRuntimeCredentials::from_env()
                .scoped_credentials(sub, app_id, state, mode)
                .await?,
        ));

        #[cfg(all(
            feature = "gcp",
            not(feature = "aws"),
            not(feature = "azure"),
            not(feature = "r2")
        ))]
        return Ok(RuntimeCredentials::Gcp(
            GcpRuntimeCredentials::from_env()
                .scoped_credentials(sub, app_id, state, mode)
                .await?,
        ));

        #[cfg(not(any(feature = "aws", feature = "azure", feature = "gcp", feature = "r2")))]
        {
            let _ = (sub, app_id, state, mode);
            Err(flow_like_types::anyhow!(
                "No storage backend feature enabled"
            ))
        }
    }

    pub async fn master_credentials() -> Result<Self> {
        #[cfg(feature = "r2")]
        return Ok(RuntimeCredentials::R2(
            R2RuntimeCredentials::from_env().master_credentials().await,
        ));

        #[cfg(all(feature = "aws", not(feature = "r2")))]
        return Ok(RuntimeCredentials::Aws(
            AwsRuntimeCredentials::from_env().master_credentials().await,
        ));

        #[cfg(all(feature = "azure", not(feature = "aws"), not(feature = "r2")))]
        return Ok(RuntimeCredentials::Azure(
            AzureRuntimeCredentials::from_env()
                .master_credentials()
                .await,
        ));

        #[cfg(all(
            feature = "gcp",
            not(feature = "aws"),
            not(feature = "azure"),
            not(feature = "r2")
        ))]
        return Ok(RuntimeCredentials::Gcp(
            GcpRuntimeCredentials::from_env().master_credentials().await,
        ));

        #[cfg(not(any(feature = "aws", feature = "azure", feature = "gcp", feature = "r2")))]
        Err(flow_like_types::anyhow!(
            "No storage backend feature enabled"
        ))
    }

    pub async fn to_store(&self, meta: bool) -> Result<FlowLikeStore> {
        match self {
            #[cfg(feature = "aws")]
            RuntimeCredentials::Aws(aws) => aws.into_shared_credentials().to_store(meta).await,
            #[cfg(feature = "azure")]
            RuntimeCredentials::Azure(azure) => {
                azure.into_shared_credentials().to_store(meta).await
            }
            #[cfg(feature = "gcp")]
            RuntimeCredentials::Gcp(gcp) => gcp.into_shared_credentials().to_store(meta).await,
            #[cfg(feature = "r2")]
            RuntimeCredentials::R2(r2) => r2.into_shared_credentials().to_store(meta).await,
        }
    }

    pub async fn to_db(&self, app_id: &str) -> Result<ConnectBuilder> {
        match self {
            #[cfg(feature = "aws")]
            RuntimeCredentials::Aws(aws) => aws.to_db(app_id).await,
            #[cfg(feature = "azure")]
            RuntimeCredentials::Azure(azure) => azure.to_db(app_id).await,
            #[cfg(feature = "gcp")]
            RuntimeCredentials::Gcp(gcp) => gcp.to_db(app_id).await,
            #[cfg(feature = "r2")]
            RuntimeCredentials::R2(r2) => r2.to_db(app_id).await,
        }
    }

    #[instrument(skip(self, state), level = "debug")]
    pub async fn to_state(&self, state: AppState) -> Result<FlowLikeState> {
        match self {
            #[cfg(feature = "aws")]
            RuntimeCredentials::Aws(aws) => aws.to_state(state).await,
            #[cfg(feature = "azure")]
            RuntimeCredentials::Azure(azure) => azure.to_state(state).await,
            #[cfg(feature = "gcp")]
            RuntimeCredentials::Gcp(gcp) => gcp.to_state(state).await,
            #[cfg(feature = "r2")]
            RuntimeCredentials::R2(r2) => r2.to_state(state).await,
        }
    }

    #[instrument(skip(self), level = "debug")]
    pub fn into_shared_credentials(&self) -> SharedCredentials {
        match self {
            #[cfg(feature = "aws")]
            RuntimeCredentials::Aws(aws) => aws.into_shared_credentials(),
            #[cfg(feature = "azure")]
            RuntimeCredentials::Azure(azure) => azure.into_shared_credentials(),
            #[cfg(feature = "gcp")]
            RuntimeCredentials::Gcp(gcp) => gcp.into_shared_credentials(),
            #[cfg(feature = "r2")]
            RuntimeCredentials::R2(r2) => r2.into_shared_credentials(),
        }
    }
}
