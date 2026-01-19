//! D9 Test: Function call seed expression
//!
//! Tests ClassifiedSeed::FunctionCall with max_key(&a, &b) seeds.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::LightAccounts;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D9FunctionCallParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub key_a: Pubkey,
    pub key_b: Pubkey,
}

/// Tests ClassifiedSeed::FunctionCall with max_key(&a, &b) seeds.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9FunctionCallParams)]
pub struct D9FunctionCall<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d9_func", crate::max_key(&params.key_a, &params.key_b).as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_func_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
