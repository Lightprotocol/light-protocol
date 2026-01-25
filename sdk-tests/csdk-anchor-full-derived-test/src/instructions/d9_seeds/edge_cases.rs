//! D9 Test: Edge cases and boundary conditions
//!
//! Tests boundary conditions:
//! - Empty literal
//! - Single byte constant
//! - Single letter constant names
//! - Constant names with digits
//! - Leading underscore constants
//! - Many literals in same seeds array

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::LightAccounts;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

/// Single letter constant
pub const A: &[u8] = b"a";

/// Constant with digits
pub const SEED_123: &[u8] = b"seed123";

/// Leading underscore constant
pub const _UNDERSCORE_CONST: &[u8] = b"underscore";

/// Single byte constant
pub const D9_SINGLE_BYTE: &[u8] = b"x";

// ============================================================================
// Test 1: Minimal literal (single character)
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9EdgeEmptyParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests minimal byte literal seed
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9EdgeEmptyParams)]
pub struct D9EdgeEmpty<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [&b"d9_edge_empty"[..], &b"_"[..], params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_edge_empty_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 2: Single byte constant
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9EdgeSingleByteParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests single byte constant
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9EdgeSingleByteParams)]
pub struct D9EdgeSingleByte<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [D9_SINGLE_BYTE],
        bump,
    )]
    #[light_account(init)]
    pub d9_edge_single_byte_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 3: Single letter constant name
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9EdgeSingleLetterParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests single letter constant name (A)
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9EdgeSingleLetterParams)]
pub struct D9EdgeSingleLetter<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [A],
        bump,
    )]
    #[light_account(init)]
    pub d9_edge_single_letter_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 4: Constant name with digits
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9EdgeDigitsParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests constant name containing digits (SEED_123)
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9EdgeDigitsParams)]
pub struct D9EdgeDigits<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [SEED_123],
        bump,
    )]
    #[light_account(init)]
    pub d9_edge_digits_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 5: Leading underscore constant
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9EdgeUnderscoreParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests leading underscore constant name
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9EdgeUnderscoreParams)]
pub struct D9EdgeUnderscore<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [_UNDERSCORE_CONST],
        bump,
    )]
    #[light_account(init)]
    pub d9_edge_underscore_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 6: Many literals in same seeds array
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9EdgeManyLiteralsParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests many byte literals in same seeds array
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9EdgeManyLiteralsParams)]
pub struct D9EdgeManyLiterals<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"a", b"b", b"c", b"d", b"e"],
        bump,
    )]
    #[light_account(init)]
    pub d9_edge_many_literals_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 7: Mixed edge cases in one struct
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9EdgeMixedParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests mixing various edge case constants
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9EdgeMixedParams)]
pub struct D9EdgeMixed<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [A, SEED_123, _UNDERSCORE_CONST, params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_edge_mixed_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
