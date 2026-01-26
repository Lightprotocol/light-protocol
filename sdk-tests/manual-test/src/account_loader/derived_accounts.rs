//! Variant structs and trait implementations for ZeroCopyRecord.
//!
//! This follows the same pattern as MinimalRecord's derived_accounts.rs,
//! adapted for the AccountLoader (zero-copy) access pattern.

use super::accounts::{CreateZeroCopy, CreateZeroCopyParams};
use super::derived_state::PackedZeroCopyRecord;
use super::state::ZeroCopyRecord;
use crate::sdk_functions::prepare_compressed_account_on_init;
use crate::traits::{LightAccount, LightAccountVariant, PackedLightAccountVariant};
use anchor_lang::prelude::*;
use light_compressed_account::instruction_data::{
    cpi_context::CompressedCpiContext, with_account_info::InstructionDataInvokeCpiWithAccountInfo,
};
use light_sdk::{
    cpi::{v2::CpiAccounts, CpiAccountsConfig, InvokeLightSystemProgram},
    error::LightSdkError,
    instruction::{PackedAccounts, PackedAddressTreeInfoExt},
    interface::{LightFinalize, LightPreInit},
    light_account_checks::packed_accounts::ProgramPackedAccounts,
    sdk_types::CpiContextWriteAccounts,
};
use solana_program_error::ProgramError;

// ============================================================================
// Manual LightPreInit Implementation
// ============================================================================

impl<'info> LightPreInit<'info, CreateZeroCopyParams> for CreateZeroCopy<'info> {
    fn light_pre_init(
        &mut self,
        remaining_accounts: &[AccountInfo<'info>],
        params: &CreateZeroCopyParams,
    ) -> std::result::Result<bool, LightSdkError> {
        use crate::traits::LightAccount;
        use light_sdk::interface::config::LightConfig;
        use solana_program::clock::Clock;
        use solana_program::sysvar::Sysvar;
        use solana_program_error::ProgramError;

        // 1. Build CPI accounts (slice remaining_accounts at system_accounts_offset)
        let system_accounts_offset = params.create_accounts_proof.system_accounts_offset as usize;
        if remaining_accounts.len() < system_accounts_offset {
            return Err(LightSdkError::FewerAccountsThanSystemAccounts);
        }
        let config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
        let cpi_accounts = CpiAccounts::new_with_config(
            &self.fee_payer,
            &remaining_accounts[system_accounts_offset..],
            config,
        );

        // 2. Get address tree pubkey from packed tree info
        let address_tree_info = &params.create_accounts_proof.address_tree_info;
        let address_tree_pubkey = address_tree_info
            .get_tree_pubkey(&cpi_accounts)
            .map_err(|_| LightSdkError::from(ProgramError::InvalidAccountData))?;
        let output_tree_index = params.create_accounts_proof.output_state_tree_index;
        let current_account_index: u8 = 0;
        // Is true if the instruction creates 1 or more light mints in addition to 1 or more light pda accounts.
        const WITH_CPI_CONTEXT: bool = false;
        // Is first if the instruction creates 1 or more light mints in addition to 1 or more light pda accounts.
        let cpi_context = if WITH_CPI_CONTEXT {
            CompressedCpiContext::first()
        } else {
            CompressedCpiContext::default()
        };
        const NUM_LIGHT_PDAS: usize = 1;
        let mut new_address_params = Vec::with_capacity(NUM_LIGHT_PDAS);
        let mut account_infos = Vec::with_capacity(NUM_LIGHT_PDAS);

        // 3. Load config and get current slot
        let light_config = LightConfig::load_checked(&self.compression_config, &crate::ID)
            .map_err(|_| LightSdkError::from(ProgramError::InvalidAccountData))?;
        let current_slot = Clock::get()
            .map_err(|_| LightSdkError::from(ProgramError::InvalidAccountData))?
            .slot;

        // 4. Prepare compressed account using helper function
        // Get the record's key from AccountLoader
        let record_key = self.record.key();
        prepare_compressed_account_on_init(
            &record_key,
            &address_tree_pubkey,
            address_tree_info,
            output_tree_index,
            current_account_index,
            &crate::ID,
            &mut new_address_params,
            &mut account_infos,
        )?;

        // 5. Set compression_info on the zero-copy record
        // For AccountLoader, we need to use load_init() which was already called by Anchor
        {
            let mut record = self
                .record
                .load_init()
                .map_err(|_| LightSdkError::from(ProgramError::AccountBorrowFailed))?;
            record.set_decompressed(&light_config, current_slot);
        }

        // 6. Build instruction data manually (no builder pattern)
        let instruction_data = InstructionDataInvokeCpiWithAccountInfo {
            mode: 1, // V2 mode
            bump: crate::LIGHT_CPI_SIGNER.bump,
            invoking_program_id: crate::LIGHT_CPI_SIGNER.program_id.into(),
            compress_or_decompress_lamports: 0,
            is_compress: false,
            with_cpi_context: WITH_CPI_CONTEXT,
            with_transaction_hash: false,
            cpi_context,
            proof: params.create_accounts_proof.proof.0,
            new_address_params,
            account_infos,
            read_only_addresses: vec![],
            read_only_accounts: vec![],
        };
        if !WITH_CPI_CONTEXT {
            // 7. Invoke Light System Program CPI
            instruction_data
                .invoke(cpi_accounts)
                .map_err(LightSdkError::from)?;
        } else {
            // For flows that combine light mints with light PDAs, write to CPI context first.
            let cpi_context_accounts = CpiContextWriteAccounts {
                fee_payer: cpi_accounts.fee_payer(),
                authority: cpi_accounts.authority().map_err(LightSdkError::from)?,
                cpi_context: cpi_accounts.cpi_context().map_err(LightSdkError::from)?,
                cpi_signer: crate::LIGHT_CPI_SIGNER,
            };
            instruction_data
                .invoke_write_to_cpi_context_first(cpi_context_accounts)
                .map_err(LightSdkError::from)?;
        }

        Ok(false) // No mints, so no CPI context write
    }
}

