//! Derived code for create_all instruction.
//!
//! This implements LightPreInit/LightFinalize for creating all account types:
//! - 2 PDAs (Borsh + ZeroCopy) via `invoke_write_to_cpi_context_first()`
//! - 1 Mint via `invoke_create_mints()` with cpi_context_offset
//! - 1 Token Vault via `CreateTokenAccountCpi`
//! - 1 ATA via `CreateTokenAtaCpi`

use anchor_lang::prelude::*;
use light_compressed_account::instruction_data::{
    cpi_context::CompressedCpiContext, with_account_info::InstructionDataInvokeCpiWithAccountInfo,
};
use light_sdk::{
    cpi::{v2::CpiAccounts, CpiAccountsConfig, InvokeLightSystemProgram},
    error::LightSdkError,
    instruction::PackedAddressTreeInfoExt,
    interface::{LightFinalize, LightPreInit},
    sdk_types::CpiContextWriteAccounts,
};
use light_token::{
    compressible::{invoke_create_mints, CreateMintsInfraAccounts},
    instruction::{
        derive_mint_compressed_address, find_mint_address,
        CreateMintsParams as SdkCreateMintsParams, CreateTokenAccountCpi, CreateTokenAtaCpi,
        SingleMintParams,
    },
};
use solana_account_info::AccountInfo;
use solana_program_error::ProgramError;

use super::accounts::{
    CreateAllAccounts, CreateAllParams, ALL_MINT_SIGNER_SEED, ALL_TOKEN_VAULT_SEED,
};
use light_sdk::interface::{prepare_compressed_account_on_init, LightAccount};

// ============================================================================
// LightPreInit Implementation - Creates all accounts at START of instruction
// ============================================================================

