use core::{
    fmt,
    mem::size_of,
    ops::{Index, IndexMut},
    slice,
};
#[cfg(feature = "std")]
use std::vec::Vec;

use zerocopy::{little_endian::U32, IntoBytes, Ref};

use crate::{add_padding, errors::ZeroCopyError, ZeroCopyTraits};

pub type ZeroCopyVecU64<'a, T> = ZeroCopyVec<'a, u64, T>;
pub type ZeroCopyVecU32<'a, T> = ZeroCopyVec<'a, u32, T>;
pub type ZeroCopyVecU16<'a, T> = ZeroCopyVec<'a, u16, T>;
pub type ZeroCopyVecU8<'a, T> = ZeroCopyVec<'a, u8, T>;
pub type ZeroCopyVecBorsh<'a, T> = ZeroCopyVec<'a, U32, T, false>;

/// `ZeroCopyVec` is a custom vector implementation which forbids
/// post-initialization reallocations. The size is not known during compile
/// time (that makes it different from arrays), but can be defined only once
/// (that makes it different from [`Vec`](std::vec::Vec)).
pub struct ZeroCopyVec<'a, L, T, const PAD: bool = true>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
{
    /// [length, capacity]
    metadata: Ref<&'a mut [u8], [L; 2]>,
    slice: Ref<&'a mut [u8], [T]>,
}

