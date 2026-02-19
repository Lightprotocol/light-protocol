//! Derived code - what the macro would generate.
//! Contains LightPreInit/LightFinalize trait implementations.

use light_account_pinocchio::{
    create_accounts, CreateMintsInput, LightFinalize, LightPreInit, LightSdkTypesError,
    SharedAccounts, SingleMintParams,
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

        create_accounts::<AccountInfo, 0, 2, 0, 0, _>(
            [],
            |_, _| Ok(()),
            Some(CreateMintsInput {
                params: [
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
                ],
                mint_seed_accounts: [self.mint_signers_slice[0], self.mint_signers_slice[1]],
                mint_accounts: [self.mints_slice[0], self.mints_slice[1]],
            }),
            [],
            [],
            &SharedAccounts {
                fee_payer: self.payer,
                cpi_signer: crate::LIGHT_CPI_SIGNER,
                proof: &params.create_accounts_proof,
                program_id: crate::ID,
                compression_config: None,
                compressible_config: Some(self.compressible_config),
                rent_sponsor: Some(self.rent_sponsor),
                cpi_authority: Some(self.cpi_authority),
                system_program: None,
            },
            remaining_accounts,
        )
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
