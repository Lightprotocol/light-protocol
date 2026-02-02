use anchor_lang::prelude::*;
use light_compressed_account::instruction_data::{
    cpi_context::CompressedCpiContext, with_account_info::InstructionDataInvokeCpiWithAccountInfo,
};
use light_sdk::{
    cpi::{v2::CpiAccounts, CpiAccountsConfig, InvokeLightSystemProgram},
    error::LightSdkError,
    instruction::PackedAddressTreeInfoExt,
    interface::{
        prepare_compressed_account_on_init, LightAccount, LightAccountVariantTrait, LightFinalize,
        LightPreInit, PackedLightAccountVariantTrait,
    },
    light_account_checks::{self, packed_accounts::ProgramPackedAccounts},
    sdk_types::CpiContextWriteAccounts,
};

use super::{
    accounts::{CreatePda, CreatePdaParams},
    derived_state::PackedMinimalRecord,
    state::MinimalRecord,
};

// ============================================================================
// Compile-time Size Validation (800-byte limit for compressed accounts)
// ============================================================================

const _: () = {
    // Use Anchor's Space trait (from #[derive(InitSpace)])
    const COMPRESSED_SIZE: usize = 8 + <MinimalRecord as anchor_lang::Space>::INIT_SPACE;
    assert!(
        COMPRESSED_SIZE <= 800,
        "Compressed account 'MinimalRecord' exceeds 800-byte compressible account size limit"
    );
};

// ============================================================================
// Manual LightPreInit Implementation
// ============================================================================

impl<'info> LightPreInit<AccountInfo<'info>, CreatePdaParams> for CreatePda<'info> {
    fn light_pre_init(
        &mut self,
        remaining_accounts: &[AccountInfo<'info>],
        params: &CreatePdaParams,
    ) -> std::result::Result<bool, light_sdk::interface::error::LightPdaError> {
        let mut inner = || -> std::result::Result<bool, LightSdkError> {
            use light_sdk::interface::{LightConfig, LightAccount};
            use solana_program::{clock::Clock, sysvar::Sysvar};
            use solana_program_error::ProgramError;

            // 1. Build CPI accounts (slice remaining_accounts at system_accounts_offset)
            let system_accounts_offset =
                params.create_accounts_proof.system_accounts_offset as usize;
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

            const NUM_LIGHT_PDAS: usize = 1;

            // 6. Set compression_info from config
            let light_config = LightConfig::load_checked(&self.compression_config, &crate::ID.to_bytes())
                .map_err(|_| LightSdkError::from(ProgramError::InvalidAccountData))?;
            let current_slot = Clock::get()
                .map_err(|_| LightSdkError::from(ProgramError::InvalidAccountData))?
                .slot;
            // Dynamic derived light pda specific. Only exists if NUM_LIGHT_PDAS > 0
            // =====================================================================
            {
                // Is first if the instruction creates 1 or more light mints in addition to 1 or more light pda accounts.
                let cpi_context = if WITH_CPI_CONTEXT {
                    CompressedCpiContext::first()
                } else {
                    CompressedCpiContext::default()
                };
                let mut new_address_params = Vec::with_capacity(NUM_LIGHT_PDAS);
                let mut account_infos = Vec::with_capacity(NUM_LIGHT_PDAS);
                // 3. Prepare compressed account using helper function
                // Dynamic code 0-N variants depending on the accounts struct
                // =====================================================================
                prepare_compressed_account_on_init(
                    &self.record.key().to_bytes(),
                    &address_tree_pubkey.to_bytes(),
                    address_tree_info,
                    output_tree_index,
                    current_account_index,
                    &crate::ID.to_bytes(),
                    &mut new_address_params,
                    &mut account_infos,
                )?;
                self.record.set_decompressed(&light_config, current_slot);
                // =====================================================================

                // current_account_index += 1;
                // For multiple accounts, repeat the pattern:
                // let prepared2 = prepare_compressed_account_on_init(..., current_account_index, ...)?;
                // current_account_index += 1;

                // 4. Build instruction data manually (no builder pattern)
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
                    // 5. Invoke Light System Program CPI
                    instruction_data
                        .invoke(cpi_accounts)
                        .map_err(LightSdkError::from)?;
                } else {
                    // For flows that combine light mints with light PDAs, write to CPI context first.
                    // The authority and cpi_context accounts must be provided in remaining_accounts.
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
            }
            // =====================================================================
            Ok(false) // No mints, so no CPI context write
        };
        inner().map_err(Into::into)
    }
}

