use light_account_pinocchio::{
    derive_associated_token_account, CreateTokenAtaCpi, LightSdkTypesError,
};
use pinocchio::account_info::AccountInfo;

use super::accounts::{CreateAtaAccounts, CreateAtaParams};

pub fn process(
    ctx: &CreateAtaAccounts<'_>,
    _params: &CreateAtaParams,
    _remaining_accounts: &[AccountInfo],
) -> Result<(), LightSdkTypesError> {
    let (_, bump) = derive_associated_token_account(ctx.ata_owner.key(), ctx.mint.key());

    CreateTokenAtaCpi {
        payer: ctx.payer,
        owner: ctx.ata_owner,
        mint: ctx.mint,
        ata: ctx.user_ata,
        bump,
    }
    .idempotent()
    .rent_free(
        ctx.compressible_config,
        ctx.rent_sponsor,
        ctx.system_program,
    )
    .invoke()?;

    Ok(())
}
