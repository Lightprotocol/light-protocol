//! D5 Test: #[rentfree] attribute with #[rentfree_program] macro
//!
//! Tests that the #[rentfree] attribute works correctly when used with the
//! #[rentfree_program] macro on instruction structs in submodules.
//!
//! Note: The params struct must contain `create_accounts_proof: CreateAccountsProof`
//! because the RentFree derive macro generates code that accesses this field.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::RentFree;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D5RentfreeBareParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests that #[rentfree] attribute compiles with the #[rentfree_program] macro.
/// The field name can now differ from the type name (e.g., `record` with type `SinglePubkeyRecord`)
/// because the macro now uses the inner_type for seed spec correlation.
#[derive(Accounts, RentFree)]
#[instruction(params: D5RentfreeBareParams)]
pub struct D5RentfreeBare<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d5_bare", params.owner.as_ref()],
        bump,
    )]
    #[rentfree]
    pub record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
