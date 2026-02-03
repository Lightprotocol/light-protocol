use light_account_pinocchio::{
    derive_associated_token_account, CreateTokenAtaCpi, LightFinalize, LightPreInit,
    LightSdkTypesError,
};
use pinocchio::account_info::AccountInfo;

use super::accounts::{CreateAtaAccounts, CreateAtaParams};

impl LightPreInit<AccountInfo, CreateAtaParams> for CreateAtaAccounts<'_> {
    fn light_pre_init(
        &mut self,
        _remaining_accounts: &[AccountInfo],
        _params: &CreateAtaParams,
    ) -> std::result::Result<bool, LightSdkTypesError> {
        let inner = || -> std::result::Result<bool, LightSdkTypesError> {
            let (_, bump) = derive_associated_token_account(self.ata_owner.key(), self.mint.key());

            CreateTokenAtaCpi {
                payer: self.payer,
                owner: self.ata_owner,
                mint: self.mint,
                ata: self.user_ata,
                bump,
            }
            .idempotent()
            .rent_free(
                self.compressible_config,
                self.rent_sponsor,
                self.system_program,
            )
            .invoke()?;

            Ok(false)
        };
        inner()
    }
}

impl LightFinalize<AccountInfo, CreateAtaParams> for CreateAtaAccounts<'_> {
    fn light_finalize(
        &mut self,
        _remaining_accounts: &[AccountInfo],
        _params: &CreateAtaParams,
        _has_pre_init: bool,
    ) -> std::result::Result<(), LightSdkTypesError> {
        Ok(())
    }
}
