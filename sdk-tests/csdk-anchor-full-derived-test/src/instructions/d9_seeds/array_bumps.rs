//! D9 Test: Various seed patterns with bump
//!
//! Tests seed combinations that use the `bump` attribute
//! Note: The actual bump byte is handled by Anchor's `bump` attribute,
//! not by including &[bump] in the seeds array.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::RentFree;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

/// Constant for bump tests
pub const D9_BUMP_SEED: &[u8] = b"d9_bump";

/// String constant for .as_bytes() test
pub const D9_BUMP_STR: &str = "d9_bump_str";

// ============================================================================
// Test 1: Literal seed with bump
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D9BumpLiteralParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests literal seed with bump attribute
#[derive(Accounts, RentFree)]
#[instruction(params: D9BumpLiteralParams)]
pub struct D9BumpLiteral<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d9_bump_lit"],
        bump,
    )]
    #[rentfree]
    pub record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 2: Constant seed with bump
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D9BumpConstantParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests constant seed with bump attribute
#[derive(Accounts, RentFree)]
#[instruction(params: D9BumpConstantParams)]
pub struct D9BumpConstant<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [D9_BUMP_SEED],
        bump,
    )]
    #[rentfree]
    pub record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 3: Qualified path with .as_bytes() and bump
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D9BumpQualifiedParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests crate::path::CONST.as_bytes() with bump - the pattern that caused type inference issues
#[derive(Accounts, RentFree)]
#[instruction(params: D9BumpQualifiedParams)]
pub struct D9BumpQualified<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [crate::instructions::d9_seeds::array_bumps::D9_BUMP_STR.as_bytes()],
        bump,
    )]
    #[rentfree]
    pub record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 4: Param seed with bump
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D9BumpParamParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests params.owner.as_ref() with bump
#[derive(Accounts, RentFree)]
#[instruction(params: D9BumpParamParams)]
pub struct D9BumpParam<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d9_bump_param", params.owner.as_ref()],
        bump,
    )]
    #[rentfree]
    pub record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 5: Ctx account seed with bump
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D9BumpCtxParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests account.key().as_ref() with bump
#[derive(Accounts, RentFree)]
#[instruction(params: D9BumpCtxParams)]
pub struct D9BumpCtx<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Authority account
    pub authority: AccountInfo<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d9_bump_ctx", authority.key().as_ref()],
        bump,
    )]
    #[rentfree]
    pub record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 6: Multiple seed types with bump
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D9BumpMixedParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
    pub id: u64,
}

/// Tests literal + constant + param + bytes with bump
#[derive(Accounts, RentFree)]
#[instruction(params: D9BumpMixedParams)]
pub struct D9BumpMixed<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d9_bump_mix", D9_BUMP_SEED, params.owner.as_ref(), params.id.to_le_bytes().as_ref()],
        bump,
    )]
    #[rentfree]
    pub record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
