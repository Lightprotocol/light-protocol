use light_account_pinocchio::{CreateTokenAccountCpi, LightSdkTypesError};
use pinocchio::account_info::AccountInfo;

use super::accounts::{CreateTokenVaultAccounts, CreateTokenVaultParams};

pub fn process(
    ctx: &CreateTokenVaultAccounts<'_>,
    params: &CreateTokenVaultParams,
    _remaining_accounts: &[AccountInfo],
) -> Result<(), LightSdkTypesError> {
    let mint_key = *ctx.mint.key();
    let vault_seeds: &[&[u8]] = &[crate::VAULT_SEED, mint_key.as_ref(), &[params.vault_bump]];

    CreateTokenAccountCpi {
        payer: ctx.payer,
        account: ctx.token_vault,
        mint: ctx.mint,
        owner: *ctx.vault_owner.key(),
    }
    .rent_free(
        ctx.compressible_config,
        ctx.rent_sponsor,
        ctx.system_program,
        &crate::ID,
    )
    .invoke_signed(vault_seeds)?;

    Ok(())
}
