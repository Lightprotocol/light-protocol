//! D9 Test: Instruction data naming variations
//!
//! Tests different naming patterns for instruction parameters:
//! - Standard "params" naming with various field access patterns
//! - Nested field access like params.config.owner
//! - Multiple data fields in seeds
//!
//! Note: The LightAccounts macro requires params.create_accounts_proof to exist,
//! so we test naming variations within the seed expressions, not the param struct name.

use anchor_lang::prelude::*;
use light_sdk_macros::LightAccounts;
use light_sdk_types::interface::CreateAccountsProof;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

// ============================================================================
// Test 1: Standard params with single Pubkey field
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9SinglePubkeyParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests standard params.owner.as_ref() pattern
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9SinglePubkeyParams)]
pub struct D9InstrSinglePubkey<'info> {
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
        seeds = [b"instr_single", params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_instr_single_pubkey_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 2: Params with u64 field requiring to_le_bytes
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9U64Params {
    pub create_accounts_proof: CreateAccountsProof,
    pub amount: u64,
}

/// Tests params.amount.to_le_bytes() pattern
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9U64Params)]
pub struct D9InstrU64<'info> {
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
        seeds = [b"instr_u64_", params.amount.to_le_bytes().as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_instr_u64_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 3: Multiple data fields in seeds
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9MultiFieldParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
    pub amount: u64,
}

/// Tests multiple params fields: owner + amount
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9MultiFieldParams)]
pub struct D9InstrMultiField<'info> {
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
        seeds = [b"instr_multi", params.owner.as_ref(), &params.amount.to_le_bytes()],
        bump,
    )]
    #[light_account(init)]
    pub d9_instr_multi_field_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 4: Mixed params and ctx account in seeds
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9MixedCtxParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub data_key: Pubkey,
}

/// Tests mixing params.data_key with ctx.authority
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9MixedCtxParams)]
pub struct D9InstrMixedCtx<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub authority: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"instr_mixed", authority.key().as_ref(), params.data_key.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_instr_mixed_ctx_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 5: Three data fields
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9TripleParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub key_a: Pubkey,
    pub value_b: u64,
    pub flag_c: u8,
}

/// Tests three params fields with different types
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9TripleParams)]
pub struct D9InstrTriple<'info> {
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
        seeds = [b"instr_triple", params.key_a.as_ref(), params.value_b.to_le_bytes().as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_instr_triple_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 6: to_be_bytes conversion
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9BigEndianParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub value: u64,
}

/// Tests params.value.to_be_bytes() (big endian)
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9BigEndianParams)]
pub struct D9InstrBigEndian<'info> {
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
        seeds = [b"instr_be", &params.value.to_be_bytes()],
        bump,
    )]
    #[light_account(init)]
    pub d9_instr_big_endian_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 7: Multiple u64 fields
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9MultiU64Params {
    pub create_accounts_proof: CreateAccountsProof,
    pub id: u64,
    pub counter: u64,
}

/// Tests multiple u64 fields with to_le_bytes
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9MultiU64Params)]
pub struct D9InstrMultiU64<'info> {
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
        seeds = [b"multi_u64", params.id.to_le_bytes().as_ref(), params.counter.to_le_bytes().as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_instr_multi_u64_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 8: Pubkey with as_ref chained
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ChainedAsRefParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub key: Pubkey,
}

/// Tests params.key.as_ref() explicitly chained
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ChainedAsRefParams)]
pub struct D9InstrChainedAsRef<'info> {
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
        seeds = [b"instr_chain", params.key.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_instr_chained_as_ref_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 9: Constant mixed with params
// ============================================================================

/// Local seed constant
pub const D9_INSTR_SEED: &[u8] = b"d9_instr_const";

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ConstMixedParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests constant + params.owner in seeds
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ConstMixedParams)]
pub struct D9InstrConstMixed<'info> {
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
        seeds = [D9_INSTR_SEED, params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_instr_const_mixed_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 10: Complex mixed - literal + constant + ctx + params
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ComplexMixedParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub data_owner: Pubkey,
    pub data_amount: u64,
}

/// Tests complex mix: literal + authority + params.data_owner + params.data_amount
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ComplexMixedParams)]
pub struct D9InstrComplexMixed<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    pub authority: Signer<'info>,

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
            b"complex",
            authority.key().as_ref(),
            params.data_owner.as_ref(),
            &params.data_amount.to_le_bytes()
        ],
        bump,
    )]
    #[light_account(init)]
    pub d9_instr_complex_mixed_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
