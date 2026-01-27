//! Accounts module for single-pda-test.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;

use crate::pda::MinimalRecord;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreatePdaParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
    pub nonce: u64,
}

/// Minimal accounts struct for testing single PDA creation.
#[derive(Accounts)] // LightAccounts
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
        seeds = [b"minimal_record", params.owner.as_ref(), &params.nonce.to_le_bytes()],
        bump,
    )]
    // #[light_account(init)]
    pub record: Account<'info, MinimalRecord>,

    pub system_program: Program<'info, System>,
}
