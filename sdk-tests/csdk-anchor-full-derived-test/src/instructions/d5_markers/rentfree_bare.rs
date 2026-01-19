//! D5 Test: #[light_account(init)] attribute with #[light_program] macro
//!
//! Tests that the #[light_account(init)] attribute works correctly when used with the
//! #[light_program] macro on instruction structs in submodules.
//!
//! Note: The params struct must contain `create_accounts_proof: CreateAccountsProof`
//! because the RentFree derive macro generates code that accesses this field.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::LightAccounts;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D5RentfreeBareParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests that #[light_account(init)] attribute compiles with the #[light_program] macro.
/// The field name can now differ from the type name (e.g., `record` with type `SinglePubkeyRecord`)
/// because the macro now uses the inner_type for seed spec correlation.
#[derive(Accounts, LightAccounts)]
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
    #[light_account(init)]
    pub record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
