//! D9 Test: All seed expression types
//!
//! Tests all 6 seed types in a single struct:
//! - Literal: b"d9_all"
//! - Constant: D9_ALL_SEED
//! - CtxAccount: authority.key()
//! - DataField (param): params.owner.as_ref()
//! - DataField (bytes): params.id.to_le_bytes()
//! - FunctionCall: max_key(&a, &b)

use anchor_lang::prelude::*;
use light_account::CreateAccountsProof;
use light_sdk_macros::LightAccounts;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

pub const D9_ALL_SEED: &[u8] = b"d9_all_const";

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D9AllParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
    pub id: u64,
    pub key_a: Pubkey,
    pub key_b: Pubkey,
}

/// Tests all 6 seed types in one struct.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D9AllParams)]
pub struct D9All<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Authority account used in seeds
    pub authority: AccountInfo<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    // Test 1: Literal only
    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d9_all_lit"],
        bump,
    )]
    #[light_account(init)]
    pub d9_all_lit: Account<'info, SinglePubkeyRecord>,

    // Test 2: Constant
    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [D9_ALL_SEED],
        bump,
    )]
    #[light_account(init)]
    pub d9_all_const: Account<'info, SinglePubkeyRecord>,

    // Test 3: CtxAccount
    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d9_all_ctx", authority.key().as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_all_ctx: Account<'info, SinglePubkeyRecord>,

    // Test 4: DataField (param Pubkey)
    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d9_all_param", params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_all_param: Account<'info, SinglePubkeyRecord>,

    // Test 5: DataField (bytes conversion)
    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d9_all_bytes", params.id.to_le_bytes().as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_all_bytes: Account<'info, SinglePubkeyRecord>,

    // Test 6: FunctionCall
    #[account(
        init,
        payer = fee_payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d9_all_func", crate::max_key(&params.key_a, &params.key_b).as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d9_all_func: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
