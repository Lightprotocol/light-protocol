//! D10 Test: Single ATA creation in mark-only mode
//!
//! Tests #[light_account(associated_token::...)] WITHOUT init keyword.
//! The macro generates no-op LightPreInit/LightFinalize impls but seed structs
//! and variant enums are still generated for decompression support.
//! User manually calls CreateTokenAtaCpi in the instruction handler.

use anchor_lang::prelude::*;
use light_account::{
    LightAccounts, LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_PROGRAM_ID, LIGHT_TOKEN_RENT_SPONSOR,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D10SingleAtaMarkonlyParams {
    /// Bump for the ATA PDA
    pub ata_bump: u8,
}

/// Tests #[light_account(associated_token::...)] mark-only mode (NO init keyword).
/// The macro generates no-op LightPreInit/LightFinalize impls.
/// User manually calls CreateTokenAtaCpi in the handler.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D10SingleAtaMarkonlyParams)]
pub struct D10SingleAtaMarkonly<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Token mint for the ATA
    pub d10_markonly_ata_mint: AccountInfo<'info>,

    /// CHECK: Owner of the ATA
    pub d10_markonly_ata_owner: AccountInfo<'info>,

    /// ATA account - mark-only mode, created manually via CreateTokenAtaCpi.
    #[account(mut)]
    // Mark-only: authority and mint only (no init keyword)
    #[light_account(associated_token::authority = d10_markonly_ata_owner, associated_token::mint = d10_markonly_ata_mint)]
    pub d10_markonly_ata: UncheckedAccount<'info>,

    #[account(address = LIGHT_TOKEN_CONFIG)]
    pub light_token_config: AccountInfo<'info>,

    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light Token Program for CPI
    #[account(address = LIGHT_TOKEN_PROGRAM_ID)]
    pub light_token_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
