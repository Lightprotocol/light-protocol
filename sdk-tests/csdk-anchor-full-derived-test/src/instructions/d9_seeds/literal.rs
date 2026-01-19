//! D9 Test: Literal seed expression
//!
//! Tests ClassifiedSeed::Literal with byte literal seeds.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::LightAccounts;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D9LiteralParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests ClassifiedSeed::Literal with byte literal seeds.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9LiteralParams)]
pub struct D9Literal<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d9_literal_record"],
        bump,
    )]
    #[light_account(init)]
    pub d9_literal_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
