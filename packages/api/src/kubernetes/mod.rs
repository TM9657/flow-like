//! Kubernetes execution module for dispatching board/event executions to K8s jobs.
//!
//! This module provides functionality to execute Flow-Like workflows on Kubernetes
//! by creating isolated Jobs or dispatching to warm executor pools.

mod config;
mod dispatch;

pub use config::KubernetesConfig;
pub use dispatch::{
    DispatchError, ExecutionContext, JobDispatcher, JobMode, JobStatus, SubmitJobRequest,
    SubmitJobResponse,
};

// Re-export execution JWT types for backwards compatibility
pub use crate::execution::{
    ExecutionClaims, ExecutionJwk, ExecutionJwks, ExecutionJwtError, ExecutionJwtParams, TokenType,
    get_execution_jwks, is_jwt_configured, sign_execution_jwt, verify_execution_jwt,
    verify_user_jwt,
};
