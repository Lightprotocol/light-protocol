use light_zero_copy::{
    errors::ZeroCopyError,
    traits::{ZeroCopyAt, ZeroCopyAtMut},
    ZeroCopyNew,
};

use crate::{AnchorDeserialize, AnchorSerialize};

/// Marker extension indicating the account belongs to a pausable mint.
/// This is a zero-size marker (no data) that indicates the token account's
/// mint has the SPL Token 2022 Pausable extension.
///
/// When present, token operations must check the SPL mint's PausableConfig
/// to determine if the mint is paused before allowing transfers.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default, AnchorSerialize, AnchorDeserialize)]
#[repr(C)]
pub struct PausableAccountExtension;

/// Zero-copy reference for PausableAccountExtension.
/// Since this is a marker with no data, it's just a unit struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ZPausableAccountExtension;

/// Zero-copy mutable reference for PausableAccountExtension.
/// Since this is a marker with no data, it's just a unit struct.
#[derive(Debug, Clone, Copy)]
pub struct ZPausableAccountExtensionMut;

impl<'a> ZeroCopyAt<'a> for PausableAccountExtension {
    type ZeroCopyAt = ZPausableAccountExtension;

    fn zero_copy_at(data: &'a [u8]) -> Result<(Self::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
        // No data to read, just return the marker and all remaining bytes
        Ok((ZPausableAccountExtension, data))
    }
}

impl<'a> ZeroCopyAtMut<'a> for PausableAccountExtension {
    type ZeroCopyAtMut = ZPausableAccountExtensionMut;

    fn zero_copy_at_mut(
        data: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
        // No data to read, just return the marker and all remaining bytes
        Ok((ZPausableAccountExtensionMut, data))
    }
}

/// Config for PausableAccountExtension initialization.
/// Empty since there's no configuration needed for a marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PausableAccountExtensionConfig;

impl<'a> ZeroCopyNew<'a> for PausableAccountExtension {
    type ZeroCopyConfig = PausableAccountExtensionConfig;
    type Output = ZPausableAccountExtensionMut;

    fn byte_len(_config: &Self::ZeroCopyConfig) -> Result<usize, ZeroCopyError> {
        // Marker extension has 0 bytes of data
        Ok(0)
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        _config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        // No data to write, just return the marker and all remaining bytes
        Ok((ZPausableAccountExtensionMut, bytes))
    }
}
