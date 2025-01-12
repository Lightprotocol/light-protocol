use core::fmt;
use std::{
    fmt::Debug,
    marker::PhantomData,
    mem::size_of,
    ops::{Index, IndexMut},
};

use zerocopy::Ref;

use crate::{add_padding, errors::ZeroCopyError, slice_mut::ZeroCopyTraits, vec::ZeroCopyVec};

pub type ZeroCopyCyclicVecU32<'a, T> = ZeroCopyCyclicVec<'a, u32, T>;
pub type ZeroCopyCyclicVecU64<'a, T> = ZeroCopyCyclicVec<'a, u64, T>;
pub type ZeroCopyCyclicVecU16<'a, T> = ZeroCopyCyclicVec<'a, u16, T>;
pub type ZeroCopyCyclicVecU8<'a, T> = ZeroCopyCyclicVec<'a, u8, T>;

pub struct ZeroCopyCyclicVec<'a, L, T, const PAD: bool = true>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L> + TryInto<L>,
{
    current_index: Ref<&'a mut [u8], L>,
    vec: ZeroCopyVec<'a, L, T, PAD>,
}

impl<'a, L, T, const PAD: bool> ZeroCopyCyclicVec<'a, L, T, PAD>
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
        let (current_index, _padding) = Ref::<&mut [u8], L>::from_prefix(meta_data)
            .map_err(|e| ZeroCopyError::CastError(e.to_string()))?;
        if u64::from(*current_index) != 0 {
            return Err(ZeroCopyError::MemoryNotZeroed);
        }

        let (vec, bytes) = ZeroCopyVec::<'a, L, T, PAD>::new_at(capacity, bytes)?;
        Ok((Self { current_index, vec }, bytes))
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

    pub fn from_bytes(bytes: &'a mut [u8]) -> Result<Self, ZeroCopyError> {
        Ok(Self::from_bytes_at(bytes)?.0)
    }

    pub fn from_bytes_at(bytes: &'a mut [u8]) -> Result<(Self, &'a mut [u8]), ZeroCopyError> {
        let (meta_data, bytes) = bytes.split_at_mut(Self::metadata_size());
        let (current_index, _padding) = Ref::<&mut [u8], L>::from_prefix(meta_data)
            .map_err(|e| ZeroCopyError::CastError(e.to_string()))?;
        let (vec, bytes) = ZeroCopyVec::<'a, L, T, PAD>::from_bytes_at(bytes)?;
        Ok((Self { current_index, vec }, bytes))
    }

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
    pub fn push(&mut self, value: T) {
        if self.vec.len() < self.vec.capacity() {
            self.vec.push(value).unwrap();
        } else {
            let current_index = self.current_index();
            self.vec[current_index] = value;
        }
        let new_index = (self.current_index() + 1) % self.vec.capacity();
        *self.current_index = (new_index as u64)
            .try_into()
            .map_err(|_| ZeroCopyError::InvalidConversion)
            .unwrap();
    }

    #[inline]
    pub fn clear(&mut self) {
        *self.current_index = 0
            .try_into()
            .map_err(|_| ZeroCopyError::InvalidConversion)
            .unwrap();
        self.vec.clear();
    }

    #[inline]
    pub fn first(&self) -> Option<&T> {
        self.vec.get(self.first_index())
    }

    #[inline]
    pub fn first_mut(&mut self) -> Option<&mut T> {
        self.vec.get_mut(self.first_index())
    }

    #[inline]
    pub fn last(&self) -> Option<&T> {
        self.vec.get(self.last_index())
    }

    #[inline]
    pub fn last_mut(&mut self) -> Option<&mut T> {
        self.vec.get_mut(self.last_index())
    }

    #[inline]
    fn current_index(&self) -> usize {
        u64::from(*self.current_index) as usize
    }

    /// First index is the next index after the last index mod capacity.
    #[inline]
    pub fn first_index(&self) -> usize {
        if self.len() < self.vec.capacity() || self.last_index() == self.vec.capacity() {
            0
        } else {
            self.last_index().saturating_add(1) % (self.vec.capacity())
        }
    }

    #[inline]
    pub fn last_index(&self) -> usize {
        if self.current_index() == 0 && self.vec.len() == self.vec.capacity() {
            self.vec.capacity().saturating_sub(1)
        } else {
            self.current_index().saturating_sub(1) % self.vec.capacity()
        }
    }

    #[inline]
    pub fn iter(&self) -> ZeroCopyCyclicVecIterator<'_, L, T, PAD> {
        ZeroCopyCyclicVecIterator {
            vec: self,
            current: self.first_index(),
            is_finished: false,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn iter_from(
        &self,
        start: usize,
    ) -> Result<ZeroCopyCyclicVecIterator<'_, L, T, PAD>, ZeroCopyError> {
        if start >= self.len() {
            return Err(ZeroCopyError::IterFromOutOfBounds);
        }
        Ok(ZeroCopyCyclicVecIterator {
            vec: self,
            current: start,
            is_finished: false,
            _marker: PhantomData,
        })
    }

    pub fn metadata_size() -> usize {
        let mut size = size_of::<L>();
        if PAD {
            add_padding::<L, T>(&mut size);
        }
        size
    }

    pub fn data_size(length: L) -> usize {
        ZeroCopyVec::<L, T>::required_size_for_capacity(length)
    }

    pub fn required_size_for_capacity(capacity: L) -> usize {
        Self::metadata_size() + Self::data_size(capacity)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.vec.capacity()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.vec.get(index)
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.vec.get_mut(index)
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        self.vec.as_slice()
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.vec.as_mut_slice()
    }

    pub fn try_into_array<const N: usize>(&self) -> Result<[T; N], ZeroCopyError> {
        self.vec.try_into_array()
    }

    #[inline]
    pub fn to_vec(&self) -> Vec<T> {
        self.vec.to_vec()
    }
}

pub struct ZeroCopyCyclicVecIterator<'a, L, T, const PAD: bool>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L> + TryInto<L>,
{
    vec: &'a ZeroCopyCyclicVec<'a, L, T, PAD>,
    current: usize,
    is_finished: bool,
    _marker: PhantomData<T>,
}

impl<'a, L, T, const PAD: bool> Iterator for ZeroCopyCyclicVecIterator<'a, L, T, PAD>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L> + TryInto<L>,
{
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.vec.capacity() == 0 || self.is_finished {
            None
        } else {
            if self.current == self.vec.last_index() {
                self.is_finished = true;
            }
            let new_current = (self.current + 1) % self.vec.capacity();
            let element = self.vec.get(self.current);
            self.current = new_current;
            element
        }
    }
}

impl<L, T, const PAD: bool> IndexMut<usize> for ZeroCopyCyclicVec<'_, L, T, PAD>
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

impl<L, T, const PAD: bool> Index<usize> for ZeroCopyCyclicVec<'_, L, T, PAD>
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

impl<L, T, const PAD: bool> PartialEq for ZeroCopyCyclicVec<'_, L, T, PAD>
where
    L: ZeroCopyTraits + PartialEq,
    T: ZeroCopyTraits + PartialEq,
    u64: From<L> + TryInto<L>,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.vec == other.vec && *self.current_index == *other.current_index
    }
}

impl<L, T, const PAD: bool> fmt::Debug for ZeroCopyCyclicVec<'_, L, T, PAD>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits + Debug,
    u64: From<L> + TryInto<L>,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_vec())
    }
}
