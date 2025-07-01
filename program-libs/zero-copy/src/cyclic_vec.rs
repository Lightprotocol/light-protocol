use core::{
    fmt::{self, Debug},
    marker::PhantomData,
    mem::size_of,
    ops::{Index, IndexMut},
};
#[cfg(feature = "std")]
use std::vec::Vec;

use zerocopy::{little_endian::U32, Ref};

use crate::{add_padding, errors::ZeroCopyError, ZeroCopyTraits};

pub type ZeroCopyCyclicVecU32<'a, T> = ZeroCopyCyclicVec<'a, u32, T>;
pub type ZeroCopyCyclicVecU64<'a, T> = ZeroCopyCyclicVec<'a, u64, T>;
pub type ZeroCopyCyclicVecU16<'a, T> = ZeroCopyCyclicVec<'a, u16, T>;
pub type ZeroCopyCyclicVecU8<'a, T> = ZeroCopyCyclicVec<'a, u8, T>;
pub type ZeroCopyCyclicVecBorsh<'a, T> = ZeroCopyCyclicVec<'a, U32, T>;

pub struct ZeroCopyCyclicVec<'a, L, T, const PAD: bool = true>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L> + TryInto<L>,
{
    /// [current_index, length, capacity]
    metadata: Ref<&'a mut [u8], [L; 3]>,
    slice: Ref<&'a mut [u8], [T]>,
}

const CURRENT_INDEX_INDEX: usize = 0;
const LENGTH_INDEX: usize = 1;
const CAPACITY_INDEX: usize = 2;

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
        let (meta_data, bytes) = bytes.split_at_mut(metadata_size);

        let (mut metadata, _padding) = Ref::<&mut [u8], [L; 3]>::from_prefix(meta_data)?;

        if u64::from(metadata[LENGTH_INDEX]) != 0
            || u64::from(metadata[CURRENT_INDEX_INDEX]) != 0
            || u64::from(metadata[CAPACITY_INDEX]) != 0
        {
            return Err(ZeroCopyError::MemoryNotZeroed);
        }
        metadata[CAPACITY_INDEX] = capacity;
        let capacity_usize: usize = u64::from(metadata[CAPACITY_INDEX]) as usize;

        let (slice, remaining_bytes) =
            Ref::<&mut [u8], [T]>::from_prefix_with_elems(bytes, capacity_usize)?;
        Ok((Self { metadata, slice }, remaining_bytes))
    }

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
        let (metadata, _padding) = Ref::<&mut [u8], [L; 3]>::from_prefix(meta_data)?;
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
            Ref::<&mut [u8], [T]>::from_prefix_with_elems(bytes, usize_capacity)?;
        Ok((Self { metadata, slice }, remaining_bytes))
    }

    /// Convenience method to get the current index of the vector.
    #[inline]
    fn get_current_index(&self) -> L {
        self.metadata[CURRENT_INDEX_INDEX]
    }

    /// Convenience method to get the current index of the vector.
    #[inline]
    fn get_current_index_mut(&mut self) -> &mut L {
        &mut self.metadata[CURRENT_INDEX_INDEX]
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
    pub fn first(&self) -> Option<&T> {
        self.get(self.first_index())
    }

    #[inline]
    pub fn first_mut(&mut self) -> Option<&mut T> {
        self.get_mut(self.first_index())
    }

    #[inline]
    pub fn last(&self) -> Option<&T> {
        self.get(self.last_index())
    }

    #[inline]
    pub fn last_mut(&mut self) -> Option<&mut T> {
        self.get_mut(self.last_index())
    }

    #[inline]
    fn current_index(&self) -> usize {
        u64::from(self.get_current_index()) as usize
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
    pub fn iter(&self) -> ZeroCopyCyclicVecIterator<'_, L, T, PAD> {
        ZeroCopyCyclicVecIterator {
            vec: self,
            front: self.first_index(),
            back: self.last_index(),
            remaining: self.len(),
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
            front: start,
            back: self.last_index(),
            remaining: self.len() - start,
            _marker: PhantomData,
        })
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
        usize_len * size_of::<T>()
    }

    pub fn required_size_for_capacity(capacity: L) -> usize {
        Self::metadata_size() + Self::data_size(capacity)
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
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.len() {
            return None;
        }
        Some(&mut self.slice[index])
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

pub struct ZeroCopyCyclicVecIterator<'a, L, T, const PAD: bool>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L> + TryInto<L>,
{
    vec: &'a ZeroCopyCyclicVec<'a, L, T, PAD>,
    front: usize,
    back: usize,
    remaining: usize,
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
        if self.remaining == 0 {
            None
        } else {
            let element = self.vec.get(self.front);
            self.front = (self.front + 1) % self.vec.capacity();
            self.remaining -= 1;
            element
        }
    }
    
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a, L, T, const PAD: bool> DoubleEndedIterator for ZeroCopyCyclicVecIterator<'a, L, T, PAD>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L> + TryInto<L>,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            None
        } else {
            let element = self.vec.get(self.back);
            self.back = if self.back == 0 {
                self.vec.capacity() - 1
            } else {
                self.back - 1
            };
            self.remaining -= 1;
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
        self.as_slice() == other.as_slice() && self.get_current_index() == other.get_current_index()
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
        write!(f, "{:?}", self.as_slice())
    }
}

