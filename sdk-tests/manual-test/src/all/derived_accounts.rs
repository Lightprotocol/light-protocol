//! Derived account types for the all module.
//! Uses different seeds than pda/account_loader modules but reuses the data types.

use anchor_lang::prelude::*;
use light_sdk::{
    instruction::PackedAccounts,
    interface::{LightAccount, LightAccountVariantTrait, PackedLightAccountVariantTrait},
    light_account_checks::packed_accounts::ProgramPackedAccounts,
};
use solana_program_error::ProgramError;

use super::accounts::{ALL_BORSH_SEED, ALL_ZERO_COPY_SEED};
use crate::{
    account_loader::{PackedZeroCopyRecord, ZeroCopyRecord},
    pda::{MinimalRecord, PackedMinimalRecord},
};

// ============================================================================
// AllBorsh Seeds (different seed prefix from MinimalRecordSeeds)
// ============================================================================

/// Seeds for AllBorsh PDA.
/// Contains the dynamic seed values (static prefix "all_borsh" is in seed_refs).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct AllBorshSeeds {
    pub owner: Pubkey,
}

/// Packed seeds with u8 indices instead of Pubkeys.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PackedAllBorshSeeds {
    pub owner_idx: u8,
    pub bump: u8,
}

// ============================================================================
// AllBorsh Variant (combines AllBorshSeeds + MinimalRecord data)
// ============================================================================

/// Full variant combining AllBorsh seeds + MinimalRecord data.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct AllBorshVariant {
    pub seeds: AllBorshSeeds,
    pub data: MinimalRecord,
}

/// Packed variant for efficient serialization.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PackedAllBorshVariant {
    pub seeds: PackedAllBorshSeeds,
    pub data: PackedMinimalRecord,
}

// ============================================================================
// LightAccountVariant Implementation for AllBorshVariant
// ============================================================================

impl LightAccountVariantTrait<3> for AllBorshVariant {
    const PROGRAM_ID: Pubkey = crate::ID;

    type Seeds = AllBorshSeeds;
    type Data = MinimalRecord;
    type Packed = PackedAllBorshVariant;

    fn data(&self) -> &Self::Data {
        &self.data
    }

    /// Get seed values as owned byte vectors for PDA derivation.
    /// Generated from: seeds = [b"all_borsh", params.owner.as_ref()]
    fn seed_vec(&self) -> Vec<Vec<u8>> {
        vec![
            ALL_BORSH_SEED.to_vec(),
            self.seeds.owner.to_bytes().to_vec(),
        ]
    }

    /// Get seed references with bump for CPI signing.
    /// Generated from: seeds = [b"all_borsh", params.owner.as_ref()]
    fn seed_refs_with_bump<'a>(&'a self, bump_storage: &'a [u8; 1]) -> [&'a [u8]; 3] {
        [ALL_BORSH_SEED, self.seeds.owner.as_ref(), bump_storage]
    }

    fn pack(&self, accounts: &mut PackedAccounts) -> Result<Self::Packed> {
        let (_, bump) = self.derive_pda();
        let packed_data = self
            .data
            .pack(accounts)
            .map_err(|_| anchor_lang::error::ErrorCode::InvalidProgramId)?;
        Ok(PackedAllBorshVariant {
            seeds: PackedAllBorshSeeds {
                owner_idx: accounts.insert_or_get(self.seeds.owner),
                bump,
            },
            data: packed_data,
        })
    }
}

// ============================================================================
// PackedLightAccountVariant Implementation for PackedAllBorshVariant
// ============================================================================

impl PackedLightAccountVariantTrait<3> for PackedAllBorshVariant {
    type Unpacked = AllBorshVariant;

    const ACCOUNT_TYPE: light_sdk::interface::AccountType =
        <MinimalRecord as LightAccount>::ACCOUNT_TYPE;

    fn bump(&self) -> u8 {
        self.seeds.bump
    }

    fn unpack(&self, accounts: &[AccountInfo]) -> Result<Self::Unpacked> {
        let owner = accounts
            .get(self.seeds.owner_idx as usize)
            .ok_or(anchor_lang::error::ErrorCode::AccountNotEnoughKeys)?;

        // Build ProgramPackedAccounts for LightAccount::unpack
        let packed_accounts = ProgramPackedAccounts { accounts };
        let data = MinimalRecord::unpack(&self.data, &packed_accounts)
            .map_err(|_| anchor_lang::error::ErrorCode::InvalidProgramId)?;

        Ok(AllBorshVariant {
            seeds: AllBorshSeeds { owner: *owner.key },
            data,
        })
    }

