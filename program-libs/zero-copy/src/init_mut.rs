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
    
    /// Initialize this type in a mutable byte slice with the given configuration
    /// 
    /// Returns the initialized mutable view and remaining bytes
    fn new_zero_copy(
        bytes: &'a mut [u8], 
        config: Self::Config
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError>;
}

// Note: Generic implementations for Vec<T> and Option<T> are complex due to trait bounds
// These will be handled by the derive macro for specific types that implement DeserializeMut


// Implementation for primitive types (no configuration needed)
impl<'a> ZeroCopyInitMut<'a> for u64 {
    type Config = ();
    type Output = <Self as crate::borsh_mut::DeserializeMut<'a>>::Output;
    
    fn new_zero_copy(
        bytes: &'a mut [u8], 
        _config: Self::Config
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        // Use the DeserializeMut trait to create the proper output
        Self::zero_copy_at_mut(bytes)
    }
}

impl<'a> ZeroCopyInitMut<'a> for u32 {
    type Config = ();
    type Output = <Self as crate::borsh_mut::DeserializeMut<'a>>::Output;
    
    fn new_zero_copy(
        bytes: &'a mut [u8], 
        _config: Self::Config
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        // Use the DeserializeMut trait to create the proper output
        Self::zero_copy_at_mut(bytes)
    }
}

impl<'a> ZeroCopyInitMut<'a> for u16 {
    type Config = ();
    type Output = <Self as crate::borsh_mut::DeserializeMut<'a>>::Output;
    
    fn new_zero_copy(
        bytes: &'a mut [u8], 
        _config: Self::Config
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        // Use the DeserializeMut trait to create the proper output
        Self::zero_copy_at_mut(bytes)
    }
}

impl<'a> ZeroCopyInitMut<'a> for u8 {
    type Config = ();
    type Output = <Self as crate::borsh_mut::DeserializeMut<'a>>::Output;
    
    fn new_zero_copy(
        bytes: &'a mut [u8], 
        _config: Self::Config
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        // Use the DeserializeMut trait to create the proper output
        Self::zero_copy_at_mut(bytes)
    }
}

// Note: bool doesn't implement DeserializeMut, so no ZeroCopyInitMut implementation

// Implementation for fixed-size arrays  
impl<'a, T: Copy + Default + zerocopy::KnownLayout + zerocopy::Immutable + zerocopy::FromBytes, const N: usize> ZeroCopyInitMut<'a> for [T; N] {
    type Config = ();
    type Output = <Self as crate::borsh_mut::DeserializeMut<'a>>::Output;
    
    fn new_zero_copy(
        bytes: &'a mut [u8], 
        _config: Self::Config
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        // Use the DeserializeMut trait to create the proper output
        Self::zero_copy_at_mut(bytes)
    }
}