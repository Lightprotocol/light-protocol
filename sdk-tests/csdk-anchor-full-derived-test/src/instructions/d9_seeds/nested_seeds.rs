//! D9 Test: Nested seed expressions
//!
//! Tests deeply nested seed access patterns:
//! - params.nested.field access
//! - params.array[index].as_slice() array indexing
//! - Complex nested struct paths

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::LightAccounts;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

// ============================================================================
// Nested structs for testing
// ============================================================================

/// Inner nested struct
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InnerNested {
    pub owner: Pubkey,
    pub id: u64,
}

/// Outer nested struct containing inner
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct OuterNested {
    pub array: [u8; 16],
    pub nested: InnerNested,
}

// ============================================================================
// Test 1: Simple nested struct access
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D9NestedSimpleParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub nested: InnerNested,
}

/// Tests params.nested.owner.as_ref() pattern
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9NestedSimpleParams)]
pub struct D9NestedSimple<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d9_nested_simple", params.nested.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 2: Double nested struct access
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D9NestedDoubleParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub outer: OuterNested,
}

/// Tests params.outer.nested.owner.as_ref() pattern (double nested)
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9NestedDoubleParams)]
pub struct D9NestedDouble<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d9_nested_double", params.outer.nested.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 3: Nested array field access
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D9NestedArrayFieldParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub outer: OuterNested,
}

/// Tests params.outer.array as seed (array field in nested struct)
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9NestedArrayFieldParams)]
pub struct D9NestedArrayField<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d9_nested_array", params.outer.array.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 4: Array indexing - params.arrays[2].as_slice()
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D9ArrayIndexParams {
    pub create_accounts_proof: CreateAccountsProof,
    /// 2D array: 10 arrays of 16 bytes each
    pub arrays: [[u8; 16]; 10],
}

/// Tests params.arrays[2].as_slice() pattern (array indexing)
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ArrayIndexParams)]
pub struct D9ArrayIndex<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d9_array_idx", params.arrays[2].as_slice()],
        bump,
    )]
    #[light_account(init)]
    pub record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 5: Combined nested struct + bytes conversion
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D9NestedBytesParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub nested: InnerNested,
}

/// Tests params.nested.id.to_le_bytes().as_ref() pattern
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9NestedBytesParams)]
pub struct D9NestedBytes<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d9_nested_bytes", params.nested.id.to_le_bytes().as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 6: Multiple nested seeds combined
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D9NestedCombinedParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub outer: OuterNested,
}

/// Tests combining multiple nested accessors in seeds array
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9NestedCombinedParams)]
pub struct D9NestedCombined<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [
            b"d9_nested_combined",
            params.outer.array.as_ref(),
            params.outer.nested.owner.as_ref()
        ],
        bump,
    )]
    #[light_account(init)]
    pub record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
