use core::{
    mem::size_of,
    ops::{Deref, DerefMut},
};
use std::vec::Vec;

use zerocopy::{
    little_endian::{U16, U32, U64},
    FromBytes, Immutable, KnownLayout, Ref,
};

use crate::errors::ZeroCopyError;

pub trait Deserialize<'a>
where
    Self: Sized,
{
    type Output;

    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::Output, &'a [u8]), ZeroCopyError>;
}

impl<'a, T: KnownLayout + Immutable + FromBytes> Deserialize<'a> for Ref<&'a [u8], T> {
    type Output = Ref<&'a [u8], T>;

    #[inline]
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self, &'a [u8]), ZeroCopyError> {
        let (bytes, remaining_bytes) = Ref::<&[u8], T>::from_prefix(bytes)?;
        Ok((bytes, remaining_bytes))
    }
}

impl<'a, T: Deserialize<'a>> Deserialize<'a> for Option<T> {
    type Output = Option<T::Output>;
    #[inline]
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::Output, &'a [u8]), ZeroCopyError> {
        if bytes.len() < size_of::<u8>() {
            return Err(ZeroCopyError::ArraySize(1, bytes.len()));
        }
        let (option_byte, bytes) = bytes.split_at(1);
        Ok(match option_byte[0] {
            0u8 => (None, bytes),
            1u8 => {
                let (value, bytes) = T::zero_copy_at(bytes)?;
                (Some(value), bytes)
            }
            _ => return Err(ZeroCopyError::InvalidOptionByte(option_byte[0])),
        })
    }
}

impl Deserialize<'_> for u8 {
    type Output = Self;

    /// Not a zero copy but cheaper.
    /// A u8 should not be deserialized on it's own but as part of a struct.
    #[inline]
    fn zero_copy_at(bytes: &[u8]) -> Result<(u8, &[u8]), ZeroCopyError> {
        if bytes.len() < size_of::<u8>() {
            return Err(ZeroCopyError::ArraySize(1, bytes.len()));
        }
        let (bytes, remaining_bytes) = bytes.split_at(size_of::<u8>());
        Ok((bytes[0], remaining_bytes))
    }
}

macro_rules! impl_deserialize_for_primitive {
    ($($t:ty),*) => {
        $(
            impl<'a> Deserialize<'a> for $t {
                type Output = Ref<&'a [u8], $t>;

                #[inline]
                fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::Output, &'a [u8]), ZeroCopyError> {
                    Self::Output::zero_copy_at(bytes)
                }
            }
        )*
    };
}

impl_deserialize_for_primitive!(u16, i16, u32, i32, u64, i64);
impl_deserialize_for_primitive!(U16, U32, U64);

impl<'a, T: Deserialize<'a>> Deserialize<'a> for Vec<T> {
    type Output = Vec<T::Output>;
    #[inline]
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::Output, &'a [u8]), ZeroCopyError> {
        let (num_slices, mut bytes) = Ref::<&[u8], U32>::from_prefix(bytes)?;
        let num_slices = u32::from(*num_slices) as usize;
        // TODO: add check that remaining data is enough to read num_slices
        // This prevents agains invalid data allocating a lot of heap memory
        let mut slices = Vec::with_capacity(num_slices);
        for _ in 0..num_slices {
            let (slice, _bytes) = T::zero_copy_at(bytes)?;
            bytes = _bytes;
            slices.push(slice);
        }
        Ok((slices, bytes))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct VecU8<T>(Vec<T>);
impl<T> VecU8<T> {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

impl<T> Deref for VecU8<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for VecU8<u8> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, T: Deserialize<'a>> Deserialize<'a> for VecU8<T> {
    type Output = Vec<T::Output>;

