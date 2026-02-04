use light_account_pinocchio::{
    CpiAccounts, CpiAccountsConfig, CreateMints, CreateMintsStaticAccounts, LightSdkTypesError,
    SingleMintParams,
};
use pinocchio::account_info::AccountInfo;

use super::accounts::{CreateMintAccounts, CreateMintParams};

pub fn process(
    ctx: &CreateMintAccounts<'_>,
    params: &CreateMintParams,
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
    let mint_signer_key = *ctx.mint_signer.key();

    let mint_signer_seeds: &[&[u8]] = &[
        crate::MINT_SIGNER_SEED_A,
        authority.as_ref(),
        &[params.mint_signer_bump],
    ];

    let sdk_mints: [SingleMintParams<'_>; 1] = [SingleMintParams {
        decimals: 9,
        mint_authority: authority,
        mint_bump: None,
        freeze_authority: None,
        mint_seed_pubkey: mint_signer_key,
        authority_seeds: None,
        mint_signer_seeds: Some(mint_signer_seeds),
        token_metadata: None,
    }];

    let mint_signers = core::slice::from_ref(ctx.mint_signer);
    let mints = core::slice::from_ref(ctx.mint);

    CreateMints {
        mints: &sdk_mints,
        proof_data: &params.create_accounts_proof,
        mint_seed_accounts: mint_signers,
        mint_accounts: mints,
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
