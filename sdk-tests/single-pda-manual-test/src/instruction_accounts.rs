//! Accounts module for single-pda-test.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::LightAccounts;

use crate::state::MinimalRecord;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreatePdaParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Minimal accounts struct for testing single PDA creation.
#[derive(Accounts)]
#[instruction(params: CreatePdaParams)]
pub struct CreatePda<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + MinimalRecord::INIT_SPACE,
        seeds = [b"minimal_record", params.owner.as_ref()],
        bump,
    )]
    // #[light_account(init)]
    pub record: Account<'info, MinimalRecord>,

    pub system_program: Program<'info, System>,
}
