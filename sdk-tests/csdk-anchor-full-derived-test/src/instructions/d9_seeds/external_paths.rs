//! D9 Test: External crate path variations
//!
//! Tests paths from external crates:
//! - light_sdk_types::constants::*
//! - light_token_types::constants::*
//! - Complex nested external paths

use anchor_lang::prelude::*;
use light_account::CreateAccountsProof;
use light_sdk_macros::LightAccounts;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

// ============================================================================
// Test 1: External crate constant (light_sdk_types)
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ExternalSdkTypesParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests external crate path: light_account::constants::CPI_AUTHORITY_PDA_SEED
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ExternalSdkTypesParams)]
pub struct D9ExternalSdkTypes<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d9_ext_sdk", light_account::constants::CPI_AUTHORITY_PDA_SEED, params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_external_sdk_types_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 2: External crate constant (light_token_types)
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ExternalCtokenParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests external crate path: light_token_types::constants::POOL_SEED
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ExternalCtokenParams)]
pub struct D9ExternalCtoken<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d9_ext_ctoken", light_token_types::constants::POOL_SEED, params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_external_ctoken_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 3: Multiple external crate constants mixed
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ExternalMixedParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests multiple external crate constants mixed together
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ExternalMixedParams)]
pub struct D9ExternalMixed<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [
            light_account::constants::CPI_AUTHORITY_PDA_SEED,
            light_token_types::constants::POOL_SEED,
            params.owner.as_ref()
        ],
        bump,
    )]
    #[light_account(init)]
    pub d9_external_mixed_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 4: External constant with local constant
// ============================================================================

/// Local constant to mix with external
pub const D9_EXTERNAL_LOCAL: &[u8] = b"d9_ext_local";

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ExternalWithLocalParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests external constant combined with local constant
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ExternalWithLocalParams)]
pub struct D9ExternalWithLocal<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [D9_EXTERNAL_LOCAL, light_account::constants::RENT_SPONSOR_SEED, params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_external_with_local_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 5: External constant with bump
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ExternalBumpParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests external constant path with bump attribute
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ExternalBumpParams)]
pub struct D9ExternalBump<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [light_token_interface::COMPRESSED_MINT_SEED, params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_external_bump_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 6: Re-exported external constant
// ============================================================================

/// Re-export from external crate for path testing
pub use light_account::CPI_AUTHORITY_PDA_SEED as REEXPORTED_SEED;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ExternalReexportParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests re-exported external constant
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ExternalReexportParams)]
pub struct D9ExternalReexport<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [REEXPORTED_SEED],
        bump,
    )]
    #[light_account(init)]
    pub d9_external_reexport_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
