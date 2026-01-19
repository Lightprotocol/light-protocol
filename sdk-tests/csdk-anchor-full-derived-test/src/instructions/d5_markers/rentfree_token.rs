//! D5 Test: #[rentfree_token] attribute with authority seeds
//!
//! Tests that the #[rentfree_token(authority = [...])] attribute works correctly
//! for token accounts that need custom authority derivation.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::LightAccounts;
use light_token_sdk::token::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as CTOKEN_RENT_SPONSOR};

pub const D5_VAULT_AUTH_SEED: &[u8] = b"d5_vault_auth";
pub const D5_VAULT_SEED: &[u8] = b"d5_vault";

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D5RentfreeTokenParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub vault_bump: u8,
}

/// Tests #[rentfree_token(authority = [...])] attribute compilation.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D5RentfreeTokenParams)]
pub struct D5RentfreeToken<'info> {
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
    #[rentfree_token(authority = [D5_VAULT_AUTH_SEED])]
    pub d5_token_vault: UncheckedAccount<'info>,

    #[account(address = COMPRESSIBLE_CONFIG_V1)]
    pub ctoken_compressible_config: AccountInfo<'info>,

    #[account(mut, address = CTOKEN_RENT_SPONSOR)]
    pub ctoken_rent_sponsor: AccountInfo<'info>,

    /// CHECK: CToken program
    pub light_token_program: AccountInfo<'info>,

    /// CHECK: CToken CPI authority
    pub ctoken_cpi_authority: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
