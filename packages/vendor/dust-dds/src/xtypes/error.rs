/// Specialized [`Result`] type for XTypes operations.
pub type XTypesResult<T> = Result<T, XTypesError>;

/// XTypes error conditions
#[derive(Debug, PartialEq)]
pub enum XTypesError {
    /// The operation ran out of memory.
    OutOfMemory,
    /// The data provided is invalid.
    InvalidData,
    /// The type provided or encountered is invalid or mismatched.
    InvalidType,
    /// The parameter ID (PID) was not found.
    PidNotFound(u16),
    /// The member ID or identifier is invalid or not found.
    InvalidId(u32),
    /// The index is out of bounds or invalid.
    InvalidIndex(u32),
    /// The member name or identifier name is invalid or not found.
    InvalidName,
    /// Not enough data was available to complete the operation.
    NotEnoughData,
    /// The representation identifier is not supported.
    NotSupported([u8; 2]),
    /// An operation was invoked on an inappropriate object or
    /// at an inappropriate time (as determined by policies set by the
    /// specification or the Service implementation). There is no
    /// precondition that could be changed to make the operation
    /// succeed.
    IllegalOperation,
}
