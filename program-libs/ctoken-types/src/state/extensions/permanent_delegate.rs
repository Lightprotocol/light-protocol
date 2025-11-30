use light_zero_copy::{
    errors::ZeroCopyError,
    traits::{ZeroCopyAt, ZeroCopyAtMut},
    ZeroCopyNew,
};

use crate::{AnchorDeserialize, AnchorSerialize};

/// Marker extension indicating the account belongs to a mint with permanent delegate.
/// This is a zero-size marker (no data) that indicates the token account's
/// mint has the SPL Token 2022 Permanent Delegate extension.
///
/// When present, token operations must check the SPL mint's PermanentDelegate
/// to determine the delegate authority before allowing transfers/burns.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default, AnchorSerialize, AnchorDeserialize)]
#[repr(C)]
pub struct PermanentDelegateAccountExtension;

/// Zero-copy reference for PermanentDelegateAccountExtension.
/// Since this is a marker with no data, it's just a unit struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ZPermanentDelegateAccountExtension;

/// Zero-copy mutable reference for PermanentDelegateAccountExtension.
/// Since this is a marker with no data, it's just a unit struct.
#[derive(Debug, Clone, Copy)]
pub struct ZPermanentDelegateAccountExtensionMut;

impl<'a> ZeroCopyAt<'a> for PermanentDelegateAccountExtension {
    type ZeroCopyAt = ZPermanentDelegateAccountExtension;

    fn zero_copy_at(data: &'a [u8]) -> Result<(Self::ZeroCopyAt, &'a [u8]), ZeroCopyError> {
        // No data to read, just return the marker and all remaining bytes
        Ok((ZPermanentDelegateAccountExtension, data))
    }
}

impl<'a> ZeroCopyAtMut<'a> for PermanentDelegateAccountExtension {
    type ZeroCopyAtMut = ZPermanentDelegateAccountExtensionMut;

    fn zero_copy_at_mut(
        data: &'a mut [u8],
    ) -> Result<(Self::ZeroCopyAtMut, &'a mut [u8]), ZeroCopyError> {
        // No data to read, just return the marker and all remaining bytes
        Ok((ZPermanentDelegateAccountExtensionMut, data))
    }
}

/// Config for PermanentDelegateAccountExtension initialization.
/// Empty since there's no configuration needed for a marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PermanentDelegateAccountExtensionConfig;

impl<'a> ZeroCopyNew<'a> for PermanentDelegateAccountExtension {
    type ZeroCopyConfig = PermanentDelegateAccountExtensionConfig;
    type Output = ZPermanentDelegateAccountExtensionMut;

    fn byte_len(_config: &Self::ZeroCopyConfig) -> Result<usize, ZeroCopyError> {
        // Marker extension has 0 bytes of data
        Ok(0)
    }

    fn new_zero_copy(
        bytes: &'a mut [u8],
        _config: Self::ZeroCopyConfig,
    ) -> Result<(Self::Output, &'a mut [u8]), ZeroCopyError> {
        // No data to write, just return the marker and all remaining bytes
        Ok((ZPermanentDelegateAccountExtensionMut, bytes))
    }
}
