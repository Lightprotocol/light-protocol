use std::{mem, slice};

/// A trait providing [`as_byte_vec()`](AsByteVec::as_byte_vec) method for types which
/// are used inside compressed accounts.
pub trait AsByteVec {
    fn as_byte_vec(&self) -> Vec<Vec<u8>>;
}

macro_rules! impl_as_byte_vec_for_integer_type {
    ($int_ty:ty) => {
        impl AsByteVec for $int_ty {
            fn as_byte_vec(&self) -> Vec<Vec<u8>> {
                vec![self.to_le_bytes().to_vec()]
            }
        }
    };
}

macro_rules! impl_as_byte_vec_for_primitive_type {
    ($int_ty:ty) => {
        impl AsByteVec for $int_ty {
            fn as_byte_vec(&self) -> Vec<Vec<u8>> {
                let len = mem::size_of_val(self);
                let self_ptr: *const Self = self;
                // SAFETY:
                // - All the primitive types we implement this macro for have
                //   an exact size (`len`). There is no chance of reads out of
                //   bounds.
                // - Casting Rust primitives to bytes works fine, there is no
                //   chance of undefined behavior.
                // - Unfortunately, there is no way to achieve the similar
                //   result with fully safe code. If we tried to do anything
                //   like `&self.to_le_bytes()` or `self.to_le_bytes().as_slice()`,
                //   compiler would complain with "cannot return reference to
                //   temporary value".
                let self_byte_slice = unsafe { slice::from_raw_parts(self_ptr.cast::<u8>(), len) };
                vec![self_byte_slice.to_vec()]
            }
        }
    };
}

impl_as_byte_vec_for_integer_type!(i8);
impl_as_byte_vec_for_integer_type!(u8);
impl_as_byte_vec_for_integer_type!(i16);
impl_as_byte_vec_for_integer_type!(u16);
impl_as_byte_vec_for_integer_type!(i32);
impl_as_byte_vec_for_integer_type!(u32);
impl_as_byte_vec_for_integer_type!(i64);
impl_as_byte_vec_for_integer_type!(u64);
impl_as_byte_vec_for_integer_type!(isize);
impl_as_byte_vec_for_integer_type!(usize);
impl_as_byte_vec_for_integer_type!(i128);
impl_as_byte_vec_for_integer_type!(u128);

impl_as_byte_vec_for_primitive_type!(bool);

impl<T> AsByteVec for Option<T>
where
    T: AsByteVec,
{
    fn as_byte_vec(&self) -> Vec<Vec<u8>> {
        match self {
            Some(hashable) => {
                let mut bytes = hashable.as_byte_vec();
                bytes.reserve(1);
                bytes.insert(0, vec![1]);
                bytes
            }
            None => vec![vec![0]],
        }
    }
}

impl<const N: usize> AsByteVec for [u8; N] {
    fn as_byte_vec(&self) -> Vec<Vec<u8>> {
        vec![self.to_vec()]
    }
}

impl AsByteVec for String {
    fn as_byte_vec(&self) -> Vec<Vec<u8>> {
        vec![self.as_bytes().to_vec()]
    }
}

