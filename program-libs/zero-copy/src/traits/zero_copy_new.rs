#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use core::mem::size_of;

use crate::{errors::ZeroCopyError, traits::ZeroCopyAtMut};

/// Trait for types that can be initialized in mutable byte slices with configuration
///
/// This trait provides a way to initialize structures in pre-allocated byte buffers
/// with specific configuration parameters that determine Vec lengths, Option states, etc.
pub trait ZeroCopyNew<'a>
where
    Self: Sized,
{
    /// Configuration type needed to initialize this type
    type ZeroCopyConfig;

    /// Output type - the mutable zero-copy view of this type
    type Output;

    /// Calculate the byte length needed for this type with the given configuration
    ///
    /// This is essential for allocating the correct buffer size before calling new_zero_copy
    /// Returns an error if the configuration would result in integer overflow
    fn byte_len(config: &Self::ZeroCopyConfig) -> Result<usize, ZeroCopyError>;

    /// Initialize this type in a mutable byte slice with the given configuration
    ///
    /// Returns the initialized mutable view and remaining bytes
    fn new_zero_copy(
        bytes: &'a mut [u8],
        config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError>;
}

// Generic implementation for Option<T>
impl<'a, T> ZeroCopyNew<'a> for Option<T>
where
    T: ZeroCopyNew<'a>,
{
    type ZeroCopyConfig = (bool, T::ZeroCopyConfig); // (enabled, inner_config)
    type Output = Option<T::Output>;

    fn byte_len(config: &Self::ZeroCopyConfig) -> Result<usize, ZeroCopyError> {
        let (enabled, inner_config) = config;
        if *enabled {
            // 1 byte for Some discriminant + inner type's byte_len
            let inner_len = T::byte_len(inner_config)?;
            inner_len.checked_add(1).ok_or(ZeroCopyError::Size)
        } else {
            // Just 1 byte for None discriminant
            Ok(1)
        }
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        if bytes.is_empty() {
            return Err(ZeroCopyError::ArraySize(1, bytes.len()));
        }

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
impl<'a> ZeroCopyNew<'a> for u64 {
    type ZeroCopyConfig = ();
    type Output = zerocopy::Ref<&'a mut [u8], zerocopy::little_endian::U64>;

    fn byte_len(_config: &Self::ZeroCopyConfig) -> Result<usize, ZeroCopyError> {
        Ok(size_of::<zerocopy::little_endian::U64>())
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        _config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        // Return U64 little-endian type for generated structs
        Ok(zerocopy::Ref::<&mut [u8], zerocopy::little_endian::U64>::from_prefix(bytes)?)
    }
}

impl<'a> ZeroCopyNew<'a> for u32 {
    type ZeroCopyConfig = ();
    type Output = zerocopy::Ref<&'a mut [u8], zerocopy::little_endian::U32>;

    fn byte_len(_config: &Self::ZeroCopyConfig) -> Result<usize, ZeroCopyError> {
        Ok(size_of::<zerocopy::little_endian::U32>())
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        _config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        // Return U32 little-endian type for generated structs
        Ok(zerocopy::Ref::<&mut [u8], zerocopy::little_endian::U32>::from_prefix(bytes)?)
    }
}

impl<'a> ZeroCopyNew<'a> for u16 {
    type ZeroCopyConfig = ();
    type Output = zerocopy::Ref<&'a mut [u8], zerocopy::little_endian::U16>;

    fn byte_len(_config: &Self::ZeroCopyConfig) -> Result<usize, ZeroCopyError> {
        Ok(size_of::<zerocopy::little_endian::U16>())
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        _config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        // Return U16 little-endian type for generated structs
        Ok(zerocopy::Ref::<&mut [u8], zerocopy::little_endian::U16>::from_prefix(bytes)?)
    }
}

impl<'a> ZeroCopyNew<'a> for u8 {
    type ZeroCopyConfig = ();
    type Output = <Self as crate::traits::ZeroCopyAtMut<'a>>::ZeroCopyAtMut;

    fn byte_len(_config: &Self::ZeroCopyConfig) -> Result<usize, ZeroCopyError> {
        Ok(size_of::<u8>())
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        _config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        // Use the ZeroCopyAtMut trait to create the proper output
        <Self as ZeroCopyAtMut<'a>>::zero_copy_at_mut(bytes)
    }
}

impl<'a> ZeroCopyNew<'a> for bool {
    type ZeroCopyConfig = ();
    type Output = <u8 as crate::traits::ZeroCopyAtMut<'a>>::ZeroCopyAtMut;

    fn byte_len(_config: &Self::ZeroCopyConfig) -> Result<usize, ZeroCopyError> {
        Ok(size_of::<u8>()) // bool is serialized as u8
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        _config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        // Treat bool as u8
        <u8 as ZeroCopyAtMut<'a>>::zero_copy_at_mut(bytes)
    }
}

// Implementation for fixed-size arrays
impl<
        'a,
        T: Copy + Default + zerocopy::KnownLayout + zerocopy::Immutable + zerocopy::FromBytes,
        const N: usize,
    > ZeroCopyNew<'a> for [T; N]
{
    type ZeroCopyConfig = ();
    type Output = <Self as crate::traits::ZeroCopyAtMut<'a>>::ZeroCopyAtMut;

    fn byte_len(_config: &Self::ZeroCopyConfig) -> Result<usize, ZeroCopyError> {
        Ok(size_of::<Self>())
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        _config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        // Use the ZeroCopyAtMut trait to create the proper output
        <Self as ZeroCopyAtMut<'a>>::zero_copy_at_mut(bytes)
    }
}

// Implementation for zerocopy little-endian types
impl<'a> ZeroCopyNew<'a> for zerocopy::little_endian::U16 {
    type ZeroCopyConfig = ();
    type Output = zerocopy::Ref<&'a mut [u8], zerocopy::little_endian::U16>;

    fn byte_len(_config: &Self::ZeroCopyConfig) -> Result<usize, ZeroCopyError> {
        Ok(size_of::<zerocopy::little_endian::U16>())
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        _config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        Ok(zerocopy::Ref::<&mut [u8], zerocopy::little_endian::U16>::from_prefix(bytes)?)
    }
}

impl<'a> ZeroCopyNew<'a> for zerocopy::little_endian::U32 {
    type ZeroCopyConfig = ();
    type Output = zerocopy::Ref<&'a mut [u8], zerocopy::little_endian::U32>;

    fn byte_len(_config: &Self::ZeroCopyConfig) -> Result<usize, ZeroCopyError> {
        Ok(size_of::<zerocopy::little_endian::U32>())
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        _config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        Ok(zerocopy::Ref::<&mut [u8], zerocopy::little_endian::U32>::from_prefix(bytes)?)
    }
}

impl<'a> ZeroCopyNew<'a> for zerocopy::little_endian::U64 {
    type ZeroCopyConfig = ();
    type Output = zerocopy::Ref<&'a mut [u8], zerocopy::little_endian::U64>;

    fn byte_len(_config: &Self::ZeroCopyConfig) -> Result<usize, ZeroCopyError> {
        Ok(size_of::<zerocopy::little_endian::U64>())
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        _config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        Ok(zerocopy::Ref::<&mut [u8], zerocopy::little_endian::U64>::from_prefix(bytes)?)
    }
}

// Implementation for Vec<T>
#[cfg(feature = "alloc")]
impl<'a, T: ZeroCopyNew<'a>> ZeroCopyNew<'a> for Vec<T> {
    type ZeroCopyConfig = Vec<T::ZeroCopyConfig>; // Vector of configs for each item
    type Output = Vec<T::Output>;

    fn byte_len(config: &Self::ZeroCopyConfig) -> Result<usize, ZeroCopyError> {
        // 4 bytes for length prefix + sum of byte_len for each element config
        let mut total = 4usize;
        for element_config in config {
            let element_len = T::byte_len(element_config)?;
            total = total.checked_add(element_len).ok_or(ZeroCopyError::Size)?;
        }
        Ok(total)
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        configs: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        use zerocopy::{little_endian::U32, Ref};

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
