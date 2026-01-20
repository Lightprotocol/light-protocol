//! D10 Test: Single ATA creation via macro
//!
//! Tests #[light_account(init, associated_token, ...)] automatic code generation
//! for creating a single compressed token associated token account.
//!
//! This differs from D5 tests which use mark-only mode and manual creation.
//! Here the macro should generate CreateTokenAtaCpi call automatically.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::LightAccounts;
use light_sdk_types::LIGHT_TOKEN_PROGRAM_ID;
use light_token::instruction::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as LIGHT_TOKEN_RENT_SPONSOR};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D10SingleAtaParams {
    pub create_accounts_proof: CreateAccountsProof,
    /// Bump for the ATA PDA
    pub ata_bump: u8,
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
    #[light_account(init, associated_token, owner = d10_ata_owner, mint = d10_ata_mint, bump = params.ata_bump)]
    pub d10_single_ata: UncheckedAccount<'info>,

    #[account(address = COMPRESSIBLE_CONFIG_V1)]
    pub light_token_compressible_config: AccountInfo<'info>,

    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light Token Program for CPI
    #[account(address = LIGHT_TOKEN_PROGRAM_ID.into())]
    pub light_token_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
