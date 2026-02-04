//! D9 Test: Context account seed expression
//!
//! Tests ClassifiedSeed::CtxAccount with authority.key() seeds.

use anchor_lang::prelude::*;
use light_account::{CreateAccountsProof, LightAccounts};

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9CtxAccountParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests ClassifiedSeed::CtxAccount with authority.key() seeds.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9CtxAccountParams)]
pub struct D9CtxAccount<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Authority account used in seeds
    pub authority: AccountInfo<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d9_ctx", authority.key().as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_ctx_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
