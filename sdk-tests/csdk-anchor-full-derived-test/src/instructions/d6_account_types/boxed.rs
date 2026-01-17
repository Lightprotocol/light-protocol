//! D6 Test: Box<Account<'info, T>> type
//!
//! Tests that #[rentfree] works with Box<Account<'info, T>> (boxed account).
//! This exercises the Box unwrap path in seed_extraction.rs with is_boxed = true.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::RentFree;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D6BoxedParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests #[rentfree] with Box<Account<'info, T>> type.
#[derive(Accounts, RentFree)]
#[instruction(params: D6BoxedParams)]
pub struct D6Boxed<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d6_boxed", params.owner.as_ref()],
        bump,
    )]
    #[rentfree]
    pub d6_boxed_record: Box<Account<'info, SinglePubkeyRecord>>,

    pub system_program: Program<'info, System>,
}
