//! D6 Test: Direct Account<'info, T> type
//!
//! Tests that #[light_account(init)] works with Account<'info, T> directly (not boxed).

use anchor_lang::prelude::*;
use light_sdk_macros::LightAccounts;
use light_account::CreateAccountsProof;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D6AccountParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests #[light_account(init)] with direct Account<'info, T> type.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D6AccountParams)]
pub struct D6Account<'info> {
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
        seeds = [b"d6_account", params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d6_account_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
