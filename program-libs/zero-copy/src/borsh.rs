use std::vec::Vec;

use crate::errors::ZeroCopyError;
use core::mem::size_of;
use zerocopy::{
    big_endian::U64,
    little_endian::{U16, U32},
    FromBytes, Immutable, KnownLayout, Ref,
};

pub trait Deserialize<'a>
where
    Self: Sized,
{
    type Output;
    fn deserialize_at(bytes: &'a [u8]) -> Result<(Self::Output, &'a [u8]), ZeroCopyError>;
}

// Macro to implement Deserialize for [u8; N] with Ref for N >= 32
macro_rules! impl_deserialize_for_u8_array_ref {
    ( $( $N:expr ),+ ) => {
        $(
            impl<'a> Deserialize<'a> for [u8; $N] {
                type Output = Ref<&'a [u8],[u8; $N]>;

                    #[inline]
                fn deserialize_at(bytes: &'a [u8]) -> Result<(Self::Output, &'a [u8]), ZeroCopyError> {

                    if bytes.len() < $N {
                        return Err(ZeroCopyError::ArraySize($N, bytes.len()));
                    }

                    let (ref_arr, remaining) = Ref::<&[u8], [u8; $N]>::from_prefix(bytes)?;
                    Ok((ref_arr, remaining))
                }
            }
        )+
    };
}
// 1 -32
impl_deserialize_for_u8_array_ref!(1);
impl_deserialize_for_u8_array_ref!(2);
impl_deserialize_for_u8_array_ref!(3);
impl_deserialize_for_u8_array_ref!(4);
impl_deserialize_for_u8_array_ref!(5);
impl_deserialize_for_u8_array_ref!(6);
impl_deserialize_for_u8_array_ref!(7);
impl_deserialize_for_u8_array_ref!(8);
impl_deserialize_for_u8_array_ref!(9);
impl_deserialize_for_u8_array_ref!(10);
impl_deserialize_for_u8_array_ref!(11);
impl_deserialize_for_u8_array_ref!(12);
impl_deserialize_for_u8_array_ref!(13);
impl_deserialize_for_u8_array_ref!(14);
impl_deserialize_for_u8_array_ref!(15);
impl_deserialize_for_u8_array_ref!(16);
impl_deserialize_for_u8_array_ref!(17);
impl_deserialize_for_u8_array_ref!(18);
impl_deserialize_for_u8_array_ref!(19);
impl_deserialize_for_u8_array_ref!(20);
impl_deserialize_for_u8_array_ref!(21);
impl_deserialize_for_u8_array_ref!(22);
impl_deserialize_for_u8_array_ref!(23);
impl_deserialize_for_u8_array_ref!(24);
impl_deserialize_for_u8_array_ref!(25);
impl_deserialize_for_u8_array_ref!(26);
impl_deserialize_for_u8_array_ref!(27);
impl_deserialize_for_u8_array_ref!(28);
impl_deserialize_for_u8_array_ref!(29);
impl_deserialize_for_u8_array_ref!(30);
impl_deserialize_for_u8_array_ref!(31);
impl_deserialize_for_u8_array_ref!(32);
impl_deserialize_for_u8_array_ref!(64);
impl_deserialize_for_u8_array_ref!(128);
impl_deserialize_for_u8_array_ref!(256);
impl_deserialize_for_u8_array_ref!(512);
impl_deserialize_for_u8_array_ref!(1024);

// TODO: add deserialize at mut
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
        let num_slices = u64::from(*num_slices) as usize;
        let mut slices = Vec::with_capacity(num_slices);
        for _ in 0..num_slices {
            let (slice, _bytes) = T::deserialize_at(bytes)?;
            bytes = _bytes;
            slices.push(slice);
        }
        Ok((slices, bytes))
    }
}

impl<'a> Deserialize<'a> for U64 {
    type Output = Ref<&'a [u8], U64>;

    #[inline]
    fn deserialize_at(bytes: &'a [u8]) -> Result<(Self::Output, &'a [u8]), ZeroCopyError> {
        Ok(Ref::<&[u8], U64>::from_prefix(bytes)?)
    }
}

impl<'a> Deserialize<'a> for U16 {
    type Output = Ref<&'a [u8], U16>;

    #[inline]
    fn deserialize_at(bytes: &'a [u8]) -> Result<(Self::Output, &'a [u8]), ZeroCopyError> {
        Ok(Ref::<&[u8], U16>::from_prefix(bytes)?)
    }
}

impl<'a> Deserialize<'a> for U32 {
    type Output = Ref<&'a [u8], U32>;

    #[inline]
    fn deserialize_at(bytes: &'a [u8]) -> Result<(Self::Output, &'a [u8]), ZeroCopyError> {
        Ok(Ref::<&[u8], U32>::from_prefix(bytes)?)
    }
}
