//! D9 Test: Constant and static patterns
//!
//! Tests various constant/static seed patterns:
//! - Associated constants: SomeStruct::SEED
//! - Const fn calls: const_fn()
//! - Const fn with generics: const_fn::<T>()
//! - Trait associated constants: <T as Trait>::CONSTANT

use anchor_lang::prelude::*;
use light_account::{CreateAccountsProof, LightAccounts};

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

// ============================================================================
// Constants and types for testing
// ============================================================================

/// Struct with associated constant
pub struct SeedHolder;

impl SeedHolder {
    pub const SEED: &'static [u8] = b"holder_seed";
    pub const NAMESPACE: &'static str = "holder_ns";
}

/// Another struct with associated constant
pub struct AnotherHolder;

impl AnotherHolder {
    pub const PREFIX: &'static [u8] = b"another_prefix";
}

/// Trait with associated constant
pub trait HasSeed {
    const TRAIT_SEED: &'static [u8];
}

impl HasSeed for SeedHolder {
    const TRAIT_SEED: &'static [u8] = b"trait_seed";
}

/// Const fn returning bytes
pub const fn const_seed() -> &'static [u8] {
    b"const_fn_seed"
}

/// Const fn with generic (returns the input)
pub const fn identity_seed<const N: usize>(seed: &'static [u8; N]) -> &'static [u8; N] {
    seed
}

/// Static seed value
pub static STATIC_SEED: [u8; 11] = *b"static_seed";

// ============================================================================
// Test 1: Associated constant
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9AssocConstParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests SomeStruct::CONSTANT pattern
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9AssocConstParams)]
pub struct D9AssocConst<'info> {
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
        seeds = [SeedHolder::SEED],
        bump,
    )]
    #[light_account(init)]
    pub d9_assoc_const_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 2: Associated constant with method
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9AssocConstMethodParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests SomeStruct::CONSTANT.as_bytes() pattern
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9AssocConstMethodParams)]
pub struct D9AssocConstMethod<'info> {
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
        seeds = [SeedHolder::NAMESPACE.as_bytes()],
        bump,
    )]
    #[light_account(init)]
    pub d9_assoc_const_method_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 3: Multiple associated constants
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9MultiAssocConstParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests multiple associated constants from different types
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9MultiAssocConstParams)]
pub struct D9MultiAssocConst<'info> {
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
        seeds = [SeedHolder::SEED, AnotherHolder::PREFIX, params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_multi_assoc_const_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 4: Const fn call
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ConstFnParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests const_fn() pattern
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ConstFnParams)]
pub struct D9ConstFn<'info> {
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
        seeds = [const_seed()],
        bump,
    )]
    #[light_account(init)]
    pub d9_const_fn_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 5: Const fn with const generic
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ConstFnGenericParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests const_fn::<N>() pattern with const generics
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ConstFnGenericParams)]
pub struct D9ConstFnGeneric<'info> {
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
        seeds = [identity_seed::<12>(b"generic_seed")],
        bump,
    )]
    #[light_account(init)]
    pub d9_const_fn_generic_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 6: Trait associated constant (fully qualified)
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9TraitAssocConstParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests <Type as Trait>::CONSTANT pattern
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9TraitAssocConstParams)]
pub struct D9TraitAssocConst<'info> {
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
        seeds = [<SeedHolder as HasSeed>::TRAIT_SEED],
        bump,
    )]
    #[light_account(init)]
    pub d9_trait_assoc_const_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 7: Static variable
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9StaticParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests static variable as seed
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9StaticParams)]
pub struct D9Static<'info> {
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
        seeds = [&STATIC_SEED],
        bump,
    )]
    #[light_account(init)]
    pub d9_static_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 8: Qualified const fn (crate path)
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9QualifiedConstFnParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests crate::module::const_fn() pattern
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9QualifiedConstFnParams)]
pub struct D9QualifiedConstFn<'info> {
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
        seeds = [crate::instructions::d9_seeds::const_patterns::const_seed()],
        bump,
    )]
    #[light_account(init)]
    pub d9_qualified_const_fn_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 9: Fully qualified associated constant
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9FullyQualifiedAssocParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests crate::module::Type::CONSTANT pattern (fully qualified associated constant)
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9FullyQualifiedAssocParams)]
pub struct D9FullyQualifiedAssoc<'info> {
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
        seeds = [crate::instructions::d9_seeds::const_patterns::SeedHolder::SEED],
        bump,
    )]
    #[light_account(init)]
    pub d9_fully_qualified_assoc_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 10: Fully qualified trait associated constant
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9FullyQualifiedTraitParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests <crate::path::Type as crate::path::Trait>::CONSTANT pattern
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9FullyQualifiedTraitParams)]
pub struct D9FullyQualifiedTrait<'info> {
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
        seeds = [<crate::instructions::d9_seeds::const_patterns::SeedHolder as crate::instructions::d9_seeds::const_patterns::HasSeed>::TRAIT_SEED],
        bump,
    )]
    #[light_account(init)]
    pub d9_fully_qualified_trait_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 11: Fully qualified const fn with generics
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9FullyQualifiedGenericParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests crate::module::const_fn::<N>() pattern
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9FullyQualifiedGenericParams)]
pub struct D9FullyQualifiedGeneric<'info> {
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
        seeds = [crate::instructions::d9_seeds::const_patterns::identity_seed::<10>(b"fq_generic")],
        bump,
    )]
    #[light_account(init)]
    pub d9_fully_qualified_generic_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Test 12: Combined patterns with full qualification
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9ConstCombinedParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests combining various constant patterns with full paths
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9ConstCombinedParams)]
pub struct D9ConstCombined<'info> {
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
            crate::instructions::d9_seeds::const_patterns::SeedHolder::SEED,
            crate::instructions::d9_seeds::const_patterns::const_seed(),
            params.owner.as_ref()
        ],
        bump,
    )]
    #[light_account(init)]
    pub d9_const_combined_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