impl<'info> LightPreInit<'info, CreateAllParams> for CreateAllAccounts<'info> {
    fn light_pre_init(
        &mut self,
        remaining_accounts: &[AccountInfo<'info>],
        params: &CreateAllParams,
    ) -> std::result::Result<bool, LightSdkError> {
        use light_sdk::interface::config::LightConfig;
        use solana_program::clock::Clock;
        use solana_program::sysvar::Sysvar;

        // Constants for this instruction
        const NUM_LIGHT_PDAS: usize = 2;
        const NUM_LIGHT_MINTS: usize = 1;
        const WITH_CPI_CONTEXT: bool = NUM_LIGHT_PDAS > 0 && NUM_LIGHT_MINTS > 0; // true

        // ====================================================================
        // 1. Build CPI accounts with cpi_context config
        // ====================================================================
        let system_accounts_offset = params.create_accounts_proof.system_accounts_offset as usize;
        if remaining_accounts.len() < system_accounts_offset {
            return Err(LightSdkError::FewerAccountsThanSystemAccounts);
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
            .map_err(|_| LightSdkError::from(ProgramError::InvalidAccountData))?;
        let output_tree_index = params.create_accounts_proof.output_state_tree_index;

        // ====================================================================
        // 3. Load config, get current slot
        // ====================================================================
        let light_config = LightConfig::load_checked(&self.compression_config, &crate::ID)
            .map_err(|_| LightSdkError::from(ProgramError::InvalidAccountData))?;
        let current_slot = Clock::get()
            .map_err(|_| LightSdkError::from(ProgramError::InvalidAccountData))?
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
                &borsh_record_key,
                &address_tree_pubkey,
                address_tree_info,
                output_tree_index,
                0, // assigned_account_index = 0
                &crate::ID,
                &mut new_address_params,
                &mut account_infos,
            )?;
            self.borsh_record
                .set_decompressed(&light_config, current_slot);

            // 4b. Prepare ZeroCopy PDA (index 1)
            let zero_copy_record_key = self.zero_copy_record.key();
            prepare_compressed_account_on_init(
                &zero_copy_record_key,
                &address_tree_pubkey,
                address_tree_info,
                output_tree_index,
                1, // assigned_account_index = 1
                &crate::ID,
                &mut new_address_params,
                &mut account_infos,
            )?;
            {
                let mut record = self
                    .zero_copy_record
                    .load_init()
                    .map_err(|_| LightSdkError::from(ProgramError::AccountBorrowFailed))?;
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
                authority: cpi_accounts.authority().map_err(LightSdkError::from)?,
                cpi_context: cpi_accounts.cpi_context().map_err(LightSdkError::from)?,
                cpi_signer: crate::LIGHT_CPI_SIGNER,
            };
            instruction_data
                .invoke_write_to_cpi_context_first(cpi_context_accounts)
                .map_err(LightSdkError::from)?;
        }

        // ====================================================================
        // 5. Create Mint via invoke_create_mints() with offset
        // ====================================================================
        {
            let authority = self.authority.key();
            let mint_signer_key = self.mint_signer.key();

            // Derive mint PDA
            let (mint_pda, mint_bump) = find_mint_address(&solana_pubkey::Pubkey::new_from_array(
                mint_signer_key.to_bytes(),
            ));

            // Derive compression address
            let compression_address = derive_mint_compressed_address(
                &solana_pubkey::Pubkey::new_from_array(mint_signer_key.to_bytes()),
                &solana_pubkey::Pubkey::new_from_array(address_tree_pubkey.to_bytes()),
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
                mint_authority: solana_pubkey::Pubkey::new_from_array(authority.to_bytes()),
                compression_address,
                mint: mint_pda,
                bump: mint_bump,
                freeze_authority: None,
                mint_seed_pubkey: solana_pubkey::Pubkey::new_from_array(mint_signer_key.to_bytes()),
                authority_seeds: None,
                mint_signer_seeds: Some(mint_signer_seeds),
                token_metadata: None,
            }];

            // Get state_tree_index
            let state_tree_index = params
                .create_accounts_proof
                .state_tree_index
                .ok_or(LightSdkError::from(ProgramError::InvalidArgument))?;

            let proof = params
                .create_accounts_proof
                .proof
                .0
                .ok_or(LightSdkError::from(ProgramError::InvalidArgument))?;

            // Build SDK params with cpi_context_offset
            let sdk_params = SdkCreateMintsParams::new(&sdk_mints, proof)
                .with_output_queue_index(params.create_accounts_proof.output_state_tree_index)
                .with_address_tree_index(address_tree_info.address_merkle_tree_pubkey_index)
                .with_state_tree_index(state_tree_index)
                .with_cpi_context_offset(NUM_LIGHT_PDAS as u8); // Offset by PDA count

            // Build infra accounts
            let infra = CreateMintsInfraAccounts {
                fee_payer: self.payer.to_account_info(),
                compressible_config: self.compressible_config.clone(),
                rent_sponsor: self.rent_sponsor.clone(),
                cpi_authority: self.cpi_authority.clone(),
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
            )?;
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

            CreateTokenAccountCpi {
                payer: self.payer.to_account_info(),
                account: self.token_vault.to_account_info(),
                mint: self.mint.to_account_info(),
                owner: *self.vault_owner.key,
            }
            .rent_free(
                self.compressible_config.clone(),
                self.rent_sponsor.clone(),
                self.system_program.to_account_info(),
                &crate::ID,
            )
            .invoke_signed(vault_seeds)?;
        }

        // ====================================================================
        // 7. Create ATA via CreateTokenAtaCpi
        // ====================================================================
        {
            let (_, ata_bump) = light_token::instruction::derive_associated_token_account(
                self.ata_owner.key,
                self.mint.key,
            );

            CreateTokenAtaCpi {
                payer: self.payer.to_account_info(),
                owner: self.ata_owner.clone(),
                mint: self.mint.to_account_info(),
                ata: self.user_ata.to_account_info(),
                bump: ata_bump,
            }
            .rent_free(
                self.compressible_config.clone(),
                self.rent_sponsor.clone(),
                self.system_program.to_account_info(),
            )
            .invoke()?;
        }

        Ok(WITH_CPI_CONTEXT)
    }
}

// ============================================================================
// LightFinalize Implementation - No-op for this flow
// ============================================================================

impl<'info> LightFinalize<'info, CreateAllParams> for CreateAllAccounts<'info> {
    fn light_finalize(
        &mut self,
        _remaining_accounts: &[AccountInfo<'info>],
        _params: &CreateAllParams,
        _has_pre_init: bool,
    ) -> std::result::Result<(), LightSdkError> {
        // All accounts were created in light_pre_init
        Ok(())
    }
}
