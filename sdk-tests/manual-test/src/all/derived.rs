//! Derived code for create_all instruction.
//!
//! This implements LightPreInit/LightFinalize for creating all account types:
//! - 2 PDAs (Borsh + ZeroCopy) via `invoke_write_to_cpi_context_first()`
//! - 1 Mint via `invoke_create_mints()` with cpi_context_offset
//! - 1 Token Vault via `CreateTokenAccountCpi`
//! - 1 ATA via `CreateTokenAtaCpi`

use anchor_lang::prelude::*;
use light_account::{
    derive_associated_token_account, derive_mint_compressed_address, find_mint_address,
    invoke_create_mints, prepare_compressed_account_on_init, CpiAccounts, CpiAccountsConfig,
    CpiContextWriteAccounts, CreateMintsInfraAccounts, CreateMintsParams as SdkCreateMintsParams,
    CreateTokenAccountCpi, CreateTokenAtaCpi, InvokeLightSystemProgram, LightAccount,
    LightFinalize, LightPreInit, LightSdkTypesError, PackedAddressTreeInfoExt, SingleMintParams,
    DEFAULT_RENT_PAYMENT, DEFAULT_WRITE_TOP_UP,
};
use light_compressed_account::instruction_data::{
    cpi_context::CompressedCpiContext, with_account_info::InstructionDataInvokeCpiWithAccountInfo,
};
use solana_account_info::AccountInfo;

use super::accounts::{
    CreateAllAccounts, CreateAllParams, ALL_MINT_SIGNER_SEED, ALL_TOKEN_VAULT_SEED,
};

// ============================================================================
// LightPreInit Implementation - Creates all accounts at START of instruction
// ============================================================================

