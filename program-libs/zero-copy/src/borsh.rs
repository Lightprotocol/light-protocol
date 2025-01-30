use std::vec::Vec;

use crate::errors::ZeroCopyError;
use core::mem::size_of;
use zerocopy::{little_endian::U32, FromBytes, Immutable, KnownLayout, Ref};

pub trait Deserialize<'a>
where
    Self: Sized,
{
    type Output;
    fn deserialize_at(bytes: &'a [u8]) -> Result<(Self::Output, &'a [u8]), ZeroCopyError>;
}

impl<'a, T: KnownLayout + Immutable + FromBytes> Deserialize<'a> for Ref<&'a [u8], T> {
    type Output = Ref<&'a [u8], T>;

    #[inline]
    fn deserialize_at(bytes: &'a [u8]) -> Result<(Self, &'a [u8]), ZeroCopyError> {
        let (bytes, remaining_bytes) = Ref::<&[u8], T>::from_prefix(bytes)?;
        Ok((bytes, remaining_bytes))
    }
}

impl<'a, T: Deserialize<'a>> Deserialize<'a> for Option<T> {
    type Output = Option<T::Output>;
    #[inline]
    fn deserialize_at(bytes: &'a [u8]) -> Result<(Self::Output, &'a [u8]), ZeroCopyError> {
        if bytes.len() < size_of::<u8>() {
            return Err(ZeroCopyError::ArraySize(1, bytes.len()));
        }
        let (option_byte, bytes) = bytes.split_at(1);
        Ok(match option_byte[0] {
            0u8 => (None, bytes),
            1u8 => {
                let (address, bytes) = T::deserialize_at(bytes)?;
                (Some(address), bytes)
            }
            _ => return Err(ZeroCopyError::InvalidOptionByte(option_byte[0])),
        })
    }
}

impl Deserialize<'_> for u8 {
    type Output = Self;

    #[inline]
    fn deserialize_at(bytes: &[u8]) -> Result<(u8, &[u8]), ZeroCopyError> {
        if bytes.len() < size_of::<u8>() {
            return Err(ZeroCopyError::ArraySize(1, bytes.len()));
        }
        let (bytes, remaining_bytes) = bytes.split_at(size_of::<u8>());
        Ok((bytes[0], remaining_bytes))
    }
}

impl<'a, T: Deserialize<'a>> Deserialize<'a> for Vec<T> {
    type Output = Vec<T::Output>;
    #[inline]
    fn deserialize_at(bytes: &'a [u8]) -> Result<(Self::Output, &'a [u8]), ZeroCopyError> {
        let (num_slices, mut bytes) = Ref::<&[u8], U32>::from_prefix(bytes)?;
        let num_slices = u32::from(*num_slices) as usize;
        let mut slices = Vec::with_capacity(num_slices);
        for _ in 0..num_slices {
            let (slice, _bytes) = T::deserialize_at(bytes)?;
            bytes = _bytes;
            slices.push(slice);
        }
        Ok((slices, bytes))
    }
}
