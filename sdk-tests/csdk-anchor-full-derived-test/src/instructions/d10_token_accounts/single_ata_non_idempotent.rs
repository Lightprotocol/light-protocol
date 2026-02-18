//! D10 Test: Single ATA creation without idempotent flag
//!
//! Tests #[light_account(init, associated_token, ...)] without the idempotent flag.
//! Strict creation: fails if the ATA already exists.
//!
//! This differs from single_ata.rs which (absent the idempotent flag) uses
//! the default idempotent=false behavior. This file makes the strict semantics explicit
//! by omitting the flag, verifying that a second call fails with "account already exists".

use anchor_lang::prelude::*;
use light_account::{
    CreateAccountsProof, LightAccounts, LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_PROGRAM_ID,
    LIGHT_TOKEN_RENT_SPONSOR,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D10SingleAtaNonIdempotentParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests strict (non-idempotent) ATA creation.
/// No `associated_token::idempotent` flag → idempotent=false → second call fails.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D10SingleAtaNonIdempotentParams)]
pub struct D10SingleAtaNonIdempotent<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Token mint for the ATA
    pub d10_non_idem_ata_mint: AccountInfo<'info>,

    /// CHECK: Owner of the ATA
    pub d10_non_idem_ata_owner: AccountInfo<'info>,

    /// ATA account - strict creation, fails if ATA already exists.
    #[account(mut)]
    #[light_account(init,
        associated_token::authority = d10_non_idem_ata_owner,
        associated_token::mint = d10_non_idem_ata_mint)]
    pub d10_non_idem_ata: UncheckedAccount<'info>,

    #[account(address = LIGHT_TOKEN_CONFIG)]
    pub light_token_config: AccountInfo<'info>,

    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light Token Program for CPI
    #[account(address = LIGHT_TOKEN_PROGRAM_ID)]
    pub light_token_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
