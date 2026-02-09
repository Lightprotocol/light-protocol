//! Derived code - what the macro would generate for associated token accounts.

use light_account_pinocchio::{CreateTokenAtaCpi, LightFinalize, LightPreInit, LightSdkTypesError};
use pinocchio::account_info::AccountInfo;

use super::accounts::{CreateAtaAccounts, CreateAtaParams};

// ============================================================================
// LightPreInit Implementation - Creates ATA at START of instruction
// ============================================================================

impl LightPreInit<AccountInfo, CreateAtaParams> for CreateAtaAccounts<'_> {
    fn light_pre_init(
        &mut self,
        _remaining_accounts: &[AccountInfo],
        _params: &CreateAtaParams,
    ) -> std::result::Result<bool, LightSdkTypesError> {
        let inner = || -> std::result::Result<bool, LightSdkTypesError> {
            // Create ATA via CPI with idempotent + rent-free mode
            // NOTE: Unlike token vaults, ATAs use .invoke() not .invoke_signed()
            // because ATAs are derived from [owner, token_program, mint], not program PDAs
            CreateTokenAtaCpi {
                payer: self.payer,
                owner: self.ata_owner,
                mint: self.mint,
                ata: self.user_ata,
            }
            .idempotent() // Safe: won't fail if ATA already exists
            .rent_free(
                self.compressible_config,
                self.rent_sponsor,
                self.system_program,
            )
            .invoke()?;

            // ATAs don't use CPI context, return false
            Ok(false)
        };
        inner()
    }
}

// ============================================================================
// LightFinalize Implementation - No-op for ATA only flow
// ============================================================================

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