// ============================================================================
// Manual LightFinalize Implementation (no-op for PDA-only flow)
// ============================================================================

impl<'info> LightFinalize<AccountInfo<'info>, CreatePdaParams> for CreatePda<'info> {
    fn light_finalize(
        &mut self,
        _remaining_accounts: &[AccountInfo<'info>],
        _params: &CreatePdaParams,
        _has_pre_init: bool,
    ) -> std::result::Result<(), light_sdk::interface::error::LightPdaError> {
        // No-op for PDA-only flow - compression CPI already executed in light_pre_init
        Ok(())
    }
}

// ============================================================================
// Seeds Structs
// Extracted from: seeds = [b"minimal_record", params.owner.as_ref()]
// ============================================================================

/// Seeds for MinimalRecord PDA.
/// Contains the dynamic seed values (static prefix "minimal_record" is in seed_refs).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct MinimalRecordSeeds {
    pub owner: Pubkey,
    pub nonce: u64,
}

/// Packed seeds with u8 indices instead of Pubkeys.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PackedMinimalRecordSeeds {
    pub owner_idx: u8,
    pub nonce_bytes: [u8; 8],
    pub bump: u8,
}

// ============================================================================
// Variant Structs
// ============================================================================

/// Full variant combining seeds + data.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct MinimalRecordVariant {
    pub seeds: MinimalRecordSeeds,
    pub data: MinimalRecord,
}

/// Packed variant for efficient serialization.
/// Contains packed seeds and data with u8 indices for Pubkey deduplication.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PackedMinimalRecordVariant {
    pub seeds: PackedMinimalRecordSeeds,
    pub data: PackedMinimalRecord,
}

// ============================================================================
// LightAccountVariant Implementation
// ============================================================================

impl LightAccountVariantTrait<4> for MinimalRecordVariant {
    const PROGRAM_ID: [u8; 32] = crate::ID.to_bytes();

    type Seeds = MinimalRecordSeeds;
    type Data = MinimalRecord;
    type Packed = PackedMinimalRecordVariant;

    fn data(&self) -> &Self::Data {
        &self.data
    }

    /// Get seed values as owned byte vectors for PDA derivation.
    /// Generated from: seeds = [b"minimal_record", params.owner.as_ref(), &params.nonce.to_le_bytes()]
    fn seed_vec(&self) -> Vec<Vec<u8>> {
        vec![
            b"minimal_record".to_vec(),
            self.seeds.owner.to_bytes().to_vec(),
            self.seeds.nonce.to_le_bytes().to_vec(),
        ]
    }

    /// Get seed references with bump for CPI signing.
    /// Note: For unpacked variants with computed bytes (like nonce.to_le_bytes()),
    /// we cannot return references to temporaries. Use the packed variant instead.
    fn seed_refs_with_bump<'a>(&'a self, _bump_storage: &'a [u8; 1]) -> [&'a [u8]; 4] {
        // The packed variant stores nonce_bytes as [u8; 8], so it can return references.
        // This unpacked variant computes nonce.to_le_bytes() which creates a temporary.
        panic!("Use PackedMinimalRecordVariant::seed_refs_with_bump instead")
    }
}

// ============================================================================
// PackedLightAccountVariant Implementation
// ============================================================================

impl PackedLightAccountVariantTrait<4> for PackedMinimalRecordVariant {
    type Unpacked = MinimalRecordVariant;

    const ACCOUNT_TYPE: light_sdk::interface::AccountType =
        <MinimalRecord as LightAccount>::ACCOUNT_TYPE;

    fn bump(&self) -> u8 {
        self.seeds.bump
    }

