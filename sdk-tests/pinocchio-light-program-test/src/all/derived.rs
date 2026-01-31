use borsh::BorshDeserialize;
use light_account_pinocchio::{
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
use pinocchio::account_info::AccountInfo;

use super::accounts::{CreateAllAccounts, CreateAllParams};

impl LightPreInit<AccountInfo, CreateAllParams> for CreateAllAccounts<'_> {
    fn light_pre_init(
        &mut self,
        remaining_accounts: &[AccountInfo],
        params: &CreateAllParams,
    ) -> std::result::Result<bool, LightSdkTypesError> {
        let inner = || -> std::result::Result<bool, LightSdkTypesError> {
            use light_account_pinocchio::LightConfig;
            use pinocchio::sysvars::{clock::Clock, Sysvar};

            const NUM_LIGHT_PDAS: usize = 2;
            const NUM_LIGHT_MINTS: usize = 1;
            const WITH_CPI_CONTEXT: bool = true;

            // 1. Build CPI accounts
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

            // 2. Address tree info
            let address_tree_info = &params.create_accounts_proof.address_tree_info;
            let address_tree_pubkey = address_tree_info
                .get_tree_pubkey(&cpi_accounts)
                .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;
            let output_tree_index = params.create_accounts_proof.output_state_tree_index;

            // 3. Load config, get slot
            let light_config = LightConfig::load_checked(self.compression_config, &crate::ID)
                .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;
            let current_slot = Clock::get()
                .map_err(|_| LightSdkTypesError::InvalidInstructionData)?
                .slot;

            // 4. Create PDAs via invoke_write_to_cpi_context_first
            {
                let cpi_context = CompressedCpiContext::first();
                let mut new_address_params = Vec::with_capacity(NUM_LIGHT_PDAS);
                let mut account_infos = Vec::with_capacity(NUM_LIGHT_PDAS);

                // 4a. Borsh PDA (index 0)
                let borsh_record_key = *self.borsh_record.key();
                prepare_compressed_account_on_init(
                    &borsh_record_key,
                    &address_tree_pubkey,
                    address_tree_info,
                    output_tree_index,
                    0,
                    &crate::ID,
                    &mut new_address_params,
                    &mut account_infos,
                )?;
                {
                    let mut account_data = self
                        .borsh_record
                        .try_borrow_mut_data()
                        .map_err(|_| LightSdkTypesError::Borsh)?;
                    let mut record =
                        crate::state::MinimalRecord::try_from_slice(&account_data[8..])
                            .map_err(|_| LightSdkTypesError::Borsh)?;
                    record.set_decompressed(&light_config, current_slot);
                    let serialized =
                        borsh::to_vec(&record).map_err(|_| LightSdkTypesError::Borsh)?;
                    account_data[8..8 + serialized.len()].copy_from_slice(&serialized);
                }

                // 4b. ZeroCopy PDA (index 1)
                let zero_copy_record_key = *self.zero_copy_record.key();
                prepare_compressed_account_on_init(
                    &zero_copy_record_key,
                    &address_tree_pubkey,
                    address_tree_info,
                    output_tree_index,
                    1,
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
                        [8..8 + core::mem::size_of::<crate::state::ZeroCopyRecord>()];
                    let record: &mut crate::state::ZeroCopyRecord =
                        bytemuck::from_bytes_mut(record_bytes);
                    record.set_decompressed(&light_config, current_slot);
                }

                // 4c. Write to CPI context
                let instruction_data = InstructionDataInvokeCpiWithAccountInfo {
                    mode: 1,
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

                let cpi_context_accounts = CpiContextWriteAccounts {
                    fee_payer: cpi_accounts.fee_payer(),
                    authority: cpi_accounts.authority()?,
                    cpi_context: cpi_accounts.cpi_context()?,
                    cpi_signer: crate::LIGHT_CPI_SIGNER,
                };
                instruction_data.invoke_write_to_cpi_context_first(cpi_context_accounts)?;
            }

            // 5. Create Mint
            {
                let authority_key = *self.authority.key();
                let mint_signer_key = *self.mint_signer.key();

                let (mint_pda, mint_bump) = find_mint_address(&mint_signer_key);
                let compression_address =
                    derive_mint_compressed_address(&mint_signer_key, &address_tree_pubkey);

                let mint_signer_seeds: &[&[u8]] = &[
                    crate::MINT_SIGNER_SEED_A,
                    authority_key.as_ref(),
                    &[params.mint_signer_bump],
                ];

                let sdk_mints: [SingleMintParams<'_>; NUM_LIGHT_MINTS] = [SingleMintParams {
                    decimals: 9,
                    address_merkle_tree_root_index: address_tree_info.root_index,
                    mint_authority: authority_key,
                    compression_address,
                    mint: mint_pda,
                    bump: mint_bump,
                    freeze_authority: None,
                    mint_seed_pubkey: mint_signer_key,
                    authority_seeds: None,
                    mint_signer_seeds: Some(mint_signer_seeds),
                    token_metadata: None,
                }];

                let state_tree_index = params
                    .create_accounts_proof
                    .state_tree_index
                    .ok_or(LightSdkTypesError::InvalidInstructionData)?;

                let proof = params
                    .create_accounts_proof
                    .proof
                    .0
                    .ok_or(LightSdkTypesError::InvalidInstructionData)?;

                let sdk_params = SdkCreateMintsParams {
                    mints: &sdk_mints,
                    proof,
                    rent_payment: DEFAULT_RENT_PAYMENT,
                    write_top_up: DEFAULT_WRITE_TOP_UP,
                    cpi_context_offset: NUM_LIGHT_PDAS as u8,
                    output_queue_index: params.create_accounts_proof.output_state_tree_index,
                    address_tree_index: address_tree_info.address_merkle_tree_pubkey_index,
                    state_tree_index,
                    base_leaf_index: 0,
                };

                let infra = CreateMintsInfraAccounts {
                    fee_payer: self.payer,
                    compressible_config: self.compressible_config,
                    rent_sponsor: self.rent_sponsor,
                    cpi_authority: self.cpi_authority,
                };

                invoke_create_mints(
                    self.mint_signers_slice,
                    self.mints_slice,
                    sdk_params,
                    infra,
                    &cpi_accounts,
                )
                .map_err(|e| LightSdkTypesError::ProgramError(e.into()))?;
            }

            // 6. Create Token Vault
            {
                let mint_key = *self.mint.key();
                let vault_seeds: &[&[u8]] = &[
                    crate::VAULT_SEED,
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

            // 7. Create ATA
            {
                let (_, ata_bump) =
                    derive_associated_token_account(self.ata_owner.key(), self.mint.key());

                CreateTokenAtaCpi {
                    payer: self.payer,
                    owner: self.ata_owner,
                    mint: self.mint,
                    ata: self.user_ata,
                    bump: ata_bump,
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

impl LightFinalize<AccountInfo, CreateAllParams> for CreateAllAccounts<'_> {
    fn light_finalize(
        &mut self,
        _remaining_accounts: &[AccountInfo],
        _params: &CreateAllParams,
        _has_pre_init: bool,
    ) -> std::result::Result<(), LightSdkTypesError> {
        Ok(())
    }
}
