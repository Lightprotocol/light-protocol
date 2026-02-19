use light_account_pinocchio::{
    create_accounts, CreateMintsInput, LightSdkTypesError, SharedAccounts, SingleMintParams,
};
use pinocchio::account_info::AccountInfo;

use super::accounts::{CreateTwoMintsAccounts, CreateTwoMintsParams};

pub fn process(
    ctx: &CreateTwoMintsAccounts<'_>,
    params: &CreateTwoMintsParams,
    remaining_accounts: &[AccountInfo],
) -> Result<(), LightSdkTypesError> {
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

    create_accounts::<AccountInfo, 0, 2, 0, 0, _>(
        [],
        |_, _| Ok(()),
        Some(CreateMintsInput {
            params: [
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
            ],
            mint_seed_accounts: [ctx.mint_signers_slice[0], ctx.mint_signers_slice[1]],
            mint_accounts: [ctx.mints_slice[0], ctx.mints_slice[1]],
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
