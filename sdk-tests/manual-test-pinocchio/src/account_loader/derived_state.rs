//! LightAccount implementation for ZeroCopyRecord.
//!
//! This follows the same pattern as MinimalRecord's derived_state.rs,
//! but for a Pod/zero-copy account type.

use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::{
    light_account_checks::{
        packed_accounts::ProgramPackedAccounts, AccountInfoTrait, AccountMetaTrait,
    },
    AccountType, CompressionInfo, HasCompressionInfo, LightAccount, LightConfig,
    LightSdkTypesError,
};

use super::state::ZeroCopyRecord;

// ============================================================================
// PackedZeroCopyRecord (compression_info excluded per implementation_details.md)
// ============================================================================

/// Packed version of ZeroCopyRecord for efficient transmission.
/// compression_info is excluded - it's cut off during pack.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct PackedZeroCopyRecord {
    /// Index into remaining_accounts instead of full Pubkey
    pub owner: u8,
    /// Value field (transmitted as-is)
    pub value: u64,
}

// ============================================================================
// LightAccount Implementation for ZeroCopyRecord
// ============================================================================

impl LightAccount for ZeroCopyRecord {
    const ACCOUNT_TYPE: AccountType = AccountType::PdaZeroCopy;

    type Packed = PackedZeroCopyRecord;

    // CompressionInfo (24) + owner (32) + value (8) = 64 bytes
    const INIT_SPACE: usize = core::mem::size_of::<Self>();

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
        accounts: &mut light_account_pinocchio::interface::instruction::PackedAccounts<AM>,
    ) -> std::result::Result<Self::Packed, LightSdkTypesError> {
        // compression_info excluded from packed struct (same as Borsh accounts)
        Ok(PackedZeroCopyRecord {
            owner: accounts.insert_or_get(AM::pubkey_from_bytes(self.owner)),
            value: self.value,
        })
    }

    fn unpack<A: AccountInfoTrait>(
        packed: &Self::Packed,
        accounts: &ProgramPackedAccounts<A>,
    ) -> std::result::Result<Self, LightSdkTypesError> {
        // Use get_u8 with a descriptive name for better error messages
        let owner_account = accounts
            .get_u8(packed.owner, "ZeroCopyRecord: owner")
            .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;

        // Set compression_info to compressed() for hash verification at decompress
        // (Same pattern as Borsh accounts - canonical compressed state for hashing)
        // Note: key() returns [u8; 32] directly, no conversion needed
        Ok(ZeroCopyRecord {
            compression_info: CompressionInfo::compressed(),
            owner: owner_account.key(),
            value: packed.value,
        })
    }
}

impl HasCompressionInfo for ZeroCopyRecord {
    fn compression_info(&self) -> std::result::Result<&CompressionInfo, LightSdkTypesError> {
        Ok(&self.compression_info)
    }

    fn compression_info_mut(
        &mut self,
    ) -> std::result::Result<&mut CompressionInfo, LightSdkTypesError> {
        Ok(&mut self.compression_info)
    }

    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo> {
        panic!("compression_info_mut_opt not supported for LightAccount types (use compression_info_mut instead)")
    }

    fn set_compression_info_none(&mut self) -> std::result::Result<(), LightSdkTypesError> {
        self.compression_info = CompressionInfo::compressed();
        Ok(())
    }
}
