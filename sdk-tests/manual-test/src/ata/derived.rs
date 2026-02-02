//! Derived code - what the macro would generate for associated token accounts.

use anchor_lang::prelude::*;
use light_sdk::{
    error::LightSdkError,
    interface::{LightFinalize, LightPreInit},
};
use light_token::instruction::CreateTokenAtaCpi;
use solana_account_info::AccountInfo;

use super::accounts::{CreateAtaAccounts, CreateAtaParams};

// ============================================================================
// LightPreInit Implementation - Creates ATA at START of instruction
// ============================================================================

impl<'info> LightPreInit<AccountInfo<'info>, CreateAtaParams> for CreateAtaAccounts<'info> {
    fn light_pre_init(
        &mut self,
        _remaining_accounts: &[AccountInfo<'info>],
        _params: &CreateAtaParams,
    ) -> std::result::Result<bool, light_sdk::interface::error::LightPdaError> {
        let inner = || -> std::result::Result<bool, LightSdkError> {
            // Derive the ATA bump on-chain
            let (_, bump) = light_token::instruction::derive_associated_token_account(
                self.ata_owner.key,
                self.mint.key,
            );

            // Create ATA via CPI with idempotent + rent-free mode
            // NOTE: Unlike token vaults, ATAs use .invoke() not .invoke_signed()
            // because ATAs are derived from [owner, token_program, mint], not program PDAs
            CreateTokenAtaCpi {
                payer: self.payer.to_account_info(),
                owner: self.ata_owner.clone(),
                mint: self.mint.clone(),
                ata: self.user_ata.to_account_info(),
                bump,
            }
            .idempotent() // Safe: won't fail if ATA already exists
            .rent_free(
                self.compressible_config.clone(),
                self.rent_sponsor.clone(),
                self.system_program.to_account_info(),
            )
            .invoke()?;

            // ATAs don't use CPI context, return false
            Ok(false)
        };
        inner().map_err(Into::into)
    }
}

// ============================================================================
// LightFinalize Implementation - No-op for ATA only flow
// ============================================================================

impl<'info> LightFinalize<AccountInfo<'info>, CreateAtaParams> for CreateAtaAccounts<'info> {
    fn light_finalize(
        &mut self,
        _remaining_accounts: &[AccountInfo<'info>],
        _params: &CreateAtaParams,
        _has_pre_init: bool,
    ) -> std::result::Result<(), light_sdk::interface::error::LightPdaError> {
        Ok(())
    }
}
