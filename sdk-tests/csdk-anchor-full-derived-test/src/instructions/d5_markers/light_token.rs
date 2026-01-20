//! D5 Test: #[light_account(token)] attribute with authority seeds
//!
//! Tests that the #[light_account(token, authority = [...])] attribute works correctly
//! for token accounts that need custom authority derivation.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::LightAccounts;
use light_token::instruction::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as LIGHT_TOKEN_RENT_SPONSOR};

pub const D5_VAULT_AUTH_SEED: &[u8] = b"d5_vault_auth";
pub const D5_VAULT_SEED: &[u8] = b"d5_vault";

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D5LightTokenParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub vault_bump: u8,
}

/// Tests #[light_account(token, authority = [...])] attribute compilation.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D5LightTokenParams)]
pub struct D5LightToken<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Token mint
    pub mint: AccountInfo<'info>,

    #[account(
        seeds = [D5_VAULT_AUTH_SEED],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [D5_VAULT_SEED, mint.key().as_ref()],
        bump,
    )]
    #[light_account(token, authority = [D5_VAULT_AUTH_SEED])]
    pub d5_token_vault: UncheckedAccount<'info>,

    #[account(address = COMPRESSIBLE_CONFIG_V1)]
    pub light_token_compressible_config: AccountInfo<'info>,

    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token program
    pub light_token_program: AccountInfo<'info>,

    /// CHECK: Light token CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
