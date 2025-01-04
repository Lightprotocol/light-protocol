use core::slice;
use std::{
    fmt,
    mem::size_of,
    ops::{Index, IndexMut},
    ptr::{self},
};

use crate::{
    add_padding, errors::ZeroCopyError, slice_mut::ZeroCopySliceMut,
    wrapped_pointer_mut::WrappedPointerMut, Length,
};

pub type ZeroCopyVecUsize<T> = ZeroCopyVec<usize, T>;
pub type ZeroCopyVecU64<T> = ZeroCopyVec<u64, T>;
pub type ZeroCopyVecU32<T> = ZeroCopyVec<u32, T>;
pub type ZeroCopyVecU16<T> = ZeroCopyVec<u16, T>;
pub type ZeroCopyVecU8<T> = ZeroCopyVec<u8, T>;

/// `ZeroCopyVec` is a custom vector implementation which forbids
/// post-initialization reallocations. The size is not known during compile
/// time (that makes it different from arrays), but can be defined only once
/// (that makes it different from [`Vec`](std::vec::Vec)).
pub struct ZeroCopyVec<L, T>
where
    L: Length,
    T: Copy,
{
    length: WrappedPointerMut<L>,
    data: ZeroCopySliceMut<L, T>,
}

impl<L: Length, T: Copy> ZeroCopyVec<L, T> {
    pub fn new(capacity: L, data: &mut [u8]) -> Result<Self, ZeroCopyError> {
        Self::new_at(capacity, data, &mut 0)
    }

    pub fn new_at(capacity: L, data: &mut [u8], offset: &mut usize) -> Result<Self, ZeroCopyError> {
        let length = WrappedPointerMut::<L>::new_at(L::try_from(0).unwrap(), data, offset).unwrap();
        add_padding::<L, T>(offset);
        let data = ZeroCopySliceMut::<L, T>::new_at(capacity, data, offset)?;
        Ok(Self { length, data })
    }

    pub fn new_at_multiple(
        num: usize,
        capacity: L,
        bytes: &mut [u8],
        offset: &mut usize,
    ) -> Result<Vec<ZeroCopyVec<L, T>>, ZeroCopyError> {
        let mut value_vecs = Vec::with_capacity(num);
        for _ in 0..num {
            let vec = Self::new_at(capacity, bytes, offset)?;
            value_vecs.push(vec);
        }
        Ok(value_vecs)
    }

    pub fn from_bytes(bytes: &mut [u8]) -> Result<ZeroCopyVec<L, T>, ZeroCopyError> {
        Self::from_bytes_at(bytes, &mut 0)
    }

    pub fn from_bytes_at(
        bytes: &mut [u8],
        offset: &mut usize,
    ) -> Result<ZeroCopyVec<L, T>, ZeroCopyError> {
        let length = WrappedPointerMut::<L>::from_bytes_at(bytes, offset)?;
        add_padding::<L, T>(offset);
        let data = ZeroCopySliceMut::from_bytes_at(bytes, offset)?;
        Ok(ZeroCopyVec { length, data })
    }

    pub fn from_bytes_at_multiple(
        num: usize,
        bytes: &mut [u8],
        offset: &mut usize,
    ) -> Result<Vec<Self>, ZeroCopyError> {
        let mut value_vecs = Vec::with_capacity(num);
        for _ in 0..num {
            let vec = Self::from_bytes_at(bytes, offset)?;
            value_vecs.push(vec);
        }
        Ok(value_vecs)
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.data.len()
    }

    #[inline]
    pub fn push(&mut self, value: T) -> Result<(), ZeroCopyError> {
        if self.len() == self.capacity() {
            return Err(ZeroCopyError::Full);
        }

        unsafe { ptr::write(self.data.data_as_mut_ptr().add(self.len()), value) };
        *self.length = *self.length + L::try_from(1).unwrap();

        Ok(())
    }

    #[inline]
    pub fn clear(&mut self) {
        *self.length.get_mut() = 0.try_into().unwrap();
    }

    #[inline]
    pub fn metadata_size() -> usize {
        let mut size = size_of::<L>();
        add_padding::<L, T>(&mut size);
        size
    }

    #[inline]
    pub fn data_size(length: L) -> usize {
        ZeroCopySliceMut::<L, T>::required_size_for_capacity(length.try_into().unwrap())
    }

    #[inline]
    pub fn required_size_for_capacity(capacity: usize) -> usize {
        Self::metadata_size() + Self::data_size(capacity.try_into().unwrap())
    }

    #[inline]
    pub fn len(&self) -> usize {
        (*self.length).try_into().unwrap()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len() {
            return None;
        }
        Some(&self.data[index])
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len() {
            return None;
        }
        Some(&mut self.data[index])
    }

    #[inline]
    pub fn first(&self) -> Option<&T> {
        self.get(0)
    }

    #[inline]
    pub fn first_mut(&mut self) -> Option<&mut T> {
        self.get_mut(0)
    }

    #[inline]
    pub fn last(&self) -> Option<&T> {
        self.get(self.len().saturating_sub(1))
    }

    #[inline]
    pub fn last_mut(&mut self) -> Option<&mut T> {
        self.get_mut(self.len().saturating_sub(1))
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        &self.data.as_slice()[..self.len()]
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        let len = self.len();
        &mut self.data.as_mut_slice()[..len]
    }

    pub fn extend_from_slice(&mut self, slice: &[T]) {
        let len = self.len();
        let new_len = len + slice.len();
        if new_len > self.capacity() {
            panic!("Capacity overflow. Cannot copy slice into ZeroCopyVec");
        }
        self.data.as_mut_slice()[len..].copy_from_slice(slice);
        *self.length = L::try_from(new_len).unwrap();
    }

    #[inline]
    pub fn to_vec(&self) -> Vec<T> {
        self.as_slice().to_vec()
    }

    pub fn try_into_array<const N: usize>(&self) -> Result<[T; N], ZeroCopyError> {
        self.data.try_into_array()
    }
}

impl<L: Length, T: Copy> IndexMut<usize> for ZeroCopyVec<L, T> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        // Access the underlying mutable slice using as_mut_slice() and index it
        &mut self.as_mut_slice()[index]
    }
}

impl<L: Length, T: Copy> Index<usize> for ZeroCopyVec<L, T> {
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        // Access the underlying slice using as_slice() and index it
        &self.as_slice()[index]
    }
}

impl<'a, L: Length, T: Copy> IntoIterator for &'a ZeroCopyVec<L, T> {
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, L: Length, T: Copy> IntoIterator for &'a mut ZeroCopyVec<L, T> {
    type Item = &'a mut T;
    type IntoIter = slice::IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<'b, L: Length, T: Copy> ZeroCopyVec<L, T> {
    #[inline]
    pub fn iter(&'b self) -> slice::Iter<'b, T> {
        self.as_slice().iter()
    }

    #[inline]
    pub fn iter_mut(&'b mut self) -> slice::IterMut<'b, T> {
        self.as_mut_slice().iter_mut()
    }
}

impl<L: Length, T: Copy + PartialEq> PartialEq for ZeroCopyVec<L, T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data && self.len() == other.len()
    }
}

impl<L: Length, T: Copy + fmt::Debug> fmt::Debug for ZeroCopyVec<L, T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_vec())
    }
}