const LENGTH_INDEX: usize = 0;
const CAPACITY_INDEX: usize = 1;

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
        let metadata_size = Self::metadata_size();
        if bytes.len() < metadata_size {
            return Err(ZeroCopyError::InsufficientMemoryAllocated(
                bytes.len(),
                metadata_size,
            ));
        }
        let (meta_data, bytes) = bytes.split_at_mut(metadata_size);

        let (mut metadata, _padding) = Ref::<&mut [u8], [L; 2]>::from_prefix(meta_data)?;
        if u64::from(metadata[LENGTH_INDEX]) != 0 || u64::from(metadata[CAPACITY_INDEX]) != 0 {
            return Err(ZeroCopyError::MemoryNotZeroed);
        }
        metadata[CAPACITY_INDEX] = capacity;
        let capacity_usize: usize = u64::from(metadata[CAPACITY_INDEX]) as usize;

        let (slice, remaining_bytes) =
            Ref::<&mut [u8], [T]>::from_prefix_with_elems(bytes, capacity_usize)?;
        Ok((Self { metadata, slice }, remaining_bytes))
    }

    #[inline]
    pub fn from_bytes(bytes: &'a mut [u8]) -> Result<Self, ZeroCopyError> {
        Ok(Self::from_bytes_at(bytes)?.0)
    }

    #[inline]
    pub fn from_bytes_at(bytes: &'a mut [u8]) -> Result<(Self, &'a mut [u8]), ZeroCopyError> {
        let metadata_size = Self::metadata_size();
        if bytes.len() < metadata_size {
            return Err(ZeroCopyError::InsufficientMemoryAllocated(
                bytes.len(),
                metadata_size,
            ));
        }

        let (meta_data, bytes) = bytes.split_at_mut(metadata_size);
        let (metadata, _padding) = Ref::<&mut [u8], [L; 2]>::from_prefix(meta_data)?;
        let usize_capacity: usize = u64::from(metadata[CAPACITY_INDEX]) as usize;
        let usize_len: usize = u64::from(metadata[LENGTH_INDEX]) as usize;

        if usize_len > usize_capacity {
            return Err(ZeroCopyError::LengthGreaterThanCapacity);
        }

        let full_vector_size = Self::data_size(metadata[CAPACITY_INDEX]);
        if bytes.len() < full_vector_size {
            return Err(ZeroCopyError::InsufficientMemoryAllocated(
                bytes.len() + metadata_size,
                full_vector_size + metadata_size,
            ));
        }
        let (slice, remaining_bytes) =
            Ref::<&mut [u8], [T]>::from_prefix_with_elems(bytes, usize_capacity)?;
        Ok((Self { metadata, slice }, remaining_bytes))
    }

    /// Convenience method to get the length of the vector.
    #[inline]
    fn get_len(&self) -> L {
        self.metadata[LENGTH_INDEX]
    }

    /// Convenience method to get the length of the vector.
    #[inline]
    fn get_len_mut(&mut self) -> &mut L {
        &mut self.metadata[LENGTH_INDEX]
    }

    /// Convenience method to get the capacity of the vector.
    #[inline]
    fn get_capacity(&self) -> L {
        self.metadata[CAPACITY_INDEX]
    }

    #[inline]
    pub fn push(&mut self, value: T) -> Result<(), ZeroCopyError> {
        if self.len() == self.capacity() {
            return Err(ZeroCopyError::Full);
        }

        let len = self.len();
        self.slice[len] = value;
        *self.get_len_mut() = (len as u64 + 1u64)
            .try_into()
            .map_err(|_| ZeroCopyError::InvalidConversion)?;

        Ok(())
    }

    #[inline]
    pub fn clear(&mut self) {
        *self.get_len_mut() = 0
            .try_into()
            .map_err(|_| ZeroCopyError::InvalidConversion)
            .unwrap();
    }

    #[inline]
    pub fn zero_out(&mut self) {
        *self.get_len_mut() = 0
            .try_into()
            .map_err(|_| ZeroCopyError::InvalidConversion)
            .unwrap();
        self.slice.as_mut_bytes().fill(0);
    }

    #[inline]
    pub fn metadata_size() -> usize {
        let mut size = size_of::<[L; 2]>();
        if PAD {
            add_padding::<[L; 2], T>(&mut size);
        }
        size
    }

    #[inline]
    pub fn data_size(capacity: L) -> usize {
        let usize_len: usize = u64::from(capacity) as usize;
        usize_len.saturating_mul(size_of::<T>())
    }

    #[inline]
    pub fn required_size_for_capacity(capacity: L) -> usize {
        Self::metadata_size().saturating_add(Self::data_size(capacity))
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        u64::from(self.get_capacity()) as usize
    }

    #[inline]
    pub fn len(&self) -> usize {
        u64::from(self.get_len()) as usize
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
        &self.slice[..self.len()]
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        let len = self.len();
        &mut self.slice[..len]
    }

    pub fn extend_from_slice(&mut self, slice: &[T]) -> Result<(), ZeroCopyError> {
        let len = self.len();
        let new_len = len + slice.len();
        if new_len > self.capacity() {
            return Err(ZeroCopyError::InsufficientCapacity);
        }
        self.slice[len..].copy_from_slice(slice);
        *self.get_len_mut() = (new_len as u64)
            .try_into()
            .map_err(|_| ZeroCopyError::InvalidConversion)?;
        Ok(())
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn to_vec(&self) -> Vec<T> {
        self.as_slice().to_vec()
    }

    #[cfg(feature = "std")]
    pub fn try_into_array<const N: usize>(&self) -> Result<[T; N], ZeroCopyError> {
        if self.len() != N {
            return Err(ZeroCopyError::ArraySize(N, self.len()));
        }
        Ok(core::array::from_fn(|i| *self.get(i).unwrap()))
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

impl<'a, L, T, const PAD: bool> IntoIterator for &'a ZeroCopyVec<'_, L, T, PAD>
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

impl<'a, L, T, const PAD: bool> IntoIterator for &'a mut ZeroCopyVec<'_, L, T, PAD>
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

impl<'a, L, T, const PAD: bool> ZeroCopyVec<'_, L, T, PAD>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L> + TryInto<L>,
{
    #[inline]
    pub fn iter(&'a self) -> slice::Iter<'a, T> {
        self.as_slice().iter()
    }

    #[inline]
    pub fn iter_mut(&'a mut self) -> slice::IterMut<'a, T> {
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
        write!(f, "{:?}", self.as_slice())
    }
}

#[test]
fn test_private_getters() {
    let mut backing_store = [0u8; 64];
    let mut zcv = ZeroCopyVec::<u16, u16>::new(5, &mut backing_store).unwrap();
    assert_eq!(zcv.get_len(), 0);
    assert_eq!(zcv.get_capacity(), 5);
    for i in 0..5 {
        zcv.push(i).unwrap();
        assert_eq!(zcv.get_len(), i + 1);
        assert_eq!(zcv.get_len_mut(), &mut (i + 1));
    }
}
