//! Accounts module for zero-copy account instruction.

use anchor_lang::prelude::*;
use light_account::CreateAccountsProof;

use super::state::ZeroCopyRecord;

/// Parameters for creating a zero-copy compressible PDA.
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CreateZeroCopyParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
    pub value: u64,
    pub name: String,
}

/// Accounts struct for creating a zero-copy compressible PDA.
/// Uses AccountLoader<'info, T> for zero-copy access pattern.
#[derive(Accounts)]
#[instruction(params: CreateZeroCopyParams)]
pub struct CreateZeroCopy<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config PDA for this program.
    pub compression_config: AccountInfo<'info>,

    /// Zero-copy account using AccountLoader.
    /// Space: 8 (discriminator) + ZeroCopyRecord::INIT_SPACE (64) = 72 bytes
    #[account(
        init,
        payer = fee_payer,
        space = 8 + ZeroCopyRecord::INIT_SPACE,
        seeds = [b"zero_copy", params.owner.as_ref(), params.name.as_bytes()],
        bump,
    )]
    pub record: AccountLoader<'info, ZeroCopyRecord>,

    pub system_program: Program<'info, System>,
}
