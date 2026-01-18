//! D5 Test: All marker types combined
//!
//! Tests #[rentfree] + #[rentfree_token] together in one instruction struct.
//! Note: #[light_mint] is tested separately in amm_test/initialize.rs.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::RentFree;
use light_token_sdk::token::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as CTOKEN_RENT_SPONSOR};

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

pub const D5_ALL_AUTH_SEED: &[u8] = b"d5_all_auth";
pub const D5_ALL_VAULT_SEED: &[u8] = b"d5_all_vault";

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D5AllMarkersParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests all marker types in one struct:
/// - #[rentfree] for PDA account
/// - #[rentfree_token] for token vault
#[derive(Accounts, RentFree)]
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
    #[rentfree]
    pub d5_all_record: Account<'info, SinglePubkeyRecord>,

    #[account(
        mut,
        seeds = [D5_ALL_VAULT_SEED, mint.key().as_ref()],
        bump,
    )]
    #[rentfree_token(authority = [D5_ALL_AUTH_SEED])]
    pub d5_all_vault: UncheckedAccount<'info>,

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
