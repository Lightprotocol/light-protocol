use light_account_pinocchio::{
    CpiAccounts, CpiAccountsConfig, CreateMints, CreateMintsStaticAccounts, LightSdkTypesError,
    SingleMintParams,
};
use pinocchio::account_info::AccountInfo;

use super::accounts::{CreateTwoMintsAccounts, CreateTwoMintsParams};

pub fn process(
    ctx: &CreateTwoMintsAccounts<'_>,
    params: &CreateTwoMintsParams,
    remaining_accounts: &[AccountInfo],
) -> Result<(), LightSdkTypesError> {
    let system_accounts_offset = params.create_accounts_proof.system_accounts_offset as usize;
    if remaining_accounts.len() < system_accounts_offset {
        return Err(LightSdkTypesError::FewerAccountsThanSystemAccounts);
    }
    let config = CpiAccountsConfig::new_with_cpi_context(crate::LIGHT_CPI_SIGNER);
    let cpi_accounts = CpiAccounts::new_with_config(
        ctx.payer,
        &remaining_accounts[system_accounts_offset..],
        config,
    );

    let authority = *ctx.authority.key();

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

    let sdk_mints: [SingleMintParams<'_>; 2] = [
        SingleMintParams {
            decimals: 9,
            mint_authority: authority,
            mint_bump: None,
            freeze_authority: None,
            mint_seed_pubkey: *ctx.mint_signer_a.key(),
            authority_seeds: None,
            mint_signer_seeds: Some(mint_signer_a_seeds),
            token_metadata: None,
        },
        SingleMintParams {
            decimals: 6,
            mint_authority: authority,
            mint_bump: None,
            freeze_authority: None,
            mint_seed_pubkey: *ctx.mint_signer_b.key(),
            authority_seeds: None,
            mint_signer_seeds: Some(mint_signer_b_seeds),
            token_metadata: None,
        },
    ];

    CreateMints {
        mints: &sdk_mints,
        proof_data: &params.create_accounts_proof,
        mint_seed_accounts: ctx.mint_signers_slice,
        mint_accounts: ctx.mints_slice,
        static_accounts: CreateMintsStaticAccounts {
            fee_payer: ctx.payer,
            compressible_config: ctx.compressible_config,
            rent_sponsor: ctx.rent_sponsor,
            cpi_authority: ctx.cpi_authority,
        },
        cpi_context_offset: 0,
    }
    .invoke(&cpi_accounts)
}
