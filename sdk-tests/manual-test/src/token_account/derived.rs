//! Derived code - what the macro would generate for token accounts.

use anchor_lang::prelude::*;
use light_sdk::{
    error::LightSdkError,
    interface::{LightFinalize, LightPreInit},
};
use light_token::instruction::CreateTokenAccountCpi;
use solana_account_info::AccountInfo;

use super::accounts::{CreateTokenVaultAccounts, CreateTokenVaultParams, TOKEN_VAULT_SEED};

// ============================================================================
// LightPreInit Implementation - Creates token account at START of instruction
// ============================================================================

impl<'info> LightPreInit<'info, CreateTokenVaultParams> for CreateTokenVaultAccounts<'info> {
    fn light_pre_init(
        &mut self,
        _remaining_accounts: &[AccountInfo<'info>],
        params: &CreateTokenVaultParams,
    ) -> std::result::Result<bool, LightSdkError> {
        // Build PDA seeds: [TOKEN_VAULT_SEED, mint.key(), &[bump]]
        let mint_key = self.mint.key();
        let vault_seeds: &[&[u8]] = &[TOKEN_VAULT_SEED, mint_key.as_ref(), &[params.vault_bump]];

        // Create token account via CPI with rent-free mode
        CreateTokenAccountCpi {
            payer: self.payer.to_account_info(),
            account: self.token_vault.to_account_info(),
            mint: self.mint.clone(),
            owner: *self.vault_owner.key,
        }
        .rent_free(
            self.compressible_config.clone(),
            self.rent_sponsor.clone(),
            self.system_program.to_account_info(),
            &crate::ID,
        )
        .invoke_signed(vault_seeds)?;

        // Token accounts don't use CPI context, return false
        Ok(false)
    }
}

// ============================================================================
// LightFinalize Implementation - No-op for token account only flow
// ============================================================================

impl<'info> LightFinalize<'info, CreateTokenVaultParams> for CreateTokenVaultAccounts<'info> {
    fn light_finalize(
        &mut self,
        _remaining_accounts: &[AccountInfo<'info>],
        _params: &CreateTokenVaultParams,
        _has_pre_init: bool,
    ) -> std::result::Result<(), LightSdkError> {
        Ok(())
    }
}

// ============================================================================
// Instruction Handler
// ============================================================================

pub fn process_create_token_vault<'a, 'info>(
    ctx: Context<'a, '_, 'info, 'info, CreateTokenVaultAccounts<'info>>,
    params: CreateTokenVaultParams,
) -> Result<()> {
    use light_sdk::interface::{LightFinalize, LightPreInit};

    let has_pre_init = ctx
        .accounts
        .light_pre_init(ctx.remaining_accounts, &params)
        .map_err(|e| {
            msg!("light_pre_init error: {:?}", e);
            anchor_lang::error::Error::from(solana_program_error::ProgramError::from(e))
        })?;

    ctx.accounts
        .light_finalize(ctx.remaining_accounts, &params, has_pre_init)
        .map_err(|e| {
            msg!("light_finalize error: {:?}", e);
            anchor_lang::error::Error::from(solana_program_error::ProgramError::from(e))
        })?;

    Ok(())
}
