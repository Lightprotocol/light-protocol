use core::{fmt, slice};
use std::{
    marker::PhantomData,
    mem::{size_of, ManuallyDrop},
    ops::{Index, IndexMut},
};

use crate::{
    add_padding, check_alignment, errors::ZeroCopyError, wrapped_pointer::WrappedPointer, Length,
};

pub type ZeroCopySliceMutUsize<T> = ZeroCopySliceMut<usize, T>;
pub type ZeroCopySliceMutU32<T> = ZeroCopySliceMut<u32, T>;
pub type ZeroCopySliceMutU64<T> = ZeroCopySliceMut<u64, T>;
pub type ZeroCopySliceMutU16<T> = ZeroCopySliceMut<u16, T>;
pub type ZeroCopySliceMutU8<T> = ZeroCopySliceMut<u8, T>;

#[repr(C)]
pub struct ZeroCopySliceMut<L, T>
where
    L: Copy,
{
    length: WrappedPointer<L>,
    data: ManuallyDrop<*mut T>,
    _marker: PhantomData<T>,
}

impl<L: Length, T: Copy> ZeroCopySliceMut<L, T> {
    pub fn new(length: L, data: &mut [u8]) -> Result<Self, ZeroCopyError> {
        Self::new_at(length, data, &mut 0)
    }

    pub fn new_at(length: L, data: &mut [u8], offset: &mut usize) -> Result<Self, ZeroCopyError> {
        let data = data.split_at_mut(*offset).1;
        if Self::required_size_for_capacity(length.try_into().unwrap()) > data.len() {
            return Err(ZeroCopyError::InsufficientMemoryAllocated(
                data.len(),
                Self::required_size_for_capacity(length.try_into().unwrap()),
            ));
        }

        let metadata_size = Self::metadata_size();
        *offset += metadata_size;
        let (metadata, data) = data.split_at_mut(metadata_size);
        let len = WrappedPointer::<L>::new(length, metadata)?;
        let data = Self::data_ptr_from_bytes(data)?;
        *offset += Self::data_size(length);

        Ok(Self {
            length: len,
            data,
            _marker: PhantomData,
        })
    }

    pub fn new_at_multiple(
        num_slices: usize,
        capacity: L,
        bytes: &mut [u8],
        offset: &mut usize,
    ) -> Result<Vec<ZeroCopySliceMut<L, T>>, ZeroCopyError> {
        let mut slices = Vec::with_capacity(num_slices);
        for _ in 0..num_slices {
            let slice = Self::new_at(capacity, bytes, offset)?;
            slices.push(slice);
        }
        Ok(slices)
    }

    fn data_ptr_from_bytes(bytes: &mut [u8]) -> Result<ManuallyDrop<*mut T>, ZeroCopyError> {
        let data_ptr = bytes.as_mut_ptr() as *mut T;
        check_alignment(data_ptr)?;
        let data = ManuallyDrop::new(data_ptr);
        Ok(data)
    }

    pub fn from_bytes(bytes: &mut [u8]) -> Result<Self, ZeroCopyError> {
        Self::from_bytes_at(bytes, &mut 0)
    }

    pub fn from_bytes_at(
        bytes: &mut [u8],
        offset: &mut usize,
    ) -> Result<ZeroCopySliceMut<L, T>, ZeroCopyError> {
        let meta_data_size = Self::metadata_size();
        if bytes.len().saturating_sub(*offset) < meta_data_size {
            return Err(ZeroCopyError::InsufficientMemoryAllocated(
                bytes.len().saturating_sub(*offset),
                meta_data_size,
            ));
        }
        let length = WrappedPointer::<L>::from_bytes_at(bytes, offset)?;
        add_padding::<L, T>(offset);
        let full_vector_size = Self::data_size(*length);
        if bytes.len().saturating_sub(*offset) < full_vector_size {
            return Err(ZeroCopyError::InsufficientMemoryAllocated(
                bytes.len().saturating_sub(*offset),
                full_vector_size + meta_data_size,
            ));
        }
        let bytes = &mut bytes[*offset..];
        *offset += full_vector_size;

        Ok(ZeroCopySliceMut {
            length,
            data: Self::data_ptr_from_bytes(bytes)?,
            _marker: PhantomData,
        })
    }

    pub fn from_bytes_at_multiple(
        num_slices: usize,
        bytes: &mut [u8],
        offset: &mut usize,
    ) -> Result<Vec<ZeroCopySliceMut<L, T>>, ZeroCopyError> {
        let mut slices = Vec::with_capacity(num_slices);
        for _ in 0..num_slices {
            let slice = Self::from_bytes_at(bytes, offset)?;
            slices.push(slice);
        }
        Ok(slices)
    }

    pub fn try_into_array<const N: usize>(&self) -> Result<[T; N], ZeroCopyError> {
        if self.len() != N {
            return Err(ZeroCopyError::ArraySize(N, self.len()));
        }
        Ok(std::array::from_fn(|i| *self.get(i).unwrap()))
    }

    #[inline]
    pub fn metadata_size() -> usize {
        let mut size = size_of::<L>();
        add_padding::<L, T>(&mut size);
        size
    }

    #[inline]
    pub fn data_size(length: L) -> usize {
        length.try_into().unwrap() * size_of::<T>()
    }

    #[inline]
    pub fn required_size_for_capacity(capacity: usize) -> usize {
        Self::metadata_size() + Self::data_size(capacity.try_into().unwrap())
    }
}

impl<L: Length, T: Copy> ZeroCopySliceMut<L, T> {
    pub fn copy_from_slice(&mut self, slice: &[T]) {
        let len = slice.len();
        if len != self.len() {
            panic!(
                "Slice length mismatch. Expected: {}, Found: {}",
                self.len(),
                len
            );
        }

        unsafe {
            std::ptr::copy_nonoverlapping(slice.as_ptr(), self.data_as_mut_ptr(), len);
        }
    }
}

impl<L: Length, T: Copy> ZeroCopySliceMut<L, T> {
    #[inline]
    pub fn len(&self) -> usize {
        (*self.length).try_into().unwrap()
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
        unsafe { slice::from_raw_parts(*self.data as *const T, self.len()) }
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(*self.data, self.len()) }
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

    #[inline]
    pub fn to_vec(&self) -> Vec<T> {
        self.as_slice().to_vec()
    }
}

impl<L: Length, T: Copy> IndexMut<usize> for ZeroCopySliceMut<L, T> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.as_mut_slice()[index]
    }
}

impl<L: Length, T: Copy> Index<usize> for ZeroCopySliceMut<L, T> {
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl<'a, L: Length, T: Copy> IntoIterator for &'a ZeroCopySliceMut<L, T> {
    type Item = &'a T;
    type IntoIter = slice::Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, L: Length, T: Copy> IntoIterator for &'a mut ZeroCopySliceMut<L, T> {
    type Item = &'a mut T;
    type IntoIter = slice::IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<'b, L: Length, T: Copy> ZeroCopySliceMut<L, T> {
    #[inline]
    pub fn iter(&'b self) -> slice::Iter<'b, T> {
        self.as_slice().iter()
    }

    #[inline]
    pub fn iter_mut(&'b mut self) -> slice::IterMut<'b, T> {
        self.as_mut_slice().iter_mut()
    }
}

impl<L: Length, T: Copy + PartialEq> PartialEq for ZeroCopySliceMut<L, T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice() && self.len() == other.len()
    }
}

impl<L: Length, T: Copy + fmt::Debug> fmt::Debug for ZeroCopySliceMut<L, T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_vec())
    }
}
