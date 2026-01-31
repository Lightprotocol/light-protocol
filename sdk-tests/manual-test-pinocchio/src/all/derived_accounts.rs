//! Derived account types for the all module.
//! Uses different seeds than pda/account_loader modules but reuses the data types.

use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::{
    light_account_checks::{self, packed_accounts::ProgramPackedAccounts},
    LightAccount, LightAccountVariantTrait, LightSdkTypesError, PackedLightAccountVariantTrait,
};
use pinocchio::account_info::AccountInfo;

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
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct AllBorshSeeds {
    pub owner: [u8; 32],
}

/// Packed seeds with u8 indices instead of Pubkeys.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct PackedAllBorshSeeds {
    pub owner_idx: u8,
    pub bump: u8,
}

// ============================================================================
// AllBorsh Variant (combines AllBorshSeeds + MinimalRecord data)
// ============================================================================

/// Full variant combining AllBorsh seeds + MinimalRecord data.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct AllBorshVariant {
    pub seeds: AllBorshSeeds,
    pub data: MinimalRecord,
}

/// Packed variant for efficient serialization.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct PackedAllBorshVariant {
    pub seeds: PackedAllBorshSeeds,
    pub data: PackedMinimalRecord,
}

// ============================================================================
// LightAccountVariant Implementation for AllBorshVariant
// ============================================================================

impl LightAccountVariantTrait<3> for AllBorshVariant {
    const PROGRAM_ID: [u8; 32] = crate::ID;

    type Seeds = AllBorshSeeds;
    type Data = MinimalRecord;
    type Packed = PackedAllBorshVariant;

    fn data(&self) -> &Self::Data {
        &self.data
    }

    /// Get seed values as owned byte vectors for PDA derivation.
    /// Generated from: seeds = [b"all_borsh", params.owner.as_ref()]
    fn seed_vec(&self) -> Vec<Vec<u8>> {
        vec![ALL_BORSH_SEED.to_vec(), self.seeds.owner.to_vec()]
    }

    /// Get seed references with bump for CPI signing.
    /// Generated from: seeds = [b"all_borsh", params.owner.as_ref()]
    fn seed_refs_with_bump<'a>(&'a self, bump_storage: &'a [u8; 1]) -> [&'a [u8]; 3] {
        [ALL_BORSH_SEED, self.seeds.owner.as_ref(), bump_storage]
    }
}

// ============================================================================
// PackedLightAccountVariant Implementation for PackedAllBorshVariant
// ============================================================================

impl PackedLightAccountVariantTrait<3> for PackedAllBorshVariant {
    type Unpacked = AllBorshVariant;

    const ACCOUNT_TYPE: light_account_pinocchio::AccountType =
        <MinimalRecord as LightAccount>::ACCOUNT_TYPE;

    fn bump(&self) -> u8 {
        self.seeds.bump
    }

    fn unpack<AI: light_account_checks::AccountInfoTrait>(
        &self,
        accounts: &[AI],
    ) -> std::result::Result<Self::Unpacked, LightSdkTypesError> {
        let owner = accounts
            .get(self.seeds.owner_idx as usize)
            .ok_or(LightSdkTypesError::NotEnoughAccountKeys)?;

        // Build ProgramPackedAccounts for LightAccount::unpack
        let packed_accounts = ProgramPackedAccounts { accounts };
        let data = MinimalRecord::unpack(&self.data, &packed_accounts)
            .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;

        Ok(AllBorshVariant {
            seeds: AllBorshSeeds { owner: owner.key() },
            data,
        })
    }

    fn seed_refs_with_bump<'a, AI: light_account_checks::AccountInfoTrait>(
        &'a self,
        _accounts: &'a [AI],
        _bump_storage: &'a [u8; 1],
    ) -> std::result::Result<[&'a [u8]; 3], LightSdkTypesError> {
        Err(LightSdkTypesError::InvalidSeeds)
    }

    fn into_in_token_data(
        &self,
        _tree_info: &light_account_pinocchio::PackedStateTreeInfo,
        _output_queue_index: u8,
    ) -> std::result::Result<
        light_token_interface::instructions::transfer2::MultiInputTokenDataWithContext,
        LightSdkTypesError,
    > {
        Err(LightSdkTypesError::InvalidInstructionData)
    }

    fn into_in_tlv(
        &self,
    ) -> std::result::Result<
        Option<Vec<light_token_interface::instructions::extensions::ExtensionInstructionData>>,
        LightSdkTypesError,
    > {
        Ok(None)
    }
}

