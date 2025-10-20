#[cfg(feature = "alloc")]
use crate::String;
use crate::{Hasher, HasherError, Poseidon};

pub trait ToByteArray {
    const NUM_FIELDS: usize;
    const IS_PRIMITIVE: bool = false;
    fn to_byte_array(&self) -> Result<[u8; 32], HasherError>;
}

macro_rules! impl_to_byte_array_for_integer_type {
    ($int_ty:ty) => {
        impl ToByteArray for $int_ty {
            const IS_PRIMITIVE: bool = true;
            const NUM_FIELDS: usize = 1;

            /// Big endian representation of $int_ty.
            fn to_byte_array(&self) -> Result<[u8; 32], HasherError> {
                let bytes = self.to_be_bytes();
                let mut result = [0; 32];
                result[32 - core::mem::size_of::<$int_ty>()..].copy_from_slice(&bytes);
                Ok(result)
            }
        }
    };
}

impl<T: ToByteArray> ToByteArray for Option<T> {
    const NUM_FIELDS: usize = 1;

    /// Some(PrimitiveType) prefixed with 1 byte
    /// Some(T) -> Poseidon::hash(T::to_byte_array())
    /// None -> [0u8;32]
    fn to_byte_array(&self) -> Result<[u8; 32], HasherError> {
        if let Some(value) = &self {
            let byte_array = if T::IS_PRIMITIVE {
                let mut byte_array = value.to_byte_array()?;
                // Prefix with 1 to indicate Some().
                byte_array[32 - core::mem::size_of::<T>() - 1] = 1;
                byte_array
            } else {
                let byte_array = value.to_byte_array()?;
                Poseidon::hash(byte_array.as_slice())?
            };
            Ok(byte_array)
        } else {
            Ok([0; 32])
        }
    }
}

impl ToByteArray for bool {
    const NUM_FIELDS: usize = 1;
    const IS_PRIMITIVE: bool = true;

    /// Big endian representation of bool.
    fn to_byte_array(&self) -> Result<[u8; 32], HasherError> {
        let mut bytes = [0u8; 32];
        bytes[31] = *self as u8;
        Ok(bytes)
    }
}

impl_to_byte_array_for_integer_type!(i8);
impl_to_byte_array_for_integer_type!(u8);
impl_to_byte_array_for_integer_type!(i16);
impl_to_byte_array_for_integer_type!(u16);
impl_to_byte_array_for_integer_type!(i32);
impl_to_byte_array_for_integer_type!(u32);
impl_to_byte_array_for_integer_type!(i64);
impl_to_byte_array_for_integer_type!(u64);
impl_to_byte_array_for_integer_type!(i128);
impl_to_byte_array_for_integer_type!(u128);

// Macro for implementing ToByteArray for zero-copy types
#[cfg(feature = "zero-copy")]
macro_rules! impl_to_byte_array_for_zero_copy_type {
    ($zero_copy_type:ty, $primitive_type:ty) => {
        impl ToByteArray for $zero_copy_type {
            const IS_PRIMITIVE: bool = true;
            const NUM_FIELDS: usize = 1;

            fn to_byte_array(&self) -> Result<[u8; 32], HasherError> {
                let value: $primitive_type = (*self).into();
                value.to_byte_array()
            }
        }
    };
}

// ToByteArray implementations for zero-copy types
#[cfg(feature = "zero-copy")]
impl_to_byte_array_for_zero_copy_type!(zerocopy::little_endian::U16, u16);
#[cfg(feature = "zero-copy")]
impl_to_byte_array_for_zero_copy_type!(zerocopy::little_endian::U32, u32);
#[cfg(feature = "zero-copy")]
impl_to_byte_array_for_zero_copy_type!(zerocopy::little_endian::U64, u64);
#[cfg(feature = "zero-copy")]
impl_to_byte_array_for_zero_copy_type!(zerocopy::little_endian::I16, i16);
#[cfg(feature = "zero-copy")]
impl_to_byte_array_for_zero_copy_type!(zerocopy::little_endian::I32, i32);
#[cfg(feature = "zero-copy")]
impl_to_byte_array_for_zero_copy_type!(zerocopy::little_endian::I64, i64);

