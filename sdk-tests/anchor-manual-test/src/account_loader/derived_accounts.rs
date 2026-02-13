//! Variant structs and trait implementations for ZeroCopyRecord.
//!
//! This follows the same pattern as MinimalRecord's derived_accounts.rs,
//! adapted for the AccountLoader (zero-copy) access pattern.

use anchor_lang::prelude::*;
use light_account::{
    create_accounts,
    light_account_checks::{self, packed_accounts::ProgramPackedAccounts},
    LightAccount, LightAccountVariantTrait, LightFinalize, LightPreInit, LightSdkTypesError,
    PackedLightAccountVariantTrait, PdaInitParam, SharedAccounts,
};
use solana_account_info::AccountInfo;

use super::{
    accounts::{CreateZeroCopy, CreateZeroCopyParams},
    derived_state::PackedZeroCopyRecord,
    state::ZeroCopyRecord,
};

// ============================================================================
// Compile-time Size Validation (800-byte limit for compressed accounts)
// ============================================================================

const _: () = {
    const COMPRESSED_SIZE: usize = 8 + core::mem::size_of::<ZeroCopyRecord>();
    assert!(
        COMPRESSED_SIZE <= 800,
        "Compressed account 'ZeroCopyRecord' exceeds 800-byte compressible account size limit"
    );
};

// ============================================================================
// Manual LightPreInit Implementation
// ============================================================================

impl<'info> LightPreInit<AccountInfo<'info>, CreateZeroCopyParams> for CreateZeroCopy<'info> {
    fn light_pre_init(
        &mut self,
        remaining_accounts: &[AccountInfo<'info>],
        params: &CreateZeroCopyParams,
    ) -> std::result::Result<bool, LightSdkTypesError> {
        let record_info = self.record.to_account_info();

        create_accounts::<AccountInfo<'info>, 1, 0, 0, 0, _>(
            [PdaInitParam {
                account: &record_info,
            }],
            |light_config, current_slot| {
                let mut record = self
                    .record
                    .load_init()
                    .map_err(|_| LightSdkTypesError::Borsh)?;
                record.set_decompressed(light_config, current_slot);
                Ok(())
            },
            None,
            [],
            [],
            &SharedAccounts {
                fee_payer: &self.fee_payer.to_account_info(),
                cpi_signer: crate::LIGHT_CPI_SIGNER,
                proof: &params.create_accounts_proof,
                program_id: crate::LIGHT_CPI_SIGNER.program_id,
                compression_config: Some(&self.compression_config),
                compressible_config: None,
                rent_sponsor: None,
                cpi_authority: None,
                system_program: None,
            },
            remaining_accounts,
        )
    }
}

// ============================================================================
// Manual LightFinalize Implementation (no-op for PDA-only flow)
// ============================================================================

impl<'info> LightFinalize<AccountInfo<'info>, CreateZeroCopyParams> for CreateZeroCopy<'info> {
    fn light_finalize(
        &mut self,
        _remaining_accounts: &[AccountInfo<'info>],
        _params: &CreateZeroCopyParams,
        _has_pre_init: bool,
    ) -> std::result::Result<(), LightSdkTypesError> {
        // No-op for PDA-only flow - compression CPI already executed in light_pre_init
        Ok(())
    }
}

// ============================================================================
// Seeds Structs
// Extracted from: seeds = [b"zero_copy", params.owner.as_ref()]
// ============================================================================

/// Seeds for ZeroCopyRecord PDA.
/// Contains the dynamic seed values (static prefix "zero_copy" is in seed_refs).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ZeroCopyRecordSeeds {
    pub owner: Pubkey,
    pub name: String,
}

/// Packed seeds with u8 indices instead of Pubkeys.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PackedZeroCopyRecordSeeds {
    pub owner_idx: u8,
    pub name: String,
    pub bump: u8,
}

// ============================================================================
// Variant Structs
// ============================================================================

/// Full variant combining seeds + data.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ZeroCopyRecordVariant {
    pub seeds: ZeroCopyRecordSeeds,
    pub data: ZeroCopyRecord,
}

/// Packed variant for efficient serialization.
/// Contains packed seeds and data with u8 indices for Pubkey deduplication.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PackedZeroCopyRecordVariant {
    pub seeds: PackedZeroCopyRecordSeeds,
    pub data: PackedZeroCopyRecord,
}

// ============================================================================
// LightAccountVariant Implementation
// ============================================================================

impl LightAccountVariantTrait<4> for ZeroCopyRecordVariant {
    const PROGRAM_ID: [u8; 32] = crate::ID.to_bytes();

