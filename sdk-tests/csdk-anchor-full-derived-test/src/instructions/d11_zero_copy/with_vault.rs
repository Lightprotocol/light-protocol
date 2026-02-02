//! D11 Test: Zero-copy + Token Vault
//!
//! Tests `#[light_account(init, zero_copy)]` combined with token vault creation.
//! Verifies that zero-copy PDAs work alongside token account creation macros.

use anchor_lang::prelude::*;
use light_sdk_macros::LightAccounts;
use light_account::{CreateAccountsProof, LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR};

use crate::state::d11_zero_copy::ZcBasicRecord;

/// Seed for the vault authority PDA.
pub const D11_ZC_VAULT_AUTH_SEED: &[u8] = b"d11_zc_vault_auth";
/// Seed for the vault token account PDA.
pub const D11_ZC_VAULT_SEED: &[u8] = b"d11_zc_vault";
/// Seed for the zero-copy record PDA.
pub const D11_ZC_RECORD_SEED: &[u8] = b"d11_zc_record";

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D11ZcWithVaultParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
    /// Bump for the vault PDA (needed for invoke_signed).
    pub vault_bump: u8,
}

/// Tests `#[light_account(init, zero_copy)]` combined with token vault creation.
/// The macro should handle both zero-copy PDA initialization and token account creation.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D11ZcWithVaultParams)]
pub struct D11ZcWithVault<'info> {
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
        seeds = [D11_ZC_RECORD_SEED, params.owner.as_ref()],
        bump,
    )]
    #[light_account(init, zero_copy)]
    pub zc_vault_record: AccountLoader<'info, ZcBasicRecord>,

    /// CHECK: Token mint.
    pub d11_mint: AccountInfo<'info>,

    /// Vault authority PDA.
    #[account(seeds = [D11_ZC_VAULT_AUTH_SEED], bump)]
    pub d11_vault_authority: UncheckedAccount<'info>,

    /// Token vault account - macro should generate creation code.
    #[account(
        mut,
        seeds = [D11_ZC_VAULT_SEED, d11_mint.key().as_ref()],
        bump,
    )]
    #[light_account(init, token::seeds = [D11_ZC_VAULT_SEED, self.d11_mint.key()], token::mint = d11_mint, token::owner = d11_vault_authority, token::bump = params.vault_bump, token::owner_seeds = [D11_ZC_VAULT_AUTH_SEED])]
    pub d11_zc_vault: UncheckedAccount<'info>,

    #[account(address = LIGHT_TOKEN_CONFIG)]
    pub light_token_config: AccountInfo<'info>,

    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token CPI authority.
    pub light_token_cpi_authority: AccountInfo<'info>,

    /// CHECK: Light token program for CPI.
    pub light_token_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
