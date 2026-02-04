//! D7 Test: "payer" field name variant
//!
//! Tests that #[light_account(init)] works when the payer field is named `payer` instead of `fee_payer`.

use anchor_lang::prelude::*;
use light_account::{CreateAccountsProof, LightAccounts};

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D7PayerParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests #[light_account(init)] with `payer` field name (InfraFieldClassifier FeePayer variant).
#[derive(Accounts, LightAccounts)]
#[instruction(params: D7PayerParams)]
pub struct D7Payer<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d7_payer", params.owner.as_ref()],
        bump,
    )]
    #[light_account(init)]
    pub d7_payer_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
