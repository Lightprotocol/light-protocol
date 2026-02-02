//! D9 Test: Param seed expression (Pubkey)
//!
//! Tests ClassifiedSeed::DataField with params.owner.as_ref() seeds.

use anchor_lang::prelude::*;
use light_sdk_macros::LightAccounts;
use light_account::CreateAccountsProof;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ParamParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests ClassifiedSeed::DataField with params.owner.as_ref() seeds.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ParamParams)]
pub struct D9Param<'info> {
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
        seeds = [b"d9_param", params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_param_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
