use anchor_lang::prelude::*;
use light_sdk::{
    compressible::CompressionInfo,
    interface::{AccountType, HasCompressionInfo, LightAccount, LightConfig},
    light_account_checks::{packed_accounts::ProgramPackedAccounts, AccountInfoTrait, AccountMetaTrait},
};
use light_sdk::interface::error::LightPdaError;

use super::state::MinimalRecord;

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
// LightAccount Implementation for MinimalRecord
// ============================================================================

impl LightAccount for MinimalRecord {
    const ACCOUNT_TYPE: AccountType = AccountType::Pda;

    type Packed = PackedMinimalRecord;

    // CompressionInfo (24) + Pubkey (32) = 56 bytes
    const INIT_SPACE: usize = core::mem::size_of::<CompressionInfo>() + 32;

    fn compression_info(&self) -> &CompressionInfo {
        &self.compression_info
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        &mut self.compression_info
    }

    fn set_decompressed(&mut self, config: &LightConfig, current_slot: u64) {
        self.compression_info = CompressionInfo::new_from_config(config, current_slot);
    }

    #[cfg(not(target_os = "solana"))]
    fn pack<AM: AccountMetaTrait>(
        &self,
        accounts: &mut light_sdk::interface::instruction::PackedAccounts<AM>,
    ) -> std::result::Result<Self::Packed, LightPdaError> {
        // compression_info excluded from packed struct
        Ok(PackedMinimalRecord {
            owner: accounts.insert_or_get(AM::pubkey_from_bytes(self.owner.to_bytes())),
        })
    }

    fn unpack<A: AccountInfoTrait>(
        packed: &Self::Packed,
        accounts: &ProgramPackedAccounts<A>,
    ) -> std::result::Result<Self, LightPdaError> {
        // Use get_u8 with a descriptive name for better error messages
        let owner_account = accounts
            .get_u8(packed.owner, "MinimalRecord: owner")
            .map_err(|_| LightPdaError::InvalidInstructionData)?;

        // Set compression_info to compressed() for hash verification at decompress
        Ok(MinimalRecord {
            compression_info: CompressionInfo::compressed(),
            owner: Pubkey::from(owner_account.key()),
        })
    }
}

impl HasCompressionInfo for MinimalRecord {
    fn compression_info(&self) -> std::result::Result<&CompressionInfo, LightPdaError> {
        Ok(&self.compression_info)
    }

    fn compression_info_mut(&mut self) -> std::result::Result<&mut CompressionInfo, LightPdaError> {
        Ok(&mut self.compression_info)
    }

    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo> {
        panic!("compression_info_mut_opt not supported for LightAccount types (use compression_info_mut instead)")
    }

    fn set_compression_info_none(&mut self) -> std::result::Result<(), LightPdaError> {
        self.compression_info = CompressionInfo::compressed();
        Ok(())
    }
}
