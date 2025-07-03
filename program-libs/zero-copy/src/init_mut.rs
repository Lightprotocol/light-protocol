use core::mem::size_of;
use std::vec::Vec;

use crate::errors::ZeroCopyError;
use crate::borsh_mut::DeserializeMut;

/// Trait for types that can be initialized in mutable byte slices with configuration
/// 
/// This trait provides a way to initialize structures in pre-allocated byte buffers
/// with specific configuration parameters that determine Vec lengths, Option states, etc.
pub trait ZeroCopyInitMut<'a>
where
    Self: Sized,
{
    /// Configuration type needed to initialize this type
    type Config;
    
    /// Output type - the mutable zero-copy view of this type
    type Output;
    
    /// Calculate the byte length needed for this type with the given configuration
    /// 
    /// This is essential for allocating the correct buffer size before calling new_zero_copy
    fn byte_len(config: &Self::Config) -> usize;
    
    /// Initialize this type in a mutable byte slice with the given configuration
    /// 
    /// Returns the initialized mutable view and remaining bytes
    fn new_zero_copy(
        bytes: &'a mut [u8], 
        config: Self::Config
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError>;
}

// Generic implementation for Option<T>
impl<'a, T> ZeroCopyInitMut<'a> for Option<T>
where
    T: ZeroCopyInitMut<'a>,
{
    type Config = (bool, T::Config); // (enabled, inner_config)
    type Output = Option<T::Output>;
    
    fn byte_len(config: &Self::Config) -> usize {
        let (enabled, inner_config) = config;
        if *enabled {
            // 1 byte for Some discriminant + inner type's byte_len
            1 + T::byte_len(inner_config)
        } else {
            // Just 1 byte for None discriminant
            1
        }
    }
    
    fn new_zero_copy(
        bytes: &'a mut [u8], 
        config: Self::Config
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        let (enabled, inner_config) = config;
        
        if enabled {
            bytes[0] = 1; // Some discriminant
            let (_, bytes) = bytes.split_at_mut(1);
            let (value, bytes) = T::new_zero_copy(bytes, inner_config)?;
            Ok((Some(value), bytes))
        } else {
            bytes[0] = 0; // None discriminant
            let (_, bytes) = bytes.split_at_mut(1);
            Ok((None, bytes))
        }
    }
}


// Implementation for primitive types (no configuration needed)
impl<'a> ZeroCopyInitMut<'a> for u64 {
    type Config = ();
    type Output = zerocopy::Ref<&'a mut [u8], zerocopy::little_endian::U64>;
    
    fn byte_len(_config: &Self::Config) -> usize {
        size_of::<zerocopy::little_endian::U64>()
    }
    
    fn new_zero_copy(
        bytes: &'a mut [u8], 
        _config: Self::Config
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        // Return U64 little-endian type for generated structs
        Ok(zerocopy::Ref::<&mut [u8], zerocopy::little_endian::U64>::from_prefix(bytes)?)
    }
}

impl<'a> ZeroCopyInitMut<'a> for u32 {
    type Config = ();
    type Output = zerocopy::Ref<&'a mut [u8], zerocopy::little_endian::U32>;
    
    fn byte_len(_config: &Self::Config) -> usize {
        size_of::<zerocopy::little_endian::U32>()
    }
    
    fn new_zero_copy(
        bytes: &'a mut [u8], 
        _config: Self::Config
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        // Return U32 little-endian type for generated structs
        Ok(zerocopy::Ref::<&mut [u8], zerocopy::little_endian::U32>::from_prefix(bytes)?)
    }
}

impl<'a> ZeroCopyInitMut<'a> for u16 {
    type Config = ();
    type Output = zerocopy::Ref<&'a mut [u8], zerocopy::little_endian::U16>;
    
    fn byte_len(_config: &Self::Config) -> usize {
        size_of::<zerocopy::little_endian::U16>()
    }
    
    fn new_zero_copy(
        bytes: &'a mut [u8], 
        _config: Self::Config
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        // Return U16 little-endian type for generated structs
        Ok(zerocopy::Ref::<&mut [u8], zerocopy::little_endian::U16>::from_prefix(bytes)?)
    }
}

impl<'a> ZeroCopyInitMut<'a> for u8 {
    type Config = ();
    type Output = <Self as crate::borsh_mut::DeserializeMut<'a>>::Output;
    
    fn byte_len(_config: &Self::Config) -> usize {
        size_of::<u8>()
    }
    
    fn new_zero_copy(
        bytes: &'a mut [u8], 
        _config: Self::Config
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        // Use the DeserializeMut trait to create the proper output
        Self::zero_copy_at_mut(bytes)
    }
}

impl<'a> ZeroCopyInitMut<'a> for bool {
    type Config = ();
    type Output = <u8 as crate::borsh_mut::DeserializeMut<'a>>::Output;
    
    fn byte_len(_config: &Self::Config) -> usize {
        size_of::<u8>()  // bool is serialized as u8
    }
    
    fn new_zero_copy(
        bytes: &'a mut [u8], 
        _config: Self::Config
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        // Treat bool as u8
        u8::zero_copy_at_mut(bytes)
    }
}

// Implementation for fixed-size arrays  
impl<'a, T: Copy + Default + zerocopy::KnownLayout + zerocopy::Immutable + zerocopy::FromBytes, const N: usize> ZeroCopyInitMut<'a> for [T; N] {
    type Config = ();
    type Output = <Self as crate::borsh_mut::DeserializeMut<'a>>::Output;
    
    fn byte_len(_config: &Self::Config) -> usize {
        size_of::<Self>()
    }
    
    fn new_zero_copy(
        bytes: &'a mut [u8], 
        _config: Self::Config
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        // Use the DeserializeMut trait to create the proper output
        Self::zero_copy_at_mut(bytes)
    }
}

// Implementation for zerocopy little-endian types
impl<'a> ZeroCopyInitMut<'a> for zerocopy::little_endian::U16 {
    type Config = ();
    type Output = zerocopy::Ref<&'a mut [u8], zerocopy::little_endian::U16>;
    
    fn byte_len(_config: &Self::Config) -> usize {
        size_of::<zerocopy::little_endian::U16>()
    }
    
    fn new_zero_copy(
        bytes: &'a mut [u8], 
        _config: Self::Config
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        Ok(zerocopy::Ref::<&mut [u8], zerocopy::little_endian::U16>::from_prefix(bytes)?)
    }
}

impl<'a> ZeroCopyInitMut<'a> for zerocopy::little_endian::U32 {
    type Config = ();
    type Output = zerocopy::Ref<&'a mut [u8], zerocopy::little_endian::U32>;
    
    fn byte_len(_config: &Self::Config) -> usize {
        size_of::<zerocopy::little_endian::U32>()
    }
    
    fn new_zero_copy(
        bytes: &'a mut [u8], 
        _config: Self::Config
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        Ok(zerocopy::Ref::<&mut [u8], zerocopy::little_endian::U32>::from_prefix(bytes)?)
    }
}

impl<'a> ZeroCopyInitMut<'a> for zerocopy::little_endian::U64 {
    type Config = ();
    type Output = zerocopy::Ref<&'a mut [u8], zerocopy::little_endian::U64>;
    
    fn byte_len(_config: &Self::Config) -> usize {
        size_of::<zerocopy::little_endian::U64>()
    }
    
    fn new_zero_copy(
        bytes: &'a mut [u8], 
        _config: Self::Config
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        Ok(zerocopy::Ref::<&mut [u8], zerocopy::little_endian::U64>::from_prefix(bytes)?)
    }
}

// Implementation for Vec<T>
impl<'a, T: ZeroCopyInitMut<'a>> ZeroCopyInitMut<'a> for Vec<T> {
    type Config = Vec<T::Config>; // Vector of configs for each item
    type Output = Vec<T::Output>;
    
    fn byte_len(config: &Self::Config) -> usize {
        // 4 bytes for length prefix + sum of byte_len for each element config
        4 + config.iter().map(|config| T::byte_len(config)).sum::<usize>()
    }
    
    fn new_zero_copy(
        bytes: &'a mut [u8], 
        configs: Self::Config
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        use zerocopy::{Ref, little_endian::U32};
        
        // Write length as U32
        let len = configs.len() as u32;
        let (mut len_ref, mut bytes) = Ref::<&mut [u8], U32>::from_prefix(bytes)?;
        *len_ref = U32::new(len);
        
        // Initialize each item with its config
        let mut items = Vec::with_capacity(configs.len());
        for config in configs {
            let (item, remaining_bytes) = T::new_zero_copy(bytes, config)?;
            bytes = remaining_bytes;
            items.push(item);
        }
        
        Ok((items, bytes))
    }
}