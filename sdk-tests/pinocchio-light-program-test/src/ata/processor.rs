use light_account_pinocchio::{CreateTokenAtaCpi, LightSdkTypesError};
use pinocchio::account_info::AccountInfo;

use super::accounts::{CreateAtaAccounts, CreateAtaParams};

pub fn process(
    ctx: &CreateAtaAccounts<'_>,
    _params: &CreateAtaParams,
    _remaining_accounts: &[AccountInfo],
) -> Result<(), LightSdkTypesError> {
    CreateTokenAtaCpi {
        payer: ctx.payer,
        owner: ctx.ata_owner,
        mint: ctx.mint,
        ata: ctx.user_ata,
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
