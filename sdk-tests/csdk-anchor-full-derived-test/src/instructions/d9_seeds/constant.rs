//! D9 Test: Constant seed expression
//!
//! Tests ClassifiedSeed::Constant with constant identifier seeds.

use anchor_lang::prelude::*;
use light_account::{CreateAccountsProof, LightAccounts};

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

pub const D9_CONSTANT_SEED: &[u8] = b"d9_constant";

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ConstantParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests ClassifiedSeed::Constant with constant identifier seeds.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ConstantParams)]
pub struct D9Constant<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [D9_CONSTANT_SEED],
        bump,
    )]
    #[light_account(init)]
    pub d9_constant_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