#[cfg(feature = "solana")]
impl AsByteVec for solana_program::pubkey::Pubkey {
    fn as_byte_vec(&self) -> Vec<Vec<u8>> {
        vec![self.to_bytes().to_vec()]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_as_byte_vec_integers() {
        let i8_min: &dyn AsByteVec = &i8::MIN;
        let i8_min_bytes = i8_min.as_byte_vec();
        assert_eq!(i8_min_bytes, &[&[128]]);
        assert_eq!(i8_min_bytes, &[i8::MIN.to_le_bytes()]);
        let i8_max: &dyn AsByteVec = &i8::MAX;
        let i8_max_bytes = i8_max.as_byte_vec();
        assert_eq!(i8_max_bytes, &[&[127]]);
        assert_eq!(i8_max_bytes, &[i8::MAX.to_le_bytes()]);

        let u8_min: &dyn AsByteVec = &u8::MIN;
        let u8_min_bytes = u8_min.as_byte_vec();
        assert_eq!(u8_min_bytes, &[&[0]]);
        assert_eq!(u8_min_bytes, &[u8::MIN.to_le_bytes()]);
        let u8_max: &dyn AsByteVec = &u8::MAX;
        let u8_max_bytes = u8_max.as_byte_vec();
        assert_eq!(u8_max_bytes, &[&[255]]);
        assert_eq!(u8_max_bytes, &[u8::MAX.to_le_bytes()]);

        let i16_min: &dyn AsByteVec = &i16::MIN;
        let i16_min_bytes = i16_min.as_byte_vec();
        assert_eq!(i16_min_bytes, &[&[0, 128]]);
        assert_eq!(i16_min_bytes, &[&i16::MIN.to_le_bytes()]);
        let i16_max: &dyn AsByteVec = &i16::MAX;
        let i16_max_bytes = i16_max.as_byte_vec();
        assert_eq!(i16_max_bytes, &[&[255, 127]]);
        assert_eq!(i16_max_bytes, &[i16::MAX.to_le_bytes()]);

        let u16_min: &dyn AsByteVec = &u16::MIN;
        let u16_min_bytes = u16_min.as_byte_vec();
        assert_eq!(u16_min_bytes, &[&[0, 0]]);
        assert_eq!(u16_min_bytes, &[u16::MIN.to_le_bytes()]);
        let u16_max: &dyn AsByteVec = &u16::MAX;
        let u16_max_bytes = u16_max.as_byte_vec();
        assert_eq!(u16_max_bytes, &[&[255, 255]]);
        assert_eq!(u16_max_bytes, &[u16::MAX.to_le_bytes()]);

        let i32_min: &dyn AsByteVec = &i32::MIN;
        let i32_min_bytes = i32_min.as_byte_vec();
        assert_eq!(i32_min_bytes, &[&[0, 0, 0, 128]]);
        assert_eq!(i32_min_bytes, &[i32::MIN.to_le_bytes()]);
        let i32_max: &dyn AsByteVec = &i32::MAX;
        let i32_max_bytes = i32_max.as_byte_vec();
        assert_eq!(i32_max_bytes, &[&[255, 255, 255, 127]]);
        assert_eq!(i32_max_bytes, &[i32::MAX.to_le_bytes()]);

        let u32_min: &dyn AsByteVec = &u32::MIN;
        let u32_min_bytes = u32_min.as_byte_vec();
        assert_eq!(u32_min_bytes, &[&[0, 0, 0, 0]]);
        assert_eq!(u32_min_bytes, &[u32::MIN.to_le_bytes()]);
        let u32_max: &dyn AsByteVec = &u32::MAX;
        let u32_max_bytes = u32_max.as_byte_vec();
        assert_eq!(u32_max_bytes, &[&[255, 255, 255, 255]]);
        assert_eq!(u32_max_bytes, &[u32::MAX.to_le_bytes()]);

        let i64_min: &dyn AsByteVec = &i64::MIN;
        let i64_min_bytes = i64_min.as_byte_vec();
        assert_eq!(i64_min_bytes, &[&[0, 0, 0, 0, 0, 0, 0, 128]]);
        assert_eq!(i64_min_bytes, &[i64::MIN.to_le_bytes()]);
        let i64_max: &dyn AsByteVec = &i64::MAX;
        let i64_max_bytes = i64_max.as_byte_vec();
        assert_eq!(i64_max_bytes, &[&[255, 255, 255, 255, 255, 255, 255, 127]]);
        assert_eq!(i64_max_bytes, &[i64::MAX.to_le_bytes()]);

        let u64_min: &dyn AsByteVec = &u64::MIN;
        let u64_min_bytes = u64_min.as_byte_vec();
        assert_eq!(u64_min_bytes, &[[0, 0, 0, 0, 0, 0, 0, 0]]);
        assert_eq!(i64_min_bytes, &[i64::MIN.to_le_bytes()]);
        let u64_max: &dyn AsByteVec = &u64::MAX;
        let u64_max_bytes = u64_max.as_byte_vec();
        assert_eq!(u64_max_bytes, &[&[255, 255, 255, 255, 255, 255, 255, 255]]);
        assert_eq!(u64_max_bytes, &[u64::MAX.to_le_bytes()]);

        let i128_min: &dyn AsByteVec = &i128::MIN;
        let i128_min_bytes = i128_min.as_byte_vec();
        assert_eq!(
            i128_min_bytes,
            &[&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128]]
        );
        assert_eq!(i128_min_bytes, &[i128::MIN.to_le_bytes()]);
        let i128_max: &dyn AsByteVec = &i128::MAX;
        let i128_max_bytes = i128_max.as_byte_vec();
        assert_eq!(
            i128_max_bytes,
            &[&[255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 127]]
        );
        assert_eq!(i128_max_bytes, &[i128::MAX.to_le_bytes()]);

        let u128_min: &dyn AsByteVec = &u128::MIN;
        let u128_min_bytes = u128_min.as_byte_vec();
        assert_eq!(
            u128_min_bytes,
            &[&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]]
        );
        assert_eq!(u128_min_bytes, &[u128::MIN.to_le_bytes()]);
        let u128_max: &dyn AsByteVec = &u128::MAX;
        let u128_max_bytes = u128_max.as_byte_vec();
        assert_eq!(
            u128_max_bytes,
            &[&[255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255]]
        );
        assert_eq!(u128_max_bytes, &[u128::MAX.to_le_bytes()]);
    }

    #[test]
    fn test_as_byte_vec_primitives() {
        let bool_false: &dyn AsByteVec = &false;
        assert_eq!(bool_false.as_byte_vec(), &[&[0]]);

        let bool_true: &dyn AsByteVec = &true;
        assert_eq!(bool_true.as_byte_vec(), &[&[1]]);
    }

    #[test]
    fn test_as_byte_vec_option() {
        // Very important property - `None` and `Some(0)` always have to be
        // different and should produce different hashes!
        let u8_none: Option<u8> = None;
        let u8_none: &dyn AsByteVec = &u8_none;
        assert_eq!(u8_none.as_byte_vec(), &[&[0]]);

        let u8_some_zero: Option<u8> = Some(0);
        let u8_some_zero: &dyn AsByteVec = &u8_some_zero;
        assert_eq!(u8_some_zero.as_byte_vec(), &[&[1], &[0]]);

        let u16_none: Option<u16> = None;
        let u16_none: &dyn AsByteVec = &u16_none;
        assert_eq!(u16_none.as_byte_vec(), &[&[0]]);

        let u16_some_zero: Option<u16> = Some(0);
        let u16_some_zero: &dyn AsByteVec = &u16_some_zero;
        assert_eq!(u16_some_zero.as_byte_vec(), &[&[1][..], &[0, 0][..]]);

        let u32_none: Option<u32> = None;
        let u32_none: &dyn AsByteVec = &u32_none;
        assert_eq!(u32_none.as_byte_vec(), &[&[0]]);

        let u32_some_zero: Option<u32> = Some(0);
        let u32_some_zero: &dyn AsByteVec = &u32_some_zero;
        assert_eq!(u32_some_zero.as_byte_vec(), &[&[1][..], &[0, 0, 0, 0][..]]);

        let u64_none: Option<u64> = None;
        let u64_none: &dyn AsByteVec = &u64_none;
        assert_eq!(u64_none.as_byte_vec(), &[&[0]]);

        let u64_some_zero: Option<u64> = Some(0);
        let u64_some_zero: &dyn AsByteVec = &u64_some_zero;
        assert_eq!(
            u64_some_zero.as_byte_vec(),
            &[&[1][..], &[0, 0, 0, 0, 0, 0, 0, 0][..]]
        );

        let u128_none: Option<u128> = None;
        let u128_none: &dyn AsByteVec = &u128_none;
        assert_eq!(u128_none.as_byte_vec(), &[&[0]]);

        let u128_some_zero: Option<u128> = Some(0);
        let u128_some_zero: &dyn AsByteVec = &u128_some_zero;
        assert_eq!(
            u128_some_zero.as_byte_vec(),
            &[
                &[1][..],
                &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0][..]
            ]
        );
    }

    #[test]
    fn test_as_byte_vec_array() {
        let arr: [u8; 0] = [];
        let arr: &dyn AsByteVec = &arr;
        assert_eq!(arr.as_byte_vec(), &[&[]]);

        let arr: [u8; 1] = [255];
        let arr: &dyn AsByteVec = &arr;
        assert_eq!(arr.as_byte_vec(), &[&[255]]);

        let arr: [u8; 4] = [255, 255, 255, 255];
        let arr: &dyn AsByteVec = &arr;
        assert_eq!(arr.as_byte_vec(), &[&[255, 255, 255, 255]]);
    }

    #[test]
    fn test_as_byte_vec_string() {
        let s: &dyn AsByteVec = &"".to_string();
        assert_eq!(s.as_byte_vec(), &[b""]);

        let s: &dyn AsByteVec = &"foobar".to_string();
        assert_eq!(s.as_byte_vec(), &[b"foobar"]);
    }
}