// ============================================================================
// Manual LightFinalize Implementation (no-op for PDA-only flow)
// ============================================================================

impl<'info> LightFinalize<'info, CreateZeroCopyParams> for CreateZeroCopy<'info> {
    fn light_finalize(
        &mut self,
        _remaining_accounts: &[AccountInfo<'info>],
        _params: &CreateZeroCopyParams,
        _has_pre_init: bool,
    ) -> std::result::Result<(), LightSdkError> {
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

impl LightAccountVariant<4> for ZeroCopyRecordVariant {
    type Seeds = ZeroCopyRecordSeeds;
    type Data = ZeroCopyRecord;
    type Packed = PackedZeroCopyRecordVariant;

    fn seeds(&self) -> &Self::Seeds {
        &self.seeds
    }

    fn data(&self) -> &Self::Data {
        &self.data
    }

    fn data_mut(&mut self) -> &mut Self::Data {
        &mut self.data
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

    fn pack(&self, accounts: &mut PackedAccounts, program_id: &Pubkey) -> Result<Self::Packed> {
        let (_, bump) = self.derive_pda(program_id);
        let packed_data = self
            .data
            .pack(accounts)
            .map_err(|_| anchor_lang::error::ErrorCode::InvalidProgramId)?;
        Ok(PackedZeroCopyRecordVariant {
            seeds: PackedZeroCopyRecordSeeds {
                owner_idx: accounts.insert_or_get(self.seeds.owner),
                name: self.seeds.name.clone(),
                bump,
            },
            data: packed_data,
        })
    }
}

// ============================================================================
// PackedLightAccountVariant Implementation
// ============================================================================

impl PackedLightAccountVariant<4> for PackedZeroCopyRecordVariant {
    type Unpacked = ZeroCopyRecordVariant;

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

        Ok(ZeroCopyRecordVariant {
            seeds: ZeroCopyRecordSeeds {
                owner: *owner.key,
                name: self.seeds.name.clone(),
            },
            data,
        })
    }

    fn seed_refs_with_bump<'a>(
        &'a self,
        accounts: &'a [AccountInfo],
        bump_storage: &'a [u8; 1],
    ) -> std::result::Result<[&'a [u8]; 4], ProgramError> {
        let owner = accounts
            .get(self.seeds.owner_idx as usize)
            .ok_or(ProgramError::InvalidAccountData)?;
        Ok([
            b"zero_copy",
            owner.key.as_ref(),
            self.seeds.name.as_bytes(),
            bump_storage,
        ])
    }
}

// ============================================================================
// IntoVariant Implementation for Seeds (client-side API)
// ============================================================================

/// Implement IntoVariant to allow building variant from seeds + compressed data.
/// This enables the high-level `create_load_instructions` API.
#[cfg(not(target_os = "solana"))]
impl light_sdk::interface::IntoVariant<ZeroCopyRecordVariant> for ZeroCopyRecordSeeds {
    fn into_variant(
        self,
        data: &[u8],
    ) -> std::result::Result<ZeroCopyRecordVariant, anchor_lang::error::Error> {
        // For ZeroCopy (Pod) accounts, data is the full Pod bytes including compression_info.
        // We deserialize using AnchorDeserialize (which ZeroCopyRecord implements).
        let record: ZeroCopyRecord = AnchorDeserialize::deserialize(&mut &data[..])
            .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize)?;

        // Verify the owner in data matches the seed
        if Pubkey::new_from_array(record.owner) != self.owner {
            return Err(anchor_lang::error::ErrorCode::ConstraintSeeds.into());
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
/// Transforms the variant into PackedProgramAccountVariant for efficient serialization.
#[cfg(not(target_os = "solana"))]
impl light_sdk::compressible::Pack for ZeroCopyRecordVariant {
    type Packed = crate::derived_variants::PackedProgramAccountVariant;

    fn pack(
        &self,
        accounts: &mut PackedAccounts,
    ) -> std::result::Result<Self::Packed, ProgramError> {
        // Use the LightAccountVariant::pack method to get PackedZeroCopyRecordVariant
        let packed = <Self as LightAccountVariant<4>>::pack(self, accounts, &crate::ID)
            .map_err(|_| ProgramError::InvalidAccountData)?;

        Ok(crate::derived_variants::PackedProgramAccountVariant::ZeroCopyRecord(packed))
    }
}
