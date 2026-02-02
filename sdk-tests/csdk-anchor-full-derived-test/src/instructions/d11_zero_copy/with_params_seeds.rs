//! D11 Test: Zero-copy with Params-only Seeds
//!
//! Tests `#[light_account(init, zero_copy)]` with params-only seed expressions.
//! Verifies that seed fields not present on the struct work correctly.

use anchor_lang::prelude::*;
use light_account::CreateAccountsProof;
use light_sdk_macros::LightAccounts;

use crate::state::d11_zero_copy::ZcWithParamsRecord;

/// Seed for the zero-copy record PDA with params-only seeds.
pub const D11_ZC_PARAMS_SEED: &[u8] = b"d11_zc_params";

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D11ZcWithParamsSeedsParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
    /// Category ID - used in seeds but not stored on ZcWithParamsRecord.
    pub category_id: u64,
}

/// Tests `#[light_account(init, zero_copy)]` with params.category_id in seeds.
/// The category_id is used as a seed component but is not stored on the struct.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D11ZcWithParamsSeedsParams)]
pub struct D11ZcWithParamsSeeds<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config PDA.
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    /// Zero-copy PDA with params.owner and params.category_id in seeds.
    #[account(
        init,
        payer = fee_payer,
        space = 8 + core::mem::size_of::<ZcWithParamsRecord>(),
        seeds = [D11_ZC_PARAMS_SEED, params.owner.as_ref(), &params.category_id.to_le_bytes()],
        bump,
    )]
    #[light_account(init, zero_copy)]
    pub zc_params_record: AccountLoader<'info, ZcWithParamsRecord>,

    pub system_program: Program<'info, System>,
}
