//! D10 Test: Single token account creation via macro
//!
//! Tests #[light_account(init, token, ...)] automatic code generation
//! for creating a single compressed token account (CToken vault).
//!
//! This differs from D5 tests which use mark-only mode and manual creation.
//! Here the macro should generate the CreateTokenAccountCpi call automatically.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::LightAccounts;
use light_token_sdk::token::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as LIGHT_TOKEN_RENT_SPONSOR};

/// Seed for the vault authority PDA
pub const D10_SINGLE_VAULT_AUTH_SEED: &[u8] = b"d10_single_vault_auth";
/// Seed for the vault token account PDA
pub const D10_SINGLE_VAULT_SEED: &[u8] = b"d10_single_vault";

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D10SingleVaultParams {
    pub create_accounts_proof: CreateAccountsProof,
    /// Bump for the d10_single_vault PDA (needed for invoke_signed)
    pub vault_bump: u8,
}

/// Tests #[light_account(init, token, ...)] automatic code generation.
/// The macro should generate CreateTokenAccountCpi in LightFinalize.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D10SingleVaultParams)]
pub struct D10SingleVault<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Token mint
    pub d10_mint: AccountInfo<'info>,

    #[account(
        seeds = [D10_SINGLE_VAULT_AUTH_SEED],
        bump,
    )]
    pub d10_vault_authority: UncheckedAccount<'info>,

    /// Token vault account - macro should generate creation code.
    /// The `authority` seeds must match the account's PDA seeds (including bump) for invoke_signed.
    #[account(
        mut,
        seeds = [D10_SINGLE_VAULT_SEED, d10_mint.key().as_ref()],
        bump,
    )]
    #[light_account(init, token, authority = [D10_SINGLE_VAULT_SEED, self.d10_mint.key(), &[params.vault_bump]], mint = d10_mint, owner = d10_vault_authority)]
    pub d10_single_vault: UncheckedAccount<'info>,

    #[account(address = COMPRESSIBLE_CONFIG_V1)]
    pub light_token_compressible_config: AccountInfo<'info>,

    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token CPI authority (required for token account creation)
    pub light_token_cpi_authority: AccountInfo<'info>,

    /// CHECK: Light token program for CPI
    pub light_token_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
