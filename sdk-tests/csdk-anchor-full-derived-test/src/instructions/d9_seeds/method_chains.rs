//! D9 Test: Method call variations
//!
//! Tests different method call patterns on seeds:
//! - .as_ref() on Pubkey
//! - .as_bytes() on string constants
//! - .to_le_bytes().as_ref() on numeric types
//! - .to_be_bytes().as_ref() on numeric types
//! - Method chains on qualified paths

use anchor_lang::prelude::*;
use light_sdk_macros::LightAccounts;
use light_account::CreateAccountsProof;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

/// String constant for .as_bytes() testing
pub const D9_METHOD_STR: &str = "d9_method_str";

/// Byte slice constant for .as_ref() testing
pub const D9_METHOD_BYTES: &[u8] = b"d9_method_bytes";

// ============================================================================
// Test 1: Constant with .as_ref()
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9MethodAsRefParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests constant.as_ref() method call
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9MethodAsRefParams)]
pub struct D9MethodAsRef<'info> {
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
        seeds = [D9_METHOD_BYTES.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_method_as_ref_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 2: String constant with .as_bytes()
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9MethodAsBytesParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests string_constant.as_bytes() method call
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9MethodAsBytesParams)]
pub struct D9MethodAsBytes<'info> {
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
        seeds = [D9_METHOD_STR.as_bytes()],
        bump,
    )]
    #[light_account(init)]
    pub d9_method_as_bytes_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 3: Qualified path with .as_bytes()
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9MethodQualifiedAsBytesParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests crate::path::CONST.as_bytes() - the pattern that caused type inference issues
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9MethodQualifiedAsBytesParams)]
pub struct D9MethodQualifiedAsBytes<'info> {
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
        seeds = [crate::instructions::d9_seeds::method_chains::D9_METHOD_STR.as_bytes()],
        bump,
    )]
    #[light_account(init)]
    pub d9_method_qualified_as_bytes_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 4: Param with .to_le_bytes().as_ref()
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9MethodToLeBytesParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub id: u64,
}

/// Tests params.field.to_le_bytes().as_ref() chain
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9MethodToLeBytesParams)]
pub struct D9MethodToLeBytes<'info> {
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
        seeds = [b"d9_le", params.id.to_le_bytes().as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_method_to_le_bytes_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 5: Param with .to_be_bytes().as_ref()
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9MethodToBeBytesParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub id: u64,
}

/// Tests params.field.to_be_bytes().as_ref() chain
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9MethodToBeBytesParams)]
pub struct D9MethodToBeBytes<'info> {
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
        seeds = [b"d9_be", params.id.to_be_bytes().as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_method_to_be_bytes_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 6: Mixed methods in same seeds array
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9MethodMixedParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
    pub id: u64,
}

/// Tests mixing different method calls in same seeds
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9MethodMixedParams)]
pub struct D9MethodMixed<'info> {
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
        seeds = [D9_METHOD_STR.as_bytes(), params.owner.as_ref(), params.id.to_le_bytes().as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_method_mixed_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
