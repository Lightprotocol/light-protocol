//! D8 Test: Multiple #[light_account(init)] fields with different state types
//!
//! Tests the builder path with multiple #[light_account(init)] fields of different state types.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::LightAccounts;

use crate::state::{
    d1_field_types::single_pubkey::SinglePubkeyRecord,
    d2_compress_as::multiple::MultipleCompressAsRecord,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D8AllParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests builder path with multiple #[light_account(init)] fields of different state types.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D8AllParams)]
pub struct D8All<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d8_all_single", params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d8_all_single: Account<'info, SinglePubkeyRecord>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + MultipleCompressAsRecord::INIT_SPACE,
        seeds = [b"d8_all_multi", params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d8_all_multi: Box<Account<'info, MultipleCompressAsRecord>>,

    pub system_program: Program<'info, System>,
}
