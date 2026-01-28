//! D11 Test: Multiple Zero-copy PDAs
//!
//! Tests `#[light_account(init, zero_copy)]` with multiple zero-copy accounts in one instruction.
//! Verifies that the macro handles multiple AccountLoader fields correctly.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::LightAccounts;

use crate::state::d11_zero_copy::ZcBasicRecord;

/// Seed for the first zero-copy record PDA.
pub const D11_ZC1_SEED: &[u8] = b"d11_zc1";
/// Seed for the second zero-copy record PDA.
pub const D11_ZC2_SEED: &[u8] = b"d11_zc2";

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D11MultipleZcParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests `#[light_account(init, zero_copy)]` with multiple zero-copy PDAs.
/// Both accounts use the same struct type but different seeds.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D11MultipleZcParams)]
pub struct D11MultipleZc<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config PDA.
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    /// First zero-copy PDA record.
    #[account(
        init,
        payer = fee_payer,
        space = 8 + core::mem::size_of::<ZcBasicRecord>(),
        seeds = [D11_ZC1_SEED, params.owner.as_ref()],
        bump,
    )]
    #[light_account(init, zero_copy)]
    pub zc_record_1: AccountLoader<'info, ZcBasicRecord>,

    /// Second zero-copy PDA record.
    #[account(
        init,
        payer = fee_payer,
        space = 8 + core::mem::size_of::<ZcBasicRecord>(),
        seeds = [D11_ZC2_SEED, params.owner.as_ref()],
        bump,
    )]
    #[light_account(init, zero_copy)]
    pub zc_record_2: AccountLoader<'info, ZcBasicRecord>,

    pub system_program: Program<'info, System>,
}