// ============================================================================
// AllZeroCopy Seeds (different seed prefix from ZeroCopyRecordSeeds)
// ============================================================================

/// Seeds for AllZeroCopy PDA.
/// Contains the dynamic seed values (static prefix "all_zero_copy" is in seed_refs).
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct AllZeroCopySeeds {
    pub owner: [u8; 32],
}

/// Packed seeds with u8 indices instead of Pubkeys.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct PackedAllZeroCopySeeds {
    pub owner_idx: u8,
    pub bump: u8,
}

// ============================================================================
// AllZeroCopy Variant (combines AllZeroCopySeeds + ZeroCopyRecord data)
// ============================================================================

/// Full variant combining AllZeroCopy seeds + ZeroCopyRecord data.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct AllZeroCopyVariant {
    pub seeds: AllZeroCopySeeds,
    pub data: ZeroCopyRecord,
}

/// Packed variant for efficient serialization.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct PackedAllZeroCopyVariant {
    pub seeds: PackedAllZeroCopySeeds,
    pub data: PackedZeroCopyRecord,
}

// ============================================================================
// LightAccountVariant Implementation for AllZeroCopyVariant
// ============================================================================

impl LightAccountVariantTrait<3> for AllZeroCopyVariant {
    const PROGRAM_ID: [u8; 32] = crate::ID;

    type Seeds = AllZeroCopySeeds;
    type Data = ZeroCopyRecord;
    type Packed = PackedAllZeroCopyVariant;

    fn data(&self) -> &Self::Data {
        &self.data
    }

    /// Get seed values as owned byte vectors for PDA derivation.
    /// Generated from: seeds = [b"all_zero_copy", params.owner.as_ref()]
    fn seed_vec(&self) -> Vec<Vec<u8>> {
        vec![ALL_ZERO_COPY_SEED.to_vec(), self.seeds.owner.to_vec()]
    }

    /// Get seed references with bump for CPI signing.
    /// Generated from: seeds = [b"all_zero_copy", params.owner.as_ref()]
    fn seed_refs_with_bump<'a>(&'a self, bump_storage: &'a [u8; 1]) -> [&'a [u8]; 3] {
        [ALL_ZERO_COPY_SEED, self.seeds.owner.as_ref(), bump_storage]
    }
}

// ============================================================================
// PackedLightAccountVariant Implementation for PackedAllZeroCopyVariant
// ============================================================================

impl PackedLightAccountVariantTrait<3> for PackedAllZeroCopyVariant {
    type Unpacked = AllZeroCopyVariant;

    const ACCOUNT_TYPE: light_account_pinocchio::AccountType =
        <ZeroCopyRecord as LightAccount>::ACCOUNT_TYPE;

    fn bump(&self) -> u8 {
        self.seeds.bump
    }

    fn unpack<AI: light_account_checks::AccountInfoTrait>(
        &self,
        accounts: &[AI],
    ) -> std::result::Result<Self::Unpacked, LightSdkTypesError> {
        let owner = accounts
            .get(self.seeds.owner_idx as usize)
            .ok_or(LightSdkTypesError::NotEnoughAccountKeys)?;

        // Build ProgramPackedAccounts for LightAccount::unpack
        let packed_accounts = ProgramPackedAccounts { accounts };
        let data = ZeroCopyRecord::unpack(&self.data, &packed_accounts)
            .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;

        Ok(AllZeroCopyVariant {
            seeds: AllZeroCopySeeds { owner: owner.key() },
            data,
        })
    }

