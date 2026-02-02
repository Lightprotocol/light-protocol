//! D6 Test: Both Account<'info, T> and Box<Account<'info, T>> together
//!
//! Tests that both account type variants work in the same struct.

use anchor_lang::prelude::*;
use light_sdk_macros::LightAccounts;
use light_account::CreateAccountsProof;

use crate::state::{
    d1_field_types::single_pubkey::SinglePubkeyRecord,
    d2_compress_as::multiple::MultipleCompressAsRecord,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D6AllParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests both account types in one struct:
/// - Account<'info, T> (direct)
/// - Box<Account<'info, T>> (boxed)
#[derive(Accounts, LightAccounts)]
#[instruction(params: D6AllParams)]
pub struct D6All<'info> {
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
        seeds = [b"d6_all_direct", params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d6_all_direct: Account<'info, SinglePubkeyRecord>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + MultipleCompressAsRecord::INIT_SPACE,
        seeds = [b"d6_all_boxed", params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d6_all_boxed: Box<Account<'info, MultipleCompressAsRecord>>,

    pub system_program: Program<'info, System>,
}
