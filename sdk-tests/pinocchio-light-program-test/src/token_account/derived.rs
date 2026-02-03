use light_account_pinocchio::{
    CreateTokenAccountCpi, LightFinalize, LightPreInit, LightSdkTypesError,
};
use pinocchio::account_info::AccountInfo;

use super::accounts::{CreateTokenVaultAccounts, CreateTokenVaultParams};

impl LightPreInit<AccountInfo, CreateTokenVaultParams> for CreateTokenVaultAccounts<'_> {
    fn light_pre_init(
        &mut self,
        _remaining_accounts: &[AccountInfo],
        params: &CreateTokenVaultParams,
    ) -> std::result::Result<bool, LightSdkTypesError> {
        let inner = || -> std::result::Result<bool, LightSdkTypesError> {
            let mint_key = *self.mint.key();
            let vault_seeds: &[&[u8]] =
                &[crate::VAULT_SEED, mint_key.as_ref(), &[params.vault_bump]];

            CreateTokenAccountCpi {
                payer: self.payer,
                account: self.token_vault,
                mint: self.mint,
                owner: *self.vault_owner.key(),
            }
            .rent_free(
                self.compressible_config,
                self.rent_sponsor,
                self.system_program,
                &crate::ID,
            )
            .invoke_signed(vault_seeds)?;

            Ok(false)
        };
        inner()
    }
}

impl LightFinalize<AccountInfo, CreateTokenVaultParams> for CreateTokenVaultAccounts<'_> {
    fn light_finalize(
        &mut self,
        _remaining_accounts: &[AccountInfo],
        _params: &CreateTokenVaultParams,
        _has_pre_init: bool,
    ) -> std::result::Result<(), LightSdkTypesError> {
        Ok(())
    }
}
