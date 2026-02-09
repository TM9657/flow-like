//! Admin sink management routes
//!
//! Provides endpoints for registering and revoking service sink tokens.
//! These are long-lived JWTs scoped to specific sink types that internal
//! services use to trigger events.

pub mod list_tokens;
pub mod register_sink;
pub mod revoke_sink;
