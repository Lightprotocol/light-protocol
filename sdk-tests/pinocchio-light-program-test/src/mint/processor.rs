use light_account_pinocchio::{
    create_accounts, CreateMintsInput, LightSdkTypesError, SharedAccounts, SingleMintParams,
};
use pinocchio::account_info::AccountInfo;

use super::accounts::{CreateMintAccounts, CreateMintParams};

pub fn process(
    ctx: &CreateMintAccounts<'_>,
    params: &CreateMintParams,
    remaining_accounts: &[AccountInfo],
) -> Result<(), LightSdkTypesError> {
    let authority = *ctx.authority.key();
    let mint_signer_key = *ctx.mint_signer.key();

    let mint_signer_seeds: &[&[u8]] = &[
        crate::MINT_SIGNER_SEED_A,
        authority.as_ref(),
        &[params.mint_signer_bump],
    ];

    create_accounts::<AccountInfo, 0, 1, 0, 0, _>(
        [],
        |_, _| Ok(()),
        Some(CreateMintsInput {
            params: [SingleMintParams {
                decimals: 9,
                mint_authority: authority,
                mint_bump: None,
                freeze_authority: None,
                mint_seed_pubkey: mint_signer_key,
                authority_seeds: None,
                mint_signer_seeds: Some(mint_signer_seeds),
                token_metadata: None,
            }],
            mint_seed_accounts: [*ctx.mint_signer],
            mint_accounts: [*ctx.mint],
        }),
        [],
        [],
        &SharedAccounts {
            fee_payer: ctx.payer,
            cpi_signer: crate::LIGHT_CPI_SIGNER,
            proof: &params.create_accounts_proof,
            program_id: crate::ID,
            compression_config: None,
            compressible_config: Some(ctx.compressible_config),
            rent_sponsor: Some(ctx.rent_sponsor),
            cpi_authority: Some(ctx.cpi_authority),
            system_program: None,
        },
        remaining_accounts,
    )?;
    Ok(())
}
