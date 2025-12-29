use aligned_sized::aligned_sized;
use light_compressible::compression_info::CompressionInfo;
use light_zero_copy::{ZeroCopy, ZeroCopyMut};

use crate::{AnchorDeserialize, AnchorSerialize};

/// Compressible extension for ctoken accounts.
/// This extension contains compression configuration and timing data.
#[derive(
    Debug,
    Clone,
    Hash,
    Copy,
    PartialEq,
    Eq,
    AnchorSerialize,
    AnchorDeserialize,
    ZeroCopy,
    ZeroCopyMut,
)]
#[repr(C)]
#[aligned_sized]
pub struct CompressibleExtension {
    /// Option discriminator for decimals (0 = None, 1 = Some)
    pub decimals_option: u8,
    /// Token decimals (only valid when decimals_option == 1)
    pub decimals: u8,
    /// Whether this account is compression-only (cannot decompress)
    pub compression_only: bool,
    /// Compression configuration and timing data
    pub info: CompressionInfo,
}

impl CompressibleExtension {
    /// Returns the decimals if present
    pub fn decimals(&self) -> Option<u8> {
        if self.decimals_option == 1 {
            Some(self.decimals)
        } else {
            None
        }
    }

    /// Sets the decimals
    pub fn set_decimals(&mut self, decimals: Option<u8>) {
        match decimals {
            Some(d) => {
                self.decimals_option = 1;
                self.decimals = d;
            }
            None => {
                self.decimals_option = 0;
                self.decimals = 0;
            }
        }
    }
}

// Getters on zero-copy immutable view
impl ZCompressibleExtension<'_> {
    /// Returns the decimals if present
    #[inline(always)]
    pub fn decimals(&self) -> Option<u8> {
        if self.decimals_option == 1 {
            Some(self.decimals)
        } else {
            None
        }
    }
}

// Getters and setters on zero-copy mutable view
impl ZCompressibleExtensionMut<'_> {
    /// Returns the decimals if present
    #[inline(always)]
    pub fn decimals(&self) -> Option<u8> {
        if self.decimals_option == 1 {
            Some(self.decimals)
        } else {
            None
        }
    }

    /// Sets the decimals value
    #[inline(always)]
    pub fn set_decimals(&mut self, decimals: Option<u8>) {
        match decimals {
            Some(d) => {
                self.decimals_option = 1;
                self.decimals = d;
            }
            None => {
                self.decimals_option = 0;
                self.decimals = 0;
            }
        }
    }
}
