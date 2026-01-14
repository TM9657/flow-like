use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug, Clone)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

// Associated constants for enum-like usage without parentheses
#[allow(non_upper_case_globals)]
impl ApiError {
    pub const NotFound: ApiError = ApiError {
        status: StatusCode::NOT_FOUND,
        message: String::new(),
    };

    pub const Forbidden: ApiError = ApiError {
        status: StatusCode::FORBIDDEN,
        message: String::new(),
    };

    pub const Unauthorized: ApiError = ApiError {
        status: StatusCode::UNAUTHORIZED,
        message: String::new(),
    };
}

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        tracing::error!("Internal error: {}", msg);
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, msg)
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        tracing::warn!("Not found: {}", msg);
        Self::new(StatusCode::NOT_FOUND, msg)
    }

    pub fn bad_request(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        tracing::warn!("Bad request: {}", msg);
        Self::new(StatusCode::BAD_REQUEST, msg)
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        tracing::warn!("Unauthorized: {}", msg);
        Self::new(StatusCode::UNAUTHORIZED, msg)
    }

    pub fn forbidden(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        tracing::warn!("Forbidden: {}", msg);
        Self::new(StatusCode::FORBIDDEN, msg)
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        tracing::warn!("Conflict: {}", msg);
        Self::new(StatusCode::CONFLICT, msg)
    }

    pub fn gone(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        tracing::warn!("Gone: {}", msg);
        Self::new(StatusCode::GONE, msg)
    }

    pub fn unprocessable(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        tracing::warn!("Unprocessable entity: {}", msg);
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, msg)
    }

    pub fn too_many_requests(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        tracing::warn!("Too many requests: {}", msg);
        Self::new(StatusCode::TOO_MANY_REQUESTS, msg)
    }

    pub fn service_unavailable(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        tracing::error!("Service unavailable: {}", msg);
        Self::new(StatusCode::SERVICE_UNAVAILABLE, msg)
    }

    // Legacy constructor for backward compatibility
    #[allow(non_snake_case)]
    pub fn BadRequest(msg: impl Into<String>) -> Self {
        Self::bad_request(msg)
    }

    #[allow(non_snake_case)]
    pub fn InternalError(err: flow_like_types::Error) -> Self {
        Self::internal(err.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let message = if self.message.is_empty() {
            self.status.canonical_reason().unwrap_or("Error").to_string()
        } else {
            self.message
        };
        (self.status, message).into_response()
    }
}

// Implement From for flow_like_types::Error
impl From<flow_like_types::Error> for ApiError {
    fn from(err: flow_like_types::Error) -> Self {
        tracing::error!("Internal error: {:?}", err);
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
    }
}

// Implement From for sea_orm::DbErr
impl From<sea_orm::DbErr> for ApiError {
    fn from(err: sea_orm::DbErr) -> Self {
        tracing::error!("Database error: {:?}", err);
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", err))
    }
}

// Implement From for common error types
impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        tracing::error!("IO error: {:?}", err);
        Self::internal(format!("IO error: {}", err))
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        tracing::error!("JSON error: {:?}", err);
        Self::bad_request(format!("JSON error: {}", err))
    }
}

impl From<std::num::ParseIntError> for ApiError {
    fn from(err: std::num::ParseIntError) -> Self {
        tracing::warn!("Parse error: {:?}", err);
        Self::bad_request(format!("Invalid number format: {}", err))
    }
}

impl From<flow_like_storage::object_store::Error> for ApiError {
    fn from(err: flow_like_storage::object_store::Error) -> Self {
        tracing::error!("Object store error: {:?}", err);
        Self::internal(format!("Storage error: {}", err))
    }
}

impl From<jsonwebtoken::errors::Error> for ApiError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        tracing::warn!("JWT error: {:?}", err);
        Self::unauthorized(format!("JWT error: {}", err))
    }
}

impl From<flow_like_storage::lancedb::Error> for ApiError {
    fn from(err: flow_like_storage::lancedb::Error) -> Self {
        tracing::error!("LanceDB error: {:?}", err);
        Self::internal(format!("Database error: {}", err))
    }
}

impl From<sea_orm::TransactionError<ApiError>> for ApiError {
    fn from(err: sea_orm::TransactionError<ApiError>) -> Self {
        match err {
            sea_orm::TransactionError::Connection(db_err) => db_err.into(),
            sea_orm::TransactionError::Transaction(api_err) => api_err,
        }
    }
}

impl From<stripe::StripeError> for ApiError {
    fn from(err: stripe::StripeError) -> Self {
        tracing::error!("Stripe error: {:?}", err);
        Self::internal(format!("Payment error: {}", err))
    }
}

impl From<flow_like_storage::datafusion::error::DataFusionError> for ApiError {
    fn from(err: flow_like_storage::datafusion::error::DataFusionError) -> Self {
        tracing::error!("DataFusion error: {:?}", err);
        Self::internal(format!("Query error: {}", err))
    }
}

impl std::error::Error for ApiError {}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.status, self.message)
    }
}

// Convenience macros for quick error creation
#[macro_export]
macro_rules! internal {
    ($($arg:tt)*) => { $crate::error::ApiError::internal(format!($($arg)*)) };
}

#[macro_export]
macro_rules! not_found {
    ($($arg:tt)*) => { $crate::error::ApiError::not_found(format!($($arg)*)) };
}

#[macro_export]
macro_rules! bad_request {
    ($($arg:tt)*) => { $crate::error::ApiError::bad_request(format!($($arg)*)) };
}

#[macro_export]
macro_rules! unauthorized {
    ($($arg:tt)*) => { $crate::error::ApiError::unauthorized(format!($($arg)*)) };
}

#[macro_export]
macro_rules! forbidden {
    ($($arg:tt)*) => { $crate::error::ApiError::forbidden(format!($($arg)*)) };
}

// Legacy type alias for backward compatibility during migration
pub type InternalError = ApiError;
pub type AuthorizationError = ApiError;
