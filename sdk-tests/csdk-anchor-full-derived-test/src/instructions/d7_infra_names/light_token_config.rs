//! D7 Test: light_token naming variant (mark-only mode)
//!
//! Tests mark-only token accounts with light_token infrastructure fields.
//! The derive generates seed structs and variant enums for decompression,
//! but the token account is created manually via CreateTokenAccountCpi.

use anchor_lang::prelude::*;
use light_account::{LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};
use light_sdk_macros::LightAccounts;

pub const D7_LIGHT_TOKEN_AUTH_SEED: &[u8] = b"d7_light_token_auth";
pub const D7_LIGHT_TOKEN_VAULT_SEED: &[u8] = b"d7_light_token_vault";

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D7LightTokenConfigParams {
    pub vault_bump: u8,
}

/// Tests mark-only token account with `light_token_config`
/// and `light_token_rent_sponsor` field names.
/// #[derive(LightAccounts)] generates no-op LightPreInit/LightFinalize impls.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D7LightTokenConfigParams)]
pub struct D7LightTokenConfig<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Token mint
    pub mint: AccountInfo<'info>,

    #[account(
        seeds = [D7_LIGHT_TOKEN_AUTH_SEED],
        bump,
    )]
    pub d7_light_token_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [D7_LIGHT_TOKEN_VAULT_SEED, mint.key().as_ref()],
        bump,
    )]
    // Mark-only: seeds and owner_seeds only (no mint/owner)
    // Token vault - created manually via CreateTokenAccountCpi in handler
    #[light_account(token::seeds = [D7_LIGHT_TOKEN_VAULT_SEED, self.mint.key()], token::owner_seeds = [D7_LIGHT_TOKEN_AUTH_SEED])]
    pub d7_light_token_vault: UncheckedAccount<'info>,

    #[account(address = LIGHT_TOKEN_CONFIG)]
    pub light_token_config: AccountInfo<'info>,

    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token program
    pub light_token_program: AccountInfo<'info>,

    /// CHECK: Light token CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
