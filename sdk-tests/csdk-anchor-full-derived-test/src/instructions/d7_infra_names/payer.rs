//! D7 Test: "payer" field name variant
//!
//! Tests that #[rentfree] works when the payer field is named `payer` instead of `fee_payer`.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::RentFree;

use crate::state::d1_field_types::single_pubkey::SinglePubkeyRecord;

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct D7PayerParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
}

/// Tests #[rentfree] with `payer` field name (InfraFieldClassifier FeePayer variant).
#[derive(Accounts, RentFree)]
#[instruction(params: D7PayerParams)]
pub struct D7Payer<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Compression config
    pub compression_config: AccountInfo<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + SinglePubkeyRecord::INIT_SPACE,
        seeds = [b"d7_payer", params.owner.as_ref()],
        bump,
    )]
    #[rentfree]
    pub d7_payer_record: Account<'info, SinglePubkeyRecord>,

    pub system_program: Program<'info, System>,
}
