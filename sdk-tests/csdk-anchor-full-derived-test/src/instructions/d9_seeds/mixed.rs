//! D9 Test: Mixed seed expression types
//!
//! Tests multiple seed types combined: literal + ctx_account + param.

use anchor_lang::prelude::*;
use light_sdk_macros::LightAccounts;
use light_sdk_types::interface::CreateAccountsProof;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9MixedParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests multiple seed types combined: literal + ctx_account + param.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9MixedParams)]
pub struct D9Mixed<'info> {
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
        seeds = [b"d9_mixed", authority.key().as_ref(), params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_mixed_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
