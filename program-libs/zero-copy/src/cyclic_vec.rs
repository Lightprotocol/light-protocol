use core::fmt;
use std::{
    fmt::Debug,
    marker::PhantomData,
    mem::size_of,
    ops::{Index, IndexMut},
};

use num_traits::{FromPrimitive, PrimInt, ToPrimitive};

use crate::{
    add_padding, errors::ZeroCopyError, vec::ZeroCopyVec, wrapped_pointer_mut::WrappedPointerMut,
};

pub type ZeroCopyCyclicVecUsize<T> = ZeroCopyCyclicVec<usize, T>;
pub type ZeroCopyCyclicVecU32<T> = ZeroCopyCyclicVec<u32, T>;
pub type ZeroCopyCyclicVecU64<T> = ZeroCopyCyclicVec<u64, T>;
pub type ZeroCopyCyclicVecU16<T> = ZeroCopyCyclicVec<u16, T>;
pub type ZeroCopyCyclicVecU8<T> = ZeroCopyCyclicVec<u8, T>;

pub struct ZeroCopyCyclicVec<LEN, T>
where
    LEN: FromPrimitive + ToPrimitive + PrimInt,
    T: Copy,
{
    current_index: WrappedPointerMut<LEN>,
    vec: ZeroCopyVec<LEN, T>,
}

impl<LEN, T> ZeroCopyCyclicVec<LEN, T>
where
    LEN: FromPrimitive + ToPrimitive + PrimInt,
    T: Copy,
{
    pub fn new(capacity: LEN, vec: &mut [u8]) -> Result<Self, ZeroCopyError> {
        Self::new_at(capacity, vec, &mut 0)
    }

    pub fn new_at(
        capacity: LEN,
        vec: &mut [u8],
        offset: &mut usize,
    ) -> Result<Self, ZeroCopyError> {
        let current_index = WrappedPointerMut::<LEN>::new_at(LEN::zero(), vec, offset)?;
        add_padding::<LEN, T>(offset);
        let vec = ZeroCopyVec::<LEN, T>::new_at(capacity, vec, offset)?;
        Ok(Self { current_index, vec })
    }

    pub fn new_at_multiple(
        num: usize,
        capacity: LEN,
        account_data: &mut [u8],
        offset: &mut usize,
    ) -> Result<Vec<ZeroCopyCyclicVec<LEN, T>>, ZeroCopyError> {
        let mut value_vecs = Vec::with_capacity(num);
        for _ in 0..num {
            let vec = Self::new_at(capacity, account_data, offset)?;
            value_vecs.push(vec);
        }
        Ok(value_vecs)
    }
}

impl<LEN, T> ZeroCopyCyclicVec<LEN, T>
where
    LEN: FromPrimitive + ToPrimitive + PrimInt,
    T: Copy,
{
    pub fn from_bytes(account_data: &mut [u8]) -> Result<Self, ZeroCopyError> {
        Self::from_bytes_at(account_data, &mut 0)
    }

    pub fn from_bytes_at(
        account_data: &mut [u8],
        offset: &mut usize,
    ) -> Result<Self, ZeroCopyError> {
        let current_index = WrappedPointerMut::<LEN>::from_bytes_at(account_data, offset)?;
        add_padding::<LEN, T>(offset);
        let vec = ZeroCopyVec::<LEN, T>::from_bytes_at(account_data, offset)?;
        Ok(Self { current_index, vec })
    }

    pub fn from_bytes_at_multiple(
        num: usize,
        account_data: &mut [u8],
        offset: &mut usize,
    ) -> Result<Vec<Self>, ZeroCopyError> {
        let mut value_vecs = Vec::with_capacity(num);
        for _ in 0..num {
            let vec = Self::from_bytes_at(account_data, offset)?;
            value_vecs.push(vec);
        }
        Ok(value_vecs)
    }
}

impl<LEN, T> ZeroCopyCyclicVec<LEN, T>
where
    LEN: FromPrimitive + ToPrimitive + PrimInt,
    T: Copy,
{
    #[inline]
    pub fn push(&mut self, value: T) {
        if self.vec.len() < self.vec.capacity() {
            self.vec.push(value).unwrap();
        } else {
            let current_index = self.current_index();
            self.vec[current_index] = value;
        }
        let new_index = (self.current_index() + 1) % self.vec.capacity();
        *self.current_index = LEN::from_usize(new_index).unwrap();
    }

    #[inline]
    pub fn clear(&mut self) {
        *self.current_index = LEN::zero();
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
        (*self.current_index.get()).to_usize().unwrap()
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
    pub fn iter(&self) -> ZeroCopyCyclicVecIterator<'_, LEN, T> {
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
    ) -> Result<ZeroCopyCyclicVecIterator<'_, LEN, T>, ZeroCopyError> {
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
        let mut size = size_of::<LEN>();
        add_padding::<LEN, T>(&mut size);
        size
    }

    pub fn data_size(length: LEN) -> usize {
        ZeroCopyVec::<LEN, T>::required_size_for_capacity(length.to_usize().unwrap())
    }

    pub fn required_size_for_capacity(capacity: usize) -> usize {
        Self::metadata_size() + Self::data_size(LEN::from_usize(capacity).unwrap())
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

pub struct ZeroCopyCyclicVecIterator<'a, LEN, T>
where
    LEN: FromPrimitive + ToPrimitive + PrimInt,
    T: Copy,
{
    vec: &'a ZeroCopyCyclicVec<LEN, T>,
    current: usize,
    is_finished: bool,
    _marker: PhantomData<T>,
}

impl<'a, LEN, T> Iterator for ZeroCopyCyclicVecIterator<'a, LEN, T>
where
    LEN: FromPrimitive + ToPrimitive + PrimInt,
    T: Copy,
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

impl<LEN, T> IndexMut<usize> for ZeroCopyCyclicVec<LEN, T>
where
    LEN: FromPrimitive + ToPrimitive + PrimInt,
    T: Copy,
{
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        // Access the underlying mutable slice using as_mut_slice() and index it
        &mut self.as_mut_slice()[index]
    }
}

impl<LEN, T> Index<usize> for ZeroCopyCyclicVec<LEN, T>
where
    LEN: FromPrimitive + ToPrimitive + PrimInt,
    T: Copy,
{
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        // Access the underlying slice using as_slice() and index it
        &self.as_slice()[index]
    }
}

impl<LEN, T> PartialEq for ZeroCopyCyclicVec<LEN, T>
where
    LEN: FromPrimitive + ToPrimitive + PrimInt,
    T: Copy + PartialEq,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.vec == other.vec && *self.current_index == *other.current_index
    }
}

impl<LEN, T> fmt::Debug for ZeroCopyCyclicVec<LEN, T>
where
    LEN: FromPrimitive + ToPrimitive + PrimInt,
    T: Copy + Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_vec())
    }
}