impl<'info> LightPreInit<AccountInfo<'info>, CreateAllParams> for CreateAllAccounts<'info> {
    fn light_pre_init(
        &mut self,
        remaining_accounts: &[AccountInfo<'info>],
        params: &CreateAllParams,
    ) -> std::result::Result<bool, LightSdkTypesError> {
        let mut inner = || -> std::result::Result<bool, LightSdkTypesError> {
            use light_account::LightConfig;
            use solana_program::{clock::Clock, sysvar::Sysvar};

            // Constants for this instruction
            const NUM_LIGHT_PDAS: usize = 2;
            const NUM_LIGHT_MINTS: usize = 1;
            const WITH_CPI_CONTEXT: bool = NUM_LIGHT_PDAS > 0 && NUM_LIGHT_MINTS > 0; // true

            // ====================================================================
            // 1. Build CPI accounts with cpi_context config
            // ====================================================================
            let system_accounts_offset =
                params.create_accounts_proof.system_accounts_offset as usize;
            if remaining_accounts.len() < system_accounts_offset {
                return Err(LightSdkTypesError::FewerAccountsThanSystemAccounts);
            }
            let config = CpiAccountsConfig::new_with_cpi_context(crate::LIGHT_CPI_SIGNER);
            let cpi_accounts = CpiAccounts::new_with_config(
                &self.payer,
                &remaining_accounts[system_accounts_offset..],
                config,
            );

            // ====================================================================
            // 2. Get address tree info
            // ====================================================================
            let address_tree_info = &params.create_accounts_proof.address_tree_info;
            let address_tree_pubkey = address_tree_info
                .get_tree_pubkey(&cpi_accounts)
                .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;
            let output_tree_index = params.create_accounts_proof.output_state_tree_index;

            // ====================================================================
            // 3. Load config, get current slot
            // ====================================================================
            let light_config =
                LightConfig::load_checked(&self.compression_config, &crate::ID.to_bytes())
                    .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;
            let current_slot = Clock::get()
                .map_err(|_| LightSdkTypesError::InvalidInstructionData)?
                .slot;

            // ====================================================================
            // 4. Create PDAs via invoke_write_to_cpi_context_first()
            // ====================================================================
            {
                // CPI context for PDAs - set to first() since we have mints coming after
                let cpi_context = CompressedCpiContext::first();
                let mut new_address_params = Vec::with_capacity(NUM_LIGHT_PDAS);
                let mut account_infos = Vec::with_capacity(NUM_LIGHT_PDAS);

                // 4a. Prepare Borsh PDA (index 0)
                let borsh_record_key = self.borsh_record.key();
                prepare_compressed_account_on_init(
                    &borsh_record_key.to_bytes(),
                    &address_tree_pubkey.to_bytes(),
                    address_tree_info,
                    output_tree_index,
                    0, // assigned_account_index = 0
                    &crate::ID.to_bytes(),
                    &mut new_address_params,
                    &mut account_infos,
                )?;
                self.borsh_record
                    .set_decompressed(&light_config, current_slot);

                // 4b. Prepare ZeroCopy PDA (index 1)
                let zero_copy_record_key = self.zero_copy_record.key();
                prepare_compressed_account_on_init(
                    &zero_copy_record_key.to_bytes(),
                    &address_tree_pubkey.to_bytes(),
                    address_tree_info,
                    output_tree_index,
                    1, // assigned_account_index = 1
                    &crate::ID.to_bytes(),
                    &mut new_address_params,
                    &mut account_infos,
                )?;
                {
                    let mut record = self
                        .zero_copy_record
                        .load_init()
                        .map_err(|_| LightSdkTypesError::Borsh)?;
                    record.set_decompressed(&light_config, current_slot);
                }

                // 4c. Build instruction data and write to CPI context (doesn't execute yet)
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

                // Write to CPI context first (combined execution happens with mints)
                let cpi_context_accounts = CpiContextWriteAccounts {
                    fee_payer: cpi_accounts.fee_payer(),
                    authority: cpi_accounts.authority()?,
                    cpi_context: cpi_accounts.cpi_context()?,
                    cpi_signer: crate::LIGHT_CPI_SIGNER,
                };
                instruction_data.invoke_write_to_cpi_context_first(cpi_context_accounts)?;
            }

            // ====================================================================
            // 5. Create Mint via invoke_create_mints() with offset
            // ====================================================================
            {
                let authority = self.authority.key();
                let mint_signer_key = self.mint_signer.key();

                // Derive mint PDA
                let (mint_pda, mint_bump) = find_mint_address(&mint_signer_key.to_bytes());

                // Derive compression address
                let compression_address = derive_mint_compressed_address(
                    &mint_signer_key.to_bytes(),
                    &address_tree_pubkey.to_bytes(),
                );

                // Build mint signer seeds
                let mint_signer_seeds: &[&[u8]] = &[
                    ALL_MINT_SIGNER_SEED,
                    authority.as_ref(),
                    &[params.mint_signer_bump],
                ];

                // Build SingleMintParams
                let sdk_mints: [SingleMintParams<'_>; NUM_LIGHT_MINTS] = [SingleMintParams {
                    decimals: 6, // mint::decimals = 6
                    address_merkle_tree_root_index: address_tree_info.root_index,
                    mint_authority: authority.to_bytes(),
                    compression_address,
                    mint: mint_pda,
                    bump: mint_bump,
                    freeze_authority: None,
                    mint_seed_pubkey: mint_signer_key.to_bytes(),
                    authority_seeds: None,
                    mint_signer_seeds: Some(mint_signer_seeds),
                    token_metadata: None,
                }];

                // Get state_tree_index
                let state_tree_index = params
                    .create_accounts_proof
                    .state_tree_index
                    .ok_or(LightSdkTypesError::InvalidInstructionData)?;

                let proof = params
                    .create_accounts_proof
                    .proof
                    .0
                    .ok_or(LightSdkTypesError::InvalidInstructionData)?;

                // Build SDK params with cpi_context_offset
                let sdk_params = SdkCreateMintsParams {
                    mints: &sdk_mints,
                    proof,
                    rent_payment: DEFAULT_RENT_PAYMENT,
                    write_top_up: DEFAULT_WRITE_TOP_UP,
                    cpi_context_offset: NUM_LIGHT_PDAS as u8,
                    output_queue_index: params.create_accounts_proof.output_state_tree_index,
                    address_tree_index: address_tree_info.address_merkle_tree_pubkey_index,
                    state_tree_index,
                    base_leaf_index: 0, // N=1, not used
                };

                // Build infra accounts
                let payer_info = self.payer.to_account_info();
                let infra = CreateMintsInfraAccounts {
                    fee_payer: &payer_info,
                    compressible_config: &self.compressible_config,
                    rent_sponsor: &self.rent_sponsor,
                    cpi_authority: &self.cpi_authority,
                };

                // Build mint account arrays
                let mint_seed_accounts = [self.mint_signer.to_account_info()];
                let mint_accounts = [self.mint.to_account_info()];

                // This executes the combined CPI (PDAs + Mint)
                invoke_create_mints(
                    &mint_seed_accounts,
                    &mint_accounts,
                    sdk_params,
                    infra,
                    &cpi_accounts,
                )
                .map_err(|e| LightSdkTypesError::ProgramError(e.into()))?;
            }

            // ====================================================================
            // 6. Create Token Vault via CreateTokenAccountCpi
            // ====================================================================
            {
                let mint_key = self.mint.key();
                let vault_seeds: &[&[u8]] = &[
                    ALL_TOKEN_VAULT_SEED,
                    mint_key.as_ref(),
                    &[params.token_vault_bump],
                ];

                let payer_info = self.payer.to_account_info();
                let token_vault_info = self.token_vault.to_account_info();
                let mint_info = self.mint.to_account_info();
                let system_program_info = self.system_program.to_account_info();
                CreateTokenAccountCpi {
                    payer: &payer_info,
                    account: &token_vault_info,
                    mint: &mint_info,
                    owner: self.vault_owner.key.to_bytes(),
                }
                .rent_free(
                    &self.compressible_config,
                    &self.rent_sponsor,
                    &system_program_info,
                    &crate::ID.to_bytes(),
                )
                .invoke_signed(vault_seeds)?;
            }

            // ====================================================================
            // 7. Create ATA via CreateTokenAtaCpi
            // ====================================================================
            {
                let (_, ata_bump) =
                    derive_associated_token_account(self.ata_owner.key, self.mint.key);

                let payer_info = self.payer.to_account_info();
                let mint_info = self.mint.to_account_info();
                let user_ata_info = self.user_ata.to_account_info();
                let system_program_info = self.system_program.to_account_info();
                CreateTokenAtaCpi {
                    payer: &payer_info,
                    owner: &self.ata_owner,
                    mint: &mint_info,
                    ata: &user_ata_info,
                    bump: ata_bump,
                }
                .rent_free(
                    &self.compressible_config,
                    &self.rent_sponsor,
                    &system_program_info,
                )
                .invoke()?;
            }

            Ok(WITH_CPI_CONTEXT)
        };
        inner()
    }
}

// ============================================================================
// LightFinalize Implementation - No-op for this flow
// ============================================================================

impl<'info> LightFinalize<AccountInfo<'info>, CreateAllParams> for CreateAllAccounts<'info> {
    fn light_finalize(
        &mut self,
        _remaining_accounts: &[AccountInfo<'info>],
        _params: &CreateAllParams,
        _has_pre_init: bool,
    ) -> std::result::Result<(), LightSdkTypesError> {
        // All accounts were created in light_pre_init
        Ok(())
    }
}
