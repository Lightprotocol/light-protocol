use core::{
    fmt,
    mem::size_of,
    ops::{Index, IndexMut},
    slice,
};
#[cfg(feature = "std")]
use std::vec::Vec;

use zerocopy::{little_endian::U32, Ref};

use crate::{add_padding, errors::ZeroCopyError, ZeroCopyTraits};

pub type ZeroCopySliceMutU64<'a, T> = ZeroCopySliceMut<'a, u64, T>;
pub type ZeroCopySliceMutU32<'a, T> = ZeroCopySliceMut<'a, u32, T>;
pub type ZeroCopySliceMutU16<'a, T> = ZeroCopySliceMut<'a, u16, T>;
pub type ZeroCopySliceMutU8<'a, T> = ZeroCopySliceMut<'a, u8, T>;
pub type ZeroCopySliceMutBorsh<'a, T> = ZeroCopySliceMut<'a, U32, T, false>;

pub struct ZeroCopySliceMut<'a, L, T, const PAD: bool = true>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
{
    length: Ref<&'a [u8], L>,
    bytes: Ref<&'a mut [u8], [T]>,
}

impl<'a, L, T, const PAD: bool> ZeroCopySliceMut<'a, L, T, PAD>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L>,
{
    pub fn new(length: L, bytes: &'a mut [u8]) -> Result<Self, ZeroCopyError> {
        Ok(Self::new_at(length, bytes)?.0)
    }

    pub fn new_at(length: L, bytes: &'a mut [u8]) -> Result<(Self, &'a mut [u8]), ZeroCopyError> {
        let len = Self::required_size_for_capacity(length);
        if len > bytes.len() {
            return Err(ZeroCopyError::InsufficientMemoryAllocated(bytes.len(), len));
        }
        // write new value then deserialize as immutable
        {
            let (mut len, _) = Ref::<&mut [u8], L>::from_prefix(bytes)?;
            if u64::from(*len) != 0 {
                return Err(ZeroCopyError::MemoryNotZeroed);
            }
            Ref::<&mut [u8], L>::write(&mut len, length);
        }
        Self::from_bytes_at(bytes)
    }

    #[inline]
    pub fn from_bytes(bytes: &'a mut [u8]) -> Result<Self, ZeroCopyError> {
        Ok(Self::from_bytes_at(bytes)?.0)
    }

    #[inline]
    pub fn from_bytes_at(
        bytes: &'a mut [u8],
    ) -> Result<(ZeroCopySliceMut<'a, L, T, PAD>, &'a mut [u8]), ZeroCopyError> {
        let metadata_size = Self::metadata_size();
        if bytes.len() < metadata_size {
            return Err(ZeroCopyError::InsufficientMemoryAllocated(
                bytes.len(),
                metadata_size,
            ));
        }

        let (meta_data, bytes) = bytes.split_at_mut(metadata_size);
        let (length, _padding) = Ref::<&[u8], L>::from_prefix(meta_data)?;
        let usize_len: usize = u64::from(*length) as usize;
        let full_vector_size = Self::data_size(*length);
        if bytes.len() < full_vector_size {
            return Err(ZeroCopyError::InsufficientMemoryAllocated(
                bytes.len() + metadata_size,
                full_vector_size + metadata_size,
            ));
        }
        let (bytes, remaining_bytes) =
            Ref::<&mut [u8], [T]>::from_prefix_with_elems(bytes, usize_len)?;
        Ok((ZeroCopySliceMut { length, bytes }, remaining_bytes))
    }

    #[cfg(feature = "std")]
    pub fn try_into_array<const N: usize>(&self) -> Result<[T; N], ZeroCopyError> {
        if self.len() != N {
            return Err(ZeroCopyError::ArraySize(N, self.len()));
        }
        Ok(core::array::from_fn(|i| *self.get(i).unwrap()))
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
        let usize_len: usize = u64::from(length) as usize;
        usize_len.saturating_mul(size_of::<T>())
    }

    #[inline]
    pub fn required_size_for_capacity(capacity: L) -> usize {
        Self::metadata_size().saturating_add(Self::data_size(capacity))
    }

    #[inline]
    pub fn len(&self) -> usize {
        let usize_len: usize = u64::from(*self.length) as usize;
        usize_len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
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
        self.bytes.as_ref()
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.bytes.as_mut()
    }

    #[inline]
    pub fn data_as_ptr(&self) -> *const T {
        self.as_slice().as_ptr()
    }

    #[inline]
    pub fn data_as_mut_ptr(&mut self) -> *mut T {
        self.as_mut_slice().as_mut_ptr()
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.as_slice().get(index)
    }

    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.as_mut_slice().get_mut(index)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn to_vec(&self) -> Vec<T> {
        self.as_slice().to_vec()
    }
}

impl<L, T, const PAD: bool> IndexMut<usize> for ZeroCopySliceMut<'_, L, T, PAD>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L>,
{
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.as_mut_slice()[index]
    }
}

impl<L, T, const PAD: bool> Index<usize> for ZeroCopySliceMut<'_, L, T, PAD>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L>,
{
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl<'a, L, T, const PAD: bool> IntoIterator for &'a ZeroCopySliceMut<'_, L, T, PAD>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L>,
{
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, L, T, const PAD: bool> IntoIterator for &'a mut ZeroCopySliceMut<'_, L, T, PAD>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L>,
{
    type Item = &'a mut T;
    type IntoIter = slice::IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<'a, L, T, const PAD: bool> ZeroCopySliceMut<'_, L, T, PAD>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L>,
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

impl<L, T, const PAD: bool> PartialEq for ZeroCopySliceMut<'_, L, T, PAD>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits + PartialEq,
    u64: From<L>,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<L, T, const PAD: bool> fmt::Debug for ZeroCopySliceMut<'_, L, T, PAD>
where
    T: ZeroCopyTraits + fmt::Debug,
    L: ZeroCopyTraits,
    u64: From<L>,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.as_slice())
    }
}

#[cfg(feature = "std")]
impl<'a, T: ZeroCopyTraits + crate::traits::ZeroCopyAtMut<'a>> crate::traits::ZeroCopyAtMut<'a>
    for ZeroCopySliceMutBorsh<'_, T>
{
    type ZeroCopyAtMut = ZeroCopySliceMutBorsh<'a, T>;

    fn zero_copy_at_mut(
        bytes: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
        ZeroCopySliceMutBorsh::from_bytes_at(bytes)
    }
}
