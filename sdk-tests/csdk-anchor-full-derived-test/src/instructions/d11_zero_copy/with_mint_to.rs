//! D11 Test: Zero-copy + Vault + MintTo
//!
//! Tests `#[light_account(init, zero_copy)]` combined with token vault and minting.
//! Verifies that zero-copy PDAs work alongside token vault creation and MintTo CPI.

use anchor_lang::prelude::*;
use light_compressible::CreateAccountsProof;
use light_sdk_macros::LightAccounts;
use light_token::instruction::{COMPRESSIBLE_CONFIG_V1, RENT_SPONSOR as LIGHT_TOKEN_RENT_SPONSOR};

use crate::state::d11_zero_copy::ZcBasicRecord;

/// Seed for the vault authority PDA.
pub const D11_MINT_VAULT_AUTH_SEED: &[u8] = b"d11_mint_vault_auth";
/// Seed for the vault token account PDA.
pub const D11_MINT_VAULT_SEED: &[u8] = b"d11_mint_vault";
/// Seed for the zero-copy record PDA.
pub const D11_MINT_ZC_RECORD_SEED: &[u8] = b"d11_mint_zc_record";

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct D11ZcWithMintToParams {
    pub create_accounts_proof: CreateAccountsProof,
    pub owner: Pubkey,
    /// Bump for the vault PDA (needed for invoke_signed).
    pub vault_bump: u8,
    /// Amount to mint to the vault.
    pub mint_amount: u64,
}

/// Tests `#[light_account(init, zero_copy)]` combined with vault and MintTo.
/// The instruction creates a zero-copy PDA, a token vault, and mints tokens.
#[derive(Accounts, LightAccounts)]
#[instruction(params: D11ZcWithMintToParams)]
pub struct D11ZcWithMintTo<'info> {
    #[account(mut)]
    pub fee_payer: Signer<'info>,

    /// CHECK: Compression config PDA.
    pub compression_config: AccountInfo<'info>,

    /// Zero-copy PDA record.
    #[account(
        init,
        payer = fee_payer,
        space = 8 + core::mem::size_of::<ZcBasicRecord>(),
        seeds = [D11_MINT_ZC_RECORD_SEED, params.owner.as_ref()],
        bump,
    )]
    #[light_account(init, zero_copy)]
    pub zc_mint_record: AccountLoader<'info, ZcBasicRecord>,

    /// CHECK: Token mint.
    #[account(mut)]
    pub d11_mint: AccountInfo<'info>,

    /// Mint authority - must sign for MintTo.
    pub mint_authority: Signer<'info>,

    /// Vault authority PDA.
    #[account(seeds = [D11_MINT_VAULT_AUTH_SEED], bump)]
    pub d11_vault_authority: UncheckedAccount<'info>,

    /// Token vault account - macro should generate creation code.
    #[account(
        mut,
        seeds = [D11_MINT_VAULT_SEED, d11_mint.key().as_ref()],
        bump,
    )]
    #[light_account(init, token::authority = [D11_MINT_VAULT_SEED, self.d11_mint.key()], token::mint = d11_mint, token::owner = d11_vault_authority, token::bump = params.vault_bump)]
    pub d11_mint_vault: UncheckedAccount<'info>,

    #[account(address = COMPRESSIBLE_CONFIG_V1)]
    pub light_token_compressible_config: AccountInfo<'info>,

    #[account(mut, address = LIGHT_TOKEN_RENT_SPONSOR)]
    pub light_token_rent_sponsor: AccountInfo<'info>,

    /// CHECK: Light token CPI authority.
    pub light_token_cpi_authority: AccountInfo<'info>,

    /// CHECK: Light token program for CPI.
    pub light_token_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}
