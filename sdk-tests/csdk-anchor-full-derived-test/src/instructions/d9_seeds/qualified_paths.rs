//! D9 Test: Qualified path variations for constants
//!
//! Tests different path qualification styles for constant seeds:
//! - Bare constant: CONST
//! - crate:: prefix: crate::CONST
//! - self:: prefix: self::CONST
//! - Nested module paths

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::LightAccounts;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

/// Local constant for self:: prefix testing
pub const D9_QUALIFIED_LOCAL: &[u8] = b"d9_qualified_local";

/// Constant for crate:: prefix testing (re-exports from lib.rs)
pub const D9_QUALIFIED_CRATE: &[u8] = b"d9_qualified_crate";

// ============================================================================
// Test 1: Bare constant (no path prefix)
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9QualifiedBareParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests bare constant reference without path prefix
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9QualifiedBareParams)]
pub struct D9QualifiedBare<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [D9_QUALIFIED_LOCAL],
        bump,
    )]
    #[light_account(init)]
    pub record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 2: self:: prefix
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9QualifiedSelfParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests self:: prefix path qualification
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9QualifiedSelfParams)]
pub struct D9QualifiedSelf<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [self::D9_QUALIFIED_LOCAL],
        bump,
    )]
    #[light_account(init)]
    pub record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 3: crate:: prefix
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9QualifiedCrateParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests crate:: prefix path qualification
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9QualifiedCrateParams)]
pub struct D9QualifiedCrate<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [crate::instructions::d9_seeds::qualified_paths::D9_QUALIFIED_CRATE],
        bump,
    )]
    #[light_account(init)]
    pub record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 4: Deep nested path
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9QualifiedDeepParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests deeply nested crate path
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9QualifiedDeepParams)]
pub struct D9QualifiedDeep<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [crate::instructions::d9_seeds::D9_CONSTANT_SEED],
        bump,
    )]
    #[light_account(init)]
    pub record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 5: Mixed qualified and bare in same seeds array
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9QualifiedMixedParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests mixing qualified and bare paths in same seeds
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9QualifiedMixedParams)]
pub struct D9QualifiedMixed<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [D9_QUALIFIED_LOCAL, crate::instructions::d9_seeds::D9_CONSTANT_SEED, params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