    fn seed_refs_with_bump<'a>(
        &'a self,
        accounts: &'a [AccountInfo],
        bump_storage: &'a [u8; 1],
    ) -> std::result::Result<[&'a [u8]; 3], ProgramError> {
        let owner = accounts
            .get(self.seeds.owner_idx as usize)
            .ok_or(ProgramError::InvalidAccountData)?;
        Ok([ALL_BORSH_SEED, owner.key.as_ref(), bump_storage])
    }

    fn into_in_token_data(&self) -> anchor_lang::Result<light_token_interface::instructions::transfer2::MultiInputTokenDataWithContext> {
        Err(ProgramError::InvalidAccountData.into())
    }

    fn into_in_tlv(&self) -> anchor_lang::Result<Option<Vec<light_token_interface::instructions::extensions::ExtensionInstructionData>>> {
        Ok(None)
    }
}

// ============================================================================
// AllZeroCopy Seeds (different seed prefix from ZeroCopyRecordSeeds)
// ============================================================================

/// Seeds for AllZeroCopy PDA.
/// Contains the dynamic seed values (static prefix "all_zero_copy" is in seed_refs).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct AllZeroCopySeeds {
    pub owner: Pubkey,
}

/// Packed seeds with u8 indices instead of Pubkeys.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PackedAllZeroCopySeeds {
    pub owner_idx: u8,
    pub bump: u8,
}

// ============================================================================
// AllZeroCopy Variant (combines AllZeroCopySeeds + ZeroCopyRecord data)
// ============================================================================

/// Full variant combining AllZeroCopy seeds + ZeroCopyRecord data.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct AllZeroCopyVariant {
    pub seeds: AllZeroCopySeeds,
    pub data: ZeroCopyRecord,
}

/// Packed variant for efficient serialization.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PackedAllZeroCopyVariant {
    pub seeds: PackedAllZeroCopySeeds,
    pub data: PackedZeroCopyRecord,
}

// ============================================================================
// LightAccountVariant Implementation for AllZeroCopyVariant
// ============================================================================

impl LightAccountVariantTrait<3> for AllZeroCopyVariant {
    const PROGRAM_ID: Pubkey = crate::ID;

    type Seeds = AllZeroCopySeeds;
    type Data = ZeroCopyRecord;
    type Packed = PackedAllZeroCopyVariant;

    fn data(&self) -> &Self::Data {
        &self.data
    }

    /// Get seed values as owned byte vectors for PDA derivation.
    /// Generated from: seeds = [b"all_zero_copy", params.owner.as_ref()]
    fn seed_vec(&self) -> Vec<Vec<u8>> {
        vec![
            ALL_ZERO_COPY_SEED.to_vec(),
            self.seeds.owner.to_bytes().to_vec(),
        ]
    }

    /// Get seed references with bump for CPI signing.
    /// Generated from: seeds = [b"all_zero_copy", params.owner.as_ref()]
    fn seed_refs_with_bump<'a>(&'a self, bump_storage: &'a [u8; 1]) -> [&'a [u8]; 3] {
        [ALL_ZERO_COPY_SEED, self.seeds.owner.as_ref(), bump_storage]
    }

    fn pack(&self, accounts: &mut PackedAccounts) -> Result<Self::Packed> {
        let (_, bump) = self.derive_pda();
        let packed_data = self
            .data
            .pack(accounts)
            .map_err(|_| anchor_lang::error::ErrorCode::InvalidProgramId)?;
        Ok(PackedAllZeroCopyVariant {
            seeds: PackedAllZeroCopySeeds {
                owner_idx: accounts.insert_or_get(self.seeds.owner),
                bump,
            },
            data: packed_data,
        })
    }
}

// ============================================================================
// PackedLightAccountVariant Implementation for PackedAllZeroCopyVariant
// ============================================================================

impl PackedLightAccountVariantTrait<3> for PackedAllZeroCopyVariant {
    type Unpacked = AllZeroCopyVariant;

    const ACCOUNT_TYPE: light_sdk::interface::AccountType =
        <ZeroCopyRecord as LightAccount>::ACCOUNT_TYPE;

    fn bump(&self) -> u8 {
        self.seeds.bump
    }

    fn unpack(&self, accounts: &[AccountInfo]) -> Result<Self::Unpacked> {
        let owner = accounts
            .get(self.seeds.owner_idx as usize)
            .ok_or(anchor_lang::error::ErrorCode::AccountNotEnoughKeys)?;

        // Build ProgramPackedAccounts for LightAccount::unpack
        let packed_accounts = ProgramPackedAccounts { accounts };
        let data = ZeroCopyRecord::unpack(&self.data, &packed_accounts)
            .map_err(|_| anchor_lang::error::ErrorCode::InvalidProgramId)?;

        Ok(AllZeroCopyVariant {
            seeds: AllZeroCopySeeds { owner: *owner.key },
            data,
        })
    }