    #[inline]
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::Output, &'a [u8]), ZeroCopyError> {
        let (num_slices, mut bytes) = Ref::<&[u8], u8>::from_prefix(bytes)?;
        let num_slices = u32::from(*num_slices) as usize;
        let mut slices = Vec::with_capacity(num_slices);
        for _ in 0..num_slices {
            let (slice, _bytes) = T::zero_copy_at(bytes)?;
            bytes = _bytes;
            slices.push(slice);
        }
        Ok((slices, bytes))
    }
}

#[test]
fn test_vecu8() {
    use std::vec;
    let bytes = vec![8, 1u8, 2, 3, 4, 5, 6, 7, 8];
    let (vec, remaining_bytes) = VecU8::<u8>::zero_copy_at(&bytes).unwrap();
    assert_eq!(vec, vec![1u8, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(remaining_bytes, &[]);
}

#[test]
fn test_deserialize_ref() {
    let bytes = [1, 0, 0, 0]; // Little-endian representation of 1
    let (ref_data, remaining) = Ref::<&[u8], U32>::zero_copy_at(&bytes).unwrap();
    assert_eq!(u32::from(*ref_data), 1);
    assert_eq!(remaining, &[]);
    let res = Ref::<&[u8], U32>::zero_copy_at(&[]);
    assert_eq!(res, Err(ZeroCopyError::Size));
}

#[test]
fn test_deserialize_option_some() {
    let bytes = [1, 2]; // 1 indicates Some, followed by the value 2
    let (option_value, remaining) = Option::<u8>::zero_copy_at(&bytes).unwrap();
    assert_eq!(option_value, Some(2));
    assert_eq!(remaining, &[]);
    let res = Option::<u8>::zero_copy_at(&[]);
    assert_eq!(res, Err(ZeroCopyError::ArraySize(1, 0)));
    let bytes = [2, 0]; // 2 indicates invalid option byte
    let res = Option::<u8>::zero_copy_at(&bytes);
    assert_eq!(res, Err(ZeroCopyError::InvalidOptionByte(2)));
}

#[test]
fn test_deserialize_option_none() {
    let bytes = [0]; // 0 indicates None
    let (option_value, remaining) = Option::<u8>::zero_copy_at(&bytes).unwrap();
    assert_eq!(option_value, None);
    assert_eq!(remaining, &[]);
}

#[test]
fn test_deserialize_u8() {
    let bytes = [0xFF]; // Value 255
    let (value, remaining) = u8::zero_copy_at(&bytes).unwrap();
    assert_eq!(value, 255);
    assert_eq!(remaining, &[]);
    let res = u8::zero_copy_at(&[]);
    assert_eq!(res, Err(ZeroCopyError::ArraySize(1, 0)));
}

#[test]
fn test_deserialize_u16() {
    let bytes = 2323u16.to_le_bytes();
    let (value, remaining) = u16::zero_copy_at(bytes.as_slice()).unwrap();
    assert_eq!(*value, 2323u16);
    assert_eq!(remaining, &[]);
    let res = u16::zero_copy_at(&[0u8]);
    assert_eq!(res, Err(ZeroCopyError::Size));
}

#[test]
fn test_deserialize_vec() {
    let bytes = [2, 0, 0, 0, 1, 2]; // Length 2, followed by values 1 and 2
    let (vec, remaining) = Vec::<u8>::zero_copy_at(&bytes).unwrap();
    assert_eq!(vec, std::vec![1, 2]);
    assert_eq!(remaining, &[]);
}

#[test]
fn test_vecu8_deref() {
    let data = std::vec![1, 2, 3];
    let vec_u8 = VecU8(data.clone());
    assert_eq!(&*vec_u8, &data);

    let mut vec = VecU8::new();
    vec.push(1u8);
    assert_eq!(*vec, std::vec![1u8]);
}

#[test]
fn test_deserialize_vecu8() {
    let bytes = [3, 4, 5, 6]; // Length 3, followed by values 4, 5, 6
    let (vec, remaining) = VecU8::<u8>::zero_copy_at(&bytes).unwrap();
    assert_eq!(vec, std::vec![4, 5, 6]);
    assert_eq!(remaining, &[]);
}
