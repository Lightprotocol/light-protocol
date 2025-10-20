#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};

use zerocopy::Ref;

use crate::{
    errors::ZeroCopyError,
    traits::{ZeroCopyAt, ZeroCopyAtMut},
};

#[cfg(feature = "alloc")]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VecU8<T>(Vec<T>);

#[cfg(feature = "alloc")]
impl<T> VecU8<T> {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

#[cfg(feature = "alloc")]
impl<T> Deref for VecU8<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "alloc")]
impl DerefMut for VecU8<u8> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: ZeroCopyAt<'a>> ZeroCopyAt<'a> for VecU8<T> {
    type ZeroCopyAt = Vec<T::ZeroCopyAt>;

    #[inline]
    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
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

#[cfg(feature = "alloc")]
impl<'a, T: ZeroCopyAtMut<'a>> ZeroCopyAtMut<'a> for VecU8<T> {
    type ZeroCopyAtMut = Vec<T::ZeroCopyAtMut>;

    #[inline]
    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
        let (num_slices, mut bytes) = Ref::<&mut [u8], u8>::from_prefix(bytes)?;
        let num_slices = u32::from(*num_slices) as usize;
        let mut slices = Vec::with_capacity(num_slices);
        for _ in 0..num_slices {
            let (slice, _bytes) = T::zero_copy_at_mut(bytes)?;
            bytes = _bytes;
            slices.push(slice);
        }
        Ok((slices, bytes))
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;

    #[test]
    fn test_vecu8() {
        let bytes = vec![8, 1u8, 2, 3, 4, 5, 6, 7, 8];
        let (vec, remaining_bytes) = VecU8::<u8>::zero_copy_at(&bytes).unwrap();
        assert_eq!(vec, vec![1u8, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(remaining_bytes, &[]);
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

    #[test]
    fn test_vecu8_mut() {
        let mut bytes = vec![8, 1u8, 2, 3, 4, 5, 6, 7, 8];
        let (vec, remaining_bytes) = VecU8::<u8>::zero_copy_at_mut(&mut bytes).unwrap();
        assert_eq!(
            vec.iter().map(|x| **x).collect::<Vec<u8>>(),
            vec![1u8, 2, 3, 4, 5, 6, 7, 8]
        );
        assert_eq!(remaining_bytes, &mut []);
    }

    #[test]
    fn test_deserialize_mut_vecu8() {
        let mut bytes = [3, 4, 5, 6]; // Length 3, followed by values 4, 5, 6
        let (vec, remaining) = VecU8::<u8>::zero_copy_at_mut(&mut bytes).unwrap();
        assert_eq!(
            vec.iter().map(|x| **x).collect::<Vec<u8>>(),
            std::vec![4, 5, 6]
        );
        assert_eq!(remaining, &mut []);
    }
}
