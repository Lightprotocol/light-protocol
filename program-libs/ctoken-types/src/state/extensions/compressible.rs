use light_compressed_account::Pubkey;
use light_zero_copy::{ZeroCopy, ZeroCopyMut};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::{AnchorDeserialize, AnchorSerialize};

/// Compressible extension for token accounts
/// Contains timing data for compression/decompression and rent authority
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    AnchorSerialize,
    AnchorDeserialize,
    ZeroCopy,
    ZeroCopyMut,
    KnownLayout,
    Immutable,
    FromBytes,
    IntoBytes,
)]
#[repr(C)]
pub struct CompressibleExtension {
    /// The slot when this account was last written to
    pub last_written_slot: u64,
    /// Number of slots that must pass before compression is allowed
    pub slots_until_compression: u64,
    /// Authority that can close this account (in addition to owner)
    pub rent_authority: Pubkey,
    pub rent_recipient: Pubkey,
    // TODO: confirm that state variable is not necessary because we realloc memory to 0.
}

// Implement PdaTimingData trait for integration with light-protocol2's compression SDK
impl CompressibleExtension {
    pub fn last_written_slot(&self) -> u64 {
        self.last_written_slot
    }

    pub fn slots_until_compression(&self) -> u64 {
        self.slots_until_compression
    }

    pub fn set_last_written_slot(&mut self, slot: u64) {
        self.last_written_slot = slot;
    }
}

impl ZCompressibleExtension<'_> {
    /// Get the remaining slots until compression is allowed
    /// Returns 0 if compression is already allowed
    #[cfg(target_os = "solana")]
    pub fn remaining_slots(&self) -> Result<u64, crate::CTokenError> {
        let current_slot = {
            use pinocchio::sysvars::{clock::Clock, Sysvar};
            Clock::get()
                .map_err(|_| crate::CTokenError::SysvarAccessError)?
                .slot
        };
        let target_slot = self.last_written_slot + self.slots_until_compression;
        Ok(u64::from(target_slot).saturating_sub(current_slot))
    }

    // Note this might clash with rust tests. (Maybe I can use an env variable)
    /// Get the remaining slots until compression is allowed (non-Solana target)
    /// Returns 0 if compression is already allowed
    #[cfg(not(target_os = "solana"))]
    pub fn remaining_slots(&self, current_slot: u64) -> u64 {
        let target_slot = self.last_written_slot + self.slots_until_compression;
        u64::from(target_slot).saturating_sub(current_slot)
    }

    /// Check if the account is compressible (timing constraints have elapsed)
    #[cfg(target_os = "solana")]
    pub fn is_compressible(&self) -> Result<bool, crate::CTokenError> {
        Ok(self.remaining_slots()? == 0)
    }

    /// Check if the account is compressible (timing constraints have elapsed) - non-Solana target
    #[cfg(not(target_os = "solana"))]
    pub fn is_compressible(&self, current_slot: u64) -> bool {
        self.remaining_slots(current_slot) == 0
    }
}
