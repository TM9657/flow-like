use std::fmt::Display;

use aws_credentials::AwsRuntimeCredentials;
use flow_like::credentials::SharedCredentials;
use flow_like::flow_like_storage::files::store::FlowLikeStore;
use flow_like::state::FlowLikeState;
use flow_like_storage::databases::vector::lancedb::LanceDBVectorStore;
use flow_like_storage::lancedb::connection::ConnectBuilder;
use flow_like_types::Result;
use flow_like_types::async_trait;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::state::AppState;
use crate::state::State;

pub mod aws_credentials;

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
}

impl RuntimeCredentials {
    pub async fn scoped(
        sub: &str,
        app_id: &str,
        state: &State,
        mode: CredentialsAccess,
    ) -> Result<Self> {
        #[cfg(feature = "aws")]
        return Ok(RuntimeCredentials::Aws(
            AwsRuntimeCredentials::from_env()
                .scoped_credentials(sub, app_id, state, mode)
                .await?,
        ));
    }

    pub async fn master_credentials() -> Result<Self> {
        #[cfg(feature = "aws")]
        return Ok(RuntimeCredentials::Aws(
            AwsRuntimeCredentials::from_env().master_credentials().await,
        ));
    }

    pub async fn to_store(&self, meta: bool) -> Result<FlowLikeStore> {
        match self {
            #[cfg(feature = "aws")]
            RuntimeCredentials::Aws(aws) => aws.into_shared_credentials().to_store(meta).await,
        }
    }

    pub async fn to_db(&self, app_id: &str) -> Result<ConnectBuilder> {
        match self {
            #[cfg(feature = "aws")]
            RuntimeCredentials::Aws(aws) => aws.to_db(app_id).await,
        }
    }

    #[instrument(skip(self, state), level = "debug")]
    pub async fn to_state(&self, state: AppState) -> Result<FlowLikeState> {
        match self {
            #[cfg(feature = "aws")]
            RuntimeCredentials::Aws(aws) => aws.to_state(state).await,
        }
    }

    #[instrument(skip(self), level = "debug")]
    pub fn into_shared_credentials(&self) -> SharedCredentials {
        match self {
            #[cfg(feature = "aws")]
            RuntimeCredentials::Aws(aws) => aws.into_shared_credentials(),
        }
    }
}
