//! D7 Test: "ctoken_config" naming variant
//!
//! Tests that #[rentfree_token] works with alternative naming for ctoken infrastructure fields.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::RentFree;
use light_token_sdk::token::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as CTOKEN_RENT_SPONSOR};

pub const D7_CTOKEN_AUTH_SEED: &[u8] = b"d7_ctoken_auth";
pub const D7_CTOKEN_VAULT_SEED: &[u8] = b"d7_ctoken_vault";

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D7CtokenConfigParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests #[rentfree_token] with `ctoken_compressible_config` and `ctoken_rent_sponsor` field names.
#[derive(Accounts, RentFree)]
#[instruction(params: D7CtokenConfigParams)]
pub struct D7CtokenConfig<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Token mint
    pub mint: AccountInfo<'info>,

    #[account(
        seeds = [D7_CTOKEN_AUTH_SEED],
        bump,
    )]
    pub d7_ctoken_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [D7_CTOKEN_VAULT_SEED, mint.key().as_ref()],
        bump,
    )]
    #[rentfree_token(authority = [D7_CTOKEN_AUTH_SEED])]
    pub d7_ctoken_vault: UncheckedAccount<'info>,

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
