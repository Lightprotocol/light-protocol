//! LightAccount implementations for state types.
//!
//! Provides PackedXxx types and LightAccount/HasCompressionInfo trait impls.

use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::{
    light_account_checks::{packed_accounts::ProgramPackedAccounts, AccountInfoTrait},
    AccountType, CompressionInfo, HasCompressionInfo, LightAccount, LightConfig,
    LightSdkTypesError,
};

use crate::state::{MinimalRecord, ZeroCopyRecord};

// ============================================================================
// PackedMinimalRecord
// ============================================================================

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct PackedMinimalRecord {
    pub owner: u8,
}

// ============================================================================
// LightAccount for MinimalRecord
// ============================================================================

impl LightAccount for MinimalRecord {
    const ACCOUNT_TYPE: AccountType = AccountType::Pda;

    type Packed = PackedMinimalRecord;

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
    fn pack<AM: light_account_pinocchio::AccountMetaTrait>(
        &self,
        accounts: &mut light_account_pinocchio::interface::instruction::PackedAccounts<AM>,
    ) -> std::result::Result<Self::Packed, LightSdkTypesError> {
        Ok(PackedMinimalRecord {
            owner: accounts.insert_or_get(AM::pubkey_from_bytes(self.owner)),
        })
    }

    fn unpack<A: AccountInfoTrait>(
        packed: &Self::Packed,
        accounts: &ProgramPackedAccounts<A>,
    ) -> std::result::Result<Self, LightSdkTypesError> {
        let owner_account = accounts
            .get_u8(packed.owner, "MinimalRecord: owner")
            .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;
        Ok(MinimalRecord {
            compression_info: CompressionInfo::compressed(),
            owner: owner_account.key(),
        })
    }
}

impl HasCompressionInfo for MinimalRecord {
    fn compression_info(&self) -> std::result::Result<&CompressionInfo, LightSdkTypesError> {
        Ok(&self.compression_info)
    }

    fn compression_info_mut(
        &mut self,
    ) -> std::result::Result<&mut CompressionInfo, LightSdkTypesError> {
        Ok(&mut self.compression_info)
    }

    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo> {
        panic!("compression_info_mut_opt not supported for LightAccount types")
    }

    fn set_compression_info_none(&mut self) -> std::result::Result<(), LightSdkTypesError> {
        self.compression_info = CompressionInfo::compressed();
        Ok(())
    }
}

// ============================================================================
// PackedZeroCopyRecord
// ============================================================================

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct PackedZeroCopyRecord {
    pub owner: u8,
    pub counter: u64,
}

// ============================================================================
// LightAccount for ZeroCopyRecord
// ============================================================================

impl LightAccount for ZeroCopyRecord {
    const ACCOUNT_TYPE: AccountType = AccountType::PdaZeroCopy;

    type Packed = PackedZeroCopyRecord;

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
    fn pack<AM: light_account_pinocchio::AccountMetaTrait>(
        &self,
        accounts: &mut light_account_pinocchio::interface::instruction::PackedAccounts<AM>,
    ) -> std::result::Result<Self::Packed, LightSdkTypesError> {
        Ok(PackedZeroCopyRecord {
            owner: accounts.insert_or_get(AM::pubkey_from_bytes(self.owner)),
            counter: self.counter,
        })
    }

    fn unpack<A: AccountInfoTrait>(
        packed: &Self::Packed,
        accounts: &ProgramPackedAccounts<A>,
    ) -> std::result::Result<Self, LightSdkTypesError> {
        let owner_account = accounts
            .get_u8(packed.owner, "ZeroCopyRecord: owner")
            .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;
        Ok(ZeroCopyRecord {
            compression_info: CompressionInfo::compressed(),
            owner: owner_account.key(),
            counter: packed.counter,
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
        panic!("compression_info_mut_opt not supported for LightAccount types")
    }

    fn set_compression_info_none(&mut self) -> std::result::Result<(), LightSdkTypesError> {
        self.compression_info = CompressionInfo::compressed();
        Ok(())
    }
}
