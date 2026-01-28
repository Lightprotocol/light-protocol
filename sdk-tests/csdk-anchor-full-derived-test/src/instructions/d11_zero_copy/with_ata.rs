//! D11 Test: Zero-copy + ATA
//!
//! Tests `#[light_account(init, zero_copy)]` combined with ATA creation.
//! Verifies that zero-copy PDAs work alongside associated token account creation macros.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::LightAccounts;
use light_token::instruction::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as LIGHT_TOKEN_RENT_SPONSOR};

use crate::state::d11_zero_copy::ZcBasicRecord;

/// Seed for the zero-copy record PDA.
pub const D11_ZC_ATA_RECORD_SEED: &[u8] = b"d11_zc_ata_record";

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D11ZcWithAtaParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
    /// Bump for the ATA (needed for idempotent creation).
    pub ata_bump: u8,
}

/// Tests `#[light_account(init, zero_copy)]` combined with ATA creation.
/// The macro should handle both zero-copy PDA initialization and ATA creation.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D11ZcWithAtaParams)]
pub struct D11ZcWithAta<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config PDA.
    pub compression_config: AccountInfo<'info>,

    /// CHECK: PDA rent sponsor for reimbursement
    #[account(mut)]
    pub pda_rent_sponsor: AccountInfo<'info>,

    /// Zero-copy PDA record.
    #[account(
        init,
        payer = fee_payer,
        space = 8 + core::mem::size_of::<ZcBasicRecord>(),
        seeds = [D11_ZC_ATA_RECORD_SEED, params.owner.as_ref()],
        bump,
    )]
    #[light_account(init, zero_copy)]
    pub zc_ata_record: AccountLoader<'info, ZcBasicRecord>,

    /// CHECK: Token mint for ATA.
    pub d11_ata_mint: AccountInfo<'info>,

    /// CHECK: ATA owner.
    pub d11_ata_owner: AccountInfo<'info>,

    /// User ATA - macro should generate idempotent creation code.
    #[account(mut)]
    #[light_account(init, associated_token::authority = d11_ata_owner, associated_token::mint = d11_ata_mint, associated_token::bump = params.ata_bump)]
    pub d11_user_ata: UncheckedAccount<'info>,

    #[account(address = COMPRESSIBLE_CONFIG_V1)]
    pub light_token_compressible_config: AccountInfo<'info>,

    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token program for CPI.
    pub light_token_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
