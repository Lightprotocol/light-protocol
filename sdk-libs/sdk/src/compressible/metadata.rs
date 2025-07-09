use crate::error::LightSdkError;
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize as BorshDeserialize, AnchorSerialize as BorshSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::{to_byte_array::ToByteArray, DataHasher, Hasher, HasherError};
use solana_clock::Clock;
use solana_sysvar::Sysvar;

/// Metadata for compressible accounts that tracks when the account was last written
#[derive(Clone, Debug, Default, BorshSerialize, BorshDeserialize)]
pub struct CompressionMetadata {
    /// The slot when this account was last written/decompressed
    pub last_written_slot: u64,
}

impl CompressionMetadata {
    /// Creates new compression metadata with the current slot
    pub fn new() -> Result<Self, LightSdkError> {
        Ok(Self {
            last_written_slot: Clock::get()?.slot,
        })
    }

    /// Updates the last written slot to the current slot
    pub fn set_last_written_slot(&mut self) -> Result<(), LightSdkError> {
        self.last_written_slot = Clock::get()?.slot;
        Ok(())
    }

    /// Sets the last written slot to a specific value
    pub fn set_last_written_slot_value(&mut self, slot: u64) {
        self.last_written_slot = slot;
    }

    /// Gets the last written slot
    pub fn last_written_slot(&self) -> u64 {
        self.last_written_slot
    }

    /// Checks if the account can be compressed based on the delay
    pub fn can_compress(&self, compression_delay: u64) -> Result<bool, LightSdkError> {
        let current_slot = Clock::get()?.slot;
        Ok(current_slot >= self.last_written_slot + compression_delay)
    }

    /// Gets the number of slots remaining before compression is allowed
    pub fn slots_until_compressible(&self, compression_delay: u64) -> Result<u64, LightSdkError> {
        let current_slot = Clock::get()?.slot;
        Ok((self.last_written_slot + compression_delay).saturating_sub(current_slot))
    }
}

// Implement ToByteArray for CompressionMetadata
impl ToByteArray for CompressionMetadata {
    const NUM_FIELDS: usize = 1;
    const IS_PRIMITIVE: bool = false;

    fn to_byte_array(&self) -> Result<[u8; 32], HasherError> {
        self.last_written_slot.to_byte_array()
    }
}

// Implement DataHasher for CompressionMetadata
impl DataHasher for CompressionMetadata {
    fn hash<H: Hasher>(&self) -> Result<[u8; 32], HasherError> {
        self.to_byte_array()
    }
}
