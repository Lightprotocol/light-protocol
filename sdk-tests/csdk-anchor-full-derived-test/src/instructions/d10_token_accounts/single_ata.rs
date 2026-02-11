//! D10 Test: Single ATA creation via macro
//!
//! Tests #[light_account(init, associated_token, ...)] automatic code generation
//! for creating a single compressed token associated token account.
//!
//! This differs from D5 tests which use mark-only mode and manual creation.
//! Here the macro should generate CreateTokenAtaCpi call automatically.

use anchor_lang::prelude::*;
use light_account::{
    CreateAccountsProof, LightAccounts, LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_PROGRAM_ID,
    LIGHT_TOKEN_RENT_SPONSOR,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D10SingleAtaParams {
    pub create_accounts_proof: CreateAccountsProof,
}

/// Tests #[light_account(init, associated_token, ...)] automatic code generation.
/// The macro should generate CreateTokenAtaCpi in LightFinalize.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D10SingleAtaParams)]
pub struct D10SingleAta<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Token mint for the ATA
    pub d10_ata_mint: AccountInfo<'info>,

    /// CHECK: Owner of the ATA (the fee_payer in this case)
    pub d10_ata_owner: AccountInfo<'info>,

    /// ATA account - macro should generate creation code.
    #[account(mut)]
    #[light_account(init, associated_token::authority = d10_ata_owner, associated_token::mint = d10_ata_mint)]
    pub d10_single_ata: UncheckedAccount<'info>,

    #[account(address = LIGHT_TOKEN_CONFIG)]
    pub light_token_config: AccountInfo<'info>,

    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light Token Program for CPI
    #[account(address = LIGHT_TOKEN_PROGRAM_ID)]
    pub light_token_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
