//! Derived code - what the macro would generate.
//! Contains LightPreInit/LightFinalize trait implementations.

use light_account_pinocchio::{
    invoke_create_mints, get_output_queue_next_index, CreateMintsInfraAccounts,
    CreateMintsParams as SdkCreateMintsParams, SingleMintParams,
    derive_mint_compressed_address, find_mint_address,
    CpiAccounts, CpiAccountsConfig, LightFinalize, LightPreInit, LightSdkTypesError,
    PackedAddressTreeInfoExt, DEFAULT_RENT_PAYMENT, DEFAULT_WRITE_TOP_UP,
};
use pinocchio::account_info::AccountInfo;

use super::accounts::{
    CreateDerivedMintsAccounts, CreateDerivedMintsParams, MINT_SIGNER_0_SEED, MINT_SIGNER_1_SEED,
};

// ============================================================================
// LightPreInit Implementation - Creates mints at START of instruction
// ============================================================================

impl LightPreInit<AccountInfo, CreateDerivedMintsParams> for CreateDerivedMintsAccounts<'_> {
    fn light_pre_init(
        &mut self,
        remaining_accounts: &[AccountInfo],
        params: &CreateDerivedMintsParams,
    ) -> std::result::Result<bool, LightSdkTypesError> {
        let inner = || -> std::result::Result<bool, LightSdkTypesError> {

            // ====================================================================
            // STATIC BOILERPLATE (same across all LightPreInit implementations)
            // ====================================================================

            // 1. Build CPI accounts (slice remaining_accounts at system_accounts_offset)
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

            // 2. Get address tree pubkey from packed tree info
            let address_tree_info = &params.create_accounts_proof.address_tree_info;
            let address_tree_pubkey = address_tree_info
                .get_tree_pubkey(&cpi_accounts)
                .map_err(|_| LightSdkTypesError::InvalidInstructionData)?;

            // Constants for this instruction (mirrors macro-generated code)
            const NUM_LIGHT_MINTS: usize = 2;
            const NUM_LIGHT_PDAS: usize = 0; // Set to actual PDA count when combining PDAs + mints
            #[allow(clippy::absurd_extreme_comparisons)]
            const WITH_CPI_CONTEXT: bool = NUM_LIGHT_PDAS > 0 && NUM_LIGHT_MINTS > 0; // true if combining mints + PDAs

            // ====================================================================
            // DYNAMIC CODE (specific to this accounts struct)
            // ====================================================================
            {
                // In pinocchio, .key() returns &[u8; 32], deref to get [u8; 32]
                let authority = *self.authority.key();

                // Get mint signer pubkeys from accounts
                let mint_signer_0 = *self.mint_signer_0.key();
                let mint_signer_1 = *self.mint_signer_1.key();

                // Derive mint PDAs (light-token derives mint PDA from mint_signer)
                // In pinocchio, keys are already [u8; 32], no .to_bytes() needed
                let (mint_0_pda, mint_0_bump) = find_mint_address(&mint_signer_0);
                let (mint_1_pda, mint_1_bump) = find_mint_address(&mint_signer_1);

                // Derive compression addresses (from mint_signer + address_tree)
                // address_tree_pubkey is already [u8; 32] from get_tree_pubkey
                let compression_address_0 = derive_mint_compressed_address(
                    &mint_signer_0,
                    &address_tree_pubkey,
                );
                let compression_address_1 = derive_mint_compressed_address(
                    &mint_signer_1,
                    &address_tree_pubkey,
                );

                // Build mint signer seeds for CPI (mint::seeds + bump)
                let mint_signer_0_seeds: &[&[u8]] = &[
                    MINT_SIGNER_0_SEED,
                    authority.as_ref(),
                    &[params.mint_signer_0_bump],
                ];
                let mint_signer_1_seeds: &[&[u8]] = &[
                    MINT_SIGNER_1_SEED,
                    authority.as_ref(),
                    &[params.mint_signer_1_bump],
                ];

                // Fixed-size array with values from accounts/attributes
                let sdk_mints: [SingleMintParams<'_>; NUM_LIGHT_MINTS] = [
                    SingleMintParams {
                        decimals: 6, // mint::decimals = 6
                        address_merkle_tree_root_index: address_tree_info.root_index,
                        mint_authority: authority,
                        compression_address: compression_address_0,
                        mint: mint_0_pda,
                        bump: mint_0_bump,
                        freeze_authority: None,
                        mint_seed_pubkey: mint_signer_0,
                        authority_seeds: None,
                        mint_signer_seeds: Some(mint_signer_0_seeds),
                        token_metadata: None,
                    },
                    SingleMintParams {
                        decimals: 9, // mint::decimals = 9
                        address_merkle_tree_root_index: address_tree_info.root_index,
                        mint_authority: authority,
                        compression_address: compression_address_1,
                        mint: mint_1_pda,
                        bump: mint_1_bump,
                        freeze_authority: None,
                        mint_seed_pubkey: mint_signer_1,
                        authority_seeds: None,
                        mint_signer_seeds: Some(mint_signer_1_seeds),
                        token_metadata: None,
                    },
                ];

                // ====================================================================
                // INVOKE invoke_create_mints
                // ====================================================================

                // Get state_tree_index (required for decompress discriminator validation)
                let state_tree_index = params
                    .create_accounts_proof
                    .state_tree_index
                    .ok_or(LightSdkTypesError::InvalidInstructionData)?;

                let proof = params
                    .create_accounts_proof
                    .proof
                    .0
                    .ok_or(LightSdkTypesError::InvalidInstructionData)?;

                // Read base_leaf_index from output queue (required for N > 1)
                let output_queue_index = params.create_accounts_proof.output_state_tree_index;
                let output_queue = cpi_accounts
                    .get_tree_account_info(output_queue_index as usize)?;
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

                // Build infra accounts from Accounts struct
                // In pinocchio, accounts are already &AccountInfo, no .to_account_info() needed
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
                .map_err(|_| LightSdkTypesError::CpiFailed)?;
            }
            Ok(WITH_CPI_CONTEXT) // false = mint-only, no CPI context write
        };
        inner()
    }
}

// ============================================================================
// LightFinalize Implementation - No-op for mint-only flow
// ============================================================================

impl LightFinalize<AccountInfo, CreateDerivedMintsParams> for CreateDerivedMintsAccounts<'_> {
    fn light_finalize(
        &mut self,
        _remaining_accounts: &[AccountInfo],
        _params: &CreateDerivedMintsParams,
        _has_pre_init: bool,
    ) -> std::result::Result<(), LightSdkTypesError> {
        // No-op for mint-only flow - create_mints already executed in light_pre_init
        Ok(())
    }
}
