use crate::{rtps::error::RtpsError, xtypes::error::XTypesError};
use alloc::{boxed::Box, format, string::String};
use core::{any::Any, error::Error, fmt::Display};

/// Result type returned by the different operations of the service
pub type DdsResult<T> = Result<T, DdsError>;

/// Errors that can be return by the different operations of the service
#[derive(Debug, PartialEq, Eq)]
pub enum DdsError {
    /// Generic, unspecified error.
    Error(String),
    /// Unsupported operation.
    Unsupported,
    /// Illegal parameter value.
    BadParameter,
    /// A pre-condition for the operation was not met.
    PreconditionNotMet(String),
    /// Service ran out of the resources needed to complete the operation.
    OutOfResources,
    /// Operation invoked on an Entity that is not yet enabled.
    NotEnabled,
    /// Application attempted to modify an immutable QosPolicy.
    ImmutablePolicy,
    /// Application specified a set of policies that are not consistent with each other.
    InconsistentPolicy,
    /// The object target of this operation has already been deleted.
    AlreadyDeleted,
    /// The operation timed out.
    Timeout,
    /// Indicates a transient situation where the operation did not
    /// return any data but there is no inherent error.
    NoData,
    /// An operation was invoked on an inappropriate object or
    /// at an inappropriate time (as determined by policies set by the
    /// specification or the Service implementation). There is no
    /// precondition that could be changed to make the operation
    /// succeed.
    IllegalOperation,
}

impl Display for DdsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Error(err) => write!(f, "unspecified: {err}"),
            Self::Unsupported => write!(f, "unsupported operation"),
            Self::BadParameter => write!(f, "illegal parameter value"),
            Self::PreconditionNotMet(err) => write!(f, "operation precondition not met: {err}"),
            Self::OutOfResources => write!(f, "insufficient resources to complete the operation"),
            Self::NotEnabled => write!(f, "operation not enabled"),
            Self::ImmutablePolicy => write!(f, "attempted to modify an immutable policy"),
            Self::InconsistentPolicy => write!(f, "specified a set of inconsistent policies"),
            Self::AlreadyDeleted => write!(f, "object of operation has been already deleted"),
            Self::Timeout => write!(f, "operation timed out"),
            Self::NoData => write!(f, "operation returned no data, but no error occurred"),
            Self::IllegalOperation => write!(f, "illegal operation"),
        }
    }
}

impl Error for DdsError {}

impl From<RtpsError> for DdsError {
    fn from(_value: RtpsError) -> Self {
        DdsError::Error(String::new())
    }
}

impl From<Box<dyn Any + Send + 'static>> for DdsError {
    fn from(_: Box<dyn Any + Send + 'static>) -> Self {
        DdsError::Error(String::from("Generic std error"))
    }
}

impl From<XTypesError> for DdsError {
    fn from(value: XTypesError) -> Self {
        DdsError::Error(format!("XTypesError: {value:?}"))
    }
}

/// Return code representing the different errors
pub type ReturnCode = i32;

/// Successful return code.
pub const RETCODE_OK: ReturnCode = 0;
/// Generic, unspecified error.
pub const RETCODE_ERROR: ReturnCode = 1;
/// Unsupported operation.
pub const RETCODE_UNSUPPORTED: ReturnCode = 2;
/// Illegal parameter value.
pub const RETCODE_BAD_PARAMETER: ReturnCode = 3;
/// A pre-condition for the operation was not met.
pub const RETCODE_PRECONDITION_NOT_MET: ReturnCode = 4;
/// Service ran out of the resources needed to complete the operation.
pub const RETCODE_OUT_OF_RESOURCES: ReturnCode = 5;
/// Operation invoked on an Entity that is not yet enabled.
pub const RETCODE_NOT_ENABLED: ReturnCode = 6;
/// Application attempted to modify an immutable QosPolicy.
pub const RETCODE_IMMUTABLE_POLICY: ReturnCode = 7;
/// Application specified a set of policies that are not consistent with each other.
pub const RETCODE_INCONSISTENT_POLICY: ReturnCode = 8;
/// The object target of this operation has already been deleted.
pub const RETCODE_ALREADY_DELETED: ReturnCode = 9;
/// The operation timed out.
pub const RETCODE_TIMEOUT: ReturnCode = 10;
/// Indicates a transient situation where the operation did not return any data but there is no inherent error.
pub const RETCODE_NO_DATA: ReturnCode = 11;
/// An operation was invoked on an inappropriate object or at an inappropriate time.
pub const RETCODE_ILLEGAL_OPERATION: ReturnCode = 12;

impl From<DdsError> for ReturnCode {
    fn from(e: DdsError) -> Self {
        match e {
            DdsError::Error(_) => RETCODE_ERROR,
            DdsError::Unsupported => RETCODE_UNSUPPORTED,
            DdsError::BadParameter => RETCODE_BAD_PARAMETER,
            DdsError::PreconditionNotMet(_) => RETCODE_PRECONDITION_NOT_MET,
            DdsError::OutOfResources => RETCODE_OUT_OF_RESOURCES,
            DdsError::NotEnabled => RETCODE_NOT_ENABLED,
            DdsError::ImmutablePolicy => RETCODE_IMMUTABLE_POLICY,
            DdsError::InconsistentPolicy => RETCODE_INCONSISTENT_POLICY,
            DdsError::AlreadyDeleted => RETCODE_ALREADY_DELETED,
            DdsError::Timeout => RETCODE_TIMEOUT,
            DdsError::NoData => RETCODE_NO_DATA,
            DdsError::IllegalOperation => RETCODE_ILLEGAL_OPERATION,
        }
    }
}
