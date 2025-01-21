use core::slice;
use std::{
    fmt,
    mem::size_of,
    ops::{Index, IndexMut},
};

use zerocopy::Ref;

use crate::{
    add_padding,
    errors::ZeroCopyError,
    slice_mut::{ZeroCopySliceMut, ZeroCopyTraits},
};

pub type ZeroCopyVecU64<'a, T> = ZeroCopyVec<'a, u64, T>;
pub type ZeroCopyVecU32<'a, T> = ZeroCopyVec<'a, u32, T>;
pub type ZeroCopyVecU16<'a, T> = ZeroCopyVec<'a, u16, T>;
pub type ZeroCopyVecU8<'a, T> = ZeroCopyVec<'a, u8, T>;

/// `ZeroCopyVec` is a custom vector implementation which forbids
/// post-initialization reallocations. The size is not known during compile
/// time (that makes it different from arrays), but can be defined only once
/// (that makes it different from [`Vec`](std::vec::Vec)).
pub struct ZeroCopyVec<'a, L, T, const PAD: bool = true>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
{
    length: Ref<&'a mut [u8], L>,
    slice: ZeroCopySliceMut<'a, L, T, PAD>,
}

impl<'a, L, T, const PAD: bool> ZeroCopyVec<'a, L, T, PAD>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L> + TryInto<L>,
{
    pub fn new(capacity: L, bytes: &'a mut [u8]) -> Result<Self, ZeroCopyError> {
        Ok(Self::new_at(capacity, bytes)?.0)
    }

    pub fn new_at(capacity: L, bytes: &'a mut [u8]) -> Result<(Self, &'a mut [u8]), ZeroCopyError> {
        let (meta_data, bytes) = bytes.split_at_mut(Self::metadata_size());

        let (length, _padding) = Ref::<&mut [u8], L>::from_prefix(meta_data)
            .map_err(|e| ZeroCopyError::CastError(e.to_string()))?;
        if u64::from(*length) != 0 {
            return Err(ZeroCopyError::MemoryNotZeroed);
        }
        let (slice, bytes) = ZeroCopySliceMut::<'a, L, T, PAD>::new_at(capacity, bytes)?;
        Ok((Self { length, slice }, bytes))
    }

    pub fn new_at_multiple(
        num: usize,
        capacity: L,
        mut bytes: &'a mut [u8],
    ) -> Result<(Vec<Self>, &'a mut [u8]), ZeroCopyError> {
        let mut value_vecs = Vec::with_capacity(num);
        for _ in 0..num {
            let (vec, _bytes) = Self::new_at(capacity, bytes)?;
            bytes = _bytes;
            value_vecs.push(vec);
        }
        Ok((value_vecs, bytes))
    }

    #[inline]
    pub fn from_bytes(bytes: &'a mut [u8]) -> Result<Self, ZeroCopyError> {
        Ok(Self::from_bytes_at(bytes)?.0)
    }

    #[inline]
    pub fn from_bytes_at(bytes: &'a mut [u8]) -> Result<(Self, &'a mut [u8]), ZeroCopyError> {
        let (meta_data, bytes) = bytes.split_at_mut(Self::metadata_size());
        let (length, _padding) = Ref::<&mut [u8], L>::from_prefix(meta_data)
            .map_err(|e| ZeroCopyError::CastError(e.to_string()))?;
        let (slice, bytes) = ZeroCopySliceMut::<'a, L, T, PAD>::from_bytes_at(bytes)?;
        Ok((Self { length, slice }, bytes))
    }

    #[inline]
    pub fn from_bytes_at_multiple(
        num: usize,
        mut bytes: &'a mut [u8],
    ) -> Result<(Vec<Self>, &'a mut [u8]), ZeroCopyError> {
        let mut value_vecs = Vec::with_capacity(num);
        for _ in 0..num {
            let (vec, _bytes) = Self::from_bytes_at(bytes)?;
            bytes = _bytes;
            value_vecs.push(vec);
        }
        Ok((value_vecs, bytes))
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.slice.len()
    }

