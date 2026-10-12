use crate::{rtps_messages::error::RtpsMessageError, xtypes::error::XTypesError};

pub type RtpsResult<T> = Result<T, RtpsError>;

#[derive(Debug)]
pub enum RtpsError {
    Io,
    InvalidData,
    NotEnoughData,
    MessageError,
    XTypesError,
    ParameterNotFound,
}

impl From<XTypesError> for RtpsError {
    fn from(_: XTypesError) -> Self {
        RtpsError::XTypesError
    }
}

impl From<RtpsMessageError> for RtpsError {
    fn from(_: RtpsMessageError) -> Self {
        RtpsError::MessageError
    }
}
