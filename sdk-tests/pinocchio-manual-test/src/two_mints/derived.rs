//! Derived code - what the macro would generate.
//! Contains LightPreInit/LightFinalize trait implementations.

use light_account_pinocchio::{
    CpiAccounts, CpiAccountsConfig, CreateMints, CreateMintsStaticAccounts, LightFinalize,
    LightPreInit, LightSdkTypesError, SingleMintParams,
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

            // Constants
            const NUM_LIGHT_MINTS: usize = 2;
            const NUM_LIGHT_PDAS: usize = 0;
            #[allow(clippy::absurd_extreme_comparisons)]
            const WITH_CPI_CONTEXT: bool = NUM_LIGHT_PDAS > 0 && NUM_LIGHT_MINTS > 0;

            // 2. Build mint params
            let authority = *self.authority.key();
            let mint_signer_0 = *self.mint_signer_0.key();
            let mint_signer_1 = *self.mint_signer_1.key();

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

            let sdk_mints: [SingleMintParams<'_>; NUM_LIGHT_MINTS] = [
                SingleMintParams {
                    decimals: 6,
                    mint_authority: authority,
                    mint_bump: None,
                    freeze_authority: None,
                    mint_seed_pubkey: mint_signer_0,
                    authority_seeds: None,
                    mint_signer_seeds: Some(mint_signer_0_seeds),
                    token_metadata: None,
                },
                SingleMintParams {
                    decimals: 9,
                    mint_authority: authority,
                    mint_bump: None,
                    freeze_authority: None,
                    mint_seed_pubkey: mint_signer_1,
                    authority_seeds: None,
                    mint_signer_seeds: Some(mint_signer_1_seeds),
                    token_metadata: None,
                },
            ];

            // 3. Create mints
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

            Ok(WITH_CPI_CONTEXT)
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
