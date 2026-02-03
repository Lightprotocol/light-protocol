use light_account_pinocchio::{
    derive_mint_compressed_address, find_mint_address, get_output_queue_next_index,
    invoke_create_mints, CpiAccounts, CpiAccountsConfig, CreateMintsInfraAccounts,
    CreateMintsParams as SdkCreateMintsParams, LightFinalize, LightPreInit, LightSdkTypesError,
    PackedAddressTreeInfoExt, SingleMintParams, DEFAULT_RENT_PAYMENT, DEFAULT_WRITE_TOP_UP,
};
use pinocchio::account_info::AccountInfo;

use super::accounts::{CreateTwoMintsAccounts, CreateTwoMintsParams};

impl LightPreInit<AccountInfo, CreateTwoMintsParams> for CreateTwoMintsAccounts<'_> {
    fn light_pre_init(
        &mut self,
        remaining_accounts: &[AccountInfo],
        params: &CreateTwoMintsParams,
    ) -> std::result::Result<bool, LightSdkTypesError> {
        let inner = || -> std::result::Result<bool, LightSdkTypesError> {
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

            let address_tree_info = &params.create_accounts_proof.address_tree_info;
            let address_tree_pubkey = address_tree_info
                .get_tree_pubkey(&cpi_accounts)
                .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;

            const NUM_LIGHT_MINTS: usize = 2;
            const NUM_LIGHT_PDAS: usize = 0;
            #[allow(clippy::absurd_extreme_comparisons)]
            const WITH_CPI_CONTEXT: bool = NUM_LIGHT_PDAS > 0 && NUM_LIGHT_MINTS > 0;

            let authority = *self.authority.key();
            let mint_signer_a_key = *self.mint_signer_a.key();
            let mint_signer_b_key = *self.mint_signer_b.key();

            let (mint_a_pda, mint_a_bump) = find_mint_address(&mint_signer_a_key);
            let (mint_b_pda, mint_b_bump) = find_mint_address(&mint_signer_b_key);

            let compression_address_a =
                derive_mint_compressed_address(&mint_signer_a_key, &address_tree_pubkey);
            let compression_address_b =
                derive_mint_compressed_address(&mint_signer_b_key, &address_tree_pubkey);

            let mint_signer_a_seeds: &[&[u8]] = &[
                crate::MINT_SIGNER_SEED_A,
                authority.as_ref(),
                &[params.mint_signer_bump_a],
            ];
            let mint_signer_b_seeds: &[&[u8]] = &[
                crate::MINT_SIGNER_SEED_B,
                authority.as_ref(),
                &[params.mint_signer_bump_b],
            ];

            let sdk_mints: [SingleMintParams<'_>; NUM_LIGHT_MINTS] = [
                SingleMintParams {
                    decimals: 9,
                    address_merkle_tree_root_index: address_tree_info.root_index,
                    mint_authority: authority,
                    compression_address: compression_address_a,
                    mint: mint_a_pda,
                    bump: mint_a_bump,
                    freeze_authority: None,
                    mint_seed_pubkey: mint_signer_a_key,
                    authority_seeds: None,
                    mint_signer_seeds: Some(mint_signer_a_seeds),
                    token_metadata: None,
                },
                SingleMintParams {
                    decimals: 6,
                    address_merkle_tree_root_index: address_tree_info.root_index,
                    mint_authority: authority,
                    compression_address: compression_address_b,
                    mint: mint_b_pda,
                    bump: mint_b_bump,
                    freeze_authority: None,
                    mint_seed_pubkey: mint_signer_b_key,
                    authority_seeds: None,
                    mint_signer_seeds: Some(mint_signer_b_seeds),
                    token_metadata: None,
                },
            ];

            let state_tree_index = params
                .create_accounts_proof
                .state_tree_index
                .ok_or(LightSdkTypesError::InvalidInstructionData)?;

            let proof = params
                .create_accounts_proof
                .proof
                .0
                .ok_or(LightSdkTypesError::InvalidInstructionData)?;

            let output_queue_index = params.create_accounts_proof.output_state_tree_index;
            let output_queue = cpi_accounts.get_tree_account_info(output_queue_index as usize)?;
            let base_leaf_index = get_output_queue_next_index(output_queue)?;

            let sdk_params = SdkCreateMintsParams {
                mints: &sdk_mints,
                proof,
                rent_payment: DEFAULT_RENT_PAYMENT,
                write_top_up: DEFAULT_WRITE_TOP_UP,
                cpi_context_offset: NUM_LIGHT_PDAS as u8,
                output_queue_index,
                address_tree_index: address_tree_info.address_merkle_tree_pubkey_index,
                state_tree_index,
                base_leaf_index,
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

            Ok(WITH_CPI_CONTEXT)
        };
        inner()
    }
}

impl LightFinalize<AccountInfo, CreateTwoMintsParams> for CreateTwoMintsAccounts<'_> {
    fn light_finalize(
        &mut self,
        _remaining_accounts: &[AccountInfo],
        _params: &CreateTwoMintsParams,
        _has_pre_init: bool,
    ) -> std::result::Result<(), LightSdkTypesError> {
        Ok(())
    }
}
