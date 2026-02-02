//! D9 Test: Param bytes seed expression
//!
//! Tests ClassifiedSeed::DataField with params.id.to_le_bytes() conversion.

use anchor_lang::prelude::*;
use light_sdk_macros::LightAccounts;
use light_sdk_types::interface::CreateAccountsProof;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ParamBytesParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub id: u64,
}

/// Tests ClassifiedSeed::DataField with params.id.to_le_bytes() conversion.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ParamBytesParams)]
pub struct D9ParamBytes<'info> {
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
        seeds = [b"d9_param_bytes", params.id.to_le_bytes().as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_param_bytes_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
