use core::slice;
use std::{
    fmt,
    mem::size_of,
    ops::{Index, IndexMut},
};
use zerocopy::LayoutVerified;
use num_traits::{FromPrimitive, ToPrimitive, PrimInt};
use crate::{
    add_padding, errors::ZeroCopyError, wrapped_pointer_mut::WrappedPointerMut,
};

pub struct ZeroCopyVec<LEN, T>
where
    LEN: FromPrimitive + Copy,
    T: zerocopy::FromBytes + zerocopy::Unaligned,
{
    length: WrappedPointerMut<LEN>,
    data: LayoutVerified<&'static [u8], [T]>,
}

impl<LEN, T> ZeroCopyVec<LEN, T>
where
    LEN: FromPrimitive + ToPrimitive + PrimInt,
    T: zerocopy::FromBytes + zerocopy::Unaligned,
{
    pub fn new(capacity: LEN, data: &mut [u8]) -> Result<Self, ZeroCopyError> {
        Self::new_at(capacity, data, &mut 0)
    }

    pub fn new_at(
        capacity: LEN,
        data: &mut [u8],
        offset: &mut usize,
    ) -> Result<Self, ZeroCopyError> {
        let length = WrappedPointerMut::<LEN>::new_at(LEN::zero(), data, offset).unwrap();
        add_padding::<LEN, T>(offset);

        let slice_size = Self::data_size(capacity);
        let data_slice = &data[*offset..*offset + slice_size];
        let data = LayoutVerified::<&[u8], [T]>::new_slice(data_slice)
            .ok_or(ZeroCopyError::Alignment)?;

        *offset += slice_size;

        Ok(Self { length, data })
    }

    pub fn from_bytes_at(
        bytes: &mut [u8],
        offset: &mut usize,
    ) -> Result<Self, ZeroCopyError> {
        let length = WrappedPointerMut::<LEN>::from_bytes_at(bytes, offset)?;
        add_padding::<LEN, T>(offset);

        let slice_size = Self::data_size(length.get().to_usize().unwrap());
        let data_slice = &bytes[*offset..*offset + slice_size];
        let data = LayoutVerified::<&[u8], [T]>::new_slice(data_slice)
            .ok_or(ZeroCopyError::Alignment)?;

        *offset += slice_size;

        Ok(Self { length, data })
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.length.get().to_usize().unwrap()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.data.len()
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len() {
            return None;
        }
        Some(&self.data[index])
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        &self.data[..self.len()]
    }

    pub fn extend_from_slice(&mut self, slice: &[T]) -> Result<(), ZeroCopyError> {
        let len = self.len();
        let new_len = len + slice.len();
        if new_len > self.capacity() {
            return Err(ZeroCopyError::Full);
        }
        
        self.data.as_ptr()[len..new_len].copy_from_slice(slice);
        *self.length.get_mut() = LEN::from_usize(new_len).unwrap();
        Ok(())
    }

    pub fn required_size_for_capacity(capacity: usize) -> usize {
        size_of::<LEN>() + Self::data_size(LEN::from_usize(capacity).unwrap())
    }

    #[inline]
    fn data_size(capacity: LEN) -> usize {
        capacity.to_usize().unwrap() * size_of::<T>()
    }
}

impl<LEN, T> Index<usize> for ZeroCopyVec<LEN, T>
where
    LEN: FromPrimitive + ToPrimitive + PrimInt,
    T: zerocopy::FromBytes + zerocopy::Unaligned,
{
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl<LEN, T> IndexMut<usize> for ZeroCopyVec<LEN, T>
where
    LEN: FromPrimitive + ToPrimitive + PrimInt,
    T: zerocopy::FromBytes + zerocopy::Unaligned,
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let len = self.len();
        &mut self.data.as_mut_slice()[..len][index]
    }
}

impl<LEN, T> fmt::Debug for ZeroCopyVec<LEN, T>
where
    LEN: FromPrimitive + ToPrimitive + PrimInt,
    T: zerocopy::FromBytes + zerocopy::Unaligned + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zerocopy::{AsBytes, FromBytes, Unaligned};

    #[derive(Debug, FromBytes, AsBytes, Unaligned, PartialEq)]
    #[repr(C)]
    struct TestStruct {
        a: u32,
        aa: u32,
        b: u64,
        c: u16,
        cc: u16,
        ccc: u32,
        d: [u8; 32],
    }

    #[test]
    fn test_zero_copy_vec_with_test_struct() {
        let mut buffer = vec![0u8; 1024];
        let mut offset = 0;

        // Create a ZeroCopyVec<TestStruct>
        let mut vec = ZeroCopyVec::<usize, TestStruct>::new_at(10, &mut buffer, &mut offset).unwrap();

        // Prepare test data
        let test_struct = TestStruct {
            a: 1,
            aa: 2,
            b: 3,
            c: 4,
            cc: 5,
            ccc: 6,
            d: [7; 32],
        };

        vec.extend_from_slice(&[test_struct]).unwrap();

        assert_eq!(vec.len(), 1);
        assert_eq!(vec.capacity(), 10);

        // Verify the stored data
        let retrieved = vec.get(0).unwrap();
        assert_eq!(retrieved, &test_struct);

        println!("Stored TestStruct: {:?}", retrieved);
    }
}