    #[inline]
    pub fn push(&mut self, value: T) -> Result<(), ZeroCopyError> {
        if self.len() == self.capacity() {
            return Err(ZeroCopyError::Full);
        }

        let len = self.len();
        self.slice.as_mut_slice()[len] = value;
        *self.length = (len as u64 + 1u64)
            .try_into()
            .map_err(|_| ZeroCopyError::InvalidConversion)?;

        Ok(())
    }

    #[inline]
    pub fn clear(&mut self) {
        let len = &mut self.length;
        **len = (0u64)
            .try_into()
            .map_err(|_| ZeroCopyError::InvalidConversion)
            .unwrap();
    }

    #[inline]
    pub fn metadata_size() -> usize {
        let mut size = size_of::<L>();
        if PAD {
            add_padding::<L, T>(&mut size);
        }
        size
    }

    #[inline]
    pub fn data_size(length: L) -> usize {
        ZeroCopySliceMut::<L, T>::required_size_for_capacity(length)
    }

    #[inline]
    pub fn required_size_for_capacity(capacity: L) -> usize {
        Self::metadata_size() + Self::data_size(capacity)
    }

    #[inline]
    pub fn len(&self) -> usize {
        u64::from(*self.length) as usize
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
        Some(&self.slice[index])
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len() {
            return None;
        }
        Some(&mut self.slice[index])
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
        &self.slice.as_slice()[..self.len()]
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        let len = self.len();
        &mut self.slice.as_mut_slice()[..len]
    }

    pub fn extend_from_slice(&mut self, slice: &[T]) {
        let len = self.len();
        let new_len = len + slice.len();
        if new_len > self.capacity() {
            panic!("Capacity overflow. Cannot copy slice into ZeroCopyVec");
        }
        self.slice.as_mut_slice()[len..].copy_from_slice(slice);
        *self.length = (new_len as u64)
            .try_into()
            .map_err(|_| ZeroCopyError::InvalidConversion)
            .unwrap();
    }

    #[inline]
    pub fn to_vec(&self) -> Vec<T> {
        self.as_slice().to_vec()
    }

    pub fn try_into_array<const N: usize>(&self) -> Result<[T; N], ZeroCopyError> {
        self.slice.try_into_array()
    }
}

impl<L, T, const PAD: bool> IndexMut<usize> for ZeroCopyVec<'_, L, T, PAD>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L> + TryInto<L>,
{
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        // Access the underlying mutable slice using as_mut_slice() and index it
        &mut self.as_mut_slice()[index]
    }
}

impl<L, T, const PAD: bool> Index<usize> for ZeroCopyVec<'_, L, T, PAD>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L> + TryInto<L>,
{
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        // Access the underlying slice using as_slice() and index it
        &self.as_slice()[index]
    }
}

impl<'a, L, T, const PAD: bool> IntoIterator for &'a ZeroCopyVec<'a, L, T, PAD>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L> + TryInto<L>,
{
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, L, T, const PAD: bool> IntoIterator for &'a mut ZeroCopyVec<'a, L, T, PAD>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L> + TryInto<L>,
{
    type Item = &'a mut T;
    type IntoIter = slice::IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<'b, L, T, const PAD: bool> ZeroCopyVec<'_, L, T, PAD>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L> + TryInto<L>,
{
    #[inline]
    pub fn iter(&'b self) -> slice::Iter<'b, T> {
        self.as_slice().iter()
    }

    #[inline]
    pub fn iter_mut(&'b mut self) -> slice::IterMut<'b, T> {
        self.as_mut_slice().iter_mut()
    }
}

impl<L, T, const PAD: bool> PartialEq for ZeroCopyVec<'_, L, T, PAD>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits + PartialEq,
    u64: From<L> + TryInto<L>,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<L, T, const PAD: bool> fmt::Debug for ZeroCopyVec<'_, L, T, PAD>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits + fmt::Debug,
    u64: From<L> + TryInto<L>,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_vec())
    }
}
