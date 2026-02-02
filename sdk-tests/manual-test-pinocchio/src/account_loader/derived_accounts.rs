//! Variant structs and trait implementations for ZeroCopyRecord.
//!
//! This follows the same pattern as MinimalRecord's derived_accounts.rs,
//! adapted for the AccountLoader (zero-copy) access pattern.

use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::{
    light_account_checks::{self, packed_accounts::ProgramPackedAccounts},
    prepare_compressed_account_on_init, CpiAccounts, CpiAccountsConfig,
    CpiContextWriteAccounts, InvokeLightSystemProgram, LightAccount, LightAccountVariantTrait,
    LightFinalize, LightPreInit, LightSdkTypesError, PackedAddressTreeInfoExt,
    PackedLightAccountVariantTrait,
};
use light_compressed_account::instruction_data::{
    cpi_context::CompressedCpiContext, with_account_info::InstructionDataInvokeCpiWithAccountInfo,
};
use pinocchio::account_info::AccountInfo;

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

impl LightPreInit<AccountInfo, CreateZeroCopyParams> for CreateZeroCopy<'_> {
    fn light_pre_init(
        &mut self,
        remaining_accounts: &[AccountInfo],
        params: &CreateZeroCopyParams,
    ) -> std::result::Result<bool, LightSdkTypesError> {
        let inner = || -> std::result::Result<bool, LightSdkTypesError> {
            use light_account_pinocchio::{LightAccount, LightConfig};
            use pinocchio::sysvars::{clock::Clock, Sysvar};

            // 1. Build CPI accounts (slice remaining_accounts at system_accounts_offset)
            let system_accounts_offset =
                params.create_accounts_proof.system_accounts_offset as usize;
            if remaining_accounts.len() < system_accounts_offset {
                return Err(LightSdkTypesError::FewerAccountsThanSystemAccounts);
            }
            let config = CpiAccountsConfig::new(crate::LIGHT_CPI_SIGNER);
            let cpi_accounts = CpiAccounts::new_with_config(
                self.fee_payer,
                &remaining_accounts[system_accounts_offset..],
                config,
            );

            // 2. Get address tree pubkey from packed tree info
            let address_tree_info = &params.create_accounts_proof.address_tree_info;
            let address_tree_pubkey = address_tree_info
                .get_tree_pubkey(&cpi_accounts)
                .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;
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
            let light_config =
                LightConfig::load_checked(self.compression_config, &crate::ID)
                    .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;
            let current_slot = Clock::get()
                .map_err(|_| LightSdkTypesError::InvalidInstructionData)?
                .slot;

            // 4. Prepare compressed account using helper function
            // Get the record's key from AccountInfo
            let record_key = *self.record.key();
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
            // For pinocchio, access data directly via try_borrow_mut_data() + bytemuck
            {
                let mut account_data = self
                    .record
                    .try_borrow_mut_data()
                    .map_err(|_| LightSdkTypesError::Borsh)?;
                let record_bytes =
                    &mut account_data[8..8 + core::mem::size_of::<ZeroCopyRecord>()];
                let record: &mut ZeroCopyRecord = bytemuck::from_bytes_mut(record_bytes);
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
                instruction_data.invoke(cpi_accounts)?;
            } else {
                // For flows that combine light mints with light PDAs, write to CPI context first.
                let cpi_context_accounts = CpiContextWriteAccounts {
                    fee_payer: cpi_accounts.fee_payer(),
                    authority: cpi_accounts.authority()?,
                    cpi_context: cpi_accounts.cpi_context()?,
                    cpi_signer: crate::LIGHT_CPI_SIGNER,
                };
                instruction_data
                    .invoke_write_to_cpi_context_first(cpi_context_accounts)?;
            }

            Ok(false) // No mints, so no CPI context write
        };
        inner()
    }
}

// ============================================================================
// Manual LightFinalize Implementation (no-op for PDA-only flow)
// ============================================================================

impl LightFinalize<AccountInfo, CreateZeroCopyParams> for CreateZeroCopy<'_> {
    fn light_finalize(
        &mut self,
        _remaining_accounts: &[AccountInfo],
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
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct ZeroCopyRecordSeeds {
    pub owner: [u8; 32],
    pub name: String,
}

/// Packed seeds with u8 indices instead of Pubkeys.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct PackedZeroCopyRecordSeeds {
    pub owner_idx: u8,
    pub name: String,
    pub bump: u8,
}

// ============================================================================
// Variant Structs
// ============================================================================

/// Full variant combining seeds + data.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct ZeroCopyRecordVariant {
    pub seeds: ZeroCopyRecordSeeds,
    pub data: ZeroCopyRecord,
}

/// Packed variant for efficient serialization.
/// Contains packed seeds and data with u8 indices for Pubkey deduplication.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct PackedZeroCopyRecordVariant {
    pub seeds: PackedZeroCopyRecordSeeds,
    pub data: PackedZeroCopyRecord,
}

// ============================================================================
// LightAccountVariant Implementation
// ============================================================================

impl LightAccountVariantTrait<4> for ZeroCopyRecordVariant {
    const PROGRAM_ID: [u8; 32] = crate::ID;

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
            self.seeds.owner.to_vec(),
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

        Ok(ZeroCopyRecordVariant {
            seeds: ZeroCopyRecordSeeds {
                owner: owner.key(),
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
// IntoVariant Implementation for Seeds (client-side API)
// ============================================================================

/// Implement IntoVariant to allow building variant from seeds + compressed data.
/// This enables the high-level `create_load_instructions` API.
#[cfg(not(target_os = "solana"))]
impl light_account_pinocchio::IntoVariant<ZeroCopyRecordVariant> for ZeroCopyRecordSeeds {
    fn into_variant(
        self,
        data: &[u8],
    ) -> std::result::Result<ZeroCopyRecordVariant, LightSdkTypesError>
    {
        // For ZeroCopy (Pod) accounts, data is the full Pod bytes including compression_info.
        // We deserialize using BorshDeserialize (which ZeroCopyRecord implements).
        let record: ZeroCopyRecord = BorshDeserialize::deserialize(&mut &data[..])
            .map_err(|_| LightSdkTypesError::Borsh)?;

        // Verify the owner in data matches the seed
        if record.owner != self.owner {
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
impl light_account_pinocchio::Pack<solana_instruction::AccountMeta>
    for ZeroCopyRecordVariant
{
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
            crate::derived_variants::PackedLightAccountVariant::ZeroCopyRecord {
                seeds: PackedZeroCopyRecordSeeds {
                    owner_idx: accounts.insert_or_get(solana_pubkey::Pubkey::from(self.seeds.owner)),
                    name: self.seeds.name.clone(),
                    bump,
                },
                data: packed_data,
            },
        )
    }
}
