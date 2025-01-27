use core::{fmt, mem::size_of, ops::Index, slice};
#[cfg(feature = "std")]
use std::vec::Vec;

use zerocopy::{little_endian::U32, Ref};

use crate::{add_padding, borsh::Deserialize, errors::ZeroCopyError, ZeroCopyTraits};

pub type ZeroCopySliceU64<'a, T> = ZeroCopySlice<'a, u64, T>;
pub type ZeroCopySliceU32<'a, T> = ZeroCopySlice<'a, u32, T>;
pub type ZeroCopySliceU16<'a, T> = ZeroCopySlice<'a, u16, T>;
pub type ZeroCopySliceU8<'a, T> = ZeroCopySlice<'a, u8, T>;
pub type ZeroCopySliceBorsh<'a, T> = ZeroCopySlice<'a, U32, T, false>;

#[derive(Clone)]
pub struct ZeroCopySlice<'a, L, T, const PAD: bool = true>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
{
    length: Ref<&'a [u8], L>,
    bytes: Ref<&'a [u8], [T]>,
}

impl<'a, L, T, const PAD: bool> ZeroCopySlice<'a, L, T, PAD>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L>,
{
    #[inline]
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, ZeroCopyError> {
        Ok(Self::from_bytes_at(bytes)?.0)
    }

    #[inline]
    pub fn from_bytes_at(
        bytes: &'a [u8],
    ) -> Result<(ZeroCopySlice<'a, L, T, PAD>, &'a [u8]), ZeroCopyError> {
        let meta_data_size = Self::metadata_size();
        if bytes.len() < meta_data_size {
            return Err(ZeroCopyError::InsufficientMemoryAllocated(
                bytes.len(),
                meta_data_size,
            ));
        }

        let (meta_data, bytes) = bytes.split_at(meta_data_size);
        let (length, _padding) = Ref::<&[u8], L>::from_prefix(meta_data)?;
        let usize_len: usize = u64::from(*length) as usize;
        let full_vector_size = Self::data_size(*length);
        if bytes.len() < full_vector_size {
            return Err(ZeroCopyError::InsufficientMemoryAllocated(
                bytes.len(),
                full_vector_size + meta_data_size,
            ));
        }
        let (bytes, remaining_bytes) = Ref::<&[u8], [T]>::from_prefix_with_elems(bytes, usize_len)?;
        Ok((ZeroCopySlice { length, bytes }, remaining_bytes))
    }

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
        usize_len * size_of::<T>()
    }

    #[inline]
    pub fn required_size_for_capacity(capacity: L) -> usize {
        Self::metadata_size() + Self::data_size(capacity)
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
    pub fn last(&self) -> Option<&T> {
        self.get(self.len().saturating_sub(1))
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        self.bytes.as_ref()
    }

    #[inline]
    pub fn data_as_ptr(&self) -> *const T {
        self.as_slice().as_ptr()
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        self.as_slice().get(index)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn to_vec(&self) -> Vec<T> {
        self.as_slice().to_vec()
    }
}

impl<L, T, const PAD: bool> Index<usize> for ZeroCopySlice<'_, L, T, PAD>
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

impl<'a, L, T, const PAD: bool> IntoIterator for &'a ZeroCopySlice<'_, L, T, PAD>
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

impl<'a, L, T, const PAD: bool> ZeroCopySlice<'_, L, T, PAD>
where
    L: ZeroCopyTraits,
    T: ZeroCopyTraits,
    u64: From<L>,
{
    #[inline]
    pub fn iter(&'a self) -> slice::Iter<'a, T> {
        self.as_slice().iter()
    }
}

impl<L, T, const PAD: bool> PartialEq for ZeroCopySlice<'_, L, T, PAD>
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

impl<L, T, const PAD: bool> fmt::Debug for ZeroCopySlice<'_, L, T, PAD>
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

impl<'a, T: ZeroCopyTraits + Deserialize<'a>> Deserialize<'a> for ZeroCopySliceBorsh<'a, T> {
    type Output = Self;

    fn zero_copy_at(bytes: &'a [u8]) -> Result<(Self, &'a [u8]), ZeroCopyError> {
        ZeroCopySliceBorsh::from_bytes_at(bytes)
    }
}
