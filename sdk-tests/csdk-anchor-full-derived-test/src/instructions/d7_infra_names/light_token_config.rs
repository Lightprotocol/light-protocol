//! D7 Test: light_token naming variant
//!
//! Tests that #[light_account(token)] works with light_token infrastructure fields.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::LightAccounts;
use light_token::instruction::{LIGHT_TOKEN_CONFIG, RENT_SPONSOR as LIGHT_TOKEN_RENT_SPONSOR};

pub const D7_LIGHT_TOKEN_AUTH_SEED: &[u8] = b"d7_light_token_auth";
pub const D7_LIGHT_TOKEN_VAULT_SEED: &[u8] = b"d7_light_token_vault";

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D7LightTokenConfigParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests #[light_account(token)] with `light_token_compressible_config` and `light_token_rent_sponsor` field names.
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
    #[light_account(init, token::seeds = [D7_LIGHT_TOKEN_VAULT_SEED, self.mint.key()], token::mint = mint, token::owner = d7_light_token_authority, token::owner_seeds = [D7_LIGHT_TOKEN_AUTH_SEED])]
    pub d7_light_token_vault: UncheckedAccount<'info>,

    #[account(address = LIGHT_TOKEN_CONFIG)]
    pub light_token_compressible_config: AccountInfo<'info>,

    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token program
    pub light_token_program: AccountInfo<'info>,

    /// CHECK: Light token CPI authority
    pub light_token_cpi_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
