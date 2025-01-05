use core::fmt;
use std::{
    fmt::{Debug, Formatter},
    marker::PhantomData,
    mem::{size_of, ManuallyDrop},
    ops::{Deref, DerefMut},
};

use crate::{check_alignment, check_size, errors::ZeroCopyError};

pub struct WrappedPointerMut<T>
where
    T: Copy,
{
    ptr: ManuallyDrop<*mut T>,
    _marker: PhantomData<T>,
}

impl<T> WrappedPointerMut<T>
where
    T: Copy,
{
    pub fn new(value: T, data: &mut [u8]) -> Result<Self, ZeroCopyError> {
        check_size::<T>(data)?;
        let ptr = data.as_mut_ptr() as *mut T;
        check_alignment(ptr)?;
        unsafe {
            ptr.write(value);
        }

        unsafe { Self::from_raw_parts(ptr) }
    }

    pub fn new_at(value: T, data: &mut [u8], offset: &mut usize) -> Result<Self, ZeroCopyError> {
        let new = Self::new(value, &mut data[*offset..])?;
        *offset += new.size();
        Ok(new)
    }

    /// Create a new `WrappedPointerMut` from a raw pointer.
    /// # Safety
    /// This function is unsafe because it creates a `WrappedPointerMut` from a raw pointer.
    /// The caller must ensure that the pointer is valid and properly aligned.
    pub unsafe fn from_raw_parts(ptr: *mut T) -> Result<Self, ZeroCopyError> {
        Ok(WrappedPointerMut {
            ptr: ManuallyDrop::new(ptr),
            _marker: PhantomData,
        })
    }

    pub fn size(&self) -> usize {
        size_of::<T>()
    }

    pub fn get(&self) -> &T {
        unsafe { &**self.ptr }
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut **self.ptr }
    }

    pub fn from_bytes(bytes: &mut [u8]) -> Result<Self, ZeroCopyError> {
        check_size::<T>(bytes)?;
        let ptr = bytes.as_mut_ptr() as *mut T;
        check_alignment(ptr)?;
        unsafe { Self::from_raw_parts(ptr) }
    }

    pub fn from_bytes_at(bytes: &mut [u8], offset: &mut usize) -> Result<Self, ZeroCopyError> {
        let new = Self::from_bytes(&mut bytes[*offset..])?;
        *offset += new.size();
        Ok(new)
    }

    pub fn from_bytes_with_discriminator(bytes: &mut [u8]) -> Result<Self, ZeroCopyError> {
        let mut offset = crate::DISCRIMINATOR_LEN;
        Self::from_bytes_at(bytes, &mut offset)
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        *self.ptr
    }

    pub fn as_ptr(&self) -> *const T {
        *self.ptr as *const T
    }
}

impl<T> PartialEq for WrappedPointerMut<T>
where
    T: Copy + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        *self.get() == *other.get()
    }
}

impl<T> Debug for WrappedPointerMut<T>
where
    T: Copy + Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.get())
    }
}

impl<T> Deref for WrappedPointerMut<T>
where
    T: Copy,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T> DerefMut for WrappedPointerMut<T>
where
    T: Copy,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

