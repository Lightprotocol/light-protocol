//! D11 Test: Zero-copy with Context Seeds
//!
//! Tests `#[light_account(init, zero_copy)]` with ctx.accounts.* in seed expressions.
//! Verifies that context account seeds work correctly with zero-copy accounts.

use anchor_lang::prelude::*;
use light_sdk_macros::LightAccounts;
use light_sdk_types::interface::CreateAccountsProof;

use crate::state::d11_zero_copy::ZcWithSeedsRecord;

/// Seed for the zero-copy record PDA with ctx seeds.
pub const D11_ZC_CTX_SEED: &[u8] = b"d11_zc_ctx";

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D11ZcWithCtxSeedsParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests `#[light_account(init, zero_copy)]` with ctx.accounts.authority in seeds.
/// The authority account is used as a seed component for the PDA.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D11ZcWithCtxSeedsParams)]
pub struct D11ZcWithCtxSeeds<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config PDA.
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    /// Authority account referenced in seeds.
    pub authority: Signer<'info>,

    /// Zero-copy PDA with ctx.accounts.authority in seeds.
    #[account(
        init,
        payer = fee_payer,
        space = 8 + core::mem::size_of::<ZcWithSeedsRecord>(),
        seeds = [D11_ZC_CTX_SEED, authority.key().as_ref()],
        bump,
    )]
    #[light_account(init, zero_copy)]
    pub zc_ctx_record: AccountLoader<'info, ZcWithSeedsRecord>,

    pub system_program: Program<'info, System>,
}