#[test]
fn test_private_getters() {
    let mut backing_store = [0u8; 64];
    let mut zcv = ZeroCopyCyclicVecU16::<u16>::new(5, &mut backing_store).unwrap();
    assert_eq!(zcv.get_len(), 0);
    assert_eq!(zcv.get_capacity(), 5);
    for i in 0..5 {
        zcv.push(i);
        assert_eq!(zcv.get_len(), i + 1);
        assert_eq!(zcv.get_len_mut(), &mut (i + 1));
    }
}

#[test]
fn test_double_ended_iterator() {
    let mut backing_store = [0u8; 64];
    let mut zcv = ZeroCopyCyclicVecU16::<u16>::new(5, &mut backing_store).unwrap();
    
    // Push some values
    for i in 0..5 {
        zcv.push(i);
    }
    
    // Test forward iteration
    let mut iter = zcv.iter();
    assert_eq!(iter.next(), Some(&0));
    assert_eq!(iter.next(), Some(&1));
    assert_eq!(iter.next(), Some(&2));
    assert_eq!(iter.next(), Some(&3));
    assert_eq!(iter.next(), Some(&4));
    assert_eq!(iter.next(), None);
    
    // Test backward iteration
    let mut iter = zcv.iter();
    assert_eq!(iter.next_back(), Some(&4));
    assert_eq!(iter.next_back(), Some(&3));
    assert_eq!(iter.next_back(), Some(&2));
    assert_eq!(iter.next_back(), Some(&1));
    assert_eq!(iter.next_back(), Some(&0));
    assert_eq!(iter.next_back(), None);
    
    // Test mixed iteration
    let mut iter = zcv.iter();
    assert_eq!(iter.next(), Some(&0));  // forward
    assert_eq!(iter.next_back(), Some(&4));  // backward
    assert_eq!(iter.next(), Some(&1));  // forward
    assert_eq!(iter.next_back(), Some(&3));  // backward
    assert_eq!(iter.next(), Some(&2));  // forward (last element)
    assert_eq!(iter.next(), None);  // exhausted
    assert_eq!(iter.next_back(), None);  // exhausted
}

#[test]
fn test_double_ended_iterator_wrap_around() {
    let mut backing_store = [0u8; 64];
    let mut zcv = ZeroCopyCyclicVecU16::<u16>::new(3, &mut backing_store).unwrap();
    
    // Fill capacity and then push more to cause wrap-around
    for i in 0..5 {
        zcv.push(i * 10);
    }
    
    // Debug: Let's see what the actual state is
    // After pushing 5 elements to capacity 3:
    // push(0): [0, _, _], current_index=1, len=1
    // push(10): [0, 10, _], current_index=2, len=2  
    // push(20): [0, 10, 20], current_index=0, len=3
    // push(30): [30, 10, 20], current_index=1, len=3 (overwrites index 0)
    // push(40): [30, 40, 20], current_index=2, len=3 (overwrites index 1)
    // So: [30, 40, 20] with current_index=2, first_index=2, last_index=1
    
    // Test what first() and last() actually return
    assert_eq!(zcv.first(), Some(&20));  // first element in logical order
    assert_eq!(zcv.last(), Some(&40));   // last element in logical order
    
    // Test forward iteration: should be [20, 30, 40] (first to last in logical order)
    let mut iter = zcv.iter();
    assert_eq!(iter.next(), Some(&20));
    assert_eq!(iter.next(), Some(&30));
    assert_eq!(iter.next(), Some(&40));
    assert_eq!(iter.next(), None);
    
    // Test backward iteration: should be [40, 30, 20] (last to first in logical order)
    let mut iter = zcv.iter();
    assert_eq!(iter.next_back(), Some(&40));
    assert_eq!(iter.next_back(), Some(&30));
    assert_eq!(iter.next_back(), Some(&20));
    assert_eq!(iter.next_back(), None);
    
    // Test mixed iteration - should handle wrap around at index 0
    let mut iter = zcv.iter();
    assert_eq!(iter.next(), Some(&20));      // forward from first
    assert_eq!(iter.next_back(), Some(&40)); // backward from last  
    assert_eq!(iter.next(), Some(&30));      // forward (remaining element)
    assert_eq!(iter.next(), None);           // exhausted
    assert_eq!(iter.next_back(), None);      // exhausted
}
