use core::{
    fmt::{self, Debug},
    marker::PhantomData,
    mem::size_of,
    ops::{Index, IndexMut},
};
#[cfg(feature = "std")]
use std::vec::Vec;

use zerocopy::{
    byte_slice::{ByteSliceMut, SplitByteSlice, SplitByteSliceMut},
    little_endian::U32,
    Ref,
};

use crate::{add_padding, errors::ZeroCopyError, ZeroCopyTraits};

/// Mutable aliases (existing API).
pub type ZeroCopyCyclicVecU32<'a, T> = ZeroCopyCyclicVec<&'a mut [u8], u32, T>;
pub type ZeroCopyCyclicVecU64<'a, T> = ZeroCopyCyclicVec<&'a mut [u8], u64, T>;
pub type ZeroCopyCyclicVecU16<'a, T> = ZeroCopyCyclicVec<&'a mut [u8], u16, T>;
pub type ZeroCopyCyclicVecU8<'a, T> = ZeroCopyCyclicVec<&'a mut [u8], u8, T>;
pub type ZeroCopyCyclicVecBorsh<'a, T> = ZeroCopyCyclicVec<&'a mut [u8], U32, T>;

/// Immutable aliases.
pub type ZeroCopyCyclicVecRefU64<'a, T> = ZeroCopyCyclicVec<&'a [u8], u64, T>;

pub struct ZeroCopyCyclicVec<B, L, T, const PAD: bool = true>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L> + TryInto<L>,
{
    /// [current_index, length, capacity]
    metadata: Ref<B, [L; 3]>,
    slice: Ref<B, [T]>,
}

const CURRENT_INDEX_INDEX: usize = 0;
const LENGTH_INDEX: usize = 1;
const CAPACITY_INDEX: usize = 2;

