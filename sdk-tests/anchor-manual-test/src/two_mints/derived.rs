//! Derived code - what the macro would generate.
//! Contains LightPreInit/LightFinalize trait implementations.

use anchor_lang::prelude::*;
use light_account::{
    create_accounts, CreateMintsInput, LightFinalize, LightPreInit, LightSdkTypesError,
    SharedAccounts, SingleMintParams,
};
use solana_account_info::AccountInfo;

use super::accounts::{
    CreateDerivedMintsAccounts, CreateDerivedMintsParams, MINT_SIGNER_0_SEED, MINT_SIGNER_1_SEED,
};

// ============================================================================
// LightPreInit Implementation - Creates mints at START of instruction
// ============================================================================

impl<'info> LightPreInit<AccountInfo<'info>, CreateDerivedMintsParams>
    for CreateDerivedMintsAccounts<'info>
{
    fn light_pre_init(
        &mut self,
        remaining_accounts: &[AccountInfo<'info>],
        params: &CreateDerivedMintsParams,
    ) -> std::result::Result<bool, LightSdkTypesError> {
        let authority = self.authority.key().to_bytes();
        let mint_signer_0 = self.mint_signer_0.key().to_bytes();
        let mint_signer_1 = self.mint_signer_1.key().to_bytes();

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

        let payer_info = self.payer.to_account_info();

        create_accounts::<AccountInfo<'info>, 0, 2, 0, 0, _>(
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
                mint_seed_accounts: [
                    self.mint_signer_0.to_account_info(),
                    self.mint_signer_1.to_account_info(),
                ],
                mint_accounts: [self.mint_0.to_account_info(), self.mint_1.to_account_info()],
            }),
            [],
            [],
            &SharedAccounts {
                fee_payer: &payer_info,
                cpi_signer: crate::LIGHT_CPI_SIGNER,
                proof: &params.create_accounts_proof,
                program_id: crate::LIGHT_CPI_SIGNER.program_id,
                compression_config: None,
                compressible_config: Some(&self.compressible_config),
                rent_sponsor: Some(&self.rent_sponsor),
                cpi_authority: Some(&self.cpi_authority),
                system_program: None,
            },
            remaining_accounts,
        )
    }
}

// ============================================================================
// LightFinalize Implementation - No-op for mint-only flow
// ============================================================================

impl<'info> LightFinalize<AccountInfo<'info>, CreateDerivedMintsParams>
    for CreateDerivedMintsAccounts<'info>
{
    fn light_finalize(
        &mut self,
        _remaining_accounts: &[AccountInfo<'info>],
        _params: &CreateDerivedMintsParams,
        _has_pre_init: bool,
    ) -> std::result::Result<(), LightSdkTypesError> {
        // No-op for mint-only flow - create_mints already executed in light_pre_init
        Ok(())
    }
}
