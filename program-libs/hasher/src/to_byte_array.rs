use crate::{Hasher, HasherError, Poseidon};

pub trait ToByteArray {
    const NUM_FIELDS: usize;
    const IS_PRIMITIVE: bool = false;
    fn to_byte_array(&self) -> Result<[u8; 32], HasherError>;

    fn to_byte_arrays<const NUM_FIELDS: usize>(
        &self,
    ) -> Result<[[u8; 32]; NUM_FIELDS], HasherError>;
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
                result[32 - std::mem::size_of::<$int_ty>()..].copy_from_slice(&bytes);
                Ok(result)
            }

            /// Big endian representation of $int_ty.
            fn to_byte_arrays<const NUM_FIELDS: usize>(
                &self,
            ) -> Result<[[u8; 32]; NUM_FIELDS], HasherError> {
                if Self::NUM_FIELDS != NUM_FIELDS {
                    return Err(HasherError::InvalidNumFields);
                }
                Ok([self.to_byte_array()?; NUM_FIELDS])
            }
        }
    };
}

#[cfg(any(
    all(
        feature = "solana",
        not(feature = "anchor"),
        not(feature = "pinocchio")
    ),
    all(
        feature = "anchor",
        not(feature = "solana"),
        not(feature = "pinocchio")
    )
))]
impl ToByteArray for crate::Pubkey {
    const NUM_FIELDS: usize = 1;

    fn to_byte_array(&self) -> Result<[u8; 32], HasherError> {
        Ok(self.to_bytes())
    }

    fn to_byte_arrays<const NUM_FIELDS: usize>(
        &self,
    ) -> Result<[[u8; 32]; NUM_FIELDS], HasherError> {
        if Self::NUM_FIELDS != NUM_FIELDS {
            return Err(HasherError::InvalidNumFields);
        }
        Ok([self.to_byte_array()?; NUM_FIELDS])
    }
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
                byte_array[32 - std::mem::size_of::<T>() - 1] = 1;
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

    /// Some(PrimitiveType) prefixed with 1 byte
    /// Some(T) -> Poseidon::hash(T::to_byte_array())
    /// None -> [0u8;32]
    fn to_byte_arrays<const NUM_FIELDS: usize>(
        &self,
    ) -> Result<[[u8; 32]; NUM_FIELDS], HasherError> {
        if Self::NUM_FIELDS != NUM_FIELDS {
            return Err(HasherError::InvalidNumFields);
        }
        Ok([self.to_byte_array()?; NUM_FIELDS])
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

    /// Big endian representation of bool.
    fn to_byte_arrays<const NUM_FIELDS: usize>(
        &self,
    ) -> Result<[[u8; 32]; NUM_FIELDS], HasherError> {
        if Self::NUM_FIELDS != NUM_FIELDS {
            return Err(HasherError::InvalidNumFields);
        }
        Ok([self.to_byte_array()?; NUM_FIELDS])
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
impl_to_byte_array_for_integer_type!(isize);
impl_to_byte_array_for_integer_type!(usize);
impl_to_byte_array_for_integer_type!(i128);
impl_to_byte_array_for_integer_type!(u128);

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

                fn to_byte_arrays<const NUM_FIELDS: usize>(
                    &self,
                ) -> Result<[[u8; 32]; NUM_FIELDS], HasherError> {
                    if Self::NUM_FIELDS != NUM_FIELDS {
                        return Err(HasherError::InvalidNumFields);
                    }

                    #[allow(clippy::unnecessary_to_owned)]
                    let element_arrays = [$(self[$index].to_byte_array()?),*];

                    let mut result = [[0u8; 32]; NUM_FIELDS];
                    for i in 0..NUM_FIELDS {
                        result[i] = element_arrays[i];
                    }

                    Ok(result)
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

            fn to_byte_arrays<const NUM_FIELDS: usize>(
                &self,
            ) -> Result<[[u8; 32]; NUM_FIELDS], HasherError> {
                if Self::NUM_FIELDS != NUM_FIELDS {
                    return Err(HasherError::InvalidNumFields);
                }
                Ok([self.to_byte_array()?; NUM_FIELDS])
            }
        }
    };
}

// Implement for common array sizes until 31 so that it is less than field size.
impl_to_byte_array_for_u8_array!(0);
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