    fn seed_refs_with_bump<'a>(
        &'a self,
        accounts: &'a [AccountInfo],
        bump_storage: &'a [u8; 1],
    ) -> std::result::Result<[&'a [u8]; 3], ProgramError> {
        let owner = accounts
            .get(self.seeds.owner_idx as usize)
            .ok_or(ProgramError::InvalidAccountData)?;
        Ok([ALL_ZERO_COPY_SEED, owner.key.as_ref(), bump_storage])
    }

    fn into_in_token_data(&self) -> anchor_lang::Result<light_token_interface::instructions::transfer2::MultiInputTokenDataWithContext> {
        Err(ProgramError::InvalidAccountData.into())
    }

    fn into_in_tlv(&self) -> anchor_lang::Result<Option<Vec<light_token_interface::instructions::extensions::ExtensionInstructionData>>> {
        Ok(None)
    }
}

// ============================================================================
// IntoVariant Implementation for AllBorshSeeds (client-side API)
// ============================================================================

/// Implement IntoVariant to allow building variant from seeds + compressed data.
/// This enables the high-level `create_load_instructions` API.
#[cfg(not(target_os = "solana"))]
impl light_sdk::interface::IntoVariant<AllBorshVariant> for AllBorshSeeds {
    fn into_variant(
        self,
        data: &[u8],
    ) -> std::result::Result<AllBorshVariant, anchor_lang::error::Error> {
        // Deserialize the compressed data (which includes compression_info)
        let record: MinimalRecord = AnchorDeserialize::deserialize(&mut &data[..])
            .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize)?;

        // Verify the owner in data matches the seed
        if record.owner != self.owner {
            return Err(anchor_lang::error::ErrorCode::ConstraintSeeds.into());
        }

        Ok(AllBorshVariant {
            seeds: self,
            data: record,
        })
    }
}

// ============================================================================
// Pack Implementation for AllBorshVariant (client-side API)
// ============================================================================

/// Implement Pack trait to allow AllBorshVariant to be used with `create_load_instructions`.
/// Transforms the variant into PackedLightAccountVariant for efficient serialization.
#[cfg(not(target_os = "solana"))]
impl light_sdk::compressible::Pack for AllBorshVariant {
    type Packed = crate::derived_variants::PackedLightAccountVariant;

    fn pack(
        &self,
        accounts: &mut PackedAccounts,
    ) -> std::result::Result<Self::Packed, ProgramError> {
        // Use the LightAccountVariant::pack method to get PackedAllBorshVariant
        let packed = <Self as LightAccountVariantTrait<3>>::pack(self, accounts)
            .map_err(|_| ProgramError::InvalidAccountData)?;

        Ok(crate::derived_variants::PackedLightAccountVariant::AllBorsh(packed))
    }
}

// ============================================================================
// IntoVariant Implementation for AllZeroCopySeeds (client-side API)
// ============================================================================

/// Implement IntoVariant to allow building variant from seeds + compressed data.
/// This enables the high-level `create_load_instructions` API.
#[cfg(not(target_os = "solana"))]
impl light_sdk::interface::IntoVariant<AllZeroCopyVariant> for AllZeroCopySeeds {
    fn into_variant(
        self,
        data: &[u8],
    ) -> std::result::Result<AllZeroCopyVariant, anchor_lang::error::Error> {
        // For ZeroCopy (Pod) accounts, data is the full Pod bytes including compression_info.
        // We deserialize using AnchorDeserialize (which ZeroCopyRecord implements).
        let record: ZeroCopyRecord = AnchorDeserialize::deserialize(&mut &data[..])
            .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize)?;

        // Verify the owner in data matches the seed
        if Pubkey::new_from_array(record.owner) != self.owner {
            return Err(anchor_lang::error::ErrorCode::ConstraintSeeds.into());
        }

        Ok(AllZeroCopyVariant {
            seeds: self,
            data: record,
        })
    }
}

// ============================================================================
// Pack Implementation for AllZeroCopyVariant (client-side API)
// ============================================================================

/// Implement Pack trait to allow AllZeroCopyVariant to be used with `create_load_instructions`.
/// Transforms the variant into PackedLightAccountVariant for efficient serialization.
#[cfg(not(target_os = "solana"))]
impl light_sdk::compressible::Pack for AllZeroCopyVariant {
    type Packed = crate::derived_variants::PackedLightAccountVariant;

    fn pack(
        &self,
        accounts: &mut PackedAccounts,
    ) -> std::result::Result<Self::Packed, ProgramError> {
        // Use the LightAccountVariant::pack method to get PackedAllZeroCopyVariant
        let packed = <Self as LightAccountVariantTrait<3>>::pack(self, accounts)
            .map_err(|_| ProgramError::InvalidAccountData)?;

        Ok(crate::derived_variants::PackedLightAccountVariant::AllZeroCopy(packed))
    }
}
