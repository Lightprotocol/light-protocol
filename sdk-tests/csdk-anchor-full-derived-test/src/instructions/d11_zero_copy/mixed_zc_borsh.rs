//! D11 Test: Mixed Zero-copy and Borsh accounts
//!
//! Tests `#[light_account(init, zero_copy)]` alongside regular `#[light_account(init)]` (Borsh).
//! Verifies that mixed serialization types work together in the same instruction.

use anchor_lang::prelude::*;
use light_account::{CreateAccountsProof, LightAccounts};

use crate::state::{
    d11_zero_copy::ZcBasicRecord, d1_field_types::single_pubkey::SinglePubkeyRecord,
};

/// Seed for the zero-copy record PDA.
pub const D11_ZC_MIXED_SEED: &[u8] = b"d11_zc_mixed";
/// Seed for the Borsh record PDA.
pub const D11_BORSH_MIXED_SEED: &[u8] = b"d11_borsh_mixed";

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D11MixedZcBorshParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests mixed zero-copy and Borsh accounts in the same instruction.
/// zc_record uses AccountLoader (zero-copy), borsh_record uses Account (Borsh).
#[derive(Accounts, LightAccounts)]
#[instruction(params: D11MixedZcBorshParams)]
pub struct D11MixedZcBorsh<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config PDA.
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    /// Zero-copy account using AccountLoader.
    #[account(
        init,
        payer = fee_payer,
        space = 8 + core::mem::size_of::<ZcBasicRecord>(),
        seeds = [D11_ZC_MIXED_SEED, params.owner.as_ref()],
        bump,
    )]
    #[light_account(init, zero_copy)]
    pub zc_mixed_record: AccountLoader<'info, ZcBasicRecord>,

    /// Regular Borsh account using Account.
    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [D11_BORSH_MIXED_SEED, params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub borsh_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
