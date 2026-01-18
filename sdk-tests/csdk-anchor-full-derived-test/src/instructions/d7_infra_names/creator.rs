//! D7 Test: "creator" field name variant
//!
//! Tests that #[rentfree] works when the payer field is named `creator` instead of `fee_payer`.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::RentFree;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D7CreatorParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests #[rentfree] with `creator` field name (InfraFieldClassifier FeePayer variant).
#[derive(Accounts, RentFree)]
#[instruction(params: D7CreatorParams)]
pub struct D7Creator<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = creator,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d7_creator", params.owner.as_ref()],
        bump,
    )]
    #[rentfree]
    pub d7_creator_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
