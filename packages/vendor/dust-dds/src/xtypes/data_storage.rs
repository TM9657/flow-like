use crate::xtypes::{
    dynamic_type::DynamicData,
    error::{XTypesError, XTypesResult},
    type_support::TypeSupport,
};
use alloc::{string::String, vec::Vec};

/// Representation of data storage for dynamic types.
#[derive(Debug, Clone, PartialEq)]
pub enum DataStorage {
    /// Unsigned 8-bit integer.
    UInt8(u8),
    /// Signed 8-bit integer.
    Int8(i8),
    /// Unsigned 16-bit integer.
    UInt16(u16),
    /// Signed 16-bit integer.
    Int16(i16),
    /// Signed 32-bit integer.
    Int32(i32),
    /// Unsigned 32-bit integer.
    UInt32(u32),
    /// Signed 64-bit integer.
    Int64(i64),
    /// Unsigned 64-bit integer.
    UInt64(u64),
    /// Single-precision floating-point number.
    Float32(f32),
    /// Double-precision floating-point number.
    Float64(f64),
    /// 128-bit floating-point number.
    Float128(i128),
    /// 8-bit character.
    Char8(char),
    /// Boolean value.
    Boolean(bool),
    /// String value.
    String(String),
    /// Complex data value represented by a [`DynamicData`].
    ComplexValue(DynamicData<'static>),
    /// Sequence of unsigned 8-bit integers.
    SequenceUInt8(Vec<u8>),
    /// Sequence of signed 8-bit integers.
    SequenceInt8(Vec<i8>),
    /// Sequence of unsigned 16-bit integers.
    SequenceUInt16(Vec<u16>),
    /// Sequence of signed 16-bit integers.
    SequenceInt16(Vec<i16>),
    /// Sequence of signed 32-bit integers.
    SequenceInt32(Vec<i32>),
    /// Sequence of unsigned 32-bit integers.
    SequenceUInt32(Vec<u32>),
    /// Sequence of signed 64-bit integers.
    SequenceInt64(Vec<i64>),
    /// Sequence of unsigned 64-bit integers.
    SequenceUInt64(Vec<u64>),
    /// Sequence of single-precision floating-point numbers.
    SequenceFloat32(Vec<f32>),
    /// Sequence of double-precision floating-point numbers.
    SequenceFloat64(Vec<f64>),
    /// Sequence of 128-bit floating-point numbers.
    SequenceFloat128(Vec<i128>),
    /// Sequence of 8-bit characters.
    SequenceChar8(Vec<char>),
    /// Sequence of boolean values.
    SequenceBoolean(Vec<bool>),
    /// Sequence of string values.
    SequenceString(Vec<String>),
    /// Sequence of complex data values.
    SequenceComplexValue(Vec<DynamicData<'static>>),
}

/// Trait used to map Rust types to and from their corresponding [`DataStorage`] variants.
///
/// This trait is typically used by the derive macro to convert types into
/// their dynamic data representation format.
pub trait DataStorageMapping: Sized {
    /// Converts this type into its corresponding [`DataStorage`] variant.
    fn into_storage(self) -> DataStorage;

    /// Tries to convert a [`DataStorage`] variant back into this type.
    ///
    /// # Errors
    ///
    /// Returns [`XTypesError::InvalidType`] if the storage variant does not match.
    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self>;
}

impl DataStorageMapping for u8 {
    fn into_storage(self) -> DataStorage {
        DataStorage::UInt8(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::UInt8(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for i8 {
    fn into_storage(self) -> DataStorage {
        DataStorage::Int8(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::Int8(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for u16 {
    fn into_storage(self) -> DataStorage {
        DataStorage::UInt16(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::UInt16(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for i16 {
    fn into_storage(self) -> DataStorage {
        DataStorage::Int16(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::Int16(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for u32 {
    fn into_storage(self) -> DataStorage {
        DataStorage::UInt32(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::UInt32(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for i32 {
    fn into_storage(self) -> DataStorage {
        DataStorage::Int32(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::Int32(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for u64 {
    fn into_storage(self) -> DataStorage {
        DataStorage::UInt64(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::UInt64(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for i64 {
    fn into_storage(self) -> DataStorage {
        DataStorage::Int64(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::Int64(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for f32 {
    fn into_storage(self) -> DataStorage {
        DataStorage::Float32(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::Float32(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for f64 {
    fn into_storage(self) -> DataStorage {
        DataStorage::Float64(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::Float64(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for bool {
    fn into_storage(self) -> DataStorage {
        DataStorage::Boolean(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::Boolean(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for char {
    fn into_storage(self) -> DataStorage {
        DataStorage::Char8(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::Char8(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for String {
    fn into_storage(self) -> DataStorage {
        DataStorage::String(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::String(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for Vec<u8> {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceUInt8(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceUInt8(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for Vec<i8> {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceInt8(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceInt8(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for Vec<u16> {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceUInt16(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceUInt16(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for Vec<i16> {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceInt16(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceInt16(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for Vec<u32> {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceUInt32(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceUInt32(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for Vec<i32> {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceInt32(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceInt32(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for Vec<u64> {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceUInt64(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceUInt64(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for Vec<i64> {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceInt64(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceInt64(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for Vec<f32> {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceFloat32(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceFloat32(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for Vec<f64> {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceFloat64(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceFloat64(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for Vec<char> {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceChar8(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceChar8(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for Vec<bool> {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceBoolean(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceBoolean(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl DataStorageMapping for Vec<String> {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceString(self)
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceString(x) => Ok(x),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl<const N: usize> DataStorageMapping for [u8; N] {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceUInt8(self.to_vec())
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceUInt8(x) => {
                Self::try_from(x).map_err(|_| XTypesError::InvalidType)
            }
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl<const N: usize> DataStorageMapping for [i8; N] {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceInt8(self.to_vec())
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceInt8(x) => Self::try_from(x).map_err(|_| XTypesError::InvalidType),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl<const N: usize> DataStorageMapping for [u16; N] {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceUInt16(self.to_vec())
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceUInt16(x) => {
                Self::try_from(x).map_err(|_| XTypesError::InvalidType)
            }
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl<const N: usize> DataStorageMapping for [i16; N] {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceInt16(self.to_vec())
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceInt16(x) => {
                Self::try_from(x).map_err(|_| XTypesError::InvalidType)
            }
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl<const N: usize> DataStorageMapping for [u32; N] {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceUInt32(self.to_vec())
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceUInt32(x) => {
                Self::try_from(x).map_err(|_| XTypesError::InvalidType)
            }
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl<const N: usize> DataStorageMapping for [i32; N] {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceInt32(self.to_vec())
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceInt32(x) => {
                Self::try_from(x).map_err(|_| XTypesError::InvalidType)
            }
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl<const N: usize> DataStorageMapping for [u64; N] {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceUInt64(self.to_vec())
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceUInt64(x) => {
                Self::try_from(x).map_err(|_| XTypesError::InvalidType)
            }
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl<const N: usize> DataStorageMapping for [i64; N] {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceInt64(self.to_vec())
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceInt64(x) => {
                Self::try_from(x).map_err(|_| XTypesError::InvalidType)
            }
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl<const N: usize> DataStorageMapping for [f32; N] {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceFloat32(self.to_vec())
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceFloat32(x) => {
                Self::try_from(x).map_err(|_| XTypesError::InvalidType)
            }
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl<const N: usize> DataStorageMapping for [f64; N] {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceFloat64(self.to_vec())
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceFloat64(x) => {
                Self::try_from(x).map_err(|_| XTypesError::InvalidType)
            }
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl<const N: usize> DataStorageMapping for [char; N] {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceChar8(self.to_vec())
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceChar8(x) => {
                Self::try_from(x).map_err(|_| XTypesError::InvalidType)
            }
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl<const N: usize> DataStorageMapping for [bool; N] {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceBoolean(self.to_vec())
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceBoolean(x) => {
                Self::try_from(x).map_err(|_| XTypesError::InvalidType)
            }
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl<const N: usize> DataStorageMapping for [String; N] {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceString(self.to_vec())
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        match data_storage {
            DataStorage::SequenceString(x) => {
                Self::try_from(x).map_err(|_| XTypesError::InvalidType)
            }
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl<T: TypeSupport> DataStorageMapping for Vec<T> {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceComplexValue(
            self.into_iter()
                .map(TypeSupport::create_dynamic_sample)
                .collect(),
        )
    }

    fn try_from_storage(mut data_storage: DataStorage) -> XTypesResult<Self> {
        match &mut data_storage {
            DataStorage::SequenceComplexValue(x) => x
                .iter_mut()
                .map(T::create_sample)
                .collect::<Option<Vec<_>>>()
                .ok_or(XTypesError::InvalidData),
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl<T: TypeSupport, const N: usize> DataStorageMapping for [T; N] {
    fn into_storage(self) -> DataStorage {
        DataStorage::SequenceComplexValue(
            self.into_iter()
                .map(TypeSupport::create_dynamic_sample)
                .collect(),
        )
    }

    fn try_from_storage(mut data_storage: DataStorage) -> XTypesResult<Self> {
        match &mut data_storage {
            DataStorage::SequenceComplexValue(x) => {
                let vec = x
                    .iter_mut()
                    .map(T::create_sample)
                    .collect::<Option<Vec<_>>>()
                    .ok_or(XTypesError::InvalidData)?;
                Self::try_from(vec).map_err(|_| XTypesError::InvalidType)
            }
            _ => Err(XTypesError::InvalidType),
        }
    }
}

impl<T: DataStorageMapping> DataStorageMapping for Option<T> {
    fn into_storage(self) -> DataStorage {
        T::into_storage(self.expect("Only options with value are converted. This usually indicats a member annotation #[dust_dds(optional)] is missing."))
    }

    fn try_from_storage(data_storage: DataStorage) -> XTypesResult<Self> {
        Ok(Some(T::try_from_storage(data_storage)?))
    }
}

impl<T: TypeSupport> DataStorageMapping for T {
    fn into_storage(self) -> DataStorage {
        DataStorage::ComplexValue(self.create_dynamic_sample())
    }

    fn try_from_storage(mut data_storage: DataStorage) -> XTypesResult<Self> {
        match &mut data_storage {
            DataStorage::ComplexValue(x) => T::create_sample(x).ok_or(XTypesError::InvalidData),
            _ => Err(XTypesError::InvalidType),
        }
    }
}
