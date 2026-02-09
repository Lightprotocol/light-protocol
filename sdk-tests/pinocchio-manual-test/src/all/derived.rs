//! Derived code for create_all instruction.
//!
//! This implements LightPreInit/LightFinalize for creating all account types:
//! - 2 PDAs (Borsh + ZeroCopy) via `invoke_write_to_cpi_context_first()`
//! - 1 Mint via `CreateMints` with cpi_context_offset
//! - 1 Token Vault via `CreateTokenAccountCpi`
//! - 1 ATA via `CreateTokenAtaCpi`

use light_account_pinocchio::{
    prepare_compressed_account_on_init, CpiAccounts, CpiAccountsConfig, CpiContextWriteAccounts,
    CreateMints, CreateMintsStaticAccounts, CreateTokenAccountCpi, CreateTokenAtaCpi,
    InvokeLightSystemProgram, LightAccount, LightFinalize, LightPreInit, LightSdkTypesError,
    PackedAddressTreeInfoExt, SingleMintParams,
};
use light_compressed_account::instruction_data::{
    cpi_context::CompressedCpiContext, with_account_info::InstructionDataInvokeCpiWithAccountInfo,
};
use pinocchio::account_info::AccountInfo;

use super::accounts::{
    CreateAllAccounts, CreateAllParams, ALL_MINT_SIGNER_SEED, ALL_TOKEN_VAULT_SEED,
};

// ============================================================================
// LightPreInit Implementation - Creates all accounts at START of instruction
// ============================================================================

impl LightPreInit<AccountInfo, CreateAllParams> for CreateAllAccounts<'_> {
    fn light_pre_init(
        &mut self,
        remaining_accounts: &[AccountInfo],
        params: &CreateAllParams,
    ) -> std::result::Result<bool, LightSdkTypesError> {
        let inner = || -> std::result::Result<bool, LightSdkTypesError> {
            use light_account_pinocchio::LightConfig;
            use pinocchio::sysvars::{clock::Clock, Sysvar};

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
                self.payer,
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
            let light_config = LightConfig::load_checked(self.compression_config, &crate::ID)
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
                let borsh_record_key = *self.borsh_record.key();
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
                // Set compression_info on the Borsh record via mut_from_account_data
                {
                    let mut account_data = self
                        .borsh_record
                        .try_borrow_mut_data()
                        .map_err(|_| LightSdkTypesError::Borsh)?;
                    let record =
                        crate::pda::MinimalRecord::mut_from_account_data(&mut account_data);
                    record.set_decompressed(&light_config, current_slot);
                }

                // 4b. Prepare ZeroCopy PDA (index 1)
                let zero_copy_record_key = *self.zero_copy_record.key();
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
                    let mut account_data = self
                        .zero_copy_record
                        .try_borrow_mut_data()
                        .map_err(|_| LightSdkTypesError::Borsh)?;
                    let record_bytes = &mut account_data
                        [8..8 + core::mem::size_of::<crate::account_loader::ZeroCopyRecord>()];
                    let record: &mut crate::account_loader::ZeroCopyRecord =
                        bytemuck::from_bytes_mut(record_bytes);
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
            // 5. Create Mint via CreateMints with cpi_context_offset
            // ====================================================================
            {
                let authority_key = *self.authority.key();
                let mint_signer_key = *self.mint_signer.key();

                let mint_signer_seeds: &[&[u8]] = &[
                    ALL_MINT_SIGNER_SEED,
                    authority_key.as_ref(),
                    &[params.mint_signer_bump],
                ];

                let sdk_mints: [SingleMintParams<'_>; NUM_LIGHT_MINTS] = [SingleMintParams {
                    decimals: 6,
                    mint_authority: authority_key,
                    mint_bump: None,
                    freeze_authority: None,
                    mint_seed_pubkey: mint_signer_key,
                    authority_seeds: None,
                    mint_signer_seeds: Some(mint_signer_seeds),
                    token_metadata: None,
                }];

                CreateMints {
                    mints: &sdk_mints,
                    proof_data: &params.create_accounts_proof,
                    mint_seed_accounts: self.mint_signers_slice,
                    mint_accounts: self.mints_slice,
                    static_accounts: CreateMintsStaticAccounts {
                        fee_payer: self.payer,
                        compressible_config: self.compressible_config,
                        rent_sponsor: self.rent_sponsor,
                        cpi_authority: self.cpi_authority,
                    },
                    cpi_context_offset: NUM_LIGHT_PDAS as u8,
                }
                .invoke(&cpi_accounts)?;
            }

            // ====================================================================
            // 6. Create Token Vault via CreateTokenAccountCpi
            // ====================================================================
            {
                let mint_key = *self.mint.key();
                let vault_seeds: &[&[u8]] = &[
                    ALL_TOKEN_VAULT_SEED,
                    mint_key.as_ref(),
                    &[params.token_vault_bump],
                ];

                CreateTokenAccountCpi {
                    payer: self.payer,
                    account: self.token_vault,
                    mint: self.mint,
                    owner: *self.vault_owner.key(),
                }
                .rent_free(
                    self.compressible_config,
                    self.rent_sponsor,
                    self.system_program,
                    &crate::ID,
                )
                .invoke_signed(vault_seeds)?;
            }

            // ====================================================================
            // 7. Create ATA via CreateTokenAtaCpi
            // ====================================================================
            {
                CreateTokenAtaCpi {
                    payer: self.payer,
                    owner: self.ata_owner,
                    mint: self.mint,
                    ata: self.user_ata,
                }
                .rent_free(
                    self.compressible_config,
                    self.rent_sponsor,
                    self.system_program,
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

impl LightFinalize<AccountInfo, CreateAllParams> for CreateAllAccounts<'_> {
    fn light_finalize(
        &mut self,
        _remaining_accounts: &[AccountInfo],
        _params: &CreateAllParams,
        _has_pre_init: bool,
    ) -> std::result::Result<(), LightSdkTypesError> {
        // All accounts were created in light_pre_init
        Ok(())
    }
}
