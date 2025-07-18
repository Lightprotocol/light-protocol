use crate::error::LightSdkError;
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize as BorshDeserialize, AnchorSerialize as BorshSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize, BorshSerialize};
use solana_clock::Clock;
use solana_sysvar::Sysvar;

/// Trait for accounts that contain CompressionInfo
pub trait HasCompressionInfo {
    fn compression_info(&self) -> &CompressionInfo;
    fn compression_info_mut(&mut self) -> &mut CompressionInfo;
}

/// Information for compressible accounts that tracks when the account was last written
#[derive(Clone, Debug, Default, BorshSerialize, BorshDeserialize)]
pub struct CompressionInfo {
    /// The slot when this account was last written/decompressed
    pub last_written_slot: u64,
    /// 0 not inited, 1 decompressed, 2 compressed
    pub state: CompressionState,
}

#[derive(Clone, Default, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum CompressionState {
    #[default]
    Uninitialized,
    Decompressed,
    Compressed,
}

impl CompressionInfo {
    /// Creates new compression info with the current slot
    pub fn new() -> Result<Self, LightSdkError> {
        Ok(Self {
            last_written_slot: Clock::get()?.slot,
            state: CompressionState::Decompressed,
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

    /// Set compressed
    pub fn set_compressed(&mut self) {
        self.state = CompressionState::Compressed;
    }

    /// Set decompressed
    pub fn set_decompressed(&mut self) {
        self.state = CompressionState::Decompressed;
    }

    /// Check if the account is compressed
    pub fn is_compressed(&self) -> bool {
        self.state == CompressionState::Compressed
    }
}

#[cfg(feature = "anchor")]
impl anchor_lang::Space for CompressionInfo {
    const INIT_SPACE: usize = 8 + 1; // u64 + enum (1 byte for discriminant)
}
