//! D8 Test: Multiple #[rentfree] fields
//!
//! Tests the builder path with multiple #[rentfree] PDA accounts of the same type.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::RentFree;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D8MultiRentfreeParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
    pub id1: u64,
    pub id2: u64,
}

/// Tests builder path with multiple #[rentfree] fields of the same type.
#[derive(Accounts, RentFree)]
#[instruction(params: D8MultiRentfreeParams)]
pub struct D8MultiRentfree<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d8_multi_1", params.owner.as_ref(), params.id1.to_le_bytes().as_ref()],
        bump,
    )]
    #[rentfree]
    pub d8_multi_record1: Account<'info, SinglePubkeyRecord>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d8_multi_2", params.owner.as_ref(), params.id2.to_le_bytes().as_ref()],
        bump,
    )]
    #[rentfree]
    pub d8_multi_record2: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