/// Example usage:
/// impl_to_byte_array_for_array! {
///     MyCustomType,
///     1 => [0],
///     2 => [0, 1]
/// }
#[macro_export]
macro_rules! impl_to_byte_array_for_array {
    // First specify the type T, then for each array, specify the length and indices
    ($t:ty, $( $len:literal => [$($index:tt),*] );* $(;)?) => {
        $(
            impl ToByteArray for [$t; $len] {
                const NUM_FIELDS: usize = $len;
                const IS_PRIMITIVE: bool = false;

                fn to_byte_array(&self) -> Result<[u8; 32], HasherError> {
                    let arrays = [$(self[$index].to_byte_array()?),*];
                    let slices = [$(arrays[$index].as_slice()),*];
                    Poseidon::hashv(&slices)
                }
            }
        )*
    }
}

// Implementation for [u8; N] arrays with N <= 31
macro_rules! impl_to_byte_array_for_u8_array {
    ($size:expr) => {
        impl ToByteArray for [u8; $size] {
            const NUM_FIELDS: usize = 1;

            fn to_byte_array(&self) -> Result<[u8; 32], HasherError> {
                let mut result = [0u8; 32];
                result[32 - $size..].copy_from_slice(self.as_slice());
                Ok(result)
            }
        }
    };
}

// Implement for common array sizes until 31 so that it is less than field size.
impl_to_byte_array_for_u8_array!(1);
impl_to_byte_array_for_u8_array!(2);
impl_to_byte_array_for_u8_array!(4);
impl_to_byte_array_for_u8_array!(5);
impl_to_byte_array_for_u8_array!(6);
impl_to_byte_array_for_u8_array!(7);
impl_to_byte_array_for_u8_array!(8);
impl_to_byte_array_for_u8_array!(9);
impl_to_byte_array_for_u8_array!(10);
impl_to_byte_array_for_u8_array!(11);
impl_to_byte_array_for_u8_array!(12);
impl_to_byte_array_for_u8_array!(13);
impl_to_byte_array_for_u8_array!(14);
impl_to_byte_array_for_u8_array!(15);
impl_to_byte_array_for_u8_array!(16);
impl_to_byte_array_for_u8_array!(17);
impl_to_byte_array_for_u8_array!(18);
impl_to_byte_array_for_u8_array!(19);
impl_to_byte_array_for_u8_array!(20);
impl_to_byte_array_for_u8_array!(21);
impl_to_byte_array_for_u8_array!(22);
impl_to_byte_array_for_u8_array!(23);
impl_to_byte_array_for_u8_array!(24);
impl_to_byte_array_for_u8_array!(25);
impl_to_byte_array_for_u8_array!(26);
impl_to_byte_array_for_u8_array!(27);
impl_to_byte_array_for_u8_array!(28);
impl_to_byte_array_for_u8_array!(29);
impl_to_byte_array_for_u8_array!(30);
impl_to_byte_array_for_u8_array!(31);

#[cfg(feature = "alloc")]
impl ToByteArray for String {
    const NUM_FIELDS: usize = 1;

    /// Max allowed String length is 31 bytes.
    /// For longer strings hash to field size or provide a custom implementation.
    fn to_byte_array(&self) -> Result<[u8; 32], HasherError> {
        let bytes = self.as_bytes();
        let mut result = [0u8; 32];
        let byte_len = bytes.len();
        if byte_len > 31 {
            return Err(HasherError::InvalidInputLength(31, bytes.len()));
        }
        result[32 - byte_len..].copy_from_slice(bytes);
        Ok(result)
    }
}
