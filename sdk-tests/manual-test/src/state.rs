//! State module for single-pda-test.

use anchor_lang::prelude::*;
use light_sdk::{
    compressible::CompressionInfo,
    instruction::PackedAccounts,
    interface::LightConfig,
    light_account_checks::{packed_accounts::ProgramPackedAccounts, AccountInfoTrait},
    LightDiscriminator, LightHasherSha,
};
use solana_program_error::ProgramError;

use crate::{light_account::LightAccount, AccountType};

// ============================================================================
// PackedMinimalRecord (compression_info excluded per implementation_details.md)
// ============================================================================

/// Packed version of MinimalRecord for efficient transmission.
/// compression_info is excluded - it's cut off during pack.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PackedMinimalRecord {
    /// Index into remaining_accounts instead of full Pubkey
    pub owner: u8,
}

// ============================================================================
// MinimalRecord with derive macros
// ============================================================================

/// Minimal record struct for testing PDA creation.
/// Contains only compression_info and one field.
///
/// Note: #[account] already derives Clone, AnchorSerialize, AnchorDeserialize
#[derive(Default, Debug, InitSpace, LightDiscriminator, LightHasherSha)]
#[account]
pub struct MinimalRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,
}

// ============================================================================
// LightAccount Implementation for MinimalRecord
// ============================================================================

impl LightAccount for MinimalRecord {
    const ACCOUNT_TYPE: AccountType = AccountType::Pda;

    type Packed = PackedMinimalRecord;

    // CompressionInfo (24) + Pubkey (32) = 56 bytes
    const INIT_SPACE: usize = CompressionInfo::INIT_SPACE + 32;

    fn compression_info(&self) -> &CompressionInfo {
        &self.compression_info
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        &mut self.compression_info
    }

    fn set_decompressed(&mut self, config: &LightConfig, current_slot: u64) {
        self.compression_info = CompressionInfo::new_from_config(config, current_slot);
    }

    fn size(&self) -> usize {
        self.try_to_vec().map(|v| v.len()).unwrap_or(0)
    }

    fn pack(
        &self,
        accounts: &mut PackedAccounts,
    ) -> std::result::Result<Self::Packed, ProgramError> {
        // compression_info excluded from packed struct
        Ok(PackedMinimalRecord {
            owner: accounts.insert_or_get(self.owner),
        })
    }

    fn unpack<A: AccountInfoTrait>(
        packed: &Self::Packed,
        accounts: &ProgramPackedAccounts<A>,
    ) -> std::result::Result<Self, ProgramError> {
        // Use get_u8 with a descriptive name for better error messages
        let owner_account = accounts
            .get_u8(packed.owner, "MinimalRecord: owner")
            .map_err(|_| ProgramError::InvalidAccountData)?;

        // Set compression_info to compressed() for hash verification at decompress
        Ok(MinimalRecord {
            compression_info: CompressionInfo::compressed(),
            owner: Pubkey::from(owner_account.key()),
        })
    }
}