    fn seed_refs_with_bump<'a, AI: light_account_checks::AccountInfoTrait>(
        &'a self,
        _accounts: &'a [AI],
        _bump_storage: &'a [u8; 1],
    ) -> std::result::Result<[&'a [u8]; 3], LightSdkTypesError> {
        Err(LightSdkTypesError::InvalidSeeds)
    }

    fn into_in_token_data(
        &self,
        _tree_info: &light_account_pinocchio::PackedStateTreeInfo,
        _output_queue_index: u8,
    ) -> std::result::Result<
        light_token_interface::instructions::transfer2::MultiInputTokenDataWithContext,
        LightSdkTypesError,
    > {
        Err(LightSdkTypesError::InvalidInstructionData)
    }

    fn into_in_tlv(
        &self,
    ) -> std::result::Result<
        Option<Vec<light_token_interface::instructions::extensions::ExtensionInstructionData>>,
        LightSdkTypesError,
    > {
        Ok(None)
    }
}

// ============================================================================
// IntoVariant Implementation for AllBorshSeeds (client-side API)
// ============================================================================

/// Implement IntoVariant to allow building variant from seeds + compressed data.
/// This enables the high-level `create_load_instructions` API.
#[cfg(not(target_os = "solana"))]
impl light_account_pinocchio::IntoVariant<AllBorshVariant> for AllBorshSeeds {
    fn into_variant(self, data: &[u8]) -> std::result::Result<AllBorshVariant, LightSdkTypesError> {
        // Deserialize the compressed data (which includes compression_info)
        let record: MinimalRecord =
            BorshDeserialize::deserialize(&mut &data[..]).map_err(|_| LightSdkTypesError::Borsh)?;

        // Verify the owner in data matches the seed
        if record.owner != self.owner {
            return Err(LightSdkTypesError::InvalidSeeds);
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
impl light_account_pinocchio::Pack<solana_instruction::AccountMeta> for AllBorshVariant {
    type Packed = crate::derived_variants::PackedLightAccountVariant;

    fn pack(
        &self,
        accounts: &mut light_account_pinocchio::PackedAccounts,
    ) -> std::result::Result<Self::Packed, LightSdkTypesError> {
        use light_account_pinocchio::LightAccountVariantTrait;
        let (_, bump) = self.derive_pda::<AccountInfo>();
        let packed_data = self
            .data
            .pack(accounts)
            .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;
        Ok(
            crate::derived_variants::PackedLightAccountVariant::AllBorsh {
                seeds: PackedAllBorshSeeds {
                    owner_idx: accounts
                        .insert_or_get(solana_pubkey::Pubkey::from(self.seeds.owner)),
                    bump,
                },
                data: packed_data,
            },
        )
    }
}

// ============================================================================
// IntoVariant Implementation for AllZeroCopySeeds (client-side API)
// ============================================================================

/// Implement IntoVariant to allow building variant from seeds + compressed data.
/// This enables the high-level `create_load_instructions` API.
#[cfg(not(target_os = "solana"))]
impl light_account_pinocchio::IntoVariant<AllZeroCopyVariant> for AllZeroCopySeeds {
    fn into_variant(
        self,
        data: &[u8],
    ) -> std::result::Result<AllZeroCopyVariant, LightSdkTypesError> {
        // For ZeroCopy (Pod) accounts, data is the full Pod bytes including compression_info.
        // We deserialize using BorshDeserialize (which ZeroCopyRecord implements).
        let record: ZeroCopyRecord =
            BorshDeserialize::deserialize(&mut &data[..]).map_err(|_| LightSdkTypesError::Borsh)?;

        // Verify the owner in data matches the seed
        if record.owner != self.owner {
            return Err(LightSdkTypesError::InvalidSeeds);
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
impl light_account_pinocchio::Pack<solana_instruction::AccountMeta> for AllZeroCopyVariant {
    type Packed = crate::derived_variants::PackedLightAccountVariant;

    fn pack(
        &self,
        accounts: &mut light_account_pinocchio::PackedAccounts,
    ) -> std::result::Result<Self::Packed, LightSdkTypesError> {
        use light_account_pinocchio::LightAccountVariantTrait;
        let (_, bump) = self.derive_pda::<AccountInfo>();
        let packed_data = self
            .data
            .pack(accounts)
            .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;
        Ok(
            crate::derived_variants::PackedLightAccountVariant::AllZeroCopy {
                seeds: PackedAllZeroCopySeeds {
                    owner_idx: accounts
                        .insert_or_get(solana_pubkey::Pubkey::from(self.seeds.owner)),
                    bump,
                },
                data: packed_data,
            },
        )
    }
}