    fn unpack<AI: light_account_checks::AccountInfoTrait>(
        &self,
        accounts: &[AI],
    ) -> std::result::Result<Self::Unpacked, light_sdk::interface::error::LightPdaError> {
        let owner = accounts
            .get(self.seeds.owner_idx as usize)
            .ok_or(light_sdk::interface::error::LightPdaError::NotEnoughAccountKeys)?;

        // Build ProgramPackedAccounts for LightAccount::unpack
        let packed_accounts = ProgramPackedAccounts { accounts };
        let data = MinimalRecord::unpack(&self.data, &packed_accounts)
            .map_err(|_| light_sdk::interface::error::LightPdaError::InvalidInstructionData)?;

        Ok(MinimalRecordVariant {
            seeds: MinimalRecordSeeds {
                owner: Pubkey::from(owner.key()),
                nonce: u64::from_le_bytes(self.seeds.nonce_bytes),
            },
            data,
        })
    }

    fn seed_refs_with_bump<'a, AI: light_account_checks::AccountInfoTrait>(
        &'a self,
        _accounts: &'a [AI],
        _bump_storage: &'a [u8; 1],
    ) -> std::result::Result<[&'a [u8]; 4], light_sdk::interface::error::LightPdaError> {
        // PDA variants use seed_vec() in the decompression path, not seed_refs_with_bump.
        // Returning a reference to the account key requires a key_ref() method on
        // AccountInfoTrait, which is not yet available. Since this method is only
        // called for token account variants, PDA variants return an error.
        Err(light_sdk::interface::error::LightPdaError::InvalidSeeds)
    }

    fn into_in_token_data(
        &self,
        _tree_info: &light_sdk::instruction::PackedStateTreeInfo,
        _output_queue_index: u8,
    ) -> std::result::Result<light_token_interface::instructions::transfer2::MultiInputTokenDataWithContext, light_sdk::interface::error::LightPdaError> {
        Err(light_sdk::interface::error::LightPdaError::InvalidInstructionData)
    }

    fn into_in_tlv(
        &self,
    ) -> std::result::Result<Option<Vec<light_token_interface::instructions::extensions::ExtensionInstructionData>>, light_sdk::interface::error::LightPdaError> {
        Ok(None)
    }
}

// ============================================================================
// IntoVariant Implementation for Seeds (client-side API)
// ============================================================================

/// Implement IntoVariant to allow building variant from seeds + compressed data.
/// This enables the high-level `create_load_instructions` API.
#[cfg(not(target_os = "solana"))]
impl light_sdk::interface::IntoVariant<MinimalRecordVariant> for MinimalRecordSeeds {
    fn into_variant(
        self,
        data: &[u8],
    ) -> std::result::Result<MinimalRecordVariant, light_sdk::interface::error::LightPdaError> {
        // Deserialize the compressed data (which includes compression_info)
        let record: MinimalRecord = AnchorDeserialize::deserialize(&mut &data[..])
            .map_err(|_| light_sdk::interface::error::LightPdaError::Borsh)?;

        // Verify the owner in data matches the seed
        if record.owner != self.owner {
            return Err(light_sdk::interface::error::LightPdaError::InvalidSeeds);
        }

        Ok(MinimalRecordVariant {
            seeds: self,
            data: record,
        })
    }
}

// ============================================================================
// Pack Implementation for MinimalRecordVariant (client-side API)
// ============================================================================

/// Implement Pack trait to allow MinimalRecordVariant to be used with `create_load_instructions`.
/// Transforms the variant into PackedLightAccountVariant for efficient serialization.
#[cfg(not(target_os = "solana"))]
impl light_sdk::interface::Pack<solana_program::instruction::AccountMeta> for MinimalRecordVariant {
    type Packed = crate::derived_variants::PackedLightAccountVariant;

    fn pack(
        &self,
        accounts: &mut light_sdk::instruction::PackedAccounts,
    ) -> std::result::Result<Self::Packed, light_sdk::interface::error::LightPdaError> {
        use light_sdk::interface::LightAccountVariantTrait;
        let (_, bump) = self.derive_pda::<solana_account_info::AccountInfo>();
        let packed_data = self
            .data
            .pack(accounts)
            .map_err(|_| light_sdk::interface::error::LightPdaError::InvalidInstructionData)?;
        Ok(
            crate::derived_variants::PackedLightAccountVariant::MinimalRecord {
                seeds: PackedMinimalRecordSeeds {
                    owner_idx: accounts.insert_or_get(self.seeds.owner),
                    nonce_bytes: self.seeds.nonce.to_le_bytes(),
                    bump,
                },
                data: packed_data,
            },
        )
    }
}
