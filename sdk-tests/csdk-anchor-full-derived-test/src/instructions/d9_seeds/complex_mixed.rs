//! D9 Test: Complex multi-seed combinations
//!
//! Tests multi-seed combinations with 3+ seeds:
//! - Various type combinations
//! - Different orderings
//! - Maximum seed complexity

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::LightAccounts;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

/// Constants for complex tests
pub const D9_COMPLEX_V1: &[u8] = b"v1";
pub const D9_COMPLEX_PREFIX: &[u8] = b"prefix";
pub const D9_COMPLEX_NAMESPACE: &str = "namespace";

// ============================================================================
// Test 1: Three seeds - literal + constant + param
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ComplexThreeParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests 3 seeds: literal + constant + param.as_ref()
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ComplexThreeParams)]
pub struct D9ComplexThree<'info> {
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
        seeds = [b"d9_complex3", D9_COMPLEX_PREFIX, params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_complex_three_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 2: Four seeds - mixed types
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ComplexFourParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
    pub id: u64,
}

/// Tests 4 seeds: version + namespace + param + bytes
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ComplexFourParams)]
pub struct D9ComplexFour<'info> {
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
        seeds = [D9_COMPLEX_V1, D9_COMPLEX_NAMESPACE.as_bytes(), params.owner.as_ref(), params.id.to_le_bytes().as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_complex_four_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 3: Five seeds - ctx account included
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ComplexFiveParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
    pub id: u64,
}

/// Tests 5 seeds with context account
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ComplexFiveParams)]
pub struct D9ComplexFive<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Authority for seeds
    pub authority: AccountInfo<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [D9_COMPLEX_V1, D9_COMPLEX_NAMESPACE.as_bytes(), authority.key().as_ref(), params.owner.as_ref(), params.id.to_le_bytes().as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_complex_five_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 4: Qualified paths mixed with local
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ComplexQualifiedMixParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests qualified crate paths mixed with local constants
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ComplexQualifiedMixParams)]
pub struct D9ComplexQualifiedMix<'info> {
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
        seeds = [crate::instructions::d9_seeds::complex_mixed::D9_COMPLEX_V1, D9_COMPLEX_PREFIX, params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_complex_qualified_mix_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 5: Function call + other seeds
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ComplexFuncParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub key_a: Pubkey,
    pub key_b: Pubkey,
    pub id: u64,
}

/// Tests function call combined with other seed types
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ComplexFuncParams)]
pub struct D9ComplexFunc<'info> {
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
        seeds = [D9_COMPLEX_V1, crate::max_key(&params.key_a, &params.key_b).as_ref(), params.id.to_le_bytes().as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_complex_func_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 6: All qualified paths
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ComplexAllQualifiedParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests all paths being fully qualified
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ComplexAllQualifiedParams)]
pub struct D9ComplexAllQualified<'info> {
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
            crate::instructions::d9_seeds::complex_mixed::D9_COMPLEX_V1,
            crate::instructions::d9_seeds::complex_mixed::D9_COMPLEX_NAMESPACE.as_bytes(),
            params.owner.as_ref()
        ],
        bump,
    )]
    #[light_account(init)]
    pub d9_complex_all_qualified_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 7: Static function (program ID) as seed
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ComplexProgramIdParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests using crate::ID (program ID) as a seed element
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ComplexProgramIdParams)]
pub struct D9ComplexProgramId<'info> {
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
        seeds = [b"d9_progid", crate::ID.as_ref(), params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_complex_program_id_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 8: Static id() function call as seed
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ComplexIdFuncParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests using crate::id() function call as a seed element
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ComplexIdFuncParams)]
pub struct D9ComplexIdFunc<'info> {
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
        seeds = [b"d9_idfunc", crate::id().as_ref(), params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_complex_id_func_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
