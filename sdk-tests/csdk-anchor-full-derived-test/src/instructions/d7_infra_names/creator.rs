//! D7 Test: "creator" field name variant
//!
//! Tests that #[light_account(init)] works when the payer field is named `creator` instead of `fee_payer`.

use anchor_lang::prelude::*;
use light_sdk_macros::LightAccounts;
use light_sdk_types::interface::CreateAccountsProof;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D7CreatorParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests #[light_account(init)] with `creator` field name (InfraFieldClassifier FeePayer variant).
#[derive(Accounts, LightAccounts)]
#[instruction(params: D7CreatorParams)]
pub struct D7Creator<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    #[account(
        init,
        payer = creator,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d7_creator", params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d7_creator_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
