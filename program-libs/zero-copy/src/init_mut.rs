use core::mem::size_of;
use std::vec::Vec;

use crate::errors::ZeroCopyError;

/// Trait for types that can be initialized in mutable byte slices with configuration
/// 
/// This trait provides a way to initialize structures in pre-allocated byte buffers
/// with specific configuration parameters that determine Vec lengths, Option states, etc.
pub trait ZeroCopyInitMut
where
    Self: Sized,
{
    /// Configuration type needed to initialize this type
    type Config;
    
    /// Initialize this type in a mutable byte slice with the given configuration
    /// 
    /// Returns the initialized instance and remaining bytes
    fn new_zero_copy<'a>(
        bytes: &'a mut [u8], 
        config: Self::Config
    ) -> Result<(Self, &'a mut [u8]), ZeroCopyError>;
}

// Implementation for Vec<T> where T implements ZeroCopyInitMut
impl<T: ZeroCopyInitMut> ZeroCopyInitMut for Vec<T> {
    type Config = Vec<T::Config>;
    
    fn new_zero_copy<'a>(
        bytes: &'a mut [u8], 
        config: Self::Config
    ) -> Result<(Self, &'a mut [u8]), ZeroCopyError> {
        // Write the length prefix
        if bytes.len() < size_of::<u32>() {
            return Err(ZeroCopyError::ArraySize(size_of::<u32>(), bytes.len()));
        }
        
        let (length_bytes, mut remaining_bytes) = bytes.split_at_mut(size_of::<u32>());
        let length = config.len() as u32;
        let length_bytes_array: [u8; 4] = length.to_le_bytes();
        length_bytes.copy_from_slice(&length_bytes_array);
        
        // Initialize each element with its config
        let mut vec = Vec::with_capacity(config.len());
        for element_config in config {
            let (element, new_remaining) = T::new_zero_copy(remaining_bytes, element_config)?;
            vec.push(element);
            remaining_bytes = new_remaining;
        }
        
        Ok((vec, remaining_bytes))
    }
}


// Implementation for Option<T> where T implements ZeroCopyInitMut
impl<T: ZeroCopyInitMut> ZeroCopyInitMut for Option<T> {
    type Config = Option<T::Config>;
    
    fn new_zero_copy<'a>(
        bytes: &'a mut [u8], 
        config: Self::Config
    ) -> Result<(Self, &'a mut [u8]), ZeroCopyError> {
        if bytes.len() < size_of::<u8>() {
            return Err(ZeroCopyError::ArraySize(size_of::<u8>(), bytes.len()));
        }
        
        let (discriminator_bytes, remaining_bytes) = bytes.split_at_mut(1);
        
        match config {
            None => {
                discriminator_bytes[0] = 0u8; // None discriminator
                Ok((None, remaining_bytes))
            },
            Some(inner_config) => {
                discriminator_bytes[0] = 1u8; // Some discriminator
                let (inner_value, final_remaining) = T::new_zero_copy(remaining_bytes, inner_config)?;
                Ok((Some(inner_value), final_remaining))
            }
        }
    }
}


// Implementation for primitive types (no configuration needed)
impl ZeroCopyInitMut for u64 {
    type Config = ();
    
    fn new_zero_copy<'a>(
        bytes: &'a mut [u8], 
        _config: Self::Config
    ) -> Result<(Self, &'a mut [u8]), ZeroCopyError> {
        if bytes.len() < size_of::<u64>() {
            return Err(ZeroCopyError::ArraySize(size_of::<u64>(), bytes.len()));
        }
        
        let (value_bytes, remaining_bytes) = bytes.split_at_mut(size_of::<u64>());
        value_bytes.fill(0); // Initialize with zero
        Ok((0u64, remaining_bytes))
    }
}

impl ZeroCopyInitMut for u32 {
    type Config = ();
    
    fn new_zero_copy<'a>(
        bytes: &'a mut [u8], 
        _config: Self::Config
    ) -> Result<(Self, &'a mut [u8]), ZeroCopyError> {
        if bytes.len() < size_of::<u32>() {
            return Err(ZeroCopyError::ArraySize(size_of::<u32>(), bytes.len()));
        }
        
        let (value_bytes, remaining_bytes) = bytes.split_at_mut(size_of::<u32>());
        value_bytes.fill(0);
        Ok((0u32, remaining_bytes))
    }
}

impl ZeroCopyInitMut for u16 {
    type Config = ();
    
    fn new_zero_copy<'a>(
        bytes: &'a mut [u8], 
        _config: Self::Config
    ) -> Result<(Self, &'a mut [u8]), ZeroCopyError> {
        if bytes.len() < size_of::<u16>() {
            return Err(ZeroCopyError::ArraySize(size_of::<u16>(), bytes.len()));
        }
        
        let (value_bytes, remaining_bytes) = bytes.split_at_mut(size_of::<u16>());
        value_bytes.fill(0);
        Ok((0u16, remaining_bytes))
    }
}

impl ZeroCopyInitMut for u8 {
    type Config = ();
    
    fn new_zero_copy<'a>(
        bytes: &'a mut [u8], 
        _config: Self::Config
    ) -> Result<(Self, &'a mut [u8]), ZeroCopyError> {
        if bytes.len() < size_of::<u8>() {
            return Err(ZeroCopyError::ArraySize(size_of::<u8>(), bytes.len()));
        }
        
        let (value_bytes, remaining_bytes) = bytes.split_at_mut(size_of::<u8>());
        value_bytes[0] = 0; // Initialize with zero
        Ok((0u8, remaining_bytes))
    }
}

impl ZeroCopyInitMut for bool {
    type Config = ();
    
    fn new_zero_copy<'a>(
        bytes: &'a mut [u8], 
        _config: Self::Config
    ) -> Result<(Self, &'a mut [u8]), ZeroCopyError> {
        if bytes.len() < size_of::<u8>() {
            return Err(ZeroCopyError::ArraySize(size_of::<u8>(), bytes.len()));
        }
        
        let (value_bytes, remaining_bytes) = bytes.split_at_mut(size_of::<u8>());
        value_bytes[0] = 0; // Initialize as false
        Ok((false, remaining_bytes))
    }
}

// Implementation for fixed-size arrays  
impl<T: Copy + Default, const N: usize> ZeroCopyInitMut for [T; N] {
    type Config = ();
    
    fn new_zero_copy<'a>(
        bytes: &'a mut [u8], 
        _config: Self::Config
    ) -> Result<(Self, &'a mut [u8]), ZeroCopyError> {
        let total_size = size_of::<[T; N]>();
        
        if bytes.len() < total_size {
            return Err(ZeroCopyError::ArraySize(total_size, bytes.len()));
        }
        
        // For fixed arrays, we initialize with zeros
        let (array_bytes, remaining_bytes) = bytes.split_at_mut(total_size);
        array_bytes.fill(0); // Initialize with zeros
        
        // Create array with default values
        let default_value = T::default();
        Ok(([default_value; N], remaining_bytes))
    }
}