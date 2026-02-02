//! Derived code - what the macro would generate for token accounts.

use anchor_lang::prelude::*;
#[cfg(not(target_os = "solana"))]
use light_account::Pack;
use light_account::{LightFinalize, LightPreInit, LightSdkTypesError, Unpack};
use light_token::instruction::CreateTokenAccountCpi;
use solana_account_info::AccountInfo;

use super::accounts::{CreateTokenVaultAccounts, CreateTokenVaultParams, TOKEN_VAULT_SEED};

// ============================================================================
// LightPreInit Implementation - Creates token account at START of instruction
// ============================================================================

impl<'info> LightPreInit<AccountInfo<'info>, CreateTokenVaultParams>
    for CreateTokenVaultAccounts<'info>
{
    fn light_pre_init(
        &mut self,
        _remaining_accounts: &[AccountInfo<'info>],
        params: &CreateTokenVaultParams,
    ) -> std::result::Result<bool, LightSdkTypesError> {
        let inner = || -> std::result::Result<bool, LightSdkTypesError> {
            // Build PDA seeds: [TOKEN_VAULT_SEED, mint.key(), &[bump]]
            let mint_key = self.mint.key();
            let vault_seeds: &[&[u8]] =
                &[TOKEN_VAULT_SEED, mint_key.as_ref(), &[params.vault_bump]];

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
            .invoke_signed(vault_seeds).map_err(|_| LightSdkTypesError::CpiFailed)?;

            // Token accounts don't use CPI context, return false
            Ok(false)
        };
        inner()
    }
}

// ============================================================================
// LightFinalize Implementation - No-op for token account only flow
// ============================================================================

impl<'info> LightFinalize<AccountInfo<'info>, CreateTokenVaultParams>
    for CreateTokenVaultAccounts<'info>
{
    fn light_finalize(
        &mut self,
        _remaining_accounts: &[AccountInfo<'info>],
        _params: &CreateTokenVaultParams,
        _has_pre_init: bool,
    ) -> std::result::Result<(), LightSdkTypesError> {
        Ok(())
    }
}
/* inside of in_tlv for (i, token) in params.token_accounts.iter().enumerate() {
    if let Some(extension) = token.extension.clone() {
        vec[i] = Some(vec![ExtensionInstructionData::CompressedOnly(extension)]);
    }
}*/
#[allow(dead_code)]
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct TokenVaultSeeds {
    pub mint: Pubkey,
}

#[cfg(not(target_os = "solana"))]
impl Pack<solana_program::instruction::AccountMeta> for TokenVaultSeeds {
    type Packed = PackedTokenVaultSeeds;
    fn pack(
        &self,
        remaining_accounts: &mut light_account::PackedAccounts,
    ) -> std::result::Result<Self::Packed, LightSdkTypesError> {
        Ok(PackedTokenVaultSeeds {
            mint_idx: remaining_accounts.insert_or_get(self.mint),
            bump: 0,
        })
    }
}

#[allow(dead_code)]
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PackedTokenVaultSeeds {
    pub mint_idx: u8,
    pub bump: u8,
}

impl<'a> Unpack<AccountInfo<'a>> for PackedTokenVaultSeeds {
    type Unpacked = TokenVaultSeeds;

    fn unpack(
        &self,
        remaining_accounts: &[AccountInfo<'a>],
    ) -> std::result::Result<Self::Unpacked, LightSdkTypesError> {
        let mint = *remaining_accounts
            .get(self.mint_idx as usize)
            .ok_or(LightSdkTypesError::NotEnoughAccountKeys)?
            .key;
        Ok(TokenVaultSeeds { mint })
    }
}
