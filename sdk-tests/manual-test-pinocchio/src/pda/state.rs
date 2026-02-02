//! State module for single-pda-test.

use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::{CompressionInfo, LightDiscriminator, LightHasherSha};

/// Minimal record struct for testing PDA creation.
#[derive(
    Default, Debug, Clone, BorshSerialize, BorshDeserialize, LightDiscriminator, LightHasherSha,
)]
#[repr(C)]
pub struct MinimalRecord {
    pub compression_info: CompressionInfo,
    pub owner: [u8; 32],
}

impl MinimalRecord {
    pub const INIT_SPACE: usize = core::mem::size_of::<CompressionInfo>() + 32;

    /// Get a mutable reference to a MinimalRecord from account data (after 8-byte discriminator).
    pub fn mut_from_account_data(data: &mut [u8]) -> &mut Self {
        let start = 8; // skip discriminator
        let end = start + Self::INIT_SPACE;
        // Safety: MinimalRecord is just bytes (CompressionInfo is 24 bytes + [u8;32])
        // We need to do manual byte access since it's Borsh-serialized
        unsafe { &mut *(data[start..end].as_mut_ptr() as *mut Self) }
    }
}