// ---------------------------------------------------------------------------
// Read-only methods (available for both &[u8] and &mut [u8]).
// ---------------------------------------------------------------------------
impl<B, L, T, const PAD: bool> ZeroCopyCyclicVec<B, L, T, PAD>
where
    B: SplitByteSlice,
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L> + TryInto<L>,
{
    #[inline]
    pub fn from_bytes_at(bytes: B) -> Result<(Self, B), ZeroCopyError> {
        let metadata_size = Self::metadata_size();
        if bytes.len() < metadata_size {
            return Err(ZeroCopyError::InsufficientMemoryAllocated(
                bytes.len(),
                metadata_size,
            ));
        }

        let (meta_data, bytes) = bytes
            .split_at(metadata_size)
            .map_err(|_| ZeroCopyError::InsufficientMemoryAllocated(0, metadata_size))?;
        let (metadata, _padding) = Ref::<B, [L; 3]>::from_prefix(meta_data)?;
        let usize_capacity: usize = u64::from(metadata[CAPACITY_INDEX]) as usize;
        let usize_len: usize = u64::from(metadata[LENGTH_INDEX]) as usize;
        let usize_current_index: usize = u64::from(metadata[CURRENT_INDEX_INDEX]) as usize;

        if usize_len > usize_capacity {
            return Err(ZeroCopyError::LengthGreaterThanCapacity);
        }

        if usize_current_index > usize_len {
            return Err(ZeroCopyError::CurrentIndexGreaterThanLength);
        }

        let full_vector_size = Self::data_size(metadata[CAPACITY_INDEX]);
        if bytes.len() < full_vector_size {
            return Err(ZeroCopyError::InsufficientMemoryAllocated(
                bytes.len() + metadata_size,
                full_vector_size + metadata_size,
            ));
        }
        let (slice, remaining_bytes) =
            Ref::<B, [T]>::from_prefix_with_elems(bytes, usize_capacity)?;
        Ok((Self { metadata, slice }, remaining_bytes))
    }

    #[inline]
    pub fn from_bytes(bytes: B) -> Result<Self, ZeroCopyError> {
        Ok(Self::from_bytes_at(bytes)?.0)
    }

    #[inline]
    fn get_current_index(&self) -> L {
        self.metadata[CURRENT_INDEX_INDEX]
    }

    #[inline]
    fn get_len(&self) -> L {
        self.metadata[LENGTH_INDEX]
    }

    #[inline]
    fn get_capacity(&self) -> L {
        self.metadata[CAPACITY_INDEX]
    }

    #[inline]
    fn current_index(&self) -> usize {
        u64::from(self.get_current_index()) as usize
    }

    #[inline]
    pub fn first(&self) -> Option<&T> {
        self.get(self.first_index())
    }

    #[inline]
    pub fn last(&self) -> Option<&T> {
        self.get(self.last_index())
    }

    /// First index is the next index after the last index mod capacity.
    #[inline]
    pub fn first_index(&self) -> usize {
        if self.len() < self.capacity() {
            0
        } else {
            self.last_index().saturating_add(1) % (self.capacity())
        }
    }

    #[inline]
    pub fn last_index(&self) -> usize {
        if self.current_index() == 0 && self.len() == self.capacity() {
            self.capacity().saturating_sub(1)
        } else {
            self.current_index().saturating_sub(1) % self.capacity()
        }
    }

    #[inline]
    pub fn metadata_size() -> usize {
        let mut size = size_of::<[L; 3]>();
        if PAD {
            add_padding::<[L; 3], T>(&mut size);
        }
        size
    }

    #[inline]
    pub fn data_size(capacity: L) -> usize {
        let usize_len: usize = u64::from(capacity) as usize;
        usize_len.saturating_mul(size_of::<T>())
    }

    pub fn required_size_for_capacity(capacity: L) -> usize {
        Self::metadata_size().saturating_add(Self::data_size(capacity))
    }

    #[inline]
    pub fn len(&self) -> usize {
        u64::from(self.get_len()) as usize
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        u64::from(self.get_capacity()) as usize
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
    pub fn as_slice(&self) -> &[T] {
        &self.slice[..self.len()]
    }

    #[cfg(feature = "std")]
    pub fn try_into_array<const N: usize>(&self) -> Result<[T; N], ZeroCopyError> {
        if self.len() != N {
            return Err(ZeroCopyError::ArraySize(N, self.len()));
        }
        Ok(core::array::from_fn(|i| *self.get(i).unwrap()))
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn to_vec(&self) -> Vec<T> {
        self.as_slice().to_vec()
    }
}

// ---------------------------------------------------------------------------
// Mutable construction (only &mut [u8]).
// ---------------------------------------------------------------------------
impl<B, L, T, const PAD: bool> ZeroCopyCyclicVec<B, L, T, PAD>
where
    B: SplitByteSliceMut,
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L> + TryInto<L>,
{
    pub fn new(capacity: L, bytes: B) -> Result<Self, ZeroCopyError> {
        Ok(Self::new_at(capacity, bytes)?.0)
    }

    pub fn new_at(capacity: L, bytes: B) -> Result<(Self, B), ZeroCopyError> {
        if u64::from(capacity) == 0 {
            return Err(ZeroCopyError::InvalidCapacity);
        }
        let metadata_size = Self::metadata_size();
        if bytes.len() < metadata_size {
            return Err(ZeroCopyError::InsufficientMemoryAllocated(
                bytes.len(),
                metadata_size,
            ));
        }
        let (meta_data, bytes) = bytes
            .split_at(metadata_size)
            .map_err(|_| ZeroCopyError::InsufficientMemoryAllocated(0, metadata_size))?;

        let (mut metadata, _padding) = Ref::<B, [L; 3]>::from_prefix(meta_data)?;

        if u64::from(metadata[LENGTH_INDEX]) != 0
            || u64::from(metadata[CURRENT_INDEX_INDEX]) != 0
            || u64::from(metadata[CAPACITY_INDEX]) != 0
        {
            return Err(ZeroCopyError::MemoryNotZeroed);
        }
        metadata[CAPACITY_INDEX] = capacity;
        let capacity_usize: usize = u64::from(metadata[CAPACITY_INDEX]) as usize;

        let (slice, remaining_bytes) =
            Ref::<B, [T]>::from_prefix_with_elems(bytes, capacity_usize)?;
        Ok((Self { metadata, slice }, remaining_bytes))
    }
}

// ---------------------------------------------------------------------------
// Mutable access methods (only &mut [u8]).
// ---------------------------------------------------------------------------
impl<B, L, T, const PAD: bool> ZeroCopyCyclicVec<B, L, T, PAD>
where
    B: ByteSliceMut + SplitByteSlice,
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L> + TryInto<L>,
{
    #[inline]
    fn get_current_index_mut(&mut self) -> &mut L {
        &mut self.metadata[CURRENT_INDEX_INDEX]
    }

    #[inline]
    fn get_len_mut(&mut self) -> &mut L {
        &mut self.metadata[LENGTH_INDEX]
    }

    #[inline]
    pub fn push(&mut self, value: T) {
        if self.len() < self.capacity() {
            let len = self.len();
            self.slice[len] = value;
            *self.get_len_mut() = (len as u64 + 1u64)
                .try_into()
                .map_err(|_| ZeroCopyError::InvalidConversion)
                .unwrap();
        } else {
            let current_index = self.current_index();
            self.slice[current_index] = value;
        }
        let new_index = (self.current_index() + 1) % self.capacity();
        *self.get_current_index_mut() = (new_index as u64)
            .try_into()
            .map_err(|_| ZeroCopyError::InvalidConversion)
            .unwrap();
    }

    #[inline]
    pub fn clear(&mut self) {
        *self.get_current_index_mut() = 0
            .try_into()
            .map_err(|_| ZeroCopyError::InvalidConversion)
            .unwrap();
        *self.get_len_mut() = self.get_current_index();
    }

    #[inline]
    pub fn first_mut(&mut self) -> Option<&mut T> {
        self.get_mut(self.first_index())
    }

    #[inline]
    pub fn last_mut(&mut self) -> Option<&mut T> {
        self.get_mut(self.last_index())
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len() {
            return None;
        }
        Some(&mut self.slice[index])
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        let len = self.len();
        &mut self.slice[..len]
    }
}

// ---------------------------------------------------------------------------
// Iterator (read-only).
// ---------------------------------------------------------------------------
impl<B, L, T, const PAD: bool> ZeroCopyCyclicVec<B, L, T, PAD>
where
    B: SplitByteSlice,
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L> + TryInto<L>,
{
    #[inline]
    pub fn iter(&self) -> ZeroCopyCyclicVecIterator<'_, B, L, T, PAD> {
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
    ) -> Result<ZeroCopyCyclicVecIterator<'_, B, L, T, PAD>, ZeroCopyError> {
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
}

pub struct ZeroCopyCyclicVecIterator<'a, B, L, T, const PAD: bool>
where
    B: SplitByteSlice,
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L> + TryInto<L>,
{
    vec: &'a ZeroCopyCyclicVec<B, L, T, PAD>,
    current: usize,
    is_finished: bool,
    _marker: PhantomData<T>,
}

impl<'a, B, L, T, const PAD: bool> Iterator for ZeroCopyCyclicVecIterator<'a, B, L, T, PAD>
where
    B: SplitByteSlice,
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
            // Perform one more iteration to perform len() iterations.
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

// ---------------------------------------------------------------------------
// Index / IndexMut / trait impls.
// ---------------------------------------------------------------------------
impl<B, L, T, const PAD: bool> IndexMut<usize> for ZeroCopyCyclicVec<B, L, T, PAD>
where
    B: ByteSliceMut + SplitByteSlice,
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L> + TryInto<L>,
{
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.as_mut_slice()[index]
    }
}

