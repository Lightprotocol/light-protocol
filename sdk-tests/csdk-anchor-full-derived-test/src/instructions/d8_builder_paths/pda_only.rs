//! D8 Test: Only #[rentfree] fields (no token accounts)
//!
//! Tests the `generate_pre_init_pdas_only` code path where only PDA accounts
//! are marked with #[rentfree], without any token accounts.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::RentFree;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D8PdaOnlyParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests builder path with only PDA accounts (no token accounts).
#[derive(Accounts, RentFree)]
#[instruction(params: D8PdaOnlyParams)]
pub struct D8PdaOnly<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d8_pda_only", params.owner.as_ref()],
        bump,
    )]
    #[rentfree]
    pub d8_pda_only_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
