//! D7 Test: Multiple naming variants combined
//!
//! Tests that different naming conventions work together in one struct.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::LightAccounts;
use light_token::instruction::{LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

pub const D7_ALL_AUTH_SEED: &[u8] = b"d7_all_auth";
pub const D7_ALL_VAULT_SEED: &[u8] = b"d7_all_vault";

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D7AllNamesParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests multiple naming variants:
/// - `payer` as the fee payer field
/// - `light_token_config` for config
/// - `light_token_rent_sponsor` for light token rent sponsor
#[derive(Accounts, LightAccounts)]
#[instruction(params: D7AllNamesParams)]
pub struct D7AllNames<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Token mint
    pub mint: AccountInfo<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    #[account(
        seeds = [D7_ALL_AUTH_SEED],
        bump,
    )]
    pub d7_all_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d7_all_record", params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d7_all_record: Account<'info, SinglePubkeyRecord>,

    #[account(
        mut,
        seeds = [D7_ALL_VAULT_SEED, mint.key().as_ref()],
        bump,
    )]
    // Mark-only: seeds and owner_seeds only (no mint/owner)
    #[light_account(token::seeds = [D7_ALL_VAULT_SEED, self.mint.key()], token::owner_seeds = [D7_ALL_AUTH_SEED])]
    pub d7_all_vault: UncheckedAccount<'info>,

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