/// Test coverage:
/// 1. Test `WrappedPointerMut::new` success
/// 2. Test `WrappedPointerMut::new` with unaligned pointer
/// 3. Test `WrappedPointerMut::new` with insufficient space
/// 4. Test `WrappedPointerMut::new_at` success
/// 5. Test `WrappedPointerMut::new_at` with out of bounds
/// 6. Test `WrappedPointerMut::new_at` with insufficient memory
/// 7. Test `WrappedPointerMut::from_bytes` with success
/// 8. Test `WrappedPointerMut::from_bytes` with insufficient memory
/// 9. Test `WrappedPointerMut::from_bytes_at` with success
/// 10. Test `WrappedPointerMut::from_bytes_with_discriminator` with success
/// 11. Test `WrappedPointerMut::from_bytes_with_discriminator` with insufficient memory (out of bounds)
/// 11. Test `WrappedPointerMut::from_bytes_with_discriminator` with insufficient memory (value)
/// 12. Test `WrappedPointerMut::deref` success
/// 13. Test `WrappedPointerMut::size` success
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_rawmutpointer_new_success() {
        let mut buffer = [0u8; 16];
        let value = 42u32;

        let pointer = WrappedPointerMut::new(value, &mut buffer).unwrap();
        assert_eq!(*pointer.get(), value);
        assert_eq!(buffer[0..4], value.to_le_bytes());
        assert_eq!(buffer[4..16], [0u8; 12]);
    }

    #[test]
    fn test_rawmutpointer_new_unaligned() {
        let mut buffer = [0u8; 5];
        let value = 42u32;

        let result = WrappedPointerMut::new(value, &mut buffer[1..]);
        assert_eq!(result, Err(ZeroCopyError::UnalignedPointer));
    }

    #[test]
    fn test_rawmutpointer_new_insufficient_space() {
        let mut buffer = [0u8; 3];
        let value = 42u32;

        let result = WrappedPointerMut::new(value, &mut buffer);
        assert_eq!(
            result,
            Err(ZeroCopyError::InsufficientMemoryAllocated(3, 4))
        );
    }

    #[test]
    fn test_rawmutpointer_new_at_success() {
        let mut buffer = [0u8; 16];
        let mut offset = 4;
        let value = 42u32;

        let pointer = WrappedPointerMut::new_at(value, &mut buffer, &mut offset).unwrap();
        assert_eq!(*pointer.get(), value);
        assert_eq!(offset, 8); // Size of u32
        assert_eq!(buffer[0..4], [0u8; 4]);
        assert_eq!(buffer[4..8], value.to_le_bytes());
        assert_eq!(buffer[8..16], [0u8; 8]);
    }

    #[test]
    #[should_panic(expected = "out of range for slice of")]
    fn test_rawmutpointer_new_at_out_of_bounds() {
        let mut buffer = [0u8; 4];
        let mut offset = 5;
        let value = 42u32;

        WrappedPointerMut::new_at(value, &mut buffer, &mut offset).unwrap();
    }

    #[test]
    fn test_rawmutpointer_new_at_insufficient_memory() {
        let mut buffer = [0u8; 4];
        let mut offset = 4;
        let value = 42u32;

        let result = WrappedPointerMut::new_at(value, &mut buffer, &mut offset);
        assert_eq!(
            result,
            Err(ZeroCopyError::InsufficientMemoryAllocated(0, 4))
        );
    }

    #[test]
    fn test_rawmutpointer_from_bytes_success() {
        let mut buffer = [0u8; 4];
        let value = 42u32;

        // Write value to buffer
        unsafe { *(buffer.as_mut_ptr() as *mut u32) = value };

        let pointer: WrappedPointerMut<u32> = WrappedPointerMut::from_bytes(&mut buffer).unwrap();
        assert_eq!(*pointer.get(), value);
    }

    #[test]
    fn test_rawmutpointer_from_bytes_insufficient_memory() {
        let value = 42u32;
        let mut buffer = value.to_le_bytes();

        let result = WrappedPointerMut::<u32>::from_bytes(&mut buffer[0..2]);
        assert_eq!(
            result,
            Err(ZeroCopyError::InsufficientMemoryAllocated(2, 4))
        );
    }

    #[test]
    fn test_rawmutpointer_from_bytes_at_success() {
        let mut buffer = [0u8; 8];
        let value = 42u32;
        let mut offset = 4;
        // Write value to buffer
        unsafe { *(buffer[offset..].as_mut_ptr() as *mut u32) = value };

        let pointer: WrappedPointerMut<u32> =
            WrappedPointerMut::from_bytes_at(&mut buffer, &mut offset).unwrap();
        assert_eq!(*pointer.get(), value);
        assert_eq!(offset, 8);
    }

    #[test]
    fn test_rawmutpointer_from_bytes_with_discriminator_success() {
        let mut buffer = [0u8; 12];
        let value = 42u32;

        // Write discriminator and value
        buffer[..8].copy_from_slice(&1u64.to_le_bytes()); // Fake discriminator
        unsafe { *(buffer[8..].as_mut_ptr() as *mut u32) = value };

        let mut pointer =
            WrappedPointerMut::<u32>::from_bytes_with_discriminator(&mut buffer).unwrap();
        assert_eq!(*pointer.get(), value);
        assert_eq!(*pointer.get_mut(), value);
    }

    #[test]
    #[should_panic(expected = "out of range for slice of length")]
    fn test_rawmutpointer_from_bytes_with_discriminator_fail() {
        let mut buffer = [0u8; 7]; // Not enough space for discriminator
        let result = WrappedPointerMut::<u32>::from_bytes_with_discriminator(&mut buffer);
        assert_eq!(
            result,
            Err(ZeroCopyError::InsufficientMemoryAllocated(7, 8))
        );
    }

    #[test]
    fn test_rawmutpointer_from_bytes_with_discriminator_insufficient_memory() {
        let mut buffer = [0u8; 9];
        let result = WrappedPointerMut::<u32>::from_bytes_with_discriminator(&mut buffer);
        assert_eq!(
            result,
            Err(ZeroCopyError::InsufficientMemoryAllocated(1, 4))
        );
    }

    #[test]
    fn test_rawmutpointer_deref_success() {
        let mut buffer = [0u8; 8];
        let value = 42u32;

        let mut pointer = WrappedPointerMut::new(value, &mut buffer).unwrap();
        assert_eq!(*pointer, value);

        // Update value via mutable dereference
        *pointer = 84;
        assert_eq!(*pointer, 84);
    }

    #[test]
    fn test_size() {
        let pointer = WrappedPointerMut::<u32>::new(42, &mut [0u8; 4]).unwrap();
        assert_eq!(pointer.size(), size_of::<u32>());
    }
}