impl<B, L, T, const PAD: bool> Index<usize> for ZeroCopyCyclicVec<B, L, T, PAD>
where
    B: SplitByteSlice,
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L> + TryInto<L>,
{
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl<B, L, T, const PAD: bool> PartialEq for ZeroCopyCyclicVec<B, L, T, PAD>
where
    B: SplitByteSlice,
    L: ZeroCopyTraits + PartialEq,
    T: ZeroCopyTraits + PartialEq,
    u64: From<L> + TryInto<L>,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice() && self.get_current_index() == other.get_current_index()
    }
}

impl<B, L, T, const PAD: bool> fmt::Debug for ZeroCopyCyclicVec<B, L, T, PAD>
where
    B: SplitByteSlice,
    L: ZeroCopyTraits,
    T: ZeroCopyTraits + Debug,
    u64: From<L> + TryInto<L>,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.as_slice())
    }
}

#[test]
fn test_private_getters() {
    let mut backing_store = [0u8; 64];
    let mut zcv = ZeroCopyCyclicVecU16::<u16>::new(5, &mut backing_store[..]).unwrap();
    assert_eq!(zcv.get_len(), 0);
    assert_eq!(zcv.get_capacity(), 5);
    for i in 0..5 {
        zcv.push(i);
        assert_eq!(zcv.get_len(), i + 1);
        assert_eq!(zcv.get_len_mut(), &mut (i + 1));
    }
}
