//! Derived code - what the macro would generate for token accounts.

use anchor_lang::prelude::*;
use light_sdk::{
    error::LightSdkError,
    interface::{LightFinalize, LightPreInit},
    Pack, Unpack,
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
/* inside of in_tlv for (i, token) in params.token_accounts.iter().enumerate() {
    if let Some(extension) = token.extension.clone() {
        vec[i] = Some(vec![ExtensionInstructionData::CompressedOnly(extension)]);
    }
}*/
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct TokenVaultSeeds {
    /// Index into remaining_accounts instead of full Pubkey
    pub mint: Pubkey,
}

impl Pack for TokenVaultSeeds {
    type Packed = PackedTokenVaultSeeds;
    fn pack(
        &self,
        remaining_accounts: &mut light_token::anchor::PackedAccounts,
    ) -> std::result::Result<Self::Packed, ProgramError> {
        let mint = remaining_accounts
            .insert_or_get(self.mint)
            .ok_or(ProgramError::NotEnoughAccountKeys)?;
        Ok(TokenVaultSeeds { mint })
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PackedTokenVaultSeeds {
    /// Index into remaining_accounts instead of full Pubkey
    pub mint: u8,
}

impl Unpack for PackedTokenVaultSeeds {
    type Unpacked = TokenVaultSeeds;

    fn unpack(remaining_accounts: &[AccountInfo]) -> Result<Self> {
        let mint = remaining_accounts
            .get(index)
            .ok_or(ProgramError::NotEnoughAccountKeys)?;
        Ok(Self { mint })
    }
}