    /// Max allowed String length is 31 bytes.
    /// For longer strings hash to field size or provide a custom implementation.
    fn to_byte_arrays<const NUM_FIELDS: usize>(
        &self,
    ) -> Result<[[u8; 32]; NUM_FIELDS], HasherError> {
        if Self::NUM_FIELDS != NUM_FIELDS {
            return Err(HasherError::InvalidNumFields);
        }
        Ok([self.to_byte_array()?; NUM_FIELDS])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_to_byte_array_integers() {
        // i8 tests
        let i8_min_result = i8::MIN.to_byte_array().unwrap();
        let mut expected_i8_min = [0u8; 32];
        expected_i8_min[31] = 128; // i8::MIN.to_be_bytes() = [128]
        assert_eq!(i8_min_result, expected_i8_min);

        let i8_max_result = i8::MAX.to_byte_array().unwrap();
        let mut expected_i8_max = [0u8; 32];
        expected_i8_max[31] = 127; // i8::MAX.to_be_bytes() = [127]
        assert_eq!(i8_max_result, expected_i8_max);

        // u8 tests
        let u8_min_result = u8::MIN.to_byte_array().unwrap();
        let mut expected_u8_min = [0u8; 32];
        expected_u8_min[31] = 0; // u8::MIN.to_be_bytes() = [0]
        assert_eq!(u8_min_result, expected_u8_min);

        let u8_max_result = u8::MAX.to_byte_array().unwrap();
        let mut expected_u8_max = [0u8; 32];
        expected_u8_max[31] = 255; // u8::MAX.to_be_bytes() = [255]
        assert_eq!(u8_max_result, expected_u8_max);

        // i16 tests
        let i16_min_result = i16::MIN.to_byte_array().unwrap();
        let mut expected_i16_min = [0u8; 32];
        expected_i16_min[30..32].copy_from_slice(&i16::MIN.to_be_bytes()); // [128, 0]
        assert_eq!(i16_min_result, expected_i16_min);

        let i16_max_result = i16::MAX.to_byte_array().unwrap();
        let mut expected_i16_max = [0u8; 32];
        expected_i16_max[30..32].copy_from_slice(&i16::MAX.to_be_bytes()); // [127, 255]
        assert_eq!(i16_max_result, expected_i16_max);

        // u16 tests
        let u16_min_result = u16::MIN.to_byte_array().unwrap();
        let mut expected_u16_min = [0u8; 32];
        expected_u16_min[30..32].copy_from_slice(&u16::MIN.to_be_bytes()); // [0, 0]
        assert_eq!(u16_min_result, expected_u16_min);

        let u16_max_result = u16::MAX.to_byte_array().unwrap();
        let mut expected_u16_max = [0u8; 32];
        expected_u16_max[30..32].copy_from_slice(&u16::MAX.to_be_bytes()); // [255, 255]
        assert_eq!(u16_max_result, expected_u16_max);

        // i32 tests
        let i32_min_result = i32::MIN.to_byte_array().unwrap();
        let mut expected_i32_min = [0u8; 32];
        expected_i32_min[28..32].copy_from_slice(&i32::MIN.to_be_bytes()); // [128, 0, 0, 0]
        assert_eq!(i32_min_result, expected_i32_min);

        let i32_max_result = i32::MAX.to_byte_array().unwrap();
        let mut expected_i32_max = [0u8; 32];
        expected_i32_max[28..32].copy_from_slice(&i32::MAX.to_be_bytes()); // [127, 255, 255, 255]
        assert_eq!(i32_max_result, expected_i32_max);

        // u32 tests
        let u32_min_result = u32::MIN.to_byte_array().unwrap();
        let mut expected_u32_min = [0u8; 32];
        expected_u32_min[28..32].copy_from_slice(&u32::MIN.to_be_bytes()); // [0, 0, 0, 0]
        assert_eq!(u32_min_result, expected_u32_min);

        let u32_max_result = u32::MAX.to_byte_array().unwrap();
        let mut expected_u32_max = [0u8; 32];
        expected_u32_max[28..32].copy_from_slice(&u32::MAX.to_be_bytes()); // [255, 255, 255, 255]
        assert_eq!(u32_max_result, expected_u32_max);

        // i64 tests
        let i64_min_result = i64::MIN.to_byte_array().unwrap();
        let mut expected_i64_min = [0u8; 32];
        expected_i64_min[24..32].copy_from_slice(&i64::MIN.to_be_bytes()); // [128, 0, 0, 0, 0, 0, 0, 0]
        assert_eq!(i64_min_result, expected_i64_min);

        let i64_max_result = i64::MAX.to_byte_array().unwrap();
        let mut expected_i64_max = [0u8; 32];
        expected_i64_max[24..32].copy_from_slice(&i64::MAX.to_be_bytes()); // [127, 255, 255, 255, 255, 255, 255, 255]
        assert_eq!(i64_max_result, expected_i64_max);

        // u64 tests
        let u64_min_result = u64::MIN.to_byte_array().unwrap();
        let mut expected_u64_min = [0u8; 32];
        expected_u64_min[24..32].copy_from_slice(&u64::MIN.to_be_bytes()); // [0, 0, 0, 0, 0, 0, 0, 0]
        assert_eq!(u64_min_result, expected_u64_min);

        let u64_max_result = u64::MAX.to_byte_array().unwrap();
        let mut expected_u64_max = [0u8; 32];
        expected_u64_max[24..32].copy_from_slice(&u64::MAX.to_be_bytes()); // [255, 255, 255, 255, 255, 255, 255, 255]
        assert_eq!(u64_max_result, expected_u64_max);

        // i128 tests
        let i128_min_result = i128::MIN.to_byte_array().unwrap();
        let mut expected_i128_min = [0u8; 32];
        expected_i128_min[16..32].copy_from_slice(&i128::MIN.to_be_bytes()); // [128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        assert_eq!(i128_min_result, expected_i128_min);

        let i128_max_result = i128::MAX.to_byte_array().unwrap();
        let mut expected_i128_max = [0u8; 32];
        expected_i128_max[16..32].copy_from_slice(&i128::MAX.to_be_bytes()); // [127, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255]
        assert_eq!(i128_max_result, expected_i128_max);

        // u128 tests
        let u128_min_result = u128::MIN.to_byte_array().unwrap();
        let mut expected_u128_min = [0u8; 32];
        expected_u128_min[16..32].copy_from_slice(&u128::MIN.to_be_bytes()); // [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        assert_eq!(u128_min_result, expected_u128_min);

        let u128_max_result = u128::MAX.to_byte_array().unwrap();
        let mut expected_u128_max = [0u8; 32];
        expected_u128_max[16..32].copy_from_slice(&u128::MAX.to_be_bytes()); // [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255]
        assert_eq!(u128_max_result, expected_u128_max);
    }

    #[test]
    fn test_to_byte_array_primitives() {
        // Test bool::to_byte_array
        let bool_false_result = false.to_byte_array().unwrap();
        let mut expected_bool_false = [0u8; 32];
        expected_bool_false[31] = 0;
        assert_eq!(bool_false_result, expected_bool_false);

        let bool_true_result = true.to_byte_array().unwrap();
        let mut expected_bool_true = [0u8; 32];
        expected_bool_true[31] = 1;
        assert_eq!(bool_true_result, expected_bool_true);
    }

    #[test]
    fn test_to_byte_array_option() {
        // Very important property - `None` and `Some(0)` always have to be
        // different and should produce different hashes!

        // Test Option<u8>::to_byte_array
        let u8_none: Option<u8> = None;
        let u8_none_result = u8_none.to_byte_array().unwrap();
        assert_eq!(u8_none_result, [0u8; 32]);

        let u8_some_zero: Option<u8> = Some(0);
        let u8_some_zero_result = u8_some_zero.to_byte_array().unwrap();
        let mut expected_u8_some_zero = [0u8; 32];
        expected_u8_some_zero[32 - std::mem::size_of::<u8>() - 1] = 1; // Mark as Some
        assert_eq!(u8_some_zero_result, expected_u8_some_zero);

        // Test Option<u16>::to_byte_array
        let u16_none: Option<u16> = None;
        let u16_none_result = u16_none.to_byte_array().unwrap();
        assert_eq!(u16_none_result, [0u8; 32]);

        let u16_some_zero: Option<u16> = Some(0);
        let u16_some_zero_result = u16_some_zero.to_byte_array().unwrap();
        let mut expected_u16_some_zero = [0u8; 32];
        expected_u16_some_zero[32 - std::mem::size_of::<u16>() - 1] = 1; // Mark as Some
        assert_eq!(u16_some_zero_result, expected_u16_some_zero);

        // Test Option<u32>::to_byte_array
        let u32_none: Option<u32> = None;
        let u32_none_result = u32_none.to_byte_array().unwrap();
        assert_eq!(u32_none_result, [0u8; 32]);

        let u32_some_zero: Option<u32> = Some(0);
        let u32_some_zero_result = u32_some_zero.to_byte_array().unwrap();
        let mut expected_u32_some_zero = [0u8; 32];
        expected_u32_some_zero[32 - std::mem::size_of::<u32>() - 1] = 1; // Mark as Some
        assert_eq!(u32_some_zero_result, expected_u32_some_zero);

        // Test Option<u64>::to_byte_array
        let u64_none: Option<u64> = None;
        let u64_none_result = u64_none.to_byte_array().unwrap();
        assert_eq!(u64_none_result, [0u8; 32]);

        let u64_some_zero: Option<u64> = Some(0);
        let u64_some_zero_result = u64_some_zero.to_byte_array().unwrap();
        let mut expected_u64_some_zero = [0u8; 32];
        expected_u64_some_zero[32 - std::mem::size_of::<u64>() - 1] = 1; // Mark as Some
        assert_eq!(u64_some_zero_result, expected_u64_some_zero);

        // Test Option<u128>::to_byte_array
        let u128_none: Option<u128> = None;
        let u128_none_result = u128_none.to_byte_array().unwrap();
        assert_eq!(u128_none_result, [0u8; 32]);

        let u128_some_zero: Option<u128> = Some(0);
        let u128_some_zero_result = u128_some_zero.to_byte_array().unwrap();
        let mut expected_u128_some_zero = [0u8; 32];
        expected_u128_some_zero[32 - std::mem::size_of::<u128>() - 1] = 1; // Mark as Some
        assert_eq!(u128_some_zero_result, expected_u128_some_zero);
    }

    #[test]
    fn test_to_byte_arrays() {
        // Test to_byte_arrays for u32
        let u32_value = 42u32;
        let arrays = u32_value.to_byte_arrays::<1>().unwrap();
        assert_eq!(arrays.len(), 1);

        let mut expected = [0u8; 32];
        expected[28..32].copy_from_slice(&u32_value.to_be_bytes());
        assert_eq!(arrays[0], expected);

        #[cfg(all(
            feature = "solana",
            not(feature = "anchor"),
            not(feature = "pinocchio")
        ))]
        {
            // Test to_byte_arrays for Pubkey
            let pubkey = crate::Pubkey::new_unique();
            let arrays = pubkey.to_byte_arrays::<1>().unwrap();
            assert_eq!(arrays.len(), 1);
            assert_eq!(arrays[0], pubkey.to_bytes());
        }
        #[cfg(all(
            feature = "anchor",
            not(feature = "solana"),
            not(feature = "pinocchio")
        ))]
        {
            // Test to_byte_arrays for Pubkey
            let pubkey = crate::Pubkey::new_unique();
            let arrays = pubkey.to_byte_arrays::<1>().unwrap();
            assert_eq!(arrays.len(), 1);
            assert_eq!(arrays[0], pubkey.to_bytes());
        }

        // Test to_byte_arrays for bool
        let bool_value = true;
        let arrays = bool_value.to_byte_arrays::<1>().unwrap();
        assert_eq!(arrays.len(), 1);

        let mut expected = [0u8; 32];
        expected[31] = 1;
        assert_eq!(arrays[0], expected);

        // Test to_byte_arrays for Option<bool>
        {
            let bool_value = Some(true);
            let arrays = bool_value.to_byte_arrays::<1>().unwrap();
            assert_eq!(arrays.len(), 1);

            let mut expected = [0u8; 32];
            expected[31] = 1;
            expected[30] = 1;
            assert_eq!(arrays[0], expected);
        }
        {
            let bool_value = Some(false);
            let arrays = bool_value.to_byte_arrays::<1>().unwrap();
            assert_eq!(arrays.len(), 1);

            let mut expected = [0u8; 32];
            expected[30] = 1;
            assert_eq!(arrays[0], expected);
        }
        {
            let bool_value: Option<bool> = None;
            let arrays = bool_value.to_byte_arrays::<1>().unwrap();
            assert_eq!(arrays.len(), 1);

            let expected = [0u8; 32];
            assert_eq!(arrays[0], expected);
        }
    }

    #[test]
    fn test_to_byte_array_u8_arrays() {
        // Test with empty array
        let empty_arr: [u8; 0] = [];
        let result = empty_arr.to_byte_array().unwrap();
        let expected = [0u8; 32];
        assert_eq!(result, expected);

        // Test with single element array
        let single_element_arr: [u8; 1] = [255];
        let result = single_element_arr.to_byte_array().unwrap();
        let mut expected = [0u8; 32];
        expected[31] = 255;
        assert_eq!(result, expected);

        // Test with multi-element array
        let multi_element_arr: [u8; 4] = [1, 2, 3, 4];
        let result = multi_element_arr.to_byte_array().unwrap();
        let mut expected = [0u8; 32];
        expected[32 - 4..].copy_from_slice(&multi_element_arr);
        assert_eq!(result, expected);

        // Test with full 32-byte array
        let full_arr: [u8; 31] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31,
        ];
        let result = full_arr.to_byte_array().unwrap();
        assert_eq!(result[0], 0);
        assert_eq!(&result[1..], full_arr.as_slice());
    }

    #[test]
    fn test_to_byte_array_string() {
        // Test with empty string
        let empty_string = "".to_string();
        let result = empty_string.to_byte_array().unwrap();
        let expected = [0u8; 32];
        assert_eq!(result, expected);

        // Test with short string
        let short_string = "foobar".to_string();
        let result = short_string.to_byte_array().unwrap();
        let mut expected = [0u8; 32];
        expected[32 - 6..].copy_from_slice(b"foobar");
        assert_eq!(result, expected);

        // Test with longer string that gets truncated
        let long_string =
            "this is a string that is longer than 32 bytes and will be fail".to_string();
        let byte_len = long_string.as_bytes().len();
        let result = long_string.to_byte_array();
        assert_eq!(result, Err(HasherError::InvalidInputLength(31, byte_len)));
    }

    // Define a custom type for testing the specific type array macro
    #[derive(Default)]
    struct TestType {
        value: u32,
    }

    impl TestType {
        fn new(value: u32) -> Self {
            Self { value }
        }
    }

    impl ToByteArray for TestType {
        const NUM_FIELDS: usize = 1;
        const IS_PRIMITIVE: bool = true;

        fn to_byte_array(&self) -> Result<[u8; 32], HasherError> {
            let mut result = [0u8; 32];
            // Store the value in the last 4 bytes of the array
            result[28..32].copy_from_slice(&self.value.to_be_bytes());
            Ok(result)
        }

        fn to_byte_arrays<const NUM_FIELDS: usize>(
            &self,
        ) -> Result<[[u8; 32]; NUM_FIELDS], HasherError> {
            if Self::NUM_FIELDS != NUM_FIELDS {
                return Err(HasherError::InvalidNumFields);
            }
            Ok([self.to_byte_array()?; NUM_FIELDS])
        }
    }

    // Use the new macro to implement ToByteArray for arrays of TestType
    impl_to_byte_array_for_array! {
        TestType,
        1 => [0];
        2 => [0, 1];
        3 => [0, 1, 2]
    }

    #[test]
    fn test_specific_type_array_implementation() {
        // Test array of size 1
        let test_array_1 = [TestType::new(42)];
        let result_1 = test_array_1.to_byte_array().unwrap();

        // Expected: Poseidon hash of a single element
        let mut expected_element = [0u8; 32];
        expected_element[28..32].copy_from_slice(&42u32.to_be_bytes());
        let expected_1 = Poseidon::hashv(&[&expected_element]).unwrap();

        assert_eq!(result_1, expected_1);

        // Test array of size 2
        let test_array_2 = [TestType::new(10), TestType::new(20)];
        let result_2 = test_array_2.to_byte_array().unwrap();

        // Expected: Poseidon hash of two elements
        let mut elem1 = [0u8; 32];
        let mut elem2 = [0u8; 32];
        elem1[28..32].copy_from_slice(&10u32.to_be_bytes());
        elem2[28..32].copy_from_slice(&20u32.to_be_bytes());
        let expected_2 = Poseidon::hashv(&[&elem1, &elem2]).unwrap();

        assert_eq!(result_2, expected_2);

        // Test array of size 3
        let test_array_3 = [TestType::new(1), TestType::new(2), TestType::new(3)];
        let result_3 = test_array_3.to_byte_array().unwrap();

        // Expected: Poseidon hash of three elements
        let mut elem1 = [0u8; 32];
        let mut elem2 = [0u8; 32];
        let mut elem3 = [0u8; 32];
        elem1[28..32].copy_from_slice(&1u32.to_be_bytes());
        elem2[28..32].copy_from_slice(&2u32.to_be_bytes());
        elem3[28..32].copy_from_slice(&3u32.to_be_bytes());
        let expected_3 = Poseidon::hashv(&[&elem1, &elem2, &elem3]).unwrap();

        assert_eq!(result_3, expected_3);
    }
}