    type Seeds = ZeroCopyRecordSeeds;
    type Data = ZeroCopyRecord;
    type Packed = PackedZeroCopyRecordVariant;

    fn data(&self) -> &Self::Data {
        &self.data
    }

    /// Get seed values as owned byte vectors for PDA derivation.
    /// Generated from: seeds = [b"zero_copy", params.owner.as_ref(), params.name.as_bytes()]
    fn seed_vec(&self) -> Vec<Vec<u8>> {
        vec![
            b"zero_copy".to_vec(),
            self.seeds.owner.to_bytes().to_vec(),
            self.seeds.name.as_bytes().to_vec(),
        ]
    }

    /// Get seed references with bump for CPI signing.
    /// Generated from: seeds = [b"zero_copy", params.owner.as_ref(), params.name.as_bytes()]
    fn seed_refs_with_bump<'a>(&'a self, bump_storage: &'a [u8; 1]) -> [&'a [u8]; 4] {
        [
            b"zero_copy",
            self.seeds.owner.as_ref(),
            self.seeds.name.as_bytes(),
            bump_storage,
        ]
    }
}

// ============================================================================
// PackedLightAccountVariant Implementation
// ============================================================================

impl PackedLightAccountVariantTrait<4> for PackedZeroCopyRecordVariant {
    type Unpacked = ZeroCopyRecordVariant;

    const ACCOUNT_TYPE: light_account::AccountType = <ZeroCopyRecord as LightAccount>::ACCOUNT_TYPE;

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

        Ok(ZeroCopyRecordVariant {
            seeds: ZeroCopyRecordSeeds {
                owner: Pubkey::from(owner.key()),
                name: self.seeds.name.clone(),
            },
            data,
        })
    }

    fn seed_refs_with_bump<'a, AI: light_account_checks::AccountInfoTrait>(
        &'a self,
        _accounts: &'a [AI],
        _bump_storage: &'a [u8; 1],
    ) -> std::result::Result<[&'a [u8]; 4], LightSdkTypesError> {
        Err(LightSdkTypesError::InvalidSeeds)
    }

    fn into_in_token_data(
        &self,
        _tree_info: &light_account::PackedStateTreeInfo,
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
// IntoVariant Implementation for Seeds (client-side API)
// ============================================================================

/// Implement IntoVariant to allow building variant from seeds + compressed data.
/// This enables the high-level `create_load_instructions` API.
#[cfg(not(target_os = "solana"))]
impl light_account::IntoVariant<ZeroCopyRecordVariant> for ZeroCopyRecordSeeds {
    fn into_variant(
        self,
        data: &[u8],
    ) -> std::result::Result<ZeroCopyRecordVariant, LightSdkTypesError> {
        // For ZeroCopy (Pod) accounts, data is the full Pod bytes including compression_info.
        // We deserialize using AnchorDeserialize (which ZeroCopyRecord implements).
        let record: ZeroCopyRecord = AnchorDeserialize::deserialize(&mut &data[..])
            .map_err(|_| LightSdkTypesError::Borsh)?;

        // Verify the owner in data matches the seed
        if Pubkey::new_from_array(record.owner) != self.owner {
            return Err(LightSdkTypesError::InvalidSeeds);
        }

        Ok(ZeroCopyRecordVariant {
            seeds: self,
            data: record,
        })
    }
}

// ============================================================================
// Pack Implementation for ZeroCopyRecordVariant (client-side API)
// ============================================================================

/// Implement Pack trait to allow ZeroCopyRecordVariant to be used with `create_load_instructions`.
/// Transforms the variant into PackedLightAccountVariant for efficient serialization.
#[cfg(not(target_os = "solana"))]
impl light_account::Pack<solana_program::instruction::AccountMeta> for ZeroCopyRecordVariant {
    type Packed = crate::derived_variants::PackedLightAccountVariant;

    fn pack(
        &self,
        accounts: &mut light_account::PackedAccounts,
    ) -> std::result::Result<Self::Packed, LightSdkTypesError> {
        use light_account::LightAccountVariantTrait;
        let (_, bump) = self.derive_pda::<solana_account_info::AccountInfo>();
        let packed_data = self
            .data
            .pack(accounts)
            .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;
        Ok(
            crate::derived_variants::PackedLightAccountVariant::ZeroCopyRecord {
                seeds: PackedZeroCopyRecordSeeds {
                    owner_idx: accounts.insert_or_get(self.seeds.owner),
                    name: self.seeds.name.clone(),
                    bump,
                },
                data: packed_data,
            },
        )
    }
}
