//! State module for single-account-loader-test.
//!
//! Defines a Pod (zero-copy) account struct for testing AccountLoader with Light Protocol.

use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use light_sdk::interface::CompressionInfo; // SDK version (24 bytes, Pod-compatible)
use light_sdk::{LightDiscriminator, LightHasherSha};
use light_sdk_macros::PodCompressionInfoField;

/// A zero-copy account using Pod serialization.
/// This account is used with AccountLoader and requires `#[light_account(init, zero_copy)]`.
///
/// Requirements for zero-copy accounts:
/// - `#[repr(C)]` for predictable memory layout
/// - `Pod + Zeroable` (bytemuck) for on-chain zero-copy access
/// - `BorshSerialize + BorshDeserialize` for hashing (same as Borsh accounts)
/// - `LightDiscriminator` for compress dispatch
/// - compression_info field for rent tracking
/// - All fields must be Pod-compatible (no Pubkey, use [u8; 32])
#[derive(
    Default,
    Debug,
    BorshSerialize,
    BorshDeserialize, // For hashing (same as Borsh accounts)
    LightDiscriminator,
    LightHasherSha,
    PodCompressionInfoField,
)]
#[account(zero_copy)]
#[repr(C)]
pub struct ZeroCopyRecord {
    /// Compression state - required for all rent-free accounts.
    /// Must be first for consistent packing with SDK CompressionInfo (24 bytes).
    pub compression_info: CompressionInfo,
    /// Owner of this record (stored as bytes for Pod compatibility).
    pub owner: [u8; 32],
    /// A simple counter value.
    pub counter: u64,
}

impl ZeroCopyRecord {
    /// Space required for this account (excluding Anchor discriminator).
    /// compression_info (24) + owner (32) + counter (8) = 64 bytes
    pub const INIT_SPACE: usize = core::mem::size_of::<Self>();
}

// ============================================================================
// PackedZeroCopyRecord (compression_info excluded per implementation_details.md)
// ============================================================================

/// Packed version of ZeroCopyRecord for efficient transmission.
/// compression_info is excluded - it's cut off during pack.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PackedZeroCopyRecord {
    /// Index into remaining_accounts instead of full Pubkey
    pub owner: u8,
    /// Counter field (transmitted as-is)
    pub counter: u64,
}

// ============================================================================
// LightAccount Implementation for ZeroCopyRecord
// ============================================================================

impl light_sdk::interface::LightAccount for ZeroCopyRecord {
    const ACCOUNT_TYPE: light_sdk::interface::AccountType = light_sdk::interface::AccountType::PdaZeroCopy;

    type Packed = PackedZeroCopyRecord;

    // CompressionInfo (24) + owner (32) + counter (8) = 64 bytes
    const INIT_SPACE: usize = core::mem::size_of::<Self>();

    fn compression_info(&self) -> &CompressionInfo {
        &self.compression_info
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        &mut self.compression_info
    }

    fn set_decompressed(&mut self, config: &light_sdk::interface::LightConfig, current_slot: u64) {
        self.compression_info = CompressionInfo::new_from_config(config, current_slot);
    }

    fn pack(
        &self,
        accounts: &mut light_sdk::instruction::PackedAccounts,
    ) -> std::result::Result<Self::Packed, solana_program_error::ProgramError> {
        // compression_info excluded from packed struct (same as Borsh accounts)
        Ok(PackedZeroCopyRecord {
            owner: accounts.insert_or_get(Pubkey::new_from_array(self.owner)),
            counter: self.counter,
        })
    }

    fn unpack<A: light_sdk::light_account_checks::AccountInfoTrait>(
        packed: &Self::Packed,
        accounts: &light_sdk::light_account_checks::packed_accounts::ProgramPackedAccounts<A>,
    ) -> std::result::Result<Self, solana_program_error::ProgramError> {
        // Use get_u8 with a descriptive name for better error messages
        let owner_account = accounts
            .get_u8(packed.owner, "ZeroCopyRecord: owner")
            .map_err(|_| solana_program_error::ProgramError::InvalidAccountData)?;

        // Set compression_info to compressed() for hash verification at decompress
        // (Same pattern as Borsh accounts - canonical compressed state for hashing)
        Ok(ZeroCopyRecord {
            compression_info: CompressionInfo::compressed(),
            owner: owner_account.key(),
            counter: packed.counter,
        })
    }
}
