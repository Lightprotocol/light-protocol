//! Derived code - what the macro would generate for token accounts.

use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::{
    light_account_checks::{self},
    CreateTokenAccountCpi, LightFinalize, LightPreInit, LightSdkTypesError, Unpack,
};
#[cfg(not(target_os = "solana"))]
use light_account_pinocchio::Pack;
use pinocchio::account_info::AccountInfo;

use super::accounts::{CreateTokenVaultAccounts, CreateTokenVaultParams, TOKEN_VAULT_SEED};

// ============================================================================
// LightPreInit Implementation - Creates token account at START of instruction
// ============================================================================

impl LightPreInit<AccountInfo, CreateTokenVaultParams> for CreateTokenVaultAccounts<'_> {
    fn light_pre_init(
        &mut self,
        _remaining_accounts: &[AccountInfo],
        params: &CreateTokenVaultParams,
    ) -> std::result::Result<bool, LightSdkTypesError> {
        let inner = || -> std::result::Result<bool, LightSdkTypesError> {
            // Build PDA seeds: [TOKEN_VAULT_SEED, mint.key(), &[bump]]
            let mint_key = *self.mint.key();
            let vault_seeds: &[&[u8]] =
                &[TOKEN_VAULT_SEED, mint_key.as_ref(), &[params.vault_bump]];

            // Create token account via CPI with rent-free mode
            // In pinocchio, accounts are already &AccountInfo, no .to_account_info() needed
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

            // Token accounts don't use CPI context, return false
            Ok(false)
        };
        inner()
    }
}

// ============================================================================
// LightFinalize Implementation - No-op for token account only flow
// ============================================================================

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

// ============================================================================
// Token Vault Seeds (for Pack/Unpack)
// ============================================================================

/// Token vault seeds for PDA derivation (client-side).
#[allow(dead_code)]
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct TokenVaultSeeds {
    pub mint: [u8; 32],
}

/// Packed token vault seeds with u8 indices.
#[allow(dead_code)]
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct PackedTokenVaultSeeds {
    pub mint_idx: u8,
    pub bump: u8,
}

// ============================================================================
// Pack/Unpack Implementations
// ============================================================================

#[cfg(not(target_os = "solana"))]
impl Pack<solana_instruction::AccountMeta> for TokenVaultSeeds {
    type Packed = PackedTokenVaultSeeds;
    fn pack(
        &self,
        remaining_accounts: &mut light_account_pinocchio::PackedAccounts,
    ) -> std::result::Result<Self::Packed, LightSdkTypesError> {
        Ok(PackedTokenVaultSeeds {
            mint_idx: remaining_accounts.insert_or_get(solana_pubkey::Pubkey::from(self.mint)),
            bump: 0,
        })
    }
}

impl<AI: light_account_checks::AccountInfoTrait> Unpack<AI> for PackedTokenVaultSeeds {
    type Unpacked = TokenVaultSeeds;

    fn unpack(
        &self,
        remaining_accounts: &[AI],
    ) -> std::result::Result<Self::Unpacked, LightSdkTypesError> {
        let mint = remaining_accounts
            .get(self.mint_idx as usize)
            .ok_or(LightSdkTypesError::NotEnoughAccountKeys)?
            .key();
        Ok(TokenVaultSeeds { mint })
    }
}
