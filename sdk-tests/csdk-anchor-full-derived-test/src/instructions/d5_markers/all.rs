//! D5 Test: All marker types combined
//!
//! Tests #[light_account(init)] + #[light_account(token)] together in one instruction struct.
//! Note: #[light_account(init)] is tested separately in amm_test/initialize.rs.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::LightAccounts;
use light_token::instruction::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as LIGHT_TOKEN_RENT_SPONSOR};

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

pub const D5_ALL_AUTH_SEED: &[u8] = b"d5_all_auth";
pub const D5_ALL_VAULT_SEED: &[u8] = b"d5_all_vault";

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D5AllMarkersParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests all marker types in one struct:
/// - #[light_account(init)] for PDA account
/// - #[light_account(token)] for token vault
#[derive(Accounts, LightAccounts)]
#[instruction(params: D5AllMarkersParams)]
pub struct D5AllMarkers<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Token mint
    pub mint: AccountInfo<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        seeds = [D5_ALL_AUTH_SEED],
        bump,
    )]
    pub d5_all_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d5_all_record", params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d5_all_record: Account<'info, SinglePubkeyRecord>,

    #[account(
        mut,
        seeds = [D5_ALL_VAULT_SEED, mint.key().as_ref()],
        bump,
    )]
    #[light_account(token::authority = [D5_ALL_AUTH_SEED])]
    pub d5_all_vault: UncheckedAccount<'info>,

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
