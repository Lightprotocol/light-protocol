//! D6 Test: Direct Account<'info, T> type
//!
//! Tests that #[rentfree] works with Account<'info, T> directly (not boxed).

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::RentFree;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D6AccountParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests #[rentfree] with direct Account<'info, T> type.
#[derive(Accounts, RentFree)]
#[instruction(params: D6AccountParams)]
pub struct D6Account<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d6_account", params.owner.as_ref()],
        bump,
    )]
    #[rentfree]
    pub d6_account_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
